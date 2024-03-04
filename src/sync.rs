use crate::{client::Client, db_dump, Crate, Status, Tool};
use anyhow::Result;
use clap::Parser;
use std::{collections::HashMap, sync::Arc};

#[derive(Parser)]
pub struct Args {
    #[clap(long)]
    tool: Tool,

    #[clap(long)]
    bucket: String,
}

#[tokio::main]
pub async fn run(args: &Args) -> Result<()> {
    let client = Arc::new(Client::new(args.tool, &args.bucket)?);

    log::info!("Updating the cached crates.io database dump");
    let crates = db_dump::download()?;
    let mut name_to_downloads = HashMap::new();
    let mut versions = Vec::new();
    for krate in &crates {
        name_to_downloads.insert(krate.name.clone(), krate.recent_downloads);
        versions.push((krate.name.clone(), krate.version.to_string()));
    }

    let serialized = serde_json::to_string(&versions).unwrap();
    client.upload("/crater-at-home/crates.json", serialized.as_bytes())?;
    let serialized = serde_json::to_string(&name_to_downloads).unwrap();
    client.upload("/crater-at-home/downloads.json", serialized.as_bytes())?;

    log::info!("Re-analyzing all crates to build the /ub page");
    let mut crates = diagnose_all(&client)?;

    // Sort crates by recent downloads, descending
    // Then by version, descending
    crates.sort_by(|(crate_a, _), (crate_b, _)| {
        let a = name_to_downloads.get(&crate_a.name).copied().flatten();
        let b = name_to_downloads.get(&crate_b.name).copied().flatten();
        b.cmp(&a)
            .then_with(|| crate_b.version.cmp(&crate_a.version))
    });
    // Since we sored by version we can dedup by name and be left with only
    // the most recent version of each crate.
    crates.dedup_by(|(a, _), (b, _)| a.name == b.name);

    let ub_page = crate::render::render_ub(&crates)?;
    client.upload(
        &format!("/crater-at-home/{}/ub", args.tool),
        ub_page.as_bytes(),
    )?;

    Ok(())
}

fn diagnose_all(client: &Client) -> Result<Vec<(Crate, Status)>> {
    let all = client.list_finished_crates(None)?;
    let mut output = Vec::new();
    for krate in all {
        let raw = client.download_raw(&krate)?;
        let status = crate::diagnose(&raw);
        output.push((krate, status));
    }
    Ok(output)
}
