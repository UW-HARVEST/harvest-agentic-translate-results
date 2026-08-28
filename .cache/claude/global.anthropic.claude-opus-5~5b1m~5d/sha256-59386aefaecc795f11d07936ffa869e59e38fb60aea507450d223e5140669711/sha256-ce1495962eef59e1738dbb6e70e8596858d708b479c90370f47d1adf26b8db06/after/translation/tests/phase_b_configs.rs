//! Phase B — valid-path differential tests, GATED on `CONFIGS.md`.
//!
//! One `#[test]` per `CONFIGS.md` row, named `cfg_rowNN_*`. Every row calls the
//! C `.so` and the Rust `.so` through their exported `rgb_to_hsv` symbol and
//! compares the 3 output floats BIT-FOR-BIT (`to_bits`), so `-0.0` vs `+0.0`
//! and differing NaN payloads are failures.
//!
//! Every row uses MANY randomized inputs (fixed seed `common::SEED`), not one
//! hand-picked vector.

mod common;

use common::*;

/// Vectors per randomized row.
const N: usize = 4_000;

// ---------------------------------------------------------------------------
// Rows 1-9: the three hue branches (lines 26/28/30) x ordering of the two
// non-max channels (which drives the line-33 wrap correction).
// ---------------------------------------------------------------------------

/// Row 1 — branch A (`r == max`), `r > g > b`, in-range. `h >= 0`, no wrap.
#[test]
fn cfg_row01_branch_a_r_gt_g_gt_b() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED);
    let mut d = Diff::new("01 branch A r>g>b");
    for _ in 0..N {
        let (lo, mid, hi) = rng.three_sorted_unit();
        d.check(&c, &rust, [hi, mid, lo]); // r=hi, g=mid, b=lo
    }
    d.finish();
}

/// Row 2 — branch A, `r > b > g` so `g < b` ⇒ `(g-b)/delta < 0` ⇒ wrap `+360`.
#[test]
fn cfg_row02_branch_a_wrap_360() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 2);
    let mut d = Diff::new("02 branch A wrap +360");
    for _ in 0..N {
        let (lo, mid, hi) = rng.three_sorted_unit();
        d.check(&c, &rust, [hi, lo, mid]); // r=hi, g=lo, b=mid  => g < b
    }
    // Also drive the exact ratio -1 boundary: g == min, b == max is impossible
    // in branch A, but g minimal with b just under r maximises |h|.
    for _ in 0..N {
        let hi = rng.range(0.5, 1.0);
        let eps = rng.range(0.0, 1e-6);
        d.check(&c, &rust, [hi, 0.0, hi - eps]);
    }
    d.finish();
}

/// Row 3 — branch A with `g == b` (< r) ⇒ `h` exactly `0`, wrap not taken.
#[test]
fn cfg_row03_branch_a_g_eq_b() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 3);
    let mut d = Diff::new("03 branch A g==b");
    for _ in 0..N {
        let hi = rng.unit();
        let lo = rng.unit() * hi;
        if lo >= hi {
            continue;
        }
        d.check(&c, &rust, [hi, lo, lo]);
    }
    d.finish();
}

/// Row 4 — branch B (`g == max`, `r != max`), `g > r > b`.
#[test]
fn cfg_row04_branch_b_g_gt_r_gt_b() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 4);
    let mut d = Diff::new("04 branch B g>r>b");
    for _ in 0..N {
        let (lo, mid, hi) = rng.three_sorted_unit();
        d.check(&c, &rust, [mid, hi, lo]);
    }
    d.finish();
}

/// Row 5 — branch B, `g > b > r`.
#[test]
fn cfg_row05_branch_b_g_gt_b_gt_r() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 5);
    let mut d = Diff::new("05 branch B g>b>r");
    for _ in 0..N {
        let (lo, mid, hi) = rng.three_sorted_unit();
        d.check(&c, &rust, [lo, hi, mid]);
    }
    d.finish();
}

