//! Phase C — error-path / boundary differential tests, one test per row of
//! `ERRORS.md`.
//!
//! The C library has no error channel at all (`void` return, both `printf` and
//! `puts` return values discarded, no `assert`, no range check). So these tests
//! cover (a) the failure conditions the code *can* actually be subjected to —
//! stdout write failures — asserting both implementations swallow them
//! identically, and (b) every boundary of the parameter domain, including bit
//! patterns that have no valid `int` reading pushed across the FFI boundary
//! through a deliberately mis-declared prototype.

mod common;

use common::*;
use std::ffi::{c_int, c_void};
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn clearerr(stream: *mut c_void);
    fn ferror(stream: *mut c_void) -> c_int;
    static stdout: *mut c_void;
}

/// Point fd 1 at `path` (opened with `opts`), run `f`, flush, restore.
/// Returns `(ferror(stdout) after the call, everything readable back from the
/// target if it is a regular file)`.
fn with_stdout_redirected_to<F: FnOnce()>(
    path: &str,
    write_mode: bool,
    f: F,
) -> (bool, Option<Vec<u8>>) {
    let file = if write_mode {
        OpenOptions::new().write(true).open(path)
    } else {
        OpenOptions::new().read(true).open(path)
    }
    .unwrap_or_else(|e| panic!("open {path}: {e}"));

    flush_all();
    unsafe { clearerr(stdout) };

    let saved = unsafe { dup(1) };
    assert!(saved >= 0);
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0);

    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    flush_all();
    let err = unsafe { ferror(stdout) } != 0;
    unsafe { clearerr(stdout) };
    assert!(unsafe { dup2(saved, 1) } >= 0);
    unsafe { close(saved) };
    // Reset the stream to a clean state for subsequent tests.
    flush_all();
    unsafe { clearerr(stdout) };

    if let Err(p) = res {
        std::panic::resume_unwind(p);
    }
    (err, None)
}

// ---------------------------------------------------------------------------
// Row 1 — there is no error channel; assert that mechanically.
// ---------------------------------------------------------------------------

#[test]
fn no_error_channel_exists_in_c_source() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/src/driver.c"
    ))
    .expect("read driver.c");
    // Strip the license comment block so we only inspect real code.
    let code: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    for forbidden in [
        "return", "assert", "NULL", "errno", "exit", "abort", "goto", "enum", "malloc", "sizeof",
    ] {
        assert!(
            !code.contains(forbidden),
            "ERRORS.md row 1 assumes the C source has no `{forbidden}`; it now does — \
             the error-surface table must be regenerated"
        );
    }
    // The signature really is `void driver(int, int)`.
    assert!(code.contains("void driver(int x, int y)"));

    // And the call itself must complete normally for both libraries (a `void`
    // function that returns is the only observable "success" signal there is).
    assert_same_each("errors-row1", &[(1, 2), (-3, 4)]);
}

// ---------------------------------------------------------------------------
// Rows 2-4 — stdout write failures / discarded stdout.
// ---------------------------------------------------------------------------

/// The inputs used by all failure-injection rows.
const FAIL_INPUTS: [(i32, i32); 6] = [
    (0, 0),
    (1, 1),
    (-1, -1),
    (i32::MIN, i32::MAX),
    (i32::MAX, i32::MIN),
    (12345, -6789),
];

