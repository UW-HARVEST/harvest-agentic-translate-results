//! Phase B — valid-path differential tests for `agglom`, the only entry point
//! declared in `c_src/include/lib.h`. It composes all nine sub-functions and
//! filters NaN sub-results through 13 separate `isnan()` guards, so it is the
//! test that exercises the whole pipeline rather than one wrapper at a time.
//!
//! Covers `CONFIGS.md` rows C59 … C62.

mod common;

use common::*;
use std::ffi::c_int;

const N: usize = 20_000;

/// The 33 `agglom` parameters, in declaration order.
#[derive(Copy, Clone, Debug, PartialEq)]
struct Args {
    f2_1: f32,
    f2_2: f32,
    f2_3: f32,
    f2_7: f32,
    f2_8: f32,
    f2_9: f32,
    f2_10: f32,
    f3_1: c_int,
    f3_2: c_int,
    f4_1: u64,
    f4_2: u64,
    f5_1: u32,
    f7_1: u32,
    f7_2: u32,
    f7_3: u32,
    f9_1: f32,
    f9_2: f32,
    f9_4: f32,
    f9_5: f32,
    f9_7: f32,
    f9_8: f32,
    f9_10: f32,
    f9_11: f32,
    f10_1: u16,
    f11_2: f32,
    f11_3: f32,
    f11_4: f32,
    f12_2: f32,
    f12_3: f32,
    f12_4: f32,
    f13_2: f32,
    f13_3: f32,
    f13_4: f32,
}

impl Args {
    /// A benign, fully finite baseline that exercises the ordinary paths.
    fn baseline() -> Args {
        Args {
            f2_1: 0.5,
            f2_2: 0.5,
            f2_3: 1.0,
            f2_7: 0.0,
            f2_8: 0.0,
            f2_9: 2.0,
            f2_10: 2.0,
            f3_1: 17,
            f3_2: 5,
            f4_1: 0x0123_4567_89AB_CDEF,
            f4_2: 0xFEDC_BA98_7654_3210,
            f5_1: 0xBEEF,
            f7_1: 4096,
            f7_2: 2,
            f7_3: 16,
            f9_1: 0.0,
            f9_2: 0.0,
            f9_4: 1.0,
            f9_5: 0.0,
            f9_7: 0.0,
            f9_8: 1.0,
            f9_10: 0.3,
            f9_11: 0.3,
            f10_1: 0x3C00,
            f11_2: 30.0,
            f11_3: 0.5,
            f11_4: 0.5,
            f12_2: 200.0,
            f12_3: 0.5,
            f12_4: 0.5,
            f13_2: 0.2,
            f13_3: 0.6,
            f13_4: 0.4,
        }
    }

    fn zeros() -> Args {
        Args {
            f2_1: 0.0,
            f2_2: 0.0,
            f2_3: 0.0,
            f2_7: 0.0,
            f2_8: 0.0,
            f2_9: 0.0,
            f2_10: 0.0,
            f3_1: 0,
            f3_2: 0,
            f4_1: 0,
            f4_2: 0,
            f5_1: 0,
            f7_1: 0,
            f7_2: 0,
            f7_3: 0,
            f9_1: 0.0,
            f9_2: 0.0,
            f9_4: 0.0,
            f9_5: 0.0,
            f9_7: 0.0,
            f9_8: 0.0,
            f9_10: 0.0,
            f9_11: 0.0,
            f10_1: 0,
            f11_2: 0.0,
            f11_3: 0.0,
            f11_4: 0.0,
            f12_2: 0.0,
            f12_3: 0.0,
            f12_4: 0.0,
            f13_2: 0.0,
            f13_3: 0.0,
            f13_4: 0.0,
        }
    }

    /// Every bit set in every parameter.
    fn all_ones() -> Args {
        let f = f32::from_bits(0xFFFF_FFFF);
        Args {
            f2_1: f,
            f2_2: f,
            f2_3: f,
            f2_7: f,
            f2_8: f,
            f2_9: f,
            f2_10: f,
            f3_1: -1,
            f3_2: -1,
            f4_1: u64::MAX,
            f4_2: u64::MAX,
            f5_1: u32::MAX,
            f7_1: u32::MAX,
            f7_2: u32::MAX,
            f7_3: u32::MAX,
            f9_1: f,
            f9_2: f,
            f9_4: f,
            f9_5: f,
            f9_7: f,
            f9_8: f,
            f9_10: f,
            f9_11: f,
            f10_1: u16::MAX,
            f11_2: f,
            f11_3: f,
            f11_4: f,
            f12_2: f,
            f12_3: f,
            f12_4: f,
            f13_2: f,
            f13_3: f,
            f13_4: f,
        }
    }

