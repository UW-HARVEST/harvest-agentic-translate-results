//! Differential tests: run the C binary and the Rust binary as subprocesses and
//! compare stdout, stderr and exit status byte for byte / value for value.
//!
//! The Rust code is never called as a library — both programs are driven exactly
//! the way a shell would drive them.
//!
//! The C program under test is:
//!
//! ```c
//! int main() {
//!     printf("Hello World!\n");
//!     return 0;
//! }
//! ```
//!
//! `main` takes no parameters, so `argc`/`argv` are unreachable; no input is
//! read, so there is no parsing, no length check and no error path. The only
//! things that can vary between the two implementations are therefore
//! environmental: what is on stdin, what is in `argv`, what the streams are
//! connected to, and how the process reacts when writing to stdout fails. Every
//! one of those classes is exercised below.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

// ---------------------------------------------------------------------------
// Locating / building the two binaries
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Path to the Rust binary produced by cargo for this integration test.
fn rust_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_driver"))
}

/// Path to the C binary, building it with cmake on first use if needed.
fn c_bin() -> &'static Path {
    static C_BIN: OnceLock<PathBuf> = OnceLock::new();
    C_BIN.get_or_init(|| {
        let c_src = workspace_root().join("c_src");
        let build = c_src.join("build");
        let exe = build.join("driver");
        if !exe.exists() {
            std::fs::create_dir_all(&build).expect("create c_src/build");
            let cfg = Command::new("cmake")
                .arg("..")
                .current_dir(&build)
                .output()
                .expect("run cmake (is cmake installed?)");
            assert!(
                cfg.status.success(),
                "cmake configure failed:\n{}\n{}",
                String::from_utf8_lossy(&cfg.stdout),
                String::from_utf8_lossy(&cfg.stderr)
            );
            let bld = Command::new("cmake")
                .args(["--build", "."])
                .current_dir(&build)
                .output()
                .expect("run cmake --build");
            assert!(
                bld.status.success(),
                "cmake build failed:\n{}\n{}",
                String::from_utf8_lossy(&bld.stdout),
                String::from_utf8_lossy(&bld.stderr)
            );
        }
        assert!(exe.exists(), "C binary missing at {}", exe.display());
        exe
    })
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

/// How a program's run ended, in a form that can be compared exactly.
#[derive(Debug, PartialEq, Eq)]
struct Ended {
    code: Option<i32>,
    #[cfg(unix)]
    signal: Option<i32>,
}

fn ended(out: &Output) -> Ended {
    Ended {
        code: out.status.code(),
        #[cfg(unix)]
        signal: out.status.signal(),
    }
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:x?}"),
    }
}

/// Run one program with the given argv tail, stdin bytes and extra env vars.
fn run(exe: &Path, args: &[&str], stdin_bytes: &[u8], env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().unwrap_or_else(|e| panic!("spawn {}: {e}", exe.display()));
    {
        let mut si = child.stdin.take().expect("stdin piped");
        // The programs never read stdin, so the write may fail once the child has
        // already exited and the pipe is gone. That is not a test failure.
        let _ = si.write_all(stdin_bytes);
        let _ = si.flush();
    }
    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("wait {}: {e}", exe.display()))
}

/// Assert stdout, stderr and exit status are identical for both programs.
fn assert_same(case: &str, args: &[&str], stdin_bytes: &[u8], env: &[(&str, &str)]) {
    let c = run(c_bin(), args, stdin_bytes, env);
    let r = run(rust_bin(), args, stdin_bytes, env);

    assert_eq!(
        c.stdout,
        r.stdout,
        "[{case}] stdout differs\n  C   : {}\n  Rust: {}",
        show(&c.stdout),
        show(&r.stdout)
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{case}] stderr differs\n  C   : {}\n  Rust: {}",
        show(&c.stderr),
        show(&r.stderr)
    );
    assert_eq!(
        ended(&c),
        ended(&r),
        "[{case}] exit status differs\n  C   : {:?}\n  Rust: {:?}",
        ended(&c),
        ended(&r)
    );
}

// ---------------------------------------------------------------------------
// Baseline: the one and only code path in the C program
// ---------------------------------------------------------------------------

#[test]
fn baseline_no_args_no_stdin() {
    assert_same("baseline", &[], b"", &[]);
}

/// Pin the exact bytes the C program emits, so a future edit to the Rust side
/// cannot silently agree with a broken C invocation.
#[test]
fn output_is_exactly_hello_world_newline() {
    let c = run(c_bin(), &[], b"", &[]);
    let r = run(rust_bin(), &[], b"", &[]);
    assert_eq!(c.stdout, b"Hello World!\n", "C stdout changed unexpectedly");
    assert_eq!(r.stdout, b"Hello World!\n");
    assert!(c.stderr.is_empty() && r.stderr.is_empty());
    assert_eq!(c.status.code(), Some(0));
    assert_eq!(r.status.code(), Some(0));
    // Exactly one trailing newline, no CR, no double newline.
    assert_eq!(r.stdout.iter().filter(|&&b| b == b'\n').count(), 1);
    assert!(!r.stdout.contains(&b'\r'));
}

