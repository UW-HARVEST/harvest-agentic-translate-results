//! Level 1: `void fma_array(int *restrict out, const int *mul1,
//!                          const int *mul2, const int *add, int len)`

mod common;

use common::*;
use std::ffi::c_int;

/// Runs both implementations over the same inputs and compares the whole output
/// buffer byte-for-byte, including the guard region past `len`.
fn compare(mul1: &[c_int], mul2: &[c_int], add: &[c_int], len: c_int, guard: usize) {
    let cap = mul1.len().max(guard);
    // Distinct sentinel so any write outside [0, len) shows up as a difference.
    let sentinel: c_int = 0x5A5A_5A5A_u32 as c_int;
    let mut c_out: Vec<c_int> = vec![sentinel; cap];
    let mut r_out: Vec<c_int> = vec![sentinel; cap];

    let cf = c_fma_array();
    let rf = rust_fma_array();

    unsafe {
        cf(
            c_out.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            len,
        );
        rf(
            r_out.as_mut_ptr(),
            mul1.as_ptr(),
            mul2.as_ptr(),
            add.as_ptr(),
            len,
        );
    }

    assert_eq!(
        bytes_of(&c_out),
        bytes_of(&r_out),
        "fma_array mismatch (len={len})\n  mul1={mul1:?}\n  mul2={mul2:?}\n  add={add:?}\n  \
         C   ={c_out:?}\n  Rust={r_out:?}"
    );
}

fn bytes_of(v: &[c_int]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

#[test]
fn len_zero_writes_nothing() {
    let a = [1, 2, 3];
    compare(&a, &a, &a, 0, 8);
}

#[test]
fn negative_len_writes_nothing() {
    // C: `for (int i = 0; i < len; i++)` simply never runs for len < 0.
    let a = [1, 2, 3];
    for len in [-1, -7, c_int::MIN] {
        compare(&a, &a, &a, len, 8);
    }
}

#[test]
fn small_hand_written_cases() {
    compare(&[2], &[3], &[4], 1, 4);
    compare(&[1, 2, 3], &[4, 5, 6], &[7, 8, 9], 3, 6);
    compare(&[0, 0, 0], &[123, -456, 789], &[-1, -2, -3], 3, 6);
    // Partial length: only the first two elements may be touched.
    compare(&[1, 2, 3, 4], &[5, 6, 7, 8], &[9, 10, 11, 12], 2, 8);
}

#[test]
fn extreme_values_and_wraparound() {
    let mul1 = [
        c_int::MAX,
        c_int::MIN,
        c_int::MAX,
        c_int::MIN,
        -1,
        2,
        65536,
        c_int::MIN,
    ];
    let mul2 = [
        2,
        2,
        c_int::MAX,
        c_int::MIN,
        c_int::MIN,
        1_073_741_824,
        65536,
        -1,
    ];
    let add = [
        1,
        -1,
        c_int::MAX,
        c_int::MIN,
        0,
        c_int::MAX,
        c_int::MIN,
        c_int::MAX,
    ];
    let len = mul1.len() as c_int;
    compare(&mul1, &mul2, &add, len, mul1.len() + 4);
}

#[test]
fn aliasing_mul1_and_mul2() {
    // `out` is `restrict`, the reads are not; passing the same buffer for both
    // multiplicands is well-defined.
    let a: Vec<c_int> = (0..64).map(|i| i * 7 - 100).collect();
    let b: Vec<c_int> = (0..64).map(|i| 3 - i).collect();
    compare(&a, &a, &b, 64, 68);
}

#[test]
fn randomized() {
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..iters(4000) {
        let n = 1 + rng.below(64);
        let mul1: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        let mul2: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        let add: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        let len = rng.below(n + 1) as c_int;
        compare(&mul1, &mul2, &add, len, n + 2);
    }
}

#[test]
fn randomized_small_magnitudes() {
    // Values small enough that the products do not overflow, exercising the
    // ordinary arithmetic path.
    let mut rng = Rng::new(0xBADC0DE);
    for _ in 0..iters(4000) {
        let n = 1 + rng.below(32);
        let mul1: Vec<c_int> = (0..n).map(|_| (rng.next_u32() % 2001) as c_int - 1000).collect();
        let mul2: Vec<c_int> = (0..n).map(|_| (rng.next_u32() % 2001) as c_int - 1000).collect();
        let add: Vec<c_int> = (0..n).map(|_| (rng.next_u32() % 2001) as c_int - 1000).collect();
        compare(&mul1, &mul2, &add, n as c_int, n + 2);
    }
}

#[test]
fn long_array() {
    let n = 10_000usize;
    let mul1: Vec<c_int> = (0..n).map(|i| (i as c_int).wrapping_mul(2_654_435_761u32 as c_int)).collect();
    let mul2: Vec<c_int> = (0..n).map(|i| i as c_int - 5000).collect();
    let add: Vec<c_int> = (0..n).map(|i| (i as c_int).wrapping_neg()).collect();
    compare(&mul1, &mul2, &add, n as c_int, n);
}
