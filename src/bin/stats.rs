use std::collections::HashMap;
use std::fs;

use color_eyre::eyre::Result;
use miri_the_world::*;

fn main() -> Result<()> {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    color_eyre::install()?;

    let mut crates = HashMap::new();
    for line in fs::read_to_string("crates.json")?.lines() {
        let krate: Crate = serde_json::from_str(&line)?;
        crates.insert(krate.name.clone(), krate);
    }

    let mut times = vec![];
    for krate in crates.values() {
        let time = krate.time as usize / 60;
        if times.len() <= time {
            times.resize(time + 1, 0);
        }
        times[time] += 1;
    }

    for (i, time) in times.iter().enumerate() {
        println!("{}: {}", i, time);
    }

    Ok(())
}
