//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`, plus the generic FFI-boundary cases.
//!
//! `driver` has no software error path (see `ERRORS.md`): its only rejection
//! behaviour is the `SIGFPE` raised by the `idiv` inside glibc's `div(3)`.
//! Observing that requires a separate process per call, so these tests re-exec
//! the test binary as a worker and compare the exact termination signal — not
//! merely "both failed somehow" — together with the bytes written before the
//! trap.

mod common;

use common::{
    assert_same, assert_same_all, assert_same_trap, op, op_raw, ops_for, run_ops, run_ops_with,
    traps, Rng, Side, BOUNDARIES, SIGFPE,
};
use std::ffi::c_int;

/// Child-side entry point for every batch this file runs.
/// Inert during a normal test run. See `common::worker_body`.
#[test]
fn difftest_worker() {
    common::worker_body();
}

// ---------------------------------------------------------------------------
// ERRORS.md row 1 — y == 0 (any x) -> SIGFPE
// ---------------------------------------------------------------------------

#[test]
fn test_err_row1_divide_by_zero_sigfpe() {
    // Every interesting dividend against the zero divisor, including the
    // extremes and zero itself.
    let mut xs: Vec<c_int> = vec![
        0,
        1,
        -1,
        2,
        -2,
        7,
        -7,
        12345,
        -12345,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
    ];
    let mut rng = Rng::new(0xE001);
    for _ in 0..5 {
        xs.push(rng.next_i32());
    }

    for x in xs {
        assert_same_trap(x, 0);
    }
}

/// Output produced *before* the trapping call must also match: a batch that
/// prints one good line and then divides by zero has to die at exactly the same
/// point on both sides.
#[test]
fn test_err_row1_output_before_trap_matches() {
    let prefix = [(5i32, 2i32), (-9, 4), (i32::MIN, 3)];

    let mut c_ops: Vec<_> = ops_for(Side::C, &prefix);
    c_ops.push(op(Side::C, 123, 0));
    let mut r_ops: Vec<_> = ops_for(Side::Rust, &prefix);
    r_ops.push(op(Side::Rust, 123, 0));

    let c = run_ops_with(&c_ops, true);
    let r = run_ops_with(&r_ops, true);

    assert_eq!(c.signal, Some(SIGFPE), "C should trap on the 4th call");
    assert_eq!(r.signal, Some(SIGFPE), "Rust should trap on the 4th call");
    assert_eq!(c.stdout, r.stdout, "pre-trap output differs");
    assert_eq!(
        c.lines().len(),
        prefix.len(),
        "exactly the pre-trap lines should survive"
    );
}

