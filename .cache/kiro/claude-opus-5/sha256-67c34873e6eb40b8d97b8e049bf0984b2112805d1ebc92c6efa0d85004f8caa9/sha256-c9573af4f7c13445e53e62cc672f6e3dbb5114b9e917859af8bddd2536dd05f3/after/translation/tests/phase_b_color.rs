//! Phase B — valid-path differential tests for the three colour conversions
//! `f11` (HSL->RGB), `f12` (HSV->RGB), `f13` (RGB->HSV).
//! CONFIGS.md rows C56-C90.
//!
//! These are the rows that also cover the `fmodf` / `floorf` question: the C
//! `.so` calls glibc's, while the Rust `.so` statically links
//! `compiler_builtins`' implementations.

mod common;

use common::*;

macro_rules! bind {
    ($l:expr, $name:expr, $ty:ty) => {{
        let c: libloading::Symbol<$ty> = $l.c.get($name);
        let r: libloading::Symbol<$ty> = $l.r.get($name);
        (c, r)
    }};
}

const N: usize = 6000;

/// Call `name` in both libraries with the same 3-float input and compare the
/// 3-float output bit-for-bit. `dest` is pre-filled with a sentinel so a
/// missing write is detected too.
#[track_caller]
fn diff3(name: &str, tag: &str, src: [f32; 3]) {
    let l = libs();
    let (c, r) = bind!(l, name, FnTriple);
    const SENTINEL: f32 = -1234.5678;
    let mut dc = [SENTINEL; 3];
    let mut dr = [SENTINEL; 3];
    unsafe {
        c(dc.as_mut_ptr(), src.as_ptr());
        r(dr.as_mut_ptr(), src.as_ptr());
    }
    eq_triple(
        &format!(
            "{tag} {name}(src=[0x{:08x},0x{:08x},0x{:08x}] = {src:?})",
            src[0].to_bits(),
            src[1].to_bits(),
            src[2].to_bits()
        ),
        dc,
        dr,
    );
}

/// `dest` aliases `src`: both libraries get their own copy of the same buffer.
#[track_caller]
fn diff3_aliased(name: &str, tag: &str, src: [f32; 3]) {
    let l = libs();
    let (c, r) = bind!(l, name, FnTriple);
    let mut bc = src;
    let mut br = src;
    unsafe {
        c(bc.as_mut_ptr(), bc.as_ptr());
        r(br.as_mut_ptr(), br.as_ptr());
    }
    eq_triple(&format!("{tag} {name} aliased(src={src:?})"), bc, br);
}

fn sweep(name: &'static str, tag: &'static str, n: usize, gen: impl Fn(&mut Rng) -> [f32; 3]) {
    let mut g = Rng::seeded();
    for i in 0..n {
        diff3(name, &format!("{tag} #{i}"), gen(&mut g));
    }
}

// ===========================================================================
// f11 — HSL to RGB. Rows C56-C67.
// ===========================================================================

#[test]
fn c56_f11_saturation_zero_early_out() {
    let mut g = Rng::seeded();
    for i in 0..N {
        for &s in &[0.0f32, -0.0f32] {
            diff3("f11", &format!("C56 s={s} #{i}"), [g.mixed_f32(), s, g.mixed_f32()]);
            diff3(
                "f11",
                &format!("C56 tame s={s} #{i}"),
                [g.range_f32(-720.0, 720.0), s, g.range_f32(-2.0, 2.0)],
            );
        }
    }
}

/// One test per hue sector (rows C57-C62) plus the out-of-range sector (C63).
fn hue_sector(tag: &'static str, lo: f32, hi: f32) {
    let mut g = Rng::seeded();
    for i in 0..N {
        let h = g.range_f32(lo, hi);
        let s = g.range_f32(0.0001, 1.0);
        let ll = g.range_f32(0.0, 1.0);
        diff3("f11", &format!("{tag} #{i}"), [h, s, ll]);
        // s and l outside [0,1] as well — the C never clamps
        diff3(
            "f11",
            &format!("{tag} wild #{i}"),
            [h, g.range_f32(-3.0, 3.0), g.range_f32(-3.0, 3.0)],
        );
    }
    // the exact sector endpoints and their neighbours
    for &b in &[lo, hi] {
        for &h in &[
            b,
            f32::from_bits(b.to_bits().wrapping_sub(1)),
            f32::from_bits(b.to_bits().wrapping_add(1)),
        ] {
            for &s in &[0.25f32, 1.0, 2.0, -1.0] {
                for &ll in &[0.0f32, 0.5, 1.0, -0.5, 1.5] {
                    diff3("f11", &format!("{tag} boundary h={h}"), [h, s, ll]);
                }
            }
        }
    }
}

