use std::collections::{BTreeMap, HashMap};

use clap::Parser;
use color_eyre::eyre::Result;
use miri_the_world::{load_completed_crates, Status};
use regex::Regex;

#[derive(Parser)]
struct Args {
    #[clap(long)]
    by_source_crate: bool,
}

fn main() -> Result<()> {
    if std::env::var("RUST_BACKTRACE").is_err() {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    color_eyre::install()?;

    let args = Args::parse();

    let crates = load_completed_crates()?;

    let tag_re = Regex::new("<\\d+>").unwrap();
    let alloc_re = Regex::new("alloc\\d+").unwrap();
    let offset_re = Regex::new("alloc\\d+\\[0x\\d+\\]").unwrap();
    let call_re = Regex::new("call \\d+").unwrap();
    let hex_re = Regex::new("0x[0-9a-f]+").unwrap();

    let mut times = vec![];
    let mut total_time = 0;
    let mut states: HashMap<_, usize> = HashMap::new();
    let mut errored: BTreeMap<&str, u64> = BTreeMap::new();
    let mut ub = 0;
    let mut known = 0;
    for krate in crates.iter() {
        let crate_time = match krate.time {
            Some(t) => t,
            None => continue,
        };
        total_time += crate_time;
        match &krate.status {
            Status::Unknown => continue,
            Status::Passing => {
                known += 1;
            }
            Status::Error(err) => {
                *errored.entry(&err).or_default() += 1;
                known += 1;
            }
            Status::UB { cause: causes, .. } => {
                for cause in causes {
                    let mut cause = cause.clone();
                    cause.kind = tag_re.replace_all(&cause.kind, "<tag>").to_string();
                    cause.kind = offset_re.replace_all(&cause.kind, "offset").to_string();
                    cause.kind = alloc_re.replace_all(&cause.kind, "alloc").to_string();
                    cause.kind = call_re.replace_all(&cause.kind, "call").to_string();
                    cause.kind = hex_re.replace_all(&cause.kind, "[hex]").to_string();
                    let total_cause = if args.by_source_crate && cause.source_crate.is_some() {
                        format!("{} {}", cause.kind, cause.source_crate.unwrap())
                    } else {
                        cause.kind.clone()
                    };
                    *states.entry(total_cause).or_default() += 1;
                }
                ub += 1;
                known += 1;
            }
        }
        let mut time = crate_time as usize / 60;
        let rm = crate_time as usize % 60;
        if rm != 0 {
            time += 1;
        }
        assert!(
            time < 60 * 24,
            "looks like your crate took more than 24h to compute, that is probably a bug"
        );
        if times.len() <= time {
            times.resize(time + 1, 0);
        }
        times[time] += 1;
    }
    let time_per_crate = total_time / known;

    let mut states: Vec<_> = states.into_iter().collect();
    states.sort();
    states.sort_by_key(|(_, i)| usize::max_value() - *i);

    for (error, count) in errored {
        println!("error({error}): {count} ({}%)", count * 100 / known);
    }
    println!("ub: {ub} ({}%)", ub * 100 / known);
    let seconds_remaining = (crates.len() as u64 - known) * time_per_crate;
    println!(
        "time per crate (MM:SS): {}:{}",
        time_per_crate / 60,
        time_per_crate % 60
    );
    println!(
        "time remaining (HH:MM): {}:{}",
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
