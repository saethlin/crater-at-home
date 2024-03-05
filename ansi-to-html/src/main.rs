use std::io::{stdin, stdout, Write};

fn main() -> Result<(), std::io::Error> {
    env_logger::init();
    let mut renderer = ansi_to_html::Renderer::new(stdin().lock(), String::from("stdin"));
    let mut out = stdout().lock();
    while let Some(line) = renderer.next_line()? {
        out.write_all(&line)?;
    }
    Ok(())
}