/// Row 6 — branch B with `r == b` (< g) ⇒ `h` exactly `120`.
#[test]
fn cfg_row06_branch_b_r_eq_b() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 6);
    let mut d = Diff::new("06 branch B r==b");
    for _ in 0..N {
        let hi = rng.unit();
        let lo = rng.unit() * hi;
        if lo >= hi {
            continue;
        }
        d.check(&c, &rust, [lo, hi, lo]);
    }
    d.finish();
}

/// Row 7 — branch C (`b == max`, `r != max`, `g != max`), `b > r > g`.
#[test]
fn cfg_row07_branch_c_b_gt_r_gt_g() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 7);
    let mut d = Diff::new("07 branch C b>r>g");
    for _ in 0..N {
        let (lo, mid, hi) = rng.three_sorted_unit();
        d.check(&c, &rust, [mid, lo, hi]);
    }
    d.finish();
}

/// Row 8 — branch C, `b > g > r`.
#[test]
fn cfg_row08_branch_c_b_gt_g_gt_r() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 8);
    let mut d = Diff::new("08 branch C b>g>r");
    for _ in 0..N {
        let (lo, mid, hi) = rng.three_sorted_unit();
        d.check(&c, &rust, [lo, mid, hi]);
    }
    d.finish();
}

/// Row 9 — branch C with `r == g` (< b) ⇒ `h` exactly `240`.
#[test]
fn cfg_row09_branch_c_r_eq_g() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 9);
    let mut d = Diff::new("09 branch C r==g");
    for _ in 0..N {
        let hi = rng.unit();
        let lo = rng.unit() * hi;
        if lo >= hi {
            continue;
        }
        d.check(&c, &rust, [lo, lo, hi]);
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 10-12: two-way ties at the maximum decide WHICH branch fires, because
// the C tests `r == max` first and `g == max` second.
// ---------------------------------------------------------------------------

/// Row 10 — `r == g == max > b` ⇒ branch A wins (`r == max` tested first).
#[test]
fn cfg_row10_tie_r_g_max() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 10);
    let mut d = Diff::new("10 tie r==g==max>b");
    for _ in 0..N {
        let hi = rng.unit();
        let lo = rng.unit() * hi;
        if lo >= hi {
            continue;
        }
        d.check(&c, &rust, [hi, hi, lo]);
    }
    d.finish();
}

/// Row 11 — `r == b == max > g` ⇒ branch A wins.
#[test]
fn cfg_row11_tie_r_b_max() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 11);
    let mut d = Diff::new("11 tie r==b==max>g");
    for _ in 0..N {
        let hi = rng.unit();
        let lo = rng.unit() * hi;
        if lo >= hi {
            continue;
        }
        d.check(&c, &rust, [hi, lo, hi]);
    }
    d.finish();
}

/// Row 12 — `g == b == max > r` ⇒ branch B wins (`r != max`, `g == max`).
#[test]
fn cfg_row12_tie_g_b_max() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 12);
    let mut d = Diff::new("12 tie g==b==max>r");
    for _ in 0..N {
        let hi = rng.unit();
        let lo = rng.unit() * hi;
        if lo >= hi {
            continue;
        }
        d.check(&c, &rust, [lo, hi, hi]);
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 13-15: the degenerate early-out (line 19) reached with VALID input,
// plus the signed-zero asymmetry of the min/max ternaries (line 13-16).
// ---------------------------------------------------------------------------

/// Row 13 — `delta == 0` with `max != 0`: valid grays `r == g == b`.
#[test]
fn cfg_row13_gray_delta_zero() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 13);
    let mut d = Diff::new("13 gray r==g==b!=0");
    for _ in 0..N {
        let g = rng.unit();
        if g == 0.0 {
            continue;
        }
        d.check(&c, &rust, [g, g, g]);
    }
    // exact 1.0 and values one ulp apart from each other (NOT equal -> not this row)
    for v in [1.0f32, 0.5, f32::MIN_POSITIVE, 255.0] {
        d.check(&c, &rust, [v, v, v]);
    }
    d.finish();
}

/// Row 14 — all channels exactly `+0.0` ⇒ both disjuncts true.
#[test]
fn cfg_row14_all_positive_zero() {
    let (c, rust) = both();
    let mut d = Diff::new("14 all +0.0");
    d.check(&c, &rust, [0.0, 0.0, 0.0]);
    d.finish();
}

