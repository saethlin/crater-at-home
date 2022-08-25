use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    #[clap(value_name = "CRATE")]
    krate: String,

    output: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    assert!(args.krate.contains("=="));

    let mut it = args.krate.split("==");
    let name = it.next().unwrap();
    let version = it.next().unwrap();
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
        let full_path = args.output.join(&components.as_path());
        // unpack doesn't make directories for us, we need to handle that
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        entry.unpack(&full_path)?;
    }

    Ok(())
}
