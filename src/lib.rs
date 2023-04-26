use color_eyre::{eyre::WrapErr, Report, Result};
use flate2::read::GzDecoder;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use std::{collections::HashMap, fmt, fs, io::Read, path::Path, str::FromStr};

pub mod db_dump;
pub mod diagnose;
pub mod render;
pub mod run;
pub mod sync;
pub mod upload;

use diagnose::diagnose;
use tar::Archive;

#[derive(Clone, Copy)]
pub enum Tool {
    Miri,
    Asan,
}

impl Tool {
    pub fn raw_path(self) -> &'static str {
        match self {
            Tool::Miri => "miri/raw",
            Tool::Asan => "asan/raw",
        }
    }

    pub fn raw_crate_path(self, krate: &Crate) -> String {
        format!("{}/{}/{}", self.raw_path(), krate.name, krate.version)
    }

    pub fn html_path(self) -> &'static str {
        match self {
            Tool::Miri => "miri/logs",
            Tool::Asan => "asan/logs",
        }
    }

    pub fn rendered_crate_path(self, krate: &Crate) -> String {
        format!("{}/{}/{}", self.html_path(), krate.name, krate.version)
    }
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Tool::Miri => "miri",
            Tool::Asan => "asan",
        };
        f.write_str(s)
    }
}

impl FromStr for Tool {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "miri" => Ok(Self::Miri),
            "asan" => Ok(Self::Asan),
            _ => Err(format!("Invalid tool {}", s)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Crate {
    pub name: String,
    pub version: Version,
    pub recent_downloads: Option<u64>,
    pub status: Status,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Debug)]
pub enum Version {
    Parsed(semver::Version),
    Unparsed(String),
}

impl serde::Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl Version {
    pub fn parse(s: &str) -> Self {
        semver::Version::parse(s)
            .map(Version::Parsed)
            .unwrap_or_else(|_| Version::Unparsed(s.to_string()))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Version::Parsed(v) => write!(f, "{}", v),
            Version::Unparsed(v) => write!(f, "{}", v),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Status {
    Unknown,
    Passing,
    Error(String),
    UB { cause: Vec<Cause> },
}

#[derive(Clone, Debug, Ord, Eq, PartialEq, PartialOrd)]
pub struct Cause {
    pub kind: String,
    pub source_crate: Option<String>,
}

static AGENT: Lazy<ureq::Agent> = Lazy::new(ureq::Agent::new);

impl Crate {
    pub fn fetch_into(&self, dest: &Path) -> Result<()> {
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

fn unpack_without_first_dir<R: Read>(archive: &mut Archive<R>, path: &Path) -> Result<()> {
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
        let full_path = path.join(components.as_path());
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        entry.unpack(&full_path)?;
    }

    Ok(())
}

pub fn load_completed_crates() -> Result<HashMap<String, Vec<Crate>>> {
    log::info!("Scanning logs directory for completed runs");

    let db_dump = std::thread::spawn(db_dump::download);

    let entries = std::fs::read_dir("logs")?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let mut crates = entries
        .par_iter()
        .map(|entry| {
            let name = entry
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();

            let mut crates = Vec::new();
            for ver in fs::read_dir(entry.path())
                .map_err(Report::new)
                .with_context(move || entry.path().display().to_string())?
            {
                let path = ver?.path();
                let ver = path.file_name().unwrap().to_str().unwrap();
                if ver.ends_with(".html") {
                    continue;
                }
                let version = Version::parse(ver);

                let mut krate = Crate {
                    name: name.clone(),
                    version,
                    status: Status::Unknown,
                    recent_downloads: None,
                };

                diagnose(&mut krate)?;
                crates.push(krate);
            }
            crates.sort_by(|a, b| a.version.cmp(&b.version));

            Ok((name, crates))
        })
        .collect::<Result<HashMap<String, Vec<Crate>>>>()?;

    log::info!("Logs collected");

    let db_dump = db_dump.join().unwrap()?;
    log::info!("Database processed");

    for c in db_dump.into_iter() {
        if let Some(krates) = crates.get_mut(&c.name) {
            for krate in krates {
                krate.recent_downloads = c.recent_downloads;
            }
        }
    }

    Ok(crates)
}
