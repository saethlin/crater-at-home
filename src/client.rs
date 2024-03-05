use crate::{Crate, Tool, Version};
use anyhow::Result;
use backoff::Error;
use backoff::ExponentialBackoff;
use ssh2::Session;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::path::Path;
use xz2::write::XzDecoder;

#[derive(Clone)]
pub struct Client {
    session: Session,
    tool: Tool,
}

impl Client {
    pub fn new(tool: Tool, server: &str) -> Result<Self> {
        let stream = TcpStream::connect(server)?;
        let mut session = Session::new()?;
        session.set_tcp_stream(stream);
        session.handshake()?;
        session.userauth_pubkey_file(
            "ubuntu",
            None,
            Path::new("/home/ben/.ssh/general-purpose.pem"),
            None,
        )?;
        session.set_keepalive(true, 60);

        let sftp = session.sftp()?;
        let raw_dir = Path::new(tool.raw_path());
        let parent = raw_dir.parent().unwrap();
        let _ = sftp.mkdir(parent, 0o755);
        let _ = sftp.mkdir(raw_dir, 0o755);

        Ok(Self { session, tool })
    }

    pub fn upload(&self, key: &str, data: &[u8]) -> Result<()> {
        self._upload(key, data)
    }

    fn _upload(&self, key: &str, data: &[u8]) -> Result<()> {
        log::debug!("Attempting upload to {}", key);
        let parent = Path::new(key).parent().unwrap();
        let sftp = self.session.sftp()?;
        let _ = sftp.mkdir(parent, 0o755);

        let mut remote_file =
            self.session
                .scp_send(Path::new(key), 0o644, data.len() as u64, None)?;
        remote_file.write_all(data)?;
        remote_file.send_eof()?;
        remote_file.wait_eof()?;
        remote_file.close()?;
        remote_file.wait_close()?;
        log::debug!("Completed upload to {}", key);
        Ok(())
    }

    pub fn upload_raw(&self, krate: &Crate, data: &[u8]) -> Result<()> {
        self.upload(&self.tool.raw_crate_path(krate), data)
    }

    pub fn download_raw(&self, krate: &Crate) -> Result<Vec<u8>> {
        let bytes = self.download(&self.tool.raw_crate_path(krate))?;
        let mut decoder = XzDecoder::new(Vec::new());
        decoder.write_all(&bytes)?;
        Ok(decoder.finish()?)
    }

    fn download(&self, key: &str) -> Result<Vec<u8>> {
        self._download(key)
    }

    fn _download(&self, key: &str) -> Result<Vec<u8>> {
        log::debug!("Attempting download of {}", key);
        let (mut remote_file, stat) = self.session.scp_recv(Path::new(key))?;
        let mut contents = Vec::with_capacity(stat.size() as usize);
        remote_file.read_to_end(&mut contents)?;
        log::debug!("Completed download of {}", key);
        Ok(contents)
    }

    pub fn get_crate_versions(&self) -> Result<Vec<Crate>> {
        let blob = self.download("/crater-at-home/crates.json")?;
        let crates: Vec<(String, String)> = serde_json::from_slice(&blob)?;
        let crates = crates
            .into_iter()
            .map(|krate| Crate {
                name: krate.0,
                version: Version::parse(&krate.1),
                recent_downloads: None,
            })
            .collect();
        Ok(crates)
    }

    pub fn list_finished_crates(&self, _dur: Option<time::Duration>) -> Result<Vec<Crate>> {
        let mut files = Vec::new();
        let prefix = Path::new(self.tool.raw_path());

        let sftp = self.session.sftp()?;
        let mut dir = match sftp.opendir(prefix) {
            Ok(dir) => dir,
            Err(e) => {
                if e.code() == ssh2::ErrorCode::SFTP(2) {
                    sftp.mkdir(prefix, 0o755)?;
                    return Ok(Vec::new());
                }
                return Err(e)?;
            }
        };
        while let Ok((name, _stat)) = dir.readdir() {
            if name == Path::new(".") || name == Path::new("..") {
                continue;
            }
            let path = prefix.join(&name);
            let mut name_dir = sftp.opendir(&path)?;
            while let Ok((version, _stat)) = name_dir.readdir() {
                if version == Path::new(".") || version == Path::new("..") {
                    continue;
                }
                files.push(Crate {
                    name: name.display().to_string(),
                    version: Version::parse(version.to_str().unwrap()),
                    recent_downloads: None,
                });
            }
        }

        Ok(files)
    }
}

fn retry<I, E, Func>(mut f: Func) -> std::result::Result<I, E>
where
    Func: FnMut() -> std::result::Result<I, E>,
    E: std::fmt::Display,
{
    backoff::retry_notify(
        ExponentialBackoff::default(),
        || f().map_err(Error::transient),
        |e, _| {
            log::warn!("{}", e);
        },
    )
    .map_err(|e| match e {
        backoff::Error::Permanent(e) => e,
        backoff::Error::Transient { err, .. } => err,
    })
}
