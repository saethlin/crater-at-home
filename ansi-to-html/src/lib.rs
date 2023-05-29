mod ansi;
mod perform;
mod renderer;

use renderer::Renderer;

pub fn convert_escaped(name: String, ansi: &str) -> (String, String) {
    let mut renderer = Renderer::new(name.clone());
    let mut parser = vte::Parser::new();
    for byte in ansi.as_bytes() {
        parser.advance(&mut renderer, *byte);
    }
    let mut html = String::new();
    renderer.emit_html(&mut html);
    (renderer.emit_css(), html)
}
