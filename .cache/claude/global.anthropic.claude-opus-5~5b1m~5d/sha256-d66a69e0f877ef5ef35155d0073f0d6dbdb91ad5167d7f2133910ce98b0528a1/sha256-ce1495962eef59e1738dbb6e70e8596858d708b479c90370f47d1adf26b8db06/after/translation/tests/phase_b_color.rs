//! Phase B — valid-path differential tests for the colour-space conversions
//! `f11` (HSL→RGB), `f12` (HSV→RGB) and `f13` (RGB→HSV).
//!
//! Covers `CONFIGS.md` rows C35 … C58. Every one of `f11`'s seven hue arms,
//! `f12`'s six `switch` arms and `f13`'s three max-channel branches gets its
//! own row driven with randomized inputs.

mod common;

use common::*;

const N: usize = 20_000;

fn chk(p: &Pair, which: u8, src: [f32; 3], tag: &str) {
    let (cf, rf) = match which {
        11 => (p.c.f11, p.rs.f11),
        12 => (p.c.f12, p.rs.f12),
        _ => (p.c.f13, p.rs.f13),
    };
    same(tag, src.map(f32::to_bits), call_f1x(cf, src), call_f1x(rf, src));
}

// ---------------------------------------------------------------------------
// f11 — HSL to RGB
// ---------------------------------------------------------------------------

#[test]
fn c35_f11_saturation_zero() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x35);
    for &s in &[0.0f32, -0.0] {
        for &hb in SPECIAL_F32 {
            for &lb in SPECIAL_F32 {
                chk(
                    p,
                    11,
                    [f32::from_bits(hb), s, f32::from_bits(lb)],
                    "f11/s==0",
                );
            }
        }
    }
    for _ in 0..N {
        let s = if r.next_u32() & 1 == 0 { 0.0 } else { -0.0 };
        chk(p, 11, [r.nice_f32(720.0), s, r.nice_f32(2.0)], "f11/s==0-rand");
    }
}

/// Drive `f11` with `h` confined to one hue sector.
fn f11_sector(seed: u64, lo: f32, hi: f32, tag: &str) {
    let p = pair();
    let mut r = Rng::new(seed);
    // sector endpoints and just-inside/just-outside neighbours
    for h in [
        lo,
        f32::from_bits(lo.to_bits().wrapping_add(1)),
        f32::from_bits(lo.to_bits().wrapping_sub(1)),
        (lo + hi) * 0.5,
        f32::from_bits(hi.to_bits().wrapping_sub(1)),
        hi,
    ] {
        for &s in &[1.0f32, 0.5, 0.25, -1.0, 2.0, 1e-30, f32::MIN_POSITIVE] {
            for &l in &[0.0f32, 0.25, 0.5, 0.75, 1.0, -1.0, 2.0, 1e30] {
                chk(p, 11, [h, s, l], tag);
            }
        }
    }
    for _ in 0..N {
        let h = r.range_f32(lo, hi);
        // s must be non-zero to reach the sector arms; keep l unconstrained
        let mut s = r.finite_f32(2.0);
        if s == 0.0 {
            s = 1.0;
        }
        chk(p, 11, [h, s, r.nice_f32(2.0)], tag);
    }
    // in-sector hue with special s / l
    for _ in 0..N / 2 {
        let h = r.range_f32(lo, hi);
        let s = r.nice_f32(2.0);
        let l = r.nice_f32(2.0);
        chk(p, 11, [h, s, l], tag);
    }
}

#[test]
fn c36_f11_sector_0_60() {
    f11_sector(SEED ^ 0x36, 0.0, 60.0, "f11/[0,60)");
}

#[test]
fn c37_f11_sector_60_120() {
    f11_sector(SEED ^ 0x37, 60.0, 120.0, "f11/[60,120)");
}

#[test]
fn c38_f11_third_arm_including_negative_hue() {
    // The C's third test is `h < 120.0f && h < 180.0f`, so it also catches
    // every h < 0 — that quirk must be reproduced, not fixed.
    f11_sector(SEED ^ 0x38, 120.0, 180.0, "f11/[120,180)");
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x138);
    for h in [
        -0.0f32,
        -1e-45,
        -1.0,
        -60.0,
        -120.0,
        -180.0,
        -360.0,
        -1e30,
        f32::NEG_INFINITY,
        f32::MIN,
    ] {
        for &s in &[1.0f32, 0.5, -1.0, 2.0] {
            for &l in &[0.0f32, 0.5, 1.0, -1.0, 2.0] {
                chk(p, 11, [h, s, l], "f11/negative-h");
            }
        }
    }
    for _ in 0..N {
        let h = -r.range_f32(0.0, 1e6);
        let mut s = r.finite_f32(2.0);
        if s == 0.0 {
            s = 1.0;
        }
        chk(p, 11, [h, s, r.nice_f32(2.0)], "f11/negative-h-rand");
    }
}

