use crate::{client::Client, render, Crate, Tool, Version};
use clap::Parser;
use color_eyre::eyre::Result;
use once_cell::sync::Lazy;
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
use uuid::Uuid;

static TEST_END_DELIMITER: Lazy<Uuid> = Lazy::new(|| Uuid::new_v4());

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

    let mut tasks = JoinSet::new();
    for cpu in 0..args.jobs.unwrap_or_else(|| num_cpus::get()) {
        let crates = crates.clone();
        let args = args.clone();
        let client = client.clone();

        let test_end_delimiter_with_dashes = format!("-{}-", TEST_END_DELIMITER.to_string());

        let mut child = spawn_worker(&args, cpu);

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
                log::debug!("{:?}", output);

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
                    child = spawn_worker(&args, cpu);
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

fn spawn_worker(args: &Args, cpu: usize) -> tokio::process::Child {
    match args.tool {
        Tool::Miri => spawn_miri_worker(args, cpu),
        Tool::Asan => spawn_asan_worker(args, cpu),
        Tool::Build => spawn_build_worker(args, cpu),
    }
}

fn spawn_asan_worker(args: &Args, cpu: usize) -> tokio::process::Child {
    let rustflags = "-Zsanitizer=address --cap-lints=allow -Zrandomize-layout \
                      -Copt-level=0 -Cdebuginfo=1 -Zvalidate-mir";
    let asan_options = "detect_stack_use_after_return=true:allocator_may_return_null=1:detect_invalid_pointer_pairs=2";
    Worker {
        args,
        cpu,
        rustflags,
        miriflags: "",
        asan_options,
    }
    .spawn()
}

fn spawn_miri_worker(args: &Args, cpu: usize) -> tokio::process::Child {
    let miriflags = "MIRIFLAGS=-Zmiri-disable-isolation -Zmiri-ignore-leaks \
                     -Zmiri-panic-on-unsupported --color=always";
    let rustflags = "--cap-lints=allow -Zrandomize-layout \
                      -Copt-level=0 -Cdebuginfo=0 -Zvalidate-mir";
    Worker {
        args,
        cpu,
        rustflags,
        miriflags,
        asan_options: "",
    }
    .spawn()
}

fn spawn_build_worker(args: &Args, cpu: usize) -> tokio::process::Child {
    let rustflags = "--cap-lints=allow -Zmir-opt-level=2 -Zinline-mir -Copt-level=0 -Cdebuginfo=1 -Zvalidate-mir";
    Worker {
        args,
        cpu,
        rustflags,
        miriflags: "",
        asan_options: "",
    }
    .spawn()
}

struct Worker<'a> {
    args: &'a Args,
    cpu: usize,
    rustflags: &'static str,
    miriflags: &'static str,
    asan_options: &'static str,
}

impl<'a> Worker<'a> {
    fn spawn(self) -> tokio::process::Child {
        let Worker {
            args,
            cpu,
            rustflags,
            miriflags,
            asan_options,
        } = self;
        tokio::process::Command::new("docker")
            .args([
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
                "--tmpfs=/root/build:exec",
                // rustdoc tries to write to and executes files in /tmp, odd move but whatever
                "--tmpfs=/tmp:exec",
                // The default cargo registry location; we download dependences in the sandbox
                "--tmpfs=/root/.cargo/registry",
                &format!("--env=RUSTFLAGS={rustflags}"),
                &format!("--env=RUSTDOCFLAGS={rustflags}"),
                &format!("--env=MIRIFLAGS={miriflags}"),
                &format!("--env=ASAN_OPTIONS={asan_options}"),
                "--env=CARGO_INCREMENTAL=0",
                "--env=RUST_BACKTRACE=1",
                &format!(
                    "--env=TEST_END_DELIMITER={}",
                    TEST_END_DELIMITER.to_string()
                ),
                // Enforce the memory limit
                &format!("--memory={}g", self.args.memory_limit_gb),
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
}
