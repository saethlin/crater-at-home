use color_eyre::eyre::Result;
use miri_the_world::*;
use rayon::prelude::*;
use std::{fmt::Write, fs, path::PathBuf};

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

    render(&crates)
}

fn render(crates: &[Crate]) -> Result<()> {
    crates.par_iter().try_for_each(|krate| -> Result<()> {
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
    let output = output.replace("\r\n", "\n");
    let encoded = ansi_to_html::convert_escaped(&output)?;

    let encoded = encoded.replacen(
        "Undefined Behavior:",
        "</pre><pre id=\"ub\">Undefined Behavior:",
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
    let mut output = String::new();
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
                        write!(output, " ({source_crate})")?;
                    }
                    write!(output, ", ")?;
                }
                output.pop();
                output.pop();
                Ok(())
            }
        }?;
        writeln!(output, "</div></div>")?;
    }
    write!(output, "</div></body></html>")?;

    fs::write(".index.html", output)?;
    fs::rename(".index.html", "index.html")?;

    let mut output = String::new();
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
                    write!(output, " ({source_crate})")?;
                }
                write!(output, ", ")?;
            }
            output.pop();
            output.pop();
            writeln!(output, "</div></div>")?;
        }
    }

    fs::write(".ub.html", output)?;
    fs::rename(".ub.html", "ub.html")?;
    Ok(())
}