/// Self-check: the harness must be able to tell a trap from a clean run,
/// otherwise row 1 could "pass" for the wrong reason.
#[test]
fn test_err_harness_distinguishes_trap_from_success() {
    let clean_c = run_ops_with(&[op(Side::C, 7, 2)], true);
    let clean_r = run_ops_with(&[op(Side::Rust, 7, 2)], true);
    assert_eq!(clean_c.signal, None, "driver(7, 2) must not trap (C)");
    assert_eq!(clean_r.signal, None, "driver(7, 2) must not trap (Rust)");
    assert_eq!(clean_c.code, Some(0));
    assert_eq!(clean_r.code, Some(0));
    assert_eq!(clean_c.stdout, b"quotient: 3, remainder: 1\n");
    assert_eq!(clean_c.stdout, clean_r.stdout);

    let trapped_c = run_ops_with(&[op(Side::C, 7, 0)], true);
    let trapped_r = run_ops_with(&[op(Side::Rust, 7, 0)], true);
    assert_eq!(trapped_c.signal, Some(SIGFPE));
    assert_eq!(trapped_r.signal, Some(SIGFPE));
    assert_ne!(clean_c.signal, trapped_c.signal);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 — INT_MIN / -1 -> SIGFPE
// ---------------------------------------------------------------------------

#[test]
fn test_err_row2_int_min_div_neg_one_sigfpe() {
    assert_same_trap(i32::MIN, -1);

    // It is specifically this pair: the near misses must all run cleanly.
    for (x, y) in [
        (i32::MIN + 1, -1),
        (i32::MIN, 1),
        (i32::MIN, -2),
        (i32::MIN / 2, -1),
        (i32::MAX, -1),
    ] {
        let c = run_ops_with(&[op(Side::C, x, y)], true);
        let r = run_ops_with(&[op(Side::Rust, x, y)], true);
        assert_eq!(c.signal, None, "C unexpectedly trapped on driver({x}, {y})");
        assert_eq!(r.signal, None, "Rust unexpectedly trapped on driver({x}, {y})");
        assert_eq!(c.stdout, r.stdout, "driver({x}, {y}) diverged");
    }
}

/// The 64-bit-register variant of row 2: a caller whose upper register halves
/// hold garbage must still trap, on both sides.
#[test]
fn test_err_row2_traps_with_garbage_upper_halves() {
    let hi = 0x1234_5678_0000_0000u64 as i64;
    let x = hi | (i32::MIN as u32 as i64);
    let y = hi | (-1i32 as u32 as i64);

    let c = run_ops_with(&[op_raw(Side::C64, x, y)], true);
    let r = run_ops_with(&[op_raw(Side::Rust64, x, y)], true);
    assert_eq!(c.signal, r.signal, "trap parity with garbage upper halves");
    assert_eq!(c.signal, Some(SIGFPE));
    assert_eq!(c.stdout, r.stdout);

    // ...and the y == 0 trap likewise.
    let zy = hi; // low half is zero
    let c = run_ops_with(&[op_raw(Side::C64, hi | 42, zy)], true);
    let r = run_ops_with(&[op_raw(Side::Rust64, hi | 42, zy)], true);
    assert_eq!(c.signal, r.signal);
    assert_eq!(c.signal, Some(SIGFPE));
    assert_eq!(c.stdout, r.stdout);
}

// ---------------------------------------------------------------------------
// Generic boundaries
// ---------------------------------------------------------------------------

/// The values immediately around both traps must *not* trap, and must match.
/// This is where an over-eager "fix" in the Rust (e.g. `checked_div`, or a Rust
/// `/` that panics on overflow) would show up as a spurious panic or abort.
#[test]
fn test_trap_neighbours_do_not_trap() {
    let neighbours: [(c_int, c_int); 16] = [
        (i32::MIN, 1),      // adjacent to INT_MIN / -1
        (i32::MIN, -2),
        (i32::MIN, 2),
        (i32::MIN, i32::MIN),
        (i32::MIN + 1, -1), // one step from the trapping dividend
        (i32::MIN + 1, 1),
        (-1, -1),
        (0, -1),
        (0, 1),
        (1, -1),
        (1, 1),
        (-1, 1),
        (i32::MAX, -1),
        (i32::MAX, 1),
        (2, -1),
        (-2, -1),
    ];

    for (x, y) in neighbours {
        assert!(!traps(x, y), "test bug: ({x}, {y}) is a trapping pair");
        let c = run_ops_with(&[op(Side::C, x, y)], true);
        let r = run_ops_with(&[op(Side::Rust, x, y)], true);
        assert_eq!(c.signal, None, "C unexpectedly trapped on driver({x}, {y})");
        assert_eq!(
            r.signal, None,
            "Rust unexpectedly trapped on driver({x}, {y})"
        );
        assert_eq!(c.code, Some(0));
        assert_eq!(r.code, Some(0));
        assert_eq!(
            c.stdout,
            r.stdout,
            "driver({x}, {y}) diverged\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&r.stdout)
        );
    }

    // INT_MIN / -1 traps, but INT_MIN+1 / -1 must produce INT_MAX.
    let c = run_ops(&[op(Side::C, i32::MIN + 1, -1)]);
    assert_eq!(c.stdout, b"quotient: 2147483647, remainder: 0\n");
}

/// `int` has no invalid bit patterns, so "one step past the valid range" means
/// crossing every extreme with every other extreme.
#[test]
fn test_full_int_boundary_matrix() {
    let extras: [c_int; 4] = [-2, 2, 3, -3];
    let all: Vec<c_int> = BOUNDARIES.iter().copied().chain(extras).collect();

    let mut ok = Vec::new();
    let mut trapping = Vec::new();
    for &x in &all {
        for &y in &all {
            if traps(x, y) {
                trapping.push((x, y));
            } else {
                ok.push((x, y));
            }
        }
    }

    assert_same_all("boundary matrix (non-trapping)", &ok);
    for (x, y) in trapping.iter().copied() {
        assert_same_trap(x, y);
    }

    assert_eq!(ok.len() + trapping.len(), all.len() * all.len());
    assert!(!ok.is_empty() && !trapping.is_empty());
    eprintln!(
        "boundary matrix: {} matched, {} trapped identically",
        ok.len(),
        trapping.len()
    );
}

/// Extreme magnitudes and the values adjacent to them, crossed with each other.
#[test]
fn test_extremes_and_one_step_past_boundaries() {
    let interesting: [c_int; 14] = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2,
        -65537,
        -65536,
        -256,
        -1,
        1,
        256,
        65536,
        65537,
        i32::MAX / 2,
        i32::MAX - 1,
        i32::MAX,
    ];
    let pairs: Vec<(c_int, c_int)> = interesting
        .iter()
        .flat_map(|&x| interesting.iter().map(move |&y| (x, y)))
        .filter(|&(x, y)| !traps(x, y)) // traps covered by rows 1 and 2
        .collect();
    assert_same_all("extremes matrix", &pairs);
}

