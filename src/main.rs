use clap::Parser;
use crates_io_api::{CratesQuery, Sort, SyncClient};
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Crate {
    name: String,
    recent_downloads: Option<u64>,
    version: String,
    status: Status,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum Status {
    Unknown,
    Passing,
    Error(String),
    UB { cause: String, status: String },
}

fn main() {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let args = Args::parse();

    let mut crates = HashMap::new();

    if Path::new("crates.json").exists() {
        for line in fs::read_to_string("crates.json").unwrap().lines() {
            let krate: Crate = serde_json::from_str(&line).unwrap();
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
                        o.insert(Crate {
                            name: c.name,
                            recent_downloads: c.recent_downloads,
                            version: c.max_version,
                            status: Status::Unknown,
                        });
                    }
                }
                Entry::Vacant(v) => {
                    v.insert(Crate {
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

    fs::create_dir_all("logs").unwrap();

    let mut previously_run = Vec::new();
    for name in fs::read_dir("logs").unwrap() {
        let name = name.unwrap();
        for file in fs::read_dir(name.path()).unwrap() {
            let file = file.unwrap();
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

    render(&mut cursor.lock().unwrap().crates);

    let mut threads = Vec::new();
    for _ in 0..args.jobs {
        let cursor = cursor.clone();
        let previously_run = previously_run.clone();
        let handle = std::thread::spawn(move || {
            loop {
                let mut lock = cursor.lock().unwrap();

                let i = lock.next;
                lock.next += 1;

                let krate = lock.crates[i].clone();

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
                    .unwrap();

                let output = String::from_utf8_lossy(&res.stdout);

                // The container is supposed to redirect everything to stdout
                assert!(
                    res.stderr.is_empty(),
                    "{}",
                    String::from_utf8_lossy(&res.stderr)
                );

                fs::create_dir_all(format!("logs/{}", krate.name)).unwrap();
                fs::write(format!("logs/{}/{}", krate.name, krate.version), &*output).unwrap();

                let mut lock = cursor.lock().unwrap();
                lock.crates[i] = krate;
                render(&mut lock.crates);
            }
        });
        threads.push(handle);
    }

    for t in threads {
        t.join().unwrap();
    }
}

fn render(crates: &mut [Crate]) {
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
            write_crate_output(krate, &output);
        }
    }

    write_output(crates);
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

fn write_crate_output(krate: &Crate, output: &str) {
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
        let line = ansi_to_html::convert_escaped(line.trim()).unwrap();
        let line = line.replace("\u{1b}(B</span>", "</span>");
        encoded.push_str(&line);
        encoded.push('\n');
    }

    fs::create_dir_all(format!("logs/{}", krate.name)).unwrap();

    let mut file = File::create(format!("logs/{}/{}.html", krate.name, krate.version)).unwrap();

    write!(file, log_format!(), krate.name, krate.version, encoded).unwrap();
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
    padding: 1em 0 1em 1em;
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
    flex: 2;
}
.status {
    order: 2;
    flex: 1;
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

fn write_output(crates: &[Crate]) {
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
            "<div class=\"row\" onclick=\"change_log(&quot;{}&quot;, &quot;{}&quot;)\"><div class=\"crate\">{} {}</div>",
            c.name, c.version, c.name, c.version
        )
        .unwrap();
        write!(output, "<div class=\"status\">").unwrap();
        match &c.status {
            Status::Unknown => write!(output, "Unknown"),
            Status::Passing => write!(output, "Passing"),
            Status::Error(cause) => write!(output, "Error: {}", cause),
            Status::UB { cause, .. } => write!(output, "UB: {}", cause),
        }
        .unwrap();
        writeln!(output, "</div></div>").unwrap();
    }
    write!(output, "</div></body></html>").unwrap();

    fs::rename(".index.html", "index.html").unwrap();

    let mut output = File::create(".ub.html").unwrap();
    writeln!(output, "{}", OUTPUT_HEADER).unwrap();
    for c in crates {
        if let Status::UB { cause, .. } = &c.status {
            write!(
            output,
            "<div class=\"row\" onclick=\"change_log(&quot;{}&quot;, &quot;{}&quot;)\"><div class=\"crate\">{} {}</div>",
            c.name, c.version, c.name, c.version
        )
            .unwrap();
            write!(output, "<div class=\"status\">").unwrap();
            write!(output, "UB: {}", cause).unwrap();
            writeln!(output, "</div></div>").unwrap();
        }
    }

    fs::rename(".ub.html", "ub.html").unwrap();
}

fn diagnose(_output: &str) -> String {
    String::new()
    /*
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
    */
}

/*
const CRATES_ROOT: &str = "https://static.crates.io/crates";

lazy_static::lazy_static! {
    static ref CLIENT: ureq::Agent = ureq::Agent::new();
}

use flate2::read::GzDecoder;
use tar::Archive;

impl Crate {
    fn fetch_url(&self) -> String {
        format!(
            "{0}/{1}/{1}-{2}.crate",
            CRATES_ROOT, self.name, self.version
        )
    }

    fn fetch(&self) -> Result<(), Box<dyn std::error::Error>> {
        std::env::set_current_dir("/")?;
        std::fs::remove_dir_all("/build")?;
        std::fs::create_dir_all("/build")?;
        std::env::set_current_dir("/build")?;

        let path = Path::new("/build");

        let body = CLIENT.get(&self.fetch_url()).call()?.into_reader();
        let mut archive = Archive::new(GzDecoder::new(body));

        let entries = archive.entries()?;
        for entry in entries {
            let mut entry = entry?;
            let relpath = {
                let path = entry.path()?;
                path.into_owned()
            };
            let mut components = relpath.components();
            // Throw away the first path component
            components.next();
            let full_path = path.join(&components.as_path());
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            entry.unpack(&full_path)?;
        }
        Ok(())
    }
}
*/
