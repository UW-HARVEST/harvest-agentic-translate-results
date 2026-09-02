//! Phase C — error/rejection-path differential tests, one test per `ERRORS.md`
//! row. Both `.so` files are loaded via `libloading`; outcomes are compared as
//! exact `f32` bit patterns (so `NaN` payloads and `-0.0` are distinguished),
//! or, for the null-pointer rows, as identical fatal signals observed
//! out-of-process.

mod common;

use common::{assert_same, check, Rng, SEED};

const NAN: f32 = f32::NAN;
const INF: f32 = f32::INFINITY;
const NINF: f32 = f32::NEG_INFINITY;

fn b(x: f32) -> u32 {
    x.to_bits()
}

// E1 — delta == 0 via r == g == b
#[test]
fn err_e1_delta_zero_achromatic() {
    for v in [0.5f32, 0.0, 1.0, 42.0, 1e-30, 1e30, f32::MAX, f32::MIN_POSITIVE] {
        let out = check(&[v, v, v], "E1");
        assert_eq!(b(out[0]), b(0.0), "E1: h must be the 0.0 initialiser");
        assert_eq!(b(out[1]), b(0.0), "E1: s must be the 0.0 initialiser");
        assert_eq!(b(out[2]), b(v), "E1: v must be max == {v}");
    }
    let mut rng = Rng::new(SEED ^ 0xE1);
    for _ in 0..5000 {
        let v = rng.range(-1e6, 1e6);
        assert_same(&[v, v, v], "E1/random");
    }
}

// E2 — max == 0 with delta != 0
#[test]
fn err_e2_max_zero() {
    for src in [
        [-1.0f32, 0.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, -1.0],
        [-1.0, -2.0, 0.0],
        [-f32::MAX, 0.0, -1.0],
        [-f32::MIN_POSITIVE, 0.0, 0.0],
    ] {
        let out = check(&src, "E2");
        assert_eq!(b(out[0]), b(0.0), "E2 {src:?}: h");
        assert_eq!(b(out[1]), b(0.0), "E2 {src:?}: s");
        assert_eq!(out[2], 0.0f32, "E2 {src:?}: v == max == 0");
        // Crucially: the max==0 disjunct must fire BEFORE the division, so no
        // inf/NaN may leak out.
        assert!(out.iter().all(|x| x.is_finite()), "E2 {src:?}: no inf/NaN may escape");
    }
    let mut rng = Rng::new(SEED ^ 0xE2);
    for _ in 0..5000 {
        // Random negative min, exact zero max.
        let n1 = -rng.unit() * 1e6 - f32::MIN_POSITIVE;
        let n2 = -rng.unit() * 1e6;
        for src in [[0.0, n1, n2], [n1, 0.0, n2], [n1, n2, 0.0]] {
            assert_same(&src, "E2/random");
        }
    }
}

// E3 — all channels zero
#[test]
fn err_e3_all_zero() {
    let out = check(&[0.0, 0.0, 0.0], "E3");
    assert_eq!([b(out[0]), b(out[1]), b(out[2])], [b(0.0), b(0.0), b(0.0)]);
}

// E4 — negative achromatic: v stays negative, no clamping
#[test]
fn err_e4_negative_achromatic() {
    for v in [-2.0f32, -1.0, -1e-30, -f32::MAX, -f32::MIN_POSITIVE] {
        let out = check(&[v, v, v], "E4");
        assert_eq!(b(out[0]), b(0.0));
        assert_eq!(b(out[1]), b(0.0));
        assert_eq!(b(out[2]), b(v), "E4: C does not clamp negative v");
    }
}

// E5 — all channels -0.0: v keeps the sign bit
#[test]
fn err_e5_negative_zero() {
    let out = check(&[-0.0, -0.0, -0.0], "E5");
    assert_eq!(b(out[0]), b(0.0), "E5: h");
    assert_eq!(b(out[1]), b(0.0), "E5: s");
    assert_eq!(b(out[2]), b(-0.0), "E5: v must retain the -0.0 sign bit (0x80000000)");
}

