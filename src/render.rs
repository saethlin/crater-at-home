use crate::{Crate, Status};
use anyhow::Result;
use std::fmt::Write;

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
    let path = window.location.pathname;
    let base = path.slice(0, path.lastIndexOf('/'));
    let html = "<object data=\"" + base + "/logs/" + crate + "/" + encodeURIComponent(version) + "\" width=100% height=100%></object>";
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
</style></head><body><pre>
Click on a crate to the right to display its build log
</pre></body></html>
</object>
</div>
<div class="crates" onclick=crate_click()>
"#;

pub fn render_ub(crates: &[(Crate, Status)]) -> Result<String> {
    let mut output = String::new();
    writeln!(output, "{OUTPUT_HEADER}")?;
    for (c, status) in crates {
        if let Status::UB { cause: causes, .. } = &status {
            write!(output, "<div class=\"row\">{} {}<br>", c.name, c.version,)?;
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

    Ok(output)
}