#[test]
fn c57_f11_hue_0_60() {
    hue_sector("C57", 0.0, 60.0);
    // -0.0 passes `h >= 0.0f` and must take the FIRST branch
    for &s in &[0.5f32, 1.0, -1.0] {
        for &ll in &[0.0f32, 0.3, 1.0] {
            diff3("f11", "C57 h=-0.0", [-0.0, s, ll]);
            diff3("f11", "C57 h=+0.0", [0.0, s, ll]);
        }
    }
}

#[test]
fn c58_f11_hue_60_120() {
    hue_sector("C58", 60.0, 120.0);
}

#[test]
fn c59_f11_hue_120_180_falls_through() {
    // The C reads `h < 120.0f && h < 180.0f`, so [120,180) reaches the final
    // `else` instead of the intended third branch.
    hue_sector("C59", 120.0, 180.0);
}

#[test]
fn c60_f11_hue_180_240() {
    hue_sector("C60", 180.0, 240.0);
}

#[test]
fn c61_f11_hue_240_300() {
    hue_sector("C61", 240.0, 300.0);
}

#[test]
fn c62_f11_hue_300_360() {
    hue_sector("C62", 300.0, 360.0);
}

#[test]
fn c63_f11_hue_out_of_range() {
    hue_sector("C63 neg", -360.0, -0.0001);
    hue_sector("C63 over", 360.0, 3600.0);
    let mut g = Rng::seeded();
    // huge hues feed fmodf with large arguments
    for i in 0..N {
        for &h in &[1e7f32, -1e7, 1e20, -1e20, 1e30, f32::MAX, f32::MIN] {
            diff3(
                "f11",
                &format!("C63 huge h={h} #{i}"),
                [h, g.range_f32(0.001, 1.0), g.range_f32(0.0, 1.0)],
            );
        }
    }
}

#[test]
fn c64_f11_unclamped_s_and_l() {
    sweep("f11", "C64", N * 2, |g| {
        [
            g.range_f32(-720.0, 720.0),
            g.range_f32(-10.0, 10.0),
            g.range_f32(-10.0, 10.0),
        ]
    });
}

#[test]
fn c65_f11_nan_and_inf() {
    let sp = special_f32s();
    for &h in &sp {
        for &s in &sp {
            for &ll in &[0.5f32, 0.0, 1.0, f32::NAN, f32::INFINITY, -0.25] {
                diff3("f11", "C65", [h, s, ll]);
            }
        }
    }
    // NaN in the l slot, driving `c` and `m` NaN with distinct payloads
    let nans = [
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0x7FAB_CDEF),
        f32::from_bits(0xFFD5_5555),
    ];
    for &a in &nans {
        for &b in &nans {
            for &h in &[0.0f32, 30.0, 90.0, 150.0, 210.0, 270.0, 330.0, 400.0, f32::NAN] {
                diff3("f11", "C65 dual-nan", [h, a, b]);
                diff3("f11", "C65 dual-nan h", [a, b, 0.5]);
            }
        }
    }
}

#[test]
fn c66_f11_fully_random() {
    sweep("f11", "C66 anybits", N * 4, |g| {
        [g.any_f32(), g.any_f32(), g.any_f32()]
    });
    sweep("f11", "C66 mixed", N * 4, |g| {
        [g.mixed_f32(), g.mixed_f32(), g.mixed_f32()]
    });
}

#[test]
fn c67_f11_dest_aliases_src() {
    let mut g = Rng::seeded();
    for i in 0..N {
        diff3_aliased(
            "f11",
            &format!("C67 #{i}"),
            [g.range_f32(-400.0, 760.0), g.range_f32(-1.0, 2.0), g.range_f32(-1.0, 2.0)],
        );
        diff3_aliased(
            "f11",
            &format!("C67 mixed #{i}"),
            [g.mixed_f32(), g.mixed_f32(), g.mixed_f32()],
        );
    }
}

// ===========================================================================
// f12 — HSV to RGB. Rows C68-C79.
// ===========================================================================

