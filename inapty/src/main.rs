use std::fs::File;
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::ptr;

fn main() {
    let mut pty: i32 = 0;

    let mut termios = libc::termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_line: 0,
        c_cc: [0; 32],
        c_ispeed: 0,
        c_ospeed: 0,
    };
    // SAFETY: Out param is a valid pointer to the right type
    unsafe {
        libc::tcgetattr(0, &mut termios);
    }

    let mut winsz = libc::winsize {
        ws_col: 0,
        ws_row: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: Out param is a valid pointer to the right type
    unsafe {
        libc::ioctl(0, libc::TIOCGWINSZ, &mut winsz);
    }

    // SAFETY: Pointer arguments are valid (and thus ignored) or null
    let pid = unsafe { libc::forkpty(&mut pty, ptr::null_mut(), &termios, &winsz) };

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
