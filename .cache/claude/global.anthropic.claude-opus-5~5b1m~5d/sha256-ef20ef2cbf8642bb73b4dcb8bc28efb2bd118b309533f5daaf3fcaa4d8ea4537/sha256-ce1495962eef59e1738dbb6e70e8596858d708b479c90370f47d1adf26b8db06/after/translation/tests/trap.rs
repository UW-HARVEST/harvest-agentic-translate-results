//! Phase C — error-path differential tests, one per `ERRORS.md` row.
//!
//! `driver` has no error return: its only rejection mechanism is the `SIGFPE`
//! that libc's `div(3)` raises through `idivl`. A fatal signal cannot be
//! observed in-process, so each row runs in a **fresh child process** which
//! `dlopen`s one of the two libraries, redirects `stdout` to a file, and calls
//! `driver`. The parent then compares the child's terminating signal, exit code,
//! and captured bytes between the C and the Rust build.

mod common;

use std::ffi::{c_int, c_void};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use common::{c_so_path, rust_so_path, Rng, SEED};
use libloading::{Library, Symbol};

extern "C" {
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const SIGFPE: i32 = 8;

/// Which build the child should exercise.
const ENV_LIB: &str = "DRIVER_CHILD_LIB";
const ENV_X: &str = "DRIVER_CHILD_X";
const ENV_Y: &str = "DRIVER_CHILD_Y";
const ENV_OUT: &str = "DRIVER_CHILD_OUT";

/// The child-process body. A no-op during a normal test run.
///
/// When the parent re-executes this test binary with `DRIVER_CHILD_LIB` set, this
/// becomes the whole program: load that one `.so`, call `driver`, exit 0. If the
/// call traps, the process dies here and never reaches the `exit`.
#[test]
fn zzz_child_worker() {
    let which = match std::env::var(ENV_LIB) {
        Ok(v) => v,
        Err(_) => return, // normal run: nothing to do
    };
    let x: i32 = std::env::var(ENV_X).unwrap().parse().unwrap();
    let y: i32 = std::env::var(ENV_Y).unwrap().parse().unwrap();
    let out = std::env::var(ENV_OUT).unwrap();

    let so: PathBuf = match which.as_str() {
        "c" => c_so_path(),
        "rust" => rust_so_path(),
        other => panic!("unknown {ENV_LIB}={other}"),
    };

    let lib = unsafe { Library::new(&so) }.expect("dlopen in child");
    let driver: Symbol<unsafe extern "C" fn(c_int, c_int)> =
        unsafe { lib.get(b"driver\0") }.expect("child: `driver` must be exported");

    // Point fd 1 at the capture file so only driver's own bytes land there.
    let f = File::create(&out).expect("child: create out file");
    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(f.as_raw_fd(), 1) >= 0, "child: dup2 failed");
    }

    unsafe { driver(x, y) };

    unsafe { fflush(std::ptr::null_mut()) };
    std::process::exit(0);
}

/// What a child run produced.
#[derive(Debug, PartialEq, Eq)]
struct ChildRun {
    /// Terminating signal, if the child was killed.
    signal: Option<i32>,
    /// Exit code, if the child exited normally.
    code: Option<i32>,
    /// Bytes `driver` wrote to stdout.
    out: Vec<u8>,
}

fn tmp_dir() -> PathBuf {
    PathBuf::from(std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string()))
}

fn run_child(which: &str, x: i32, y: i32) -> ChildRun {
    let exe = std::env::current_exe().expect("current_exe");
    let out_path = tmp_dir().join(format!(
        "driver-trap-{which}-{}-{}-{}.txt",
        x as u32,
        y as u32,
        std::process::id()
    ));
    let _ = std::fs::remove_file(&out_path);

    let status = Command::new(exe)
        .args(["--exact", "zzz_child_worker", "--nocapture", "--test-threads=1"])
        .env(ENV_LIB, which)
        .env(ENV_X, x.to_string())
        .env(ENV_Y, y.to_string())
        .env(ENV_OUT, &out_path)
        // Keep the harness's own chatter out of our capture file and our logs.
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn child");

    let out = std::fs::read(&out_path).unwrap_or_default();
    let _ = std::fs::remove_file(&out_path);

    ChildRun {
        signal: status.signal(),
        code: status.code(),
        out,
    }
}

/// Runs `(x, y)` under both libraries and asserts identical process outcomes.
fn assert_same_outcome(row: &str, x: i32, y: i32) -> ChildRun {
    let c = run_child("c", x, y);
    let r = run_child("rust", x, y);
    assert_eq!(
        c.signal, r.signal,
        "[{row}] driver({x}, {y}): terminating signal differs — C {:?} vs Rust {:?}",
        c.signal, r.signal
    );
    assert_eq!(
        c.code, r.code,
        "[{row}] driver({x}, {y}): exit code differs — C {:?} vs Rust {:?}",
        c.code, r.code
    );
    assert_eq!(
        c.out,
        r.out,
        "[{row}] driver({x}, {y}): stdout differs — C {:?} vs Rust {:?}",
        String::from_utf8_lossy(&c.out),
        String::from_utf8_lossy(&r.out)
    );
    c
}

