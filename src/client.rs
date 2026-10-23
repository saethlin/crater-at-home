use crate::Store;
use crate::{Crate, Status, Tool, Version};
use backoff::Error;
use backoff::ExponentialBackoff;
use chrono::TimeDelta;
use chrono::Utc;
use color_eyre::Result;
use futures_util::StreamExt;
use futures_util::TryFutureExt;
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::path::Path;
use object_store::ObjectMeta;
use object_store::ObjectStore;
use object_store::ObjectStoreExt;
use object_store::WriteMultipart;
use std::collections::HashMap;
use std::fs;
use std::future::Future;

pub struct Client {
    inner: Box<dyn ObjectStore>,
    tool: Tool,
}

impl Client {
    pub async fn new(tool: Tool, store: &Store) -> Result<Self> {
        let inner: Box<dyn ObjectStore> = if let Some(bucket) = &store.bucket {
            Box::new(
                AmazonS3Builder::from_env()
                    .with_bucket_name(bucket)
                    .build()?,
            )
        } else {
            let storage_dir = store.storage_dir.as_ref().expect("validated by clap");
            fs::create_dir_all(storage_dir)?;
            Box::new(LocalFileSystem::new_with_prefix(storage_dir)?)
        };
        Ok(Self { inner, tool })
    }

    pub fn tool(&self) -> Tool {
        self.tool
    }

    pub async fn upload(&self, key: &str, data: &[u8]) -> Result<()> {
        retry(|| self._upload(key, data), Some(format!("upload of {key}"))).await
    }

    async fn _upload(&self, key: &str, data: &[u8]) -> Result<()> {
        let upload = self.inner.put_multipart(&Path::from(key)).await?;
        let mut write = WriteMultipart::new(upload);
        write.write(data);
        write.finish().await?;

        Ok(())
    }

    pub async fn upload_raw(&self, krate: &Crate, data: Vec<u8>) -> Result<()> {
        self.upload(&self.tool.raw_crate_path(krate), &data).await
    }

    pub async fn upload_html(&self, krate: &Crate, data: Vec<u8>) -> Result<()> {
        let key = self.tool.rendered_crate_path(krate);
        self.upload(&key, &data).await
    }

    pub async fn download_raw(&self, krate: &Crate) -> Result<Vec<u8>> {
        self.download(&self.tool.raw_crate_path(krate)).await
    }

    pub async fn download_html(&self, krate: &Crate) -> Result<Vec<u8>> {
        self.download(&self.tool.rendered_crate_path(krate)).await
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        retry(|| self._download(key), Some(format!("download of {key}"))).await
    }

    async fn _download(&self, key: &str) -> Result<Vec<u8>> {
        let response = self.inner.get(&Path::from(key)).await?;
        Ok(response.bytes().await?.to_vec())
    }

    pub async fn get_crate_downloads(&self) -> Result<HashMap<String, Option<u64>>> {
        let blob = self._download("downloads.json").await?;
        let crates: HashMap<_, _> = serde_json::from_slice(&blob)?;
        Ok(crates)
    }

    pub async fn get_crate_versions(&self) -> Result<Vec<Crate>> {
        let blob = self._download("crates.json").await?;
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

    pub async fn list_finished_crates(&self, dur: Option<TimeDelta>) -> Result<Vec<Crate>> {
        let now = Utc::now();
        let prefix = Path::from(format!("{}/", self.tool.raw_path()));
        let mut res = self.inner.list(Some(&prefix));
        let mut files = Vec::new();
        while let Some(res) = res.next().await {
            let obj = res?;
            if let Some(dur) = dur {
                // Ignore crates older than dur
                if now - obj.last_modified > dur {
                    continue;
                }
            }
            if let Some(key) = obj
                .location
                .as_ref()
                .strip_prefix(prefix.as_ref())
                .map(|location| location.trim_start_matches('/'))
            {
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
        Ok(files)
    }

    pub async fn list_rendered_crates(&self) -> Result<Vec<String>> {
        let prefix = Path::from(format!("{}/", self.tool.html_path()));
        let mut res = self.inner.list(Some(&prefix));
        let mut files = Vec::new();
        while let Some(res) = res.next().await {
            let obj = res?;
            if let Some(key) = obj
                .location
                .as_ref()
                .strip_prefix(prefix.as_ref())
                .map(|key| key.trim_start_matches('/'))
            {
                files.push(key.to_string());
            }
        }
        Ok(files)
    }

    pub async fn upload_landing_page(&self, data: Vec<u8>) -> Result<()> {
        self.upload(self.tool.landing_page_path(), &data).await
    }

    pub async fn list_db(&self) -> Result<Option<ObjectMeta>> {
        let res = retry(
            async move || self.inner.get(&Path::from("crates.json")).await,
            None,
        )
        .await;
        match res {
            Ok(res) => Ok(Some(res.meta)),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

async fn retry<I, E, Func, Fut>(mut f: Func, msg: Option<String>) -> std::result::Result<I, E>
where
    Func: FnMut() -> Fut,
    Fut: Future<Output = std::result::Result<I, E>>,
    E: std::fmt::Debug,
{
    backoff::future::retry_notify(
        ExponentialBackoff::default(),
        || f().map_err(Error::transient),
        |e, _| {
            if let Some(msg) = &msg {
                log::warn!("During {msg}, encountered {:?}", e);
            } else {
                log::warn!("{:?}", e);
            }
        },
    )
    .await
}