/// Row 15 — all 8 sign combinations of `±0.0`.
///
/// This is the observable consequence of axis H4: the C ternaries are
/// `(a < b) ? a : b` / `(a > b) ? a : b`, and `+0.0 < -0.0` is FALSE, so on a
/// signed-zero tie the ternary yields the SECOND operand. The sign bit of `v`
/// therefore depends on argument order, and `f32::min`/`f32::max` would get it
/// wrong. A bit-exact comparison is required to see this.
#[test]
fn cfg_row15_signed_zero_combinations() {
    let (c, rust) = both();
    let mut d = Diff::new("15 signed-zero combos");
    const Z: [u32; 2] = [0x0000_0000, 0x8000_0000];
    for &r in &Z {
        for &g in &Z {
            for &b in &Z {
                d.check_raw(&c, &rust, [r, g, b]);
            }
        }
    }
    assert_eq!(d.checked, 8);
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 16-19: out-of-documented-range but perfectly valid float inputs.
// ---------------------------------------------------------------------------

/// Row 16 — negative channels with `max > 0` ⇒ `delta > max` ⇒ `s > 1`.
#[test]
fn cfg_row16_negative_with_positive_max() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 16);
    let mut d = Diff::new("16 negatives, max>0");
    for _ in 0..N {
        let r = rng.range(-1.0, 1.0);
        let g = rng.range(-1.0, 1.0);
        let b = rng.range(-1.0, 1.0);
        if r.max(g).max(b) <= 0.0 {
            continue;
        }
        d.check(&c, &rust, [r, g, b]);
    }
    d.finish();
}

/// Row 17 — `max == ±0.0` while `min < 0` ⇒ early-out SECOND disjunct.
/// The `||` short-circuit is what stops `delta / 0` from producing `Inf`.
#[test]
fn cfg_row17_max_zero_min_negative() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 17);
    let mut d = Diff::new("17 max==0, min<0");
    for _ in 0..N {
        let neg = -rng.range(1e-30, 10.0);
        let neg2 = -rng.range(1e-30, 10.0);
        // Place the zero in each slot, and try both zero signs.
        let z = if rng.below(2) == 0 { 0.0f32 } else { -0.0f32 };
        match rng.below(3) {
            0 => d.check(&c, &rust, [z, neg, neg2]),
            1 => d.check(&c, &rust, [neg, z, neg2]),
            _ => d.check(&c, &rust, [neg, neg2, z]),
        }
    }
    d.finish();
}

/// Row 18 — every channel negative (`max < 0`) ⇒ `s = delta/max` is negative.
#[test]
fn cfg_row18_all_negative() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 18);
    let mut d = Diff::new("18 all negative");
    for _ in 0..N {
        let r = -rng.range(1e-6, 4.0);
        let g = -rng.range(1e-6, 4.0);
        let b = -rng.range(1e-6, 4.0);
        d.check(&c, &rust, [r, g, b]);
    }
    d.finish();
}

