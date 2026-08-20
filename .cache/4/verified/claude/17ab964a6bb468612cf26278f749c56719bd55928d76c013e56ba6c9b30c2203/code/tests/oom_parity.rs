//! Out-of-memory and crash parity tests.
//!
//! Some C paths can only be reached when `malloc()` actually fails, and some C
//! paths dereference a NULL pointer (undefined behaviour that the translation
//! must nevertheless reproduce, because the C is the ground truth). Neither can
//! be observed in-process, so every case here is executed in a **child
//! process** that
//!
//! 1. lowers `RLIMIT_AS` so that large allocations fail deterministically,
//! 2. calls one library through its exported symbols,
//! 3. `_exit()`s (or dies from a signal).
//!
//! The parent then compares exit code / terminating signal / `stderr` bytes of
//! the C child against the Rust child.
//!
//! This is what covers `ERRORS.md` rows E1 (struct `malloc` fails), E6 (`strdup`
//! fails), E9b/E13b (the *unchecked* NULL that C dereferences when a large
//! positive dimension makes `allocate_matrix` fail) and the positive-size
//! `malloc` failure of `matrix_to_string` (E17b).

mod common;

use common::*;
use std::ffi::{c_int, c_void};
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const PAYLOAD_ENV: &str = "DRIVER_PAYLOAD_CASE";
const LIB_ENV: &str = "DRIVER_PAYLOAD_LIB";

#[repr(C)]
struct RLimit {
    rlim_cur: u64,
    rlim_max: u64,
}

/// `RLIMIT_AS` on Linux.
const RLIMIT_AS: c_int = 9;

unsafe extern "C" {
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
    fn getrlimit(resource: c_int, rlim: *mut RLimit) -> c_int;
    fn _exit(status: c_int) -> !;
    fn malloc(size: usize) -> *mut c_void;
}

fn limit_address_space(bytes: u64) {
    let mut cur = RLimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    assert_eq!(unsafe { getrlimit(RLIMIT_AS, &mut cur) }, 0, "getrlimit");
    let want = RLimit {
        rlim_cur: bytes,
        rlim_max: cur.rlim_max,
    };
    assert_eq!(unsafe { setrlimit(RLIMIT_AS, &want) }, 0, "setrlimit");
}

// ---------------------------------------------------------------------------
// The child payload (ignored by default; the parent runs it explicitly).
// ---------------------------------------------------------------------------