#[test]
fn c68_f12_saturation_zero_early_out() {
    let mut g = Rng::seeded();
    for i in 0..N {
        for &s in &[0.0f32, -0.0f32] {
            diff3("f12", &format!("C68 s={s} #{i}"), [g.mixed_f32(), s, g.mixed_f32()]);
            diff3(
                "f12",
                &format!("C68 tame s={s} #{i}"),
                [g.range_f32(-720.0, 720.0), s, g.range_f32(-2.0, 2.0)],
            );
        }
    }
}

fn f12_sector(tag: &'static str, lo: f32, hi: f32) {
    let mut g = Rng::seeded();
    for i in 0..N {
        diff3(
            "f12",
            &format!("{tag} #{i}"),
            [g.range_f32(lo, hi), g.range_f32(0.0001, 1.0), g.range_f32(0.0, 1.0)],
        );
        diff3(
            "f12",
            &format!("{tag} wild #{i}"),
            [g.range_f32(lo, hi), g.range_f32(-3.0, 3.0), g.range_f32(-3.0, 3.0)],
        );
    }
}

#[test]
fn c69_f12_i0() {
    f12_sector("C69", 0.0, 60.0);
}
#[test]
fn c70_f12_i1() {
    f12_sector("C70", 60.0, 120.0);
}
#[test]
fn c71_f12_i2() {
    f12_sector("C71", 120.0, 180.0);
}
#[test]
fn c72_f12_i3() {
    f12_sector("C72", 180.0, 240.0);
}
#[test]
fn c73_f12_i4() {
    f12_sector("C73", 240.0, 300.0);
}

#[test]
fn c74_f12_default_arm() {
    // i == 5, i >= 6, i < 0 all land in `default:`
    f12_sector("C74 i5", 300.0, 360.0);
    f12_sector("C74 i6+", 360.0, 3600.0);
    f12_sector("C74 ineg", -3600.0, -0.0001);
}

#[test]
fn c75_f12_sector_boundaries() {
    let bounds = [
        -360.0f32, -60.0, -0.0, 0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0, 420.0,
    ];
    for &b in &bounds {
        for &h in &[
            b,
            f32::from_bits(b.to_bits().wrapping_sub(1)),
            f32::from_bits(b.to_bits().wrapping_add(1)),
            b - 1e-5,
            b + 1e-5,
        ] {
            for &s in &[1e-7f32, 0.5, 1.0, 2.0, -1.0] {
                for &v in &[0.0f32, 0.5, 1.0, -1.0, 1e30] {
                    diff3("f12", "C75", [h, s, v]);
                }
            }
        }
    }
}

#[test]
fn c76_f12_float_to_int_out_of_range() {
    // (int)floorf(h/60) is UB in C for out-of-range values; x86 cvttss2si
    // yields INT_MIN, which selects the `default:` arm.
    let hs = [
        1e30f32,
        -1e30,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        2147483648.0 * 60.0,
        -2147483648.0 * 60.0,
        2147483647.0 * 60.0,
        1.28849018e11, // ~ 2^31 * 60
        -1.28849018e11,
        1e10,
        -1e10,
    ];
    for &h in &hs {
        for &s in &[0.5f32, 1.0, -1.0, 1e-30, f32::NAN] {
            for &v in &[0.0f32, 0.5, 1.0, f32::INFINITY, f32::NAN] {
                diff3("f12", "C76", [h, s, v]);
            }
        }
    }
    // dense sweep around the 2^31 boundary of h/60
    let mut g = Rng::seeded();
    for i in 0..N {
        let scale = 2147483648.0f32 * 60.0;
        let h = scale * g.range_f32(0.9999, 1.0001) * if i % 2 == 0 { 1.0 } else { -1.0 };
        diff3("f12", &format!("C76 boundary #{i}"), [h, 0.5, 0.75]);
    }
}

#[test]
fn c77_f12_nan() {
    let sp = special_f32s();
    for &h in &sp {
        for &s in &sp {
            for &v in &[0.5f32, 0.0, 1.0, f32::NAN, f32::INFINITY, -0.25] {
                diff3("f12", "C77", [h, s, v]);
            }
        }
    }
    let nans = [
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0x7FAB_CDEF),
        f32::from_bits(0xFFD5_5555),
    ];
    for &a in &nans {
        for &b in &nans {
            for &h in &[30.0f32, 90.0, 150.0, 210.0, 270.0, 330.0, f32::NAN] {
                diff3("f12", "C77 dual-nan", [h, a, b]);
            }
            diff3("f12", "C77 nan-h", [a, b, 0.5]);
            diff3("f12", "C77 nan-h2", [a, 0.5, b]);
        }
    }
}

