use crate::{Crate, Status, Version, Tool};
use color_eyre::{Report, Result};
use tokio::runtime::Runtime;
use futures_util::StreamExt;

pub struct Client {
    runtime: Runtime,
    inner: aws_sdk_s3::Client,
    bucket: String,
    tool: Tool,
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
            tool: args.tool,
        })
    }

    pub fn upload_raw(&self, path: &str, data: Vec<u8>) -> Result<()> {
        let fut = self
            .inner
            .put_object()
            .bucket(&self.bucket)
            .key(path)
            .body(data.into())
            .content_type("text/plain")
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

    pub fn get_crate_list(&self) -> Result<Vec<Crate>> {
        let fut = self
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key("crates.json")
            .send();
        let blob = self.runtime.block_on(async {
            let bytes = fut.await?.body.collect().await?;
            Ok::<_, Report>(bytes.to_vec())
        })?;
        let crates: Vec<(String, String)> = serde_json::from_slice(&blob)?;
        let crates = crates
            .into_iter()
            .map(|krate| Crate {
                name: krate.0,
                version: Version::parse(&krate.1),
                status: Status::Unknown,
                recent_downloads: None,
            })
            .collect();
        Ok(crates)
    }

    pub fn get_finished_crates(&self) -> Result<Vec<String>> {
        let prefix = format!("{}/", self.tool.raw_path());
        self.runtime.block_on(async {
            let mut res = self.inner
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&prefix)
                .into_paginator()
                .send();
            let mut files = Vec::new();
            while let Some(res) = res.next().await {
                let page = res?;
                for obj in page.contents().unwrap_or_default() {
                    if let Some(key) = obj.key().and_then(|key| key.strip_prefix(&prefix)) {
                        files.push(key.to_string());
                    }
                }
            }
            Ok::<_, color_eyre::Report>(files)
        })
    }
}

/*
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
*/
