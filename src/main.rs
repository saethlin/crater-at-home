use backoff::{retry, ExponentialBackoff};
use clap::Parser;
use color_eyre::eyre::Result;
use indicatif::{ProgressBar, ProgressStyle};
use miri_the_world::*;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::Stdio;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

#[derive(Parser)]
struct Args {
    /// Run the top `n` most-recently-downloaded crates
    #[clap(long, conflicts_with = "crate-list")]
    crates: Option<usize>,

    /// A path to a file containing a whitespace-separated list of crates to run
    #[clap(long, conflicts_with = "crates")]
    crate_list: Option<String>,

    #[clap(long, default_value_t = 8)]
    memory_limit_gb: usize,

    #[clap(long, default_value_t = RerunWhen::LockfileChanged)]
    rerun_when: RerunWhen,
}

enum RerunWhen {
    Always,
    Never,
    LockfileChanged,
}

impl FromStr for RerunWhen {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "always" => Ok(RerunWhen::Always),
            "never" => Ok(RerunWhen::Never),
            "lockfile-changed" => Ok(RerunWhen::LockfileChanged),
            _ => Err("invalid rerun-when option"),
        }
    }
}

impl fmt::Display for RerunWhen {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RerunWhen::Always => "always",
                RerunWhen::Never => "never",
                RerunWhen::LockfileChanged => "lockfile-changed",
            }
        )
    }
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

    let all_crates = db_dump::download()?;
    let crates = if let Some(crate_count) = args.crates {
        let mut crates = all_crates.clone();
        crates.truncate(crate_count);
        crates
    } else {
        let crate_list = fs::read_to_string(&args.crate_list.clone().unwrap())?;
        let crate_list: HashSet<_> = crate_list.trim().split_whitespace().collect();
        all_crates
            .into_iter()
            .filter(|c| crate_list.contains(c.name.as_str()))
            .collect()
    };

    fs::create_dir_all("logs")?;

    log::info!("Building list of crates to run");

    let bar = ProgressBar::new(crates.len() as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}/{duration_precise}] {wide_bar} {pos}/{len}"),
    );

    let crates = crates
        .into_par_iter()
        .filter(|krate| {
            // Keep all crates if rerun-when=always
            if let RerunWhen::Always = args.rerun_when {
                return true;
            }

            if let Ok(contents) =
                fs::read_to_string(format!("logs/{}/{}", krate.name, krate.version))
            {
                // Skip crates if a log exists and rerun-when=never
                if let RerunWhen::Never = args.rerun_when {
                    return false;
                }

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
                    bar.inc(1);
                    return false;
                }
            }

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

    let miri_flags = "MIRIFLAGS=-Zmiri-disable-isolation -Zmiri-ignore-leaks \
                     -Zmiri-panic-on-unsupported";

    // Reverse the sort order, most-downloaded last
    let crates = crates.into_iter().rev().collect::<Vec<_>>();
    let crates = Arc::new(Mutex::new(crates));

    let test_end_delimiter = uuid::Uuid::new_v4().to_string();

    let mut threads = Vec::new();
    for _ in 0..num_cpus::get() / 2 {
        let bar = bar.clone();
        let crates = crates.clone();
        let test_end_delimiter = test_end_delimiter.clone();

        let child = std::process::Command::new("docker")
            .args(&[
                "run",
                "--rm",
                "--interactive",
                "--cpu-shares=2",
                "--env",
                "RUSTFLAGS=-Zrandomize-layout --cap-lints allow -Copt-level=0 -Cdebuginfo=0",
                "--env",
                "RUSTDOCFLAGS=--color=always",
                "--env",
                "CARGO_INCREMENTAL=0",
                "--env",
                "RUST_BACKTRACE=0",
                "--env",
                miri_flags,
                "--env",
                "TEST_TIMEOUT=900",
                "--env",
                &format!("TEST_END_DELIMITER={}", test_end_delimiter),
                &format!("--memory={}g", args.memory_limit_gb),
                "miri:latest",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let mut stdin = child.stdin.unwrap();

        let mut stdout = BufReader::new(child.stdout.unwrap());

        let handle = std::thread::spawn(move || loop {
            let krate = match crates.lock().unwrap().pop() {
                None => break,
                Some(krate) => krate,
            };

            bar.println(format!("Running {} {}", krate.name, krate.version));

            stdin
                .write_all(format!("{}=={}\n", krate.name, krate.version).as_bytes())
                .unwrap();

            let mut output = String::new();
            loop {
                let bytes_read = stdout.read_line(&mut output).unwrap();
                if output.trim_end().ends_with(&test_end_delimiter) {
                    output.truncate(output.len() - test_end_delimiter.len() - 1);
                    break;
                }
                if bytes_read == 0 {
                    break;
                }
            }

            fs::create_dir_all(format!("logs/{}", krate.name)).unwrap();
            fs::write(format!("logs/{}/{}", krate.name, krate.version), &*output).unwrap();
            bar.inc(1);
            bar.println(format!("Finished {} {}", krate.name, krate.version));
        });
        threads.push(handle);
    }

    for t in threads {
        t.join().unwrap();
    }

    log::info!("done!");

    Ok(())
}
