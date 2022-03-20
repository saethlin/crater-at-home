use clap::Parser;
use crates_io_api::{CratesQuery, Sort, SyncClient};
use regex::Regex;
use rustwide::{
    cmd::{Command, SandboxBuilder},
    logging, Toolchain, WorkspaceBuilder,
};
use std::{
    collections::hash_map::Entry,
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Krate {
    name: String,
    recent_downloads: Option<u64>,
    version: String,
    status: Status,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum Status {
    Unknown,
    Passing,
    Error(String),
    UB { cause: String, status: String },
}

const HEADER: &str = r#"<html><body><pre style="word-wrap: break-word; white-space: pre-wrap;">\n"#;
const FOOTER: &str = "\n</pre></body></html>";

fn main() {
    let tag_re = Regex::new(r"<\d+>").unwrap();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    logging::init_with(env_logger::Logger::from_default_env());

    let args = Args::parse();

    let mut crates = HashMap::new();

    if Path::new("crates.json").exists() {
        for line in fs::read_to_string("crates.json").unwrap().lines() {
            let krate: Krate = serde_json::from_str(&line).unwrap();
            crates.insert(krate.name.clone(), krate);
        }
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
            match crates.entry(c.name.clone()) {
                Entry::Occupied(mut o) => {
                    if o.get().version == c.max_version {
                        o.get_mut().recent_downloads = c.recent_downloads;
                    } else {
                        o.insert(Krate {
                            name: c.name,
                            recent_downloads: c.recent_downloads,
                            version: c.max_version,
                            status: Status::Unknown,
                        });
                    }
                }
                Entry::Vacant(v) => {
                    v.insert(Krate {
                        name: c.name,
                        recent_downloads: c.recent_downloads,
                        version: c.max_version,
                        status: Status::Unknown,
                    });
                }
            }
        }

        log::info!("{} of {}", crates.len(), args.crates);

        page += 1;
    }

    log::info!("Loading missing metadata...");
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

    if !Path::exists(&build_path.join("cargo-home/bin/xargo")) {
        let tc = workspace.installed_toolchains().unwrap()[0].clone();
        tc.add_target(&workspace, "x86_64-unknown-linux-musl")
            .unwrap();
        Command::new(&workspace, tc.cargo())
            .args(&["install", "xargo", "--target=x86_64-unknown-linux-musl"])
            .run()
            .unwrap();
    }

    workspace.purge_all_build_dirs().unwrap();
    workspace.purge_all_caches().unwrap();

    let toolchain = Toolchain::ci("e95b10ba4ac4564ed25f7eef143e3182c33b3902", false);
    toolchain.install(&workspace).unwrap();

    Toolchain::dist("stable").uninstall(&workspace).unwrap();

    Command::new(&workspace, toolchain.cargo())
        .args(&[
            "install",
            "--git",
            "https://github.com/saethlin/miri",
            "--branch=track-alloc-history",
            "--force",
            "--target=x86_64-unknown-linux-musl",
            "miri",
            "cargo-miri",
        ])
        .run()
        .unwrap();

    Command::new(&workspace, toolchain.cargo())
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

    fs::create_dir_all("logs").unwrap();

    let mut previously_run = Vec::new();
    for name in fs::read_dir("logs").unwrap() {
        let name = name.unwrap();
        for file in fs::read_dir(name.path()).unwrap() {
            let file = file.unwrap();
            let name = name.file_name().into_string().unwrap();
            let version = file.file_name().into_string().unwrap();
            previously_run.push((name, version));
        }
    }

    for i in 0..crates.len() {
        let krate = &mut crates[i];

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
             -Zmiri-panic-on-unsupported";

        let res = logging::capture(&storage, || {
            build_dir
                .build(&toolchain, &current_crate, sandbox.clone())
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
        let mut output = format!("{}{}{}", HEADER, storage.to_string(), FOOTER);
        if res.is_ok() {
            fs::create_dir_all(format!("logs/{}", krate.name)).unwrap();
            fs::write(
                format!("logs/{}/{}.html", krate.name, krate.version),
                output.as_bytes(),
            )
            .unwrap();
            krate.status = Status::Passing;
            write_output(&crates);
            continue;
        }

        krate.status = Status::Error(String::new());

        if output.contains("Undefined Behavior: ") {
            krate.status = Status::UB {
                cause: String::new(), //diagnose(&output),
                status: String::new(),
            };
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
                    .build(&toolchain, &current_crate, sandbox.clone())
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
            output = format!("{}{}{}", HEADER, storage.to_string(), FOOTER);

            // Re-diagnose the problem
            krate.status = Status::UB {
                cause: String::new(), //diagnose(&output),
                status: String::new(),
            };
        }

        fs::create_dir_all(format!("logs/{}", krate.name)).unwrap();
        fs::write(
            format!("logs/{}/{}.html", krate.name, krate.version),
            output.as_bytes(),
        )
        .unwrap();

        write_output(&crates);
    }
}

const OUTPUT_HEADER: &str = r#"<style>
    body {
        background: #111;
        color: #eee;
        font-family: sans-serif;
        font-size: 18px;
    }
    a {
        color: #eee;
        text-decoration: none;
    }
    .row {
        display: flex;
        border-bottom: 1px solid #333;
        width: 40em;
        padding: 1em;
    }
    .crate {
        flex: 1;
    }
    .status {
        flex: 2;
    }
    .page {
        width: 40em;
        margin: auto;
    }
</style>
<html>
<body>
<div class="page">"#;

fn write_output(crates: &[Krate]) {
    let mut output = File::create(".crates.json").unwrap();

    for c in crates {
        writeln!(output, "{}", serde_json::to_string(c).unwrap()).unwrap();
    }

    fs::rename(".crates.json", "crates.json").unwrap();

    let mut output = File::create(".index.html").unwrap();
    writeln!(output, "{}", OUTPUT_HEADER).unwrap();
    for c in crates {
        write!(
            output,
            "<div class=\"row\"><div class=\"crate\"><a href=\"{}/{}.html\">{} {}</a></div>",
            c.name, c.version, c.name, c.version
        )
        .unwrap();
        write!(output, "<div class=\"status\">").unwrap();
        match &c.status {
            Status::Unknown => write!(output, "Unknown"),
            Status::Passing => write!(output, "Passing"),
            Status::Error(_) => write!(output, "Error"),
            Status::UB { cause, .. } => write!(output, "{}", cause),
        }
        .unwrap();
        writeln!(output, "</div></div>").unwrap();
    }

    fs::rename(".index.html", "index.html").unwrap();
}

/*
fn diagnose(output: &str) -> String {
    if output.contains("-Zmiri-track-pointer-tag") {
        return diagnose_sb(output);
    }

    let mut causes = Vec::new();

    let lines = output.lines().collect::<Vec<_>>();

    for (l, line) in lines
        .iter()
        .emumerate()
        .filter(|(_, line)| line.contains("Undefined Behavior: "))
    {
        if line.contains("uninitialized") {
            causes.push("uninitialized memory".to_string());
        } else if line.contains("out-of-bounds") {
            causes.push("invalid pointer offset".to_string());
        } else if line.contains("null pointer is not a valid pointer for this operation") {
            causes.push("null pointer dereference".to_string());
        } else if line.contains("accessing memory with alignment") {
            causes.push("misaligned pointer dereference".to_string());
        } else if line.contains("dangling reference") {
            causes.push("dangling reference".to_string());
        } else if line.contains("unaligned reference") {
            causes.push("unaligned reference".to_string());
        } else if line.contains("incorrect layout on deallocation") {
            causes.push("incorrect layout on deallocation".to_string());
        } else if line.contains("borrow stack") {
            if line.contains("<untagged>") {
                causes.push("int-to-ptr cast".to_string());
            } else {
                causes.push("SB".to_string());
            }
        } else {
            causes.push(line.split("Undefined Behavior: ").nth(1).unwrap().trim());
        }

        for line in &lines[l..] {
            if line.contains("note: inside ") && line.contains(" at ") {
                let path = line.split(" at ").nth(1).unwrap();
                if path.contains("workdir") || !path.starts_with("/") {
                    break;
                } else if path.contains("github") {
                    let last = causes.last().unwrap().to_string();
                    *causes.last_mut().unwrap() =
                        format!("{} ({})", last, path.split("/").nth(7).unwrap());
                    break;
                }
            }
        }
    }

    causes.sort();
    causes.dedup();
}
*/