// ---------------------------------------------------------------------------
// Input classes on stdin. The C program never reads stdin (no scanf, no fgets),
// so all of these must be ignored identically, including the "maximum" case.
// ---------------------------------------------------------------------------

#[test]
fn stdin_empty() {
    assert_same("stdin: empty", &[], b"", &[]);
}

#[test]
fn stdin_single_item() {
    assert_same("stdin: single item", &[], b"1\n", &[]);
}

#[test]
fn stdin_single_item_no_trailing_newline() {
    assert_same("stdin: single item, no newline", &[], b"1", &[]);
}

#[test]
fn stdin_only_a_newline() {
    assert_same("stdin: bare newline", &[], b"\n", &[]);
}

#[test]
fn stdin_only_whitespace() {
    assert_same("stdin: whitespace", &[], b"   \t \r\n  \n", &[]);
}

#[test]
fn stdin_multiple_items_across_lines() {
    assert_same("stdin: multiline", &[], b"3\n1 2 3\nextra\n", &[]);
}

#[test]
fn stdin_non_numeric_garbage() {
    // Would reach a scanf/atoi error path in a program that parsed input.
    assert_same("stdin: garbage", &[], b"not a number at all\n", &[]);
}

#[test]
fn stdin_numeric_extremes_and_overflow() {
    for s in [
        "0\n",
        "-1\n",
        "2147483647\n",
        "2147483648\n",
        "-2147483648\n",
        "-2147483649\n",
        "4294967296\n",
        "9223372036854775808\n",
        "99999999999999999999999999\n",
    ] {
        assert_same("stdin: numeric extreme", &[], s.as_bytes(), &[]);
    }
}

#[test]
fn stdin_binary_and_invalid_utf8() {
    let bytes: Vec<u8> = vec![0x00, 0xff, 0xfe, 0x80, b'\n', 0x01, 0x7f, 0xc3, 0x28];
    assert_same("stdin: binary", &[], &bytes, &[]);
}

#[test]
fn stdin_large_maximum_ish_payload() {
    // 1 MiB — far more than any fixed buffer the C might have had; must still be
    // ignored by both programs without truncation, blocking or error output.
    let big = vec![b'x'; 1024 * 1024];
    assert_same("stdin: 1 MiB", &[], &big, &[]);
}

#[test]
fn stdin_many_lines() {
    let mut s = Vec::new();
    for i in 0..10_000 {
        s.extend_from_slice(format!("{i}\n").as_bytes());
    }
    assert_same("stdin: 10k lines", &[], &s, &[]);
}

#[test]
fn stdin_closed_dev_null() {
    // stdin at EOF from the very first read, via /dev/null rather than a pipe.
    let mk = |exe: &Path| {
        let devnull = std::fs::File::open("/dev/null").expect("open /dev/null");
        Command::new(exe)
            .stdin(Stdio::from(devnull))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("run with /dev/null stdin")
    };
    let c = mk(c_bin());
    let r = mk(rust_bin());
    assert_eq!(c.stdout, r.stdout, "stdout differs with /dev/null stdin");
    assert_eq!(c.stderr, r.stderr, "stderr differs with /dev/null stdin");
    assert_eq!(ended(&c), ended(&r), "status differs with /dev/null stdin");
}

// ---------------------------------------------------------------------------
// argv classes. `int main()` ignores them; verify neither program reacts.
// ---------------------------------------------------------------------------

#[test]
fn args_ignored() {
    for args in [
        vec![],
        vec!["one"],
        vec!["--help"],
        vec!["-h"],
        vec!["--version"],
        vec![""],
        vec!["a", "b", "c"],
        vec!["-1", "0", "2147483648"],
        vec!["arg with spaces", "tab\there"],
    ] {
        assert_same("argv", &args, b"", &[]);
    }
}

#[test]
fn many_args() {
    let owned: Vec<String> = (0..200).map(|i| format!("arg{i}")).collect();
    let args: Vec<&str> = owned.iter().map(String::as_str).collect();
    assert_same("argv: 200 args", &args, b"", &[]);
}

// ---------------------------------------------------------------------------
// Environment: locale must not change the ASCII output or the exit status.
// ---------------------------------------------------------------------------

#[test]
fn locale_does_not_change_output() {
    for env in [
        vec![("LC_ALL", "C")],
        vec![("LC_ALL", "en_US.UTF-8")],
        vec![("LC_ALL", "de_DE.UTF-8"), ("LANG", "de_DE.UTF-8")],
        vec![("LC_ALL", "tr_TR.UTF-8")],
        vec![("LC_NUMERIC", "de_DE.UTF-8")],
    ] {
        assert_same("env: locale", &[], b"", &env);
    }
}

// ---------------------------------------------------------------------------
// Stream targets: a file (block buffered) versus a pipe, and stdout redirected
// to the same place as stderr.
// ---------------------------------------------------------------------------

