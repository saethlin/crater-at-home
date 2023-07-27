use crate::{client::Client, db_dump, render, Crate, Tool};
use clap::Parser;
use color_eyre::{Report, Result};
use std::{collections::HashMap, fmt::Write, sync::Arc};
use tokio::{sync::Mutex, sync::Semaphore, task::JoinSet};

#[derive(Parser)]
pub struct Args {
    #[clap(long)]
    tool: Tool,

    #[clap(long)]
    bucket: String,
}

#[tokio::main]
pub async fn run(args: Args) -> Result<()> {
    let client = Arc::new(Client::new(args.tool, &args.bucket).await?);

    log::info!("Rendering fresh landing page");
    sync_landing_page(&client).await?;

    log::info!("Uploading the error page");
    client
        .put_object()
        .key(format!("{}/403", args.tool))
        .body(ERROR_PAGE.as_bytes().to_vec().into())
        .content_type("text/html")
        .send()
        .await?;

    log::info!("Updating the cached crates.io database dump");
    let crates = db_dump::download()?;
    let name_to_downloads: HashMap<_, _> = crates
        .iter()
        .map(|c| (c.name.clone(), c.recent_downloads))
        .collect();
    let mut output = Vec::new();
    for krate in crates.iter().cloned() {
        output.push((krate.name, krate.version));
    }
    let serialized = serde_json::to_string(&output).unwrap();

    client
        .put_object()
        .key("crates.json")
        .body(serialized.into_bytes().into())
        .content_type("application/json")
        .send()
        .await?;

    log::info!("Downloading, rendering, and uploading rendered HTML for all crates");
    let mut crates = sync_all_html(client.clone()).await?;

    // Sort crates by recent downloads, descending
    // Then by version, descending
    crates.sort_by(|crate_a, crate_b| {
        let a = name_to_downloads.get(&crate_a.name).cloned().flatten();
        let b = name_to_downloads.get(&crate_b.name).cloned().flatten();
        b.cmp(&a) //.then_with(|| crate_a.version.cmp(&crate_b.version))
    });
    // Since we sored by version we can dedup by name and be left with only
    // the most recent version of each crate.
    crates.dedup_by(|a, b| a.name == b.name);

    let ub_page = crate::render::render_ub(&crates)?;
    client
        .put_object()
        .key(format!("{}/ub", args.tool))
        .body(ub_page.into_bytes().into())
        .content_type("text/html")
        .send()
        .await?;

    Ok(())
}

async fn sync_all_html(client: Arc<Client>) -> Result<Vec<Crate>> {
    let all = client.list_finished_crates().await?;
    log::info!("Re-rendering HTML for {} crates", all.len());
    let mut tasks = JoinSet::new();
    let limit = Arc::new(Semaphore::new(256));
    let all_raw = Arc::new(Mutex::new(tar::Builder::new(xz2::write::XzEncoder::new(
        Vec::new(),
        5,
    ))));
    let all_rendered = Arc::new(Mutex::new(tar::Builder::new(xz2::write::XzEncoder::new(
        Vec::new(),
        5,
    ))));
    for krate in all {
        let limit = Arc::clone(&limit);
        let client = Arc::clone(&client);
        let all_raw = Arc::clone(&all_raw);
        let all_rendered = Arc::clone(&all_rendered);
        let permit = limit.acquire_owned().await.unwrap();
        tasks.spawn(async move {
            let raw = client.download_raw(&krate).await?;
            let mut header = tar::Header::new_gnu();
            if header
                .set_path(format!("raw/{}/{}", krate.name, krate.version))
                .is_ok()
            {
                header.set_size(raw.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                all_raw
                    .lock()
                    .await
                    .append(&header, raw.as_bytes())
                    .unwrap();
            }

            let rendered = render::render_crate(&krate, &raw);
            let mut header = tar::Header::new_gnu();
            if header
                .set_path(format!("html/{}/{}", krate.name, krate.version))
                .is_ok()
            {
                header.set_size(rendered.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                all_rendered
                    .lock()
                    .await
                    .append(&header, rendered.as_bytes())
                    .unwrap();
            }

            client.upload_html(&krate, rendered.into_bytes()).await?;
            // Ensure the permit is released once we are done with the client
            drop(permit);
            let mut krate = krate;
            crate::diagnose(&mut krate, &raw)?;
            Ok::<_, Report>(krate)
        });
    }
    let mut crates = Vec::new();
    while let Some(task) = tasks.join_next().await {
        let krate = task??;
        crates.push(krate);
    }

    let raw: Vec<u8> = Arc::into_inner(all_raw)
        .unwrap()
        .into_inner()
        .into_inner()
        .unwrap()
        .finish()
        .unwrap();
    client
        .put_object()
        .key(format!("{}/raw.tar.xz", client.tool()))
        .body(raw.into())
        .content_type("application/octet-stream")
        .send()
        .await?;

    let rendered: Vec<u8> = Arc::into_inner(all_rendered)
        .unwrap()
        .into_inner()
        .into_inner()
        .unwrap()
        .finish()
        .unwrap();
    client
        .put_object()
        .key(format!("{}/html.tar.xz", client.tool()))
        .body(rendered.into())
        .content_type("application/octet-stream")
        .send()
        .await?;

    Ok(crates)
}

async fn sync_landing_page(client: &Client) -> Result<()> {
    // List all rendered HTML
    let rendered = client.list_rendered_crates().await?;

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

static ERROR_PAGE: &str = r#"<!DOCTYPE HTML>
<html><head><style>
body {
    background: #111;
    color: #eee;
}
pre {
    word-wrap: break-word;
    white-space: pre-wrap;
    font-size: 14px;
    font-size-adjust: none;
    text-size-adjust: none;
    -webkit-text-size-adjust: 100%;
    -moz-text-size-adjust: 100%;
    -ms-text-size-adjust: 100%;
}
</style><title>oops</title></head>
<body><pre><span style='color:#f55; font-weight:bold'>error</span>: No such file or directory (http error 404)

<span style='color:#f55; font-weight:bold'>error</span>: aborting due to previous error</pre></body></html>"#;
