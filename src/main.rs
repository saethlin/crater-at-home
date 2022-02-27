use clap::Parser;
use crates_io_api::{CratesQuery, Sort, SyncClient};
use regex::Regex;
use rustwide::{
    cmd::{Command, SandboxBuilder},
    logging, Toolchain, WorkspaceBuilder,
};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    crates: usize,

    #[clap(long)]
    build_dir: Option<PathBuf>,

    #[clap(long, default_value_t = 63)]
    memory_limit_gb: usize,

    #[clap(long, default_value_t = 15)]
    timeout_minutes: u64,
}

#[derive(Debug)]
struct Krate {
    name: String,
    recent_downloads: Option<u64>,
    version: String,
    cause: String,
    status: String,
}

fn main() {
    let tag_re = Regex::new(r"<\d+>").unwrap();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    logging::init_with(env_logger::Logger::from_default_env());

    let args = Args::parse();

    let mut crates = HashMap::new();

    for line in std::fs::read_to_string("README.md")
        .unwrap()
        .lines()
        .filter(|line| line.bytes().filter(|b| *b == b'|').count() == 4)
        .skip(2)
    {
        let line = line.trim();
        let mut it = line.split('|').skip(1);
        let n_it = it.next().unwrap().trim();
        let name = n_it.split(' ').nth(0).unwrap().to_string();
        let version = n_it.split(' ').nth(1).unwrap().to_string();
        let cause = it.next().unwrap().trim().to_string();
        let status = it.next().unwrap().trim().to_string();

        crates.insert(
            name.clone(),
            Krate {
                name,
                recent_downloads: None,
                version,
                cause,
                status,
            },
        );
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
            crates.insert(
                c.name.clone(),
                Krate {
                    name: c.name,
                    recent_downloads: c.recent_downloads,
                    version: c.max_version,
                    cause: String::new(),
                    status: String::new(),
                },
            );
        }

        log::info!("{} of {}", crates.len(), args.crates);

        page += 1;
    }

    log::info!("Loading missing metadaata...");
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

    let build_path = args
        .build_dir
        .unwrap_or_else(|| tempfile::tempdir().unwrap().into_path());

    let workspace = WorkspaceBuilder::new(&build_path, "miri").init().unwrap();

    workspace.purge_all_build_dirs().unwrap();
    workspace.purge_all_caches().unwrap();

    for tc in workspace.installed_toolchains().unwrap() {
        tc.uninstall(&workspace).unwrap();
    }

    let nightly = Toolchain::dist("nightly-2022-02-23");

    nightly.install(&workspace).unwrap();
    nightly.add_component(&workspace, "rust-src").unwrap();
    nightly.add_component(&workspace, "miri").unwrap();

    if !Path::exists(&build_path.join("cargo-home/bin/cargo/xargo")) {
        nightly
            .add_target(&workspace, "x86_64-unknown-linux-musl")
            .unwrap();
        Command::new(&workspace, nightly.cargo())
            .args(&["install", "xargo", "--target=x86_64-unknown-linux-musl"])
            .run()
            .unwrap();
    }

    Command::new(&workspace, nightly.cargo())
        .env("CARGO", build_path.join("cargo-home/bin/cargo"))
        .env("CARGO_HOME", build_path.join("cargo-home"))
        .env("XDG_CACHE_HOME", build_path.join("cache"))
        .args(&["miri", "setup"])
        .run()
        .unwrap();

    let sandbox = SandboxBuilder::new()
        .memory_limit(Some(1024 * 1024 * 1024 * args.memory_limit_gb))
        .cpu_limit(None)
        .enable_networking(true);

    let mut build_dir = workspace.build_dir("miri");

    fs::create_dir_all("success").unwrap();
    fs::create_dir_all("failure").unwrap();

    let mut previously_run = Vec::new();
    for name in fs::read_dir("success")
        .unwrap()
        .chain(fs::read_dir("failure").unwrap())
    {
        let name = name.unwrap();
        for file in fs::read_dir(name.path()).unwrap() {
            let file = file.unwrap();
            let name = name.file_name().into_string().unwrap();
            let version = file.file_name().into_string().unwrap();
            previously_run.push((name, version));
        }
    }

    let mut out = File::create("output").unwrap();

    for krate in crates.iter_mut() {
        if previously_run
            .iter()
            .any(|(name, version)| &krate.name == name && &krate.version == version)
        {
            log::info!("Already ran {} {}", krate.name, krate.version);
            continue;
        }

        // Clean the build dirs often, they grow constantly and will eventually run the system out
        // of disk space. The cached downloads do not grow large enough to be worth dumping
        // entirely.
        workspace.purge_all_build_dirs().unwrap();

        let current_crate = rustwide::Crate::crates_io(&krate.name, &krate.version);

        if current_crate.fetch(&workspace).is_err() {
            log::error!("Unable to fetch {} {}", krate.name, krate.version);
            continue;
        }

        let storage = logging::LogStorage::new(log::LevelFilter::Debug);

        let miri_flags =
            "-Zmiri-disable-isolation -Zmiri-ignore-leaks -Zmiri-check-number-validity \
            -Zmiri-tag-raw-pointers -Zmiri-panic-on-unsupported";

        let res = logging::capture(&storage, || {
            build_dir
                .build(&nightly, &current_crate, sandbox.clone())
                .run(|build| {
                    build
                        .cargo()
                        .env("XARGO_CHECK", "/opt/rustwide/cargo-home/bin/xargo")
                        .env("XDG_CACHE_HOME", build_path.join("cache"))
                        .env("RUSTFLAGS", "-Zrandomize-layout")
                        .env("RUST_BACKTRACE", "0")
                        .env("MIRIFLAGS", miri_flags)
                        .args(&[
                            "miri",
                            "test",
                            "--jobs=1",
                            "--no-fail-fast",
                            "--",
                            "--test-threads=1",
                        ])
                        .timeout(Some(Duration::from_secs(60 * args.timeout_minutes)))
                        .run()?;
                    Ok(())
                })
        });
        let mut output = storage.to_string();
        if res.is_ok() {
            fs::create_dir_all(format!("success/{}", krate.name)).unwrap();
            fs::write(
                format!("success/{}/{}", krate.name, krate.version),
                output.as_bytes(),
            )
            .unwrap();
            continue;
        }

        let tag = output
            .lines()
            .find(|line| line.contains("Undefined Behavior"))
            .and_then(|line| tag_re.find(line));

        let storage = logging::LogStorage::new(log::LevelFilter::Debug);

        if let Some(tag) = tag {
            let tag = tag.as_str();
            let tag = &tag[1..tag.len() - 1];
            let miri_flags = format!("{} -Zmiri-track-pointer-tag={}", miri_flags, tag);
            let res = logging::capture(&storage, || {
                build_dir
                    .build(&nightly, &current_crate, sandbox.clone())
                    .run(|build| {
                        build
                            .cargo()
                            .env("XARGO_CHECK", "/opt/rustwide/cargo-home/bin/xargo")
                            .env("XDG_CACHE_HOME", build_path.join("cache"))
                            .env("RUSTFLAGS", "-Zrandomize-layout")
                            .env("RUST_BACKTRACE", "0")
                            .env("MIRIFLAGS", miri_flags)
                            .args(&[
                                "miri",
                                "test",
                                "--jobs=1",
                                "--no-fail-fast",
                                "--",
                                "--test-threads=1",
                            ])
                            .timeout(Some(Duration::from_secs(60 * args.timeout_minutes)))
                            .run()?;
                        Ok(())
                    })
            });
            assert!(!res.is_ok()); // Assert that we failed with the tag tracking on
            output = storage.to_string();
        }

        fs::create_dir_all(format!("failure/{}", krate.name)).unwrap();
        fs::write(
            format!("failure/{}/{}", krate.name, krate.version),
            output.as_bytes(),
        )
        .unwrap();

        if output.contains("Undefined Behavior") {
            out.write_all(
                format!(
                    "| {} | {} | {} | {} |\n",
                    krate.name, krate.version, krate.cause, krate.status
                )
                .as_bytes(),
            )
            .unwrap();
            out.flush().unwrap();
        }
    }
}
