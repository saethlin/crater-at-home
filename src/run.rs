use crate::{client::Client, render, Crate, Tool, Version};
use clap::Parser;
use color_eyre::eyre::Result;
use std::{
    collections::HashMap,
    fs,
    process::Stdio,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    task::JoinSet,
};

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
}

impl Args {
    fn docker_tag(&self) -> String {
        format!("{}-the-world", self.tool)
    }

    fn dockerfile(&self) -> String {
        format!("docker/Dockerfile-{}", self.tool)
    }
}

async fn build_crate_list(args: &Args, client: &Client) -> Result<Vec<Crate>> {
    let all_crates = client.get_crate_list().await?;
    let crates = if let Some(crate_list) = &args.crate_list {
        let crate_list = fs::read_to_string(crate_list).unwrap();
        let all_crates: HashMap<String, Crate> = all_crates
            .into_iter()
            .map(|c| (c.name.clone(), c))
            .collect();
        let mut crates = Vec::new();
        for line in crate_list.split_whitespace() {
            let mut it = line.split('/');
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

#[tokio::main]
pub async fn run(args: Args) -> Result<()> {
    let status = std::process::Command::new("docker")
        .args([
            "build",
            "-t",
            &args.docker_tag(),
            "-f",
            &args.dockerfile(),
            "docker/",
        ])
        .status()?;
    color_eyre::eyre::ensure!(status.success(), "docker image build failed!");

    log::info!("Figuring out what crates have a build log already");
    let client = Arc::new(Client::new(args.tool, &args.bucket).await?);
    let mut crates = build_crate_list(&args, &client).await?;
    let finished_crates = client.list_finished_crates().await?;
    crates.retain(|krate| {
        args.rerun
            || !finished_crates
                .iter()
                .any(|c| c.name == krate.name && c.version == krate.version)
    });

    // We are going to pop crates from this, so we now need to invert the order
    let crates = crates.into_iter().rev().collect::<Vec<_>>();
    let crates = Arc::new(Mutex::new(crates));

    let test_end_delimiter = uuid::Uuid::new_v4().to_string();

    let mut tasks = JoinSet::new();
    for _ in 0..num_cpus::get() {
        let crates = crates.clone();
        let args = args.clone();
        let client = client.clone();
        let test_end_delimiter = test_end_delimiter.clone();

        let test_end_delimiter_with_dashes = format!("-{}-", test_end_delimiter);

        let mut child = spawn_worker(&args, &test_end_delimiter);

        tasks.spawn(async move {
            loop {
                let mut stdout = BufReader::new(child.stdout.as_mut().unwrap());
                let krate = match crates.lock().unwrap().pop() {
                    None => break,
                    Some(krate) => krate,
                };

                log::info!("Running {} {}", krate.name, krate.version);

                child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(format!("{}=={}\n", krate.name, krate.version).as_bytes())
                    .await
                    .unwrap();

                let mut output = String::new();
                loop {
                    let bytes_read = stdout.read_line(&mut output).await.unwrap();
                    if output.trim_end().ends_with(&test_end_delimiter_with_dashes) {
                        output.truncate(output.len() - test_end_delimiter_with_dashes.len() - 1);
                        break;
                    }
                    if bytes_read == 0 {
                        break;
                    }
                }

                // Render HTML for the stderr/stdout we captured
                let rendered = render::render_crate(&krate, &output);

                // Upload both
                client
                    .upload_raw(&krate, output.into_bytes())
                    .await
                    .unwrap();
                client
                    .upload_html(&krate, rendered.into_bytes())
                    .await
                    .unwrap();

                log::info!("Finished {} {}", krate.name, krate.version);

                if let Ok(Some(_)) = child.try_wait() {
                    log::warn!("A worker crashed! Standing up a new one...");
                    child = spawn_worker(&args, &test_end_delimiter);
                }
            }
        });
    }

    while let Some(task) = tasks.join_next().await {
        task?;
    }

    log::info!("done!");

    Ok(())
}

fn spawn_worker(args: &Args, test_end_delimiter: &str) -> tokio::process::Child {
    match args.tool {
        Tool::Miri => spawn_miri_worker(args, test_end_delimiter),
        Tool::Asan => spawn_asan_worker(args, test_end_delimiter),
    }
}

fn spawn_asan_worker(args: &Args, test_end_delimiter: &str) -> tokio::process::Child {
    let rust_flags = "-Zsanitizer=address -Zrandomize-layout --cap-lints allow \
                      -Copt-level=0 -Cdebuginfo=0 -Zvalidate-mir";
    tokio::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "--interactive",
            "--cpus=1",       // Limit the build to one CPU
            "--cpu-shares=2", // And reduce priority
            // Create tmpfs mounts for all the locations we expect to be doing work in, so that
            // we minimize actual disk I/O
            "--tmpfs=/root/build:exec",
            "--tmpfs=/root/.cache",
            "--tmpfs=/tmp:exec",
            "--env",
            &format!("RUSTFLAGS={rust_flags}"),
            "--env",
            &format!("RUSTDOCFLAGS={rust_flags}"),
            "--env",
            "CARGO_INCREMENTAL=0",
            "--env",
            "RUST_BACKTRACE=1",
            "--env",
            &format!("TEST_END_DELIMITER={}", test_end_delimiter),
            // Enforce the memory limit
            &format!("--memory={}g", args.memory_limit_gb),
            // Setting --memory-swap to the same value turns off swap
            &format!("--memory-swap={}g", args.memory_limit_gb),
            &format!("{}:latest", args.docker_tag()),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn spawn_miri_worker(args: &Args, test_end_delimiter: &str) -> tokio::process::Child {
    let miri_flags = "MIRIFLAGS=-Zmiri-disable-isolation -Zmiri-ignore-leaks \
                     -Zmiri-panic-on-unsupported";
    let rust_flags = "-Zrandomize-layout --cap-lints allow \
                      -Copt-level=0 -Cdebuginfo=0 -Zvalidate-mir";

    tokio::process::Command::new("docker")
        .args([
            "run",
            "--rm",
            "--interactive",
            "--cpus=1",       // Limit the build to one CPU
            "--cpu-shares=2", // And reduce priority
            // Create tmpfs mounts for all the locations we expect to be doing work in, so that
            // we minimize actual disk I/O
            "--tmpfs=/root/build:exec",
            "--tmpfs=/root/.cache",
            "--tmpfs=/tmp:exec",
            "--env",
            &format!("RUSTFLAGS={rust_flags}"),
            "--env",
            &format!("RUSTDOCFLAGS={rust_flags}"),
            "--env",
            "CARGO_INCREMENTAL=0",
            "--env",
            "RUST_BACKTRACE=0",
            "--env",
            miri_flags,
            "--env",
            &format!("TEST_END_DELIMITER={}", test_end_delimiter),
            // Enforce the memory limit
            &format!("--memory={}g", args.memory_limit_gb),
            // Setting --memory-swap to the same value turns off swap
            &format!("--memory-swap={}g", args.memory_limit_gb),
            &format!("{}:latest", args.docker_tag()),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}