/// Row 19 — `[0, 255]`-scaled integral channels: out of the documented range
/// and full of exact ties.
#[test]
fn cfg_row19_zero_to_255_integral() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 19);
    let mut d = Diff::new("19 [0,255] integral");
    for _ in 0..N {
        let r = rng.below(256) as f32;
        let g = rng.below(256) as f32;
        let b = rng.below(256) as f32;
        d.check(&c, &rust, [r, g, b]);
    }
    // Small-range integers maximise exact ties.
    for _ in 0..N {
        let r = rng.below(4) as f32;
        let g = rng.below(4) as f32;
        let b = rng.below(4) as f32;
        d.check(&c, &rust, [r, g, b]);
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 20-24: magnitude interactions - overflow, underflow, infinities.
// ---------------------------------------------------------------------------

/// Row 20 — huge magnitudes where `max - min` OVERFLOWS to `+Inf`
/// (e.g. `+FLT_MAX` together with `-FLT_MAX`), so `s = Inf/finite = Inf`
/// and the hue ratio becomes `finite/Inf = ±0`.
#[test]
fn cfg_row20_delta_overflows_to_inf() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 20);
    let mut d = Diff::new("20 delta overflow -> Inf");
    let big = f32::MAX;
    for _ in 0..N {
        let scale = rng.range(0.5, 1.0);
        let hi = big * scale;
        let lo = -big * rng.range(0.5, 1.0);
        let mid = rng.range(-1.0, 1.0) * big * 0.25;
        match rng.below(6) {
            0 => d.check(&c, &rust, [hi, mid, lo]),
            1 => d.check(&c, &rust, [hi, lo, mid]),
            2 => d.check(&c, &rust, [mid, hi, lo]),
            3 => d.check(&c, &rust, [lo, hi, mid]),
            4 => d.check(&c, &rust, [mid, lo, hi]),
            _ => d.check(&c, &rust, [lo, mid, hi]),
        }
    }
    // Exact extremes in every arrangement.
    let ext = [f32::MAX, -f32::MAX, 0.0f32];
    for &r in &ext {
        for &g in &ext {
            for &b in &ext {
                d.check(&c, &rust, [r, g, b]);
            }
        }
    }
    d.finish();
}

/// Row 21 — all channels subnormal.
#[test]
fn cfg_row21_all_subnormal() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 21);
    let mut d = Diff::new("21 all subnormal");
    for _ in 0..N {
        // Random subnormal magnitudes: mantissa in [0, 2^23), zero exponent.
        let mk = |rng: &mut Pcg32| -> u32 {
            let sign = (rng.below(2) as u32) << 31;
            sign | (rng.next_u32() & 0x007F_FFFF)
        };
        let r = mk(&mut rng);
        let g = mk(&mut rng);
        let b = mk(&mut rng);
        d.check_raw(&c, &rust, [r, g, b]);
    }
    d.finish();
}

/// Row 22 — subnormal `delta` next to a large `max` ⇒ `s` UNDERFLOWS
/// (to a subnormal or to `+0`) while the hue ratio stays `O(1)`.
#[test]
fn cfg_row22_delta_subnormal_max_large() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 22);
    let mut d = Diff::new("22 tiny delta, large max");
    for _ in 0..N {
        // Two values one/two ulps apart at a large exponent: delta is tiny
        // relative to max, so delta/max underflows.
        let base_exp = 100 + rng.below(28); // exponent field 100..128
        let base = f32::from_bits((base_exp << 23) | (rng.next_u32() & 0x007F_FFFF));
        let ulps = 1 + rng.below(4);
        let hi = f32::from_bits(base.to_bits() + ulps);
        let lo = base;
        match rng.below(3) {
            0 => d.check(&c, &rust, [hi, lo, lo]),
            1 => d.check(&c, &rust, [lo, hi, lo]),
            _ => d.check(&c, &rust, [lo, lo, hi]),
        }
    }
    d.finish();
}

/// Row 23 — subnormal `max` with subnormal `delta` ⇒ `delta / max` can be a
/// normal number or OVERFLOW to `+Inf`.
#[test]
fn cfg_row23_subnormal_max_ratio_overflow() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 23);
    let mut d = Diff::new("23 subnormal max, s overflow");
    let tiny = f32::from_bits(0x0000_0001); // FLT_TRUE_MIN
    for _ in 0..N {
        // max = k ulps of subnormal, min = -large  =>  delta/max overflows.
        let k = 1 + rng.below(8);
        let mx = f32::from_bits(k);
        let mn = -rng.range(1.0, f32::MAX / 4.0);
        match rng.below(3) {
            0 => d.check(&c, &rust, [mx, mn, mn]),
            1 => d.check(&c, &rust, [mn, mx, mn]),
            _ => d.check(&c, &rust, [mn, mn, mx]),
        }
    }
    // Ratios of adjacent subnormals.
    for _ in 0..N {
        let a = f32::from_bits(1 + rng.below(64));
        let b = f32::from_bits(1 + rng.below(64));
        d.check(&c, &rust, [tiny, a, b]);
    }
    d.finish();
}