#[test]
fn c78_f12_fully_random() {
    sweep("f12", "C78 anybits", N * 4, |g| {
        [g.any_f32(), g.any_f32(), g.any_f32()]
    });
    sweep("f12", "C78 mixed", N * 4, |g| {
        [g.mixed_f32(), g.mixed_f32(), g.mixed_f32()]
    });
}

#[test]
fn c79_f12_dest_aliases_src() {
    let mut g = Rng::seeded();
    for i in 0..N {
        diff3_aliased(
            "f12",
            &format!("C79 #{i}"),
            [g.range_f32(-400.0, 760.0), g.range_f32(-1.0, 2.0), g.range_f32(-1.0, 2.0)],
        );
        diff3_aliased(
            "f12",
            &format!("C79 mixed #{i}"),
            [g.mixed_f32(), g.mixed_f32(), g.mixed_f32()],
        );
    }
}

// ===========================================================================
// f13 — RGB to HSV. Rows C80-C90.
// ===========================================================================

#[test]
fn c80_f13_r_is_max() {
    let mut g = Rng::seeded();
    for i in 0..N {
        let r = g.range_f32(0.2, 1.0);
        // g > b -> positive hue; g < b -> negative hue needing the +360 fix
        let gv = g.range_f32(0.0, r);
        let bv = g.range_f32(0.0, r);
        diff3("f13", &format!("C80 #{i}"), [r, gv, bv]);
        diff3("f13", &format!("C80 gb-swap #{i}"), [r, bv, gv]);
        // guarantee both sub-cases
        diff3("f13", &format!("C80 g>b #{i}"), [r, r * 0.9, r * 0.1]);
        diff3("f13", &format!("C80 g<b #{i}"), [r, r * 0.1, r * 0.9]);
    }
}

#[test]
fn c81_f13_g_is_max() {
    let mut g = Rng::seeded();
    for i in 0..N {
        let gv = g.range_f32(0.2, 1.0);
        diff3("f13", &format!("C81 #{i}"), [g.range_f32(0.0, gv), gv, g.range_f32(0.0, gv)]);
        diff3("f13", &format!("C81 a #{i}"), [gv * 0.5, gv, gv * 0.1]);
        diff3("f13", &format!("C81 b #{i}"), [gv * 0.1, gv, gv * 0.5]);
    }
}

#[test]
fn c82_f13_b_is_max() {
    let mut g = Rng::seeded();
    for i in 0..N {
        let bv = g.range_f32(0.2, 1.0);
        diff3("f13", &format!("C82 #{i}"), [g.range_f32(0.0, bv), g.range_f32(0.0, bv), bv]);
        diff3("f13", &format!("C82 a #{i}"), [bv * 0.5, bv * 0.1, bv]);
        diff3("f13", &format!("C82 b #{i}"), [bv * 0.1, bv * 0.5, bv]);
    }
}

#[test]
fn c83_f13_ties() {
    let mut g = Rng::seeded();
    for i in 0..N {
        let hi = g.range_f32(0.2, 1.0);
        let lo = g.range_f32(0.0, hi * 0.9);
        // r == g == max
        diff3("f13", &format!("C83 rg #{i}"), [hi, hi, lo]);
        // r == b == max
        diff3("f13", &format!("C83 rb #{i}"), [hi, lo, hi]);
        // g == b == max
        diff3("f13", &format!("C83 gb #{i}"), [lo, hi, hi]);
    }
    // signed zeros in the tie positions
    for &a in &[0.0f32, -0.0] {
        for &b in &[0.0f32, -0.0] {
            for &c in &[0.0f32, -0.0, 1.0, -1.0] {
                diff3("f13", "C83 zeros", [a, b, c]);
                diff3("f13", "C83 zeros2", [c, a, b]);
                diff3("f13", "C83 zeros3", [b, c, a]);
            }
        }
    }
}

#[test]
fn c84_f13_delta_zero() {
    let mut g = Rng::seeded();
    for i in 0..N {
        let v = g.finite_f32(100.0);
        diff3("f13", &format!("C84 #{i}"), [v, v, v]);
    }
    for &v in &special_f32s() {
        diff3("f13", "C84 special", [v, v, v]);
    }
}

