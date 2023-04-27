use crate::db_dump;
use clap::Parser;
use color_eyre::Result;
use std::sync::Arc;

#[derive(Parser)]
pub struct Args {
    #[clap(long)]
    bucket: String,
}

pub fn run(args: Args) -> Result<()> {
    let crates = db_dump::download()?;
    let mut output = Vec::new();
    for krate in crates.iter().cloned() {
        output.push((krate.name, krate.version));
    }
    let serialized = serde_json::to_string(&output).unwrap();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let config = rt.block_on(aws_config::load_from_env());
    let client = Arc::new(aws_sdk_s3::Client::new(&config));
    let fut = client
        .put_object()
        .bucket(&args.bucket)
        .key("crates.json")
        .body(serialized.into_bytes().into())
        .content_type("application/json")
        .send();
    rt.block_on(fut)?;
    Ok(())
}
