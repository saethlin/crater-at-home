use clap::Parser;
use rustwide::{
    cmd::{Command, SandboxBuilder},
    logging, Toolchain, WorkspaceBuilder,
};
use std::{fs, path::PathBuf, time::Duration};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    crate_list: PathBuf,

    #[clap(long)]
    build_dir: Option<PathBuf>,

    #[clap(long, default_value_t = 63)]
    memory_limit_gb: usize,

    #[clap(long, default_value_t = 15)]
    timeout_minutes: u64,
}

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    logging::init_with(env_logger::Logger::from_default_env());

    let args = Args::parse();

    let crate_list = std::fs::read_to_string(args.crate_list).unwrap();

    let build_path = args
        .build_dir
        .unwrap_or_else(|| tempfile::tempdir().unwrap().into_path());

    let workspace = WorkspaceBuilder::new(&build_path, "miri-the-world")
        .init()
        .unwrap();

    workspace.purge_all_build_dirs().unwrap();
    workspace.purge_all_caches().unwrap();

    for tc in workspace.installed_toolchains().unwrap() {
        tc.uninstall(&workspace).unwrap();
    }

    let nightly = Toolchain::dist("nightly-2022-01-27");

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

    let mut build_dir = workspace.build_dir("miri-the-world");

    fs::create_dir_all("success").unwrap();
    fs::create_dir_all("failure").unwrap();

    for line in crate_list.lines() {
        let mut it = line.split(' ');
        let crate_name = it.next().unwrap();
        let crate_version = it.next().unwrap();

        let current_crate = rustwide::Crate::crates_io(crate_name, crate_version);

        if current_crate.fetch(&workspace).is_err() {
            log::error!("Unable to fetch {}-{}", crate_name, crate_version);
            continue;
        }

        // If the crate packaged a lockfile, delete it.
        // This ensures that we build with the latest-permitted versions of dependencies, picking
        // up any patches that have been released since publication.
        let lockfile = build_path.join("builds/miri-the-world/source/Cargo.lock");
        if lockfile.exists() {
            log::warn!("Found packaged Cargo.lock, attempting to delete");
            fs::remove_file(lockfile).unwrap();
        }

        let storage = logging::LogStorage::new(log::LevelFilter::Debug);

        let res = logging::capture(&storage, || {
            build_dir
                .build(&nightly, &current_crate, sandbox.clone())
                .run(|build| {
                    build
                        .cargo()
                        .env("XARGO_CHECK", "/opt/rustwide/cargo-home/bin/xargo")
                        .env("XDG_CACHE_HOME", build_path.join("cache"))
                        .env("RUSTFLAGS", "-Zrandomize-layout")
                        .env("MIRIFLAGS", "-Zmiri-disable-isolation -Zmiri-ignore-leaks -Zmiri-check-number-validity -Zmiri-tag-raw-pointers")
                        .args(&["miri", "test", "--jobs=1", "--", "--test-threads=1"])
                        .timeout(Some(Duration::from_secs(60 * args.timeout_minutes)))
                        .run()?;
                    Ok(())
                })
        });
        let base = if res.is_ok() { "success" } else { "failure" };
        fs::write(
            format!("{}/{}-{}", base, crate_name, crate_version),
            storage.to_string().as_bytes(),
        )
        .unwrap();
    }
}
