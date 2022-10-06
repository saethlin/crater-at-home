use aws_sdk_s3::{types::ByteStream, Client};
use color_eyre::Report;
use futures_util::stream::StreamExt;
use std::{collections::HashMap, fs, sync::Arc, time::SystemTime};

#[tokio::main]
async fn main() -> Result<(), Report> {
    let mut tasks = Vec::new();
    let config = aws_config::load_from_env().await;
    let client = Arc::new(Client::new(&config));

    let mut res = client
        .list_objects_v2()
        .bucket("miri-runs")
        .into_paginator()
        .send();

    for path in ["index.html", "ub", "all.html"] {
        let client = client.clone();
        tasks.push(tokio::spawn(async move {
            client
                .put_object()
                .bucket("miri-runs")
                .key(path)
                .body(ByteStream::from_path(path).await?)
                .content_type("text/html")
                .send()
                .await?;
            Ok::<(), Report>(())
        }));
    }

    let mut bucket_files = HashMap::new();
    while let Some(res) = res.next().await {
        let page = res?;
        for obj in page.contents().unwrap_or_default() {
            if let (Some(key), Some(modified)) = (obj.key(), obj.last_modified()) {
                bucket_files.insert(key.to_string(), SystemTime::try_from(*modified).unwrap());
            }
        }
    }

    for entry in fs::read_dir("logs")? {
        let entry = entry?;
        for entry in fs::read_dir(entry.path())? {
            let entry = entry?;
            let path = entry.path();
            let path = path.to_str().unwrap().to_string();
            if !path.ends_with(".html") {
                continue;
            }
            let disk_modified = entry.metadata()?.modified()?;

            if let Some(bucket_modified) = bucket_files.get(&path) {
                if bucket_modified > &disk_modified {
                    continue;
                }
            }

            println!("Uploading {}", path);
            let client = client.clone();
            tasks.push(tokio::spawn(async move {
                client
                    .put_object()
                    .bucket("miri-runs")
                    .key(&path)
                    .body(ByteStream::from_path(&path).await?)
                    .content_type("text/html")
                    .send()
                    .await?;
                Ok::<(), Report>(())
            }));
        }
    }

    for t in tasks {
        t.await??;
    }

    Ok(())
}