// E6 — mixed signed zeros: tie-breaking of the C ternaries
#[test]
fn err_e6_mixed_signed_zero() {
    let zeros = [0.0f32, -0.0f32];
    for &r in &zeros {
        for &g in &zeros {
            for &bb in &zeros {
                let out = check(&[r, g, bb], "E6");
                assert_eq!(b(out[0]), b(0.0), "E6 {r} {g} {bb}: h");
                assert_eq!(b(out[1]), b(0.0), "E6 {r} {g} {bb}: s");
                // The ternaries use `<` / `>`, which are false for equal
                // magnitudes, so max keeps whichever operand the C picks. The
                // only requirement is that Rust picks identically -- already
                // asserted bitwise by `check`.
                assert_eq!(out[2], 0.0f32, "E6 {r} {g} {bb}: v is a zero");
            }
        }
    }
    // Pinned: {-0.0, +0.0, -0.0}. max starts at r=-0.0; `max > g` is false so
    // max=g=+0.0; `max > b` is false so max=b=-0.0. => v is -0.0.
    let out = check(&[-0.0, 0.0, -0.0], "E6/pinned");
    assert_eq!(b(out[2]), b(-0.0));
}

// E7 — NaN in src[0]
#[test]
fn err_e7_nan_r() {
    for g in [0.0f32, 1.0, -1.0, INF, NINF, f32::MAX] {
        for bb in [0.0f32, 1.0, -1.0, INF, NINF, f32::MAX] {
            assert_same(&[NAN, g, bb], "E7");
        }
    }
    // Pinned: {NaN, 0.0, 0.0}. min: NaN<0 false => min=0; 0<0 false => min=0.
    // max: NaN>0 false => max=0; 0>0 false => max=0. delta=0 => early return.
    let out = check(&[NAN, 0.0, 0.0], "E7/pinned");
    assert_eq!([b(out[0]), b(out[1]), b(out[2])], [b(0.0), b(0.0), b(0.0)]);
    // Pinned: {NaN, 1.0, 0.0}. min=0, max=1, delta=1, s=1.
    // r==max? NaN==1 false. g==max? 1==1 true => h=2+(0-NaN)/1 = NaN.
    let out = check(&[NAN, 1.0, 0.0], "E7/pinned2");
    assert!(out[0].is_nan(), "E7: h must be NaN, got {}", out[0]);
    assert_eq!(b(out[1]), b(1.0));
    assert_eq!(b(out[2]), b(1.0));
}

// E8 — NaN in src[1] (g): no short-circuit, full NaN propagation
#[test]
fn err_e8_nan_g() {
    let out = check(&[0.0, NAN, 0.0], "E8");
    // min: 0<NaN false => min=NaN; NaN<0 false => min=0. Wait: second ternary
    // is min<b i.e. NaN<0 => false => min=b=0.
    // max: 0>NaN false => max=NaN; NaN>0 false => max=b=0. delta = 0-0 = 0.
    // => early return with v = 0.
    assert_eq!([b(out[0]), b(out[1]), b(out[2])], [b(0.0), b(0.0), b(0.0)]);

    // A shape where the NaN survives into max: {0.0, NaN, NaN}.
    // min: 0<NaN false => NaN; NaN<NaN false => NaN. max likewise NaN.
    // delta = NaN-NaN = NaN; NaN==0 false, NaN==0 false => no short-circuit.
    let out = check(&[0.0, NAN, NAN], "E8/survives");
    assert!(out[0].is_nan() && out[1].is_nan() && out[2].is_nan(), "E8: all NaN, got {out:?}");

    for r in [0.0f32, 1.0, -1.0, INF, NINF, f32::MAX] {
        for bb in [0.0f32, 1.0, -1.0, INF, NINF, NAN, f32::MAX] {
            assert_same(&[r, NAN, bb], "E8/grid");
        }
    }
}

// E9 — NaN in src[2] (b)
#[test]
fn err_e9_nan_b() {
    for r in [0.0f32, 1.0, -1.0, INF, NINF, NAN, f32::MAX] {
        for g in [0.0f32, 1.0, -1.0, INF, NINF, NAN, f32::MAX] {
            assert_same(&[r, g, NAN], "E9");
        }
    }
    // Pinned: {0.0, 0.0, NaN}. min: 0<0 false => 0; 0<NaN false => NaN.
    // max: 0>0 false => 0; 0>NaN false => NaN. delta = NaN-NaN = NaN.
    // No short-circuit => everything NaN.
    let out = check(&[0.0, 0.0, NAN], "E9/pinned");
    assert!(out[0].is_nan() && out[1].is_nan() && out[2].is_nan(), "E9 got {out:?}");
}

