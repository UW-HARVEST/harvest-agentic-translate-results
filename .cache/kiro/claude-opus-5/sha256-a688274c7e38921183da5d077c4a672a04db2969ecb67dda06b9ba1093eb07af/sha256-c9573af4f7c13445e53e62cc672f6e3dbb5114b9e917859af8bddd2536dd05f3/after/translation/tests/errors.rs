//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. Rows the C handles gracefully are called
//! in-process and their (non-)effects diffed; rows that fault are run in a
//! `fork()`ed child for BOTH implementations and the termination status is
//! compared, so a pass means "the C and the Rust reject this input the same
//! way" (same signal, or same exit code) rather than "both broke somehow".

mod common;

use common::*;
use std::os::raw::c_int;

const SENTINEL: c_int = -0x5EED_BEEF;
const GUARD_LEN: usize = 32;

/// A length large enough that indexing with it runs far past any real
/// allocation, but still a positive `int`.
const OVERSIZED: c_int = 1 << 24; // 16 Mi elements = 64 MiB

// ---------------------------------------------------------------------------
// Rows 1-5 — `fma_array` with a non-positive length: the loop guard makes the
// body unreachable, so even all-null pointers must be a silent no-op.
// ---------------------------------------------------------------------------

/// Call `fma_array` with the given (possibly null) pointers and length on both
/// implementations and require identical behaviour and identical buffer state.
fn expect_fma_noop(row: &str, len: c_int, null_ptrs: bool) {
    preload_both();
    let fc = fma_array_of(Impl::C);
    let fr = fma_array_of(Impl::Rust);

    for (which, f) in [(Impl::C, fc), (Impl::Rust, fr)] {
        let mut out = vec![SENTINEL; GUARD_LEN];
        let m1 = vec![7 as c_int; GUARD_LEN];
        let m2 = vec![11 as c_int; GUARD_LEN];
        let a = vec![13 as c_int; GUARD_LEN];
        if null_ptrs {
            unsafe {
                f(
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    len,
                )
            };
        } else {
            unsafe { f(out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), a.as_ptr(), len) };
        }
        assert!(
            out.iter().all(|&x| x == SENTINEL),
            "[{row}] {} wrote to `out` with len={len}",
            which.name()
        );
        assert!(
            m1.iter().all(|&x| x == 7) && m2.iter().all(|&x| x == 11) && a.iter().all(|&x| x == 13),
            "[{row}] {} clobbered an input buffer with len={len}",
            which.name()
        );
    }
}

#[test]
fn err_01_fma_len_zero_no_writes() {
    expect_fma_noop("err_01", 0, false);
}

#[test]
fn err_02_fma_len_negative() {
    expect_fma_noop("err_02", -1, false);
    expect_fma_noop("err_02", -7, false);
}

#[test]
fn err_03_fma_len_int_min() {
    expect_fma_noop("err_03", c_int::MIN, false);
    expect_fma_noop("err_03", c_int::MIN + 1, false);
}

#[test]
fn err_04_fma_all_null_len_zero() {
    expect_fma_noop("err_04", 0, true);
}

#[test]
fn err_05_fma_all_null_len_negative() {
    expect_fma_noop("err_05", -1, true);
    expect_fma_noop("err_05", c_int::MIN, true);
}

// ---------------------------------------------------------------------------
// Rows 6-10 — `fma_array` faulting cases. Compared via fork exit status.
// ---------------------------------------------------------------------------

/// Which of the four pointer arguments to poison with NULL.
#[derive(Copy, Clone)]
enum NullArg {
    Out,
    Mul1,
    Mul2,
    Add,
    None,
}

fn fma_call_with_null(which: Impl, arg: NullArg, len: c_int) {
    let f = fma_array_of(which);
    let mut out = vec![0 as c_int; GUARD_LEN];
    let m1 = vec![3 as c_int; GUARD_LEN];
    let m2 = vec![5 as c_int; GUARD_LEN];
    let a = vec![9 as c_int; GUARD_LEN];
    let (po, p1, p2, pa) = match arg {
        NullArg::Out => (
            std::ptr::null_mut(),
            m1.as_ptr(),
            m2.as_ptr(),
            a.as_ptr(),
        ),
        NullArg::Mul1 => (out.as_mut_ptr(), std::ptr::null(), m2.as_ptr(), a.as_ptr()),
        NullArg::Mul2 => (out.as_mut_ptr(), m1.as_ptr(), std::ptr::null(), a.as_ptr()),
        NullArg::Add => (out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), std::ptr::null()),
        NullArg::None => (out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), a.as_ptr()),
    };
    // Leak the buffers: an out-of-bounds write corrupts the allocator's
    // metadata, so freeing afterwards would add allocator noise on top of the
    // fault we are actually comparing.
    std::mem::forget((out, m1, m2, a));
    unsafe { f(po, p1, p2, pa, len) };
}

