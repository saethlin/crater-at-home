mod ansi;
mod perform;
mod renderer;

use renderer::Renderer;

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

fn main() {
    let mut renderer = Renderer::default();
    let mut html = String::new();
    for line in std::fs::read_to_string("logs").unwrap().lines() {
        println!("{}", line);
        let bytes = std::fs::read(line).unwrap();
        let mut parser = vte::Parser::new();
        for byte in bytes {
            parser.advance(&mut renderer, byte);
        }
        renderer.emit_html(&mut html);
        std::fs::write(
            format!("{}.html", line),
            format!(log_format!(), "test", "crate", html).as_bytes(),
        )
        .unwrap();
        renderer.clear();
    }
}
