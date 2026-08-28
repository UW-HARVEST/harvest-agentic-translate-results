//! CONFIGS.md rows 1–7 and 41–44: the exported data symbols, and the effect of
//! *tampering* with them (they are non-const globals in C, so they are part of
//! the runtime configuration surface).

mod common;

use common::*;

fn read_u8s(p: &Pair, sym: &[u8], n: usize) -> (Vec<u8>, Vec<u8>) {
    let a: *mut u8 = p.c.data(sym);
    let b: *mut u8 = p.rs.data(sym);
    unsafe {
        (
            std::slice::from_raw_parts(a, n).to_vec(),
            std::slice::from_raw_parts(b, n).to_vec(),
        )
    }
}

fn read_u32s(p: &Pair, sym: &[u8], n: usize) -> (Vec<u32>, Vec<u32>) {
    let a: *mut u32 = p.c.data(sym);
    let b: *mut u32 = p.rs.data(sym);
    unsafe {
        (
            std::slice::from_raw_parts(a, n).to_vec(),
            std::slice::from_raw_parts(b, n).to_vec(),
        )
    }
}

// --- row 1 -----------------------------------------------------------------
#[test]
fn g01_fixed_table() {
    let p = pair();
    let (c, r) = read_u8s(&p, b"cp_fixed_table\0", 288 + 32);
    assert_eq!(c, r, "cp_fixed_table differs");
    // and it really is the DEFLATE fixed table
    assert_eq!(&c[..144], &[8u8; 144][..]);
    assert_eq!(&c[144..256], &[9u8; 112][..]);
    assert_eq!(&c[256..280], &[7u8; 24][..]);
    assert_eq!(&c[280..288], &[8u8; 8][..]);
    assert_eq!(&c[288..320], &[5u8; 32][..]);
}

// --- row 2 -----------------------------------------------------------------
#[test]
fn g02_permutation_order() {
    let p = pair();
    let (c, r) = read_u8s(&p, b"cp_permutation_order\0", 19);
    assert_eq!(c, r, "cp_permutation_order differs");
    assert_eq!(
        c,
        vec![16u8, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15]
    );
}

// --- row 3 -----------------------------------------------------------------
#[test]
fn g03_len_extra_bits() {
    let p = pair();
    let (c, r) = read_u8s(&p, b"cp_len_extra_bits\0", 29 + 2);
    assert_eq!(c, r, "cp_len_extra_bits differs");
}

// --- row 4 -----------------------------------------------------------------
#[test]
fn g04_len_base() {
    let p = pair();
    let (c, r) = read_u32s(&p, b"cp_len_base\0", 29 + 2);
    assert_eq!(c, r, "cp_len_base differs");
}

// --- row 5 -----------------------------------------------------------------
#[test]
fn g05_dist_extra_bits() {
    let p = pair();
    let (c, r) = read_u8s(&p, b"cp_dist_extra_bits\0", 30 + 2);
    assert_eq!(c, r, "cp_dist_extra_bits differs");
}

// --- row 6 -----------------------------------------------------------------
#[test]
fn g06_dist_base() {
    let p = pair();
    let (c, r) = read_u32s(&p, b"cp_dist_base\0", 30 + 2);
    assert_eq!(c, r, "cp_dist_base differs");
}

// --- row 7 -----------------------------------------------------------------
#[test]
fn g07_error_reason_initially_null() {
    let p = pair();
    assert_eq!(p.c.error_reason(), None, "C cp_error_reason not NULL at load");
    assert_eq!(p.rs.error_reason(), None, "Rust cp_error_reason not NULL at load");
}
