//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads both `.so` files via `libloading` and compares the three
//! written `f32`s bit-for-bit. All randomized rows use the fixed seed
//! `common::SEED` so failures are reproducible.

mod common;

use common::{assert_same, check, Rng, ITERS, SEED};

// ---------------------------------------------------------------------------
// C1 — canonical [0,1] domain
// ---------------------------------------------------------------------------
#[test]
fn cfg_c1_unit_domain_random() {
    let mut rng = Rng::new(SEED);
    for _ in 0..ITERS {
        let src = [rng.unit(), rng.unit(), rng.unit()];
        assert_same(&src, "C1");
    }
}

// ---------------------------------------------------------------------------
// C2 — r strict max, g > b  =>  h >= 0, no wrap
// ---------------------------------------------------------------------------
#[test]
fn cfg_c2_r_max_no_wrap() {
    let mut rng = Rng::new(SEED ^ 2);
    let mut seen = 0usize;
    for _ in 0..ITERS {
        // b < g < r, all in (0,1]
        let r = rng.range(0.5, 1.0);
        let g = rng.range(0.2, 0.5);
        let b = rng.range(0.0, 0.2);
        let src = [r, g, b];
        let out = check(&src, "C2");
        // r is the strict max and g > b, so the C must take the r branch with
        // a non-negative hue (0..60 degrees) and skip the +360 wrap.
        assert!(
            out[0] >= 0.0 && out[0] <= 60.0,
            "C2 expected h in [0,60], got {} for {src:?}",
            out[0]
        );
        seen += 1;
    }
    assert_eq!(seen, ITERS);
}

// ---------------------------------------------------------------------------
// C3 — r strict max, g < b  =>  h < 0  =>  +360 wrap
// ---------------------------------------------------------------------------
#[test]
fn cfg_c3_r_max_wrap() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..ITERS {
        // g < b < r
        let r = rng.range(0.5, 1.0);
        let b = rng.range(0.2, 0.5);
        let g = rng.range(0.0, 0.2);
        let src = [r, g, b];
        let out = check(&src, "C3");
        assert!(
            out[0] >= 300.0 && out[0] <= 360.0,
            "C3 expected wrapped h in [300,360], got {} for {src:?}",
            out[0]
        );
    }
    // Pinned example from ERRORS.md row E16.
    let out = check(&[1.0, 0.0, 0.5], "C3/pinned");
    assert_eq!(out[0].to_bits(), 330.0f32.to_bits());
    assert_eq!(out[1].to_bits(), 1.0f32.to_bits());
    assert_eq!(out[2].to_bits(), 1.0f32.to_bits());
}

// ---------------------------------------------------------------------------
// C4 — g strict max
// ---------------------------------------------------------------------------
#[test]
fn cfg_c4_g_max() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..ITERS {
        let g = rng.range(0.6, 1.0);
        let r = rng.range(0.0, 0.6);
        let b = rng.range(0.0, 0.6);
        let src = [r, g, b];
        let out = check(&src, "C4");
        // h = 60 * (2 + (b-r)/delta), with (b-r)/delta in [-1,1] => [60,180]
        assert!(
            out[0] >= 60.0 && out[0] <= 180.0,
            "C4 expected h in [60,180], got {} for {src:?}",
            out[0]
        );
    }
}

// ---------------------------------------------------------------------------
// C5 — b strict max  =>  else branch
// ---------------------------------------------------------------------------
#[test]
fn cfg_c5_b_max() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..ITERS {
        let b = rng.range(0.6, 1.0);
        let r = rng.range(0.0, 0.6);
        let g = rng.range(0.0, 0.6);
        let src = [r, g, b];
        let out = check(&src, "C5");
        // h = 60 * (4 + (r-g)/delta) in [180,300]
        assert!(
            out[0] >= 180.0 && out[0] <= 300.0,
            "C5 expected h in [180,300], got {} for {src:?}",
            out[0]
        );
    }
}

