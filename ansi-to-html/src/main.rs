use std::io::{copy, stdin, stdout};

fn main() {
    env_logger::init();
    let lock = stdin().lock();
    let mut handle = ansi_to_html::Handle::new(lock);
    copy(&mut handle, &mut stdout().lock()).unwrap();
}
