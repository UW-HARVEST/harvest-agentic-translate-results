//! Middle layer: `multiply_matrices` and `matrix_to_string`.

mod common;

use common::*;
use std::ffi::c_int;

/// Builds A (wa x ha) and B (wb x hb) in both libraries from the given values,
/// multiplies, and compares the results.
fn mul_case(wa: c_int, ha: c_int, a: &[c_int], wb: c_int, hb: c_int, b: &[c_int]) {
    let p = pair();
    unsafe {
        let ca = make_matrix(&p.c, wa, ha, a);
        let cb = make_matrix(&p.c, wb, hb, b);
        let ra = make_matrix(&p.rs, wa, ha, a);
        let rb = make_matrix(&p.rs, wb, hb, b);

        let cres = (p.c.multiply_matrices)(ca, cb);
        let rres = (p.rs.multiply_matrices)(ra, rb);

        assert_eq!(
            cres.is_null(),
            rres.is_null(),
            "multiply({wa}x{ha} * {wb}x{hb}) nullness differs"
        );
        assert_eq!(
            snapshot(cres, true),
            snapshot(rres, true),
            "multiply({wa}x{ha} * {wb}x{hb}) results differ"
        );

        // Inputs must be untouched.
        assert_eq!(snapshot(ca, true), snapshot(ra, true), "input A mutated");
        assert_eq!(snapshot(cb, true), snapshot(rb, true), "input B mutated");

        (p.c.free_matrix)(cres);
        (p.rs.free_matrix)(rres);
        (p.c.free_matrix)(ca);
        (p.c.free_matrix)(cb);
        (p.rs.free_matrix)(ra);
        (p.rs.free_matrix)(rb);
    }
}

#[test]
fn multiply_basic() {
    mul_case(2, 2, &[1, 2, 3, 4], 2, 2, &[5, 6, 7, 8]);
    mul_case(1, 1, &[3], 1, 1, &[7]);
    mul_case(3, 2, &[1, 2, 3, 4, 5, 6], 2, 3, &[7, 8, 9, 10, 11, 12]);
    mul_case(1, 3, &[1, 2, 3], 3, 1, &[4, 5, 6]);
    mul_case(3, 1, &[1, 2, 3], 1, 3, &[4, 5, 6]);
    // Identity.
    mul_case(3, 3, &[1, 2, 3, 4, 5, 6, 7, 8, 9], 3, 3, &[1, 0, 0, 0, 1, 0, 0, 0, 1]);
    // Zeros and negatives.
    mul_case(2, 2, &[0, 0, 0, 0], 2, 2, &[1, 2, 3, 4]);
    mul_case(2, 2, &[-1, -2, -3, -4], 2, 2, &[-5, -6, -7, -8]);
    mul_case(2, 2, &[-1, 2, 3, -4], 2, 2, &[5, -6, -7, 8]);
}

#[test]
fn multiply_overflow_wraparound() {
    // Signed int overflow: C wraps on this target; the translation must match.
    mul_case(1, 1, &[i32::MAX], 1, 1, &[2]);
    mul_case(1, 1, &[i32::MIN], 1, 1, &[-1]);
    mul_case(1, 1, &[i32::MAX], 1, 1, &[i32::MAX]);
    mul_case(1, 1, &[i32::MIN], 1, 1, &[i32::MIN]);
    mul_case(2, 2, &[i32::MAX, i32::MAX, i32::MIN, i32::MIN], 2, 2,
             &[i32::MAX, 2, -3, i32::MIN]);
    mul_case(4, 1, &[i32::MAX; 4], 1, 4, &[i32::MAX; 4]);
    mul_case(3, 3, &[1 << 20; 9], 3, 3, &[1 << 20; 9]);
    mul_case(2, 2, &[65536, 65536, 65536, 65536], 2, 2, &[65536, 65536, 65536, 65536]);
}

#[test]
fn multiply_dimension_mismatch() {
    // mat_a->width != mat_b->height => NULL plus a stderr message.
    mul_case(2, 2, &[1, 2, 3, 4], 3, 3, &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    mul_case(3, 1, &[1, 2, 3], 1, 1, &[4]);
    mul_case(1, 1, &[1], 2, 2, &[1, 2, 3, 4]);
}

#[test]
fn multiply_zero_dims() {
    // width_a == height_b == 0: allocation of 0x0 rows, empty result.
    mul_case(0, 0, &[], 0, 0, &[]);
    mul_case(0, 2, &[], 3, 0, &[]);
    mul_case(0, 3, &[], 0, 0, &[]);
    // K == 0 leaves every cell explicitly zeroed by the C code.
    mul_case(0, 2, &[], 2, 0, &[]);
}

fn to_string_case(w: c_int, h: c_int, vals: &[c_int]) {
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
        assert_eq!(cs, rs, "matrix_to_string bytes differ");
        (p.c.free_matrix)(cm);
        (p.rs.free_matrix)(rm);
    }
}

#[test]
fn matrix_to_string_null() {
    let p = pair();
    unsafe {
        let cs = take_cstring((p.c.matrix_to_string)(std::ptr::null_mut()));
        let rs = take_cstring((p.rs.matrix_to_string)(std::ptr::null_mut()));
        assert_eq!(cs, rs);
        assert!(cs.is_none());
    }
}

#[test]
fn matrix_to_string_basic() {
    to_string_case(1, 1, &[0]);
    to_string_case(1, 1, &[42]);
    to_string_case(2, 2, &[1, 2, 3, 4]);
    to_string_case(3, 2, &[1, 22, 333, 4444, 55555, 666666]);
    to_string_case(1, 4, &[1, 2, 3, 4]);
    to_string_case(4, 1, &[1, 2, 3, 4]);
    to_string_case(5, 5, &(1..=25).collect::<Vec<c_int>>());
}

#[test]
fn matrix_to_string_widths() {
    // Single-column rows leave the C size estimate with plenty of slack.
    to_string_case(1, 1, &[i32::MAX]);
    to_string_case(1, 1, &[i32::MIN]);
    to_string_case(1, 3, &[i32::MIN, 0, i32::MAX]);
    to_string_case(1, 5, &[-1, -22, -333, -4444, -55555]);
    to_string_case(2, 1, &[-1, 2]);
    to_string_case(2, 2, &[-1, -2, -3, -4]);
    to_string_case(3, 3, &[-100, 200, -300, 400, -500, 600, -700, 800, -900]);
}

#[test]
fn matrix_to_string_zero_dims() {
    to_string_case(0, 0, &[]);
    to_string_case(0, 3, &[]);
    to_string_case(3, 0, &[]);
}