#[test]
fn c39_f11_sector_180_240() {
    f11_sector(SEED ^ 0x39, 180.0, 240.0, "f11/[180,240)");
}

#[test]
fn c40_f11_sector_240_300() {
    f11_sector(SEED ^ 0x40, 240.0, 300.0, "f11/[240,300)");
}

#[test]
fn c41_f11_sector_300_360() {
    f11_sector(SEED ^ 0x41, 300.0, 360.0, "f11/[300,360)");
}

#[test]
fn c42_f11_final_else_arm() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x42);
    // reachable only for h >= 360 or h NaN
    for h in [
        360.0f32,
        f32::from_bits(360.0f32.to_bits() + 1),
        361.0,
        720.0,
        1e30,
        f32::MAX,
        f32::INFINITY,
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFFFF_FFFF),
    ] {
        for &s in &[1.0f32, 0.5, -1.0, 2.0, f32::NAN, f32::INFINITY] {
            for &l in &[0.0f32, 0.5, 1.0, -1.0, f32::NAN, f32::INFINITY] {
                chk(p, 11, [h, s, l], "f11/else");
            }
        }
    }
    for _ in 0..N {
        let h = 360.0 + r.range_f32(0.0, 1e6);
        let mut s = r.finite_f32(2.0);
        if s == 0.0 {
            s = 1.0;
        }
        chk(p, 11, [h, s, r.nice_f32(2.0)], "f11/else-rand");
    }
}

