use clap::Parser;
use crates_io_api::{CratesQuery, Sort, SyncClient};
use regex::Regex;
use rustwide::{
    cmd::{Command, SandboxBuilder},
    logging, Toolchain, WorkspaceBuilder,
};
use std::{fs, path::PathBuf, time::Duration};

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

fn main() {
    let tag_re = Regex::new(r"<\d+>").unwrap();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    logging::init_with(env_logger::Logger::from_default_env());

    let args = Args::parse();

    let client = SyncClient::new(
        "miri (kimockb@gmail.com)",
        std::time::Duration::from_millis(1000),
    )
    .unwrap();

    let mut crates = Vec::new();

    let mut page = 1;
    while crates.len() < args.crates {
        let mut query = CratesQuery::builder()
            .sort(Sort::RecentDownloads)
            .page_size(100)
            .build();
        query.set_page(page);

        let response = client.crates(query).unwrap();

        crates.extend(
            response
                .crates
                .into_iter()
                .take(args.crates - crates.len())
                .map(|c| (c.name, c.max_version)),
        );
        page += 1;
    }

    let build_path = args
        .build_dir
        .unwrap_or_else(|| tempfile::tempdir().unwrap().into_path());

    let workspace = WorkspaceBuilder::new(&build_path, "miri").init().unwrap();

    workspace.purge_all_build_dirs().unwrap();
    workspace.purge_all_caches().unwrap();

    for tc in workspace.installed_toolchains().unwrap() {
        tc.uninstall(&workspace).unwrap();
    }

    let nightly = Toolchain::dist("nightly-2022-02-17");

    nightly.install(&workspace).unwrap();
    nightly.add_component(&workspace, "rust-src").unwrap();
    nightly.add_component(&workspace, "miri").unwrap();

    nightly
        .add_target(&workspace, "x86_64-unknown-linux-musl")
        .unwrap();
    Command::new(&workspace, nightly.cargo())
        .args(&["install", "xargo", "--target=x86_64-unknown-linux-musl"])
        .run()
        .unwrap();
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

    for (crate_name, crate_version) in crates {
        let current_crate = rustwide::Crate::crates_io(&crate_name, &crate_version);

        if current_crate.fetch(&workspace).is_err() {
            log::error!("Unable to fetch {}-{}", crate_name, crate_version);
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
            fs::write(
                format!("success/{}-{}", crate_name, crate_version),
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

        fs::write(
            format!("failure/{}-{}", crate_name, crate_version),
            output.as_bytes(),
        )
        .unwrap();
    }
}
