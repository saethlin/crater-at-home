use clap::Parser;
use color_eyre::eyre::Result;
use miri_the_world::{Crate, Status};
use rayon::prelude::*;
use std::{
    fs::{self, File},
    io::Write,
};

#[derive(Parser)]
struct Args {}

fn main() -> Result<()> {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    color_eyre::install()?;

    let _args = Args::parse();

    let mut crates = Vec::new();

    for line in fs::read_to_string("crates.json")?.lines() {
        let krate: Crate = serde_json::from_str(&line)?;
        crates.push(krate);
    }

    crates.sort_by(|a, b| b.recent_downloads.cmp(&a.recent_downloads));

    render(&crates)
}

fn render(crates: &[Crate]) -> Result<()> {
    crates.par_iter().try_for_each(|krate| -> Result<()> {
        log::info!("Processing {} {}", krate.name, krate.version);
        let path = format!("logs/{}/{}", krate.name, krate.version);
        if let Ok(output) = fs::read_to_string(&path) {
            write_crate_output(krate, &output)?;
        }
        Ok(())
    })?;

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
    let mut output = File::create(".index.html")?;
    writeln!(output, "{}", OUTPUT_HEADER)?;
    for c in crates {
        log::info!("Rendering {} {}", c.name, c.version);
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
                        write!(output, " ({source_crate})")?;
                    }
                    write!(output, ",")?;
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
