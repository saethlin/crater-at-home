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
    let mut states: HashMap<_, usize> = HashMap::new();
    let mut errored = 0;
    for krate in crates.values() {
        match &krate.status {
            Status::Unknown => continue,
            Status::Passing => {}
            Status::Error(_) => {
                errored += 1;
                continue;
            }
            Status::UB { cause, .. } => {
                *states.entry(cause).or_default() += 1;
            }
        }
        let mut time = krate.time as usize / 60;
        let rm = krate.time as usize % 60;
        if rm != 0 {
            time += 1;
        }
        if times.len() <= time {
            times.resize(time + 1, 0);
        }
        times[time] += 1;
    }

    let mut states: Vec<_> = states.into_iter().collect();
    states.sort();
    states.sort_by_key(|(_, i)| usize::max_value() - *i);

    println!("errored: {}", errored);
    println!();
    println!("histogram over time taken to run each crate");
    print_histogram(
        times
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .map(|(i, n)| (Ok(i), n)),
    );
    println!();
    println!("histogram over kind of UB");
    print_histogram(states.iter().map(|&(k, v)| (Err(&**k), v)));

    Ok(())
}

fn print_histogram<'a>(entries: impl Iterator<Item = (Result<usize, &'a str>, usize)> + Clone) {
    let max = entries.clone().map(|(_, x)| x).max().unwrap();
    for (k, v) in entries {
        match k {
            Ok(i) => print!("{:2}: ", i),
            Err(msg) => println!("{}", msg),
        }
        print!("{:5} ", v);
        for _ in 0..(v * 50 / max) {
            print!("#");
        }
        println!();
    }
}
