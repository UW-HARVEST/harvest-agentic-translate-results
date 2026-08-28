//! Level 1: the scalar/vector primitives (`c2V`, `c2Mulvs`, `c2Sub`, `c2Dot`,
//! `c2Minv`, `c2Maxv`, `c2Clampv`) compared bit-for-bit against the C `.so`.
//!
//! Contexts are formatted lazily (only on mismatch) so the exhaustive sweeps
//! stay fast even in a debug build.

#![allow(non_snake_case)]

mod common;

use common::*;

/// Exhaustive-sweep guard for a `c2v` result.
macro_rules! check_v {
    ($what:expr, $c:expr, $rs:expr, $($ctx:tt)*) => {{
        let (c, rs) = ($c, $rs);
        if !eqv(c, rs) {
            assert_v_bits($what, &format!($($ctx)*), c, rs);
        }
    }};
}

/// Exhaustive-sweep guard for an `f32` result.
macro_rules! check_f {
    ($what:expr, $c:expr, $rs:expr, $($ctx:tt)*) => {{
        let (c, rs) = ($c, $rs);
        if !eqf(c, rs) {
            assert_f32_bits($what, &format!($($ctx)*), c, rs);
        }
    }};
}

#[test]
fn c2v_constructor() {
    let p = pair();
    for &x in SCALARS {
        for &y in SCALARS {
            let c = unsafe { (p.c.c2V)(x, y) };
            let rs = unsafe { (p.rs.c2V)(x, y) };
            check_v!("c2V", c, rs, "c2V({x:?}, {y:?})");
            // The constructor must be a pure move of its arguments.
            check_f!("c2V", x, c.x, "c2V({x:?}, {y:?}) passthrough .x");
            check_f!("c2V", y, c.y, "c2V({x:?}, {y:?}) passthrough .y");
        }
    }
}

#[test]
fn c2mulvs_exhaustive_scalars() {
    let p = pair();
    for &x in SCALARS {
        for &y in SCALARS {
            for &s in SCALARS {
                let a = c2v { x, y };
                check_v!(
                    "c2Mulvs",
                    unsafe { (p.c.c2Mulvs)(a, s) },
                    unsafe { (p.rs.c2Mulvs)(a, s) },
                    "c2Mulvs(({x:?}, {y:?}), {s:?})"
                );
            }
        }
    }
}

#[test]
fn c2sub_exhaustive_scalars() {
    let p = pair();
    for &ax in SCALARS {
        for &bx in SCALARS {
            for &ay in SCALARS {
                for &by in SCALARS {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: by };
                    check_v!(
                        "c2Sub",
                        unsafe { (p.c.c2Sub)(a, b) },
                        unsafe { (p.rs.c2Sub)(a, b) },
                        "c2Sub(({ax:?}, {ay:?}), ({bx:?}, {by:?}))"
                    );
                }
            }
        }
    }
}

#[test]
fn c2minv_c2maxv_exhaustive_scalars() {
    let p = pair();
    // The C uses raw ternaries, so ordering with NaN operands is asymmetric:
    // both argument positions must be covered for every scalar pair.
    for &ax in SCALARS {
        for &bx in SCALARS {
            for &ay in SCALARS {
                for &by in SCALARS {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: by };
                    check_v!(
                        "c2Minv",
                        unsafe { (p.c.c2Minv)(a, b) },
                        unsafe { (p.rs.c2Minv)(a, b) },
                        "c2Minv(({ax:?}, {ay:?}), ({bx:?}, {by:?}))"
                    );
                    check_v!(
                        "c2Maxv",
                        unsafe { (p.c.c2Maxv)(a, b) },
                        unsafe { (p.rs.c2Maxv)(a, b) },
                        "c2Maxv(({ax:?}, {ay:?}), ({bx:?}, {by:?}))"
                    );
                }
            }
        }
    }
}

#[test]
fn c2dot_exhaustive_scalars() {
    let p = pair();
    for &ax in SCALARS {
        for &bx in SCALARS {
            for &ay in SCALARS {
                for &by in SCALARS {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: by };
                    check_f!(
                        "c2Dot",
                        unsafe { (p.c.c2Dot)(a, b) },
                        unsafe { (p.rs.c2Dot)(a, b) },
                        "c2Dot(({ax:?}, {ay:?}), ({bx:?}, {by:?}))"
                    );
                }
            }
        }
    }
}

