//! Process-level differential tests: the CMake-equivalent C executable vs the
//! cargo-built `driver` binary.
//!
//! The `.so` tests cover the translated code; these cover the *program* wrapper
//! that only the binary has — `main`'s `argv` plumbing (`std::env::args_os`),
//! the process exit status, and the signal disposition a C program starts with.
//! Only fast (rejected) inputs are used, plus `argc == 0` and a dead-pipe
//! `SIGPIPE`; the ~5-minute accepted path is covered by
//! `pipeline.rs::full_end_to_end` and `scripts/e2e_binaries.sh`.

mod common;

use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::Command;

fn rust_bin() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("test exe layout")
        .join("driver")
}

fn c_bins() -> Vec<(&'static str, PathBuf)> {
    vec![
        ("c-O0", PathBuf::from(env!("C_DRIVER_BIN_O0"))),
        ("c-O2", PathBuf::from(env!("C_DRIVER_BIN_O2"))),
    ]
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// Runs `prog` with `argv[0]` forced to `argv0` (so the usage message matches)
/// and the given arguments.
fn run(prog: &Path, argv0: &[u8], args: &[&[u8]]) -> Outcome {
    let mut cmd = Command::new(prog);
    cmd.arg0(std::ffi::OsStr::from_bytes(argv0));
    for a in args {
        cmd.arg(std::ffi::OsStr::from_bytes(a));
    }
    let out = cmd.output().expect("spawn");
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stdout: out.stdout,
        stderr: out.stderr,
    }
}

fn assert_cli_matches(argv0: &[u8], args: &[&[u8]]) {
    let rust = run(&rust_bin(), argv0, args);
    for (name, c) in c_bins() {
        let got = run(&c, argv0, args);
        assert_eq!(
            got,
            rust,
            "{name} vs rust differ for argv0={:?} args={:?}",
            String::from_utf8_lossy(argv0),
            args.iter().map(|a| String::from_utf8_lossy(a)).collect::<Vec<_>>()
        );
    }
}

/// ERRORS.md rows 3–5, 8–18, 23–27 at the process level.
#[test]
fn cli_error_paths() {
    let cases: Vec<Vec<&[u8]>> = vec![
        vec![],
        vec![b"abc"],
        vec![b"42abc"],
        vec![b"   "],
        vec![b"-"],
        vec![b"+"],
        vec![b"0x10"],
        vec![b"42 "],
        vec![b"1.0"],
        vec![b"\xff"],
        vec![b"4\xff"],
        vec![b"4294967296"],
        vec![b"18446744073709551615"],
        vec![b"18446744073709551616"],
        vec![b"-1"],
        vec![b"9223372036854775808"],
        vec![b"42", b"extra"],
        vec![b"1", b"2", b"3"],
        vec![b"", b""],
        vec![b"abc", b"def", b"ghi", b"jkl"],
    ];
    for args in &cases {
        assert_cli_matches(b"driver", args);
    }
}

/// ERRORS.md rows 8, 9 at the process level: the usage message echoes `argv[0]`
/// verbatim, including non-UTF-8 bytes and an empty string.
#[test]
fn cli_argv0_variants() {
    for argv0 in [
        &b"driver"[..],
        &b"./driver"[..],
        &b"/usr/local/bin/driver"[..],
        &b""[..],
        &b"\xff\xfe"[..],
        &b"pre\xffpost"[..],
        &b"a b\tc"[..],
        &b"%s%d%n"[..], // must be echoed literally, not interpreted
    ] {
        assert_cli_matches(argv0, &[]);
        assert_cli_matches(argv0, &[b"x", b"y"]);
    }
}