// E10 — non-canonical / signalling NaN payloads must be preserved identically
#[test]
fn err_e10_nan_payloads() {
    // quiet NaNs, signalling NaNs, negative NaNs, min/max payloads.
    let payloads: [u32; 9] = [
        0x7FC0_0000, 0xFFC0_0000, 0x7FC0_1234, 0x7F80_0001, 0xFF80_0001, 0x7FBF_FFFF,
        0x7FFF_FFFF, 0xFFFF_FFFF, 0x7FA5_A5A5,
    ];
    for &p in &payloads {
        let n = f32::from_bits(p);
        for other in [0.0f32, 1.0, -1.0, 0.5, INF, NINF] {
            assert_same(&[n, other, other], "E10/r");
            assert_same(&[other, n, other], "E10/g");
            assert_same(&[other, other, n], "E10/b");
            assert_same(&[n, n, other], "E10/rg");
            assert_same(&[n, n, n], "E10/all");
        }
    }
    // Two different NaN payloads in the same call: whichever one the C
    // arithmetic propagates, Rust must propagate the same bits.
    for &p in &payloads {
        for &q in &payloads {
            assert_same(&[f32::from_bits(p), f32::from_bits(q), 1.0], "E10/pair");
        }
    }
}

// E11 — +inf as the max channel
#[test]
fn err_e11_pos_inf() {
    let out = check(&[INF, 0.0, 0.0], "E11");
    // max=inf, min=0, delta=inf, s=inf/inf=NaN, r==max => h=(0-0)/inf=0, *60=0.
    assert_eq!(b(out[0]), b(0.0), "E11: h must be +0.0, got {}", out[0]);
    assert!(out[1].is_nan(), "E11: s = inf/inf must be NaN, got {}", out[1]);
    assert_eq!(b(out[2]), b(INF), "E11: v must be +inf");

    for src in [
        [INF, 0.0, 0.0],
        [0.0, INF, 0.0],
        [0.0, 0.0, INF],
        [INF, 1.0, -1.0],
        [1.0, INF, -1.0],
        [1.0, -1.0, INF],
        [INF, INF, 0.0],
        [INF, INF, INF],
    ] {
        assert_same(&src, "E11/grid");
    }
}

// E12 — -inf present: max==0 short-circuit fires
#[test]
fn err_e12_neg_inf() {
    let out = check(&[NINF, 0.0, 0.0], "E12");
    // min=-inf, max=0, delta=0-(-inf)=inf. delta==0 false; max==0 TRUE.
    assert_eq!([b(out[0]), b(out[1]), b(out[2])], [b(0.0), b(0.0), b(0.0)],
        "E12: the max==0 disjunct must short-circuit before any division");

    for src in [
        [NINF, 0.0, 0.0],
        [0.0, NINF, 0.0],
        [0.0, 0.0, NINF],
        [NINF, 1.0, 0.0],
        [NINF, NINF, 0.0],
        [NINF, NINF, NINF],
        [NINF, -1.0, -2.0],
    ] {
        assert_same(&src, "E12/grid");
    }
    // All -inf: min=max=-inf, delta = -inf - -inf = NaN. NaN==0 false;
    // -inf==0 false => NO short-circuit => full NaN path.
    let out = check(&[NINF, NINF, NINF], "E12/all");
    assert!(out[0].is_nan() && out[1].is_nan(), "E12/all got {out:?}");
    assert_eq!(b(out[2]), b(NINF), "E12/all: v = max = -inf");
}

// E13 — inf - inf mixes
#[test]
fn err_e13_inf_mixes() {
    for src in [
        [INF, NINF, 0.0],
        [NINF, INF, 0.0],
        [0.0, INF, NINF],
        [INF, INF, NINF],
        [NINF, NINF, INF],
        [INF, NINF, INF],
        [INF, 0.0, NINF],
    ] {
        assert_same(&src, "E13");
    }
    // {inf, -inf, 0}: min=-inf, max=inf, delta=inf. s=inf/inf=NaN.
    // r==max (inf==inf) true => h=(-inf-0)/inf = -inf/inf = NaN. NaN<0 false.
    let out = check(&[INF, NINF, 0.0], "E13/pinned");
    assert!(out[0].is_nan(), "E13: h got {}", out[0]);
    assert!(out[1].is_nan(), "E13: s got {}", out[1]);
    assert_eq!(b(out[2]), b(INF));
    // {inf, inf, -inf}: min=-inf, max=inf, delta=inf, r==max => h=(inf-(-inf))/inf=inf/inf=NaN
    let out = check(&[INF, INF, NINF], "E13/pinned2");
    assert!(out[0].is_nan());
    assert_eq!(b(out[2]), b(INF));
}