#[test]
fn c85_f13_max_zero_with_nonzero_delta() {
    // All-negative inputs: max is <= 0 while delta > 0, so the
    // `delta == 0 || max == 0` guard fires only in the max == 0 sub-case.
    let mut g = Rng::seeded();
    for i in 0..N {
        let a = -g.range_f32(0.0001, 10.0);
        let b = -g.range_f32(0.0001, 10.0);
        diff3("f13", &format!("C85 neg #{i}"), [a, b, 0.0]);
        diff3("f13", &format!("C85 neg2 #{i}"), [a, 0.0, b]);
        diff3("f13", &format!("C85 neg3 #{i}"), [0.0, a, b]);
        diff3("f13", &format!("C85 negz #{i}"), [a, b, -0.0]);
        diff3("f13", &format!("C85 allneg #{i}"), [a, b, -g.range_f32(0.0001, 10.0)]);
    }
    for t in [
        [-1.0f32, -2.0, 0.0],
        [-1.0, -2.0, -0.0],
        [0.0, -1.0, -2.0],
        [-0.0, -1.0, -2.0],
        [-1.0, 0.0, -2.0],
    ] {
        diff3("f13", "C85 fixed", t);
    }
}

#[test]
fn c86_f13_negative_hue_correction() {
    // r is max and g < b -> (g-b)/delta < 0 -> h*60 < 0 -> += 360
    let mut g = Rng::seeded();
    for i in 0..N {
        let r = g.range_f32(0.5, 1.0);
        let bv = g.range_f32(0.0, r);
        let gv = g.range_f32(0.0, bv);
        diff3("f13", &format!("C86 #{i}"), [r, gv, bv]);
    }
    // extreme ratios: h*60 hugely negative, so h += 360 still leaves h < 0
    for i in 1..500 {
        let d = 1.0f32 / i as f32;
        diff3("f13", "C86 extreme", [d, -1e30, 0.0]);
        diff3("f13", "C86 extreme2", [1e30, -1e30, 1e-30]);
        diff3("f13", "C86 extreme3", [f32::MAX, -f32::MAX, 0.0]);
    }
}

#[test]
fn c87_f13_out_of_unit_range_and_inf() {
    sweep("f13", "C87", N * 2, |g| {
        [g.finite_f32(1e6), g.finite_f32(1e6), g.finite_f32(1e6)]
    });
    let infs = [f32::INFINITY, f32::NEG_INFINITY, 0.0, -0.0, 1.0, -1.0, f32::MAX, f32::MIN];
    for &a in &infs {
        for &b in &infs {
            for &c in &infs {
                diff3("f13", "C87 inf", [a, b, c]);
            }
        }
    }
}

#[test]
fn c88_f13_nan_in_each_position() {
    let nans = [
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0x7FAB_CDEF),
        f32::from_bits(0xFFD5_5555),
        f32::from_bits(0xFF80_0002),
    ];
    let vals = [0.0f32, -0.0, 0.25, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY];
    for &n in &nans {
        for &a in &vals {
            for &b in &vals {
                diff3("f13", "C88 n0", [n, a, b]);
                diff3("f13", "C88 n1", [a, n, b]);
                diff3("f13", "C88 n2", [a, b, n]);
            }
        }
        for &m in &nans {
            diff3("f13", "C88 nn0", [n, m, 0.5]);
            diff3("f13", "C88 nn1", [n, 0.5, m]);
            diff3("f13", "C88 nn2", [0.5, n, m]);
            diff3("f13", "C88 nnn", [n, m, n]);
        }
    }
}

#[test]
fn c89_f13_fully_random() {
    sweep("f13", "C89 anybits", N * 4, |g| {
        [g.any_f32(), g.any_f32(), g.any_f32()]
    });
    sweep("f13", "C89 mixed", N * 4, |g| {
        [g.mixed_f32(), g.mixed_f32(), g.mixed_f32()]
    });
}

#[test]
fn c90_f13_dest_aliases_src() {
    let mut g = Rng::seeded();
    for i in 0..N {
        diff3_aliased(
            "f13",
            &format!("C90 #{i}"),
            [g.range_f32(-2.0, 2.0), g.range_f32(-2.0, 2.0), g.range_f32(-2.0, 2.0)],
        );
        diff3_aliased(
            "f13",
            &format!("C90 mixed #{i}"),
            [g.mixed_f32(), g.mixed_f32(), g.mixed_f32()],
        );
    }
}