#[test]
fn c2clampv_exhaustive_scalars() {
    let p = pair();
    // A full 36^6 sweep is impractical; the clamped value walks the whole
    // corpus while the bounds walk a representative set (including inverted
    // bounds, signed zeros, infinities and NaN).
    const BOUNDS: &[f32] = &[
        -40.0,
        -15.0,
        0.0,
        -0.0,
        15.0,
        40.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    for &ax in SCALARS {
        for &ay in SCALARS {
            for &lx in BOUNDS {
                for &hx in BOUNDS {
                    for &ly in BOUNDS {
                        for &hy in BOUNDS {
                            let a = c2v { x: ax, y: ay };
                            let lo = c2v { x: lx, y: ly };
                            let hi = c2v { x: hx, y: hy };
                            check_v!(
                                "c2Clampv",
                                unsafe { (p.c.c2Clampv)(a, lo, hi) },
                                unsafe { (p.rs.c2Clampv)(a, lo, hi) },
                                "c2Clampv(({ax:?}, {ay:?}), ({lx:?}, {ly:?}), ({hx:?}, {hy:?}))"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn primitives_random_bit_patterns() {
    let p = pair();
    let mut rng = Rng::new();
    for i in 0..1_000_000u32 {
        let a = rng.any_v();
        let b = rng.any_v();
        let s = rng.any_f32();
        let lo = rng.any_v();
        let hi = rng.any_v();

        check_v!("c2Mulvs", unsafe { (p.c.c2Mulvs)(a, s) }, unsafe {
            (p.rs.c2Mulvs)(a, s)
        }, "iter {i}: a={a:?} s={s:?}");
        check_v!("c2Sub", unsafe { (p.c.c2Sub)(a, b) }, unsafe {
            (p.rs.c2Sub)(a, b)
        }, "iter {i}: a={a:?} b={b:?}");
        check_v!("c2Minv", unsafe { (p.c.c2Minv)(a, b) }, unsafe {
            (p.rs.c2Minv)(a, b)
        }, "iter {i}: a={a:?} b={b:?}");
        check_v!("c2Maxv", unsafe { (p.c.c2Maxv)(a, b) }, unsafe {
            (p.rs.c2Maxv)(a, b)
        }, "iter {i}: a={a:?} b={b:?}");
        check_f!("c2Dot", unsafe { (p.c.c2Dot)(a, b) }, unsafe {
            (p.rs.c2Dot)(a, b)
        }, "iter {i}: a={a:?} b={b:?}");
        check_v!("c2Clampv", unsafe { (p.c.c2Clampv)(a, lo, hi) }, unsafe {
            (p.rs.c2Clampv)(a, lo, hi)
        }, "iter {i}: a={a:?} lo={lo:?} hi={hi:?}");
        check_v!("c2V", unsafe { (p.c.c2V)(s, a.x) }, unsafe {
            (p.rs.c2V)(s, a.x)
        }, "iter {i}: s={s:?} a.x={:?}", a.x);
    }
}

#[test]
fn primitives_random_finite() {
    let p = pair();
    let mut rng = Rng::new();
    for i in 0..1_000_000u32 {
        let a = rng.v(200.0);
        let b = rng.v(200.0);
        let s = rng.range(50.0);
        let lo = rng.v(80.0);
        let hi = rng.v(80.0);

        check_v!("c2Mulvs", unsafe { (p.c.c2Mulvs)(a, s) }, unsafe {
            (p.rs.c2Mulvs)(a, s)
        }, "iter {i}: a={a:?} s={s:?}");
        check_v!("c2Sub", unsafe { (p.c.c2Sub)(a, b) }, unsafe {
            (p.rs.c2Sub)(a, b)
        }, "iter {i}: a={a:?} b={b:?}");
        check_f!("c2Dot", unsafe { (p.c.c2Dot)(a, b) }, unsafe {
            (p.rs.c2Dot)(a, b)
        }, "iter {i}: a={a:?} b={b:?}");
        check_v!("c2Clampv", unsafe { (p.c.c2Clampv)(a, lo, hi) }, unsafe {
            (p.rs.c2Clampv)(a, lo, hi)
        }, "iter {i}: a={a:?} lo={lo:?} hi={hi:?}");
    }
}

/// `c2Dot` must not be contracted into an FMA on either side: `x*x + y*y`
/// evaluated with a fused multiply-add differs in the last bit for these
/// operands.
#[test]
fn c2dot_no_fma_contraction() {
    let p = pair();
    let cases = [
        (
            c2v { x: 1.000_000_1, y: 1.0 },
            c2v { x: 1.000_000_1, y: -1.0 },
        ),
        (
            c2v { x: 1.192_092_9e-7, y: 1.0 },
            c2v { x: 1.192_092_9e-7, y: 1.0 },
        ),
        (
            c2v { x: 16_777_217.0, y: 1.0 },
            c2v { x: 1.000_000_1, y: 1.0 },
        ),
        (
            c2v { x: 3.402_823_5e38, y: -3.402_823_5e38 },
            c2v { x: 1.0, y: 1.0 },
        ),
    ];
    for (a, b) in cases {
        check_f!(
            "c2Dot",
            unsafe { (p.c.c2Dot)(a, b) },
            unsafe { (p.rs.c2Dot)(a, b) },
            "c2Dot({a:?}, {b:?})"
        );
    }
}