    fn random(r: &mut Rng) -> Args {
        Args {
            f2_1: r.nice_f32(4.0),
            f2_2: r.nice_f32(4.0),
            f2_3: r.nice_f32(4.0),
            f2_7: r.nice_f32(4.0),
            f2_8: r.nice_f32(4.0),
            f2_9: r.nice_f32(4.0),
            f2_10: r.nice_f32(4.0),
            f3_1: r.edgy_i32(),
            f3_2: r.edgy_i32(),
            f4_1: r.edgy_u64(),
            f4_2: r.edgy_u64(),
            f5_1: r.edgy_u32(),
            f7_1: r.edgy_u32(),
            f7_2: r.edgy_u32(),
            f7_3: r.edgy_u32(),
            f9_1: r.nice_f32(4.0),
            f9_2: r.nice_f32(4.0),
            f9_4: r.nice_f32(4.0),
            f9_5: r.nice_f32(4.0),
            f9_7: r.nice_f32(4.0),
            f9_8: r.nice_f32(4.0),
            f9_10: r.nice_f32(4.0),
            f9_11: r.nice_f32(4.0),
            f10_1: r.next_u16(),
            f11_2: r.nice_f32(400.0),
            f11_3: r.nice_f32(2.0),
            f11_4: r.nice_f32(2.0),
            f12_2: r.nice_f32(400.0),
            f12_3: r.nice_f32(2.0),
            f12_4: r.nice_f32(2.0),
            f13_2: r.nice_f32(2.0),
            f13_3: r.nice_f32(2.0),
            f13_4: r.nice_f32(2.0),
        }
    }

    /// Fully uniform raw bit patterns in every field.
    fn raw(r: &mut Rng) -> Args {
        Args {
            f2_1: r.raw_f32(),
            f2_2: r.raw_f32(),
            f2_3: r.raw_f32(),
            f2_7: r.raw_f32(),
            f2_8: r.raw_f32(),
            f2_9: r.raw_f32(),
            f2_10: r.raw_f32(),
            f3_1: r.next_i32(),
            f3_2: r.next_i32(),
            f4_1: r.next_u64(),
            f4_2: r.next_u64(),
            f5_1: r.next_u32(),
            f7_1: r.next_u32(),
            f7_2: r.next_u32(),
            f7_3: r.next_u32(),
            f9_1: r.raw_f32(),
            f9_2: r.raw_f32(),
            f9_4: r.raw_f32(),
            f9_5: r.raw_f32(),
            f9_7: r.raw_f32(),
            f9_8: r.raw_f32(),
            f9_10: r.raw_f32(),
            f9_11: r.raw_f32(),
            f10_1: r.next_u16(),
            f11_2: r.raw_f32(),
            f11_3: r.raw_f32(),
            f11_4: r.raw_f32(),
            f12_2: r.raw_f32(),
            f12_3: r.raw_f32(),
            f12_4: r.raw_f32(),
            f13_2: r.raw_f32(),
            f13_3: r.raw_f32(),
            f13_4: r.raw_f32(),
        }
    }
}

#[rustfmt::skip]
fn call(f: FnAgglom, a: Args) -> f64 {
    unsafe {
        f(
            a.f2_1, a.f2_2, a.f2_3, a.f2_7, a.f2_8, a.f2_9, a.f2_10,
            a.f3_1, a.f3_2,
            a.f4_1, a.f4_2,
            a.f5_1,
            a.f7_1, a.f7_2, a.f7_3,
            a.f9_1, a.f9_2, a.f9_4, a.f9_5, a.f9_7, a.f9_8, a.f9_10, a.f9_11,
            a.f10_1,
            a.f11_2, a.f11_3, a.f11_4,
            a.f12_2, a.f12_3, a.f12_4,
            a.f13_2, a.f13_3, a.f13_4,
        )
    }
}

#[track_caller]
fn chk(p: &Pair, a: Args, tag: &str) {
    same(tag, a, call(p.c.agglom, a), call(p.rs.agglom, a));
}

// ---------------------------------------------------------------------------
// C59 — fully random parameters
// ---------------------------------------------------------------------------

