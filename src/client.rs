use crate::{Crate, Status, Tool, Version};
use aws_sdk_s3::model::{CompletedMultipartUpload, CompletedPart, Object};
use aws_smithy_types_convert::date_time::DateTimeExt;
use backoff::Error;
use backoff::ExponentialBackoff;
use color_eyre::Result;
use futures_util::StreamExt;
use futures_util::TryFutureExt;
use std::collections::HashMap;
use std::future::Future;

#[derive(Clone)]
pub struct Client {
    inner: aws_sdk_s3::Client,
    bucket: String,
    tool: Tool,
}

const CHUNK_SIZE: usize = 5 * 1024 * 1024;

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

    pub async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<()> {
        retry(|| self._upload(key, data, content_type)).await
    }

    async fn _upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<()> {
        // S3 has a minimum multipart upload size of 5 MB. If we are below that, we need to use
        // PutObject.
        if data.len() < CHUNK_SIZE {
            self.inner
                .put_object()
                .bucket(&self.bucket)
                .key(key)
                .body(data.to_vec().into())
                .content_type(content_type)
                .send()
                .await?;
            return Ok(());
        }

        let res = self
            .inner
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .send()
            .await?;
        let upload_id = res.upload_id().unwrap();
        let mut parts = Vec::new();
        for (part_number, chunk) in data.chunks(CHUNK_SIZE).enumerate() {
            // part numbers must start at 1
            let part_number = part_number as i32 + 1;
            let upload_part_res = self
                .inner
                .upload_part()
                .key(key)
                .bucket(&self.bucket)
                .upload_id(upload_id)
                .body(chunk.to_vec().into())
                .part_number(part_number)
                .send()
                .await?;
            parts.push(
                CompletedPart::builder()
                    .e_tag(upload_part_res.e_tag.unwrap_or_default())
                    .part_number(part_number)
                    .build(),
            )
        }
        let completed_multipart_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(parts))
            .build();
        self.inner
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .multipart_upload(completed_multipart_upload)
            .upload_id(upload_id)
            .send()
            .await?;

        Ok(())
    }

    pub async fn upload_raw(&self, krate: &Crate, data: Vec<u8>) -> Result<()> {
        self.upload(&self.tool.raw_crate_path(krate), &data, "text/plain")
            .await
    }

    pub async fn upload_html(&self, krate: &Crate, data: Vec<u8>) -> Result<()> {
        let key = self.tool.rendered_crate_path(krate);
        self.upload(&key, &data, "text/html;charset=utf-8").await
    }

    pub async fn download_raw(&self, krate: &Crate) -> Result<Vec<u8>> {
        self.download(&self.tool.raw_crate_path(krate)).await
    }

    pub async fn download_html(&self, krate: &Crate) -> Result<Vec<u8>> {
        self.download(&self.tool.rendered_crate_path(krate)).await
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        retry(|| self._download(key)).await
    }

    async fn _download(&self, key: &str) -> Result<Vec<u8>> {
        let response = self
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        let bytes = response.body.collect().await?;
        Ok(bytes.to_vec())
    }

    pub async fn get_crate_downloads(&self) -> Result<HashMap<String, Option<u64>>> {
        let response = self
            .inner
            .get_object()
            .bucket(&self.bucket)
            .key("downloads.json")
            .send()
            .await?;
        let bytes = response.body.collect().await?;
        let blob = bytes.to_vec();
        let crates: HashMap<_, _> = serde_json::from_slice(&blob)?;
        Ok(crates)
    }

    pub async fn get_crate_versions(&self) -> Result<Vec<Crate>> {
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

    pub async fn list_finished_crates(&self, dur: Option<time::Duration>) -> Result<Vec<Crate>> {
        let now = time::OffsetDateTime::now_utc();
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
                if let Some(dur) = dur {
                    if let Some(modified) = obj.last_modified() {
                        let modified = modified.to_time().unwrap();
                        // Ignore crates older than dur
                        if now - modified > dur {
                            continue;
                        }
                    }
                }
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
        self.upload(
            self.tool.landing_page_path(),
            &data,
            "text/html;charset=utf-8",
        )
        .await
    }

    pub async fn list_db(&self) -> Result<Option<Object>> {
        let res = retry(move || {
            self.inner
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix("crates.json")
                .max_keys(1)
                .send()
        })
        .await?;
        let meta = res.contents.and_then(|c| c.first().cloned());
        Ok(meta)
    }
}

async fn retry<I, E, Func, Fut>(mut f: Func) -> std::result::Result<I, E>
where
    Func: FnMut() -> Fut,
    Fut: Future<Output = std::result::Result<I, E>>,
    E: std::fmt::Display,
{
    backoff::future::retry_notify(
        ExponentialBackoff::default(),
        || f().map_err(Error::transient),
        |e, _| {
            log::warn!("{}", e);
        },
    )
    .await
}
