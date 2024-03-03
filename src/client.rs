use crate::{Crate, Tool, Version};
use anyhow::Result;
use backoff::Error;
use backoff::ExponentialBackoff;
use ssh2::Session;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::path::Path;

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

        Ok(Self { session, tool })
    }

    pub fn upload(&self, key: &str, data: &[u8]) -> Result<()> {
        retry(|| self._upload(key, data))
    }

    fn _upload(&self, key: &str, data: &[u8]) -> Result<()> {
        let mut remote_file =
            self.session
                .scp_send(Path::new(key), 0o644, data.len() as u64, None)?;
        remote_file.write_all(data)?;
        remote_file.send_eof()?;
        remote_file.wait_eof()?;
        remote_file.close()?;
        remote_file.wait_close()?;
        Ok(())
    }

    pub fn upload_raw(&self, krate: &Crate, data: Vec<u8>) -> Result<()> {
        self.upload(&self.tool.raw_crate_path(krate), &data)
    }

    pub fn download_raw(&self, krate: &Crate) -> Result<Vec<u8>> {
        self.download(&self.tool.raw_crate_path(krate))
    }

    fn download(&self, key: &str) -> Result<Vec<u8>> {
        retry(|| self._download(key))
    }

    fn _download(&self, key: &str) -> Result<Vec<u8>> {
        let (mut remote_file, stat) = self.session.scp_recv(Path::new(key))?;
        let mut contents = Vec::with_capacity(stat.size() as usize);
        remote_file.read_to_end(&mut contents)?;
        Ok(contents)
    }

    pub fn get_crate_versions(&self) -> Result<Vec<Crate>> {
        let blob = self.download("crates.json")?;
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
        let prefix = std::path::PathBuf::from(self.tool.raw_path());

        let sftp = self.session.sftp()?;
        let mut dir = sftp.opendir(&prefix)?;
        while let Ok((name, _stat)) = dir.readdir() {
            let path = prefix.join(&name);
            let mut name_dir = sftp.opendir(&path)?;
            while let Ok((version, _stat)) = name_dir.readdir() {
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