/// `expect_err` states whether the injection must actually make the stream
/// fail. Asserting it keeps the row from silently becoming vacuous (e.g. if
/// `/dev/full` stopped returning ENOSPC).
fn failure_injection_row(row: &str, path: &str, write_mode: bool, expect_err: bool) {
    let c = c_lib();
    let r = rust_lib();

    // Unbuffered, so each printf/puts really performs the failing write(2)
    // *inside* the call rather than at the flush after it.
    set_stdout_buffering(IONBF, 0);

    let (c_err, _) = with_stdout_redirected_to(path, write_mode, || {
        for &(x, y) in &FAIL_INPUTS {
            unsafe { (c.driver)(x, y) };
        }
    });
    let (r_err, _) = with_stdout_redirected_to(path, write_mode, || {
        for &(x, y) in &FAIL_INPUTS {
            unsafe { (r.driver)(x, y) };
        }
    });

    set_stdout_buffering(IOFBF, 4096);

    // Same error indication on the stream, and — crucially — both survived:
    // reaching this line at all proves neither aborted or unwound across FFI.
    assert_eq!(
        c_err, r_err,
        "[{row}] ferror(stdout) after the call differs: C={c_err} Rust={r_err}"
    );
    assert_eq!(
        c_err, expect_err,
        "[{row}] the failure injection via {path} did not have the intended \
         effect (ferror={c_err}, expected {expect_err}) — the row would be vacuous"
    );

    // And both must still work afterwards, identically.
    assert_same_batch(&format!("{row}-recovery"), &FAIL_INPUTS);
}

#[test]
fn err_stdout_write_fails_enospc_dev_full() {
    // Row 2: writes to /dev/full always fail with ENOSPC.
    failure_injection_row("errors-row2", "/dev/full", true, true);
}

#[test]
fn err_stdout_write_fails_ebadf_readonly_fd() {
    // Row 3: fd 1 is a read-only descriptor -> write(2) fails with EBADF.
    failure_injection_row("errors-row3", "/dev/null", false, true);
}

#[test]
fn err_stdout_discarded_dev_null() {
    // Row 4: writes succeed but the bytes vanish.
    failure_injection_row("errors-row4", "/dev/null", true, false);
}

// ---------------------------------------------------------------------------
// Rows 5-10, 13-14 — parameter-domain boundaries, with randomised partners.
// ---------------------------------------------------------------------------

#[test]
fn err_boundary_y_zero_collapses_to_minus_one() {
    // Row 5: y = 0 => ~y = -1 => result = -1 for every x.
    let mut rng = Rng::new(SEED ^ 0xE5);
    let mut v: Vec<(i32, i32)> = (0..2000).map(|_| (rng.next_i32(), 0)).collect();
    v.extend_from_slice(&[(i32::MIN, 0), (i32::MAX, 0), (0, 0), (-1, 0), (1, 0)]);
    assert_same_batch("errors-row5", &v);

    let out = capture_stdout("c", || unsafe { (c_lib().driver)(0x5A5A_5A5A, 0) });
    assert_eq!(out, b"-1\n", "C collapses to -1 when y == 0");
}

#[test]
fn err_boundary_x_all_bits_set() {
    // Row 6: x = -1 => result = -1 for every y.
    let mut rng = Rng::new(SEED ^ 0xE6);
    let mut v: Vec<(i32, i32)> = (0..2000).map(|_| (-1, rng.next_i32())).collect();
    v.extend_from_slice(&[(-1, i32::MIN), (-1, i32::MAX), (-1, 0), (-1, -1)]);
    assert_same_batch("errors-row6", &v);

    let out = capture_stdout("c", || unsafe { (c_lib().driver)(-1, 0x1234_5678) });
    assert_eq!(out, b"-1\n", "C collapses to -1 when x == -1");
}

#[test]
fn err_boundary_int_min() {
    // Row 7: x = INT_MIN against random and extreme y.
    let mut rng = Rng::new(SEED ^ 0xE7);
    let mut v: Vec<(i32, i32)> = (0..2000).map(|_| (i32::MIN, rng.next_i32())).collect();
    v.extend_from_slice(&[
        (i32::MIN, i32::MIN),
        (i32::MIN, i32::MAX),
        (i32::MIN, -1),
        (i32::MIN, 0),
        (i32::MIN, 1),
    ]);
    assert_same_batch("errors-row7", &v);
    assert_same_each("errors-row7-each", &v[v.len() - 5..]);

    // Widest possible output: 11 characters.
    let out = capture_stdout("c", || unsafe { (c_lib().driver)(i32::MIN, -1) });
    assert_eq!(out, b"-2147483648\n");
}

