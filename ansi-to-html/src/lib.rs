use std::io::Write;

mod ansi;
mod perform;
mod renderer;

pub struct Handle {
    renderer: renderer::Renderer,
    parser: vte::Parser,
}

impl Handle {
    pub fn new() -> Self {
        Self {
            renderer: renderer::Renderer::new(String::new()),
            parser: vte::Parser::new(),
        }
    }

    pub fn finish<F: Write>(&self, mut output: F) -> std::io::Result<()> {
        output.write_all(
            br#"<!DOCTYPE html><html><body style="background:#111;color:#eee;"><body>
            <pre style="word-wrap:break-word;white-space:pre-wrap;font-size:14px;font-size-adjust:none;text-size-adjust:none;-webkit-text-size-adjust:100%;-moz-text-size-adjust:100%;-ms-text-size-adjust:100%;"></style>"#
        )?;
        self.renderer.emit_html(&mut output)?;
        output.write_all(b"</pre></body><style>")?;
        self.renderer.emit_css(&mut output)?;
        output.write_all(b"</style></html>")
    }
}

impl Write for Handle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for b in buf {
            self.parser.advance(&mut self.renderer, *b);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub fn render(name: String, bytes: &[u8]) -> (String, String) {
    let mut h = Handle {
        renderer: renderer::Renderer::new(name),
        parser: vte::Parser::new(),
    };
    h.write_all(&bytes).unwrap();
    let mut html = Vec::new();
    h.renderer.emit_html(&mut html).unwrap();
    let mut css = Vec::new();
    h.renderer.emit_css(&mut css).unwrap();

    (
        String::from_utf8(css).unwrap(),
        String::from_utf8(html).unwrap(),
    )
}
