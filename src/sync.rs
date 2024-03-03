use crate::{client::Client, db_dump, Crate, Tool, Status};
use aws_smithy_types_convert::date_time::DateTimeExt;
use clap::Parser;
use anyhow::{Error, Result};
use std::{collections::HashMap, fmt::Write, sync::Arc};
use tokio::{sync::Semaphore, task::JoinSet};

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

    log::info!("Re-analyzing all crates to build the /ub page");
    let mut crates = sync_all_html(client.clone()).await?;

    // Sort crates by recent downloads, descending
    // Then by version, descending
    crates.sort_by(|(crate_a, _), (crate_b, _)| {
        let a = name_to_downloads.get(&crate_a.name).cloned().flatten();
        let b = name_to_downloads.get(&crate_b.name).cloned().flatten();
        b.cmp(&a)
            .then_with(|| crate_b.version.cmp(&crate_a.version))
    });
    // Since we sored by version we can dedup by name and be left with only
    // the most recent version of each crate.
    crates.dedup_by(|(a, _), (b, _)| a.name == b.name);

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

async fn sync_all_html(client: Arc<Client>) -> Result<Vec<(Crate, Status)>> {
    let all = client.list_finished_crates(None).await?;
    let mut tasks = JoinSet::new();
    let limit = Arc::new(Semaphore::new(256));
    for krate in all {
        let limit = Arc::clone(&limit);
        let client = Arc::clone(&client);
        let permit = limit.acquire_owned().await.unwrap();
        tasks.spawn(async move {
            let raw = client.download_raw(&krate).await?;
            drop(permit);
            let status = crate::diagnose(&raw);
            Ok::<_, Error>((krate, status))
        });
    }
    let mut output = Vec::new();
    while let Some(task) = tasks.join_next().await {
        let result = task??;
        output.push(result);
    }

    Ok(output)
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