fn diff_fma_fault(row: &str, arg: NullArg, len: c_int) {
    preload_both();
    assert_same_outcome(
        row,
        || fma_call_with_null(Impl::C, arg, len),
        || fma_call_with_null(Impl::Rust, arg, len),
    );
}

#[test]
fn err_06_fma_null_out_len_positive() {
    diff_fma_fault("err_06", NullArg::Out, 1);
    diff_fma_fault("err_06", NullArg::Out, 64);
}

#[test]
fn err_07_fma_null_mul1_len_positive() {
    diff_fma_fault("err_07", NullArg::Mul1, 1);
    diff_fma_fault("err_07", NullArg::Mul1, 64);
}

#[test]
fn err_08_fma_null_mul2_len_positive() {
    diff_fma_fault("err_08", NullArg::Mul2, 1);
    diff_fma_fault("err_08", NullArg::Mul2, 64);
}

#[test]
fn err_09_fma_null_add_len_positive() {
    diff_fma_fault("err_09", NullArg::Add, 1);
    diff_fma_fault("err_09", NullArg::Add, 64);
}

#[test]
fn err_10_fma_len_oversized() {
    // Buffers hold GUARD_LEN elements; `len` claims 16 Mi. Both must walk off
    // the end and be killed the same way.
    diff_fma_fault("err_10", NullArg::None, OVERSIZED);
}

// ---------------------------------------------------------------------------
// Rows 11-13 — signed-overflow behaviour of the arithmetic (UB in C, no check
// present, wraps in the emitted code). Diffed value-for-value in-process.
// ---------------------------------------------------------------------------

fn expect_fma_values(row: &str, m1: &[c_int], m2: &[c_int], a: &[c_int]) {
    preload_both();
    let n = m1.len();
    assert!(n == m2.len() && n == a.len());
    let mut results = Vec::new();
    for which in [Impl::C, Impl::Rust] {
        let f = fma_array_of(which);
        let mut out = vec![SENTINEL; n];
        unsafe { f(out.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), a.as_ptr(), n as c_int) };
        results.push(out);
    }
    assert_eq!(
        results[0], results[1],
        "[{row}] overflow result mismatch\n  mul1={m1:?}\n  mul2={m2:?}\n  add={a:?}\n  \
         C={:?}\n  Rust={:?}",
        results[0], results[1]
    );
}

#[test]
fn err_11_fma_mul_overflow_wraps() {
    let m1 = [i32::MAX, i32::MAX, i32::MIN, i32::MIN, 65536, -65536, 46341];
    let m2 = [i32::MAX, 2, i32::MIN, 2, 65536, 65536, 46341];
    let a = [0, 0, 0, 0, 0, 0, 0];
    expect_fma_values("err_11", &m1, &m2, &a);
}

#[test]
fn err_12_fma_add_overflow_wraps() {
    // product == INT_MAX, then + 1 overflows; and product == INT_MIN, then - 1.
    let m1 = [i32::MAX, 1, i32::MIN, 1, i32::MAX, i32::MIN];
    let m2 = [1, i32::MAX, 1, i32::MIN, 1, 1];
    let a = [1, 1, -1, -1, i32::MAX, i32::MIN];
    expect_fma_values("err_12", &m1, &m2, &a);
}

#[test]
fn err_13_fma_int_min_times_minus_one() {
    let m1 = [i32::MIN, -1, i32::MIN, -1];
    let m2 = [-1, i32::MIN, -1, i32::MIN];
    let a = [0, 0, i32::MIN, i32::MAX];
    expect_fma_values("err_13", &m1, &m2, &a);
}

// ---------------------------------------------------------------------------
// Rows 14-15 — `driver` degenerate but non-faulting inputs.
// ---------------------------------------------------------------------------

#[test]
fn err_14_driver_len_zero_no_output() {
    preload_both();
    let data = vec![1 as c_int, 2, 3, 4];
    let p = data.as_ptr();
    let c_out = capture_stdout(|| unsafe { driver_of(Impl::C)(p, 0) });
    let r_out = capture_stdout(|| unsafe { driver_of(Impl::Rust)(p, 0) });
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "[err_14] stdout differs for len=0"
    );
    assert!(
        c_out.is_empty(),
        "[err_14] C produced output for len=0: {:?}",
        String::from_utf8_lossy(&c_out)
    );
}

#[test]
fn err_15_driver_null_data_len_zero() {
    preload_both();
    let c_out = capture_stdout(|| unsafe { driver_of(Impl::C)(std::ptr::null(), 0) });
    let r_out = capture_stdout(|| unsafe { driver_of(Impl::Rust)(std::ptr::null(), 0) });
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "[err_15] stdout differs for data=NULL, len=0"
    );
    assert!(c_out.is_empty(), "[err_15] unexpected output");
}

