use crate::{Cause, Crate, Status};
use std::fs;

use color_eyre::Result;

pub fn diagnose(krate: &mut Crate) -> Result<()> {
    let path = format!("logs/{}/{}", krate.name, krate.version);
    if let Ok(output) = fs::read_to_string(&path) {
        krate.status = if output.contains("Undefined Behavior: ") {
            Status::UB {
                cause: diagnose_output(&output),
                status: String::new(),
            }
        } else if output.contains("Command exited with non-zero status 124") {
            Status::Error("Timeout".to_string())
        } else if output.contains("Command exited with non-zero status 255") {
            Status::Error("OOM".to_string())
        } else if output.contains("Command exited with non-zero status") {
            Status::Error(String::new())
        } else {
            Status::Passing
        };
    }
    Ok(())
}

fn diagnose_output(output: &str) -> Vec<Cause> {
    let mut causes = Vec::new();

    let lines = output.lines().collect::<Vec<_>>();

    for (l, line) in lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains("Undefined Behavior: "))
    {
        let end = lines
            .iter()
            .enumerate()
            .skip(l)
            .find_map(|(l, line)| {
                if line.trim().is_empty() {
                    Some(l)
                } else {
                    None
                }
            })
            .unwrap();

        let kind;
        if line.contains("uninitialized") {
            kind = "uninitialized memory".to_string();
        } else if line.contains("out-of-bounds") {
            kind = "invalid pointer offset".to_string();
        } else if line.contains("dereferencing pointer failed: null pointer is not a valid pointer")
        {
            kind = "null pointer dereference".to_string();
        } else if line.contains("accessing memory with alignment") {
            kind = "misaligned pointer dereference".to_string();
        } else if line.contains("dangling reference") {
            kind = "dangling reference".to_string();
        } else if line.contains("unaligned reference") {
            kind = "unaligned reference".to_string();
        } else if line.contains("incorrect layout on deallocation") {
            kind = "incorrect layout on deallocation".to_string();
        } else if line.contains("borrow stack") || line.contains("reborrow") {
            if line.contains("<untagged>") {
                kind = "int-to-ptr cast".to_string();
            } else {
                kind = diagnose_sb(&lines[l..end]);
            }
        } else if line.contains("type validation failed")
            && line.contains("encountered pointer")
            && line.contains("expected initialized plain (non-pointer) bytes")
        {
            kind = "ptr-int transmute".to_string();
        } else {
            kind = line
                .split("Undefined Behavior: ")
                .nth(1)
                .unwrap()
                .trim()
                .to_string();
        }

        let mut source_crate = None;

        for line in &lines[l..] {
            if line.contains("inside `") && line.contains(" at ") {
                let path = line.split(" at ").nth(1).unwrap();
                if path.contains("workdir") || !path.starts_with("/") {
                    break;
                } else if path.contains("/root/.cargo/registry/src/") {
                    let crate_name = path
                        .split("/root/.cargo/registry/src/github.com-1ecc6299db9ec823/")
                        .nth(1)
                        .unwrap()
                        .split("/")
                        .nth(0)
                        .unwrap();

                    source_crate = Some(format!("{}", crate_name));
                    break;
                }
            }
        }
        causes.push(Cause { kind, source_crate })
    }

    causes.sort();
    causes.dedup();
    causes
}

fn diagnose_sb(lines: &[&str]) -> String {
    if lines[0].contains("only grants SharedReadOnly") && lines[0].contains("for Unique") {
        String::from("&->&mut")
    } else if lines.iter().any(|line| {
        line.contains("attempting a write access") && line.contains("only grants SharedReadOnly")
    }) {
        String::from("write through pointer based on &")
    } else if lines.iter().any(|line| line.contains("invalidated")) {
        String::from("SB-invalidation")
    } else if lines
        .iter()
        .any(|line| line.contains("created due to a retag at offsets [0x0..0x0]"))
    {
        String::from("SB-null-provenance")
    } else if lines[0].contains("does not exist in the borrow stack") {
        String::from("SB-use-outside-provenance")
    } else if lines[0].contains("no item granting write access for deallocation") {
        String::from("SB-invalid-dealloc")
    } else {
        String::from("SB-uncategorized")
    }
}
