use crate::{Cause, Crate, Status};

use color_eyre::Result;
use once_cell::sync::Lazy;
use regex::Regex;

static ANSI_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new("\x1b(\\[[0-9;?]*[A-HJKSTfhilmnsu]|\\(B)").unwrap());

pub fn diagnose(krate: &mut Crate, output: &str) -> Result<()> {
    let output = ANSI_REGEX.replace_all(output, "").to_string();
    // Strip ANSI escape codes from the output;
    krate.status = if output.contains("Undefined Behavior: ") {
        Status::UB {
            cause: diagnose_output(&output),
        }
    } else if output.contains("ERROR: AddressSanitizer: ") {
        if output
            .contains("WARNING: ASan is ignoring requested __asan_handle_no_return: stack type")
        {
            Status::Error("ASan false positive?".to_string())
        } else {
            Status::UB {
                cause: diagnose_asan(&output),
            }
        }
    } else if output.contains("SIGILL: illegal instruction") {
        Status::UB {
            cause: vec![Cause {
                kind: "SIGILL debug assertion".to_string(),
                source_crate: None,
            }],
        }
    } else if output.contains("attempted to leave type") {
        Status::UB {
            cause: vec![Cause {
                kind: "uninit type which does not permit uninit".to_string(),
                source_crate: None,
            }],
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
    Ok(())
}

fn diagnose_asan(output: &str) -> Vec<Cause> {
    let mut causes = Vec::new();

    let lines = output.lines().collect::<Vec<_>>();

    for line in lines
        .iter()
        .filter(|line| line.contains("ERROR: AddressSanitizer: "))
    {
        if line.contains("requested allocation size") {
            causes.push(Cause {
                kind: "requested allocation size exceeds maximum supported size".to_string(),
                source_crate: None,
            });
        } else if let Some(kind) = line.split_whitespace().nth(2) {
            causes.push(Cause {
                kind: kind.to_string(),
                source_crate: None,
            });
        }
    }
    causes.sort();
    causes.dedup();
    causes
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
            .unwrap_or(l + 1);

        let kind;
        if line.contains("Data race detected") {
            kind = "data race".to_string()
        } else if line.contains("encountered uninitialized")
            || line.contains("this operation requires initialized memory")
        {
            kind = "uninitialized memory".to_string();
        } else if line.contains("out-of-bounds") {
            kind = "invalid pointer offset".to_string();
        } else if line.contains("dereferencing pointer failed: null pointer is not a valid pointer")
        {
            kind = "null pointer dereference".to_string();
        } else if line.contains("encountered 0, but expected something greater or equal to 1") {
            kind = "zero-initialized nonzero type".to_string();
        } else if line.contains("encountered a null reference") {
            kind = "null reference".to_string();
        } else if line.contains("accessing memory with alignment") {
            kind = "misaligned pointer dereference".to_string();
        } else if line.contains("dangling reference") {
            kind = "dangling reference".to_string();
        } else if line.contains("unaligned reference") {
            kind = "unaligned reference".to_string();
        } else if line.contains("incorrect layout on deallocation") {
            kind = "incorrect layout on deallocation".to_string();
        } else if line.contains("deallocating while") && line.contains("is protected") {
            kind = "deallocation conflict with dereferenceable".to_string();
        } else if line.contains("attempting a write access")
            && line.contains("only grants SharedReadOnly")
        {
            kind = "SB-write-via-&".to_string();
        } else if line.contains("borrow stack")
            || line.contains("reborrow")
            || line.contains("retag")
        {
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
        } else if line.contains("type validation failed") {
            let second = line.split(": encountered").nth(1).unwrap().trim();
            kind = format!("type validation failed: encountered {}", second);
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
                if path.contains("/root/build") || !path.starts_with('/') {
                    break;
                } else if path.contains("/root/.cargo/registry/src/") {
                    let crate_name = path.split('/').nth(6).unwrap();
                    source_crate = Some(crate_name.to_string());
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
