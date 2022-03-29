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
    let mut total_time = 0;
    let mut states: HashMap<_, usize> = HashMap::new();
    let mut errored = 0;
    let mut ub = 0;
    let mut known = 0;
    for krate in crates.values() {
        total_time += krate.time;
        match &krate.status {
            Status::Unknown => continue,
            Status::Passing => {
                known += 1;
            }
            Status::Error(_) => {
                errored += 1;
                known += 1;
                continue;
            }
            Status::UB { cause: causes, .. } => {
                for cause in causes {
                    *states.entry(cause.kind.clone()).or_default() += 1;
                }
                ub += 1;
                known += 1;
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
    let time_per_crate = total_time / known;

    let mut states: Vec<_> = states.into_iter().collect();
    states.sort();
    states.sort_by_key(|(_, i)| usize::max_value() - *i);

    println!("errored: {errored} ({}%)", errored * 100 / known);
    println!("ub: {ub} ({}%)", ub * 100 / known);
    println!("done: {}%", known * 100 / crates.len() as u64);
    let seconds_remaining = (crates.len() as u64 - known) * time_per_crate;
    println!(
        "time remaining: {}:{}",
        seconds_remaining / 3600,
        (seconds_remaining % 3600) / 60
    );
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
    print_histogram(states.iter().map(|(k, v)| (Err(k.clone()), *v)));

    Ok(())
}

fn print_histogram(entries: impl Iterator<Item = (Result<usize, String>, usize)> + Clone) {
    let max = entries.clone().map(|(_, x)| x).max().unwrap_or_default();
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
