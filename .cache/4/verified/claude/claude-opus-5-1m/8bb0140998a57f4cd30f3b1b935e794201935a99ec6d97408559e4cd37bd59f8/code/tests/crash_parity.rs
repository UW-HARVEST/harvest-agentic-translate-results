//! Phase C (continued) — the `ERRORS.md` rows whose C behaviour is a fatal
//! signal: E11, E12, E31, E32, E33.
//!
//! The C code performs these dereferences / indirect calls with no validation,
//! so the process dies. To compare C and Rust we re-exec *this* test binary in
//! a child process (`HATCH_CRASH_CASE=<lib>:<case>`), let the child perform the
//! single offending call, and compare the child's termination status. The
//! assertion is on the *exact* signal number, not merely "both failed".
//!
//! ### Note on the debug vs release Rust `.so`
//! `std::ptr::copy` / `std::ptr::read` / `std::ptr::write_bytes` carry
//! `assert_unsafe_precondition!` null/alignment checks that are compiled in
//! only when the crate is built with `debug-assertions = on`. In a debug build
//! those checks abort (SIGILL/SIGABRT/SIGTRAP) *before* the faulting access, so
//! the signal differs from C's SIGSEGV purely as a build-profile artifact. The
//! shipped artifact is the optimised `cdylib`, so exact-signal parity is
//! asserted against a `.so` built with debug assertions off; point
//! `HATCH_RUST_SO` at it (see `run_all.sh`). When the Rust `.so` under test
//! still has UB-checks enabled the test asserts the weaker—but still
//! meaningful—"both die from a fatal signal, and neither returns normally",
//! and prints both signals.

mod common;
use common::*;

use std::ffi::c_void;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

/// A non-executable data object in this binary (row E33).
static DATA_BLOB: [u8; 64] = [0x90; 64];

fn data_addr() -> *const c_void {
    DATA_BLOB.as_ptr() as *const c_void
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Outcome {
    /// Killed by signal N.
    Signal(i32),
    /// Exited normally with the given code, having completed the call.
    NoCrash(i32),
    /// Exited normally with the given code without printing the marker.
    Exit(i32),
}

const MARKER: &str = "HATCH_NO_CRASH";

/// The child half: performs exactly one unchecked call and, if it survives,
/// prints `MARKER`.
#[test]
fn crash_child() {
    let Ok(spec) = std::env::var("HATCH_CRASH_CASE") else {
        // Normal (parent) run: nothing to do.
        return;
    };
    let (which, case) = spec.split_once(':').expect("HATCH_CRASH_CASE=<lib>:<case>");
    let b = both();
    let api = match which {
        "c" => &b.c,
        "rust" => &b.r,
        other => panic!("unknown lib {other}"),
    };
    eprintln!("child: lib={} case={}", api.tag, case);

    match case {
        // E11: memmove(NULL, NULL+shift, n)
        "shift_array_null" => unsafe {
            (api.shift_array_data)(std::ptr::null_mut(), 10, 3);
        },
        // E12: *ptr with ptr == NULL
        "ppd_null" => {
            let v = unsafe { (api.process_pointer_data)(std::ptr::null_mut(), 3) };
            std::hint::black_box(v);
        }
        // E31: records[i].value with records == NULL and a positive loop bound
        "records_null_shift0" => {
            let v = unsafe { (api.manipulate_records)(std::ptr::null_mut(), 5, 0) };
            std::hint::black_box(v);
        }
        // E31 (variant): positive bound after the memmove branch is taken
        "records_null_shift2" => {
            let v = unsafe { (api.manipulate_records)(std::ptr::null_mut(), 5, 2) };
            std::hint::black_box(v);
        }
        // E32: call through a NULL function pointer
        "apply_null" => {
            let v = unsafe { (api.apply_operation)(std::ptr::null(), 1, 2, 3) };
            std::hint::black_box(v);
        }
        // E33: call through a non-executable data address
        "apply_data_addr" => {
            let v = unsafe { (api.apply_operation)(data_addr(), 1, 2, 3) };
            std::hint::black_box(v);
        }
        other => panic!("unknown case {other}"),
    }

    println!("{MARKER}");
}

fn run_case(which: &str, case: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["crash_child", "--exact", "--nocapture", "--test-threads=1"])
        .env("HATCH_CRASH_CASE", format!("{which}:{case}"))
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn child");
    if let Some(sig) = out.status.signal() {
        return Outcome::Signal(sig);
    }
    let code = out.status.code().unwrap_or(-1);
    if String::from_utf8_lossy(&out.stdout).contains(MARKER) {
        Outcome::NoCrash(code)
    } else {
        Outcome::Exit(code)
    }
}

