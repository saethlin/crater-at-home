use crate::{client::Client, Crate, Tool, Version};
use anyhow::{ensure, Result};
use clap::Parser;
use once_cell::sync::Lazy;
use std::io::BufRead;
use std::io::Write;
use std::{
    collections::HashMap,
    fs,
    io::BufReader,
    process::Stdio,
    sync::{Arc, Mutex},
};
use uuid::Uuid;
use xz2::write::XzEncoder;

static TEST_END_DELIMITER: Lazy<Uuid> = Lazy::new(Uuid::new_v4);

// These crates generate gigabytes of output then don't build.
const IGNORED_CRATES: &[&str] = &["clacks_mtproto", "stdweb"];

#[derive(Parser, Clone)]
pub struct Args {
    /// Run the top `n` most-recently-downloaded crates
    #[clap(long, conflicts_with = "crate_list")]
    crates: Option<usize>,

    /// A path to a file containing a whitespace-separated list of crates to run
    #[clap(long, conflicts_with = "crates")]
    crate_list: Option<String>,

    #[clap(long, default_value_t = 8)]
    memory_limit_gb: usize,

    #[clap(long)]
    rerun: bool,

    #[clap(long)]
    pub tool: Tool,

    #[clap(long)]
    pub bucket: String,

    #[clap(long)]
    jobs: Option<usize>,

    #[clap(long)]
    rev: bool,

    #[clap(
        long,
        default_value = "x86_64-unknown-linux-gnu",
        value_parser = clap::builder::PossibleValuesParser::new(["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"])
    )]
    target: String,
}

fn build_crate_list(args: &Args, client: &Client) -> Result<Vec<Crate>> {
    let all_crates = client.get_crate_versions()?;
    let crates = if let Some(crate_list) = &args.crate_list {
        let crate_list = fs::read_to_string(crate_list).unwrap();
        let all_crates: HashMap<String, Crate> = all_crates
            .into_iter()
            .map(|c| (c.name.clone(), c))
            .collect();
        let mut crates = Vec::new();
        for line in crate_list.split_whitespace() {
            let mut it = line.split(|c| c == '@' || c == '/');
            let name = it.next().unwrap();
            let version = it.next();
            if let Some(c) = all_crates.get(name) {
                crates.push(Crate {
                    version: version
                        .map(Version::parse)
                        .unwrap_or_else(|| c.version.clone()),
                    ..c.clone()
                });
            }
        }
        crates.sort_by(|a, b| a.recent_downloads.cmp(&b.recent_downloads));
        crates
    } else if let Some(crate_count) = args.crates {
        let mut crates = all_crates;
        crates.truncate(crate_count);
        crates
    } else {
        all_crates
    };
    Ok(crates)
}

pub fn run(args: &Args) -> Result<()> {
    let dockerfile = if std::env::var_os("CI").is_some() {
        "docker/Dockerfile.ci"
    } else {
        "docker/Dockerfile"
    };
    let status = std::process::Command::new("docker")
        .args(["build", "-t", "crater-at-home", "-f", dockerfile, "docker/"])
        .status()?;
    ensure!(status.success(), "docker image build failed!");

    log::info!("Figuring out what crates have a build log already");
    let client = Client::new(args.tool, &args.bucket)?;
    let mut crates = build_crate_list(args, &client)?;
    if !args.rerun {
        let finished_crates = client.list_finished_crates(Some(time::Duration::days(90)))?;
        crates.retain(|krate| {
            !finished_crates
                .iter()
                .any(|c| c.name == krate.name && c.version == krate.version)
        });
    }

    if !args.rev {
        // We are going to pop crates from this, so we now need to invert the order
        crates = crates.into_iter().rev().collect::<Vec<_>>();
    }
    let crates = Arc::new(Mutex::new(crates));

    let mut tasks = Vec::new();
    for cpu in 0..args.jobs.unwrap_or_else(num_cpus::get) {
        let crates = crates.clone();
        let args = args.clone();
        let client = Client::new(args.tool, &args.bucket)?;

        let test_end_delimiter_with_dashes = format!("-{}-\n", *TEST_END_DELIMITER).into_bytes();

        let mut child = spawn_worker(&args, cpu);

        let handle = std::thread::spawn(move || {
            loop {
                let mut stdout = BufReader::new(child.stdout.as_mut().unwrap());
                let krate = match crates.lock().unwrap().pop() {
                    None => break,
                    Some(krate) => krate,
                };

                if IGNORED_CRATES.contains(&&krate.name[..]) {
                    continue;
                }

                log::info!("Running {} {}", krate.name, krate.version);

                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(format!("{}@{}\n", krate.name, krate.version).as_bytes())
                    .unwrap();

                let mut encoder = XzEncoder::new(Vec::new(), 6);
                let mut output = Vec::new();
                loop {
                    let bytes_read = stdout.read_until(b'\n', &mut output).unwrap();
                    if output.ends_with(&test_end_delimiter_with_dashes) {
                        output.truncate(output.len() - test_end_delimiter_with_dashes.len());
                        encoder.write_all(&output).unwrap();
                        break;
                    }
                    encoder.write_all(&output).unwrap();
                    output.clear();
                    if bytes_read == 0 {
                        break;
                    }
                }
                let compressed = encoder.finish().unwrap();

                if let Ok(Some(_)) = child.try_wait() {
                    log::warn!("A worker crashed! Standing up a new one...");
                    child = spawn_worker(&args, cpu);
                    // Don't upload logs for crashed runs
                    continue;
                }

                client.upload_raw(&krate, &compressed).unwrap();

                log::info!("Finished {} {}", krate.name, krate.version);
            }
        });
        tasks.push(handle);
    }

    for task in tasks {
        task.join().unwrap();
    }

    log::info!("done!");

    Ok(())
}

fn spawn_worker(args: &Args, cpu: usize) -> std::process::Child {
    let mut cmd = std::process::Command::new("docker");
    cmd.args([
        "run",
        "--rm",
        "--interactive",
        // Pin the build to a single CPU; this also ensures that anything doing
        // make -j $(nproc)
        // will not spawn processes appropriate for the host.
        &format!("--cpuset-cpus={cpu}"),
        // We set up our filesystem as read-only, but with 3 exceptions
        "--read-only",
        // The directory we are building in (not just its target dir!) is all writable
        "--volume=/build",
        // rustdoc tries to write to and executes files in /tmp, odd move but whatever
        "--tmpfs=/tmp:exec",
        // The default cargo registry location; we download dependences in the sandbox
        "--tmpfs=/root/.cargo/registry",
        // cargo-miri builds a sysroot under /root/.cache, so why not make it all writeable
        "--tmpfs=/root/.cache:exec",
        &format!("--env=TEST_END_DELIMITER={}", *TEST_END_DELIMITER),
        &format!("--env=TOOL={}", args.tool),
        &format!("--env=TARGET={}", args.target),
    ]);
    cmd.args([
        // Enforce the memory limit
        &format!("--memory={}g", args.memory_limit_gb),
        // Setting --memory-swap to the same value turns off swap
        &format!("--memory-swap={}g", args.memory_limit_gb),
        "crater-at-home:latest",
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap()
}
