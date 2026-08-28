//! Systematic sweep of `matrix_to_string`, staying inside the region where the
//! C size estimate is actually sufficient.
//!
//! The C code sizes the buffer as `h*(11*w) + h + 1` but writes
//! `sum(digit_len) + h*(w-1) + h + 1` bytes. So the allocation suffices exactly
//! when `sum(digit_len) <= 10*w*h + h`, i.e. when values average at most ten
//! characters per cell. Beyond that the original overruns its heap block
//! (undefined behaviour, and glibc aborts on the subsequent free), so those
//! inputs have no well-defined byte output to compare against. The translation
//! performs the identical writes; this test pins down the defined region,
//! including the exact boundary.

mod common;

use common::*;
use std::ffi::c_int;

/// Bytes `%d` produces for `v`.
fn dlen(v: c_int) -> usize {
    v.to_string().len()
}

/// True when C's `buffer_size` covers everything it writes, NUL included.
fn c_buffer_suffices(w: c_int, h: c_int, vals: &[c_int]) -> bool {
    if w <= 0 || h <= 0 {
        return true;
    }
    let sum: usize = vals.iter().map(|&v| dlen(v)).sum();
    let limit = 10usize * (w as usize) * (h as usize) + (h as usize);
    sum <= limit
}

fn cmp(w: c_int, h: c_int, vals: &[c_int]) {
    assert!(
        c_buffer_suffices(w, h, vals),
        "test input {w}x{h} overruns the C buffer; not a defined comparison"
    );
    let p = pair();
    unsafe {
        let cm = make_matrix(&p.c, w, h, vals);
        let rm = make_matrix(&p.rs, w, h, vals);
        let cs = take_cstring((p.c.matrix_to_string)(cm));
        let rs = take_cstring((p.rs.matrix_to_string)(rm));
        assert_eq!(
            cs.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
            rs.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
            "matrix_to_string({w}x{h}, {vals:?}) differs"
        );
        (p.c.free_matrix)(cm);
        (p.rs.free_matrix)(rm);
    }
}

#[test]
fn exact_boundary_single_column() {
    // w == 1 with 11-character values fills the buffer exactly: 12*h + 1 bytes
    // written into 12*h + 1 allocated.
    cmp(1, 1, &[i32::MIN]);
    cmp(1, 2, &[i32::MIN, i32::MIN]);
    cmp(1, 4, &[i32::MIN; 4]);
    cmp(1, 8, &[-1000000000; 8]);
    cmp(1, 16, &[i32::MIN; 16]);
}

#[test]
fn ten_char_values_always_fit() {
    // The widest values that are safe for any width: 10 characters.
    for &w in &[1i32, 2, 3, 5, 8] {
        for &h in &[1i32, 2, 3, 7] {
            let n = (w * h) as usize;
            cmp(w, h, &vec![i32::MAX; n]);
            cmp(w, h, &vec![-999999999; n]);
            cmp(w, h, &vec![1000000000; n]);
        }
    }
}

#[test]
fn mixed_widths_within_budget() {
    // One wide value per row, padded with short ones, stays under budget.
    cmp(4, 1, &[i32::MIN, 0, 1, -1]);
    cmp(4, 2, &[i32::MIN, 0, 1, -1, i32::MAX, 2, -2, 3]);
    cmp(5, 3, &[
        i32::MIN, 1, 2, 3, 4,
        i32::MAX, 5, 6, 7, 8,
        -1000000000, 9, 10, 11, 12,
    ]);
}

#[test]
fn deterministic_sweep() {
    // A spread of shapes and value magnitudes, all inside the defined region.
    let magnitudes: &[c_int] = &[0, 1, -1, 9, -9, 10, -10, 99, -100, 1234, -12345,
                                 999999, -1000000, 123456789, -123456789];
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };

    for w in 1..=6i32 {
        for h in 1..=6i32 {
            let n = (w * h) as usize;
            let vals: Vec<c_int> = (0..n)
                .map(|_| magnitudes[(next() as usize) % magnitudes.len()])
                .collect();
            cmp(w, h, &vals);
        }
    }
}
