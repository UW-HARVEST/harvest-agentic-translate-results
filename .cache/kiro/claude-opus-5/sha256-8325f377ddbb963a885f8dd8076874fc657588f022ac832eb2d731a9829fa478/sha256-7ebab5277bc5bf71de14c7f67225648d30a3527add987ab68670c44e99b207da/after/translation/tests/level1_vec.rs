//! Level 1: the leaf scalar / vector helpers.

#![allow(non_snake_case)]

mod harness;
use harness::*;

const N: u32 = 20_000;

type FnVff = unsafe extern "C" fn(f32, f32) -> V;
type FnVv = unsafe extern "C" fn(V) -> V;
type FnVvf = unsafe extern "C" fn(V, f32) -> V;
type FnVvv = unsafe extern "C" fn(V, V) -> V;
type FnVvvv = unsafe extern "C" fn(V, V, V) -> V;
type FnFv = unsafe extern "C" fn(V) -> f32;
type FnFvv = unsafe extern "C" fn(V, V) -> f32;
type FnR = unsafe extern "C" fn() -> R;
type FnX = unsafe extern "C" fn() -> X;
type FnVrv = unsafe extern "C" fn(R, V) -> V;
type FnVxv = unsafe extern "C" fn(X, V) -> V;

#[test]
fn c2V_matches() {
    let (c, r) = pair::<FnVff>("c2V");
    let mut rng = Rng::new(1);
    for _ in 0..volume(N) {
        let (x, y) = (rng.float(), rng.float());
        let a = unsafe { c(x, y) };
        let b = unsafe { r(x, y) };
        assert_v("c2V", &(x, y), a, b);
    }
}

#[test]
fn unary_vector_fns_match() {
    let mut rng = Rng::new(2);
    let names = ["c2Neg", "c2Skew", "c2CCW90", "c2Norm"];
    let fns: Vec<_> = names.iter().map(|n| (n, pair::<FnVv>(n))).collect();
    for _ in 0..volume(N) {
        let a = rng.v();
        for (name, (c, r)) in &fns {
            let x = unsafe { c(a) };
            let y = unsafe { r(a) };
            assert_v(name, &a, x, y);
        }
    }
}

#[test]
fn binary_vector_fns_match() {
    let mut rng = Rng::new(3);
    let names = ["c2Sub", "c2Add", "c2Maxv", "c2Minv"];
    let fns: Vec<_> = names.iter().map(|n| (n, pair::<FnVvv>(n))).collect();
    for _ in 0..volume(N) {
        let (a, b) = (rng.v(), rng.v());
        for (name, (c, r)) in &fns {
            let x = unsafe { c(a, b) };
            let y = unsafe { r(a, b) };
            assert_v(name, &(a, b), x, y);
        }
    }
}

#[test]
fn c2Clampv_matches() {
    let (c, r) = pair::<FnVvvv>("c2Clampv");
    let mut rng = Rng::new(4);
    for _ in 0..volume(N) {
        let (a, lo, hi) = (rng.v(), rng.v(), rng.v());
        let x = unsafe { c(a, lo, hi) };
        let y = unsafe { r(a, lo, hi) };
        assert_v("c2Clampv", &(a, lo, hi), x, y);
    }
}

#[test]
fn vector_scalar_fns_match() {
    let mut rng = Rng::new(5);
    let names = ["c2Mulvs", "c2Div"];
    let fns: Vec<_> = names.iter().map(|n| (n, pair::<FnVvf>(n))).collect();
    for _ in 0..volume(N) {
        let a = rng.v();
        let s = rng.float();
        for (name, (c, r)) in &fns {
            let x = unsafe { c(a, s) };
            let y = unsafe { r(a, s) };
            assert_v(name, &(a, s), x, y);
        }
    }
}

#[test]
fn scalar_returning_fns_match() {
    let mut rng = Rng::new(6);
    let names = ["c2Dot", "c2Det2"];
    let fns: Vec<_> = names.iter().map(|n| (n, pair::<FnFvv>(n))).collect();
    let (clen, rlen) = pair::<FnFv>("c2Len");
    for _ in 0..volume(N) {
        let (a, b) = (rng.v(), rng.v());
        for (name, (c, r)) in &fns {
            let x = unsafe { c(a, b) };
            let y = unsafe { r(a, b) };
            assert_f(name, &(a, b), x, y);
        }
        let x = unsafe { clen(a) };
        let y = unsafe { rlen(a) };
        assert_f("c2Len", &a, x, y);
    }
}

#[test]
fn identity_fns_match() {
    let (c, r) = pair::<FnR>("c2RotIdentity");
    assert_r("c2RotIdentity", &(), unsafe { c() }, unsafe { r() });
    let (c, r) = pair::<FnX>("c2xIdentity");
    assert_x("c2xIdentity", &(), unsafe { c() }, unsafe { r() });
}

#[test]
fn rotation_fns_match() {
    let mut rng = Rng::new(7);
    let names = ["c2Mulrv", "c2MulrvT"];
    let fns: Vec<_> = names.iter().map(|n| (n, pair::<FnVrv>(n))).collect();
    let (cx, rx) = pair::<FnVxv>("c2Mulxv");
    for _ in 0..volume(N) {
        let rot = R {
            c: rng.float(),
            s: rng.float(),
        };
        let b = rng.v();
        for (name, (c, r)) in &fns {
            let x = unsafe { c(rot, b) };
            let y = unsafe { r(rot, b) };
            assert_v(name, &(rot, b), x, y);
        }
        let xf = X { p: rng.v(), r: rot };
        let x = unsafe { cx(xf, b) };
        let y = unsafe { rx(xf, b) };
        assert_v("c2Mulxv", &(xf, b), x, y);
    }
}

/// Rotations that are actually normalised (cos/sin of an angle) - the regime
/// the algorithm is used in, where cancellation behaviour matters.
#[test]
fn rotation_fns_match_for_real_angles() {
    let mut rng = Rng::new(8);
    let (cm, rm) = pair::<FnVrv>("c2Mulrv");
    let (ct, rt) = pair::<FnVrv>("c2MulrvT");
    for _ in 0..volume(N) {
        let angle = rng.unit() * std::f32::consts::PI;
        let rot = R {
            c: angle.cos(),
            s: angle.sin(),
        };
        let b = rng.v_finite();
        assert_v("c2Mulrv", &(rot, b), unsafe { cm(rot, b) }, unsafe {
            rm(rot, b)
        });
        assert_v("c2MulrvT", &(rot, b), unsafe { ct(rot, b) }, unsafe {
            rt(rot, b)
        });
    }
}