#[test]
fn c59_agglom_random() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x59);
    for _ in 0..N {
        chk(p, Args::random(&mut r), "agglom/nice");
    }
    for _ in 0..N {
        chk(p, Args::raw(&mut r), "agglom/raw");
    }
    // Finite, in-range parameters: keeps the running double sum meaningful so
    // a sign/ordering error cannot hide behind an Inf or a NaN. (A tiny
    // fraction still goes non-finite because a near-degenerate `f9` triangle
    // produces ±Inf, which `isnan()` does not filter — so we only require that
    // the great majority are finite, and always compare bit-for-bit.)
    let mut finite = 0usize;
    for _ in 0..N {
        let mut a = Args::random(&mut r);
        a.f2_1 = r.finite_f32(4.0);
        a.f2_2 = r.finite_f32(4.0);
        a.f2_3 = r.finite_f32(4.0);
        a.f2_7 = r.finite_f32(4.0);
        a.f2_8 = r.finite_f32(4.0);
        a.f2_9 = r.finite_f32(4.0);
        a.f2_10 = r.finite_f32(4.0);
        a.f9_1 = r.finite_f32(4.0);
        a.f9_2 = r.finite_f32(4.0);
        a.f9_4 = r.finite_f32(4.0);
        a.f9_5 = r.finite_f32(4.0);
        a.f9_7 = r.finite_f32(4.0);
        a.f9_8 = r.finite_f32(4.0);
        a.f9_10 = r.finite_f32(4.0);
        a.f9_11 = r.finite_f32(4.0);
        a.f11_2 = r.range_f32(0.0, 360.0);
        a.f11_3 = r.range_f32(0.0, 1.0);
        a.f11_4 = r.range_f32(0.0, 1.0);
        a.f12_2 = r.range_f32(0.0, 360.0);
        a.f12_3 = r.range_f32(0.0, 1.0);
        a.f12_4 = r.range_f32(0.0, 1.0);
        a.f13_2 = r.range_f32(0.0, 1.0);
        a.f13_3 = r.range_f32(0.0, 1.0);
        a.f13_4 = r.range_f32(0.0, 1.0);
        if call(p.c.agglom, a).is_finite() {
            finite += 1;
        }
        chk(p, a, "agglom/finite");
    }
    assert!(
        finite > N * 9 / 10,
        "only {finite}/{N} in-range cases produced a finite sum — the \
         in-range generator is not exercising the ordinary arithmetic path"
    );
}

// ---------------------------------------------------------------------------
// C60 — extremes and per-parameter boundary sweeps
// ---------------------------------------------------------------------------

#[test]
fn c60_agglom_extremes_and_sweeps() {
    let p = pair();
    chk(p, Args::zeros(), "agglom/zeros");
    chk(p, Args::all_ones(), "agglom/all-ones");
    chk(p, Args::baseline(), "agglom/baseline");

    // One-hot sweeps: set exactly one parameter to a boundary value while
    // the other 32 keep the benign baseline. 33 params x boundary corpus.
    macro_rules! sweep_f32 {
        ($field:ident) => {
            for &bits in SPECIAL_F32 {
                let mut a = Args::baseline();
                a.$field = f32::from_bits(bits);
                chk(p, a, concat!("agglom/sweep-", stringify!($field)));
            }
        };
    }
    sweep_f32!(f2_1);
    sweep_f32!(f2_2);
    sweep_f32!(f2_3);
    sweep_f32!(f2_7);
    sweep_f32!(f2_8);
    sweep_f32!(f2_9);
    sweep_f32!(f2_10);
    sweep_f32!(f9_1);
    sweep_f32!(f9_2);
    sweep_f32!(f9_4);
    sweep_f32!(f9_5);
    sweep_f32!(f9_7);
    sweep_f32!(f9_8);
    sweep_f32!(f9_10);
    sweep_f32!(f9_11);
    sweep_f32!(f11_2);
    sweep_f32!(f11_3);
    sweep_f32!(f11_4);
    sweep_f32!(f12_2);
    sweep_f32!(f12_3);
    sweep_f32!(f12_4);
    sweep_f32!(f13_2);
    sweep_f32!(f13_3);
    sweep_f32!(f13_4);

    macro_rules! sweep {
        ($field:ident, $corpus:expr) => {
            for &vv in $corpus {
                let mut a = Args::baseline();
                a.$field = vv;
                chk(p, a, concat!("agglom/sweep-", stringify!($field)));
            }
        };
    }
    sweep!(f3_1, SPECIAL_I32);
    sweep!(f3_2, SPECIAL_I32);
    sweep!(f4_1, SPECIAL_U64);
    sweep!(f4_2, SPECIAL_U64);
    sweep!(f5_1, SPECIAL_U32);
    sweep!(f7_1, SPECIAL_U32);
    sweep!(f7_2, SPECIAL_U32);
    sweep!(f7_3, SPECIAL_U32);

    // f10_1 sweep over interesting half-float encodings + every bucket
    for n in 0u16..64 {
        for lo in [0u16, 1, 0x1FF, 0x3FE, 0x3FF] {
            let mut a = Args::baseline();
            a.f10_1 = (n << 10) | lo;
            chk(p, a, "agglom/sweep-f10_1");
        }
    }

    // pairwise sweep of the f3 parameters (the most branch-dense sub-function)
    for &v1 in SPECIAL_I32 {
        for &v2 in SPECIAL_I32 {
            let mut a = Args::baseline();
            a.f3_1 = v1;
            a.f3_2 = v2;
            chk(p, a, "agglom/f3-pair");
        }
    }
}