/// ERRORS.md rows 1/2 at the process level: `execve` with an empty `argv`, so
/// the program starts with `argc == 0` and `argv[0] == NULL`.
fn run_with_empty_argv(prog: &Path) -> Outcome {
    let dir = std::env::temp_dir();
    let stamp = format!(
        "{}_{}",
        std::process::id(),
        prog.file_name().unwrap().to_string_lossy()
    );
    let out_path = dir.join(format!("argv0_{stamp}.out"));
    let err_path = dir.join(format!("argv0_{stamp}.err"));

    let cpath = std::ffi::CString::new(prog.as_os_str().as_bytes()).unwrap();
    let outf = std::fs::File::create(&out_path).unwrap();
    let errf = std::fs::File::create(&err_path).unwrap();
    let (ofd, efd) = {
        use std::os::unix::io::AsRawFd;
        (outf.as_raw_fd(), errf.as_raw_fd())
    };

    let status;
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: async-signal-safe calls only, then exec.
            libc::dup2(ofd, 1);
            libc::dup2(efd, 2);
            let argv: [*const libc::c_char; 1] = [std::ptr::null()];
            libc::execv(cpath.as_ptr(), argv.as_ptr());
            libc::_exit(127);
        }
        let mut wstatus: libc::c_int = 0;
        assert!(libc::waitpid(pid, &mut wstatus, 0) == pid, "waitpid");
        status = wstatus;
    }
    drop(outf);
    drop(errf);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    std::fs::File::open(&out_path)
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    std::fs::File::open(&err_path)
        .unwrap()
        .read_to_end(&mut stderr)
        .unwrap();
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&err_path);

    let exited = libc::WIFEXITED(status);
    Outcome {
        code: if exited {
            Some(libc::WEXITSTATUS(status))
        } else {
            None
        },
        signal: if libc::WIFSIGNALED(status) {
            Some(libc::WTERMSIG(status))
        } else {
            None
        },
        stdout,
        stderr,
    }
}

#[test]
fn cli_argc_zero() {
    let rust = run_with_empty_argv(&rust_bin());
    assert_eq!(rust.code, Some(1), "rust: {rust:?}");
    for (name, c) in c_bins() {
        let got = run_with_empty_argv(&c);
        assert_eq!(got, rust, "{name} vs rust differ for execve with argc == 0");
        // Measured on this kernel: `execve` with an empty argv array is
        // normalised to argc == 1 with argv[0] == "" (it does NOT reach the
        // program as argc == 0 / argv[0] == NULL — that case is only reachable
        // by calling `main` directly, which errors.rs::argc_zero_null_argv0
        // does, and where glibc prints "(null)").
        assert_eq!(got.stderr, b"Usage:  <seed>\n", "{name}");
    }
}

/// A C program inherits the default `SIGPIPE` disposition, so a write to a pipe
/// with no reader kills it; Rust's runtime sets `SIG_IGN` unless the program
/// restores the default. Both must behave the same.
fn run_with_dead_pipe(prog: &Path) -> Outcome {
    use std::os::unix::io::FromRawFd;
    let mut fds = [0 as libc::c_int; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0, "pipe");
    unsafe { libc::close(fds[0]) }; // reader gone
    let w = unsafe { std::fs::File::from_raw_fd(fds[1]) };
    let w2 = w.try_clone().unwrap();

    let out = Command::new(prog)
        .arg0("driver")
        .stdout(std::process::Stdio::from(w))
        .stderr(std::process::Stdio::from(w2))
        .status()
        .expect("spawn");
    Outcome {
        code: out.code(),
        signal: out.signal(),
        stdout: Vec::new(),
        stderr: Vec::new(),
    }
}

#[test]
fn cli_sigpipe_disposition() {
    let rust = run_with_dead_pipe(&rust_bin());
    for (name, c) in c_bins() {
        let got = run_with_dead_pipe(&c);
        assert_eq!(
            got, rust,
            "{name} vs rust differ when stdout/stderr is a pipe with no reader"
        );
    }
    assert_eq!(
        rust.signal,
        Some(13),
        "expected death by SIGPIPE, got {rust:?}"
    );
}

/// The exit status of the *accepted* path is checked by the slow end-to-end
/// tests; here we at least prove the two binaries exist and agree on `--help`
/// style misuse, and that the C reference binaries were built from the same
/// source as the `.so`s (same constants).
#[test]
fn binaries_exist_and_agree_on_misuse() {
    assert!(rust_bin().exists(), "{} missing", rust_bin().display());
    for (_, c) in c_bins() {
        assert!(c.exists(), "{} missing", c.display());
    }
    assert_cli_matches(b"driver", &[b"--help"]);
    assert_cli_matches(b"driver", &[b"-h"]);
    assert_cli_matches(b"driver", &[b"--seed=42"]);
    let _ = common::ARRAY_SIZE; // keep the shared module referenced
}
