//! Level 1: leaf functions with no dependencies.
//! `is_valid_operation`, `get_operation_priority`, and the five math ops.
mod common;

use common::both;
use std::ffi::{c_char, c_int};

#[test]
fn is_valid_operation_all_chars() {
    let b = both();
    // Every possible value of a (signed) C char.
    for v in i8::MIN..=i8::MAX {
        let c_in = v as c_char;
        let (cr, rr) = unsafe {
            (
                (b.c.is_valid_operation)(c_in),
                (b.rust.is_valid_operation)(c_in),
            )
        };
        assert_eq!(cr, rr, "is_valid_operation({v})");
    }
}

#[test]
fn is_valid_operation_raw_byte_return() {
    // The C function returns `_Bool`; make sure the Rust export produces the
    // same single byte (0 or 1) and not some other non-zero encoding.
    let b = both();
    type RawFn = unsafe extern "C" fn(c_char) -> u8;
    let c_raw: RawFn = unsafe { std::mem::transmute(b.c.is_valid_operation) };
    let r_raw: RawFn = unsafe { std::mem::transmute(b.rust.is_valid_operation) };
    for v in i8::MIN..=i8::MAX {
        let (cv, rv) = unsafe { (c_raw(v as c_char), r_raw(v as c_char)) };
        assert_eq!(cv, rv, "raw _Bool byte for char {v}");
    }
}

/// `op * 10` is signed multiplication; stay inside the non-overflowing range so
/// the comparison is against C's *defined* behaviour.
fn priority_inputs() -> Vec<c_int> {
    let mut v = vec![
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        -1,
        -2,
        -5,
        -6,
        100,
        -100,
        12345,
        -12345,
        214_748_364,  // 10 * this == i32::MAX - 7
        -214_748_364,
    ];
    for i in -50..=50 {
        v.push(i);
    }
    v
}

#[test]
fn get_operation_priority_matches() {
    let b = both();
    for op in priority_inputs() {
        let (cr, rr) = unsafe {
            (
                (b.c.get_operation_priority)(op),
                (b.rust.get_operation_priority)(op),
            )
        };
        assert_eq!(cr, rr, "get_operation_priority({op})");
    }
}

/// Operand pairs for add/subtract/multiply that never overflow a signed int
/// (signed overflow is UB in C, so it is not a defined behaviour to match).
fn safe_pairs() -> Vec<(c_int, c_int)> {
    let vals: Vec<c_int> = vec![
        0, 1, -1, 2, -2, 3, 7, -7, 10, -10, 100, -100, 255, -255, 1000, -1000, 30_000, -30_000,
        46_340, -46_340,
    ];
    let mut out = Vec::new();
    for &a in &vals {
        for &b in &vals {
            out.push((a, b));
        }
    }
    // Additive edges that do not overflow.
    out.extend([
        (i32::MAX, 0),
        (0, i32::MAX),
        (i32::MIN, 0),
        (i32::MAX - 1, 1),
        (i32::MIN + 1, -1),
        (i32::MAX, 1_i32.wrapping_neg()),
    ]);
    out
}

#[test]
fn add_operation_matches() {
    let b = both();
    for (a, x) in safe_pairs() {
        // skip pairs whose sum would overflow
        if a.checked_add(x).is_none() {
            continue;
        }
        for unused in [0, 1, -1, 12345] {
            let (cr, rr) = unsafe {
                (
                    (b.c.add_operation)(a, x, unused),
                    (b.rust.add_operation)(a, x, unused),
                )
            };
            assert_eq!(cr, rr, "add_operation({a},{x},{unused})");
        }
    }
}

#[test]
fn subtract_operation_matches() {
    let b = both();
    for (a, x) in safe_pairs() {
        if a.checked_sub(x).is_none() {
            continue;
        }
        for unused in [0, -999, 7] {
            let (cr, rr) = unsafe {
                (
                    (b.c.subtract_operation)(a, x, unused),
                    (b.rust.subtract_operation)(a, x, unused),
                )
            };
            assert_eq!(cr, rr, "subtract_operation({a},{x},{unused})");
        }
    }
}

#[test]
fn multiply_operation_matches() {
    let b = both();
    for (a, x) in safe_pairs() {
        if a.checked_mul(x).is_none() {
            continue;
        }
        for unused in [0, 42] {
            let (cr, rr) = unsafe {
                (
                    (b.c.multiply_operation)(a, x, unused),
                    (b.rust.multiply_operation)(a, x, unused),
                )
            };
            assert_eq!(cr, rr, "multiply_operation({a},{x},{unused})");
        }
    }
}

/// Division/remainder pairs, including the `b == 0` guard the C code has.
/// `INT_MIN / -1` is excluded: it traps (SIGFPE) in C, so there is no defined
/// result to compare against.
fn div_pairs() -> Vec<(c_int, c_int)> {
    let vals: Vec<c_int> = vec![
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        5,
        -5,
        7,
        -7,
        10,
        -10,
        99,
        -99,
        100,
        -100,
        128,
        -128,
        12345,
        -12345,
        i32::MAX,
        i32::MIN,
    ];
    let mut out = Vec::new();
    for &a in &vals {
        for &b in &vals {
            if a == i32::MIN && b == -1 {
                continue; // UB / SIGFPE in C
            }
            out.push((a, b));
        }
    }
    out
}

#[test]
fn divide_operation_matches() {
    let b = both();
    for (a, x) in div_pairs() {
        for unused in [0, 5] {
            let (cr, rr) = unsafe {
                (
                    (b.c.divide_operation)(a, x, unused),
                    (b.rust.divide_operation)(a, x, unused),
                )
            };
            assert_eq!(cr, rr, "divide_operation({a},{x},{unused})");
        }
    }
}

#[test]
fn modulo_operation_matches() {
    let b = both();
    for (a, x) in div_pairs() {
        for unused in [0, -3] {
            let (cr, rr) = unsafe {
                (
                    (b.c.modulo_operation)(a, x, unused),
                    (b.rust.modulo_operation)(a, x, unused),
                )
            };
            assert_eq!(cr, rr, "modulo_operation({a},{x},{unused})");
        }
    }
}

#[test]
fn divide_and_modulo_zero_divisor_guard() {
    let b = both();
    for a in [i32::MIN, -7, -1, 0, 1, 7, i32::MAX] {
        unsafe {
            assert_eq!((b.c.divide_operation)(a, 0, 0), 0);
            assert_eq!((b.rust.divide_operation)(a, 0, 0), 0);
            assert_eq!((b.c.modulo_operation)(a, 0, 0), 0);
            assert_eq!((b.rust.modulo_operation)(a, 0, 0), 0);
        }
    }
}