#[test]
fn stdout_to_regular_file_matches() {
    let dir = std::env::temp_dir();
    let run_to_file = |exe: &Path, tag: &str| -> (Vec<u8>, Ended) {
        let path = dir.join(format!("driver_diff_{tag}_{}.out", std::process::id()));
        let f = std::fs::File::create(&path).expect("create temp file");
        let out = Command::new(exe)
            .stdin(Stdio::null())
            .stdout(Stdio::from(f))
            .stderr(Stdio::piped())
            .output()
            .expect("run with file stdout");
        let bytes = std::fs::read(&path).expect("read temp file");
        let _ = std::fs::remove_file(&path);
        assert!(out.stderr.is_empty(), "unexpected stderr from {tag}");
        (bytes, ended(&out))
    };
    let (cb, cs) = run_to_file(c_bin(), "c");
    let (rb, rs) = run_to_file(rust_bin(), "rust");
    assert_eq!(cb, rb, "file-redirected stdout differs");
    assert_eq!(cs, rs, "status differs with file stdout");
}

#[test]
fn stdout_and_stderr_merged_to_one_file() {
    // 2>&1 into a single file: catches any ordering/interleaving difference.
    let dir = std::env::temp_dir();
    let run_merged = |exe: &Path, tag: &str| -> (Vec<u8>, Ended) {
        let path = dir.join(format!("driver_merged_{tag}_{}.out", std::process::id()));
        let f = std::fs::File::create(&path).expect("create temp file");
        let f2 = f.try_clone().expect("dup fd");
        let out = Command::new(exe)
            .stdin(Stdio::null())
            .stdout(Stdio::from(f))
            .stderr(Stdio::from(f2))
            .output()
            .expect("run with merged output");
        let bytes = std::fs::read(&path).expect("read temp file");
        let _ = std::fs::remove_file(&path);
        (bytes, ended(&out))
    };
    let (cb, cs) = run_merged(c_bin(), "c");
    let (rb, rs) = run_merged(rust_bin(), "rust");
    assert_eq!(cb, rb, "merged stdout+stderr differs");
    assert_eq!(cs, rs, "status differs with merged output");
}

#[cfg(unix)]
#[test]
fn stdout_closed_fd() {
    // Shell equivalent: `driver >&-`. printf fails; C does not check it and
    // still returns 0.
    let run_closed = |exe: &Path| -> Output {
        let devnull = std::fs::File::open("/dev/null").expect("open /dev/null");
        // A read-only fd as stdout makes every write fail with EBADF.
        Command::new(exe)
            .stdin(Stdio::null())
            .stdout(Stdio::from(devnull))
            .stderr(Stdio::piped())
            .output()
            .expect("run with unwritable stdout")
    };
    let c = run_closed(c_bin());
    let r = run_closed(rust_bin());
    assert_eq!(c.stderr, r.stderr, "stderr differs with unwritable stdout");
    assert_eq!(ended(&c), ended(&r), "status differs with unwritable stdout");
}

#[cfg(unix)]
#[test]
fn stdout_pipe_with_closed_reader_sigpipe() {
    // Deterministic broken pipe: create a pipe, drop the read end, hand the
    // write end to the child as stdout. The first write raises SIGPIPE.
    let run_broken = |exe: &Path| -> Output {
        let (reader, writer) = std::io::pipe().expect("create pipe");
        drop(reader);
        Command::new(exe)
            .stdin(Stdio::null())
            .stdout(Stdio::from(writer))
            .stderr(Stdio::piped())
            .output()
            .expect("run with broken stdout pipe")
    };
    let c = run_broken(c_bin());
    let r = run_broken(rust_bin());
    assert_eq!(c.stderr, r.stderr, "stderr differs on broken stdout pipe");
    assert_eq!(
        ended(&c),
        ended(&r),
        "status differs on broken stdout pipe (Rust ignores SIGPIPE by default)"
    );
    // Document the ground truth: C is killed by SIGPIPE (13), not exit 0.
    assert_eq!(c.status.signal(), Some(13), "expected C to die from SIGPIPE");
    assert_eq!(r.status.signal(), Some(13));
}

// ---------------------------------------------------------------------------
// Determinism / repeatability
// ---------------------------------------------------------------------------

#[test]
fn repeated_runs_are_identical() {
    let first = run(rust_bin(), &[], b"", &[]);
    for _ in 0..25 {
        assert_same("repeat", &[], b"", &[]);
    }
    let last = run(rust_bin(), &[], b"", &[]);
    assert_eq!(first.stdout, last.stdout);
    assert_eq!(first.stderr, last.stderr);
    assert_eq!(ended(&first), ended(&last));
}

#[test]
fn concurrent_runs_are_identical() {
    let handles: Vec<_> = (0..8)
        .map(|i| {
            std::thread::spawn(move || {
                let stdin = format!("{i}\n");
                assert_same("concurrent", &[], stdin.as_bytes(), &[]);
            })
        })
        .collect();
    for h in handles {
        h.join().expect("thread panicked");
    }
}
