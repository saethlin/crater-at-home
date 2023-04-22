use aws_sdk_s3::types::ByteStream;
use clap::Parser;
use color_eyre::{Report, Result};
use futures_util::stream::StreamExt;
use std::{collections::HashMap, fs, sync::Arc, time::SystemTime};
use tokio::runtime::Runtime;
use tokio::task::JoinSet;

#[derive(Parser)]
pub struct Args {
    #[clap(long)]
    dry_run: bool,

    #[clap(long)]
    bucket: String,
}

pub struct Client {
    runtime: Runtime,
    inner: aws_sdk_s3::Client,
    bucket: String,
}

impl Client {
    pub fn new(args: &crate::run::Args) -> Result<Self> {
        let rt = Runtime::new()?;
        let config = rt.block_on(aws_config::load_from_env());
        let inner = aws_sdk_s3::Client::new(&config);
        Ok(Self {
            runtime: rt,
            inner,
            bucket: args.bucket.clone(),
        })
    }

    pub fn upload_raw(&self, path: &str, data: Vec<u8>) -> Result<()> {
        let fut = self
            .inner
            .put_object()
            .bucket(&self.bucket)
            .key(path)
            .body(data.into())
            .content_type("application/octet-stream")
            .send();
        self.runtime.block_on(fut)?;
        Ok(())
    }

    pub fn upload_html(&self, path: &str, data: Vec<u8>) -> Result<()> {
        let fut = self
            .inner
            .put_object()
            .bucket(&self.bucket)
            .key(path)
            .body(data.into())
            .content_type("text/html")
            .send();
        self.runtime.block_on(fut)?;
        Ok(())
    }
}

#[tokio::main]
pub async fn run(args: Args) -> Result<()> {
    let args = Arc::new(args);

    let mut tasks = JoinSet::new();
    let config = aws_config::load_from_env().await;
    let client = Arc::new(aws_sdk_s3::Client::new(&config));

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