/// Row 24 — infinities, including `+Inf` with `-Inf`
/// (`delta = Inf`, `s = Inf/Inf = NaN`, `Inf - Inf = NaN`).
#[test]
fn cfg_row24_infinities() {
    let (c, rust) = both();
    let mut d = Diff::new("24 infinities");
    let vals = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0f32,
        -0.0f32,
        1.0f32,
        -1.0f32,
        f32::MAX,
    ];
    for &r in &vals {
        for &g in &vals {
            for &b in &vals {
                if r.is_infinite() || g.is_infinite() || b.is_infinite() {
                    d.check(&c, &rust, [r, g, b]);
                }
            }
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 25-28: NaN placement. Because every `<`, `>` and `==` involving NaN is
// FALSE, the ternaries at lines 13-16 either DISCARD or ADOPT the NaN
// depending on which operand slot it occupies -- so each position is a
// genuinely different code path.
// ---------------------------------------------------------------------------

fn nan_bits() -> [u32; 5] {
    [
        0x7FC0_0000, // default quiet NaN
        0xFFC0_0000, // negative quiet NaN
        0x7F80_0001, // signalling NaN
        0xFFBF_FFFF, // negative signalling NaN
        0x7FD5_5555, // quiet NaN, custom payload
    ]
}

/// Row 25 — NaN in `r` only. `min = c_min(NaN, g) = g` and
/// `max = c_max(NaN, g) = g`: the NaN is DISCARDED, then `r == max` is false,
/// so branch B or C runs on clean numbers with a NaN only in the numerator.
#[test]
fn cfg_row25_nan_in_r() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 25);
    let mut d = Diff::new("25 NaN in r");
    for _ in 0..N {
        let n = *rng.pick(&nan_bits());
        let g = rng.range(-2.0, 2.0);
        let b = rng.range(-2.0, 2.0);
        d.check_raw(&c, &rust, [n, g.to_bits(), b.to_bits()]);
    }
    d.finish();
}

/// Row 26 — NaN in `g` only. `c_max(r, NaN)` is `NaN` (because `r > NaN` is
/// false), and the NEXT ternary `c_max(NaN, b)` then yields `b`. So `max = b`
/// unconditionally — a different path from row 25.
#[test]
fn cfg_row26_nan_in_g() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 26);
    let mut d = Diff::new("26 NaN in g");
    for _ in 0..N {
        let n = *rng.pick(&nan_bits());
        let r = rng.range(-2.0, 2.0);
        let b = rng.range(-2.0, 2.0);
        d.check_raw(&c, &rust, [r.to_bits(), n, b.to_bits()]);
    }
    d.finish();
}

/// Row 27 — NaN in `b` only: it is the LAST ternary operand, so it is adopted
/// and never displaced ⇒ `min = max = NaN`, `delta = NaN`, both `==` tests
/// false ⇒ **branch C** (the `else` at line 30) is the one that runs.
#[test]
fn cfg_row27_nan_in_b() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 27);
    let mut d = Diff::new("27 NaN in b");
    for _ in 0..N {
        let n = *rng.pick(&nan_bits());
        let r = rng.range(-2.0, 2.0);
        let g = rng.range(-2.0, 2.0);
        d.check_raw(&c, &rust, [r.to_bits(), g.to_bits(), n]);
    }
    d.finish();
}