// ---------------------------------------------------------------------------
// C6 — achromatic r == g == b (delta == 0 early return)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c6_achromatic_random() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..ITERS {
        // Cover every magnitude class, including non-finite.
        let v = match rng.below(6) {
            0 => rng.unit(),
            1 => rng.range(-1000.0, 1000.0),
            2 => rng.subnormal(),
            3 => rng.any_f32(),
            4 => f32::MAX,
            _ => rng.range(0.0, f32::MAX),
        };
        assert_same(&[v, v, v], "C6");
    }
    for v in [0.0f32, -0.0, 0.5, 1.0, -1.0, -2.0, f32::MAX, f32::MIN, f32::MIN_POSITIVE] {
        assert_same(&[v, v, v], "C6/pinned");
    }
}

// ---------------------------------------------------------------------------
// C7 — channels 1..4 ULP apart (tiny / subnormal delta)
// ---------------------------------------------------------------------------
fn add_ulps(x: f32, n: i32) -> f32 {
    let b = x.to_bits() as i32;
    f32::from_bits(b.wrapping_add(n) as u32)
}

#[test]
fn cfg_c7_one_ulp_apart() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..ITERS {
        let base = match rng.below(4) {
            0 => rng.unit(),
            1 => rng.subnormal(),
            2 => rng.range(-1.0, 1.0),
            _ => rng.any_f32(),
        };
        let d1 = (rng.below(9) as i32) - 4;
        let d2 = (rng.below(9) as i32) - 4;
        assert_same(&[base, add_ulps(base, d1), add_ulps(base, d2)], "C7");
    }
    // Pinned: adjacent floats at several scales.
    for base in [1.0f32, 0.5, 1e-30, 1e30, f32::MIN_POSITIVE, 0.0] {
        for d in [-2i32, -1, 0, 1, 2] {
            assert_same(&[base, add_ulps(base, d), add_ulps(base, -d)], "C7/pinned");
        }
    }
}

// ---------------------------------------------------------------------------
// C8 — exact two-channel ties (if / else-if priority)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c8_two_channel_ties() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..ITERS {
        let hi = rng.range(0.3, 1.0);
        let lo = rng.range(-1.0, 0.3);
        // r == g > b : r branch must win over the g branch.
        assert_same(&[hi, hi, lo], "C8/r==g");
        // g == b > r : r==max false, g==max true => g branch (not else).
        assert_same(&[lo, hi, hi], "C8/g==b");
        // r == b > g : r==max true => r branch.
        assert_same(&[hi, lo, hi], "C8/r==b");
        // all three tie => delta == 0 early return.
        assert_same(&[hi, hi, hi], "C8/all");
    }
    // Pinned checks of the documented branch priority.
    let out = check(&[1.0, 1.0, 0.0], "C8/pinned r==g");
    assert_eq!(out[0].to_bits(), 60.0f32.to_bits(), "r branch: (g-b)/delta*60 = 60");
    let out = check(&[0.0, 1.0, 1.0], "C8/pinned g==b");
    assert_eq!(out[0].to_bits(), 180.0f32.to_bits(), "g branch: (2+(b-r)/delta)*60 = 180");
    let out = check(&[1.0, 0.0, 1.0], "C8/pinned r==b");
    assert_eq!(out[0].to_bits(), 300.0f32.to_bits(), "r branch: (g-b)/delta*60 = -60 -> 300");
}