// E14 — finite inputs whose delta overflows to +inf
#[test]
fn err_e14_delta_overflow() {
    let big = f32::MAX;
    let out = check(&[big, -big, 0.0], "E14");
    // min=-MAX, max=MAX, delta = MAX-(-MAX) -> overflows to +inf.
    assert_eq!(b(out[2]), b(big), "E14: v = MAX");
    assert_eq!(b(out[1]), b(INF), "E14: s = inf/MAX = inf, got {}", out[1]);
    // r==max => h = (-MAX - 0)/inf = -0.0; -0.0 < 0 is FALSE => no +360;
    // h *= 60 keeps -0.0.
    assert_eq!(b(out[0]), b(-0.0), "E14: h must be -0.0 (0x80000000), got 0x{:08x}", b(out[0]));

    for src in [
        [big, -big, 0.0],
        [-big, big, 0.0],
        [0.0, big, -big],
        [big, 0.0, -big],
        [big / 2.0, -big, big],
        [big, -big, -big],
    ] {
        assert_same(&src, "E14/grid");
    }
    let mut rng = Rng::new(SEED ^ 0xE14);
    for _ in 0..5000 {
        let a = rng.unit() * big;
        let c = rng.unit() * big;
        assert_same(&[a, -c, 0.0], "E14/random");
        assert_same(&[-a, c, 0.0], "E14/random2");
    }
}

// E15 — subnormal delta / channels; no flush-to-zero
#[test]
fn err_e15_subnormals() {
    let tiny = f32::from_bits(1); // 1.4e-45
    let out = check(&[tiny, 0.0, 0.0], "E15");
    // min=0, max=tiny, delta=tiny. s = tiny/tiny = 1.0. r==max => h=(0-0)/tiny=0.
    assert_eq!(b(out[0]), b(0.0), "E15: h");
    assert_eq!(b(out[1]), b(1.0), "E15: s must be exactly 1.0 (no flush-to-zero), got {}", out[1]);
    assert_eq!(b(out[2]), b(tiny), "E15: v must remain the subnormal");

    for src in [
        [tiny, 0.0, 0.0],
        [0.0, tiny, 0.0],
        [0.0, 0.0, tiny],
        [tiny, -tiny, 0.0],
        [f32::MIN_POSITIVE, tiny, 0.0],
        [f32::MIN_POSITIVE, 0.0, tiny],
        [f32::from_bits(3), f32::from_bits(2), f32::from_bits(1)],
    ] {
        assert_same(&src, "E15/grid");
    }
    let mut rng = Rng::new(SEED ^ 0xE15);
    for _ in 0..10_000 {
        assert_same(&[rng.subnormal(), rng.subnormal(), rng.subnormal()], "E15/random");
    }
}

// E16 — h < 0 branch: +360 wrap
#[test]
fn err_e16_h_negative_wrap() {
    let out = check(&[1.0, 0.0, 0.5], "E16");
    // r==max, delta=1, h=(0-0.5)/1=-0.5, *60=-30, <0 => +360 => 330.
    assert_eq!(b(out[0]), b(330.0), "E16: h must wrap to 330, got {}", out[0]);
    assert_eq!(b(out[1]), b(1.0));
    assert_eq!(b(out[2]), b(1.0));

    let mut rng = Rng::new(SEED ^ 0xE16);
    for _ in 0..10_000 {
        // r strict max, g < b => negative hue before wrap.
        let r = rng.range(0.5, 1.0);
        let bb = rng.range(0.1, 0.5);
        let g = rng.range(0.0, 0.1);
        let out = check(&[r, g, bb], "E16/random");
        assert!(out[0] > 300.0 && out[0] <= 360.0, "E16: expected wrap, got {}", out[0]);
    }
}