#[test]
fn err_boundary_int_max() {
    // Row 8: x = INT_MAX against random and extreme y.
    let mut rng = Rng::new(SEED ^ 0xE8);
    let mut v: Vec<(i32, i32)> = (0..2000).map(|_| (i32::MAX, rng.next_i32())).collect();
    v.extend_from_slice(&[
        (i32::MAX, i32::MIN),
        (i32::MAX, i32::MAX),
        (i32::MAX, -1),
        (i32::MAX, 0),
        (i32::MAX, 1),
    ]);
    assert_same_batch("errors-row8", &v);
    assert_same_each("errors-row8-each", &v[v.len() - 5..]);

    let out = capture_stdout("c", || unsafe { (c_lib().driver)(i32::MAX, -1) });
    assert_eq!(out, b"2147483647\n");
}

#[test]
fn err_boundary_y_int_min() {
    // Row 9: y = INT_MIN => ~y = INT_MAX.
    let mut rng = Rng::new(SEED ^ 0xE9);
    let mut v: Vec<(i32, i32)> = (0..2000).map(|_| (rng.next_i32(), i32::MIN)).collect();
    v.extend_from_slice(&[(0, i32::MIN), (-1, i32::MIN), (i32::MIN, i32::MIN)]);
    assert_same_batch("errors-row9", &v);

    let out = capture_stdout("c", || unsafe { (c_lib().driver)(0, i32::MIN) });
    assert_eq!(out, b"2147483647\n", "0 | ~INT_MIN == INT_MAX");
}

#[test]
fn err_boundary_y_int_max() {
    // Row 10: y = INT_MAX => ~y = INT_MIN (result always negative).
    let mut rng = Rng::new(SEED ^ 0xEA);
    let mut v: Vec<(i32, i32)> = (0..2000).map(|_| (rng.next_i32(), i32::MAX)).collect();
    v.extend_from_slice(&[(0, i32::MAX), (-1, i32::MAX), (i32::MAX, i32::MAX)]);
    assert_same_batch("errors-row10", &v);

    let out = capture_stdout("c", || unsafe { (c_lib().driver)(0, i32::MAX) });
    assert_eq!(out, b"-2147483648\n", "0 | ~INT_MAX == INT_MIN");
}

#[test]
fn err_boundary_all_four_extreme_corners() {
    // Row 13.
    let v = [
        (i32::MIN, i32::MIN),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
        (i32::MAX, i32::MAX),
    ];
    assert_same_batch("errors-row13", &v);
    assert_same_each("errors-row13-each", &v);
}

#[test]
fn err_boundary_only_zero_result_input() {
    // Row 14: the single (x, y) whose result is 0.
    assert_same_each("errors-row14", &[(0, -1)]);
    let out = capture_stdout("c", || unsafe { (c_lib().driver)(0, -1) });
    assert_eq!(out, b"0\n");

    // Neighbours of that point must NOT be 0 — guards against an accidental
    // "always print 0" implementation passing this row.
    for &(x, y) in &[(1, -1), (-1, -1), (0, 0), (0, -2), (0, 1)] {
        let o = capture_stdout("c", || unsafe { (c_lib().driver)(x, y) });
        assert_ne!(o, b"0\n", "driver({x},{y}) must not print 0");
    }
    assert_same_each("errors-row14-neighbours", &[(1, -1), (-1, -1), (0, 0), (0, -2), (0, 1)]);
}

// ---------------------------------------------------------------------------
// Rows 11-12 — bit patterns with no valid `int` reading, pushed through a
// mis-declared 64-bit prototype (the FFI analogue of an out-of-range enum).
// ---------------------------------------------------------------------------

