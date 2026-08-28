//! Level 0: the vector helpers (`c2V` .. `c2MulmvT`).
//!
//! These are the leaves of the call graph in `c_src/src/lib.c`.
#![allow(non_snake_case)]

mod common;

use common::*;

type FnVV = extern "C" fn(C2v, C2v) -> C2v;
type FnVVf = extern "C" fn(C2v, C2v) -> f32;
type FnV = extern "C" fn(C2v) -> C2v;
type FnVf = extern "C" fn(C2v) -> f32;
type FnVsV = extern "C" fn(C2v, f32) -> C2v;

/// Drive a `c2v -> c2v` symbol over the same inputs in both libraries.
fn check_unary(name: &[u8], seed: u64) {
    let (c, r): (FnV, FnV) = syms(name);
    let mut rng = Rng::new(seed);
    for i in 0..iters(20_000) {
        let a = rng.vec();
        let (gc, gr) = (c(a), r(a));
        assert!(
            v_eq(gc, gr),
            "{} mismatch at iter {i}\n  input: {}\n  C:    {}\n  Rust: {}",
            String::from_utf8_lossy(&name[..name.len() - 1]),
            show_v(a),
            show_v(gc),
            show_v(gr)
        );
    }
}

/// Drive a `(c2v, c2v) -> c2v` symbol.
fn check_binary(name: &[u8], seed: u64) {
    let (c, r): (FnVV, FnVV) = syms(name);
    let mut rng = Rng::new(seed);
    for i in 0..iters(20_000) {
        let (a, b) = (rng.vec(), rng.vec());
        let (gc, gr) = (c(a, b), r(a, b));
        assert!(
            v_eq(gc, gr),
            "{} mismatch at iter {i}\n  inputs: {} {}\n  C:    {}\n  Rust: {}",
            String::from_utf8_lossy(&name[..name.len() - 1]),
            show_v(a),
            show_v(b),
            show_v(gc),
            show_v(gr)
        );
    }
}

#[test]
fn c2V_matches() {
    let (c, r): (extern "C" fn(f32, f32) -> C2v, _) = syms(b"c2V\0");
    let mut rng = Rng::new(1);
    for i in 0..iters(20_000) {
        let (x, y) = (rng.f32v(), rng.f32v());
        let (gc, gr) = (c(x, y), r(x, y));
        assert!(
            v_eq(gc, gr),
            "c2V mismatch at iter {i} for ({}, {}): C {} vs Rust {}",
            show_f32(x),
            show_f32(y),
            show_v(gc),
            show_v(gr)
        );
    }
}

#[test]
fn c2Dot_matches() {
    let (c, r): (FnVVf, FnVVf) = syms(b"c2Dot\0");
    let mut rng = Rng::new(2);
    for i in 0..iters(20_000) {
        let (a, b) = (rng.vec(), rng.vec());
        let (gc, gr) = (c(a, b), r(a, b));
        assert!(
            f32_eq(gc, gr),
            "c2Dot mismatch at iter {i} for {} {}: C {} vs Rust {}",
            show_v(a),
            show_v(b),
            show_f32(gc),
            show_f32(gr)
        );
    }
}

#[test]
fn c2Len_matches() {
    let (c, r): (FnVf, FnVf) = syms(b"c2Len\0");
    let mut rng = Rng::new(3);
    for i in 0..iters(20_000) {
        let a = rng.vec();
        let (gc, gr) = (c(a), r(a));
        assert!(
            f32_eq(gc, gr),
            "c2Len mismatch at iter {i} for {}: C {} vs Rust {}",
            show_v(a),
            show_f32(gc),
            show_f32(gr)
        );
    }
}

#[test]
fn c2Add_matches() {
    check_binary(b"c2Add\0", 4);
}

#[test]
fn c2Sub_matches() {
    check_binary(b"c2Sub\0", 5);
}

#[test]
fn c2Minv_matches() {
    check_binary(b"c2Minv\0", 6);
}

#[test]
fn c2Maxv_matches() {
    check_binary(b"c2Maxv\0", 7);
}

#[test]
fn c2Skew_matches() {
    check_unary(b"c2Skew\0", 8);
}

#[test]
fn c2Absv_matches() {
    check_unary(b"c2Absv\0", 9);
}

#[test]
fn c2CCW90_matches() {
    check_unary(b"c2CCW90\0", 10);
}

#[test]
fn c2Norm_matches() {
    check_unary(b"c2Norm\0", 11);
}

#[test]
fn c2Mulvs_matches() {
    let (c, r): (FnVsV, FnVsV) = syms(b"c2Mulvs\0");
    let mut rng = Rng::new(12);
    for i in 0..iters(20_000) {
        let (a, s) = (rng.vec(), rng.f32v());
        let (gc, gr) = (c(a, s), r(a, s));
        assert!(
            v_eq(gc, gr),
            "c2Mulvs mismatch at iter {i} for {} * {}: C {} vs Rust {}",
            show_v(a),
            show_f32(s),
            show_v(gc),
            show_v(gr)
        );
    }
}