// ---------------------------------------------------------------------------
// C9 — exact hue boundaries at several s/v levels
// ---------------------------------------------------------------------------
#[test]
fn cfg_c9_hue_boundaries() {
    // Unit hue wheel in 1-degree steps, reconstructed exactly the way an HSV
    // consumer would, at a spread of v and s levels.
    for vi in 0..=8u32 {
        let v = vi as f32 / 8.0;
        for si in 0..=8u32 {
            let s = si as f32 / 8.0;
            for deg in 0..=360u32 {
                let h = deg as f32;
                let c = v * s;
                let hp = h / 60.0;
                let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
                let m = v - c;
                let (r1, g1, b1) = match deg / 60 {
                    0 => (c, x, 0.0),
                    1 => (x, c, 0.0),
                    2 => (0.0, c, x),
                    3 => (0.0, x, c),
                    4 => (x, 0.0, c),
                    _ => (c, 0.0, x),
                };
                assert_same(&[r1 + m, g1 + m, b1 + m], "C9");
            }
        }
    }
    // The six primaries/secondaries plus black and white.
    for src in [
        [1.0f32, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 1.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
    ] {
        assert_same(&src, "C9/pinned");
    }
}

// ---------------------------------------------------------------------------
// C10 — all channels negative
// ---------------------------------------------------------------------------
#[test]
fn cfg_c10_all_negative() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..ITERS {
        let scale = match rng.below(4) {
            0 => 1.0f32,
            1 => 1e-20,
            2 => 1e20,
            _ => f32::MAX / 4.0,
        };
        let src = [
            -rng.unit() * scale,
            -rng.unit() * scale,
            -rng.unit() * scale,
        ];
        assert_same(&src, "C10");
    }
    for src in [
        [-1.0f32, -2.0, -3.0],
        [-3.0, -2.0, -1.0],
        [-1.0, -3.0, -2.0],
        [-0.5, -0.5, -1.0],
    ] {
        assert_same(&src, "C10/pinned");
    }
}

// ---------------------------------------------------------------------------
// C11 — mixed sign (includes max == 0 with delta > 0)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c11_mixed_sign() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..ITERS {
        let pick = |rng: &mut Rng| -> f32 {
            match rng.below(5) {
                0 => 0.0,
                1 => -0.0,
                2 => rng.range(-1.0, 0.0),
                3 => rng.range(0.0, 1.0),
                _ => rng.range(-1e6, 1e6),
            }
        };
        let src = [pick(&mut rng), pick(&mut rng), pick(&mut rng)];
        assert_same(&src, "C11");
    }
    // max == 0 with delta != 0 (ERRORS.md E2) in each channel position.
    for src in [
        [-1.0f32, 0.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, -1.0],
        [-1.0, -2.0, 0.0],
        [0.0, -0.0, -5.0],
    ] {
        let out = check(&src, "C11/max==0");
        assert_eq!(out[0].to_bits(), 0.0f32.to_bits());
        assert_eq!(out[1].to_bits(), 0.0f32.to_bits());
        assert_eq!(out[2], 0.0f32);
    }
}