#[test]
fn c43_f11_raw_bit_patterns() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x43);
    for _ in 0..N {
        chk(p, 11, [r.raw_f32(), r.raw_f32(), r.raw_f32()], "f11/raw");
    }
    for _ in 0..N {
        chk(
            p,
            11,
            [r.nice_f32(400.0), r.nice_f32(2.0), r.nice_f32(2.0)],
            "f11/nice",
        );
    }
    // exhaustive over the special corpus for (s, l) with a hue in each sector
    for &hs in &[30.0f32, 90.0, 150.0, 210.0, 270.0, 330.0, 400.0, -30.0] {
        for &sb in SPECIAL_F32 {
            for &lb in SPECIAL_F32 {
                chk(
                    p,
                    11,
                    [hs, f32::from_bits(sb), f32::from_bits(lb)],
                    "f11/special-sl",
                );
            }
        }
    }
    // and over h with fixed s/l
    for &hb in SPECIAL_F32 {
        for &s in &[1.0f32, 0.5, -1.0, 2.0] {
            for &l in &[0.5f32, 0.0, 1.0, -1.0] {
                chk(p, 11, [f32::from_bits(hb), s, l], "f11/special-h");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// f12 — HSV to RGB
// ---------------------------------------------------------------------------

#[test]
fn c44_f12_saturation_zero() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x44);
    for &s in &[0.0f32, -0.0] {
        for &hb in SPECIAL_F32 {
            for &vb in SPECIAL_F32 {
                chk(
                    p,
                    12,
                    [f32::from_bits(hb), s, f32::from_bits(vb)],
                    "f12/s==0",
                );
            }
        }
    }
    for _ in 0..N {
        let s = if r.next_u32() & 1 == 0 { 0.0 } else { -0.0 };
        chk(p, 12, [r.nice_f32(720.0), s, r.nice_f32(2.0)], "f12/s==0-rand");
    }
}

/// Drive `f12` so `i = (int)floorf(h/60)` equals `want`.
fn f12_case(seed: u64, lo: f32, hi: f32, tag: &str) {
    let p = pair();
    let mut r = Rng::new(seed);
    for h in [
        lo,
        f32::from_bits(lo.to_bits().wrapping_add(1)),
        f32::from_bits(lo.to_bits().wrapping_sub(1)),
        (lo + hi) * 0.5,
        f32::from_bits(hi.to_bits().wrapping_sub(1)),
        hi,
    ] {
        for &s in &[1.0f32, 0.5, 0.25, -1.0, 2.0, f32::MIN_POSITIVE] {
            for &v in &[0.0f32, 0.25, 1.0, -1.0, 2.0, 1e30] {
                chk(p, 12, [h, s, v], tag);
            }
        }
    }
    for _ in 0..N {
        let h = r.range_f32(lo, hi);
        let mut s = r.finite_f32(2.0);
        if s == 0.0 {
            s = 1.0;
        }
        chk(p, 12, [h, s, r.nice_f32(2.0)], tag);
    }
    for _ in 0..N / 2 {
        let h = r.range_f32(lo, hi);
        chk(p, 12, [h, r.nice_f32(2.0), r.nice_f32(2.0)], tag);
    }
}

#[test]
fn c45_f12_case_0() {
    f12_case(SEED ^ 0x45, 0.0, 60.0, "f12/i==0");
}

#[test]
fn c46_f12_case_1() {
    f12_case(SEED ^ 0x46, 60.0, 120.0, "f12/i==1");
}

#[test]
fn c47_f12_case_2() {
    f12_case(SEED ^ 0x47, 120.0, 180.0, "f12/i==2");
}

#[test]
fn c48_f12_case_3() {
    f12_case(SEED ^ 0x48, 180.0, 240.0, "f12/i==3");
}

#[test]
fn c49_f12_case_4() {
    f12_case(SEED ^ 0x49, 240.0, 300.0, "f12/i==4");
}

#[test]
fn c50_f12_default_arm() {
    // i == 5, i > 5 and i < 0 all land in `default:` (the C compares
    // `cmpl $0x4, i; ja` — an *unsigned* comparison).
    f12_case(SEED ^ 0x50, 300.0, 360.0, "f12/i==5");
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x150);
    for h in [
        360.0f32, 420.0, 600.0, 1e6, 1e30, f32::MAX, f32::INFINITY, -1e-30, -1.0, -60.0, -1e6,
        -1e30, f32::MIN, f32::NEG_INFINITY,
    ] {
        for &s in &[1.0f32, 0.5, -1.0, 2.0] {
            for &v in &[0.0f32, 0.5, 1.0, -1.0, 2.0] {
                chk(p, 12, [h, s, v], "f12/default");
            }
        }
    }
    // exact multiples of 60 pin down `floorf` at the integer boundary
    for k in -20..=20i32 {
        let h = 60.0f32 * k as f32;
        for d in [-1i32, 0, 1] {
            let hh = if d == 0 {
                h
            } else if d < 0 {
                f32::from_bits(h.to_bits().wrapping_sub(1))
            } else {
                f32::from_bits(h.to_bits().wrapping_add(1))
            };
            for &s in &[1.0f32, 0.5, -1.0] {
                chk(p, 12, [hh, s, 0.75], "f12/60k-boundary");
            }
        }
    }
    // huge h/60 -> (int) conversion is undefined in C; x86 yields INT_MIN
    for _ in 0..N {
        let h = if r.next_u32() & 1 == 0 {
            r.range_f32(3e10, 1e38)
        } else {
            -r.range_f32(3e10, 1e38)
        };
        let mut s = r.finite_f32(2.0);
        if s == 0.0 {
            s = 1.0;
        }
        chk(p, 12, [h, s, r.nice_f32(2.0)], "f12/huge-h");
    }
    // NaN h -> cvttss2si gives INT_MIN -> default
    for &nb in &[0x7FC0_0000u32, 0xFFC0_0000, 0x7F80_0001, 0xFFFF_FFFF, 0x7FFF_FFFF] {
        for &s in &[1.0f32, -1.0, 0.5] {
            for &v in &[0.5f32, -0.5, f32::INFINITY] {
                chk(p, 12, [f32::from_bits(nb), s, v], "f12/nan-h");
            }
        }
    }
}