#[test]
fn err_out_of_int_range_args_truncate_via_i64_abi() {
    // Row 11: one step past each end of the int range, plus far-out values.
    let over: [i64; 10] = [
        i32::MAX as i64 + 1,
        i32::MIN as i64 - 1,
        i32::MAX as i64 + 2,
        i32::MIN as i64 - 2,
        u32::MAX as i64,
        u32::MAX as i64 + 1,
        i64::MAX,
        i64::MIN,
        0x1_0000_0000,
        0x7FFF_FFFF_FFFF_FFFF,
    ];
    let mut v: Vec<(i64, i64)> = Vec::new();
    for &a in &over {
        for &b in &over {
            v.push((a, b));
        }
    }
    assert_same_batch_i64("errors-row11", &v);

    // Both must additionally agree with the *truncated* int call, which is what
    // the C ABI mandates.
    let c = c_lib();
    let r = rust_lib();
    for &a in &over {
        for &b in &over {
            let wide_c = capture_stdout("c64", || unsafe { (c.driver_i64)(a, b) });
            let wide_r = capture_stdout("r64", || unsafe { (r.driver_i64)(a, b) });
            let narrow = capture_stdout("c32", || unsafe {
                (c.driver)(a as i32, b as i32)
            });
            assert_eq!(
                wide_c, narrow,
                "C ABI must truncate driver({a}, {b}) to ({}, {})",
                a as i32, b as i32
            );
            assert_eq!(
                wide_r, narrow,
                "Rust must truncate driver({a}, {b}) identically to C",
            );
        }
    }
}

#[test]
fn err_upper_argument_register_bits_ignored() {
    // Row 12: garbage in the upper half of each argument register.
    let mut rng = Rng::new(SEED ^ 0xEC);
    let los: [i32; 6] = [0, -1, 1, i32::MIN, i32::MAX, 0x5A5A_5A5A];
    let mut v: Vec<(i64, i64)> = Vec::new();
    for &lx in &los {
        for &ly in &los {
            let hx = rng.next_u64() << 32;
            let hy = rng.next_u64() << 32;
            v.push((
                (hx | (lx as u32 as u64)) as i64,
                (hy | (ly as u32 as u64)) as i64,
            ));
        }
    }
    assert_same_batch_i64("errors-row12", &v);

    // Equivalence with the plain 32-bit call.
    let c = c_lib();
    let r = rust_lib();
    for (i, &(a, b)) in v.iter().enumerate() {
        let lx = los[i / los.len()];
        let ly = los[i % los.len()];
        let expect = capture_stdout("c32", || unsafe { (c.driver)(lx, ly) });
        let got_c = capture_stdout("c64", || unsafe { (c.driver_i64)(a, b) });
        let got_r = capture_stdout("r64", || unsafe { (r.driver_i64)(a, b) });
        assert_eq!(got_c, expect, "C ignored/used upper bits unexpectedly");
        assert_eq!(got_r, expect, "Rust differs from C on upper argument bits");
    }
}

// ---------------------------------------------------------------------------
// Rows 15-16 — pathological stream buffering.
// ---------------------------------------------------------------------------

fn buffering_error_row(row: &str, mode: c_int, size: usize) {
    let mut rng = Rng::new(SEED ^ 0xF0 ^ size as u64);
    let mut v: Vec<(i32, i32)> = (0..500).map(|_| (rng.next_i32(), rng.next_i32())).collect();
    v.extend_from_slice(&[(0, -1), (i32::MIN, -1), (i32::MAX, -1), (-1, 0)]);

    let c = c_lib();
    let r = rust_lib();
    let c_out = capture_stdout("c", || {
        set_stdout_buffering(mode, size);
        for &(x, y) in &v {
            unsafe { (c.driver)(x, y) };
        }
    });
    let r_out = capture_stdout("rust", || {
        set_stdout_buffering(mode, size);
        for &(x, y) in &v {
            unsafe { (r.driver)(x, y) };
        }
    });
    set_stdout_buffering(IOFBF, 4096);

    assert_eq!(c_out, r_out, "[{row}] byte streams differ under this buffering");
    assert_eq!(
        c_out.iter().filter(|&&b| b == b'\n').count(),
        v.len(),
        "[{row}] one newline per call"
    );
}

#[test]
fn err_stdout_unbuffered_setvbuf() {
    // Row 15: printf and puts each reach write(2) separately.
    buffering_error_row("errors-row15", IONBF, 0);
}

#[test]
fn err_stdout_one_byte_buffer() {
    // Row 16: a 1-byte fully-buffered stream forces a flush per character.
    buffering_error_row("errors-row16", IOFBF, 1);
}
