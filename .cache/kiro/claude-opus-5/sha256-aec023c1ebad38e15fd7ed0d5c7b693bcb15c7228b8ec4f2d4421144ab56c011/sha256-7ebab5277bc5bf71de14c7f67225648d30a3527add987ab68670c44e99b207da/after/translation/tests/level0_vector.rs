//! Level 0: the leaf vector/rotation helpers.
//!
//! Every function is invoked through `dlsym` on both shared objects and the
//! results are compared bit-for-bit.

#![allow(non_snake_case)]

mod common;

use common::*;

type FnVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
type FnVVf = unsafe extern "C" fn(c2v, c2v) -> f32;
type FnV = unsafe extern "C" fn(c2v) -> c2v;
type FnVf = unsafe extern "C" fn(c2v) -> f32;
type FnVsV = unsafe extern "C" fn(c2v, f32) -> c2v;

fn n() -> usize {
    common::scale(4000)
}

#[test]
fn c2V_matches() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn(f32, f32) -> c2v>("c2V");
    let mut rng = Rng::new(0x5eed_0001);
    for i in 0..n() {
        let (x, y) = (rng.coord(), rng.coord());
        let (cv, rv) = unsafe { (c(x, y), r(x, y)) };
        assert_bytes_eq(&cv, &rv, &format!("c2V #{i} ({x:?}, {y:?})"));
    }
}

/// Drives every `(c2v, c2v) -> c2v` helper with the same input stream.
#[test]
fn binary_vector_ops_match() {
    let l = libs();
    for name in ["c2Maxv", "c2Minv", "c2Sub", "c2Add"] {
        let (c, r) = l.pair::<FnVV>(name);
        let mut rng = Rng::new(0x5eed_0002);
        for i in 0..n() {
            let (a, b) = (rng.vec(), rng.vec());
            let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
            assert_bytes_eq(&cv, &rv, &format!("{name} #{i} a={a:?} b={b:?}"));
        }
    }
}

#[test]
fn binary_scalar_ops_match() {
    let l = libs();
    for name in ["c2Dot", "c2Det2"] {
        let (c, r) = l.pair::<FnVVf>(name);
        let mut rng = Rng::new(0x5eed_0003);
        for i in 0..n() {
            let (a, b) = (rng.vec(), rng.vec());
            let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
            assert_f32_eq(cv, rv, &format!("{name} #{i} a={a:?} b={b:?}"));
        }
    }
}

#[test]
fn unary_vector_ops_match() {
    let l = libs();
    for name in ["c2Neg", "c2Skew", "c2CCW90", "c2Norm"] {
        let (c, r) = l.pair::<FnV>(name);
        let mut rng = Rng::new(0x5eed_0004);
        for i in 0..n() {
            let a = rng.vec();
            let (cv, rv) = unsafe { (c(a), r(a)) };
            assert_bytes_eq(&cv, &rv, &format!("{name} #{i} a={a:?}"));
        }
    }
}

#[test]
fn c2Len_matches() {
    let l = libs();
    let (c, r) = l.pair::<FnVf>("c2Len");
    let mut rng = Rng::new(0x5eed_0005);
    for i in 0..n() {
        let a = rng.vec();
        let (cv, rv) = unsafe { (c(a), r(a)) };
        assert_f32_eq(cv, rv, &format!("c2Len #{i} a={a:?}"));
    }
}

#[test]
fn vector_scalar_ops_match() {
    let l = libs();
    for name in ["c2Mulvs", "c2Div"] {
        let (c, r) = l.pair::<FnVsV>(name);
        let mut rng = Rng::new(0x5eed_0006);
        for i in 0..n() {
            let a = rng.vec();
            let b = rng.coord();
            let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
            assert_bytes_eq(&cv, &rv, &format!("{name} #{i} a={a:?} b={b:?}"));
        }
    }
}

