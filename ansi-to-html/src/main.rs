use std::io::{Read, Write};
fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let rendered = ansi_to_html::convert_escaped(&input);
    std::io::stdout().write_all(rendered.as_bytes()).unwrap();
}
