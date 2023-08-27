use crate::{Crate, Status, Tool, Version};
use aws_sdk_s3::client::fluent_builders::PutObject;
use backoff::Error;
use backoff::ExponentialBackoff;
use color_eyre::Result;
use futures_util::StreamExt;
use futures_util::TryFutureExt;
use std::future::Future;

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

    pub fn tool(&self) -> Tool {
        self.tool
    }

    pub fn put_object(&self) -> PutObject {
        self.inner.put_object().bucket(&self.bucket)
    }

    pub async fn upload_raw(&self, krate: &Crate, data: Vec<u8>) -> Result<()> {
        retry(|| {
            self.put_object()
                .key(self.tool.raw_crate_path(krate))
                .body(data.clone().into())
                .content_type("text/plain")
                .send()
        })
        .await?;
        Ok(())
    }

    pub async fn download_raw(&self, krate: &Crate) -> Result<Vec<u8>> {
        retry(|| self._download_raw(krate)).await
    }

    async fn _download_raw(&self, krate: &Crate) -> Result<Vec<u8>> {
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
        Ok(bytes.to_vec())
    }

    pub async fn upload_html(&self, krate: &Crate, data: Vec<u8>) -> Result<()> {
        retry(move || {
            self.inner
                .put_object()
                .bucket(&self.bucket)
                .key(self.tool.rendered_crate_path(krate))
                .body(data.clone().into())
                .content_type("text/html")
                .send()
        })
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
                    let Some(name) = it.next() else {
                        continue;
                    };
                    let Some(version) = it.next() else {
                        continue;
                    };
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
        retry(move || {
            self.inner
                .put_object()
                .bucket(&self.bucket)
                .key(self.tool.landing_page_path())
                .body(data.clone().into())
                .content_type("text/html")
                .send()
        })
        .await?;
        Ok(())
    }
}

async fn retry<I, E, Func, Fut>(mut f: Func) -> std::result::Result<I, E>
where
    Func: FnMut() -> Fut,
    Fut: Future<Output = std::result::Result<I, E>>,
{
    backoff::future::retry(ExponentialBackoff::default(), || {
        f().map_err(Error::transient)
    })
    .await
}
