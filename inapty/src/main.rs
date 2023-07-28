use std::fs::File;
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::ptr;

fn main() {
    let winsz = libc::winsize {
        ws_col: 256,
        ws_row: 64,
        ws_xpixel: 2560,
        ws_ypixel: 1408,
    };

    let mut pty: i32 = 0;

    // SAFETY: Pointer arguments are valid (and thus ignored) or null
    let pid = unsafe { libc::forkpty(&mut pty, ptr::null_mut(), ptr::null_mut(), &winsz) };

    if pid == 0 {
        // We are the child. Spawn the subprocess based off our arguments.
        let mut args = std::env::args_os().skip(1);
        Command::new(args.next().unwrap()).args(args).exec();
        unreachable!("exec should not return");
    } else {
        // We are the originating process. Copy from the pty to output.
        // SAFETY: master is open and valid, it was just opened by forkpty
        let mut pty = unsafe { File::from_raw_fd(pty) };
        // Copy all the output from the child pty to our stdout
        while std::io::copy(&mut pty, &mut std::io::stdout()).is_ok() {}
        // Exit according to our child's status
        // SAFETY: No preconditions
        let status = unsafe {
            let mut status = 0;
            libc::waitpid(pid, &mut status, 0);
            libc::WEXITSTATUS(status)
        };
        std::process::exit(status);
    }
}