/// Row 28 — NaN in 2 and in all 3 channels, across all NaN encodings.
/// Quieting of signalling NaNs and payload selection must be bit-identical.
#[test]
fn cfg_row28_nan_multiple() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 28);
    let mut d = Diff::new("28 NaN in 2 or 3 slots");
    let nans = nan_bits();
    // Exhaustive over all NaN encodings for each of the 3 "two-NaN" placements
    // and for the all-NaN case.
    for &n1 in &nans {
        for &n2 in &nans {
            let x = 0.25f32.to_bits();
            d.check_raw(&c, &rust, [n1, n2, x]);
            d.check_raw(&c, &rust, [n1, x, n2]);
            d.check_raw(&c, &rust, [x, n1, n2]);
            for &n3 in &nans {
                d.check_raw(&c, &rust, [n1, n2, n3]);
            }
        }
    }
    // Randomised NaN payloads (any exponent-max, non-zero mantissa pattern).
    for _ in 0..N {
        let mk = |rng: &mut Pcg32| -> u32 {
            let sign = (rng.below(2) as u32) << 31;
            let mant = 1 + (rng.next_u32() & 0x007F_FFFE);
            sign | 0x7F80_0000 | mant
        };
        let r = mk(&mut rng);
        let g = mk(&mut rng);
        let b = mk(&mut rng);
        d.check_raw(&c, &rust, [r, g, b]);
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 29-31: pointer aliasing. `src` is `const float*` but NOT `restrict`, so
// overlap is legal C. All three loads precede all stores (confirmed by
// objdump), which the Rust must reproduce.
// ---------------------------------------------------------------------------

fn alias_vectors(rng: &mut Pcg32, n: usize) -> Vec<[u32; 3]> {
    let mut v = Vec::with_capacity(n + SPECIALS.len());
    for _ in 0..n {
        v.push([
            rng.range(-1.5, 1.5).to_bits(),
            rng.range(-1.5, 1.5).to_bits(),
            rng.range(-1.5, 1.5).to_bits(),
        ]);
    }
    for _ in 0..n {
        v.push([
            *rng.pick(&SPECIALS),
            *rng.pick(&SPECIALS),
            *rng.pick(&SPECIALS),
        ]);
    }
    v
}

/// Row 29 — in-place: `dest == src`.
#[test]
fn cfg_row29_alias_dest_eq_src() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 29);
    let mut d = Diff::new("29 alias dest==src");
    for v in alias_vectors(&mut rng, N) {
        let mut buf_c = [f32::from_bits(v[0]), f32::from_bits(v[1]), f32::from_bits(v[2])];
        let mut buf_r = buf_c;
        unsafe {
            (c.f)(buf_c.as_mut_ptr(), buf_c.as_ptr());
            (rust.f)(buf_r.as_mut_ptr(), buf_r.as_ptr());
        }
        let bc: Vec<u32> = buf_c.iter().map(|x| x.to_bits()).collect();
        let br: Vec<u32> = buf_r.iter().map(|x| x.to_bits()).collect();
        d.check_outputs(format!("in-place src={v:#010x?}"), &bc, &br);
    }
    d.finish();
}

/// Row 30 — forward overlap: `dest == src + 1` inside a 4-float buffer.
/// Reads `buf[0..3]`, writes `buf[1..4]`.
#[test]
fn cfg_row30_alias_dest_eq_src_plus_1() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 30);
    let mut d = Diff::new("30 alias dest==src+1");
    for v in alias_vectors(&mut rng, N) {
        let init = [
            f32::from_bits(v[0]),
            f32::from_bits(v[1]),
            f32::from_bits(v[2]),
            f32::from_bits(CANARY),
        ];
        let mut bc = init;
        let mut br = init;
        unsafe {
            (c.f)(bc.as_mut_ptr().add(1), bc.as_ptr());
            (rust.f)(br.as_mut_ptr().add(1), br.as_ptr());
        }
        let oc: Vec<u32> = bc.iter().map(|x| x.to_bits()).collect();
        let or: Vec<u32> = br.iter().map(|x| x.to_bits()).collect();
        d.check_outputs(format!("dest=src+1 src={v:#010x?}"), &oc, &or);
    }
    d.finish();
}

/// Row 31 — backward overlap: `dest == src - 1` inside a 4-float buffer.
/// Reads `buf[1..4]`, writes `buf[0..3]`.
#[test]
fn cfg_row31_alias_dest_eq_src_minus_1() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 31);
    let mut d = Diff::new("31 alias dest==src-1");
    for v in alias_vectors(&mut rng, N) {
        let init = [
            f32::from_bits(CANARY),
            f32::from_bits(v[0]),
            f32::from_bits(v[1]),
            f32::from_bits(v[2]),
        ];
        let mut bc = init;
        let mut br = init;
        unsafe {
            (c.f)(bc.as_mut_ptr(), bc.as_ptr().add(1));
            (rust.f)(br.as_mut_ptr(), br.as_ptr().add(1));
        }
        let oc: Vec<u32> = bc.iter().map(|x| x.to_bits()).collect();
        let or: Vec<u32> = br.iter().map(|x| x.to_bits()).collect();
        d.check_outputs(format!("dest=src-1 src={v:#010x?}"), &oc, &or);
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 32-35: broad property sweeps.
// ---------------------------------------------------------------------------

/// Row 32 — 20 000 uniform random vectors in `[0,1]^3` (the documented range).
#[test]
fn cfg_row32_property_unit_cube() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 32);
    let mut d = Diff::new("32 property [0,1]^3");
    for _ in 0..20_000 {
        d.check(&c, &rust, [rng.unit(), rng.unit(), rng.unit()]);
    }
    d.finish();
}