#[test]
fn c51_f12_raw_bit_patterns() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x51);
    for _ in 0..N {
        chk(p, 12, [r.raw_f32(), r.raw_f32(), r.raw_f32()], "f12/raw");
    }
    for _ in 0..N {
        chk(
            p,
            12,
            [r.nice_f32(400.0), r.nice_f32(2.0), r.nice_f32(2.0)],
            "f12/nice",
        );
    }
    for &hs in &[30.0f32, 90.0, 150.0, 210.0, 270.0, 330.0, 400.0, -30.0] {
        for &sb in SPECIAL_F32 {
            for &vb in SPECIAL_F32 {
                chk(
                    p,
                    12,
                    [hs, f32::from_bits(sb), f32::from_bits(vb)],
                    "f12/special-sv",
                );
            }
        }
    }
    for &hb in SPECIAL_F32 {
        for &s in &[1.0f32, 0.5, -1.0, 2.0] {
            for &v in &[0.5f32, 0.0, 1.0, -1.0] {
                chk(p, 12, [f32::from_bits(hb), s, v], "f12/special-h");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// f13 — RGB to HSV
// ---------------------------------------------------------------------------

#[test]
fn c52_f13_r_is_max() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x52);
    // r strictly greatest; g >= b (no wrap) and g < b (h += 360 wrap)
    for _ in 0..N {
        let rr = r.range_f32(0.5, 1.0);
        let g = r.range_f32(0.0, 0.5);
        let b = r.range_f32(0.0, 0.5);
        chk(p, 13, [rr, g, b], "f13/r-max");
    }
    for _ in 0..N {
        let rr = r.range_f32(0.5, 1.0);
        let b = r.range_f32(0.0, 0.5);
        let g = r.range_f32(0.0, b.max(f32::MIN_POSITIVE));
        // g < b guarantees (g-b)/delta < 0 -> h < 0 -> the += 360 fix-up
        chk(p, 13, [rr, g, b], "f13/r-max-wrap");
    }
    for &(rr, g, b) in &[
        (1.0f32, 0.0, 0.0),
        (1.0, 1.0, 0.0),
        (1.0, 0.0, 1.0),
        (1.0, 0.5, 0.25),
        (1.0, 0.25, 0.5),
        (2.0, -1.0, -2.0),
        (1e30, 1.0, 0.0),
        (f32::MAX, f32::MIN, 0.0),
        (f32::MIN_POSITIVE, 0.0, 0.0),
        (f32::from_bits(3), f32::from_bits(1), 0.0),
    ] {
        chk(p, 13, [rr, g, b], "f13/r-max-struct");
    }
}

#[test]
fn c53_f13_g_is_max() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x53);
    for _ in 0..N {
        let g = r.range_f32(0.5, 1.0);
        let rr = r.range_f32(0.0, 0.5);
        let b = r.range_f32(0.0, 0.5);
        chk(p, 13, [rr, g, b], "f13/g-max");
    }
    for &(rr, g, b) in &[
        (0.0f32, 1.0, 0.0),
        (0.0, 1.0, 1.0),
        (1.0, 2.0, 0.0),
        (0.25, 1.0, 0.5),
        (-1.0, 2.0, -2.0),
        (1.0, 1e30, 0.0),
    ] {
        chk(p, 13, [rr, g, b], "f13/g-max-struct");
    }
}

#[test]
fn c54_f13_b_is_max() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x54);
    for _ in 0..N {
        let b = r.range_f32(0.5, 1.0);
        let rr = r.range_f32(0.0, 0.5);
        let g = r.range_f32(0.0, 0.5);
        chk(p, 13, [rr, g, b], "f13/b-max");
    }
    for &(rr, g, b) in &[
        (0.0f32, 0.0, 1.0),
        (1.0, 0.0, 2.0),
        (0.5, 0.25, 1.0),
        (-1.0, -2.0, 2.0),
        (1.0, 0.0, 1e30),
    ] {
        chk(p, 13, [rr, g, b], "f13/b-max-struct");
    }
}

#[test]
fn c55_f13_ties_and_delta_zero() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x55);
    // the C's branch order is r, then g, then b — ties must resolve the same way
    for _ in 0..N {
        let hi = r.range_f32(0.25, 1.0);
        let lo = r.range_f32(-1.0, 0.25);
        for t in [
            [hi, hi, lo], // r == g > b
            [lo, hi, hi], // g == b > r
            [hi, lo, hi], // r == b > g
            [hi, hi, hi], // all equal -> delta == 0
        ] {
            chk(p, 13, t, "f13/tie");
        }
    }
    // exact equality with signed zeros
    for &a in &[0.0f32, -0.0] {
        for &b in &[0.0f32, -0.0] {
            for &c in &[0.0f32, -0.0] {
                chk(p, 13, [a, b, c], "f13/zero-tie");
            }
        }
    }
    for &v in &[1.0f32, -1.0, f32::MAX, f32::MIN, f32::MIN_POSITIVE, f32::INFINITY] {
        chk(p, 13, [v, v, v], "f13/all-equal");
    }
}

