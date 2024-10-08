use crate::{client::Client, db_dump, render, Crate, Tool};
use aws_smithy_types_convert::date_time::DateTimeExt;
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
        .upload(
            &format!("{}/403", args.tool),
            ERROR_PAGE.as_bytes(),
            "text/html",
        )
        .await?;

    let should_refresh_db = client
        .list_db()
        .await?
        .map(|db| {
            let now = time::OffsetDateTime::now_utc();
            let modified = db.last_modified.unwrap().to_time().unwrap();
            (now - modified).whole_hours() > 24
        })
        .unwrap_or(true);

    let name_to_downloads = if should_refresh_db {
        log::info!("Updating the cached crates.io database dump");
        let crates = db_dump::download()?;
        let mut name_to_downloads = HashMap::new();
        let mut versions = Vec::new();
        for krate in crates.iter() {
            name_to_downloads.insert(krate.name.clone(), krate.recent_downloads);
            versions.push((krate.name.clone(), krate.version.to_string()));
        }

        let serialized = serde_json::to_string(&versions).unwrap();
        client
            .upload("crates.json", serialized.as_bytes(), "application/json")
            .await?;
        let serialized = serde_json::to_string(&name_to_downloads).unwrap();
        client
            .upload("downloads.json", serialized.as_bytes(), "application/json")
            .await?;
        name_to_downloads
    } else {
        client.get_crate_downloads().await?
    };

    log::info!("Downloading, rendering, and uploading rendered HTML for all crates");
    let mut crates = sync_all_html(client.clone()).await?;

    // Sort crates by recent downloads, descending
    // Then by version, descending
    crates.sort_by(|crate_a, crate_b| {
        let a = name_to_downloads.get(&crate_a.name).cloned().flatten();
        let b = name_to_downloads.get(&crate_b.name).cloned().flatten();
        b.cmp(&a)
            .then_with(|| crate_b.version.cmp(&crate_a.version))
    });
    // Since we sored by version we can dedup by name and be left with only
    // the most recent version of each crate.
    crates.dedup_by(|a, b| a.name == b.name);

    let ub_page = crate::render::render_ub(&crates)?;
    client
        .upload(
            &format!("{}/ub", args.tool),
            ub_page.as_bytes(),
            "text/html",
        )
        .await?;

    Ok(())
}

async fn sync_all_html(client: Arc<Client>) -> Result<Vec<Crate>> {
    log::info!("Enumerating all finished crates");
    let all = client.list_finished_crates(None).await?;
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
    for krate in all.into_iter().rev() {
        let limit = Arc::clone(&limit);
        let client = Arc::clone(&client);
        //let all_raw = Arc::clone(&all_raw);
        //let all_rendered = Arc::clone(&all_rendered);
        let permit = limit.acquire_owned().await.unwrap();
        tasks.spawn(async move {
            let raw = client.download_raw(&krate).await?;
            /*
            let mut header = tar::Header::new_gnu();
            if header
                .set_path(format!("raw/{}/{}", krate.name, krate.version))
                .is_ok()
            {
                header.set_size(raw.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                all_raw.lock().await.append(&header, &*raw).unwrap();
            }
            */

            let rendered = render::render_crate(&krate, &raw);
            /*
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
            */

            let previous = client.download_html(&krate).await;
            if let Ok(previous) = previous {
                if previous != rendered.as_bytes() {
                    log::info!("Uploading {}@{}", krate.name, krate.version);
                    client.upload_html(&krate, rendered.into_bytes()).await?;
                }
            } else {
                log::info!("Uploading {}@{}", krate.name, krate.version);
                client.upload_html(&krate, rendered.into_bytes()).await?;
            }
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
        .upload(
            &format!("{}/raw.tar.xz", client.tool()),
            &raw,
            "application/octet-stream",
        )
        .await?;

    let rendered: Vec<u8> = Arc::into_inner(all_rendered)
        .unwrap()
        .into_inner()
        .into_inner()
        .unwrap()
        .finish()
        .unwrap();
    client
        .upload(
            &format!("{}/html.tar.xz", client.tool()),
            &rendered,
            "application/octet-stream",
        )
        .await?;

    Ok(crates)
}

async fn sync_landing_page(client: &Client) -> Result<()> {
    // List all rendered HTML
    let rendered = client.list_rendered_crates().await?;

    let mut output = String::from(crate::render::LANDING_PAGE);
    for c in &rendered {
        let mut it = c.splitn(2, '/');
        let Some(name) = it.next() else {
            continue;
        };
        let Some(version) = it.next() else {
            continue;
        };
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
