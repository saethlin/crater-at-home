use std::io::{copy, stdin, stdout};

fn main() {
    env_logger::init();
    let mut handle = ansi_to_html::Handle::new();
    copy(&mut stdin().lock(), &mut handle).unwrap();
    handle.finish(stdout().lock()).unwrap();
}