// E17 — h == -0.0 before the `h < 0` test: no wrap, sign bit preserved
#[test]
fn err_e17_h_negative_zero() {
    // r is max, g == b, and g - b == -0.0 requires g = b = -0.0 ... or
    // (g-b) == +0.0 for equal non-zero g,b. To get exactly -0.0 we need
    // g - b = -0.0, which for finite equal operands only happens as
    // (-x) - (-x) = +0.0. So use the division-sign route: (g-b) = -0.0 when
    // g = -0.0 and b = +0.0.
    let out = check(&[1.0, -0.0, 0.0], "E17");
    // delta = 1 - (-0.0)... min: 1 < -0.0 false => min=-0.0; -0.0 < 0.0 false
    // => min=-0.0. max: 1 > -0.0 true => 1; 1 > 0 true => 1. delta = 1-(-0)=1.
    // r==max => h = (-0.0 - 0.0)/1 = -0.0. -0.0 < 0 FALSE => no +360.
    assert_eq!(
        b(out[0]),
        b(-0.0),
        "E17: h must stay -0.0 (0x80000000), not +0.0 and not 360.0; got {} (0x{:08x})",
        out[0],
        b(out[0])
    );
    // The other route to -0.0: g - b negative but divided by +inf delta.
    let out = check(&[f32::MAX, -f32::MAX, 0.0], "E17/via-inf-delta");
    assert_eq!(b(out[0]), b(-0.0), "E17: got 0x{:08x}", b(out[0]));

    // And confirm +0.0 stays +0.0 in the mirror case.
    let out = check(&[1.0, 0.0, -0.0], "E17/positive-zero");
    assert_eq!(b(out[0]), b(0.0), "E17: +0.0 case, got 0x{:08x}", b(out[0]));
}

// E18 — tie r == g == max: the `r` branch wins
#[test]
fn err_e18_tie_r_g() {
    let out = check(&[1.0, 1.0, 0.0], "E18");
    // r branch: h = (g-b)/delta = 1/1 = 1, *60 = 60. (The g branch would give
    // 60*(2 + (b-r)/delta) = 60*(2-1) = 60 as well -- so use an asymmetric case.)
    assert_eq!(b(out[0]), b(60.0));

    // Asymmetric tie that distinguishes the branches: r == g == max, b such
    // that the two formulas differ.
    // r=g=1, b=-1: delta = 2. r branch: (1-(-1))/2 = 1 -> 60.
    //              g branch: 2 + (-1-1)/2 = 1 -> 60. Still equal by symmetry.
    // Use r == b == max instead (see E19) plus a NaN-free asymmetric check:
    // r = g = max with b < min? impossible. The r/g tie is inherently
    // symmetric here, so assert bitwise agreement over many b values.
    let mut rng = Rng::new(SEED ^ 0xE18);
    for _ in 0..10_000 {
        let hi = rng.range(-1e3, 1e3);
        let lo = rng.range(-1e6, 1e3);
        assert_same(&[hi, hi, lo], "E18/random");
    }
    for bb in [0.0f32, -0.0, -1.0, 1.0, INF, NINF, NAN, f32::MAX, f32::MIN] {
        assert_same(&[1.0, 1.0, bb], "E18/grid");
    }
    // r == b == max, g smaller: r branch (not else).
    let out = check(&[1.0, 0.0, 1.0], "E18/r==b");
    // r branch: (g-b)/delta = (0-1)/1 = -1, *60 = -60, <0 => 300.
    // else branch would give 60*(4 + (r-g)/delta) = 60*5 = 300. Also equal.
    assert_eq!(b(out[0]), b(300.0));
}

// E19 — tie g == b == max, r smaller: the `g` branch, not `else`
#[test]
fn err_e19_tie_g_b() {
    let out = check(&[0.0, 1.0, 1.0], "E19");
    // r==max? 0==1 false. g==max? true => g branch: 2 + (b-r)/delta = 2+1 = 3,
    // *60 = 180.  The else branch would give 4 + (r-g)/delta = 4-1 = 3 -> 180.
    assert_eq!(b(out[0]), b(180.0));
    let mut rng = Rng::new(SEED ^ 0xE19);
    for _ in 0..10_000 {
        let hi = rng.range(-1e3, 1e3);
        let lo = hi - rng.unit() * 1e3 - 1.0;
        assert_same(&[lo, hi, hi], "E19/random");
    }
    for r in [0.0f32, -0.0, -1.0, INF, NINF, NAN, f32::MIN] {
        assert_same(&[r, 1.0, 1.0], "E19/grid");
    }
}

