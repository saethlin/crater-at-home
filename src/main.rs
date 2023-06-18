use clap::Parser;
use color_eyre::Result;
use diagnose::diagnose;
use std::{fmt, str::FromStr};

mod client;
mod db_dump;
mod diagnose;
mod render;
mod run;
mod sync;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser)]
enum Commands {
    Run(run::Args),
    Sync(sync::Args),
}

fn main() -> Result<()> {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    color_eyre::install()?;

    let args = Cli::parse();
    match args.command {
        Commands::Run(args) => run::run(args),
        Commands::Sync(args) => sync::run(args),
    }
}

#[derive(Clone, Copy)]
pub enum Tool {
    Miri,
    Asan,
    Build,
}

impl Tool {
    pub fn raw_path(self) -> &'static str {
        match self {
            Tool::Miri => "miri/raw",
            Tool::Asan => "asan/raw",
            Tool::Build => "build/raw",
        }
    }

    pub fn raw_crate_path(self, krate: &Crate) -> String {
        format!("{}/{}/{}", self.raw_path(), krate.name, krate.version)
    }

    pub fn html_path(self) -> &'static str {
        match self {
            Tool::Miri => "miri/logs",
            Tool::Asan => "asan/logs",
            Tool::Build => "build/logs",
        }
    }

    pub fn rendered_crate_path(self, krate: &Crate) -> String {
        format!("{}/{}/{}", self.html_path(), krate.name, krate.version)
    }

    pub fn landing_page_path(self) -> &'static str {
        match self {
            Tool::Miri => "miri/index.html",
            Tool::Asan => "asan/index.html",
            Tool::Build => "build/index.html",
        }
    }
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Tool::Miri => "miri",
            Tool::Asan => "asan",
            Tool::Build => "build",
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
            "build" => Ok(Self::Build),
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
