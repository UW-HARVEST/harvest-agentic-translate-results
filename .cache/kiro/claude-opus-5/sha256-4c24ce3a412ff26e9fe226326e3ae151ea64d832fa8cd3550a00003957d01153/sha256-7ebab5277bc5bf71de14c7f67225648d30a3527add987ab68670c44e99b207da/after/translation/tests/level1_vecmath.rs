//! Level 1: leaf-level vector / rotation math.
//!
//! These are the bottom of the call hierarchy - every higher level function is
//! built out of them, so they are verified first and most aggressively
//! (including infinities, NaNs, signed zeros and denormals).

#![allow(non_snake_case)]

mod common;

use common::*;

const ITERS: u32 = 20_000;

#[test]
fn c2V_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(1);
    for _ in 0..scaled(ITERS) {
        let (x, y) = (g.f32_nasty(), g.f32_nasty());
        unsafe {
            eq_v((c.c2V)(x, y), (r.c2V)(x, y), "c2V");
        }
    }
}

#[test]
fn c2Mulvs_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(2);
    for _ in 0..scaled(ITERS) {
        let a = g.v_nasty();
        let b = g.f32_nasty();
        unsafe {
            eq_v((c.c2Mulvs)(a, b), (r.c2Mulvs)(a, b), "c2Mulvs");
        }
    }
}

#[test]
fn c2Div_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(3);
    for _ in 0..scaled(ITERS) {
        let a = g.v_nasty();
        let b = g.f32_nasty();
        unsafe {
            eq_v((c.c2Div)(a, b), (r.c2Div)(a, b), "c2Div");
        }
    }
}

#[test]
fn c2Maxv_c2Minv_match() {
    let (c, r) = apis();
    let mut g = Rng::new(4);
    for _ in 0..scaled(ITERS) {
        let (a, b) = (g.v_nasty(), g.v_nasty());
        unsafe {
            eq_v((c.c2Maxv)(a, b), (r.c2Maxv)(a, b), "c2Maxv");
            eq_v((c.c2Minv)(a, b), (r.c2Minv)(a, b), "c2Minv");
        }
    }
    // Explicit signed-zero / NaN ordering cases: the C code uses `>` and `<`
    // ternaries, not fmaxf/fminf, so the tie and NaN behaviour is specific.
    let cases = [
        (0.0f32, -0.0f32),
        (-0.0f32, 0.0f32),
        (f32::NAN, 1.0),
        (1.0, f32::NAN),
        (f32::NAN, f32::NAN),
        (f32::INFINITY, f32::NEG_INFINITY),
    ];
    for &(x, y) in cases.iter() {
        let a = c2v { x, y: x };
        let b = c2v { x: y, y };
        unsafe {
            eq_v((c.c2Maxv)(a, b), (r.c2Maxv)(a, b), "c2Maxv/edge");
            eq_v((c.c2Minv)(a, b), (r.c2Minv)(a, b), "c2Minv/edge");
        }
    }
}

#[test]
fn c2Clampv_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(5);
    for _ in 0..scaled(ITERS) {
        let (a, lo, hi) = (g.v_nasty(), g.v_nasty(), g.v_nasty());
        unsafe {
            eq_v((c.c2Clampv)(a, lo, hi), (r.c2Clampv)(a, lo, hi), "c2Clampv");
        }
    }
    // Inverted bounds (lo > hi) must behave the same as the C ternary chain.
    let mut g = Rng::new(55);
    for _ in 0..scaled(ITERS) {
        let a = g.v(80.0);
        let lo = g.v(40.0);
        let hi = c2v {
            x: lo.x - 10.0,
            y: lo.y - 10.0,
        };
        unsafe {
            eq_v(
                (c.c2Clampv)(a, lo, hi),
                (r.c2Clampv)(a, lo, hi),
                "c2Clampv/inverted",
            );
        }
    }
}

#[test]
fn c2Sub_c2Add_match() {
    let (c, r) = apis();
    let mut g = Rng::new(6);
    for _ in 0..scaled(ITERS) {
        let (a, b) = (g.v_nasty(), g.v_nasty());
        unsafe {
            eq_v((c.c2Sub)(a, b), (r.c2Sub)(a, b), "c2Sub");
            eq_v((c.c2Add)(a, b), (r.c2Add)(a, b), "c2Add");
        }
    }
}

