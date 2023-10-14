use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    #[clap(value_name = "CRATE")]
    krate: String,

    output: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse_from(std::env::args().skip(1));

    let mut it = args.krate.split('@');
    let name = it.next().unwrap();
    let version = it.next();
    let version = match version {
        Some(v) => v.to_string(),
        None => {
            let response = ureq::get(&format!("https://crates.io/api/v1/crates/{name}")).call()?;
            let body = response.into_string()?;
            let body = json::parse(&body)?;
            body["crate"]["max_version"].as_str().unwrap().to_string()
        }
    };

    // Fail fast if the destination directory can't be made
    let output: PathBuf = args
        .output
        .unwrap_or_else(|| format!("{name}-{version}"))
        .into();
    std::fs::create_dir_all(&output)?;

    assert!(it.next().is_none());
    let download_url = format!(
        "https://static.crates.io/crates/{}/{}-{}.crate",
        name, name, version
    );

    let response = ureq::get(&download_url).call()?;
    let reader = flate2::read::GzDecoder::new(response.into_reader());
    let mut archive = tar::Archive::new(reader);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let relpath = entry.path()?;
        let mut components = relpath.components();
        // Throw away the first path component
        components.next();
        let full_path = output.join(&components.as_path());
        // unpack doesn't make directories for us, we need to handle that
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        entry.unpack(&full_path)?;
    }

    Ok(())
}
