use std::io::{copy, stdin, stdout, BufWriter};

fn main() {
    env_logger::init();
    let out = BufWriter::new(stdout().lock());
    let mut handle = ansi_to_html::Handle::new(out);
    copy(&mut stdin().lock(), &mut handle).unwrap();
    handle.finish().unwrap()
}