// ---------------------------------------------------------------------------
// C12 — all 8 signed-zero combinations
// ---------------------------------------------------------------------------
#[test]
fn cfg_c12_signed_zero_grid() {
    let zeros = [0.0f32, -0.0f32];
    for &r in &zeros {
        for &g in &zeros {
            for &b in &zeros {
                assert_same(&[r, g, b], "C12");
            }
        }
    }
    // Signed zeros mixed with non-zeros, exhaustively over 3 positions.
    let vals = [0.0f32, -0.0, 1.0, -1.0];
    for &r in &vals {
        for &g in &vals {
            for &b in &vals {
                assert_same(&[r, g, b], "C12/mixed");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C13 — subnormals
// ---------------------------------------------------------------------------
#[test]
fn cfg_c13_subnormals() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..ITERS {
        let src = [rng.subnormal(), rng.subnormal(), rng.subnormal()];
        assert_same(&src, "C13");
    }
    // Subnormal / normal mixes.
    for _ in 0..ITERS {
        let src = [rng.subnormal(), rng.range(-1.0, 1.0), rng.subnormal()];
        assert_same(&src, "C13/mixed");
    }
    let tiny = f32::from_bits(1); // 1e-45, smallest positive subnormal
    for src in [
        [tiny, 0.0, 0.0],
        [0.0, tiny, 0.0],
        [0.0, 0.0, tiny],
        [-tiny, tiny, 0.0],
        [f32::MIN_POSITIVE, tiny, 0.0],
        [f32::MIN_POSITIVE, f32::MIN_POSITIVE, tiny],
    ] {
        assert_same(&src, "C13/pinned");
    }
}

// ---------------------------------------------------------------------------
// C14 — huge magnitudes (delta overflow)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c14_huge_magnitudes() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..ITERS {
        let pick = |rng: &mut Rng| -> f32 {
            let m = rng.unit() * f32::MAX;
            if rng.next_u32() & 1 == 0 {
                m
            } else {
                -m
            }
        };
        let src = [pick(&mut rng), pick(&mut rng), pick(&mut rng)];
        assert_same(&src, "C14");
    }
    let big = f32::MAX;
    for src in [
        [big, -big, 0.0],
        [-big, big, 0.0],
        [0.0, big, -big],
        [big, big, -big],
        [big, 0.0, 0.0],
        [-big, 0.0, 0.0],
        [big, big / 2.0, -big],
    ] {
        assert_same(&src, "C14/pinned");
    }
}

// ---------------------------------------------------------------------------
// C15 — wide exponent spread
// ---------------------------------------------------------------------------
#[test]
fn cfg_c15_wide_exponent_spread() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..ITERS {
        // Random sign + random exponent + random mantissa: covers the whole
        // finite range log-uniformly, hitting every exponent bucket.
        let pick = |rng: &mut Rng| -> f32 {
            let sign = (rng.next_u32() & 1) << 31;
            let exp = (rng.below(255) as u32) << 23; // 0..254 -> no inf/nan
            let man = rng.next_u32() & 0x007F_FFFF;
            f32::from_bits(sign | exp | man)
        };
        let src = [pick(&mut rng), pick(&mut rng), pick(&mut rng)];
        assert_same(&src, "C15");
    }
    for src in [
        [f32::MAX, f32::MIN_POSITIVE, 0.0],
        [f32::MIN_POSITIVE, f32::MAX, 1.0],
        [1e-38, 1e38, 1.0],
        [1e38, 1e-38, -1e-38],
    ] {
        assert_same(&src, "C15/pinned");
    }
}

// ---------------------------------------------------------------------------
// C16 — unconstrained bit-pattern fuzz
// ---------------------------------------------------------------------------
#[test]
fn cfg_c16_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..(ITERS * 5) {
        let src = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        assert_same(&src, "C16");
    }
}

