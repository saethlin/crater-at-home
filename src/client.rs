use crate::{Crate, Status, Tool, Version};
use color_eyre::Result;
use futures_util::StreamExt;

#[derive(Clone)]
pub struct Client {
    inner: aws_sdk_s3::Client,
    bucket: String,
    tool: Tool,
}

impl Client {
    pub async fn new(tool: Tool, bucket: &str) -> Result<Self> {
        let config = aws_config::load_from_env().await;
        let inner = aws_sdk_s3::Client::new(&config);
        Ok(Self {
            inner,
            bucket: bucket.to_string(),
            tool,
        })
    }

    pub async fn upload_raw(&self, krate: &Crate, data: Vec<u8>) -> Result<()> {
        self.inner
            .put_object()
            .bucket(&self.bucket)
            .key(self.tool.raw_crate_path(krate))
            .body(data.into())
            .content_type("text/plain")
            .send()
            .await?;
        Ok(())
    }

    pub async fn download_raw(&self, krate: &Crate) -> Result<String> {
        let response = self
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key(format!(
                "{}/{}/{}",
                self.tool.raw_path(),
                krate.name,
                krate.version
            ))
            .send()
            .await?;
        let bytes = response.body.collect().await?;
        let blob = bytes.to_vec();
        Ok(String::from_utf8(blob).unwrap())
    }

    pub async fn upload_html(&self, krate: &Crate, data: Vec<u8>) -> Result<()> {
        self.inner
            .put_object()
            .bucket(&self.bucket)
            .key(self.tool.rendered_crate_path(krate))
            .body(data.into())
            .content_type("text/html")
            .send()
            .await?;
        Ok(())
    }

    pub async fn get_crate_list(&self) -> Result<Vec<Crate>> {
        let response = self
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key("crates.json")
            .send()
            .await?;
        let bytes = response.body.collect().await?;
        let blob = bytes.to_vec();
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

    pub async fn list_finished_crates(&self) -> Result<Vec<Crate>> {
        let prefix = format!("{}/", self.tool.raw_path());
        let mut res = self
            .inner
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
                    let mut it = key.split('/');
                    let Some(name) = it.next() else { continue; };
                    let Some(version) = it.next() else { continue; };
                    files.push(Crate {
                        name: name.to_string(),
                        version: Version::parse(version),
                        status: Status::Unknown,
                        recent_downloads: None,
                    });
                }
            }
        }
        Ok(files)
    }

    pub async fn list_rendered_crates(&self) -> Result<Vec<String>> {
        let prefix = format!("{}/", self.tool.html_path());
        let mut res = self
            .inner
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
        Ok(files)
    }

    pub async fn upload_landing_page(&self, data: Vec<u8>) -> Result<()> {
        self.inner
            .put_object()
            .bucket(&self.bucket)
            .key(self.tool.landing_page_path())
            .body(data.into())
            .content_type("text/html")
            .send()
            .await?;
        Ok(())
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
