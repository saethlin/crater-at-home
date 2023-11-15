use std::io::Write;

mod ansi;
mod perform;
mod renderer;

pub struct Handle<W> {
    renderer: renderer::Renderer<W>,
    parser: vte::Parser,
}

impl<W: Write> Handle<W> {
    pub fn new(mut out: W) -> Self {
        out.write_all(
            br#"<!DOCTYPE html><html><body style="background:#111;color:#eee;"><body>
            <pre style="word-wrap:break-word;white-space:pre-wrap;font-size:14px;font-size-adjust:none;text-size-adjust:none;-webkit-text-size-adjust:100%;-moz-text-size-adjust:100%;-ms-text-size-adjust:100%;"></style>"#
        ).unwrap();
        Self {
            renderer: renderer::Renderer::new(out, String::new()),
            parser: vte::Parser::new(),
        }
    }

    pub fn finish(&mut self) -> std::io::Result<()> {
        self.renderer.emit_html()?;
        self.renderer.out.write_all(b"</pre></body><style>")?;
        self.renderer.emit_css()?;
        self.renderer.out.write_all(b"</style></html>")
    }
}

impl<W: Write> Write for Handle<W> {
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
    let mut html = Vec::new();
    let mut handle = Handle {
        renderer: renderer::Renderer::new(&mut html, name),
        parser: vte::Parser::new(),
    };
    handle.write_all(&bytes).unwrap();
    handle.renderer.emit_html().unwrap();

    let mut css = Vec::new();
    handle.renderer.out = &mut css;
    handle.renderer.emit_css().unwrap();

    (
        String::from_utf8(css).unwrap(),
        String::from_utf8(html).unwrap(),
    )
}
