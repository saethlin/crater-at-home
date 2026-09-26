use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::ptr;

fn main() {
    let winsz = libc::winsize {
        ws_col: 512,
        ws_row: 64,
        ws_xpixel: 5120,
        ws_ypixel: 1408,
    };

    let mut pty: i32 = 0;

    // SAFETY: Pointer arguments are valid (and thus ignored) or null
    let pid = unsafe { libc::forkpty(&mut pty, ptr::null_mut(), ptr::null_mut(), &winsz) };

    if pid == 0 {
        // We are the child. Spawn the subprocess based off our arguments.
        let mut args = std::env::args_os().skip(1);
        let program = args.next().unwrap();
        // exec only returns if it failed, in which case it hands back the reason why.
        let err = Command::new(&program).args(args).exec();
        eprintln!("inapty: failed to exec {}: {}", program.display(), err);
        // Same codes a shell reports for a command it could not run.
        std::process::exit(if err.kind() == ErrorKind::NotFound {
            127
        } else {
            126
        });
    } else {
        // We are the originating process. Copy from the pty to output.
        // SAFETY: master is open and valid, it was just opened by forkpty
        let mut pty = unsafe { File::from_raw_fd(pty) };

        let mut buf = [0u8; 8192];
        let mut stdout = std::io::stdout().lock();
        // Once our own output is gone there is nothing useful left to do with the child's bytes,
        // but we must keep draining the pty anyway. If we stop reading, the child blocks forever
        // writing into a full pty buffer and the waitpid below never returns.
        let mut stdout_broken = false;
        loop {
            let n = match pty.read(&mut buf) {
                // The child closed the pty. On Linux this is reported as EIO below instead, but
                // handle a real EOF too so that we cannot spin here.
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                // Any other error means the pty is done; the child has exited or is about to.
                Err(_) => break,
            };
            if !stdout_broken && stdout.write_all(&buf[..n]).is_err() {
                stdout_broken = true;
            }
        }
        let _ = stdout.flush();

        // Exit according to our child's status
        let mut status = 0;
        // SAFETY: No preconditions
        while unsafe { libc::waitpid(pid, &mut status, 0) } == -1 {
            let err = std::io::Error::last_os_error();
            // A signal can interrupt the wait before the child is reaped; retrying is the only
            // way to still learn its status.
            if err.kind() == ErrorKind::Interrupted {
                continue;
            }
            // Anything else means we will never learn it. `status` is still untouched here, so
            // using it would report a clean exit for a child we know nothing about.
            eprintln!("inapty: waitpid failed: {err}");
            std::process::exit(125);
        }

        let status = if libc::WIFSIGNALED(status) {
            // WEXITSTATUS is only meaningful for a child that exited normally; for one killed
            // by a signal it reports 0, which would make a segfault or an OOM kill look like
            // a clean run. Report it the way a shell does instead.
            128 + libc::WTERMSIG(status)
        } else {
            libc::WEXITSTATUS(status)
        };
        std::process::exit(status);
    }
}