// E20 — the final `else` is an unconditional fallthrough, not a `b == max` test
#[test]
fn err_e20_else_fallthrough() {
    // Ordinary route: b strict max.
    let out = check(&[0.0, 0.25, 1.0], "E20");
    // delta=1, else branch: 4 + (0-0.25)/1 = 3.75, *60 = 225.
    assert_eq!(b(out[0]), b(225.0), "E20: else branch, got {}", out[0]);

    // Pathological route: NO channel equals max, so `else` is taken even though
    // b != max. Requires NaN so that max is a NaN that compares unequal to all
    // three channels... but then delta is NaN too and the branch still runs.
    // {NaN, NaN, 1.0}: min: NaN<NaN false => NaN; NaN<1 false => 1 => min=1.
    //   Actually min = (min<g? min:g) with min=NaN,g=NaN => g = NaN;
    //   then (NaN<1.0)? false => b = 1.0 => min = 1.0.
    // max = (NaN>NaN? NaN : NaN) = NaN; (NaN>1.0)? false => 1.0 => max = 1.0.
    // delta = 0 => early return. So use a shape where max stays NaN:
    // {1.0, NaN, NaN}: min = (1<NaN? 1 : NaN) = NaN; (NaN<NaN? NaN:NaN)=NaN.
    // max = (1>NaN? 1 : NaN) = NaN; (NaN>NaN? NaN : NaN) = NaN. delta = NaN.
    // NaN==0 false, NaN==0 false => proceed. r==max: 1==NaN false.
    // g==max: NaN==NaN false => ELSE taken with no channel equal to max.
    let out = check(&[1.0, NAN, NAN], "E20/no-channel-equals-max");
    assert!(out[0].is_nan(), "E20: else-with-NaN h, got {}", out[0]);
    assert!(out[1].is_nan(), "E20: else-with-NaN s, got {}", out[1]);
    assert!(out[2].is_nan(), "E20: else-with-NaN v, got {}", out[2]);

    let mut rng = Rng::new(SEED ^ 0xE20);
    for _ in 0..10_000 {
        let bb = rng.range(0.6, 1.0);
        assert_same(&[rng.range(0.0, 0.5), rng.range(0.0, 0.5), bb], "E20/random");
    }
}

// E21 — dest == src (full aliasing)
#[test]
fn err_e21_alias_exact() {
    let c = common::c_fn();
    let ru = common::rust_fn();
    let mut rng = Rng::new(SEED ^ 0xE21);
    for i in 0..20_000 {
        let src: [f32; 3] = if i % 3 == 0 {
            [rng.any_f32(), rng.any_f32(), rng.any_f32()]
        } else {
            [rng.unit(), rng.unit(), rng.unit()]
        };
        let mut bc = src;
        let mut br = src;
        unsafe {
            c(bc.as_mut_ptr(), bc.as_ptr());
            ru(br.as_mut_ptr(), br.as_ptr());
        }
        assert_eq!(
            common::bits3(&bc),
            common::bits3(&br),
            "E21 divergence for {src:?}"
        );
    }
}

