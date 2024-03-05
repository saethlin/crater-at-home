use std::io::Read;

mod ansi;
mod perform;
mod renderer;

pub struct Renderer<R> {
    inner: renderer::Renderer,
    parser: vte::Parser,
    bytes: R,
    state: State,
    title: String,
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
</style><title>{}</title></head>
<script>
function scroll_to_ub() {{
    var ub = document.getElementById("ub");
    if (ub !== null) {{
        ub.scrollIntoView();
    }}
}}
</script>
<body onload="scroll_to_ub()">
<pre style="text-align: center;">{}</pre>
<pre>"#
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Fresh,
    Rendering,
    Done,
}

impl<R: Read> Renderer<R> {
    pub fn new(bytes: R, title: String) -> Self {
        Self {
            inner: renderer::Renderer::new(String::new()),
            parser: vte::Parser::new(),
            bytes,
            state: State::Fresh,
            title,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.state == State::Done
    }

    fn first_line(&self) -> Vec<u8> {
        format!(log_format!(), self.title, self.title).into_bytes()
    }

    pub fn next_line(&mut self) -> Result<Option<Vec<u8>>, std::io::Error> {
        match self.state {
            State::Done => return Ok(None),
            State::Fresh => {
                self.state = State::Rendering;
                return Ok(Some(self.first_line()));
            }
            State::Rendering => {}
        }
        loop {
            let mut byte = [0u8];
            let n = self.bytes.read(&mut byte)?;
            if n == 0 {
                let line = if let Some(line) = self.inner.remove_oldest_row() {
                    Some(line)
                } else {
                    self.state = State::Done;
                    let mut line = Vec::new();
                    line.extend(b"</span></pre></body><style>");
                    self.inner.emit_css(&mut line)?;
                    line.extend(b"</style></html>");
                    line.into()
                };
                return Ok(line);
            }
            self.parser.advance(&mut self.inner, byte[0]);
            if let Some(mut line) = self.inner.pop_completed_row() {
                if line.is_empty() {
                    log::warn!("renderer produced an empty line!");
                    line = b"\n".to_vec();
                }
                return Ok(Some(line));
            }
        }
    }
}
