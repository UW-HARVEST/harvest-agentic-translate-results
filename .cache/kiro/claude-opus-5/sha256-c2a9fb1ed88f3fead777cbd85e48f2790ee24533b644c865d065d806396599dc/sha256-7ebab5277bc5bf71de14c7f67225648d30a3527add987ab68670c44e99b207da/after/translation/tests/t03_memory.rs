//! Level 3: the memory-shuffling helpers — `shift_array_data`,
//! `compute_with_dynamic_memory` and `manipulate_records`.
//!
//! For the two functions that mutate caller memory the *buffer* is compared
//! byte-for-byte after the call, not just the return value.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

fn bytes_of<T>(v: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }
}

#[test]
fn shift_array_data_matches() {
    let libs = load();
    let (shift_c, shift_r) = libs.pair::<FnShiftArray>("shift_array_data");

    // A recognisable pattern so any mis-shift shows up immediately.
    let seed: Vec<c_int> = (0..24).map(|i| 1000 + i * 7).collect();

    // `size` is only ever compared against `shift_by`, so negative/zero sizes
    // must be exercised too (they make the guard fail and the call a no-op).
    let sizes: &[c_int] = &[-5, -1, 0, 1, 2, 3, 8, 16, 24];
    let shifts: &[c_int] = &[
        i32::MIN,
        -100,
        -1,
        0,
        1,
        2,
        3,
        7,
        8,
        15,
        16,
        23,
        24,
        25,
        1000,
        i32::MAX,
    ];

    for &size in sizes {
        for &shift in shifts {
            // Guard: only feed in-bounds work to the libraries. The C body
            // touches memory only when 0 < shift_by < size, and then only the
            // first `size` elements.
            if shift > 0 && shift < size && size as usize > seed.len() {
                continue;
            }

            let mut a = seed.clone();
            let mut b = seed.clone();
            unsafe {
                shift_c(a.as_mut_ptr(), size, shift);
                shift_r(b.as_mut_ptr(), size, shift);
            }
            assert_eq!(
                bytes_of(&a),
                bytes_of(&b),
                "shift_array_data(buf, {size}, {shift})\n  C:    {a:?}\n  Rust: {b:?}"
            );
        }
    }

    // Zero-length buffers: the guard rejects every shift, so a dangling-but-
    // aligned pointer is never dereferenced (same as in C).
    let mut empty_a: Vec<c_int> = Vec::new();
    let mut empty_b: Vec<c_int> = Vec::new();
    for &shift in shifts {
        unsafe {
            shift_c(empty_a.as_mut_ptr(), 0, shift);
            shift_r(empty_b.as_mut_ptr(), 0, shift);
        }
    }
}

#[test]
fn compute_with_dynamic_memory_matches() {
    let libs = load();
    let (f_c, f_r) = libs.pair::<FnBinary>("compute_with_dynamic_memory");

    // Non-positive counts make the C loops degenerate (and, for negative
    // counts, malloc fails) — the function still returns 0.
    let counts: &[c_int] = &[
        i32::MIN,
        -1_000_000,
        -100,
        -2,
        -1,
        0,
        1,
        2,
        3,
        8,
        9,
        10,
        100,
        1000,
        65_536,
        200_000,
    ];

    for &base in INTS {
        for &count in counts {
            let ec = unsafe { f_c(base, count) };
            let er = unsafe { f_r(base, count) };
            assert_eq!(ec, er, "compute_with_dynamic_memory({base}, {count})");
        }
    }

    // Counts big enough that `i * 3` and the running sum both wrap.
    for &count in &[1_000_000_i32, 1_500_000, 2_000_000] {
        for &base in &[0_i32, 1, -1, i32::MAX, i32::MIN, 123_456_789] {
            let ec = unsafe { f_c(base, count) };
            let er = unsafe { f_r(base, count) };
            assert_eq!(ec, er, "compute_with_dynamic_memory({base}, {count})");
        }
    }
}

fn make_records(n: usize) -> Vec<DataRecord> {
    (0..n)
        .map(|i| {
            let mut r = DataRecord::new(i as c_int, (i as c_int) * 13 - 40);
            // Distinct, deterministic filler so a wrong struct stride shows up
            // in the byte-for-byte buffer comparison.
            r.timestamp = 0x0102_0304_0500_0000 + i as i64;
            for (k, slot) in r.name.iter_mut().enumerate() {
                *slot = (b'a' + ((i + k) % 26) as u8) as c_char;
            }
            r.name[31] = 0;
            r
        })
        .collect()
}

#[test]
fn manipulate_records_matches() {
    let libs = load();
    let (mr_c, mr_r) = libs.pair::<FnManipulateRecords>("manipulate_records");

    // The trailing loop runs to `num_records - shift`, which for a negative
    // shift walks *past* `num_records`. Over-allocate so those reads stay
    // inside a deterministically initialised buffer instead of being garbage.
    const CAP: usize = 64;
    let seed = make_records(CAP);

    let cases: &[(c_int, c_int)] = &[
        (0, 0),
        (1, 0),
        (1, 1),
        (2, 1),
        (5, 0),
        (5, 1),
        (5, 2),
        (5, 3),
        (5, 4),
        (5, 5),
        (5, 6),
        (5, -1),
        (5, -3),
        (5, -20),
        (8, 3),
        (16, 7),
        (32, 31),
        (32, 1),
        (32, 16),
        (0, 5),
        (0, -5),
        (-4, 2),
        (-4, -2),
    ];

    for &(num_records, shift) in cases {
        let hi = (num_records as i64) - (shift as i64);
        if hi > CAP as i64 || num_records as i64 > CAP as i64 {
            continue; // would read outside the buffer in *both* impls
        }

        let mut a = seed.clone();
        let mut b = seed.clone();
        let ec = unsafe { mr_c(a.as_mut_ptr(), num_records, shift) };
        let er = unsafe { mr_r(b.as_mut_ptr(), num_records, shift) };

        assert_eq!(ec, er, "manipulate_records(buf, {num_records}, {shift}) return");
        assert_eq!(
            bytes_of(&a),
            bytes_of(&b),
            "manipulate_records(buf, {num_records}, {shift}) buffer contents"
        );
    }

    // Values chosen to overflow the `total` accumulator.
    let mut big: Vec<DataRecord> = (0..32)
        .map(|i| DataRecord::new(i, if i % 2 == 0 { i32::MAX } else { i32::MIN }))
        .collect();
    let mut big2 = big.clone();
    let ec = unsafe { mr_c(big.as_mut_ptr(), 32, 5) };
    let er = unsafe { mr_r(big2.as_mut_ptr(), 32, 5) };
    assert_eq!(ec, er, "manipulate_records overflow return");
    assert_eq!(bytes_of(&big), bytes_of(&big2), "manipulate_records overflow buffer");
}
