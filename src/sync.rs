use crate::{client::Client, db_dump, Tool};
use clap::Parser;
use color_eyre::Result;
use std::{fmt::Write, sync::Arc};

#[derive(Parser)]
pub struct Args {
    #[clap(long)]
    tool: Tool,

    #[clap(long)]
    bucket: String,
}

#[tokio::main]
pub async fn run(args: Args) -> Result<()> {
    log::info!("Rendering fresh landing page...");
    let client = Client::new(args.tool, &args.bucket).await?;
    sync_landing_page(&client).await?;
    log::info!("New landing page uploaded");

    let crates = db_dump::download()?;
    let mut output = Vec::new();
    for krate in crates.iter().cloned() {
        output.push((krate.name, krate.version));
    }
    let serialized = serde_json::to_string(&output).unwrap();

    let config = aws_config::load_from_env().await;
    let client = Arc::new(aws_sdk_s3::Client::new(&config));
    client
        .put_object()
        .bucket(&args.bucket)
        .key("crates.json")
        .body(serialized.into_bytes().into())
        .content_type("application/json")
        .send()
        .await?;

    Ok(())
}

async fn sync_landing_page(client: &Client) -> Result<()> {
    // Download the crate list so we know what the ordering is
    //let crates = client.get_crate_list().await?;
    //let ranks: HashMap<_, _> = crates.iter().enumerate().map(|(index, krate)| (&krate.name, index)).collect();

    // List all rendered HTML
    let rendered = client.list_rendered_crates().await?;

    println!("{:?}", rendered.len());

    let mut output = String::from(crate::render::LANDING_PAGE);
    for c in &rendered {
        let mut it = c.splitn(2, '/');
        let Some(name) = it.next() else { continue; };
        let Some(version) = it.next() else { continue; };
        writeln!(output, "\"{}\": [\"{}\"],", name, version)?;
    }
    output.pop();
    output.push_str("};</script></html>");

    client.upload_landing_page(output.into_bytes()).await?;

    Ok(())
}