/// Asserts the outcome is specifically "killed by SIGFPE, having printed nothing".
fn assert_sigfpe(row: &str, x: i32, y: i32) {
    let outcome = assert_same_outcome(row, x, y);
    assert_eq!(
        outcome.signal,
        Some(SIGFPE),
        "[{row}] driver({x}, {y}) should be killed by SIGFPE ({SIGFPE}), got {outcome:?}"
    );
    assert!(
        outcome.out.is_empty(),
        "[{row}] driver({x}, {y}) trapped but still produced output: {:?}",
        String::from_utf8_lossy(&outcome.out)
    );
}

// ---------------------------------------------------------------------------
// Control: prove the child mechanism reports a *successful* run correctly,
// so a SIGFPE assertion below cannot be a false positive from a broken harness.
// ---------------------------------------------------------------------------

#[test]
fn trap_harness_control_valid_input() {
    let outcome = assert_same_outcome("control", 7, 3);
    assert_eq!(outcome.signal, None, "valid input must not raise a signal");
    assert_eq!(outcome.code, Some(0), "valid input must exit 0");
    assert_eq!(
        outcome.out, b"quotient: 2, remainder: 1\n",
        "control child produced unexpected bytes"
    );

    // A second, negative-operand control to be sure output really flows through.
    let neg = assert_same_outcome("control", -7, 2);
    assert_eq!(neg.signal, None);
    assert_eq!(neg.out, b"quotient: -3, remainder: -1\n");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 1 — y == 0 with x == 0
// ---------------------------------------------------------------------------

#[test]
fn trap_row1_zero_over_zero() {
    assert_sigfpe("ERRORS row 1", 0, 0);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 — y == 0 with x != 0
// ---------------------------------------------------------------------------

#[test]
fn trap_row2_nonzero_over_zero() {
    let mut rng = Rng::new(SEED ^ 0xF0);
    let mut xs = vec![1, -1, 2, -2, 42, -42, i32::MAX, i32::MIN, i32::MIN + 1, i32::MAX - 1];
    for _ in 0..6 {
        xs.push(rng.next_i32_nonzero());
    }
    for x in xs {
        assert_sigfpe("ERRORS row 2", x, 0);
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 3 — INT_MIN / -1 signed overflow
// ---------------------------------------------------------------------------

#[test]
fn trap_row3_int_min_over_minus_one() {
    assert_sigfpe("ERRORS row 3", i32::MIN, -1);
}

// ---------------------------------------------------------------------------
// Neighbourhood of the trapping inputs: one step away must NOT trap, and must
// agree. Guards against a Rust translation that over- or under-rejects.
// ---------------------------------------------------------------------------

#[test]
fn trap_neighbourhood_must_not_trap() {
    let near: &[(i32, i32)] = &[
        (0, 1),
        (0, -1),
        (1, 1),
        (-1, -1),
        (i32::MIN, 1),
        (i32::MIN, -2),
        (i32::MIN, 2),
        (i32::MIN + 1, -1),
        (i32::MAX, -1),
        (i32::MAX, 1),
    ];
    for &(x, y) in near {
        let outcome = assert_same_outcome("near-trap", x, y);
        assert_eq!(
            outcome.signal, None,
            "driver({x}, {y}) must not trap, got {outcome:?}"
        );
        assert_eq!(outcome.code, Some(0), "driver({x}, {y}) must exit 0");
        assert_eq!(
            outcome.out,
            common::expected_line(x, y).into_bytes(),
            "driver({x}, {y}) output mismatch vs reference model"
        );
    }
}

// ---------------------------------------------------------------------------
// Symbol-level guard: the Rust `.so` must not have quietly renamed the export.
// ---------------------------------------------------------------------------

#[test]
fn both_libraries_expose_the_same_symbol_name() {
    for p in [c_so_path(), rust_so_path()] {
        assert!(p.exists(), "missing shared object {}", p.display());
        let lib = unsafe { Library::new(&p) }.expect("dlopen");
        let sym: Result<Symbol<unsafe extern "C" fn(c_int, c_int)>, _> =
            unsafe { lib.get(b"driver\0") };
        assert!(
            sym.is_ok(),
            "{} does not export `driver` under that exact name",
            p.display()
        );
        // And nothing named after a Rust mangling scheme should be needed.
        let mangled: Result<Symbol<unsafe extern "C" fn(c_int, c_int)>, _> =
            unsafe { lib.get(b"_ZN6driver6driver17h0000000000000000E\0") };
        assert!(
            mangled.is_err(),
            "{} unexpectedly exports a mangled `driver`",
            Path::new(&p).display()
        );
    }
}
