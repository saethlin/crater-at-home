use rustwide::{Toolchain, WorkspaceBuilder};
use std::path::Path;

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();
    let workspace = WorkspaceBuilder::new(Path::new("/tmp"), "miri-the-world")
        .init()
        .unwrap();

    let nightly = Toolchain::dist("nightly-2022-01-27");

    for tc in workspace.installed_toolchains().unwrap() {
        tc.uninstall(&workspace).unwrap();
    }

    workspace.purge_all_build_dirs().unwrap();
    workspace.purge_all_caches().unwrap();

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
        .memory_limit(Some(1024 * 1024 * 1024 * 32))
        .cpu_limit(None)
        .enable_networking(true);

    let mut build_dir = workspace.build_dir("miri-the-world");

    let krate = rustwide::Crate::crates_io("ryu", "1.0.9");

    krate.fetch(&workspace).unwrap();

    build_dir
        .build(&nightly, &krate, sandbox)
        .run(|build| {
            build
                .cargo()
                .env("XARGO_CHECK", "/opt/rustwide/cargo-home/bin/xargo")
                .env("XDG_CACHE_HOME", "/tmp/cache")
                .env("MIRIFLAGS", "-Zmiri-disable-isolation")
                .args(&["miri", "test", "--jobs=1", "--", "--test-threads=1"])
                .run()?;
            Ok(())
        })
        .unwrap();
}
