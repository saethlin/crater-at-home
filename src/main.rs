use backoff::{retry, ExponentialBackoff};
use clap::Parser;
use color_eyre::eyre::{ensure, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use miri_the_world::*;
use rayon::prelude::*;
use std::fs;

#[derive(Parser)]
struct Args {
    #[clap(long, default_value_t = 10000)]
    crates: usize,

    #[clap(long, default_value_t = 8)]
    memory_limit_gb: usize,
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

    let mut crates = db_dump::download()?;
    crates.truncate(args.crates);

    fs::create_dir_all("logs")?;

    let bar = ProgressBar::new(args.crates as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}/{duration_precise}] {wide_bar} {pos}/{len}"),
    );
    bar.enable_steady_tick(1);
    bar.set_draw_rate(1);

    let crates = crates
        .into_par_iter()
        .filter(|krate| {
            if let Ok(contents) =
                fs::read_to_string(format!("logs/{}/{}", krate.name, krate.version))
            {
                let previous_lockfile = contents.rsplit("cat Cargo.lock\r\n").nth(0).unwrap();
                let previous_lockfile = previous_lockfile.replace("\r\n", "\n");

                let dir = tempfile::tempdir().unwrap();

                let backoff = ExponentialBackoff::default();
                if let Err(e) = retry(backoff, || {
                    krate.fetch_into(dir.path()).map_err(|e| {
                        // Yanked crates return a 403, retrying those is a mistake
                        if let Error::Net(ureq::Error::Status(403, _)) = e {
                            backoff::Error::permanent(e)
                        } else {
                            backoff::Error::transient(e)
                        }
                    })
                }) {
                    bar.println(&format!("{:?}", e));
                    bar.inc(1);
                    return true;
                }

                let res = std::process::Command::new("cargo")
                    .args(&[
                        "+nightly",
                        "update",
                        &format!(
                            "--manifest-path={}",
                            dir.path().join("Cargo.toml").display()
                        ),
                        "-Zno-index-update",
                    ])
                    .output()
                    .unwrap();

                let new_lockfile = if res.status.success() {
                    std::fs::read_to_string(dir.path().join("Cargo.lock")).unwrap()
                } else {
                    bar.println(&format!(
                        "Error when generating lockfile for {} {} {}",
                        krate.name,
                        krate.version,
                        String::from_utf8_lossy(&res.stderr)
                    ));
                    bar.inc(1);
                    return true;
                };

                if new_lockfile == previous_lockfile {
                    bar.println(&format!(
                        "Lockfile unchanged for {} {}",
                        krate.name, krate.version
                    ));
                    bar.inc(1);
                    return false;
                }
            }

            bar.println(&format!(
                "Lockfile changed for {} {}",
                krate.name, krate.version
            ));
            bar.inc(1);
            true
        })
        .collect::<Vec<_>>();
    bar.finish();

    let bar = ProgressBar::new(crates.len() as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}/{duration_precise}] {wide_bar} {pos}/{len}"),
    );
    bar.enable_steady_tick(1);
    bar.set_draw_rate(1);

    crates
        .into_par_iter()
        .map(|krate| {
            bar.println(format!("Running {} {}", krate.name, krate.version));

            let miri_flags =
            "MIRIFLAGS=-Zmiri-disable-isolation -Zmiri-ignore-leaks -Zmiri-check-number-validity \
                     -Zmiri-panic-on-unsupported -Zmiri-tag-raw-pointers";

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

            let output = String::from_utf8_lossy(&res.stdout);

            // The container is supposed to redirect everything to stdout
            ensure!(
                res.stderr.is_empty(),
                "{}",
                String::from_utf8_lossy(&res.stderr)
            );

            fs::create_dir_all(format!("logs/{}", krate.name))?;
            fs::write(format!("logs/{}/{}", krate.name, krate.version), &*output)?;
            bar.inc(1);
            bar.println(format!("Finished {} {}", krate.name, krate.version));

            Ok(())
        })
        .collect::<Result<_>>()?;

    Ok(())
}
