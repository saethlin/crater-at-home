use clap::Parser;
use color_eyre::eyre::{ensure, eyre, Context, ErrReport, Result};
use crates_io_api::{CratesQuery, Sort, SyncClient};
use miri_the_world::*;
use std::{
    collections::hash_map::Entry,
    collections::HashMap,
    fs,
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Parser)]
struct Args {
    #[clap(long, default_value_t = 10000)]
    crates: usize,

    #[clap(long, default_value_t = 8)]
    memory_limit_gb: usize,

    #[clap(long, default_value_t = 8)]
    jobs: usize,
}

fn main() -> Result<()> {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    color_eyre::install()?;

    let args = Args::parse();

    let mut crates = HashMap::new();

    if Path::new("crates.json").exists() {
        for line in fs::read_to_string("crates.json")
            .unwrap()
            .lines()
            .take(args.crates)
        {
            let krate: Crate = serde_json::from_str(&line)?;
            crates.insert(krate.name.clone(), krate);
        }
    }

    let client = SyncClient::new(
        "miri (kimockb@gmail.com)",
        std::time::Duration::from_millis(1000),
    )?;

    log::info!("Discovering crates...");
    let mut page = 1;
    while crates.len() < args.crates {
        let mut query = CratesQuery::builder()
            .sort(Sort::RecentDownloads)
            .page_size(100)
            .build();
        query.set_page(page);

        let response = client.crates(query)?;

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
                            time: None,
                        });
                    }
                }
                Entry::Vacant(v) => {
                    v.insert(Crate {
                        name: c.name,
                        recent_downloads: c.recent_downloads,
                        version: c.max_version,
                        status: Status::Unknown,
                        time: None,
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
            krate.recent_downloads = client.get_crate(&krate.name)?.crate_data.recent_downloads;
            assert!(krate.recent_downloads.is_some());
            log::info!("{} of {}", k, args.crates);
        }
    }

    let mut crates = crates.into_iter().map(|pair| pair.1).collect::<Vec<_>>();

    // Sort by recent downloads, descending
    crates.sort_by(|a, b| b.recent_downloads.cmp(&a.recent_downloads));

    fs::create_dir_all("logs")?;

    let mut previously_run = Vec::new();
    for name in fs::read_dir("logs")? {
        let name = name?;
        for file in fs::read_dir(name.path())? {
            let file = file?;
            let name = name.file_name().into_string().unwrap();
            let version = file.file_name().into_string().unwrap();
            if !version.ends_with(".html") {
                previously_run.push((name, version));
            }
        }
    }

    struct Cursor {
        crates: Vec<Crate>,
        next: usize,
    }

    let cursor = Arc::new(Mutex::new(Cursor { crates, next: 0 }));

    let mut threads = Vec::new();
    for _ in 0..args.jobs {
        let cursor = cursor.clone();
        let previously_run = previously_run.clone();
        let handle = std::thread::spawn(move || -> Result<()> {
            loop {
                let mut lock = cursor
                    .lock()
                    .map_err(|_| eyre!("the main thread panicked"))?;

                let i = lock.next;
                lock.next += 1;

                let mut krate = if let Some(krate) = lock.crates.get(i) {
                    krate.clone()
                } else {
                    break Ok(());
                };

                drop(lock);

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

                let start = std::time::Instant::now();

                let res = std::process::Command::new("docker")
                    .args(&[
                        "run",
                        "--rm",
                        "--tty",
                        "--env",
                        "RUSTFLAGS=-Zrandomize-layout",
                        "--env",
                        "RUST_BACKTRACE=0",
                        "--env",
                        miri_flags,
                        &format!("--memory={}g", args.memory_limit_gb),
                        "miri:latest",
                        &format!("{}=={}", krate.name, krate.version),
                    ])
                    .output()
                    .wrap_err("failed to execute docker")?;

                let end = std::time::Instant::now();
                krate.time = Some((end - start).as_secs());

                let output = String::from_utf8_lossy(&res.stdout);

                // The container is supposed to redirect everything to stdout
                ensure!(
                    res.stderr.is_empty(),
                    "{}",
                    String::from_utf8_lossy(&res.stderr)
                );

                fs::create_dir_all(format!("logs/{}", krate.name))?;
                fs::write(format!("logs/{}/{}", krate.name, krate.version), &*output)?;

                diagnose_crate(&mut krate)?;

                let mut lock = cursor
                    .lock()
                    .map_err(|_| eyre!("the main thread panicked"))?;
                lock.crates[i] = krate;
            }
        });
        threads.push(handle);
    }

    for t in threads {
        t.join()
            .map_err(|e| *e.downcast::<ErrReport>().unwrap())??;
    }

    log::info!("dumping info to `crates.json`");

    let crates = Arc::try_unwrap(cursor)
        .map_err(|_| eyre!("all threads joined, but Arc still shared?"))?
        .into_inner()
        .map_err(|_| eyre!("some thread panicked and poisoned our mutex"))?
        .crates;

    ensure!(
        !Path::new(".crates.json").exists(),
        "lock file already exists"
    );

    let mut file = fs::File::create(".crates.json")?;
    for krate in crates {
        serde_json::to_writer(&mut file, &krate)?;
        file.write(b"\n")?;
    }
    fs::copy("crates.json", "crates.json.bak")?;
    fs::copy(".crates.json", "crates.json")?;
    fs::remove_file(".crates.json")?;

    Ok(())
}

