//! Process-level edge cases that the piped harness in `common` cannot express:
//! unwritable stdout, a closed stdin descriptor, and a reader that hangs up.
//!
//! The C program ignores every `printf` return value and unconditionally
//! `return 0`, so all of these must still exit 0 — but the Rust program could
//! easily diverge here by panicking on a write error or by inheriting a
//! different SIGPIPE disposition.

mod common;

use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};

use common::{c_binary, rust_binary};

fn write_input(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("driver_io_{name}_{}", std::process::id()));
    std::fs::write(&path, bytes).expect("cannot stage input file");
    path
}

/// Runs `exe` with stdout pointed at `stdout_target`, returning (code, signal, stderr).
fn run_with_stdout_file(
    exe: &Path,
    input: &Path,
    stdout_target: &Path,
) -> (Option<i32>, Option<i32>, Vec<u8>) {
    let out = Command::new(exe)
        .stdin(Stdio::from(File::open(input).expect("cannot open input")))
        .stdout(Stdio::from(
            File::options()
                .write(true)
                .open(stdout_target)
                .expect("cannot open stdout target"),
        ))
        .stderr(Stdio::piped())
        .output()
        .expect("failed to run program");

    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        out.status.signal()
    };
    #[cfg(not(unix))]
    let signal = None;

    (out.status.code(), signal, out.stderr)
}

#[test]
fn unwritable_stdout_is_ignored_by_both() {
    // /dev/full accepts opens but fails every write with ENOSPC.
    let full = Path::new("/dev/full");
    if !full.exists() {
        // Not a Linux-like host; nothing to compare.
        return;
    }
    let input = write_input("devfull", b"1 2 3\n");

    let c = run_with_stdout_file(&c_binary(), &input, full);
    let r = run_with_stdout_file(&rust_binary(), &input, full);

    assert_eq!(c.0, r.0, "exit code mismatch with stdout=/dev/full");
    assert_eq!(c.1, r.1, "exit signal mismatch with stdout=/dev/full");
    assert_eq!(c.2, r.2, "stderr mismatch with stdout=/dev/full");

    let _ = std::fs::remove_file(input);
}

#[test]
fn discarded_stdout_is_ignored_by_both() {
    let null = Path::new("/dev/null");
    if !null.exists() {
        return;
    }
    for (name, bytes) in [
        ("null_empty", &b""[..]),
        ("null_happy", &b"1 2 3"[..]),
        ("null_stage2", &b"1"[..]),
    ] {
        let input = write_input(name, bytes);
        let c = run_with_stdout_file(&c_binary(), &input, null);
        let r = run_with_stdout_file(&rust_binary(), &input, null);
        assert_eq!(c, r, "mismatch with stdout=/dev/null for case {name}");
        let _ = std::fs::remove_file(input);
    }
}

#[cfg(unix)]
#[test]
fn closed_stdin_descriptor_behaves_like_eof() {
    use std::os::unix::process::CommandExt;

    fn run_with_closed_stdin(exe: &Path) -> (Option<i32>, Option<i32>, Vec<u8>, Vec<u8>) {
        let mut cmd = Command::new(exe);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        unsafe {
            // Close fd 0 outright, so every read fails with EBADF rather than
            // reporting a clean end of file.
            cmd.pre_exec(|| {
                libc_close(0);
                Ok(())
            });
        }
        let out = cmd.output().expect("failed to run with closed stdin");
        use std::os::unix::process::ExitStatusExt;
        (
            out.status.code(),
            out.status.signal(),
            out.stdout,
            out.stderr,
        )
    }

    let c = run_with_closed_stdin(&c_binary());
    let r = run_with_closed_stdin(&rust_binary());
    assert_eq!(c, r, "mismatch when fd 0 is closed");
}

#[cfg(unix)]
fn libc_close(fd: i32) {
    // Declared locally to avoid adding a dependency on the `libc` crate.
    extern "C" {
        fn close(fd: i32) -> i32;
    }
    unsafe {
        close(fd);
    }
}

#[cfg(unix)]
#[test]
fn reader_hangup_on_stdout() {
    // Spawn with a piped stdout, drop the read end immediately, then let the
    // program write. C is killed by SIGPIPE; the Rust program must behave the
    // same rather than swallowing EPIPE and exiting 0.
    fn run_with_dropped_reader(exe: &Path) -> (Option<i32>, Option<i32>) {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn failed");

        // Close the read end before the child gets a chance to write.
        drop(child.stdout.take().expect("stdout piped"));

        {
            use std::io::Write;
            let mut sink = child.stdin.take().expect("stdin piped");
            let _ = sink.write_all(b"1 2 3\n");
        }

        let status = child.wait().expect("wait failed");
        use std::os::unix::process::ExitStatusExt;
        (status.code(), status.signal())
    }

    let c = run_with_dropped_reader(&c_binary());
    let r = run_with_dropped_reader(&rust_binary());
    assert_eq!(
        c, r,
        "mismatch when the stdout reader hangs up (C={c:?}, RS={r:?})"
    );
}
