use clap::Parser;
use color_eyre::eyre::{ensure, eyre, Context, ErrReport, Result};
use crates_io_api::{CratesQuery, Sort, SyncClient};
use miri_the_world::*;
use std::{
    collections::hash_map::Entry,
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Parser)]
struct Args {
    #[clap(long, default_value_t = 10000)]
    crates: usize,

    #[clap(long, default_value_t = 8)]
    memory_limit_gb: usize,

    #[clap(long, default_value_t = 8)]
    jobs: usize,
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

    let mut crates = HashMap::new();

    if Path::new("crates.json").exists() {
        for line in fs::read_to_string("crates.json")
            .unwrap()
            .lines()
            .take(args.crates)
        {
            let krate: Crate = serde_json::from_str(&line)?;
            crates.insert(krate.name.clone(), krate);
        }
    }

    let client = SyncClient::new(
        "miri (kimockb@gmail.com)",
        std::time::Duration::from_millis(1000),
    )?;

    log::info!("Discovering crates...");
    let mut page = 1;
    while crates.len() < args.crates {
        let mut query = CratesQuery::builder()
            .sort(Sort::RecentDownloads)
            .page_size(100)
            .build();
        query.set_page(page);

        let response = client.crates(query)?;

        for c in response.crates.into_iter().take(args.crates - crates.len()) {
            match crates.entry(c.name.clone()) {
                Entry::Occupied(mut o) => {
                    if o.get().version == c.max_version {
                        o.get_mut().recent_downloads = c.recent_downloads;
                    } else {
                        o.insert(Crate {
                            name: c.name,
                            recent_downloads: c.recent_downloads,
                            version: c.max_version,
                            status: Status::Unknown,
                            time: u64::max_value(),
                        });
                    }
                }
                Entry::Vacant(v) => {
                    v.insert(Crate {
                        name: c.name,
                        recent_downloads: c.recent_downloads,
                        version: c.max_version,
                        status: Status::Unknown,
                        time: u64::max_value(),
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
            krate.recent_downloads = client.get_crate(&krate.name)?.crate_data.recent_downloads;
            assert!(krate.recent_downloads.is_some());
            log::info!("{} of {}", k, args.crates);
        }
    }

    let mut crates = crates.into_iter().map(|pair| pair.1).collect::<Vec<_>>();

    // Sort by recent downloads, descending
    crates.sort_by(|a, b| b.recent_downloads.cmp(&a.recent_downloads));

    fs::create_dir_all("logs")?;

    let mut previously_run = Vec::new();
    for name in fs::read_dir("logs")? {
        let name = name?;
        for file in fs::read_dir(name.path())? {
            let file = file?;
            let name = name.file_name().into_string().unwrap();
            let version = file.file_name().into_string().unwrap();
            if !version.ends_with(".html") {
                previously_run.push((name, version));
            }
        }
    }

    struct Cursor {
        crates: Vec<Crate>,
        next: usize,
    }

    let cursor = Arc::new(Mutex::new(Cursor { crates, next: 0 }));

    render(
        &mut cursor
            .lock()
            .map_err(|_| eyre!("an executor thread panicked"))?
            .crates,
    )?;

    let mut threads = Vec::new();
    for _ in 0..args.jobs {
        let cursor = cursor.clone();
        let previously_run = previously_run.clone();
        let handle = std::thread::spawn(move || -> Result<()> {
            loop {
                let mut lock = cursor
                    .lock()
                    .map_err(|_| eyre!("the main thread panicked"))?;

                let i = lock.next;
                lock.next += 1;

                let mut krate = if let Some(krate) = lock.crates.get(i) {
                    krate.clone()
                } else {
                    break Ok(());
                };

                drop(lock);

                if previously_run
                    .iter()
                    .any(|(name, version)| &krate.name == name && &krate.version == version)
                {
                    log::info!("Already ran {} {}", krate.name, krate.version);
                    continue;
                }

                log::info!("Running {} {}", krate.name, krate.version);

                let miri_flags =
                    "MIRIFLAGS=-Zmiri-disable-isolation -Zmiri-ignore-leaks -Zmiri-check-number-validity \
                     -Zmiri-panic-on-unsupported -Zmiri-tag-raw-pointers";

                let start = std::time::Instant::now();

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

                let end = std::time::Instant::now();
                krate.time = (end - start).as_secs();

                let output = String::from_utf8_lossy(&res.stdout);

                // The container is supposed to redirect everything to stdout
                ensure!(
                    res.stderr.is_empty(),
                    "{}",
                    String::from_utf8_lossy(&res.stderr)
                );

                fs::create_dir_all(format!("logs/{}", krate.name))?;
                fs::write(format!("logs/{}/{}", krate.name, krate.version), &*output)?;

                let mut lock = cursor
                    .lock()
                    .map_err(|_| eyre!("the main thread panicked"))?;
                lock.crates[i] = krate;
                render(&mut lock.crates)?;
            }
        });
        threads.push(handle);
    }

    for t in threads {
        t.join()
            .map_err(|e| *e.downcast::<ErrReport>().unwrap())??;
    }

    Ok(())
}

fn render(crates: &mut [Crate]) -> Result<()> {
    for krate in crates.iter_mut() {
        let path = format!("logs/{}/{}", krate.name, krate.version);
        if let Ok(output) = fs::read_to_string(&path) {
            krate.status = if output.contains("Undefined Behavior: ") {
                Status::UB {
                    cause: diagnose(&output),
                    status: String::new(),
                }
            } else if output.contains("Command exited with non-zero status 124") {
                Status::Error("Timeout".to_string())
            } else if output.contains("Command exited with non-zero status 255") {
                Status::Error("OOM".to_string())
            } else if output.contains("Command exited with non-zero status") {
                Status::Error(String::new())
            } else {
                Status::Passing
            };
            write_crate_output(krate, &output)?;
        }
    }

    write_output(crates)
}

#[rustfmt::skip]
macro_rules! log_format {
    () => {
r#"<html><head><style>
body {{
    background: #111;
    color: #eee;
}}
pre {{
    word-wrap: break-word;
    white-space: pre-wrap;
    font-size: 14px;
    font-size-adjust: none;
    text-size-adjust: none;
    -webkit-text-size-adjust: 100%;
    -moz-text-size-adjust: 100%;
    -ms-text-size-adjust: 100%;
}}
</style><title>{} {}</title></head>
<script>
function scroll_to_ub() {{
    var ub = document.getElementById("ub");
    if (ub !== null) {{
        ub.scrollIntoView();
    }}
}}
</script>
<body onload="scroll_to_ub()"><pre>
{}
</pre></body></html>"#
    }
}

fn write_crate_output(krate: &Crate, output: &str) -> Result<()> {
    let mut encoded = String::new();
    let mut found_ub = false;
    for mut line in output.lines() {
        while let Some(pos) = line.find('\r') {
            line = &line[pos + 1..];
        }
        if !found_ub && line.contains("Undefined Behavior:") {
            found_ub = true;
            encoded.push_str("</pre><pre id=\"ub\">\n");
        }
        let line = ansi_to_html::convert_escaped(line.trim())?;
        let line = line.replace("\u{1b}(B</span>", "</span>");
        encoded.push_str(&line);
        encoded.push('\n');
    }

    fs::create_dir_all(format!("logs/{}", krate.name))?;

    let mut file = File::create(format!("logs/{}/{}.html", krate.name, krate.version))?;

    write!(file, log_format!(), krate.name, krate.version, encoded)?;
    Ok(())
}

const OUTPUT_HEADER: &str = r#"<!DOCTYPE HTML>
<html><head><style>
body {
    background: #111;
    color: #eee;
    font-family: sans-serif;
    font-size: 18px;
    margin: 0;
}
a {
    color: #eee;
}
.row {
    display: flex;
    border-bottom: 1px solid #333;
    padding: 1em 2em 1em 1em;
    width: 100%;
}
.log {
    order: 1;
    height: 100vh;
    margin: 0;
    width: 100%;
    font-size: 14px;
}
.pre {
    word-wrap: break-word;
    white-space: pre-wrap;
}
.crates {
    order: 2;
    height: 100vh;
    width: 25%;
    overflow-y: scroll;
    overflow-x: hidden;
}
.crate {
    order: 1;
    flex: 1;
    padding-right: 2em;
}
.status {
    order: 2;
    flex: 1;
    padding-right: 2em;
}
.page {
    display: flex;
    flex-direction: row;
    width: 100%;
    height: 100%;
    margin: 0;
}
</style></head><body onload="init()">
<script>
function init() {
    var params = decode_params();
    if (params.crate != undefined && params.version != undefined) {
        change_log(params.crate, params.version);
    }
}
function decode_params() {
    var params = {};
    var paramsarr = window.location.search.substr(1).split('&');
    for (var i = 0; i < paramsarr.length; ++i) {
        var tmp = paramsarr[i].split("=");
        if (!tmp[0] || !tmp[1]) continue;
        params[tmp[0]]  = decodeURIComponent(tmp[1]);
    }
    return params;
}
function change_log(crate, version) {
    var html = "<object data=\"logs/" + crate + "/" + version + ".html\" width=100% height=100%></object>";
    var build_log = document.getElementById("log");
    build_log.innerHTML = html;

    params = decode_params();
    params.crate = crate;
    params.version = version;
    history.replaceState(null, null, encode_params(params));
}
function encode_params(params) {
    var uri = "?";
    for (var key in params) {
        uri += key + '=' + encodeURIComponent(params[key]) + '&';
    }
    if (uri.slice(-1) == "&") {
        uri = uri.substring(0, uri.length - 1);
    }
    if (uri == '?') {
        uri = window.location.href.split('?')[0];
    }
    return uri;
}
</script>
<div class="page">
<div class="log" id="log">
<object>
<html><head><style>
body {
    background: #111;
    color: #eee;
}
pre {
    word-wrap: break-word;
    white-space: pre-wrap;
    font-size: 14px;
    font-size-adjust: none;
    text-size-adjust: none;
    -webkit-text-size-adjust: 100%;
    -moz-text-size-adjust: 100%;
    -ms-text-size-adjust: 100%;
}
</style><title>Miri build logs</title></head><body><pre>
Click on a crate to the right to display its build log
</pre></body></html>
</object>
</div>
<div class="crates">
"#;

fn write_output(crates: &[Crate]) -> Result<()> {
    let mut output = File::create(".crates.json")?;

    for c in crates {
        writeln!(output, "{}", serde_json::to_string(c)?)?;
    }

    fs::rename(".crates.json", "crates.json")?;

    let mut output = File::create(".index.html")?;
    writeln!(output, "{}", OUTPUT_HEADER)?;
    for c in crates {
        write!(
            output,
            "<div class=\"row\" onclick=\"change_log(&quot;{}&quot;, &quot;{}&quot;)\"><div class=\"crate\">{} {}</div>",
            c.name, c.version, c.name, c.version
        )
        ?;
        write!(output, "<div class=\"status\">")?;
        match &c.status {
            Status::Unknown => write!(output, "Unknown"),
            Status::Passing => write!(output, "Passing"),
            Status::Error(cause) => write!(output, "Error: {}", cause),
            Status::UB { cause: causes, .. } => {
                write!(output, "UB: ")?;
                for cause in causes {
                    write!(output, "{}", cause.kind)?;
                    if let Some(source_crate) = &cause.source_crate {
                        write!(output, "{source_crate}")?;
                    }
                }
                Ok(())
            }
        }?;
        writeln!(output, "</div></div>")?;
    }
    write!(output, "</div></body></html>")?;

    fs::rename(".index.html", "index.html")?;

    let mut output = File::create(".ub.html")?;
    writeln!(output, "{}", OUTPUT_HEADER)?;
    for c in crates {
        if let Status::UB { cause: causes, .. } = &c.status {
            write!(
            output,
            "<div class=\"row\" onclick=\"change_log(&quot;{}&quot;, &quot;{}&quot;)\"><div class=\"crate\">{} {}</div>",
            c.name, c.version, c.name, c.version
        )
            ?;
            write!(output, "<div class=\"status\">")?;
            write!(output, "UB: ")?;
            for cause in causes {
                write!(output, "{}", cause.kind)?;
                if let Some(source_crate) = &cause.source_crate {
                    write!(output, "{source_crate}")?;
                }
            }
            writeln!(output, "</div></div>")?;
        }
    }

    fs::rename(".ub.html", "ub.html")?;
    Ok(())
}

fn diagnose(output: &str) -> Vec<Cause> {
    let mut causes = Vec::new();

    let lines = output.lines().collect::<Vec<_>>();

    for (l, line) in lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains("Undefined Behavior: "))
    {
        let end = lines
            .iter()
            .enumerate()
            .skip(l)
            .find_map(|(l, line)| {
                if line.trim().is_empty() {
                    Some(l)
                } else {
                    None
                }
            })
            .unwrap();

        let kind;
        if line.contains("uninitialized") {
            kind = "uninitialized memory".to_string();
        } else if line.contains("out-of-bounds") {
            kind = "invalid pointer offset".to_string();
        } else if line.contains("null pointer is not a valid pointer for this operation") {
            kind = "null pointer dereference".to_string();
        } else if line.contains("accessing memory with alignment") {
            kind = "misaligned pointer dereference".to_string();
        } else if line.contains("dangling reference") {
            kind = "dangling reference".to_string();
        } else if line.contains("unaligned reference") {
            kind = "unaligned reference".to_string();
        } else if line.contains("incorrect layout on deallocation") {
            kind = "incorrect layout on deallocation".to_string();
        } else if line.contains("borrow stack") || line.contains("reborrow") {
            if line.contains("<untagged>") {
                kind = "int-to-ptr cast".to_string();
            } else {
                kind = diagnose_sb(&lines[l..end]);
            }
        } else {
            kind = line
                .split("Undefined Behavior: ")
                .nth(1)
                .unwrap()
                .trim()
                .to_string();
        }

        let mut source_crate = None;

        for line in &lines[l..] {
            if line.contains("inside `") && line.contains(" at ") {
                let path = line.split(" at ").nth(1).unwrap();
                if path.contains("workdir") || !path.starts_with("/") {
                    break;
                } else if path.contains("/root/.cargo/registry/src/") {
                    let crate_name = path
                        .split("/root/.cargo/registry/src/github.com-1ecc6299db9ec823/")
                        .nth(1)
                        .unwrap()
                        .split("/")
                        .nth(0)
                        .unwrap();

                    source_crate = Some(format!("{}", crate_name));
                    break;
                }
            }
        }
        causes.push(Cause { kind, source_crate })
    }

    causes.sort();
    causes.dedup();
    causes
}

fn diagnose_sb(lines: &[&str]) -> String {
    if lines[0].contains("only grants SharedReadOnly") && lines[0].contains("for Unique") {
        String::from("&->&mut")
    } else if lines.iter().any(|line| line.contains("invalidated")) {
        String::from("SB-invalidation")
    } else if lines
        .iter()
        .any(|line| line.contains("created due to a retag at offsets [0x0..0x0]"))
    {
        String::from("SB-null-provenance")
    } else if lines[0].contains("does not exist in the borrow stack") {
        String::from("SB-use-outside-provenance")
    } else if lines[0].contains("no item granting write access for deallocation") {
        String::from("SB-invalid-dealloc")
    } else {
        String::from("SB-uncategorized")
    }
}
