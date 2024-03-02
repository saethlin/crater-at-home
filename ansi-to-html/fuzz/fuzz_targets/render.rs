#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::Write;
use ansi_to_html::Renderer;

fuzz_target!(|data: &[u8]| {
    let mut renderer = Renderer::new(data);
    while let Some(line) = renderer.next_line().unwrap() {
        std::io::sink().write_all(&line).unwrap();
    }
    assert!(matches!(renderer.next_line(), Ok(None)));
});