#[test]
#[ignore = "executed in a child process by the parity tests"]
fn payload() {
    let case = match std::env::var(PAYLOAD_ENV) {
        Ok(c) => c,
        Err(_) => return,
    };
    let which = std::env::var(LIB_ENV).unwrap_or_default();
    // Resolve the symbols BEFORE the address space is capped.
    let api = match which.as_str() {
        "c" => c_api(),
        "rust" => rust_api(),
        other => panic!("bad {LIB_ENV}: {other:?}"),
    };

    match case.as_str() {
        // E1 — malloc(sizeof(matrix_t)) itself fails.
        "alloc_struct_oom" => {
            limit_address_space(512 * 1024 * 1024);
            // Drain the heap: after this, even a 16-byte malloc fails.
            for chunk in [1024 * 1024usize, 64 * 1024, 4096, 64, 16] {
                while !unsafe { malloc(chunk) }.is_null() {}
            }
            // No Rust allocation is allowed from here on.
            let p = unsafe { (api.allocate_matrix)(2, 2) };
            unsafe { _exit(if p.is_null() { 0 } else { 21 }) };
        }
        // E6 — strdup(input) fails.
        "strdup_oom" => {
            // Build the input FIRST, then leave only a small address-space
            // slack: the library's tiny `allocate_matrix` allocations still
            // succeed, but `strdup()` of the 4 MiB input cannot.
            const S: usize = 64 * 1024 * 1024;
            let mut bytes: Vec<u8> = Vec::with_capacity(S + 1);
            let pattern = b"1 2\n3 4\n";
            while bytes.len() < S {
                bytes.push(pattern[bytes.len() % pattern.len()]);
            }
            let text = CBuf::new(bytes);
            let statm = std::fs::read_to_string("/proc/self/statm").expect("statm");
            let pages: u64 = statm
                .split_whitespace()
                .next()
                .and_then(|f| f.parse().ok())
                .expect("statm size");
            limit_address_space(pages * 4096 + 1024 * 1024);
            // Make sure no S-sized block can be served any more (the probes are
            // deliberately never freed); small allocations still succeed from the
            // arena's top chunk, so `allocate_matrix` keeps working and `strdup`
            // is the call that fails — exactly the E6 path.
            for _ in 0..4 {
                if unsafe { malloc(S + 1) }.is_null() {
                    break;
                }
            }
            let p = unsafe { (api.initialize_matrix_from_string)(text.as_ptr(), 2, 2) };
            unsafe { _exit(if p.is_null() { 0 } else { 22 }) };
        }
        // E9b — allocate_matrix fails for a large POSITIVE width, and the
        // unchecked `mat` is then dereferenced by the `j < width` loop.
        "init_null_deref" => {
            limit_address_space(512 * 1024 * 1024);
            let text = CBuf::new("7 8\n");
            let p = unsafe { (api.initialize_matrix_from_string)(text.as_ptr(), 500_000_000, 1) };
            unsafe { _exit(if p.is_null() { 23 } else { 24 }) };
        }
        // E13b — multiply_matrices dereferences the NULL result of a failed
        // allocate_matrix (large POSITIVE result width).
        "mul_null_deref" => {
            limit_address_space(512 * 1024 * 1024);
            let text = CBuf::new("7\n");
            let a = unsafe { (api.initialize_matrix_from_string)(text.as_ptr(), 1, 1) };
            let mut b = MatrixT {
                matrix: std::ptr::null_mut(),
                width: 500_000_000,
                height: 1,
            };
            let p = unsafe { (api.multiply_matrices)(a, &mut b) };
            unsafe { _exit(if p.is_null() { 25 } else { 26 }) };
        }
        // E17b — matrix_to_string's malloc fails for a POSITIVE buffer_size.
        "to_string_malloc_fail" => {
            limit_address_space(512 * 1024 * 1024);
            let mut m = MatrixT {
                matrix: std::ptr::null_mut(),
                width: 100_000_000, // buffer_size = 1_100_000_002 > limit
                height: 1,
            };
            let p = unsafe { (api.matrix_to_string)(&mut m) };
            unsafe { _exit(if p.is_null() { 0 } else { 27 }) };
        }
        // driver() reaching the same unchecked NULL dereference.
        "driver_null_deref" => {
            limit_address_space(512 * 1024 * 1024);
            let a = CBuf::new("7 8\n");
            let b = CBuf::new("3\n");
            let rc = unsafe { (api.driver)(500_000_000, 1, a.as_ptr(), 1, 1, b.as_ptr()) };
            unsafe { _exit(28 + rc) };
        }
        other => panic!("unknown payload case {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Parent side
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    stderr: Vec<u8>,
}

fn run_payload(case: &str, which: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args([
            "--exact",
            "payload",
            "--include-ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(PAYLOAD_ENV, case)
        .env(LIB_ENV, which)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn payload child");
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stderr: out.stderr,
    }
}

/// Runs one payload case on both libraries and asserts full parity.
fn assert_payload_parity(case: &str) -> Outcome {
    let c = run_payload(case, "c");
    let r = run_payload(case, "rust");
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{case}] exit status differs\n  C   : {:?}\n  Rust: {:?}\n  C stderr: {}\n  Rust stderr: {}",
        (c.code, c.signal),
        (r.code, r.signal),
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_bytes_eq(&c.stderr, &r.stderr, &format!("[{case}] stderr differs"));
    c
}

#[test]
fn oom_e1_allocate_struct_malloc_fails() {
    let out = assert_payload_parity("alloc_struct_oom");
    assert_eq!(out.code, Some(0), "allocate_matrix must return NULL: {out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("Failed to allocate memory for matrix struct"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn oom_e6_strdup_fails() {
    let out = assert_payload_parity("strdup_oom");
    assert_eq!(out.code, Some(0), "init must return NULL: {out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("Failed to duplicate input string"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn oom_e9b_init_dereferences_unchecked_null() {
    // C does NOT check allocate_matrix()'s result; with a large positive width
    // the `j < width` loop body runs and dereferences NULL ⇒ the process dies.
    // The Rust port must die in exactly the same way.
    let out = assert_payload_parity("init_null_deref");
    assert!(
        out.signal.is_some(),
        "expected a fatal signal from the unchecked NULL dereference: {out:?}"
    );
}

#[test]
fn oom_e13b_multiply_dereferences_unchecked_null() {
    let out = assert_payload_parity("mul_null_deref");
    assert!(out.signal.is_some(), "expected a fatal signal: {out:?}");
}

#[test]
fn oom_e17b_to_string_positive_size_malloc_fails() {
    let out = assert_payload_parity("to_string_malloc_fail");
    assert_eq!(out.code, Some(0), "matrix_to_string must return NULL: {out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("Failed to allocate memory for matrix string"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn oom_driver_null_deref() {
    let out = assert_payload_parity("driver_null_deref");
    assert!(out.signal.is_some(), "expected a fatal signal: {out:?}");
}
