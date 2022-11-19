use color_eyre::eyre::Result;
use miri_the_world::{Crate, Status};
use rayon::prelude::*;
use std::{collections::HashMap, fmt::Write, fs, path::PathBuf};

use miri_the_world::load_completed_crates;

fn main() -> Result<()> {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    color_eyre::install()?;

    let crates = load_completed_crates()?;

    log::info!("Rendering");
    render(&crates)
}

fn render(crates: &HashMap<String, Vec<Crate>>) -> Result<()> {
    let flat_crates = crates.values().flat_map(|v| &v[..]).collect::<Vec<_>>();
    flat_crates.par_iter().try_for_each(|krate| -> Result<()> {
        let path = format!("logs/{}/{}", krate.name, krate.version);
        if let Ok(output) = fs::read_to_string(&path) {
            write_crate_output(krate, &output)?;
        }
        Ok(())
    })?;

    let mut crates = crates
        .iter()
        .filter_map(|(name, c)| {
            let version = c.iter().max_by(|a, b| a.version.cmp(&b.version));
            if version.is_none() {
                log::warn!("No versions found for {:?}", name);
            }
            version
        })
        .cloned()
        .collect::<Vec<_>>();
    crates.sort_by(|a, b| b.recent_downloads.cmp(&a.recent_downloads));

    write_output(&crates)
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
    let encoded = ansi_to_html::convert_escaped(&output);

    let encoded = encoded.replacen(
        "Undefined Behavior:",
        "<span id=\"ub\"></span>Undefined Behavior:",
        1,
    );

    fs::create_dir_all(format!("logs/{}", krate.name))?;

    let path = PathBuf::from(format!("logs/{}/{}.html", krate.name, krate.version));
    let html = format!(log_format!(), krate.name, krate.version, encoded);

    if path.exists() {
        let previous = std::fs::read_to_string(&path)?;
        if previous != html {
            std::fs::write(path, html)?;
        }
    } else {
        std::fs::write(path, html)?;
    }

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
    overflow: hidden;
}
a {
    color: #eee;
}
.row {
    text-indent: 1em;
    border-bottom: 1px solid #333;
    line-height: 1.5;
    vertical-align: top;
    margin-right: 0.5em;
    margin-left: 0.5em;
    padding-top: 0.5em;
    padding-bottom: 0.5em;
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
        params[tmp[0]] = decodeURIComponent(tmp[1]);
    }
    return params;
}
function crate_click() {
    if (event.target.classList.contains("row")) {
        let fields = event.target.innerHTML.split("<")[0].split(" ");
        let crate = fields[0];
        let version = fields[1];

        change_log(crate, version);
   }
}
let build_log;
function change_log(crate, version) {
    let html = "<object data=\"logs/" + crate + "/" + version + ".html\" width=100% height=100%></object>";
    if (build_log == undefined)  {
        build_log = document.getElementById("log");
    }
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
<div class="crates" onclick=crate_click()>
"#;

fn write_output(crates: &[Crate]) -> Result<()> {
    let mut output = String::from(LANDING_PAGE);
    for c in crates {
        writeln!(output, "\"{}\": [\"{}\"],", c.name, c.version)?;
    }
    output.pop();
    output.push_str("};</script></html>");
    fs::write(".index.html", output)?;
    fs::rename(".index.html", "index.html")?;

    let mut output = String::new();
    writeln!(output, "{}", OUTPUT_HEADER)?;
    for c in crates {
        write!(output, "<div class=\"row\">{} {}<br>", c.name, c.version,)?;
        match &c.status {
            Status::Unknown => write!(output, "Unknown"),
            Status::Passing => write!(output, "Passing"),
            Status::Error(cause) => write!(output, "Error: {}", cause),
            Status::UB { cause: causes, .. } => {
                write!(output, "UB: ")?;
                for cause in causes {
                    write!(output, "{}", cause.kind)?;
                    if let Some(source_crate) = &cause.source_crate {
                        write!(output, " ({source_crate})")?;
                    }
                    write!(output, ", ")?;
                }
                output.pop();
                output.pop();
                Ok(())
            }
        }?;
        writeln!(output, "</div>")?;
    }
    write!(output, "</div></body></html>")?;

    fs::write(".all.html", output)?;
    fs::rename(".all.html", "all.html")?;

    let mut output = String::new();
    writeln!(output, "{}", OUTPUT_HEADER)?;
    for c in crates {
        if let Status::UB { cause: causes, .. } = &c.status {
            write!(output, "<div class=\"row\">{} {}<br>", c.name, c.version,)?;
            write!(output, "UB: ")?;
            for cause in causes {
                write!(output, "{}", cause.kind)?;
                if let Some(source_crate) = &cause.source_crate {
                    write!(output, " ({source_crate})")?;
                }
                write!(output, ", ")?;
            }
            output.pop();
            output.pop();
            writeln!(output, "</div>")?;
        }
    }

    fs::write(".ub", output)?;
    fs::rename(".ub", "ub")?;
    Ok(())
}

const LANDING_PAGE: &str = r#"<!DOCTYPE HTML>
<html><head><style>
body {
    background: #111;
    color: #eee;
    font-family: sans-serif;
    font-size: 20px;
}
input {
    background: #111;
    color: #eee;
    font-family: monospace;
    font-size: 20px;
}
</style></head><body onload="init()">
<script>
function init() {
    document.getElementById("search").focus();
    document.getElementById("search").addEventListener("change", (event) => {
        let crate = event.target.value;
        let version = all[crate];
        if (version != undefined) {
            move_to(crate, version);
        }
    });

    var params = decode_params();
    if (params.crate != undefined && params.version != undefined) {
        move_to(params.crate, params.version);
    }
}
function move_to(crate, version) {
    let base = window.location.origin + window.location.pathname;
    window.location.href = base + "logs/" + crate + "/" + version + ".html"
}
function decode_params() {
    var params = {};
    var paramsarr = window.location.search.substr(1).split('&');
    for (var i = 0; i < paramsarr.length; ++i) {
        var tmp = paramsarr[i].split("=");
        if (!tmp[0] || !tmp[1]) continue;
        params[tmp[0]] = decodeURIComponent(tmp[1]);
    }
    return params;
}
</script>
<title>Miri build logs</title>
<p>Hello! This website hosts a library of logs, displayed as if you have just run <span style="font-family:monospace; font-size: 19px; background-color:#333;">cargo miri test</span> on every published crate on crates.io.
<p>Try searching for a crate below, if one is found you will be redirected to the build output for its most recently published version. For crates where Miri detects UB, the page will be automatically scrolled to the first UB report.
<div style="text-align: center">
<input id="search" style="width: 80%; height: 100%; margin: 0 auto;"></input>
<p><span id=search-result style="font-family:monospace; font-size: 19px;"></span>
</div>
<script>
const all =
{"#;
