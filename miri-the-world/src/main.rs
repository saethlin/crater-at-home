use rustwide::{logging, Toolchain, WorkspaceBuilder};
use std::path::Path;
use std::time::Duration;

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug");
    }
    rustwide::logging::init_with(env_logger::Logger::from_default_env());

    let workspace = WorkspaceBuilder::new(Path::new("/tmp"), "miri-the-world")
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
    rustwide::cmd::Command::new(&workspace, nightly.cargo())
        .args(&["install", "xargo", "--target=x86_64-unknown-linux-musl"])
        .run()
        .unwrap();
    rustwide::cmd::Command::new(&workspace, nightly.cargo())
        .env("CARGO", "/tmp/cargo-home/bin/cargo")
        .env("CARGO_HOME", "/tmp/cargo-home")
        .env("XDG_CACHE_HOME", "/tmp/cache")
        .args(&["miri", "setup"])
        .run()
        .unwrap();

    let sandbox = rustwide::cmd::SandboxBuilder::new()
        .memory_limit(Some(1024 * 1024 * 1024 * 63))
        .cpu_limit(None)
        .enable_networking(true);

    let mut build_dir = workspace.build_dir("miri-the-world");

    std::fs::create_dir_all("success").unwrap();
    std::fs::create_dir_all("failure").unwrap();

    for line in include_str!("../top-crates").lines() {
        let mut it = line.split(' ');
        let krate_name = it.next().unwrap();
        let ver = it.next().unwrap();

        let krate = rustwide::Crate::crates_io(krate_name, ver);

        krate.fetch(&workspace).unwrap();

        let storage = logging::LogStorage::new(log::LevelFilter::Debug);

        let res = logging::capture(&storage, || {
            build_dir
                .build(&nightly, &krate, sandbox.clone())
                .run(|build| {
                    build.cargo().args(&["update", "--offline"]).run()?;
                    build
                        .cargo()
                        .env("XARGO_CHECK", "/opt/rustwide/cargo-home/bin/xargo")
                        .env("XDG_CACHE_HOME", "/tmp/cache")
                        .env("RUSTFLAGS", "-Zrandomize-layout")
                        .env("MIRIFLAGS", "-Zmiri-disable-isolation -Zmiri-ignore-leaks -Zmiri-check-number-validity -Zmiri-tag-raw-pointers")
                        .args(&["miri", "test", "--jobs=1", "--", "--test-threads=1"])
                        .timeout(Some(Duration::from_secs(60 * 15)))
                        .run()?;
                    Ok(())
                })
        });
        let base = if res.is_ok() { "success" } else { "failure" };
        std::fs::write(
            format!("{}/{}-{}", base, krate_name, ver),
            storage.to_string().as_bytes(),
        )
        .unwrap();
    }
}
