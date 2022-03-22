use clap::Parser;
use crates_io_api::{CratesQuery, Sort, SyncClient};
use std::{
    collections::hash_map::Entry,
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::Path,
};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    crates: usize,

    #[clap(long, default_value_t = 63)]
    memory_limit_gb: usize,

    #[clap(long, default_value_t = 15)]
    timeout_minutes: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Crate {
    name: String,
    recent_downloads: Option<u64>,
    version: String,
    status: Status,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum Status {
    Unknown,
    Passing,
    Error(String),
    UB { cause: String, status: String },
}

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let args = Args::parse();

    let mut crates = HashMap::new();

    if Path::new("crates.json").exists() {
        for line in fs::read_to_string("crates.json").unwrap().lines() {
            let krate: Crate = serde_json::from_str(&line).unwrap();
            crates.insert(krate.name.clone(), krate);
        }
    }

    let client = SyncClient::new(
        "miri (kimockb@gmail.com)",
        std::time::Duration::from_millis(1000),
    )
    .unwrap();

    log::info!("Discovering crates...");
    let mut page = 1;
    while crates.len() < args.crates {
        let mut query = CratesQuery::builder()
            .sort(Sort::RecentDownloads)
            .page_size(100)
            .build();
        query.set_page(page);

        let response = client.crates(query).unwrap();

        for c in response.crates.into_iter().take(args.crates - crates.len()) {
            match crates.entry(c.name.clone()) {
                Entry::Occupied(mut o) => {
                    if o.get().version == c.max_version {
                        o.get_mut().recent_downloads = c.recent_downloads;
                    } else {
                        o.insert(Crate {
                            name: c.name,
                            recent_downloads: c.recent_downloads,
                            version: c.max_version,
                            status: Status::Unknown,
                        });
                    }
                }
                Entry::Vacant(v) => {
                    v.insert(Crate {
                        name: c.name,
                        recent_downloads: c.recent_downloads,
                        version: c.max_version,
                        status: Status::Unknown,
                    });
                }
            }
        }

        log::info!("{} of {}", crates.len(), args.crates);

        page += 1;
    }

    log::info!("Loading missing metadata...");
    for (k, krate) in crates.values_mut().enumerate() {
        if krate.recent_downloads.is_none() {
            krate.recent_downloads = client
                .get_crate(&krate.name)
                .unwrap()
                .crate_data
                .recent_downloads;
            assert!(krate.recent_downloads.is_some());
            log::info!("{} of {}", k, args.crates);
        }
    }

    let mut crates = crates.into_iter().map(|pair| pair.1).collect::<Vec<_>>();

    // Sort by recent downloads, descending
    crates.sort_by(|a, b| b.recent_downloads.cmp(&a.recent_downloads));

    fs::create_dir_all("logs").unwrap();

    let mut previously_run = Vec::new();
    for name in fs::read_dir("logs").unwrap() {
        let name = name.unwrap();
        for file in fs::read_dir(name.path()).unwrap() {
            let file = file.unwrap();
            let name = name.file_name().into_string().unwrap();
            let version = file.file_name().into_string().unwrap();
            if !version.ends_with(".html") {
                previously_run.push((name, version));
            }
        }
    }

    for i in 0..crates.len() {
        let krate = &mut crates[i];

        if previously_run
            .iter()
            .any(|(name, version)| &krate.name == name && &krate.version == version)
        {
            log::info!("Already ran {} {}", krate.name, krate.version);
            continue;
        }

        log::info!("Running {} {}", krate.name, krate.version);

        let miri_flags =
            "MIRIFLAGS=-Zmiri-disable-isolation -Zmiri-ignore-leaks -Zmiri-check-number-validity \
             -Zmiri-panic-on-unsupported -Zmiri-tag-raw-pointers";

        let container_id = std::process::Command::new("docker")
            .args(&[
                "create",
                "-e",
                "RUSTFLAGS=-Zrandomize-layout",
                "-e",
                "RUST_BACKTRACE=0",
                "-e",
                miri_flags,
                "miri:latest",
                &format!("{}=={}", krate.name, krate.version),
            ])
            .output()
            .unwrap()
            .stdout;
        let container_id = String::from_utf8(container_id).unwrap().trim().to_string();

        let mut build = std::process::Command::new("docker")
            .args(&["start", "-a", &container_id])
            .spawn()
            .unwrap();

        for _ in 0..(15 * 60) {
            if build.try_wait().unwrap().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        std::process::Command::new("docker")
            .args(&["stop", &container_id])
            .status()
            .unwrap();

        log::info!("{} {} completed", krate.name, krate.version);

        let res = std::process::Command::new("docker")
            .args(&["logs", &container_id])
            .output()
            .unwrap();

        let output = String::from_utf8_lossy(&res.stdout);

        assert!(res.stderr.is_empty()); // The container is supposed to redirect everything to stdout

        let status = build.wait().unwrap();
        if status.success() {
            krate.status = Status::Passing;
        } else if output.contains("Undefined Behavior: ") {
            krate.status = Status::UB {
                cause: String::new(), //diagnose(&output),
                status: String::new(),
            };
        } else {
            krate.status = Status::Error(String::new());
        }

        fs::create_dir_all(format!("logs/{}", krate.name)).unwrap();
        fs::write(format!("logs/{}/{}", krate.name, krate.version), &*output).unwrap();

        render(&crates);

        /*
        let tag_re = regex::Regex::new(r"<\d+>").unwrap();
        let tag = output
            .lines()
            .find(|line| line.contains("Undefined Behavior"))
            .and_then(|line| tag_re.find(line));

        let storage = logging::LogStorage::new(log::LevelFilter::Debug);

        if let Some(tag) = tag {
            let tag = tag.as_str();
            let tag = &tag[1..tag.len() - 1];
            let miri_flags = format!("{} -Zmiri-track-pointer-tag={}", miri_flags, tag);
        }
        */
    }
}

fn render(crates: &[Crate]) {
    write_output(crates);

    for krate in crates {
        let path = format!("logs/{}/{}", krate.name, krate.version);
        if let Ok(contents) = fs::read_to_string(&path) {
            write_crate_output(krate, &contents);
        }
    }
}

#[rustfmt::skip]
macro_rules! log_format {
    () => {
r#"<html><head><style>
pre {{
    word-wrap: break-word;
    white-space: pre-wrap;
    font-size-adjust: none;
}}
</style><title>{} {}</title></head><body><pre>
{}
</pre></body></html>"#
    }
}

fn write_crate_output(krate: &Crate, output: &str) {
    fs::create_dir_all(format!("logs/{}", krate.name)).unwrap();
    let mut file = File::create(format!("logs/{}/{}.html", krate.name, krate.version)).unwrap();
    write!(
        file,
        log_format!(),
        krate.name,
        krate.version,
        html_escape::encode_text(output.trim())
    )
    .unwrap();
}

const OUTPUT_HEADER: &str = r#"<html><head><style>
    body {
        background: #111;
        color: #eee;
        font-family: sans-serif;
        font-size: 18px;
    }
    a {
        color: #eee;
    }
    .row {
        display: flex;
        border-bottom: 1px solid #333;
        width: 40em;
        padding: 1em;
    }
    .crate {
        flex: 1;
        flex-basis: 50%;
    }
    .status {
        flex: 2;
        flex-basis: 50%;
    }
    .page {
        width: 40em;
        margin: auto;
    }
</style></head><body>
<div class="page">"#;

fn write_output(crates: &[Crate]) {
    let mut output = File::create(".crates.json").unwrap();

    for c in crates {
        writeln!(output, "{}", serde_json::to_string(c).unwrap()).unwrap();
    }

    fs::rename(".crates.json", "crates.json").unwrap();

    let mut output = File::create(".index.html").unwrap();
    writeln!(output, "{}", OUTPUT_HEADER).unwrap();
    for c in crates {
        write!(
            output,
            "<div class=\"row\"><div class=\"crate\"><a href=\"logs/{}/{}.html\">{} {}</a></div>",
            c.name, c.version, c.name, c.version
        )
        .unwrap();
        write!(output, "<div class=\"status\">").unwrap();
        match &c.status {
            Status::Unknown => write!(output, "Unknown"),
            Status::Passing => write!(output, "Passing"),
            Status::Error(_) => write!(output, "Error"),
            Status::UB { cause, .. } => write!(output, "UB: {}", cause),
        }
        .unwrap();
        writeln!(output, "</div></div>").unwrap();
    }

    fs::rename(".index.html", "index.html").unwrap();

    let mut output = File::create(".ub.html").unwrap();
    writeln!(output, "{}", OUTPUT_HEADER).unwrap();
    for c in crates {
        if let Status::UB { cause, .. } = &c.status {
            write!(
            output,
            "<div class=\"row\"><div class=\"crate\"><a href=\"logs/{}/{}.html\">{} {}</a></div>",
            c.name, c.version, c.name, c.version
        )
            .unwrap();
            write!(output, "<div class=\"status\">").unwrap();
            write!(output, "UB: {}", cause).unwrap();
            writeln!(output, "</div></div>").unwrap();
        }
    }

    fs::rename(".ub.html", "ub.html").unwrap();
}

/*
fn diagnose(output: &str) -> String {
    if output.contains("-Zmiri-track-pointer-tag") {
        return diagnose_sb(output);
    }

    let mut causes = Vec::new();

    let lines = output.lines().collect::<Vec<_>>();

    for (l, line) in lines
        .iter()
        .emumerate()
        .filter(|(_, line)| line.contains("Undefined Behavior: "))
    {
        if line.contains("uninitialized") {
            causes.push("uninitialized memory".to_string());
        } else if line.contains("out-of-bounds") {
            causes.push("invalid pointer offset".to_string());
        } else if line.contains("null pointer is not a valid pointer for this operation") {
            causes.push("null pointer dereference".to_string());
        } else if line.contains("accessing memory with alignment") {
            causes.push("misaligned pointer dereference".to_string());
        } else if line.contains("dangling reference") {
            causes.push("dangling reference".to_string());
        } else if line.contains("unaligned reference") {
            causes.push("unaligned reference".to_string());
        } else if line.contains("incorrect layout on deallocation") {
            causes.push("incorrect layout on deallocation".to_string());
        } else if line.contains("borrow stack") {
            if line.contains("<untagged>") {
                causes.push("int-to-ptr cast".to_string());
            } else {
                causes.push("SB".to_string());
            }
        } else {
            causes.push(line.split("Undefined Behavior: ").nth(1).unwrap().trim());
        }

        for line in &lines[l..] {
            if line.contains("note: inside ") && line.contains(" at ") {
                let path = line.split(" at ").nth(1).unwrap();
                if path.contains("workdir") || !path.starts_with("/") {
                    break;
                } else if path.contains("github") {
                    let last = causes.last().unwrap().to_string();
                    *causes.last_mut().unwrap() =
                        format!("{} ({})", last, path.split("/").nth(7).unwrap());
                    break;
                }
            }
        }
    }

    causes.sort();
    causes.dedup();
}
*/

/*
const CRATES_ROOT: &str = "https://static.crates.io/crates";

lazy_static::lazy_static! {
    static ref CLIENT: ureq::Agent = ureq::Agent::new();
}

use flate2::read::GzDecoder;
use tar::Archive;

impl Crate {
    fn fetch_url(&self) -> String {
        format!(
            "{0}/{1}/{1}-{2}.crate",
            CRATES_ROOT, self.name, self.version
        )
    }

    fn fetch(&self) -> Result<(), Box<dyn std::error::Error>> {
        std::env::set_current_dir("/")?;
        std::fs::remove_dir_all("/build")?;
        std::fs::create_dir_all("/build")?;
        std::env::set_current_dir("/build")?;

        let path = Path::new("/build");

        let body = CLIENT.get(&self.fetch_url()).call()?.into_reader();
        let mut archive = Archive::new(GzDecoder::new(body));

        let entries = archive.entries()?;
        for entry in entries {
            let mut entry = entry?;
            let relpath = {
                let path = entry.path()?;
                path.into_owned()
            };
            let mut components = relpath.components();
            // Throw away the first path component
            components.next();
            let full_path = path.join(&components.as_path());
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            entry.unpack(&full_path)?;
        }
        Ok(())
    }
}
*/