// ---------------------------------------------------------------------------
// Rows 16-20 — `driver` faulting cases, compared via fork exit status.
// ---------------------------------------------------------------------------

/// Call `driver` with a buffer of `buf_len` elements but declaring `len`.
fn driver_call(which: Impl, buf_len: usize, len: c_int, null_data: bool) {
    let d = driver_of(which);
    let data = vec![5 as c_int; buf_len];
    let p = if null_data {
        std::ptr::null()
    } else {
        data.as_ptr()
    };
    std::mem::forget(data);
    unsafe { d(p, len) };
}

fn diff_driver_fault(row: &str, buf_len: usize, len: c_int, null_data: bool) {
    preload_both();
    assert_same_outcome(
        row,
        || driver_call(Impl::C, buf_len, len, null_data),
        || driver_call(Impl::Rust, buf_len, len, null_data),
    );
}

#[test]
fn err_16_driver_null_data_len_positive() {
    diff_driver_fault("err_16", 0, 1, true);
    diff_driver_fault("err_16", 0, 64, true);
}

#[test]
fn err_17_driver_len_minus_one() {
    // `len * sizeof(int)` converts int -1 to size_t => 0xFFFFFFFFFFFFFFFC.
    diff_driver_fault("err_17", 16, -1, false);
    diff_driver_fault("err_17", 16, -7, false);
}

#[test]
fn err_18_driver_len_int_min() {
    diff_driver_fault("err_18", 16, c_int::MIN, false);
    diff_driver_fault("err_18", 16, c_int::MIN + 1, false);
}

#[test]
fn err_19_driver_len_oversized() {
    // 16 elements available, 16 Mi claimed.
    diff_driver_fault("err_19", 16, OVERSIZED, false);
}

#[test]
fn err_20_driver_vla_stack_overflow() {
    // 1 << 28 ints = 1 GiB VLA, far beyond any default stack.
    diff_driver_fault("err_20", 16, 1 << 28, false);
}

// ---------------------------------------------------------------------------
// Generic FFI boundary sweep required by Phase C, beyond the table rows.
// `len` is the only non-pointer parameter of either function and it is a plain
// `int` (there is no enum, mode or flag anywhere in the public API), so the
// "out-of-range enum value" class of input is covered by sweeping the extremes
// of the `int` domain across both entry points.
// ---------------------------------------------------------------------------

#[test]
fn boundary_sweep_len_domain_non_faulting() {
    preload_both();
    // Every non-positive len must be an identical silent no-op in both.
    for len in [0, -1, -2, -3, -100, -32768, -65536, c_int::MIN, c_int::MIN + 1] {
        expect_fma_noop("sweep/fma", len, false);
        expect_fma_noop("sweep/fma-null", len, true);
    }
    // driver with len == 0 must print nothing, with and without a null buffer.
    for null_data in [false, true] {
        let data = vec![9 as c_int; 8];
        let p = if null_data {
            std::ptr::null()
        } else {
            data.as_ptr()
        };
        let c_out = capture_stdout(|| unsafe { driver_of(Impl::C)(p, 0) });
        let r_out = capture_stdout(|| unsafe { driver_of(Impl::Rust)(p, 0) });
        assert_eq!(
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
            "[sweep/driver] len=0 null_data={null_data}"
        );
    }
}

#[test]
fn boundary_sweep_len_domain_faulting() {
    preload_both();
    // One step past the largest length the buffer can serve, and further.
    let buf = 16usize;
    for len in [buf as c_int + 1, 1024, 1 << 20, c_int::MAX] {
        diff_fma_fault("sweep/fma-oob", NullArg::None, len);
        diff_driver_fault("sweep/driver-oob", buf, len, false);
    }
}

#[test]
fn boundary_sweep_valid_len_one_step_inside() {
    // The largest in-range length must still be a normal success in both, so
    // the faulting sweep above is really testing the boundary and not a
    // blanket failure.
    preload_both();
    let n = 16usize;
    let m1: Vec<c_int> = (0..n).map(|i| i as c_int - 8).collect();
    let m2: Vec<c_int> = (0..n).map(|i| 3 * i as c_int).collect();
    let a: Vec<c_int> = (0..n).map(|i| -(i as c_int)).collect();
    expect_fma_values("sweep/inside", &m1, &m2, &a);

    let c_out = capture_stdout(|| unsafe { driver_of(Impl::C)(m1.as_ptr(), n as c_int) });
    let r_out = capture_stdout(|| unsafe { driver_of(Impl::Rust)(m1.as_ptr(), n as c_int) });
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "[sweep/inside] driver stdout differs at the maximum valid length"
    );
    assert!(!c_out.is_empty(), "[sweep/inside] expected output");
}