#[test]
fn c2Dot_c2Det2_match() {
    let (c, r) = apis();
    let mut g = Rng::new(7);
    for _ in 0..scaled(ITERS) {
        let (a, b) = (g.v_nasty(), g.v_nasty());
        unsafe {
            eq_f32((c.c2Dot)(a, b), (r.c2Dot)(a, b), "c2Dot");
            eq_f32((c.c2Det2)(a, b), (r.c2Det2)(a, b), "c2Det2");
        }
    }
    // Magnitudes chosen to exercise rounding / cancellation in `x*x + y*y`.
    let mut g = Rng::new(77);
    for _ in 0..scaled(ITERS) {
        let a = c2v {
            x: g.f32_range(1.0e18),
            y: g.f32_range(1.0e-18),
        };
        let b = c2v {
            x: g.f32_range(1.0e-18),
            y: g.f32_range(1.0e18),
        };
        unsafe {
            eq_f32((c.c2Dot)(a, b), (r.c2Dot)(a, b), "c2Dot/mixed");
            eq_f32((c.c2Det2)(a, b), (r.c2Det2)(a, b), "c2Det2/mixed");
        }
    }
}

#[test]
fn c2Len_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(8);
    for _ in 0..scaled(ITERS) {
        let a = g.v_nasty();
        unsafe {
            eq_f32((c.c2Len)(a), (r.c2Len)(a), "c2Len");
        }
    }
}

#[test]
fn c2Norm_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(9);
    for _ in 0..scaled(ITERS) {
        let a = g.v_nasty();
        unsafe {
            eq_v((c.c2Norm)(a), (r.c2Norm)(a), "c2Norm");
        }
    }
    // Zero vector -> 1/0 == inf -> inf*0 == NaN; must match exactly.
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
    ] {
        unsafe {
            eq_v((c.c2Norm)(a), (r.c2Norm)(a), "c2Norm/zero");
        }
    }
}

#[test]
fn c2Neg_c2Skew_c2CCW90_match() {
    let (c, r) = apis();
    let mut g = Rng::new(10);
    for _ in 0..scaled(ITERS) {
        let a = g.v_nasty();
        unsafe {
            eq_v((c.c2Neg)(a), (r.c2Neg)(a), "c2Neg");
            eq_v((c.c2Skew)(a), (r.c2Skew)(a), "c2Skew");
            eq_v((c.c2CCW90)(a), (r.c2CCW90)(a), "c2CCW90");
        }
    }
}

#[test]
fn identities_match() {
    let (c, r) = apis();
    unsafe {
        eq_r((c.c2RotIdentity)(), (r.c2RotIdentity)(), "c2RotIdentity");
        eq_x((c.c2xIdentity)(), (r.c2xIdentity)(), "c2xIdentity");
    }
}

#[test]
fn c2Mulrv_c2MulrvT_match() {
    let (c, r) = apis();
    let mut g = Rng::new(11);
    for _ in 0..scaled(ITERS) {
        let rot = c2r {
            c: g.f32_nasty(),
            s: g.f32_nasty(),
        };
        let v = g.v_nasty();
        unsafe {
            eq_v((c.c2Mulrv)(rot, v), (r.c2Mulrv)(rot, v), "c2Mulrv");
            eq_v((c.c2MulrvT)(rot, v), (r.c2MulrvT)(rot, v), "c2MulrvT");
        }
    }
    let mut g = Rng::new(111);
    for _ in 0..scaled(ITERS) {
        let rot = g.rot();
        let v = g.v(100.0);
        unsafe {
            eq_v((c.c2Mulrv)(rot, v), (r.c2Mulrv)(rot, v), "c2Mulrv/real");
            eq_v((c.c2MulrvT)(rot, v), (r.c2MulrvT)(rot, v), "c2MulrvT/real");
        }
    }
}

#[test]
fn c2Mulxv_matches() {
    let (c, r) = apis();
    let mut g = Rng::new(12);
    for _ in 0..scaled(ITERS) {
        let x = c2x {
            p: g.v_nasty(),
            r: c2r {
                c: g.f32_nasty(),
                s: g.f32_nasty(),
            },
        };
        let v = g.v_nasty();
        unsafe {
            eq_v((c.c2Mulxv)(x, v), (r.c2Mulxv)(x, v), "c2Mulxv");
        }
    }
    let mut g = Rng::new(122);
    for _ in 0..scaled(ITERS) {
        let x = g.xform();
        let v = g.v(100.0);
        unsafe {
            eq_v((c.c2Mulxv)(x, v), (r.c2Mulxv)(x, v), "c2Mulxv/real");
        }
    }
}
