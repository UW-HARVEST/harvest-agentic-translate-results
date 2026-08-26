//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through `dlopen`/`dlsym` and compares the three output floats
//! bit-for-bit. Inputs are randomized with a fixed seed (`SplitMix64`).

mod common;

use common::*;

const N: usize = 4000;

fn rng(row: u64) -> Rng {
    // Distinct but deterministic stream per row.
    Rng::new(0x5EED_0000_0000_0000 ^ (row.wrapping_mul(0x9E37_79B9_7F4A_7C15)))
}

/// Sort helper: returns a permutation of `[a,b,c]` (descending) so a caller can
/// place a chosen component at the maximum / middle / minimum slot.
fn desc3(mut v: [f32; 3]) -> [f32; 3] {
    v.sort_by(|x, y| y.partial_cmp(x).unwrap());
    v
}

// ---------------------------------------------------------------------------
// Row 1 — r strictly max, g > b  (sector r, no +360 fixup)
// ---------------------------------------------------------------------------
#[test]
fn row01_r_max_g_gt_b() {
    let p = load_pair();
    let mut rg = rng(1);
    for _ in 0..N {
        let s = desc3([rg.range(0.0, 1.0), rg.range(0.0, 1.0), rg.range(0.0, 1.0)]);
        // r = largest, g = middle, b = smallest -> g > b
        p.assert_same("row01 r_max_g_gt_b", [s[0], s[1], s[2]]);
    }
    for v in [
        [1.0f32, 0.5, 0.0],
        [1.0, 1.0 - f32::EPSILON, 0.0],
        [f32::MIN_POSITIVE * 4.0, f32::MIN_POSITIVE * 2.0, f32::MIN_POSITIVE],
        [3.0, 2.0, 1.0],
    ] {
        p.assert_same("row01 fixed", v);
    }
}

