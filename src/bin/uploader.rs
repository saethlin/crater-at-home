use aws_sdk_s3::{types::ByteStream, Client};
use clap::Parser;
use color_eyre::Report;
use futures_util::stream::StreamExt;
use std::{collections::HashMap, fs, sync::Arc, time::SystemTime};
use tokio::task::JoinSet;

#[derive(Parser)]
struct Args {
    #[clap(long)]
    dry_run: bool,

    #[clap(long)]
    bucket: String,
}

#[tokio::main]
async fn main() -> Result<(), Report> {
    let args = Arc::new(Args::parse());

    let mut tasks = JoinSet::new();
    let config = aws_config::load_from_env().await;
    let client = Arc::new(Client::new(&config));

    let mut res = client
        .list_objects_v2()
        .bucket(&args.bucket)
        .into_paginator()
        .send();

    if !args.dry_run {
        for path in ["index.html", "ub", "all.html"] {
            let args = args.clone();
            let client = client.clone();
            tasks.spawn(async move {
                client
                    .put_object()
                    .bucket(&args.bucket)
                    .key(path)
                    .body(ByteStream::from_path(path).await?)
                    .content_type("text/html")
                    .send()
                    .await?;
                Ok::<(), Report>(())
            });
        }
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

            if args.dry_run {
                println!("Would upload {}", path);
            } else {
                println!("Uploading {}", path);

                if tasks.len() >= 256 {
                    tasks.join_next().await.unwrap()??;
                }

                let client = client.clone();
                let args = args.clone();
                tasks.spawn(async move {
                    client
                        .put_object()
                        .bucket(&args.bucket)
                        .key(&path)
                        .body(ByteStream::from_path(&path).await?)
                        .content_type("text/html")
                        .send()
                        .await?;
                    Ok::<(), Report>(())
                });
            }
        }
    }

    while let Some(task) = tasks.join_next().await {
        task??;
    }

    Ok(())
}