#[test]
fn c56_f13_negative_and_extreme() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x56);
    // max == 0 is reachable only with a negative channel
    for &t in &[
        [0.0f32, 0.0, -1.0],
        [0.0, -1.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, -1.0, -2.0],
        [-0.0, -1.0, -2.0],
        [-1.0, -2.0, -3.0],
        [-3.0, -2.0, -1.0],
        [f32::MIN, f32::MIN, 0.0],
    ] {
        chk(p, 13, t, "f13/max<=0");
    }
    // all channels negative -> negative max and negative s
    for _ in 0..N {
        let t = [
            -r.range_f32(0.0, 10.0),
            -r.range_f32(0.0, 10.0),
            -r.range_f32(0.0, 10.0),
        ];
        chk(p, 13, t, "f13/all-negative");
    }
    // subnormal delta with a large max -> s underflows
    for _ in 0..N {
        let m = r.range_f32(1e20, 1e30);
        let d = f32::from_bits(r.next_u32() % 1024 + 1);
        chk(p, 13, [m, m - d, m], "f13/subnormal-delta");
        chk(p, 13, [m, m, m - d], "f13/subnormal-delta2");
    }
    // huge spread -> delta overflows / s overflows
    for &t in &[
        [f32::MAX, f32::MIN, 0.0f32],
        [f32::MAX, -f32::MAX, f32::MAX],
        [f32::MIN_POSITIVE, f32::MAX, 0.0],
        [f32::from_bits(1), f32::MAX, -f32::MAX],
    ] {
        chk(p, 13, t, "f13/huge-spread");
    }
}

#[test]
fn c57_f13_raw_bit_patterns() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x57);
    for _ in 0..N {
        chk(p, 13, [r.raw_f32(), r.raw_f32(), r.raw_f32()], "f13/raw");
    }
    for _ in 0..N {
        chk(p, 13, [r.nice_f32(2.0), r.nice_f32(2.0), r.nice_f32(2.0)], "f13/nice");
    }
    // exhaustive over the whole special corpus (28^3 = 21952) — this is where
    // the NaN-never-displaces-the-incumbent min/max quirk shows up
    for &a in SPECIAL_F32 {
        for &b in SPECIAL_F32 {
            for &c in SPECIAL_F32 {
                chk(
                    p,
                    13,
                    [f32::from_bits(a), f32::from_bits(b), f32::from_bits(c)],
                    "f13/special-cube",
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C58 — aliasing and unaligned placement
// ---------------------------------------------------------------------------

#[test]
fn c58_aliasing_and_offsets() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x58);

    for _ in 0..N {
        let src = [r.nice_f32(400.0), r.nice_f32(2.0), r.nice_f32(2.0)];
        for which in [11u8, 12, 13] {
            let (cf, rf) = match which {
                11 => (p.c.f11, p.rs.f11),
                12 => (p.c.f12, p.rs.f12),
                _ => (p.c.f13, p.rs.f13),
            };
            same(
                "f1x/aliased",
                (which, src.map(f32::to_bits)),
                call_f1x_aliased(cf, src),
                call_f1x_aliased(rf, src),
            );
        }
    }

    // `dest` and `src` as interior slices of a bigger buffer, at every offset
    for _ in 0..2_000 {
        let base: [f32; 8] = [
            r.nice_f32(400.0),
            r.nice_f32(2.0),
            r.nice_f32(2.0),
            r.nice_f32(400.0),
            r.nice_f32(2.0),
            r.nice_f32(2.0),
            r.nice_f32(2.0),
            r.nice_f32(2.0),
        ];
        for off_d in 0..3usize {
            for off_s in 0..3usize {
                for which in [11u8, 12, 13] {
                    let (cf, rf) = match which {
                        11 => (p.c.f11, p.rs.f11),
                        12 => (p.c.f12, p.rs.f12),
                        _ => (p.c.f13, p.rs.f13),
                    };
                    let mut bc = base;
                    let mut br = base;
                    unsafe {
                        cf(bc.as_mut_ptr().add(off_d), bc.as_ptr().add(off_s));
                        rf(br.as_mut_ptr().add(off_d), br.as_ptr().add(off_s));
                    }
                    same(
                        "f1x/overlapping",
                        (which, off_d, off_s, base.map(f32::to_bits)),
                        (bc[0].to_bits(), bc[1].to_bits(), bc[2].to_bits()),
                        (br[0].to_bits(), br[1].to_bits(), br[2].to_bits()),
                    );
                    assert_eq!(
                        bc.map(f32::to_bits),
                        br.map(f32::to_bits),
                        "full buffer mismatch f{which} off_d={off_d} off_s={off_s}"
                    );
                }
            }
        }
    }
}