// ---------------------------------------------------------------------------
// Row 2 — r strictly max, b > g  (sector r, h < 0 -> +360)
// ---------------------------------------------------------------------------
#[test]
fn row02_r_max_b_gt_g() {
    let p = load_pair();
    let mut rg = rng(2);
    for _ in 0..N {
        let s = desc3([rg.range(0.0, 1.0), rg.range(0.0, 1.0), rg.range(0.0, 1.0)]);
        // r = largest, b = middle, g = smallest -> b > g -> h negative
        p.assert_same("row02 r_max_b_gt_g", [s[0], s[2], s[1]]);
    }
    for v in [[1.0f32, 0.0, 0.5], [1.0, 0.0, 1.0 - f32::EPSILON], [2.0, 0.5, 1.5]] {
        p.assert_same("row02 fixed", v);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — r max, g == b exactly (h == +0.0, fixup not taken)
// ---------------------------------------------------------------------------
#[test]
fn row03_r_max_g_eq_b() {
    let p = load_pair();
    let mut rg = rng(3);
    for _ in 0..N {
        let hi = rg.range(0.5, 1.0);
        let lo = rg.range(0.0, 0.5);
        p.assert_same("row03 r_max_g_eq_b", [hi, lo, lo]);
    }
    for v in [[1.0f32, 0.0, 0.0], [1.0, 1.0e-30, 1.0e-30], [f32::MAX, 0.0, 0.0]] {
        p.assert_same("row03 fixed", v);
    }
}

// ---------------------------------------------------------------------------
// Row 4 — g strictly max (sector g)
// ---------------------------------------------------------------------------
#[test]
fn row04_g_max() {
    let p = load_pair();
    let mut rg = rng(4);
    for _ in 0..N {
        let s = desc3([rg.range(0.0, 1.0), rg.range(0.0, 1.0), rg.range(0.0, 1.0)]);
        // g = largest; alternate which of r/b is middle so both hue signs appear
        if rg.bool() {
            p.assert_same("row04 g_max (r mid)", [s[1], s[0], s[2]]);
        } else {
            p.assert_same("row04 g_max (b mid)", [s[2], s[0], s[1]]);
        }
    }
    for v in [[0.0f32, 1.0, 0.0], [0.25, 1.0, 0.75], [0.75, 1.0, 0.25]] {
        p.assert_same("row04 fixed", v);
    }
}

// ---------------------------------------------------------------------------
// Row 5 — b strictly max (sector b)
// ---------------------------------------------------------------------------
#[test]
fn row05_b_max() {
    let p = load_pair();
    let mut rg = rng(5);
    for _ in 0..N {
        let s = desc3([rg.range(0.0, 1.0), rg.range(0.0, 1.0), rg.range(0.0, 1.0)]);
        if rg.bool() {
            p.assert_same("row05 b_max (r mid)", [s[1], s[2], s[0]]);
        } else {
            p.assert_same("row05 b_max (g mid)", [s[2], s[1], s[0]]);
        }
    }
    for v in [[0.0f32, 0.0, 1.0], [0.25, 0.75, 1.0], [0.75, 0.25, 1.0]] {
        p.assert_same("row05 fixed", v);
    }
}

// ---------------------------------------------------------------------------
// Row 6 — tie r == g > b  (sector priority must pick r)
// ---------------------------------------------------------------------------
#[test]
fn row06_tie_r_eq_g() {
    let p = load_pair();
    let mut rg = rng(6);
    for _ in 0..N {
        let hi = rg.range(0.25, 1.0);
        let lo = rg.range(-1.0, 0.25);
        p.assert_same("row06 tie r==g>b", [hi, hi, lo]);
    }
    for v in [[1.0f32, 1.0, 0.0], [0.5, 0.5, -0.5], [1.0, 1.0, -1.0]] {
        p.assert_same("row06 fixed", v);
    }
}

// ---------------------------------------------------------------------------
// Row 7 — tie g == b > r  (max ternary keeps the later operand: b)
// ---------------------------------------------------------------------------
#[test]
fn row07_tie_g_eq_b() {
    let p = load_pair();
    let mut rg = rng(7);
    for _ in 0..N {
        let hi = rg.range(0.25, 1.0);
        let lo = rg.range(-1.0, 0.25);
        p.assert_same("row07 tie g==b>r", [lo, hi, hi]);
    }
    for v in [[0.0f32, 1.0, 1.0], [-0.5, 0.5, 0.5]] {
        p.assert_same("row07 fixed", v);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — tie r == b > g  (sector picks r)
// ---------------------------------------------------------------------------
#[test]
fn row08_tie_r_eq_b() {
    let p = load_pair();
    let mut rg = rng(8);
    for _ in 0..N {
        let hi = rg.range(0.25, 1.0);
        let lo = rg.range(-1.0, 0.25);
        p.assert_same("row08 tie r==b>g", [hi, lo, hi]);
    }
    for v in [[1.0f32, 0.0, 1.0], [0.5, -0.5, 0.5]] {
        p.assert_same("row08 fixed", v);
    }
}

// ---------------------------------------------------------------------------
// Row 9 — r == g == b, positive (delta == 0 early return)
// ---------------------------------------------------------------------------
#[test]
fn row09_all_equal_positive() {
    let p = load_pair();
    let mut rg = rng(9);
    for _ in 0..N {
        let v = rg.range(f32::MIN_POSITIVE, 1.0);
        p.assert_same("row09 all equal", [v, v, v]);
    }
    for v in [1.0f32, 0.5, f32::MAX, f32::MIN_POSITIVE, 1e-45, 1e30] {
        p.assert_same("row09 fixed", [v, v, v]);
    }
}

// ---------------------------------------------------------------------------
// Row 10 — all zero (both disjuncts of the guard true)
// ---------------------------------------------------------------------------
#[test]
fn row10_all_zero() {
    let p = load_pair();
    p.assert_same("row10 all +0.0", [0.0, 0.0, 0.0]);
    p.assert_same("row10 all -0.0", [-0.0, -0.0, -0.0]);
}

// ---------------------------------------------------------------------------
// Row 11 — max == 0 while delta != 0 (guard hit through `max == 0`)
// ---------------------------------------------------------------------------
#[test]
fn row11_max_zero_delta_nonzero() {
    let p = load_pair();
    let mut rg = rng(11);
    for _ in 0..N {
        let a = -rg.range(0.0, 1000.0);
        let b = -rg.range(0.0, 1000.0);
        match rg.below(3) {
            0 => p.assert_same("row11 zero at r", [0.0, a, b]),
            1 => p.assert_same("row11 zero at g", [a, 0.0, b]),
            _ => p.assert_same("row11 zero at b", [a, b, 0.0]),
        }
        // negative-zero variants of the maximum
        p.assert_same("row11 -0.0 max", [-0.0, a, b]);
    }
    for v in [
        [0.0f32, -1.0, -2.0],
        [-1.0, 0.0, -2.0],
        [-1.0, -2.0, 0.0],
        [-0.0, -1.0, -1.0],
        [0.0, -f32::MAX, -f32::MIN_POSITIVE],
    ] {
        p.assert_same("row11 fixed", v);
    }
}

// ---------------------------------------------------------------------------
// Row 12 — all negative, distinct (s < 0, all three sectors)
// ---------------------------------------------------------------------------
#[test]
fn row12_all_negative() {
    let p = load_pair();
    let mut rg = rng(12);
    for _ in 0..N {
        let v = [
            -rg.range(f32::MIN_POSITIVE, 1.0),
            -rg.range(f32::MIN_POSITIVE, 1.0),
            -rg.range(f32::MIN_POSITIVE, 1.0),
        ];
        p.assert_same("row12 all negative", v);
    }
    for v in [
        [-1.0f32, -2.0, -3.0],
        [-3.0, -1.0, -2.0],
        [-3.0, -2.0, -1.0],
        [-1.0, -1.0, -2.0],
    ] {
        p.assert_same("row12 fixed", v);
    }
}

// ---------------------------------------------------------------------------
// Row 13 — mixed sign (min < 0 < max, so delta > max and s > 1)
// ---------------------------------------------------------------------------
#[test]
fn row13_mixed_sign() {
    let p = load_pair();
    let mut rg = rng(13);
    for _ in 0..N {
        let v = [rg.range(-1.0, 1.0), rg.range(-1.0, 1.0), rg.range(-1.0, 1.0)];
        p.assert_same("row13 mixed sign", v);
    }
    for v in [
        [1.0f32, -1.0, 0.0],
        [-1.0, 1.0, 0.0],
        [-1.0, 0.0, 1.0],
        [0.5, -0.5, 0.5],
    ] {
        p.assert_same("row13 fixed", v);
    }
}

// ---------------------------------------------------------------------------
// Row 14 — all 8 signed-zero combinations
// ---------------------------------------------------------------------------
#[test]
fn row14_signed_zeros() {
    let p = load_pair();
    let zs = [0.0f32, -0.0f32];
    for &r in &zs {
        for &g in &zs {
            for &b in &zs {
                p.assert_same("row14 signed zeros", [r, g, b]);
            }
        }
    }
    // signed zero mixed with non-zero values
    let others = [1.0f32, -1.0, f32::MIN_POSITIVE, -f32::MIN_POSITIVE];
    for &z in &zs {
        for &o in &others {
            p.assert_same("row14 zero+other A", [z, o, o]);
            p.assert_same("row14 zero+other B", [o, z, o]);
            p.assert_same("row14 zero+other C", [o, o, z]);
            p.assert_same("row14 zero+other D", [z, z, o]);
            p.assert_same("row14 zero+other E", [o, z, z]);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — subnormal inputs (subnormal delta, subnormal max)
// ---------------------------------------------------------------------------
#[test]
fn row15_subnormals() {
    let p = load_pair();
    let tiny = [
        f32::from_bits(1),
        f32::from_bits(2),
        f32::from_bits(3),
        f32::from_bits(0x0000_0FFF),
        f32::from_bits(0x007F_FFFF), // largest subnormal
        f32::MIN_POSITIVE,           // smallest normal
        -f32::from_bits(1),
        -f32::from_bits(0x007F_FFFF),
        0.0,
    ];
    for &r in &tiny {
        for &g in &tiny {
            for &b in &tiny {
                p.assert_same("row15 subnormals", [r, g, b]);
            }
        }
    }
    let mut rg = rng(15);
    for _ in 0..N {
        let v = [
            f32::from_bits(rg.below(0x0080_0000)),
            f32::from_bits(rg.below(0x0080_0000)),
            f32::from_bits(rg.below(0x0080_0000)),
        ];
        p.assert_same("row15 random subnormals", v);
    }
}

// ---------------------------------------------------------------------------
// Row 16 — extreme magnitudes / overflow of max - min
// ---------------------------------------------------------------------------
#[test]
fn row16_extremes() {
    let p = load_pair();
    let ext = [
        f32::MAX,
        -f32::MAX,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1.0,
        -1.0,
        0.0,
        3.4e38,
        -3.4e38,
    ];
    for &r in &ext {
        for &g in &ext {
            for &b in &ext {
                p.assert_same("row16 extremes", [r, g, b]);
            }
        }
    }
    let mut rg = rng(16);
    for _ in 0..N {
        let scale = 1e38f32;
        let v = [
            rg.range(-1.0, 1.0) * scale,
            rg.range(-1.0, 1.0) * scale,
            rg.range(-1.0, 1.0) * scale,
        ];
        p.assert_same("row16 random extremes", v);
    }
}

// ---------------------------------------------------------------------------
// Row 17 — every placement of +-inf
// ---------------------------------------------------------------------------
#[test]
fn row17_infinities() {
    let p = load_pair();
    let vals = [f32::INFINITY, f32::NEG_INFINITY, 1.0f32, 0.0, -1.0, f32::MAX];
    for &r in &vals {
        for &g in &vals {
            for &b in &vals {
                p.assert_same("row17 infinities", [r, g, b]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 18 — NaN in every slot, several payloads
// ---------------------------------------------------------------------------
#[test]
fn row18_nans() {
    let p = load_pair();
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0x7FFF_FFFF),
        f32::from_bits(0xFFC0_1234),
        f32::from_bits(0x7F80_0001), // signalling NaN
    ];
    let normals = [0.0f32, 1.0, -1.0, 0.5, f32::INFINITY, f32::NEG_INFINITY];
    for &n in &nans {
        for &a in &normals {
            for &b in &normals {
                p.assert_same("row18 nan at r", [n, a, b]);
                p.assert_same("row18 nan at g", [a, n, b]);
                p.assert_same("row18 nan at b", [a, b, n]);
                p.assert_same("row18 nan x2 rg", [n, n, b]);
                p.assert_same("row18 nan x2 gb", [a, n, n]);
                p.assert_same("row18 nan x2 rb", [n, b, n]);
            }
        }
        p.assert_same("row18 all nan", [n, n, n]);
    }
}

// ---------------------------------------------------------------------------
// Row 19 — 8-bit sRGB shape (the intended real input)
// ---------------------------------------------------------------------------
#[test]
fn row19_srgb_8bit() {
    let p = load_pair();
    // Coarse exhaustive grid over the 0..255 cube.
    let mut i = 0u32;
    while i < 256 {
        let mut j = 0u32;
        while j < 256 {
            let mut k = 0u32;
            while k < 256 {
                p.assert_same(
                    "row19 srgb grid",
                    [i as f32 / 255.0, j as f32 / 255.0, k as f32 / 255.0],
                );
                k += 17;
            }
            j += 17;
        }
        i += 17;
    }
    // Random 8-bit triples.
    let mut rg = rng(19);
    for _ in 0..20_000 {
        let v = [
            rg.below(256) as f32 / 255.0,
            rg.below(256) as f32 / 255.0,
            rg.below(256) as f32 / 255.0,
        ];
        p.assert_same("row19 srgb random", v);
    }
}

// ---------------------------------------------------------------------------
// Row 20 — large positive normals
// ---------------------------------------------------------------------------
#[test]
fn row20_large_positive() {
    let p = load_pair();
    let mut rg = rng(20);
    for _ in 0..N {
        let v = [
            rg.range(1.0, 1.0e6),
            rg.range(1.0, 1.0e6),
            rg.range(1.0, 1.0e6),
        ];
        p.assert_same("row20 large positive", v);
    }
    for v in [
        [255.0f32, 128.0, 0.0],
        [1e6, 1.0, 1.0],
        [1.0, 1e6, 1e-6],
        [1e20, 1e-20, 1.0],
    ] {
        p.assert_same("row20 fixed", v);
    }
}

// ---------------------------------------------------------------------------
// Row 21 — fully random bit patterns (any float class)
// ---------------------------------------------------------------------------
#[test]
fn row21_random_bits() {
    let p = load_pair();
    let mut rg = rng(21);
    for _ in 0..20_000 {
        let v = [rg.any_f32(), rg.any_f32(), rg.any_f32()];
        p.assert_same("row21 random bits", v);
    }
    // Same, but with duplicated components to force ties in random classes.
    for _ in 0..5_000 {
        let a = rg.any_f32();
        let b = rg.any_f32();
        p.assert_same("row21 dup rg", [a, a, b]);
        p.assert_same("row21 dup gb", [b, a, a]);
        p.assert_same("row21 dup rb", [a, b, a]);
        p.assert_same("row21 dup all", [a, a, a]);
    }
}

// ---------------------------------------------------------------------------
// Row 22 — in-place call (dest == src)
// ---------------------------------------------------------------------------
#[test]
fn row22_in_place() {
    let p = load_pair();
    let mut rg = rng(22);
    for _ in 0..N {
        let v = [rg.range(-1.0, 1.0), rg.range(-1.0, 1.0), rg.range(-1.0, 1.0)];
        p.assert_same_in_place("row22", v);
    }
    for v in [
        [1.0f32, 0.5, 0.0],
        [0.0, 0.0, 0.0],
        [f32::NAN, 1.0, 2.0],
        [f32::INFINITY, 0.0, 0.0],
        [-1.0, -2.0, -3.0],
    ] {
        p.assert_same_in_place("row22 fixed", v);
    }
    let mut rg = rng(2200);
    for _ in 0..2_000 {
        p.assert_same_in_place("row22 random bits", [rg.any_f32(), rg.any_f32(), rg.any_f32()]);
    }
}

// ---------------------------------------------------------------------------
// Row 23 — partially overlapping dest / src
// ---------------------------------------------------------------------------
#[test]
fn row23_partial_overlap() {
    let p = load_pair();
    let mut rg = rng(23);
    for _ in 0..N {
        let base = [
            rg.range(-1.0, 1.0),
            rg.range(-1.0, 1.0),
            rg.range(-1.0, 1.0),
            rg.range(-1.0, 1.0),
        ];

        // (a) dest = buf, src = buf + 1
        let mut cbuf = base;
        let mut rbuf = base;
        unsafe {
            (p.c.rgb_to_hsv)(cbuf.as_mut_ptr(), cbuf.as_ptr().add(1));
            (p.rs.rgb_to_hsv)(rbuf.as_mut_ptr(), rbuf.as_ptr().add(1));
        }
        assert_eq!(
            cbuf.map(f32::to_bits),
            rbuf.map(f32::to_bits),
            "row23 (dest=buf, src=buf+1) base={base:?}"
        );

        // (b) dest = buf + 1, src = buf
        let mut cbuf = base;
        let mut rbuf = base;
        unsafe {
            (p.c.rgb_to_hsv)(cbuf.as_mut_ptr().add(1), cbuf.as_ptr());
            (p.rs.rgb_to_hsv)(rbuf.as_mut_ptr().add(1), rbuf.as_ptr());
        }
        assert_eq!(
            cbuf.map(f32::to_bits),
            rbuf.map(f32::to_bits),
            "row23 (dest=buf+1, src=buf) base={base:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 24 — disjoint but not 16-byte aligned
// ---------------------------------------------------------------------------
#[test]
fn row24_unaligned() {
    let p = load_pair();
    let mut rg = rng(24);
    for off_s in 0..4usize {
        for off_d in 0..4usize {
            for _ in 0..200 {
                let mut sbuf = [0.0f32; 8];
                sbuf[off_s] = rg.range(-1.0, 1.0);
                sbuf[off_s + 1] = rg.range(-1.0, 1.0);
                sbuf[off_s + 2] = rg.range(-1.0, 1.0);
                let mut cdst = [f32::from_bits(POISON); 8];
                let mut rdst = [f32::from_bits(POISON); 8];
                unsafe {
                    (p.c.rgb_to_hsv)(cdst.as_mut_ptr().add(off_d), sbuf.as_ptr().add(off_s));
                    (p.rs.rgb_to_hsv)(rdst.as_mut_ptr().add(off_d), sbuf.as_ptr().add(off_s));
                }
                assert_eq!(
                    cdst.map(f32::to_bits),
                    rdst.map(f32::to_bits),
                    "row24 unaligned off_s={off_s} off_d={off_d} src={:?}",
                    &sbuf[off_s..off_s + 3]
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25 — exactly three floats written, canaries untouched
// ---------------------------------------------------------------------------
#[test]
fn row25_write_window() {
    let p = load_pair();
    let mut rg = rng(25);
    const CANARY: u32 = 0xA5A5_5A5A;
    for _ in 0..N {
        let src = [rg.any_f32(), rg.any_f32(), rg.any_f32()];
        for lib in [&p.c, &p.rs] {
            let mut buf = [f32::from_bits(CANARY); 9];
            unsafe {
                (lib.rgb_to_hsv)(buf.as_mut_ptr().add(3), src.as_ptr());
            }
            for i in (0..3).chain(6..9) {
                assert_eq!(
                    buf[i].to_bits(),
                    CANARY,
                    "{}: canary at index {i} clobbered (src={src:?})",
                    lib.name
                );
            }
        }
        p.assert_same("row25 window", src);
    }
}

// ---------------------------------------------------------------------------
// Row 26 — statelessness across repeated calls with reused buffers
// ---------------------------------------------------------------------------
#[test]
fn row26_stateless_repeat() {
    let p = load_pair();
    let first = [0.75f32, 0.25, 0.5];
    let (c0, r0) = p.call_both(first);
    assert_bits_eq("row26 first call", first, c0, r0);

    let mut rg = rng(26);
    let mut cbuf = [0.0f32; 3];
    let mut rbuf = [0.0f32; 3];
    for _ in 0..1_000 {
        let src = [rg.any_f32(), rg.any_f32(), rg.any_f32()];
        unsafe {
            (p.c.rgb_to_hsv)(cbuf.as_mut_ptr(), src.as_ptr());
            (p.rs.rgb_to_hsv)(rbuf.as_mut_ptr(), src.as_ptr());
        }
        assert_bits_eq("row26 loop", src, bits3(&cbuf), bits3(&rbuf));
    }

    // Re-running the very first input must give the very first output.
    let (c1, r1) = p.call_both(first);
    assert_bits_eq("row26 replay", first, c1, r1);
    assert_eq!(c0, c1, "row26: C is not stateless");
    assert_eq!(r0, r1, "row26: Rust is not stateless");
}

// ---------------------------------------------------------------------------
// Row 27 — whole-image batch through both .so files
// ---------------------------------------------------------------------------
#[test]
fn row27_batch_image() {
    let p = load_pair();
    let mut rg = rng(27);
    const PIXELS: usize = 4096;

    let mut src = vec![0.0f32; PIXELS * 3];
    for (i, v) in src.iter_mut().enumerate() {
        *v = match i % 7 {
            0 => rg.below(256) as f32 / 255.0,
            1 => rg.unit(),
            2 => rg.range(-1.0, 1.0),
            3 => f32::from_bits(rg.below(0x0080_0000)),
            4 => rg.any_f32(),
            5 => [0.0f32, -0.0, 1.0, f32::INFINITY, f32::NAN][rg.below(5) as usize],
            _ => rg.range(0.0, 255.0),
        };
    }

    let mut cdst = vec![f32::from_bits(POISON); PIXELS * 3];
    let mut rdst = vec![f32::from_bits(POISON); PIXELS * 3];
    unsafe {
        for i in 0..PIXELS {
            (p.c.rgb_to_hsv)(cdst.as_mut_ptr().add(i * 3), src.as_ptr().add(i * 3));
        }
        for i in 0..PIXELS {
            (p.rs.rgb_to_hsv)(rdst.as_mut_ptr().add(i * 3), src.as_ptr().add(i * 3));
        }
    }
    for i in 0..PIXELS * 3 {
        assert_eq!(
            cdst[i].to_bits(),
            rdst[i].to_bits(),
            "row27 batch mismatch at float {i} (pixel {}): src={:?} C={} Rust={}",
            i / 3,
            &src[(i / 3) * 3..(i / 3) * 3 + 3],
            cdst[i],
            rdst[i]
        );
    }

    // Also convert the same image in place, in both libraries, and compare.
    let mut cimg = src.clone();
    let mut rimg = src.clone();
    unsafe {
        for i in 0..PIXELS {
            let q = cimg.as_mut_ptr().add(i * 3);
            (p.c.rgb_to_hsv)(q, q as *const f32);
            let q = rimg.as_mut_ptr().add(i * 3);
            (p.rs.rgb_to_hsv)(q, q as *const f32);
        }
    }
    for i in 0..PIXELS * 3 {
        assert_eq!(
            cimg[i].to_bits(),
            rimg[i].to_bits(),
            "row27 in-place batch mismatch at float {i}"
        );
        assert_eq!(
            cimg[i].to_bits(),
            cdst[i].to_bits(),
            "row27 in-place differs from disjoint at float {i}"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 28 — concurrent calls from several threads (the C keeps no global state,
// so results must be identical to the single-threaded ones)
// ---------------------------------------------------------------------------
#[test]
fn row28_concurrent() {
    use std::sync::Arc;

    let p = Arc::new(load_pair());
    let mut handles = Vec::new();
    for t in 0..4u64 {
        let p = Arc::clone(&p);
        handles.push(std::thread::spawn(move || {
            let mut rg = rng(280 + t);
            for _ in 0..2000 {
                let src = [rg.any_f32(), rg.any_f32(), rg.any_f32()];
                p.assert_same("row28 concurrent", src);
            }
        }));
    }
    for h in handles {
        h.join().expect("worker thread panicked");
    }
}