/// Row 33 — 50 000 vectors of UNIFORM RANDOM 32-BIT PATTERNS. This covers
/// every IEEE class simultaneously (normals of every exponent, subnormals,
/// `±0`, `±Inf`, NaNs with arbitrary payloads) and is the strongest single
/// check that no value-dependent path diverges.
#[test]
fn cfg_row33_property_raw_bit_patterns() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 33);
    let mut d = Diff::new("33 property raw bit patterns");
    for _ in 0..50_000 {
        d.check_raw(&c, &rust, [rng.next_u32(), rng.next_u32(), rng.next_u32()]);
    }
    assert_eq!(d.checked, 50_000);
    d.finish();
}

/// Row 34 — channels from the small grid `{-2,-1,0,1,2}/2`, producing dense
/// exact ties, exact zeros and exact `delta == 0` hits.
#[test]
fn cfg_row34_property_integer_grid() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 34);
    let mut d = Diff::new("34 property small grid");
    let grid = [-1.0f32, -0.5, -0.0, 0.0, 0.5, 1.0];
    for _ in 0..8_000 {
        d.check(
            &c,
            &rust,
            [*rng.pick(&grid), *rng.pick(&grid), *rng.pick(&grid)],
        );
    }
    // Exhaustive over the grid as well (6^3 = 216).
    for &r in &grid {
        for &g in &grid {
            for &b in &grid {
                d.check(&c, &rust, [r, g, b]);
            }
        }
    }
    d.finish();
}

/// Row 35 — EXHAUSTIVE cross product of the 24 curated special values over
/// `(r, g, b)` = 13 824 vectors. Guarantees every combination of IEEE class,
/// tie multiplicity and sign is visited at least once.
#[test]
fn cfg_row35_exhaustive_specials() {
    let (c, rust) = both();
    let mut d = Diff::new("35 exhaustive specials 24^3");
    for &r in &SPECIALS {
        for &g in &SPECIALS {
            for &b in &SPECIALS {
                d.check_raw(&c, &rust, [r, g, b]);
            }
        }
    }
    assert_eq!(d.checked, 24 * 24 * 24);
    d.finish();
}

/// Row 36 — statelessness: replay a vector after 1 000 interleaved random
/// calls and require identical bits. Confirms neither library carries hidden
/// global state (the C has no globals; the Rust must not introduce any).
#[test]
fn cfg_row36_statelessness() {
    let (c, rust) = both();
    let mut rng = Pcg32::new(SEED ^ 36);
    let mut d = Diff::new("36 statelessness / replay");
    let probe = [0.3f32, 0.7, 0.1];
    let first_c = call_bits(c.f, &probe);
    let first_r = call_bits(rust.f, &probe);
    d.check_outputs("initial probe".into(), &first_c, &first_r);
    for _ in 0..1_000 {
        let junk = [rng.next_u32(), rng.next_u32(), rng.next_u32()];
        let _ = call_bits_raw(c.f, &junk);
        let _ = call_bits_raw(rust.f, &junk);
    }
    let again_c = call_bits(c.f, &probe);
    let again_r = call_bits(rust.f, &probe);
    assert_eq!(first_c, again_c, "C is not stateless?!");
    assert_eq!(
        first_r, again_r,
        "Rust drifted across calls -> hidden mutable state"
    );
    d.check_outputs("replayed probe".into(), &again_c, &again_r);
    d.finish();
}
