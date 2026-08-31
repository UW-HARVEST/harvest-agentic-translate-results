//! Level 2: `int call_fma(const int *data, int len)`
//!
//! Only `len >= 0` is compared. For `len < 0` the C version declares
//! `int out[len]` (a negative-size VLA) and then reads `out[len-1]`, so it
//! returns uninitialized stack bytes that differ from one process to the next —
//! there is no reproducible behaviour to match, and `driver` never produces a
//! negative `len` (its counter runs 0..=100).

mod common;

use common::*;
use std::ffi::c_int;

fn compare(data: &[c_int], len: c_int) {
    let cf = c_call_fma();
    let rf = rust_call_fma();
    let c_ret = unsafe { cf(data.as_ptr(), len) };
    let r_ret = unsafe { rf(data.as_ptr(), len) };
    assert_eq!(
        c_ret.to_ne_bytes(),
        r_ret.to_ne_bytes(),
        "call_fma(len={len}) mismatch: C={c_ret} Rust={r_ret}\n  data={data:?}"
    );
}

#[test]
fn len_zero_returns_zero() {
    let data = [11, 22, 33];
    compare(&data, 0);
    // Also with a null-ish but unread pointer: C returns before touching `data`.
    let empty: [c_int; 0] = [];
    compare(&empty, 0);
}

#[test]
fn returns_last_element() {
    // out[i] = 1 * data[i] + 0, so the result is always data[len-1].
    let data: Vec<c_int> = vec![5, -3, 0, 77, c_int::MIN, c_int::MAX];
    for len in 1..=data.len() as c_int {
        compare(&data, len);
    }
}

#[test]
fn extreme_values() {
    for v in [
        0,
        1,
        -1,
        c_int::MAX,
        c_int::MIN,
        c_int::MIN + 1,
        i32::MAX - 1,
        0x5A5A_5A5A_u32 as c_int,
        -0x7FFF_FFFF,
    ] {
        compare(&[v], 1);
        compare(&[0, v], 2);
        compare(&[v, 0], 2);
    }
}

#[test]
fn randomized() {
    let mut rng = Rng::new(0x5EED_1234);
    for _ in 0..iters(6000) {
        let n = 1 + rng.below(128);
        let data: Vec<c_int> = (0..n).map(|_| rng.next_i32()).collect();
        let len = rng.below(n + 1) as c_int; // includes 0
        compare(&data, len);
    }
}

#[test]
fn larger_lengths() {
    // Kept well under the default stack limit: the C version declares three
    // VLAs of `len` ints.
    for &n in &[100usize, 255, 256, 1000, 4096, 20_000] {
        let data: Vec<c_int> = (0..n).map(|i| (i as c_int).wrapping_mul(31).wrapping_sub(7)).collect();
        compare(&data, n as c_int);
        compare(&data, (n - 1) as c_int);
    }
}
