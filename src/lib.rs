use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::Path;

use color_eyre::Report;
use flate2::read::GzDecoder;
use once_cell::sync::Lazy;
use serde::de::Visitor;
use tar::Archive;

pub mod db_dump;
pub mod diagnose;

use diagnose::diagnose;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Crate {
    pub name: String,
    pub recent_downloads: Option<u64>,
    pub version: String,
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Time that the run took, in seconds
    pub time: Option<u64>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Status {
    Unknown,
    Passing,
    Error(String),
    UB {
        #[serde(
            deserialize_with = "cause_version_remap",
            default,
            skip_serializing_if = "Vec::is_empty"
        )]
        cause: Vec<Cause>,
        status: String,
    },
}

fn cause_version_remap<'de, D>(deserializer: D) -> Result<Vec<Cause>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct StringOrStruct;

    impl<'de> Visitor<'de> for StringOrStruct {
        type Value = Vec<Cause>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_str<E>(self, value: &str) -> Result<Vec<Cause>, E>
        where
            E: serde::de::Error,
        {
            let mut causes = Vec::new();
            for cause in value.split(',') {
                let mut splits = cause.split_terminator('(');
                let kind = splits.next().unwrap_or_default().trim().to_string();
                let source_crate = splits
                    .next()
                    .map(|s| s.trim_end_matches(')').trim().to_string());
                causes.push(Cause { kind, source_crate })
            }
            Ok(causes)
        }

        fn visit_seq<M>(self, map: M) -> Result<Vec<Cause>, M::Error>
        where
            M: serde::de::SeqAccess<'de>,
        {
            serde::Deserialize::deserialize(serde::de::value::SeqAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(StringOrStruct)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Ord, Eq, PartialEq, PartialOrd)]
pub struct Cause {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_crate: Option<String>,
}

static AGENT: Lazy<ureq::Agent> = Lazy::new(ureq::Agent::new);

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Net(ureq::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(fmt, "{}", e),
            Error::Net(e) => write!(fmt, "{}", e),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<ureq::Error> for Error {
    fn from(e: ureq::Error) -> Self {
        Error::Net(e)
    }
}

impl std::error::Error for Error {}

impl Crate {
    pub fn fetch_into(&self, dest: &Path) -> Result<(), Error> {
        let cache_path = format!("cache/{}-{}.crate", self.name, self.version);
        let archive = match fs::read(&cache_path) {
            Ok(archive) => archive,
            Err(_) => {
                let url = format!(
                    "https://static.crates.io/crates/{0}/{0}-{1}.crate",
                    self.name, self.version
                );

                let mut archive = Vec::new();
                AGENT
                    .get(&url)
                    .call()?
                    .into_reader()
                    .read_to_end(&mut archive)?;

                fs::create_dir_all("cache")?;
                let _ = fs::write(cache_path, &archive);
                archive
            }
        };

        let mut tar = Archive::new(GzDecoder::new(archive.as_slice()));

        unpack_without_first_dir(&mut tar, dest)
    }
}

fn unpack_without_first_dir<R: Read>(archive: &mut Archive<R>, path: &Path) -> Result<(), Error> {
    let entries = archive.entries()?;
    for entry in entries {
        let mut entry = entry?;
        let relpath = {
            let path = entry.path();
            let path = path?;
            path.into_owned()
        };
        let mut components = relpath.components();
        // Throw away the first path component
        components.next();
        let full_path = path.join(&components.as_path());
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        entry.unpack(&full_path)?;
    }

    Ok(())
}

pub fn load_completed_crates() -> Result<Vec<Crate>, Report> {
    let mut crates = HashMap::new();

    for entry in std::fs::read_dir("logs")? {
        let entry = entry?;
        let mut versions = Vec::new();
        for ver in fs::read_dir(entry.path())? {
            let path = ver?.path();
            let ver = path.file_name().unwrap().to_str().unwrap();
            if !ver.ends_with(".html") {
                versions.push(ver.to_string());
            }
        }
        versions.sort_by_key(|ver| semver::Version::parse(ver).ok());

        let version = if let Some(latest) = versions.pop() {
            latest
        } else {
            continue;
        };

        let mut krate = Crate {
            name: entry
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            version,
            status: Status::Unknown,
            recent_downloads: None,
            time: None,
        };

        let path = format!("logs/{}/{}", krate.name, krate.version);
        if let Ok(output) = fs::read_to_string(path) {
            let time_prefix = "\tElapsed (wall clock) time (h:mm:ss or m:ss): ";
            if let Some(line) = output
                .lines()
                .rev()
                .find(|line| line.starts_with(time_prefix))
            {
                let line = line.strip_prefix(time_prefix).unwrap().trim();
                let mut duration = 0;
                let mut it = line.rsplit(':');
                if let Some(seconds) = it.next() {
                    duration += seconds.parse::<f64>()? as u64;
                }
                if let Some(minutes) = it.next() {
                    duration += minutes.parse::<u64>()? * 60;
                }
                if let Some(hours) = it.next() {
                    duration += hours.parse::<u64>()? * 60 * 60;
                }
                krate.time = Some(duration);
            }
        }

        diagnose(&mut krate)?;

        crates.insert(krate.name.clone(), krate);
    }

    for c in db_dump::download()?.into_iter() {
        if let Some(krate) = crates.get_mut(&c.name) {
            krate.recent_downloads = c.recent_downloads;
        }
    }

    let mut crates = crates.values().cloned().collect::<Vec<_>>();

    crates.sort_by(|a, b| b.recent_downloads.cmp(&a.recent_downloads));

    Ok(crates)
}
