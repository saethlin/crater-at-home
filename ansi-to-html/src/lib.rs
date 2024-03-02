use std::collections::VecDeque;
use std::io::Read;

mod ansi;
mod perform;
mod renderer;

pub struct Handle<R> {
    renderer: renderer::Renderer,
    parser: vte::Parser,
    bytes: R,
    finished_line: Option<VecDeque<u8>>,
    wrote_final_line: bool,
}

const FIRST_LINE: &str = r#"<!DOCTYPE html><html><body style="background:#111;color:#eee;"><body>
<pre style="word-wrap:break-word;white-space:pre-wrap;font-size:14px;font-size-adjust:none;text-size-adjust:none;-webkit-text-size-adjust:100%;-moz-text-size-adjust:100%;-ms-text-size-adjust:100%;"></style><span>"#;

impl<R: Read> Handle<R> {
    pub fn new(bytes: R) -> Self {
        Self {
            renderer: renderer::Renderer::new(String::new()),
            parser: vte::Parser::new(),
            bytes,
            finished_line: Some(FIRST_LINE.as_bytes().to_vec().into()),
            wrote_final_line: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.wrote_final_line && self.finished_line.is_none()
    }
}

impl<R> Read for Handle<R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        assert!(buf.len() > 0);
        while self.finished_line.is_none() && !self.wrote_final_line {
            let mut byte = [0u8];
            let n = self.bytes.read(&mut byte)?;
            if n == 0 {
                log::info!("input exhausted, finalizing");
                if let Some(line) = self.renderer.remove_oldest_row() {
                    self.finished_line = Some(line.into());
                } else {
                    log::info!("starting last line");
                    let mut line = Vec::new();
                    line.extend(b"</span></pre></body><style>");
                    self.renderer.emit_css(&mut line)?;
                    line.extend(b"</style></html>");
                    self.finished_line = Some(line.into());
                    self.wrote_final_line = true;
                }
                break;
            }
            self.parser.advance(&mut self.renderer, byte[0]);
            if let Some(line) = self.renderer.pop_completed_row() {
                if line.is_empty() {
                    panic!("empty line??");
                }
                self.finished_line = Some(line.into());
            }
        }
        let Some(line) = self.finished_line.as_mut() else {
            return Ok(0);
        };
        let n = buf.len().min(line.len());
        for (src, dst) in line.drain(..n).zip(buf.iter_mut().take(n)) {
            *dst = src;
        }
        if line.is_empty() {
            self.finished_line = None;
        }
        if n == 0 {
            log::info!("no bytes, eof");
        }
        Ok(n)
    }
}

pub fn render(name: String, bytes: &[u8]) -> (String, String) {
    let mut h = Handle {
        renderer: renderer::Renderer::new(name),
        parser: vte::Parser::new(),
        bytes,
        finished_line: None,
        wrote_final_line: false,
    };
    let mut html = Vec::new();
    h.read_to_end(&mut html).unwrap();
    (String::new(), String::from_utf8(html).unwrap())
}
