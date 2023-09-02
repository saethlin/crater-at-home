#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let _ = ansi_to_html::convert_escaped(String::new(), data);
});