// ---------------------------------------------------------------------------
// C61 — every sub-function forced onto a specific branch at once
// ---------------------------------------------------------------------------

#[test]
fn c61_agglom_composed_branch_matrix() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x61);

    // Cross-product of one interesting configuration per sub-function.
    let f3_pairs: &[(i32, i32)] = &[
        (17, 5),
        (17, 0),
        (i32::MIN, i32::MIN),
        (i32::MIN, 1),
        (i32::MIN, -1),
        (-7, 3),
        (7, -3),
    ];
    let f7_triples: &[(u32, u32, u32)] = &[
        (4096, 2, 16),
        (4096, 2, 32),
        (4096, 1, 16),
        (4096, 3, 24),
        (0, 2, 32),
        (u32::MAX, u32::MAX, u32::MAX),
    ];
    // (f9 params) degenerate vs proper triangle
    let f9_sets: &[[f32; 8]] = &[
        [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.3, 0.3], // proper
        [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5, 0.5], // fully degenerate
        [0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 0.5, 0.5], // collinear
    ];
    // f11: s == 0 / each sector / else
    let f11_sets: &[[f32; 3]] = &[
        [30.0, 0.0, 0.5],   // s == 0
        [30.0, 0.5, 0.5],   // sector 0
        [90.0, 0.5, 0.5],   // sector 1
        [150.0, 0.5, 0.5],  // third arm
        [-10.0, 0.5, 0.5],  // third arm via h < 0
        [210.0, 0.5, 0.5],  // sector 4
        [270.0, 0.5, 0.5],  // sector 5
        [330.0, 0.5, 0.5],  // sector 6
        [400.0, 0.5, 0.5],  // else
        [f32::NAN, 0.5, 0.5], // else via NaN
    ];
    // f12: s == 0 / i == 0..5 / default
    let f12_sets: &[[f32; 3]] = &[
        [200.0, 0.0, 0.5],
        [30.0, 0.5, 0.5],
        [90.0, 0.5, 0.5],
        [150.0, 0.5, 0.5],
        [210.0, 0.5, 0.5],
        [270.0, 0.5, 0.5],
        [330.0, 0.5, 0.5],
        [-30.0, 0.5, 0.5],
        [f32::NAN, 0.5, 0.5],
    ];
    // f13: delta == 0 / max == 0 / r,g,b max / wrap
    let f13_sets: &[[f32; 3]] = &[
        [0.5, 0.5, 0.5],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, -1.0],
        [1.0, 0.5, 0.25],
        [1.0, 0.25, 0.5],
        [0.25, 1.0, 0.5],
        [0.25, 0.5, 1.0],
        [-1.0, -2.0, -3.0],
    ];

    for &(v1, v2) in f3_pairs {
        for &(b, c, d) in f7_triples {
            for f9s in f9_sets {
                for f11s in f11_sets {
                    for f12s in f12_sets {
                        for f13s in f13_sets {
                            let mut a = Args::baseline();
                            a.f3_1 = v1;
                            a.f3_2 = v2;
                            a.f7_1 = b;
                            a.f7_2 = c;
                            a.f7_3 = d;
                            a.f9_1 = f9s[0];
                            a.f9_2 = f9s[1];
                            a.f9_4 = f9s[2];
                            a.f9_5 = f9s[3];
                            a.f9_7 = f9s[4];
                            a.f9_8 = f9s[5];
                            a.f9_10 = f9s[6];
                            a.f9_11 = f9s[7];
                            a.f11_2 = f11s[0];
                            a.f11_3 = f11s[1];
                            a.f11_4 = f11s[2];
                            a.f12_2 = f12s[0];
                            a.f12_3 = f12s[1];
                            a.f12_4 = f12s[2];
                            a.f13_2 = f13s[0];
                            a.f13_3 = f13s[1];
                            a.f13_4 = f13s[2];
                            // randomise the remaining, independent parameters
                            a.f4_1 = r.next_u64();
                            a.f4_2 = r.next_u64();
                            a.f5_1 = r.next_u32();
                            a.f10_1 = r.next_u16();
                            chk(p, a, "agglom/branch-matrix");
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C62 — the 13 isnan() filters
// ---------------------------------------------------------------------------

#[test]
fn c62_agglom_nan_filters() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x62);

    let nans = [
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFFFF_FFFF),
    ];

    // f9 NaN -> both `isnan(f9_r.x)` and `isnan(f9_r.y)` filters fire
    for &nn in &nans {
        let mut a = Args::baseline();
        a.f9_1 = nn;
        chk(p, a, "agglom/nan-f9");
        let mut a = Args::baseline();
        a.f9_10 = nn;
        chk(p, a, "agglom/nan-f9-probe");
    }
    // f10 NaN encodings (half-float NaN: exponent all ones, mantissa != 0)
    for m in [1u16, 0x100, 0x1FF, 0x200, 0x3FF] {
        for sign in [0u16, 0x8000] {
            let mut a = Args::baseline();
            a.f10_1 = sign | 0x7C00 | m;
            chk(p, a, "agglom/nan-f10");
        }
    }
    // f11 / f12 / f13 NaN outputs
    for &nn in &nans {
        for (i, field) in [0usize, 1, 2].into_iter().enumerate() {
            let _ = i;
            let mut a = Args::baseline();
            match field {
                0 => a.f11_2 = nn,
                1 => a.f11_3 = nn,
                _ => a.f11_4 = nn,
            }
            chk(p, a, "agglom/nan-f11");
            let mut a = Args::baseline();
            match field {
                0 => a.f12_2 = nn,
                1 => a.f12_3 = nn,
                _ => a.f12_4 = nn,
            }
            chk(p, a, "agglom/nan-f12");
            let mut a = Args::baseline();
            match field {
                0 => a.f13_2 = nn,
                1 => a.f13_3 = nn,
                _ => a.f13_4 = nn,
            }
            chk(p, a, "agglom/nan-f13");
        }
    }
    // all NaN-producing inputs simultaneously: every filter fires at once
    for &nn in &nans {
        let mut a = Args::baseline();
        a.f9_1 = nn;
        a.f9_2 = nn;
        a.f11_2 = nn;
        a.f11_3 = nn;
        a.f11_4 = nn;
        a.f12_2 = nn;
        a.f12_3 = nn;
        a.f12_4 = nn;
        a.f13_2 = nn;
        a.f13_3 = nn;
        a.f13_4 = nn;
        a.f10_1 = 0x7E00;
        chk(p, a, "agglom/nan-all");
    }
    // Inf inputs so the accumulator itself can turn into NaN mid-way
    for &inf in &[f32::INFINITY, f32::NEG_INFINITY] {
        for &inf2 in &[f32::INFINITY, f32::NEG_INFINITY] {
            let mut a = Args::baseline();
            a.f9_10 = inf;
            a.f11_4 = inf2;
            chk(p, a, "agglom/inf-mix");
            let mut a = Args::baseline();
            a.f13_2 = inf;
            a.f13_4 = inf2;
            a.f12_4 = inf;
            chk(p, a, "agglom/inf-mix2");
        }
    }
    // randomized NaN-heavy parameters
    for _ in 0..N {
        let mut a = Args::random(&mut r);
        let pick = |r: &mut Rng| nans[(r.below(4)) as usize];
        if r.next_u32() & 1 == 0 {
            a.f9_1 = pick(&mut r);
        }
        if r.next_u32() & 1 == 0 {
            a.f11_3 = pick(&mut r);
        }
        if r.next_u32() & 1 == 0 {
            a.f12_3 = pick(&mut r);
        }
        if r.next_u32() & 1 == 0 {
            a.f13_3 = pick(&mut r);
        }
        chk(p, a, "agglom/nan-random");
    }
}
