//! Level 1: the leaf arithmetic operations.
//!
//! `multiply_with_static`, `add_with_static`, `xor_operation` and
//! `shift_with_static` take no pointers and print nothing, so they can be
//! compared exhaustively over a wide input grid.

mod common;

use common::*;
use std::ffi::c_int;

fn check_binop(name: &str, pairs: impl Iterator<Item = (c_int, c_int)>) {
    let libs = impls();
    let c: libloading::Symbol<FnBinop> = libs.sym(Which::C, name);
    let r: libloading::Symbol<FnBinop> = libs.sym(Which::Rust, name);

    for (a, b) in pairs {
        let (cv, cout) = capture_stdout(|| unsafe { c(a, b) });
        let (rv, rout) = capture_stdout(|| unsafe { r(a, b) });
        assert_eq!(cv, rv, "{name}({a}, {b}): C returned {cv}, Rust returned {rv}");
        assert_eq!(
            cout,
            rout,
            "{name}({a}, {b}) stdout differs: C={:?} Rust={:?}",
            show(&cout),
            show(&rout)
        );
        assert!(
            cout.is_empty(),
            "{name} unexpectedly printed: {}",
            show(&cout)
        );
    }
}

fn grid() -> Vec<(c_int, c_int)> {
    let v = sample_ints();
    let mut out = Vec::with_capacity(v.len() * v.len());
    for &a in &v {
        for &b in &v {
            out.push((a, b));
        }
    }
    let mut rng = Rng::new(0xC0FF_EE12_3456_789A);
    for _ in 0..20_000 {
        out.push((rng.next_i32(), rng.next_i32()));
    }
    out
}

fn multiply_with_static_matches() {
    check_binop("multiply_with_static", grid().into_iter());
}

fn add_with_static_matches() {
    check_binop("add_with_static", grid().into_iter());
}

fn xor_operation_matches() {
    check_binop("xor_operation", grid().into_iter());
}

fn shift_with_static_matches() {
    check_binop("shift_with_static", grid().into_iter());
}

/// Shift is the most delicate of the four: the C source left-shifts a possibly
/// negative `int` and right-shifts a possibly negative `int`. Hammer the sign
/// boundaries specifically.
fn shift_with_static_sign_boundaries() {
    let mut pairs = Vec::new();
    for a in [-8, -4, -3, -2, -1, 0, 1, 2, 3, 4, 8] {
        for b in [-8, -4, -3, -2, -1, 0, 1, 2, 3, 4, 8] {
            pairs.push((a, b));
        }
    }
    for base in [c_int::MIN, c_int::MIN + 1, c_int::MAX, c_int::MAX - 1, 0] {
        for d in -4..=4 {
            pairs.push((base.wrapping_add(d), base.wrapping_sub(d)));
            pairs.push((base.wrapping_sub(d), base.wrapping_add(d)));
        }
    }
    // every single-bit pattern, positive and negated
    for i in 0..32u32 {
        let bit = 1i32.wrapping_shl(i);
        for j in 0..32u32 {
            let other = 1i32.wrapping_shl(j);
            pairs.push((bit, other));
            pairs.push((bit.wrapping_neg(), other));
            pairs.push((bit, other.wrapping_neg()));
            pairs.push((bit.wrapping_neg(), other.wrapping_neg()));
        }
    }
    check_binop("shift_with_static", pairs.into_iter());
}

fn main() {
    let mut r = Runner::new();
    r.case("multiply_with_static_matches", multiply_with_static_matches);
    r.case("add_with_static_matches", add_with_static_matches);
    r.case("xor_operation_matches", xor_operation_matches);
    r.case("shift_with_static_matches", shift_with_static_matches);
    r.case("shift_with_static_sign_boundaries", shift_with_static_sign_boundaries);
    r.finish();
}
