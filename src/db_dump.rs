use crate::{Crate, Status, Version};
use color_eyre::Result;
use flate2::read::GzDecoder;
use fxhash::FxHashMap;
use serde::Deserialize;
use std::{collections::hash_map::Entry, io::Read};
use tar::Archive;

struct PublishedCrate {
    crate_id: u64,
    recent_downloads: u64,
    version: Version,
}

pub fn download() -> Result<Vec<crate::Crate>> {
    log::info!("Downloading crate database");

    let mut archive = Vec::new();
    GzDecoder::new(
        ureq::get("https://static.crates.io/db-dump.tar.gz")
            .call()?
            .into_reader(),
    )
    .read_to_end(&mut archive)?;

    log::info!("Processing crate database");

    let mut tar = Archive::new(archive.as_slice());

    let mut version_to_downloads = FxHashMap::default();
    let mut version_to_crate = FxHashMap::default();
    let mut num_to_name = FxHashMap::default();

    for entry in tar.entries()? {
        let entry = entry?;
        let path = entry.path()?;
        let mut components = path.components();
        components.next(); // The first element of the path is the date

        if components.as_path().to_str() == Some("data/version_downloads.csv") {
            version_to_downloads = decode_downloads(
                &archive[entry.raw_file_position() as usize..][..entry.size() as usize],
            )?;
        }
        if components.as_path().to_str() == Some("data/versions.csv") {
            version_to_crate = decode_versions(
                &archive[entry.raw_file_position() as usize..][..entry.size() as usize],
            )?;
        }
        if components.as_path().to_str() == Some("data/crates.csv") {
            num_to_name = decode_crates(
                &archive[entry.raw_file_position() as usize..][..entry.size() as usize],
            )?;
        }
    }

    // Aggregate download statistics by crate
    let mut crate_to_downloads: FxHashMap<u64, PublishedCrate> = FxHashMap::default();

    for (version_id, mut krate) in version_to_crate {
        if let Some(downloads) = version_to_downloads.get(&version_id) {
            match crate_to_downloads.entry(krate.crate_id) {
                Entry::Vacant(v) => {
                    krate.recent_downloads += downloads;
                    v.insert(krate);
                }
                Entry::Occupied(mut v) => {
                    let existing = v.get_mut();
                    existing.recent_downloads += downloads;
                    if krate.version > existing.version {
                        existing.version = krate.version;
                    }
                }
            }
        }
    }

    // Sort by downloads
    let mut crates = crate_to_downloads
        .into_iter()
        .filter_map(|(_id, krate)| {
            num_to_name.get(&krate.crate_id).map(|name| Crate {
                name: name.clone(),
                recent_downloads: Some(krate.recent_downloads),
                version: krate.version,
                status: Status::Unknown,
                time: None,
            })
        })
        .collect::<Vec<_>>();
    crates.sort_by(|a, b| b.recent_downloads.cmp(&a.recent_downloads));
    Ok(crates)
}

#[derive(Deserialize)]
struct DownloadsRecord {
    #[serde(skip)]
    _date: String,
    downloads: u64,
    version_id: u64,
}

fn decode_downloads(csv: &[u8]) -> Result<FxHashMap<u64, u64>> {
    let mut downloads = FxHashMap::default();

    let mut reader = csv::Reader::from_reader(csv);
    for record in reader.deserialize::<DownloadsRecord>() {
        let record = record?;
        *downloads.entry(record.version_id).or_default() += record.downloads;
    }

    Ok(downloads)
}

#[derive(Deserialize)]
struct VersionsRecord {
    crate_id: u64,
    #[serde(skip)]
    _crate_size: u64,
    #[serde(skip)]
    _created_at: String,
    #[serde(skip)]
    _downloads: u64,
    #[serde(skip)]
    _features: String,
    id: u64,
    #[serde(skip)]
    _license: String,
    num: String, // crate version
    #[serde(skip)]
    _published_by: String,
    #[serde(skip)]
    _updated_at: String,
    #[serde(skip)]
    _yanked: String,
}

fn decode_versions(csv: &[u8]) -> Result<FxHashMap<u64, PublishedCrate>> {
    let mut map = FxHashMap::default();

    let mut reader = csv::Reader::from_reader(csv);
    for record in reader.deserialize::<VersionsRecord>() {
        let record = record?;
        map.insert(
            record.id,
            PublishedCrate {
                crate_id: record.crate_id,
                recent_downloads: 0,
                version: Version::parse(&record.num),
            },
        );
    }

    Ok(map)
}

#[derive(Deserialize)]
struct CratesRecord {
    #[serde(skip)]
    _created_at: String,
    #[serde(skip)]
    _description: String,
    #[serde(skip)]
    _documentation: String,
    #[serde(skip)]
    _downloads: u64,
    id: u64,
    #[serde(skip)]
    _max_upload_size: u64,
    name: String,
    #[serde(skip)]
    _readme: String,
    #[serde(skip)]
    _repository: String,
    #[serde(skip)]
    _updated_at: String,
}

fn decode_crates(csv: &[u8]) -> Result<FxHashMap<u64, String>> {
    let mut map = FxHashMap::default();

    let mut reader = csv::Reader::from_reader(csv);
    for record in reader.deserialize::<CratesRecord>() {
        let record = record?;
        map.insert(record.id, record.name);
    }
    Ok(map)
}