fn diagnose_crate(krate: &mut Crate) -> Result<()> {
    let path = format!("logs/{}/{}", krate.name, krate.version);
    if let Ok(output) = fs::read_to_string(&path) {
        krate.status = if output.contains("Undefined Behavior: ") {
            Status::UB {
                cause: diagnose(&output),
                status: String::new(),
            }
        } else if output.contains("Command exited with non-zero status 124") {
            Status::Error("Timeout".to_string())
        } else if output.contains("Command exited with non-zero status 255") {
            Status::Error("OOM".to_string())
        } else if output.contains("Command exited with non-zero status") {
            Status::Error(String::new())
        } else {
            Status::Passing
        };
    }
    Ok(())
}

fn diagnose(output: &str) -> Vec<Cause> {
    let mut causes = Vec::new();

    let lines = output.lines().collect::<Vec<_>>();

    for (l, line) in lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains("Undefined Behavior: "))
    {
        let end = lines
            .iter()
            .enumerate()
            .skip(l)
            .find_map(|(l, line)| {
                if line.trim().is_empty() {
                    Some(l)
                } else {
                    None
                }
            })
            .unwrap();

        let kind;
        if line.contains("uninitialized") {
            kind = "uninitialized memory".to_string();
        } else if line.contains("out-of-bounds") {
            kind = "invalid pointer offset".to_string();
        } else if line.contains("null pointer is not a valid pointer for this operation") {
            kind = "null pointer dereference".to_string();
        } else if line.contains("accessing memory with alignment") {
            kind = "misaligned pointer dereference".to_string();
        } else if line.contains("dangling reference") {
            kind = "dangling reference".to_string();
        } else if line.contains("unaligned reference") {
            kind = "unaligned reference".to_string();
        } else if line.contains("incorrect layout on deallocation") {
            kind = "incorrect layout on deallocation".to_string();
        } else if line.contains("borrow stack") || line.contains("reborrow") {
            if line.contains("<untagged>") {
                kind = "int-to-ptr cast".to_string();
            } else {
                kind = diagnose_sb(&lines[l..end]);
            }
        } else {
            kind = line
                .split("Undefined Behavior: ")
                .nth(1)
                .unwrap()
                .trim()
                .to_string();
        }

        let mut source_crate = None;

        for line in &lines[l..] {
            if line.contains("inside `") && line.contains(" at ") {
                let path = line.split(" at ").nth(1).unwrap();
                if path.contains("workdir") || !path.starts_with("/") {
                    break;
                } else if path.contains("/root/.cargo/registry/src/") {
                    let crate_name = path
                        .split("/root/.cargo/registry/src/github.com-1ecc6299db9ec823/")
                        .nth(1)
                        .unwrap()
                        .split("/")
                        .nth(0)
                        .unwrap();

                    source_crate = Some(format!("{}", crate_name));
                    break;
                }
            }
        }
        causes.push(Cause { kind, source_crate })
    }

    causes.sort();
    causes.dedup();
    causes
}

fn diagnose_sb(lines: &[&str]) -> String {
    if lines[0].contains("only grants SharedReadOnly") && lines[0].contains("for Unique") {
        String::from("&->&mut")
    } else if lines.iter().any(|line| line.contains("invalidated")) {
        String::from("SB-invalidation")
    } else if lines
        .iter()
        .any(|line| line.contains("created due to a retag at offsets [0x0..0x0]"))
    {
        String::from("SB-null-provenance")
    } else if lines[0].contains("does not exist in the borrow stack") {
        String::from("SB-use-outside-provenance")
    } else if lines[0].contains("no item granting write access for deallocation") {
        String::from("SB-invalid-dealloc")
    } else {
        String::from("SB-uncategorized")
    }
}
