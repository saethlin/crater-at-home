use std::io::Read;

#[rustfmt::skip]
macro_rules! page_template {
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
{}
</style></head>
<body><pre>{}</pre></body></html>"#
    }
}

fn main() {
    env_logger::init();
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let (css, html) = ansi_to_html::convert_escaped(&input);
    print!(page_template!(), css, html);
}
