//! Phase B row 24 — whole-program differential test.
//!
//! The `.so`-level tests drive `main` through the FFI boundary; this one runs the
//! two *programs* (the CMake-built C `driver` and the Rust `driver`) as real
//! subprocesses and compares stdout, stderr, the exit code **and** the
//! terminating signal. That is the only way to observe the crash-on-missing
//! argument behaviour end to end, including the exit status the shell sees.

mod common;

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{pretty, Rng};

const SEED: u64 = 0x5EED_0024;

fn c_exe() -> PathBuf {
    // Prefer the executable produced by the project's own CMake build.
    let cmake = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("driver");
    if cmake.exists() {
        return cmake;
    }
    // Fall back to the copy build.rs compiles, so the test never silently skips.
    PathBuf::from(env!("C_EXE_PATH"))
}

fn rust_exe() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("target/<profile>");
    let candidate = dir.join("driver");
    assert!(
        candidate.exists(),
        "the Rust driver binary is missing at {}",
        candidate.display()
    );
    candidate
}

#[derive(Debug, PartialEq, Eq)]
struct Run {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    code: Option<i32>,
    signal: Option<i32>,
}

fn run(exe: &Path, args: &[&[u8]]) -> Run {
    let out = Command::new(exe)
        .args(args.iter().map(|a| OsStr::from_bytes(a)))
        .output()
        .unwrap_or_else(|e| panic!("failed to run {}: {e}", exe.display()));

    Run {
        stdout: out.stdout,
        stderr: out.stderr,
        code: out.status.code(),
        signal: out.status.signal(),
    }
}

fn assert_same(c: &Path, r: &Path, args: &[&[u8]]) {
    let c_run = run(c, args);
    let r_run = run(r, args);
    assert_eq!(
        c_run,
        r_run,
        "program behaviour mismatch for args {:?}\n  C   : {:?}\n  Rust: {:?}",
        pretty(args),
        c_run,
        r_run
    );
}

#[test]
fn row24_end_to_end_program_equivalence() {
    let c = c_exe();
    let r = rust_exe();
    let mut rng = Rng::new(SEED);

    // Argument-count shapes, including the two crashing ones.
    assert_same(&c, &r, &[]);
    assert_same(&c, &r, &[b"5"]);
    assert_same(&c, &r, &[b"5", b"6"]);
    assert_same(&c, &r, &[b"5", b"6", b"7"]);
    assert_same(&c, &r, &[b"5", b"6", b"7", b"8", b"9"]);

    // Value classes.
    for pair in [
        (&b"2147483647"[..], &b"1"[..]),
        (b"-2147483648", b"-1"),
        (b"0", b"0"),
        (b"  \t\n12", b"-0034"),
        (b"abc", b"def"),
        (b"", b""),
        (b"99999999999999999999", b"0"),
        (b"-99999999999999999999", b"0"),
        (b"9223372036854775807", b"1"),
        (b"-9223372036854775808", b"0"),
        (b"0x10", b"010"),
        (b"+7", b"+8"),
        (b"\x80\xff", b"\xfe"),
    ] {
        assert_same(&c, &r, &[pair.0, pair.1]);
    }

    // Randomised corpus.
    for _ in 0..200 {
        let a = format!("{}", rng.next_i32()).into_bytes();
        let b = format!("{}", rng.next_i32()).into_bytes();
        assert_same(&c, &r, &[&a, &b]);
    }

    // Randomised junk (no NUL, which a process argument can never contain).
    for _ in 0..200 {
        let mk = |rng: &mut Rng| -> Vec<u8> {
            let n = rng.below(12);
            (0..n).map(|_| (rng.below(255) + 1) as u8).collect()
        };
        let a = mk(&mut rng);
        let b = mk(&mut rng);
        assert_same(&c, &r, &[&a, &b]);
    }
}

/// Row 25 — signal disposition.
///
/// A C program inherits the default `SIGPIPE` disposition, so writing to a pipe
/// with no reader kills it with signal 13. Rust's runtime sets `SIGPIPE` to
/// `SIG_IGN` before `main`, which would instead make the write fail silently and
/// the process exit 0 — an observable divergence. `src/main.rs` restores
/// `SIG_DFL`, and this test proves it.
///
/// The read end is closed *before* the child is spawned, so the very first write
/// is guaranteed to find a reader-less pipe; there is no race.
#[test]
fn row25_sigpipe_disposition_matches() {
    use std::os::fd::{FromRawFd, OwnedFd};
    use std::process::Stdio;

    fn run_with_readerless_stdout(exe: &Path) -> (Option<i32>, Option<i32>) {
        unsafe {
            let mut fds = [0i32; 2];
            assert_eq!(libc::pipe(fds.as_mut_ptr()), 0, "pipe() failed");
            let read_end = fds[0];
            let write_end = fds[1];

            // No reader will ever exist for this pipe.
            libc::close(read_end);

            let stdout = Stdio::from(OwnedFd::from_raw_fd(write_end));
            let status = Command::new(exe)
                .args([OsStr::from_bytes(b"1"), OsStr::from_bytes(b"2")])
                .stdout(stdout)
                .stderr(Stdio::null())
                .status()
                .unwrap_or_else(|e| panic!("failed to run {}: {e}", exe.display()));

            (status.code(), status.signal())
        }
    }

    let c = run_with_readerless_stdout(&c_exe());
    let r = run_with_readerless_stdout(&rust_exe());

    assert_eq!(
        c, r,
        "SIGPIPE behaviour differs: C=(code {:?}, signal {:?}), Rust=(code {:?}, signal {:?})",
        c.0, c.1, r.0, r.1
    );
    assert_eq!(
        c.1,
        Some(libc::SIGPIPE),
        "the C reference is expected to die from SIGPIPE"
    );
}

/// The exit status of the successful path must be 0 for both, because the C
/// `main` falls off the end of its body (C99: an implicit `return 0`).
#[test]
fn row24_exit_status_zero_on_success() {
    let c = c_exe();
    let r = rust_exe();
    let c_run = run(&c, &[b"1", b"2"]);
    let r_run = run(&r, &[b"1", b"2"]);
    assert_eq!(c_run.code, Some(0));
    assert_eq!(r_run.code, Some(0));
    assert_eq!(c_run.stdout, b"3\n");
    assert_eq!(r_run.stdout, b"3\n");
}
