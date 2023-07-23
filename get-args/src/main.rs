use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::process::Command;

#[derive(Debug, Deserialize)]
struct Config {
    packages: Vec<Package>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    metadata: Option<HashMap<String, Value>>,
}

impl Package {
    fn playground(&self) -> Option<Metadata> {
        let value = self.metadata.as_ref()?.get("playground")?.clone();
        serde_json::from_value(value).ok()
    }

    fn docsrs(&self) -> Option<Metadata> {
        let value = self.metadata.as_ref()?.get("docs.rs")?.clone();
        serde_json::from_value(value).ok()
    }

    fn docs_rs(&self) -> Option<Metadata> {
        let value = self.metadata.as_ref()?.get("docs")?.clone();
        let value = value.as_object()?.get("rs")?.clone();
        serde_json::from_value(value).ok()
    }
}

#[derive(Default, Debug, Deserialize)]
struct Metadata {
    #[serde(default)]
    features: HashSet<String>,
    #[serde(rename = "no-default-features", default)]
    no_default_features: bool,
    #[serde(rename = "all-features", default)]
    all_features: bool,
}

impl Metadata {
    fn merge(&mut self, other: Self) {
        self.features.extend(other.features);
        self.no_default_features |= other.no_default_features;
        self.all_features |= other.all_features;
    }
}

fn run() -> Option<String> {
    let krate = std::env::args().nth(1)?;
    let krate = krate.split("==").next()?;

    let mut cmd = Command::new("cargo");
    if let Ok(toolchain) = std::env::var("TOOLCHAIN") {
        cmd.arg(format!("+{toolchain}"));
    }
    let config = cmd.arg("metadata").output().ok()?;
    if !config.status.success() {
        std::process::exit(config.status.code().unwrap_or(1));
    }

    let config: Config = serde_json::from_slice(&config.stdout).ok()?;

    let krate = config
        .packages
        .iter()
        .find(|package| package.name == krate)?;

    let mut metadata = Metadata::default();

    for section in [krate.playground(), krate.docsrs(), krate.docs_rs()] {
        if let Some(section) = section {
            metadata.merge(section);
        }
    }

    let mut args: Vec<String> = Vec::new();
    if metadata.no_default_features {
        args.push("--no-default-features".to_string());
    }
    if metadata.all_features {
        args.push("--all-features".to_string());
    }

    let mut features = metadata.features.into_iter().collect::<Vec<_>>();
    features.sort();
    let features = features.join(",");
    if !features.is_empty() {
        args.push(format!("--features={}", features));
    }

    Some(args.join(" "))
}

fn main() {
    if let Some(args) = run() {
        println!("{args}");
    } else {
        std::process::exit(1);
    }
}
