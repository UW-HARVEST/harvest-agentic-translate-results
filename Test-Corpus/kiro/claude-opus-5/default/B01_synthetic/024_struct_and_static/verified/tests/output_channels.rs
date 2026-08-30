//! Differential tests for the shape of the *output channel* rather than the
//! input. `c_src/src/main.c` ignores every `printf` return value and always
//! `return 0`, but the process can still die before reaching that `return` if a
//! write raises `SIGPIPE`. These cases pin that behavior down.
//!
//! Unix only: they depend on file descriptors and signal dispositions.

#![cfg(unix)]

mod common;

use common::{c_bin, rust_bin};

use std::io::Read;
use std::os::fd::{AsFd, OwnedFd};
use std::path::Path;
use std::process::{Command, Stdio};

/// (exit code, terminating signal, stderr bytes)
type Outcome = (Option<i32>, Option<i32>, Vec<u8>);

fn outcome(mut child: std::process::Child) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    let mut err = Vec::new();
    child
        .stderr
        .take()
        .expect("piped stderr")
        .read_to_end(&mut err)
        .expect("read stderr");
    let status = child.wait().expect("wait for child");
    (status.code(), status.signal(), err)
}

/// Run `bin` with stdout wired to a pipe whose read end is closed before the
/// program produces output, so the write is guaranteed to hit `EPIPE`.
fn with_closed_pipe_reader(bin: &Path) -> Outcome {
    let (reader, writer) = std::io::pipe().expect("create pipe");
    let child = Command::new(bin)
        .stdin(Stdio::null())
        .stdout(Stdio::from(OwnedFd::from(writer)))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    // Both ends held by this process are dropped here; the child holds the only
    // remaining write end and there is no reader left.
    drop(reader);
    outcome(child)
}

/// Run `bin` with file descriptor 1 closed outright.
fn with_stdout_closed(bin: &Path) -> Outcome {
    extern "C" {
        fn close(fd: core::ffi::c_int) -> core::ffi::c_int;
    }

    let mut cmd = Command::new(bin);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    // Safety: `close` is async-signal-safe and this runs in the forked child
    // between fork and exec.
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| {
            close(1);
            Ok(())
        });
    }
    outcome(cmd.spawn().unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display())))
}

/// Run `bin` with stdout pointed at `/dev/full`, where every write fails with
/// `ENOSPC`.
fn with_dev_full(bin: &Path) -> Outcome {
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .expect("open /dev/full");
    let child = Command::new(bin)
        .stdin(Stdio::null())
        .stdout(Stdio::from(full.as_fd().try_clone_to_owned().expect("dup fd")))
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", bin.display()));
    outcome(child)
}

#[test]
fn write_to_closed_pipe_matches() {
    let c = with_closed_pipe_reader(c_bin());
    let r = with_closed_pipe_reader(rust_bin());
    assert_eq!(
        c, r,
        "closed pipe reader: C {c:?} vs Rust {r:?} (C dies by SIGPIPE; the Rust \
         runtime ignores SIGPIPE unless the default disposition is restored)"
    );
}

#[test]
fn write_to_closed_stdout_matches() {
    let c = with_stdout_closed(c_bin());
    let r = with_stdout_closed(rust_bin());
    assert_eq!(c, r, "stdout closed: C {c:?} vs Rust {r:?}");
}

#[test]
fn write_to_dev_full_matches() {
    if !Path::new("/dev/full").exists() {
        // Not a skipped assertion: the device simply does not exist here, so
        // there is no C behavior to compare against.
        return;
    }
    let c = with_dev_full(c_bin());
    let r = with_dev_full(rust_bin());
    assert_eq!(c, r, "/dev/full: C {c:?} vs Rust {r:?}");
}

#[test]
fn stdout_to_regular_file_matches() {
    let dir = std::env::temp_dir();
    let mut paths = Vec::new();
    for (tag, bin) in [("c", c_bin()), ("rust", rust_bin())] {
        let path = dir.join(format!("driver_out_{tag}_{}.txt", std::process::id()));
        let file = std::fs::File::create(&path).expect("create output file");
        let child = Command::new(bin)
            .stdin(Stdio::null())
            .stdout(Stdio::from(file))
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        let out = outcome(child);
        assert_eq!(out.0, Some(0));
        assert_eq!(out.1, None);
        assert!(out.2.is_empty());
        paths.push(path);
    }
    let c_bytes = std::fs::read(&paths[0]).expect("read C output");
    let r_bytes = std::fs::read(&paths[1]).expect("read Rust output");
    assert_eq!(c_bytes, r_bytes, "file-redirected stdout differs");
    for p in paths {
        let _ = std::fs::remove_file(p);
    }
}