// ---------------------------------------------------------------------------
// C17 — exhaustive non-finite / special-value grid
// ---------------------------------------------------------------------------
#[test]
fn cfg_c17_nonfinite_grid() {
    let vals: [f32; 10] = [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
    ];
    for &r in &vals {
        for &g in &vals {
            for &b in &vals {
                assert_same(&[r, g, b], "C17");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C18 — in-place, dest == src
// ---------------------------------------------------------------------------
#[test]
fn cfg_c18_inplace_alias() {
    let mut rng = Rng::new(SEED ^ 18);
    let c = common::c_fn();
    let ru = common::rust_fn();
    for i in 0..ITERS {
        let src: [f32; 3] = if i % 2 == 0 {
            [rng.unit(), rng.unit(), rng.unit()]
        } else {
            [rng.any_f32(), rng.any_f32(), rng.any_f32()]
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
            "C18 in-place divergence for {src:?}: C={bc:?} Rust={br:?}"
        );
        // And the in-place result must equal the disjoint-buffer result.
        let mut dj = [0.0f32; 3];
        unsafe { c(dj.as_mut_ptr(), src.as_ptr()) };
        assert_eq!(
            common::bits3(&bc),
            common::bits3(&dj),
            "C18 aliasing changed the C result for {src:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// C19 — partial overlap dest = src +/- 1
// ---------------------------------------------------------------------------
#[test]
fn cfg_c19_partial_overlap() {
    let mut rng = Rng::new(SEED ^ 19);
    let c = common::c_fn();
    let ru = common::rust_fn();
    for i in 0..ITERS {
        let vals: [f32; 5] = [
            rng.any_f32(),
            if i % 2 == 0 { rng.unit() } else { rng.any_f32() },
            if i % 2 == 0 { rng.unit() } else { rng.any_f32() },
            if i % 2 == 0 { rng.unit() } else { rng.any_f32() },
            rng.any_f32(),
        ];
        for shift in [1isize, -1] {
            let mut bc = vals;
            let mut br = vals;
            // src at index 1..4; dest at index 1+shift.
            unsafe {
                let sc = bc.as_ptr().add(1);
                let dc = bc.as_mut_ptr().offset(1 + shift);
                c(dc, sc);
                let sr = br.as_ptr().add(1);
                let dr = br.as_mut_ptr().offset(1 + shift);
                ru(dr, sr);
            }
            let bcb: Vec<u32> = bc.iter().map(|x| x.to_bits()).collect();
            let brb: Vec<u32> = br.iter().map(|x| x.to_bits()).collect();
            assert_eq!(
                bcb, brb,
                "C19 overlap(shift={shift}) divergence for {vals:?}: C={bc:?} Rust={br:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C20 — 8-bit quantised pixel values
// ---------------------------------------------------------------------------
#[test]
fn cfg_c20_u8_quantised() {
    let q = |k: u32| k as f32 / 255.0;
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..ITERS {
        let src = [
            q(rng.below(256) as u32),
            q(rng.below(256) as u32),
            q(rng.below(256) as u32),
        ];
        assert_same(&src, "C20");
    }
    // Full grey ramp (delta == 0 for every step).
    for k in 0..=255u32 {
        assert_same(&[q(k), q(k), q(k)], "C20/grey");
    }
    // Saturated ramps along each axis.
    for k in 0..=255u32 {
        assert_same(&[q(255), q(k), q(0)], "C20/ramp-r");
        assert_same(&[q(0), q(255), q(k)], "C20/ramp-g");
        assert_same(&[q(k), q(0), q(255)], "C20/ramp-b");
    }
}

// ---------------------------------------------------------------------------
// C21 — bulk sequential invocation (no cross-call state)
// ---------------------------------------------------------------------------
#[test]
fn cfg_c21_bulk_sequence() {
    let mut rng = Rng::new(SEED ^ 21);
    const N: usize = 50_000;
    let mut input = Vec::with_capacity(N * 3);
    for i in 0..N * 3 {
        input.push(if i % 7 == 0 { rng.any_f32() } else { rng.unit() });
    }
    let mut oc = vec![0.0f32; N * 3];
    let mut or = vec![0.0f32; N * 3];
    let c = common::c_fn();
    let ru = common::rust_fn();
    unsafe {
        for i in 0..N {
            c(oc.as_mut_ptr().add(i * 3), input.as_ptr().add(i * 3));
        }
        for i in 0..N {
            ru(or.as_mut_ptr().add(i * 3), input.as_ptr().add(i * 3));
        }
    }
    for i in 0..N * 3 {
        assert_eq!(
            oc[i].to_bits(),
            or[i].to_bits(),
            "C21 divergence at element {i} (pixel {}): C={} Rust={}",
            i / 3,
            oc[i],
            or[i]
        );
    }

    // Same buffer, reversed traversal order, to rule out order-dependent state.
    let mut oc2 = vec![0.0f32; N * 3];
    let mut or2 = vec![0.0f32; N * 3];
    unsafe {
        for i in (0..N).rev() {
            c(oc2.as_mut_ptr().add(i * 3), input.as_ptr().add(i * 3));
            ru(or2.as_mut_ptr().add(i * 3), input.as_ptr().add(i * 3));
        }
    }
    for i in 0..N * 3 {
        assert_eq!(oc2[i].to_bits(), or2[i].to_bits(), "C21 reversed divergence at {i}");
        assert_eq!(oc2[i].to_bits(), oc[i].to_bits(), "C21 C not order-independent at {i}");
    }
}