// E22 — partial overlap dest = src +/- 1
#[test]
fn err_e22_alias_offset() {
    let c = common::c_fn();
    let ru = common::rust_fn();
    let mut rng = Rng::new(SEED ^ 0xE22);
    for _ in 0..20_000 {
        let vals: [f32; 5] = [
            rng.any_f32(),
            rng.unit(),
            rng.unit(),
            rng.unit(),
            rng.any_f32(),
        ];
        for shift in [1isize, -1] {
            let mut bc = vals;
            let mut br = vals;
            unsafe {
                c(bc.as_mut_ptr().offset(1 + shift), bc.as_ptr().add(1));
                ru(br.as_mut_ptr().offset(1 + shift), br.as_ptr().add(1));
            }
            let cb: Vec<u32> = bc.iter().map(|x| x.to_bits()).collect();
            let rb: Vec<u32> = br.iter().map(|x| x.to_bits()).collect();
            assert_eq!(cb, rb, "E22 shift={shift} divergence for {vals:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// E23 / E24 — null pointers. The C performs no null check (verified by grep:
// there is no `if (!src)` / `if (src == NULL)` anywhere), so both libraries
// dereference NULL. Run each call in a forked child and assert BOTH die on the
// SAME fatal signal.
// ---------------------------------------------------------------------------

/// Which library to invoke in the child process.
#[derive(Copy, Clone)]
enum Which {
    C,
    Rust,
}

/// Fork, call `rgb_to_hsv` with the given (possibly null) pointers in the
/// child, and report the child's exit status: `Ok(code)` on normal exit,
/// `Err(signal)` if it was killed.
fn run_isolated(which: Which, dest_null: bool, src_null: bool) -> Result<i32, i32> {
    // Resolve the symbol in the parent so dlopen work is already done and the
    // child only has to make the call.
    let f = match which {
        Which::C => common::c_fn(),
        Which::Rust => common::rust_fn(),
    };
    let mut buf = [0.0f32; 3];
    let src_buf = [0.5f32, 0.25, 0.75];

    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child. Silence the crash so the test output stays readable.
            let dest = if dest_null {
                std::ptr::null_mut()
            } else {
                buf.as_mut_ptr()
            };
            let src = if src_null {
                std::ptr::null()
            } else {
                src_buf.as_ptr()
            };
            f(dest, src);
            // If we somehow survive, exit 0 so the parent sees "no fault".
            libc::_exit(0);
        }
        let mut status: libc::c_int = 0;
        let w = libc::waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Err(libc::WTERMSIG(status))
        } else {
            Ok(libc::WEXITSTATUS(status))
        }
    }
}

// E23 — src == NULL
#[test]
fn err_e23_null_src_faults_identically() {
    let c = run_isolated(Which::C, false, true);
    let r = run_isolated(Which::Rust, false, true);
    assert_eq!(
        c, r,
        "E23: null src must produce the same outcome; C={c:?} Rust={r:?}"
    );
    assert_eq!(
        c,
        Err(libc::SIGSEGV),
        "E23: expected SIGSEGV from the unchecked src[0] dereference, got {c:?}"
    );
}

// E24 — dest == NULL
#[test]
fn err_e24_null_dest_faults_identically() {
    let c = run_isolated(Which::C, true, false);
    let r = run_isolated(Which::Rust, true, false);
    assert_eq!(
        c, r,
        "E24: null dest must produce the same outcome; C={c:?} Rust={r:?}"
    );
    assert_eq!(
        c,
        Err(libc::SIGSEGV),
        "E24: expected SIGSEGV from the unchecked dest[0] store, got {c:?}"
    );

    // Both null as well.
    let c = run_isolated(Which::C, true, true);
    let r = run_isolated(Which::Rust, true, true);
    assert_eq!(c, r, "E24/both: C={c:?} Rust={r:?}");
}

// ---------------------------------------------------------------------------
// Generic boundary coverage required by Phase C beyond the table rows.
// ---------------------------------------------------------------------------

// There is no enum, flag, or integer mode parameter in `lib.h`
// (`void rgb_to_hsv(float *dest, const float *src)`), so "out-of-range enum
// value across the FFI boundary" has no representative for this API. The
// closest analogue is an out-of-domain float: a value outside the documented
// [0,1] RGB range, and a value one step past every representable boundary.
#[test]
fn err_generic_out_of_domain_and_one_step_past_boundaries() {
    fn ulp_up(x: f32) -> f32 {
        f32::from_bits((x.to_bits() as i32).wrapping_add(1) as u32)
    }
    fn ulp_down(x: f32) -> f32 {
        f32::from_bits((x.to_bits() as i32).wrapping_sub(1) as u32)
    }

    // One step past each documented/implicit boundary of the RGB domain and of
    // the float type itself.
    let boundaries: Vec<f32> = vec![
        0.0,
        -0.0,
        ulp_up(0.0),
        ulp_down(0.0),
        1.0,
        ulp_up(1.0),
        ulp_down(1.0),
        -1.0,
        ulp_up(-1.0),
        ulp_down(-1.0),
        f32::MIN_POSITIVE,
        ulp_down(f32::MIN_POSITIVE),
        ulp_up(f32::MIN_POSITIVE),
        f32::MAX,
        ulp_down(f32::MAX),
        ulp_up(f32::MAX), // == +inf
        f32::MIN,
        ulp_down(f32::MIN), // == -inf
        INF,
        NINF,
        NAN,
        // Well outside the documented 0..1 domain.
        2.0,
        -2.0,
        255.0,
        1e30,
        -1e30,
    ];
    for &r in &boundaries {
        for &g in &boundaries {
            for &bb in &boundaries {
                assert_same(&[r, g, bb], "generic/boundary");
            }
        }
    }
}

// Zero-length and oversized "lengths": the API takes no length, so the
// equivalent generic boundary is a buffer larger than needed (extra bytes must
// be left untouched) and a buffer of exactly 3 (no overrun).
#[test]
fn err_generic_no_overrun_or_underrun() {
    let c = common::c_fn();
    let ru = common::rust_fn();
    let mut rng = Rng::new(SEED ^ 0xB0F);
    const GUARD: f32 = 1.5e-30;
    for _ in 0..20_000 {
        let src = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        // 9-element buffer, function writes into the middle 3.
        let mut bc = [GUARD; 9];
        let mut br = [GUARD; 9];
        unsafe {
            c(bc.as_mut_ptr().add(3), src.as_ptr());
            ru(br.as_mut_ptr().add(3), src.as_ptr());
        }
        let cb: Vec<u32> = bc.iter().map(|x| x.to_bits()).collect();
        let rb: Vec<u32> = br.iter().map(|x| x.to_bits()).collect();
        assert_eq!(cb, rb, "generic: buffer contents diverge for {src:?}");
        // Neither may touch the guard region.
        for i in [0usize, 1, 2, 6, 7, 8] {
            assert_eq!(bc[i].to_bits(), GUARD.to_bits(), "C wrote outside dest[0..3] at {i}");
            assert_eq!(br[i].to_bits(), GUARD.to_bits(), "Rust wrote outside dest[0..3] at {i}");
        }
        // And only 3 source elements may be read: put poison right after.
        let padded = [src[0], src[1], src[2], f32::NAN, f32::NAN];
        let mut dc = [0.0f32; 3];
        let mut dr = [0.0f32; 3];
        unsafe {
            c(dc.as_mut_ptr(), padded.as_ptr());
            ru(dr.as_mut_ptr(), padded.as_ptr());
        }
        assert_eq!(common::bits3(&dc), common::bits3(&dr));
        assert_eq!(
            common::bits3(&dc),
            common::bits3(&[bc[3], bc[4], bc[5]]),
            "trailing source elements must not be read"
        );
    }
}

// ---------------------------------------------------------------------------
// Branch-priority coverage BY CONSTRUCTION.
//
// The `if r == max / else if g == max / else` chain can only be distinguished
// when two channels tie at `max`. Random inputs almost never produce an exact
// tie, so ties are constructed here instead of hoped for, across arbitrary bit
// patterns (so `NaN`, `inf`, subnormal and overflowing-`delta` shapes all reach
// the tie).
// ---------------------------------------------------------------------------
#[test]
fn err_branch_priority_ties_constructed() {
    let mut rng = Rng::new(SEED ^ 0xBEEF);
    let specials: [f32; 12] = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        INF,
        NINF,
        NAN,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::from_bits(0x7F80_0001), // sNaN
    ];

    // Exhaustive tie grid over the special values, in all three tie positions.
    for &x in &specials {
        for &y in &specials {
            assert_same(&[x, x, y], "tie/r==g"); // r == g
            assert_same(&[x, y, x], "tie/r==b"); // r == b
            assert_same(&[y, x, x], "tie/g==b"); // g == b
            assert_same(&[x, x, x], "tie/all"); // r == g == b
        }
    }

    // Randomized ties over arbitrary bit patterns.
    for _ in 0..40_000 {
        let x = rng.any_f32();
        let y = rng.any_f32();
        assert_same(&[x, x, y], "tie/r==g/rand");
        assert_same(&[x, y, x], "tie/r==b/rand");
        assert_same(&[y, x, x], "tie/g==b/rand");
    }

    // Randomized ties inside the canonical domain, where delta is well behaved
    // and the tie decides the hue formula.
    for _ in 0..40_000 {
        let x = rng.unit();
        let y = rng.unit();
        assert_same(&[x, x, y], "tie/r==g/unit");
        assert_same(&[x, y, x], "tie/r==b/unit");
        assert_same(&[y, x, x], "tie/g==b/unit");
    }
}