#[test]
fn c2Clampv_matches() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn(c2v, c2v, c2v) -> c2v>("c2Clampv");
    let mut rng = Rng::new(0x5eed_0007);
    for i in 0..n() {
        let (a, lo, hi) = (rng.vec(), rng.vec(), rng.vec());
        let (cv, rv) = unsafe { (c(a, lo, hi), r(a, lo, hi)) };
        assert_bytes_eq(&cv, &rv, &format!("c2Clampv #{i} a={a:?} lo={lo:?} hi={hi:?}"));
    }
}

#[test]
fn identities_match() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn() -> c2r>("c2RotIdentity");
    let (cv, rv) = unsafe { (c(), r()) };
    assert_bytes_eq(&cv, &rv, "c2RotIdentity");

    let (c, r) = l.pair::<unsafe extern "C" fn() -> c2x>("c2xIdentity");
    let (cv, rv) = unsafe { (c(), r()) };
    assert_bytes_eq(&cv, &rv, "c2xIdentity");
}

#[test]
fn rotation_ops_match() {
    let l = libs();
    for name in ["c2Mulrv", "c2MulrvT"] {
        let (c, r) = l.pair::<unsafe extern "C" fn(c2r, c2v) -> c2v>(name);
        let mut rng = Rng::new(0x5eed_0008);
        for i in 0..n() {
            let a = rng.rot();
            let b = rng.vec();
            let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
            assert_bytes_eq(&cv, &rv, &format!("{name} #{i} a={a:?} b={b:?}"));
        }
    }
}

#[test]
fn c2Mulxv_matches() {
    let l = libs();
    let (c, r) = l.pair::<unsafe extern "C" fn(c2x, c2v) -> c2v>("c2Mulxv");
    let mut rng = Rng::new(0x5eed_0009);
    for i in 0..n() {
        let a = rng.xform();
        let b = rng.vec();
        let (cv, rv) = unsafe { (c(a, b), r(a, b)) };
        assert_bytes_eq(&cv, &rv, &format!("c2Mulxv #{i} a={a:?} b={b:?}"));
    }
}

/// Hand-picked edge cases, kept separate from the fuzz streams so a regression
/// here is immediately readable.
#[test]
fn edge_case_scalars_match() {
    let l = libs();
    let edge = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1e-45, // smallest denormal
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];

    let (c_dot, r_dot) = l.pair::<FnVVf>("c2Dot");
    let (c_det, r_det) = l.pair::<FnVVf>("c2Det2");
    let (c_len, r_len) = l.pair::<FnVf>("c2Len");
    let (c_norm, r_norm) = l.pair::<FnV>("c2Norm");
    let (c_max, r_max) = l.pair::<FnVV>("c2Maxv");
    let (c_min, r_min) = l.pair::<FnVV>("c2Minv");
    let (c_mul, r_mul) = l.pair::<FnVsV>("c2Mulvs");
    let (c_div, r_div) = l.pair::<FnVsV>("c2Div");

    for &x in &edge {
        for &y in &edge {
            let a = c2v { x, y };
            let b = c2v { x: y, y: x };
            unsafe {
                assert_f32_eq(c_dot(a, b), r_dot(a, b), &format!("c2Dot {a:?} {b:?}"));
                assert_f32_eq(c_det(a, b), r_det(a, b), &format!("c2Det2 {a:?} {b:?}"));
                assert_f32_eq(c_len(a), r_len(a), &format!("c2Len {a:?}"));
                assert_bytes_eq(&c_norm(a), &r_norm(a), &format!("c2Norm {a:?}"));
                assert_bytes_eq(&c_max(a, b), &r_max(a, b), &format!("c2Maxv {a:?} {b:?}"));
                assert_bytes_eq(&c_min(a, b), &r_min(a, b), &format!("c2Minv {a:?} {b:?}"));
                assert_bytes_eq(&c_mul(a, y), &r_mul(a, y), &format!("c2Mulvs {a:?} {y:?}"));
                assert_bytes_eq(&c_div(a, y), &r_div(a, y), &format!("c2Div {a:?} {y:?}"));
            }
        }
    }
}