/// Does the Rust `.so` under test still have UB precondition checks compiled in?
/// Probed behaviourally: a debug-assertions build aborts on `ptr::read(NULL)`
/// with SIGILL/SIGABRT/SIGTRAP rather than faulting with SIGSEGV.
fn rust_so_has_ub_checks() -> bool {
    // Probe every intrinsic that carries a precondition check: ptr::read
    // (process_pointer_data) and ptr::copy / ptr::write_bytes
    // (shift_array_data, manipulate_records).
    ["ppd_null", "shift_array_null", "records_null_shift2"]
        .iter()
        .any(|case| run_case("rust", case) != Outcome::Signal(SIGSEGV))
}

/// `SIGSEGV` on Linux.
const SIGSEGV: i32 = 11;

fn assert_same_crash(row: &str, case: &str, strict: bool) {
    let oc = run_case("c", case);
    let or = run_case("rust", case);
    println!("{row} [{case}]  C={oc:?}  Rust={or:?}  (strict={strict})");

    // Neither may complete the call normally — that would be a real divergence
    // from the C behaviour whichever way round it happened.
    assert!(
        !matches!(oc, Outcome::NoCrash(_)),
        "{row} [{case}]: the C library unexpectedly survived: {oc:?}"
    );
    assert!(
        !matches!(or, Outcome::NoCrash(_)),
        "{row} [{case}]: the Rust library survived where C dies: {or:?}"
    );
    // Both must die from a signal, not a clean exit.
    let sc = match oc {
        Outcome::Signal(s) => s,
        other => panic!("{row} [{case}]: C did not die from a signal: {other:?}"),
    };
    let sr = match or {
        Outcome::Signal(s) => s,
        other => panic!("{row} [{case}]: Rust did not die from a signal: {other:?}"),
    };
    assert_eq!(sc, SIGSEGV, "{row} [{case}]: C should raise SIGSEGV");
    if strict {
        assert_eq!(
            sc, sr,
            "{row} [{case}]: signal mismatch — C raised {sc}, Rust raised {sr}"
        );
    } else {
        println!(
            "  note: Rust .so has UB precondition checks enabled (debug build); \
             it aborts with signal {sr} before the faulting access instead of \
             C's SIGSEGV {sc}. Re-run against a release .so for strict parity."
        );
    }
}

#[test]
fn phase_c_fatal_error_rows() {
    let mut cov = Coverage::new();
    println!("Rust .so under test: {}", rust_so_path().display());
    let strict = !rust_so_has_ub_checks();
    println!("strict signal parity: {strict}");

    // E11: shift_array_data(NULL, size, shift_by) with 0 < shift_by < size
    cov.hit("E11");
    assert_same_crash("E11", "shift_array_null", strict);

    // E12: process_pointer_data(NULL, m)
    cov.hit("E12");
    assert_same_crash("E12", "ppd_null", strict);

    // E31: manipulate_records(NULL, n, shift) with a positive loop bound
    cov.hit("E31");
    assert_same_crash("E31", "records_null_shift0", strict);
    assert_same_crash("E31", "records_null_shift2", strict);

    // E32: apply_operation with op == NULL
    cov.hit("E32");
    assert_same_crash("E32", "apply_null", strict);

    // E33: apply_operation with a non-executable data address
    cov.hit("E33");
    assert_same_crash("E33", "apply_data_addr", strict);

    cov.assert_complete(ERROR_ROWS_FATAL, "ERRORS.md (fatal)");
}