#[test]
fn c2Div_matches() {
    let (c, r): (FnVsV, FnVsV) = syms(b"c2Div\0");
    let mut rng = Rng::new(13);
    for i in 0..iters(20_000) {
        let a = rng.vec();
        // include exact zero divisors: the C code does not guard against them
        let s = if i % 17 == 0 { 0.0 } else { rng.f32v() };
        let (gc, gr) = (c(a, s), r(a, s));
        assert!(
            v_eq(gc, gr),
            "c2Div mismatch at iter {i} for {} / {}: C {} vs Rust {}",
            show_v(a),
            show_f32(s),
            show_v(gc),
            show_v(gr)
        );
    }
}

#[test]
fn c2MulmvT_matches() {
    type F = extern "C" fn(C2m, C2v) -> C2v;
    let (c, r): (F, F) = syms(b"c2MulmvT\0");
    let mut rng = Rng::new(14);
    for i in 0..iters(20_000) {
        let m = C2m {
            x: rng.vec(),
            y: rng.vec(),
        };
        let b = rng.vec();
        let (gc, gr) = (c(m, b), r(m, b));
        assert!(
            v_eq(gc, gr),
            "c2MulmvT mismatch at iter {i} for m=({}, {}) b={}: C {} vs Rust {}",
            show_v(m.x),
            show_v(m.y),
            show_v(b),
            show_v(gc),
            show_v(gr)
        );
    }
}

/// Signed zero and infinity handling is where naive translations of the
/// ternary-operator macros (`c2Minv`, `c2Maxv`, `c2Absv`) tend to drift from
/// the C source, so pin those inputs down explicitly.
#[test]
fn edge_values_match() {
    let edges: [f32; 12] = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
        1.0e-45,
    ];

    let unary: [&[u8]; 4] = [b"c2Skew\0", b"c2Absv\0", b"c2CCW90\0", b"c2Norm\0"];
    for name in unary {
        let (c, r): (FnV, FnV) = syms(name);
        for &x in &edges {
            for &y in &edges {
                let a = C2v { x, y };
                let (gc, gr) = (c(a), r(a));
                assert!(
                    v_eq(gc, gr),
                    "{} mismatch for {}: C {} vs Rust {}",
                    String::from_utf8_lossy(&name[..name.len() - 1]),
                    show_v(a),
                    show_v(gc),
                    show_v(gr)
                );
            }
        }
    }

    let binary: [&[u8]; 4] = [b"c2Minv\0", b"c2Maxv\0", b"c2Add\0", b"c2Sub\0"];
    for name in binary {
        let (c, r): (FnVV, FnVV) = syms(name);
        for &ax in &edges {
            for &ay in &edges {
                for &bx in &edges {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: ay };
                    let (gc, gr) = (c(a, b), r(a, b));
                    assert!(
                        v_eq(gc, gr),
                        "{} mismatch for {} {}: C {} vs Rust {}",
                        String::from_utf8_lossy(&name[..name.len() - 1]),
                        show_v(a),
                        show_v(b),
                        show_v(gc),
                        show_v(gr)
                    );
                }
            }
        }
    }

    let (dot_c, dot_r): (FnVVf, FnVVf) = syms(b"c2Dot\0");
    let (len_c, len_r): (FnVf, FnVf) = syms(b"c2Len\0");
    for &ax in &edges {
        for &ay in &edges {
            let a = C2v { x: ax, y: ay };
            assert!(
                f32_eq(len_c(a), len_r(a)),
                "c2Len mismatch for {}: C {} vs Rust {}",
                show_v(a),
                show_f32(len_c(a)),
                show_f32(len_r(a))
            );
            for &bx in &edges {
                let b = C2v { x: bx, y: ax };
                assert!(
                    f32_eq(dot_c(a, b), dot_r(a, b)),
                    "c2Dot mismatch for {} {}: C {} vs Rust {}",
                    show_v(a),
                    show_v(b),
                    show_f32(dot_c(a, b)),
                    show_f32(dot_r(a, b))
                );
            }
        }
    }

    let (div_c, div_r): (FnVsV, FnVsV) = syms(b"c2Div\0");
    let (mul_c, mul_r): (FnVsV, FnVsV) = syms(b"c2Mulvs\0");
    for &ax in &edges {
        for &ay in &edges {
            for &s in &edges {
                let a = C2v { x: ax, y: ay };
                assert!(
                    v_eq(div_c(a, s), div_r(a, s)),
                    "c2Div mismatch for {} / {}: C {} vs Rust {}",
                    show_v(a),
                    show_f32(s),
                    show_v(div_c(a, s)),
                    show_v(div_r(a, s))
                );
                assert!(
                    v_eq(mul_c(a, s), mul_r(a, s)),
                    "c2Mulvs mismatch for {} * {}: C {} vs Rust {}",
                    show_v(a),
                    show_f32(s),
                    show_v(mul_c(a, s)),
                    show_v(mul_r(a, s))
                );
            }
        }
    }
}