/// The ABI edge every `extern "C"` boundary has: a caller that leaves garbage in
/// the upper halves of the argument registers. Both implementations must look at
/// the low 32 bits only, and must agree with the clean 32-bit call.
#[test]
fn test_garbage_in_upper_register_halves() {
    let mut rng = Rng::new(0xABCD_1234);
    let mut clean_pairs = Vec::new();
    let mut c_ops = Vec::new();
    let mut r_ops = Vec::new();

    for _ in 0..500 {
        let lo_x = rng.next_i32();
        let mut lo_y = rng.nonzero_i32();
        if lo_x == i32::MIN && lo_y == -1 {
            lo_y = 1;
        }
        // Non-zero garbage in the upper halves.
        let hi_x = ((rng.next_u64() as u32 as u64) | 1) << 32;
        let hi_y = ((rng.next_u64() as u32 as u64) | 1) << 32;
        let gx = (hi_x | lo_x as u32 as u64) as i64;
        let gy = (hi_y | lo_y as u32 as u64) as i64;

        clean_pairs.push((lo_x, lo_y));
        c_ops.push(op_raw(Side::C64, gx, gy));
        r_ops.push(op_raw(Side::Rust64, gx, gy));
    }

    let c = run_ops(&c_ops);
    let r = run_ops(&r_ops);
    assert_eq!(c.signal, None, "C worker died: {:?}", c.signal);
    assert_eq!(r.signal, None, "Rust worker died: {:?}", r.signal);
    assert_eq!(
        c.stdout, r.stdout,
        "garbage upper register halves produced different output"
    );

    // And both must equal the clean 32-bit calls.
    let clean_c = run_ops(&ops_for(Side::C, &clean_pairs));
    let clean_r = run_ops(&ops_for(Side::Rust, &clean_pairs));
    assert_eq!(
        c.stdout, clean_c.stdout,
        "C: upper register halves changed the result"
    );
    assert_eq!(
        r.stdout, clean_r.stdout,
        "Rust: upper register halves changed the result"
    );
    assert_eq!(c.lines().len(), clean_pairs.len());
}

/// `driver` returns `void` and takes no pointers, so there is no null-pointer or
/// length parameter to abuse. What *can* be abused is the return value: a caller
/// that reads one must see both sides behave the same (namely: no crash, and the
/// same side effect).
#[test]
fn test_void_return_and_no_pointer_parameters() {
    let pairs: [(c_int, c_int); 6] = [
        (1, 1),
        (-9, 4),
        (i32::MAX, 3),
        (i32::MIN, 7),
        (0, i32::MIN),
        (i32::MIN, i32::MIN),
    ];
    assert_same_all("void return", &pairs);

    // Reading a nonexistent return value must not change behaviour either.
    type DriverRet = unsafe extern "C" fn(c_int, c_int) -> c_int;
    let (c, r) = common::drivers();
    let _c_ret: DriverRet = unsafe { std::mem::transmute(c) };
    let _r_ret: DriverRet = unsafe { std::mem::transmute(r) };
    // The transmutes themselves are the point: both symbols have the identical
    // C signature, so an external caller can mis-declare either one the same
    // way. Behaviour parity is asserted through the batch above.
    assert_same(-9, 4);
}

/// A symbol that neither library defines must fail to resolve in *both*, so the
/// Rust `.so` is not silently exporting extra API surface under a C-looking name
/// (and so the loader lookups in this harness are genuinely resolving symbols).
#[test]
fn test_no_extra_or_missing_public_symbol() {
    use libloading::{Library, Symbol};

    let c_lib = unsafe { Library::new(common::c_so_path()) }.expect("dlopen C");
    let r_lib = unsafe { Library::new(common::rust_so_path()) }.expect("dlopen Rust");

    // `driver` resolves in both, at distinct addresses (two real libraries).
    unsafe {
        let cs: Symbol<common::DriverFn> = c_lib.get(b"driver\0").expect("C driver");
        let rs: Symbol<common::DriverFn> = r_lib.get(b"driver\0").expect("Rust driver");
        let ca = *cs as usize;
        let ra = *rs as usize;
        assert_ne!(ca, 0, "C `driver` resolved to a null address");
        assert_ne!(ra, 0, "Rust `driver` resolved to a null address");
        assert_ne!(
            ca, ra,
            "C and Rust `driver` resolved to the same address; the harness is \
             comparing one library against itself"
        );
    }

    // A name the C library does not define must not appear in the Rust one.
    for name in [
        b"driver_init\0".as_slice(),
        b"driver2\0".as_slice(),
        b"c_div\0".as_slice(),
        b"driver_ex\0".as_slice(),
    ] {
        let in_c = unsafe { c_lib.get::<common::DriverFn>(name) }.is_ok();
        let in_r = unsafe { r_lib.get::<common::DriverFn>(name) }.is_ok();
        assert_eq!(
            in_c,
            in_r,
            "symbol {:?}: present in C = {in_c}, present in Rust = {in_r}",
            String::from_utf8_lossy(name)
        );
        assert!(!in_c, "test assumption: {:?} should not exist", String::from_utf8_lossy(name));
    }
}
