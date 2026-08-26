//! Phase C — process-level error surface of the linked programs
//! (`add_executable(driver src/main.c)` vs `[[bin]] driver`).
//!
//! Rows E14–E17 and C25 of ERRORS.md / CONFIGS.md: the descriptors the program
//! is handed can themselves fail, and a C program's startup state (in
//! particular its `SIGPIPE` disposition) is part of the behaviour being
//! translated.

mod common;

use common::*;

use std::io::Write;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
use std::process::{Command, Stdio};

extern "C" {
    fn close(fd: std::os::raw::c_int) -> std::os::raw::c_int;
}

/// stdout and exit description of a run.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    stdout: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

/// E14 — stdin is `/dev/null`: EOF straight away, so `scanf` reports an input
/// failure and `x` keeps its initializer.
#[test]
fn e14_stdin_is_dev_null() {
    let mut outs = Vec::new();
    for exe in [c_exe(), rust_exe()] {
        let out = Command::new(exe)
            .stdin(Stdio::null())
            .output()
            .expect("spawn with /dev/null stdin");
        outs.push(Outcome {
            stdout: out.stdout,
            code: out.status.code(),
            signal: out.status.signal(),
        });
    }
    assert_eq!(
        String::from_utf8_lossy(&outs[0].stdout),
        "00000000030000000000000000000040\n",
        "E14: unexpected C result"
    );
    assert_eq!(outs[0].code, Some(0));
    assert_eq!(outs[0], outs[1], "E14: /dev/null stdin diverged");
}

/// E15 — stdin cannot be read (it is a directory, so `read` fails with
/// `EISDIR`): `scanf` reports an input failure and `x` keeps its initializer.
#[test]
fn e15_stdin_read_error() {
    let dir = std::fs::File::open(manifest_dir()).expect("open manifest dir as a file");
    let mut outs = Vec::new();
    for exe in [c_exe(), rust_exe()] {
        let dir = dir.try_clone().expect("clone dir fd");
        let out = Command::new(exe)
            .stdin(Stdio::from(dir))
            .output()
            .expect("spawn with a directory as stdin");
        outs.push(Outcome {
            stdout: out.stdout,
            code: out.status.code(),
            signal: out.status.signal(),
        });
    }
    assert_eq!(
        String::from_utf8_lossy(&outs[0].stdout),
        "00000000030000000000000000000040\n",
        "E15: unexpected C result"
    );
    assert_eq!(outs[0].code, Some(0));
    assert_eq!(outs[0], outs[1], "E15: unreadable stdin diverged");
}

/// E16 — stdout is closed before the program starts: every `printf` fails with
/// `EBADF`, the C code ignores the return value, and the program still exits 0.
#[test]
fn e16_stdout_closed() {
    let mut outs = Vec::new();
    for exe in [c_exe(), rust_exe()] {
        let mut cmd = Command::new(exe);
        cmd.stdin(Stdio::piped()).stderr(Stdio::piped());
        unsafe {
            // Runs in the child between fork and exec.
            cmd.pre_exec(|| {
                close(1);
                Ok(())
            });
        }
        let mut child = cmd.spawn().expect("spawn with closed stdout");
        let _ = child.stdin.take().expect("stdin").write_all(b"5\n");
        let out = child.wait_with_output().expect("wait");
        outs.push(Outcome {
            stdout: out.stdout,
            code: out.status.code(),
            signal: out.status.signal(),
        });
    }
    assert_eq!(outs[0].code, Some(0), "E16: C must still exit 0");
    assert_eq!(outs[0].signal, None, "E16: C must not be killed");
    assert_eq!(outs[0], outs[1], "E16: closed stdout diverged");
}

/// E17 — the read end of the stdout pipe is closed before the program writes:
/// the write fails with `EPIPE` and raises `SIGPIPE`.
///
/// A C program starts with the default `SIGPIPE` disposition, so it is killed
/// by the signal. The translated program must reproduce that, which means
/// restoring `SIG_DFL` (the Rust runtime installs `SIG_IGN`).
#[test]
fn e17_stdout_pipe_reader_closed() {
    fn run(exe: &Path) -> Outcome {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn");
        // Close the read end before the child gets a chance to write.
        drop(child.stdout.take());
        let mut stdin = child.stdin.take().expect("stdin");
        let _ = stdin.write_all(b"5\n");
        drop(stdin);
        let status = child.wait().expect("wait");
        Outcome {
            stdout: Vec::new(),
            code: status.code(),
            signal: status.signal(),
        }
    }

    let c = run(c_exe());
    let r = run(rust_exe());
    assert_eq!(
        c.signal,
        Some(13),
        "E17: the C program is expected to die from SIGPIPE, got {c:?}"
    );
    assert_eq!(
        r, c,
        "E17: SIGPIPE handling diverged\n  C   : {c:?}\n  Rust: {r:?}"
    );
}

/// C25 — the C `main` is declared without parameters and ignores `argv`
/// entirely; extra command-line arguments must change nothing.
#[test]
fn c25_extra_argv_ignored() {
    for args in [
        vec![],
        vec!["extra"],
        vec!["-h"],
        vec!["--help"],
        vec!["1", "2", "3"],
        vec![""],
    ] {
        let mut outs = Vec::new();
        for exe in [c_exe(), rust_exe()] {
            let mut child = Command::new(exe)
                .args(&args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn with argv");
            let _ = child.stdin.take().expect("stdin").write_all(b"9\n");
            let out = child.wait_with_output().expect("wait");
            outs.push(Outcome {
                stdout: out.stdout,
                code: out.status.code(),
                signal: out.status.signal(),
            });
        }
        assert_eq!(
            String::from_utf8_lossy(&outs[0].stdout),
            "09000000030000000000000000000040\n",
            "C25: unexpected C result for argv {args:?}"
        );
        assert_eq!(outs[0], outs[1], "C25: argv {args:?} diverged");
    }
}
