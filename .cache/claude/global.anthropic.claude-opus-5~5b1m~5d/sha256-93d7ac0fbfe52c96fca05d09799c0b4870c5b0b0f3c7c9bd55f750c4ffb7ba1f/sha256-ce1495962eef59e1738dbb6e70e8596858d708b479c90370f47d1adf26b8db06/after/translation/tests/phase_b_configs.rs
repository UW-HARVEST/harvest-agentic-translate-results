//! Phase B — valid-path differential tests, one `#[test]` per row of
//! `CONFIGS.md`. Every test drives BOTH shared objects through their exported
//! `hsv_to_rgb` symbol and compares raw bit patterns.

mod common;

use common::*;

/// Hues that land in a given `switch` arm, sampled randomly inside the arm's
/// half-open interval.
fn arm_hue(rng: &mut Rng, arm: i32) -> f32 {
    let (lo, hi) = hue_range_for_arm(arm);
    rng.range(lo, hi)
}

/// `s` values in `(0, 1]` (never exactly 0, so the main path is taken).
fn s_nonzero(rng: &mut Rng) -> f32 {
    let s = rng.range(f32::MIN_POSITIVE, 1.0);
    if s == 0.0 {
        1.0
    } else {
        s
    }
}

const ARMS: [i32; 8] = [0, 1, 2, 3, 4, 5, 6, -1];

// ===========================================================================
// rows 1-9: the `s == 0` early-return configuration
// ===========================================================================

#[test]
fn b01_s_zero_random() {
    let mut rng = Rng::new(0x0001);
    for _ in 0..20_000 {
        let h = rng.range(-1000.0, 1000.0);
        let v = rng.range(-2.0, 2.0);
        assert_same("b01", h, 0.0, v);
    }
}

#[test]
fn b02_s_negative_zero_random() {
    let mut rng = Rng::new(0x0002);
    for _ in 0..20_000 {
        let h = rng.range(-1000.0, 1000.0);
        let v = rng.range(-2.0, 2.0);
        assert_same("b02", h, -0.0, v);
    }
}

#[test]
fn b03_s_zero_v_special() {
    let mut rng = Rng::new(0x0003);
    let vs: Vec<f32> = SPECIAL.iter().copied().chain(nans()).collect();
    for &s in &[0.0f32, -0.0f32] {
        for &v in &vs {
            for _ in 0..64 {
                let h = rng.range(-4000.0, 4000.0);
                assert_same("b03", h, s, v);
            }
            assert_same("b03", 0.0, s, v);
            assert_same("b03", f32::NAN, s, v);
        }
    }
}

#[test]
fn b04_s_zero_h_special() {
    let mut rng = Rng::new(0x0004);
    let hs: Vec<f32> = SPECIAL.iter().copied().chain(nans()).collect();
    for &s in &[0.0f32, -0.0f32] {
        for &h in &hs {
            for _ in 0..64 {
                let v = rng.range(-3.0, 3.0);
                assert_same("b04", h, s, v);
            }
            for &v in &[0.0f32, -0.0, 1.0, f32::INFINITY, f32::NAN] {
                assert_same("b04", h, s, v);
            }
        }
    }
}

#[test]
fn b05_s_zero_in_place() {
    let mut rng = Rng::new(0x0005);
    for _ in 0..5_000 {
        let h = rng.range(-1000.0, 1000.0);
        let v = rng.range(-2.0, 2.0);
        for &s in &[0.0f32, -0.0f32] {
            // dest == src
            assert_same_shaped("b05", [h, s, v], 16, 16);
        }
    }
}

#[test]
fn b06_s_zero_overlap_one() {
    let mut rng = Rng::new(0x0006);
    for _ in 0..5_000 {
        let h = rng.range(-1000.0, 1000.0);
        let v = rng.range(-2.0, 2.0);
        for &s in &[0.0f32, -0.0f32] {
            assert_same_shaped("b06 dest=src+1", [h, s, v], 16, 20);
            assert_same_shaped("b06 src=dest+1", [h, s, v], 20, 16);
        }
    }
}

#[test]
fn b07_s_zero_overlap_two() {
    let mut rng = Rng::new(0x0007);
    for _ in 0..5_000 {
        let h = rng.range(-1000.0, 1000.0);
        let v = rng.range(-2.0, 2.0);
        for &s in &[0.0f32, -0.0f32] {
            assert_same_shaped("b07 dest=src+2", [h, s, v], 16, 24);
            assert_same_shaped("b07 src=dest+2", [h, s, v], 24, 16);
        }
    }
}

#[test]
fn b08_s_zero_misaligned() {
    let mut rng = Rng::new(0x0008);
    for _ in 0..2_000 {
        let h = rng.range(-1000.0, 1000.0);
        let v = rng.range(-2.0, 2.0);
        for &s in &[0.0f32, -0.0f32] {
            for src_off in [1usize, 2, 3, 5, 6, 7] {
                for dst_off in [1usize, 2, 3, 4, 33, 34, 35] {
                    assert_same_shaped("b08", [h, s, v], src_off, dst_off + 16);
                }
            }
        }
    }
}

#[test]
fn b09_s_zero_extent() {
    // `run_pair`/`assert_same` already assert the canaries around `dest[0..3]`
    // and that `src` is unmodified; `run_shaped` compares the whole 64-byte
    // buffer, which additionally pins *where* the writes land.
    let mut rng = Rng::new(0x0009);
    for _ in 0..5_000 {
        let h = rng.range(-1000.0, 1000.0);
        let v = rng.range(-2.0, 2.0);
        assert_same("b09 canary", h, 0.0, v);
        assert_same_shaped("b09 extent", [h, 0.0, v], 4, 32);
        assert_same_shaped("b09 extent", [h, -0.0, v], 4, 32);
    }
}

// ===========================================================================
// rows 10-17: one row per `switch` arm, randomized inside the arm
// ===========================================================================

fn arm_row(label: &str, seed: u64, arm: i32) {
    let mut rng = Rng::new(seed);
    for _ in 0..20_000 {
        let h = arm_hue(&mut rng, arm);
        let s = s_nonzero(&mut rng);
        let v = rng.unit();
        assert_same(label, h, s, v);
    }
    // plus the exact endpoints of the arm
    let (lo, hi) = hue_range_for_arm(arm);
    for h in [lo, next_down(hi), (lo + hi) * 0.5] {
        for s in [f32::MIN_POSITIVE, 1e-40, 0.5, 1.0] {
            for v in [0.0, 0.5, 1.0] {
                assert_same(label, h, s, v);
            }
        }
    }
}

/// The `f32` immediately below `x` in IEEE ordering (no `f32::next_down`
/// dependency, and no bit-pattern overflow at the zeros).
fn next_down(x: f32) -> f32 {
    let b = x.to_bits();
    if b == 0 {
        f(0x8000_0001) // next below +0.0 is the smallest negative subnormal
    } else if x.is_sign_negative() {
        f(b + 1)
    } else {
        f(b - 1)
    }
}

/// The `f32` immediately above `x`.
fn next_up(x: f32) -> f32 {
    let b = x.to_bits();
    if b == 0x8000_0000 {
        f(1) // next above -0.0 is the smallest positive subnormal
    } else if x.is_sign_negative() {
        f(b - 1)
    } else {
        f(b + 1)
    }
}

#[test]
fn b10_arm0_random() {
    arm_row("b10", 0x0010, 0);
}

#[test]
fn b11_arm1_random() {
    arm_row("b11", 0x0011, 1);
}

#[test]
fn b12_arm2_random() {
    arm_row("b12", 0x0012, 2);
}

#[test]
fn b13_arm3_random() {
    arm_row("b13", 0x0013, 3);
}

#[test]
fn b14_arm4_random() {
    arm_row("b14", 0x0014, 4);
}

#[test]
fn b15_arm5_default_random() {
    arm_row("b15", 0x0015, 5);
}

#[test]
fn b16_arm_ge6_default_random() {
    let mut rng = Rng::new(0x0016);
    for _ in 0..20_000 {
        let h = rng.range(360.0, 3600.0);
        let s = s_nonzero(&mut rng);
        let v = rng.unit();
        assert_same("b16", h, s, v);
    }
    for k in 6..64 {
        arm_row("b16 arm", 0x1600 + k as u64, k);
    }
}

#[test]
fn b17_arm_negative_default_random() {
    let mut rng = Rng::new(0x0017);
    for _ in 0..20_000 {
        let h = rng.range(-3600.0, -1e-6);
        let s = s_nonzero(&mut rng);
        let v = rng.unit();
        assert_same("b17", h, s, v);
    }
    for k in -64..0 {
        arm_row("b17 arm", 0x1700 + (k + 64) as u64, k);
    }
}

// ===========================================================================
// rows 18-25: `h` value classes
// ===========================================================================

#[test]
fn b18_hue_exact_multiples() {
    let mut rng = Rng::new(0x0018);
    for k in -64..=64 {
        let h = k as f32 * 60.0;
        for _ in 0..256 {
            let s = s_nonzero(&mut rng);
            let v = rng.range(-2.0, 2.0);
            assert_same("b18", h, s, v);
        }
        for &s in &[f32::MIN_POSITIVE, 1e-40, 0.25, 1.0, 1.5, -0.5, f32::INFINITY] {
            for &v in &[0.0f32, -0.0, 0.5, 1.0, f32::INFINITY, f32::NAN] {
                assert_same("b18", h, s, v);
            }
        }
    }
}

#[test]
fn b19_hue_next_to_multiples() {
    let mut rng = Rng::new(0x0019);
    for k in -8..=8 {
        let base = k as f32 * 60.0;
        // 8 steps either side of every arm boundary
        let mut hs = vec![base];
        let mut up = base;
        let mut dn = base;
        for _ in 0..8 {
            up = next_up(up);
            dn = next_down(dn);
            hs.push(up);
            hs.push(dn);
        }
        for &h in &hs {
            for _ in 0..64 {
                let s = s_nonzero(&mut rng);
                let v = rng.unit();
                assert_same("b19", h, s, v);
            }
            for &s in &[1.0f32, 0.5, 1e-40] {
                for &v in &[0.0f32, 0.5, 1.0] {
                    assert_same("b19", h, s, v);
                }
            }
        }
    }
}

#[test]
fn b20_hue_signed_zero() {
    let mut rng = Rng::new(0x0020);
    for &h in &[0.0f32, -0.0f32] {
        for _ in 0..5_000 {
            let s = s_nonzero(&mut rng);
            let v = rng.range(-2.0, 2.0);
            assert_same("b20", h, s, v);
        }
        for &s in &SPECIAL[..] {
            for &v in &[0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NAN] {
                assert_same("b20", h, s, v);
            }
        }
    }
}

#[test]
fn b21_hue_subnormal() {
    let mut rng = Rng::new(0x0021);
    let hs = [
        1e-45f32,
        -1e-45,
        1e-40,
        -1e-40,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::EPSILON,
        -f32::EPSILON,
        f(1),      // smallest positive subnormal bit pattern
        f(0x8000_0001),
    ];
    for &h in &hs {
        for _ in 0..2_000 {
            let s = s_nonzero(&mut rng);
            let v = rng.range(-2.0, 2.0);
            assert_same("b21", h, s, v);
        }
        for &s in SPECIAL {
            for &v in SPECIAL {
                assert_same("b21", h, s, v);
            }
        }
    }
}

#[test]
fn b22_hue_huge_in_int_range() {
    let mut rng = Rng::new(0x0022);
    // h/60 stays inside [-2^31, 2^31) -> `i` is a large but representable int
    let hs = [
        6.0e10f32,
        -6.0e10,
        1.28e11,      // ~2^31 * 60 / 1.007
        -1.28e11,
        60.0 * 2_147_483_520.0_f32, // largest multiple of 60 with i < 2^31
        -60.0 * 2_147_483_520.0_f32,
        60.0 * 16_777_216.0,
        1e9,
        -1e9,
        16_777_216.0,
    ];
    for &h in &hs {
        for _ in 0..2_000 {
            let s = s_nonzero(&mut rng);
            let v = rng.range(-2.0, 2.0);
            assert_same("b22", h, s, v);
        }
        for &s in SPECIAL {
            for &v in &[0.0f32, 1.0, -1.0, f32::INFINITY, f32::NAN] {
                assert_same("b22", h, s, v);
            }
        }
    }
}

#[test]
fn b23_hue_int_conversion_boundary() {
    let mut rng = Rng::new(0x0023);
    let two31 = 2_147_483_648.0f32;
    let mut hs = vec![
        two31 * 60.0,
        -two31 * 60.0,
        f32::MAX,
        f32::MIN,
        1e30,
        -1e30,
        3.4e38,
        -3.4e38,
    ];
    // hues whose quotient sits exactly on / next to +-2^31
    for base in [two31, -two31, 2_147_483_520.0, -2_147_483_520.0] {
        let h = base * 60.0;
        hs.push(h);
        hs.push(f(h.to_bits() + 1));
        hs.push(f(h.to_bits() - 1));
    }
    for &h in &hs {
        for _ in 0..1_000 {
            let s = s_nonzero(&mut rng);
            let v = rng.range(-2.0, 2.0);
            assert_same("b23", h, s, v);
        }
        for &s in SPECIAL {
            for &v in SPECIAL {
                assert_same("b23", h, s, v);
            }
        }
    }
}

#[test]
fn b24_hue_infinite() {
    let mut rng = Rng::new(0x0024);
    for &h in &[f32::INFINITY, f32::NEG_INFINITY] {
        for _ in 0..2_000 {
            let s = s_nonzero(&mut rng);
            let v = rng.range(-2.0, 2.0);
            assert_same("b24", h, s, v);
        }
        for &s in SPECIAL {
            for &v in SPECIAL {
                assert_same("b24", h, s, v);
            }
        }
        for s in nans() {
            for &v in SPECIAL {
                assert_same("b24 nan s", h, s, v);
            }
        }
    }
}

#[test]
fn b25_hue_nan() {
    let mut rng = Rng::new(0x0025);
    for h in nans() {
        for _ in 0..2_000 {
            let s = s_nonzero(&mut rng);
            let v = rng.range(-2.0, 2.0);
            assert_same("b25", h, s, v);
        }
        for &s in SPECIAL {
            for &v in SPECIAL {
                assert_same("b25", h, s, v);
            }
        }
        for s in nans() {
            for v in nans() {
                assert_same("b25 all-nan", h, s, v);
            }
        }
    }
}

// ===========================================================================
// rows 26-33: `s` and `v` value classes, crossed with every arm
// ===========================================================================

fn per_arm_s(label: &str, seed: u64, s_values: &[f32]) {
    let mut rng = Rng::new(seed);
    for &arm in &ARMS {
        for &s in s_values {
            for _ in 0..512 {
                let h = arm_hue(&mut rng, arm);
                let v = rng.range(-2.0, 2.0);
                assert_same(label, h, s, v);
            }
            let (lo, hi) = hue_range_for_arm(arm);
            for &h in &[lo, next_down(hi), (lo + hi) * 0.5] {
                for &v in SPECIAL {
                    assert_same(label, h, s, v);
                }
                for v in nans() {
                    assert_same(label, h, s, v);
                }
            }
        }
    }
}

#[test]
fn b26_s_exactly_one() {
    per_arm_s("b26", 0x0026, &[1.0]);
}

#[test]
fn b27_s_subnormal() {
    per_arm_s(
        "b27",
        0x0027,
        &[1e-45, 1e-40, f32::MIN_POSITIVE, f(1), f32::EPSILON],
    );
}

#[test]
fn b28_s_above_one() {
    per_arm_s("b28", 0x0028, &[1.5, 2.0, 1e30, f32::MAX, 16_777_216.0]);
}

#[test]
fn b29_s_negative() {
    per_arm_s(
        "b29",
        0x0029,
        &[-1e-45, -1e-40, -0.5, -1.0, -1e30, f32::MIN],
    );
}

#[test]
fn b30_s_infinite() {
    per_arm_s("b30", 0x0030, &[f32::INFINITY, f32::NEG_INFINITY]);
}

#[test]
fn b31_s_nan() {
    let all_nans: Vec<f32> = nans().collect();
    per_arm_s("b31", 0x0031, &all_nans);
}

#[test]
fn b32_v_zero() {
    let mut rng = Rng::new(0x0032);
    for &arm in &ARMS {
        for &v in &[0.0f32, -0.0f32] {
            for _ in 0..2_000 {
                let h = arm_hue(&mut rng, arm);
                let s = s_nonzero(&mut rng);
                assert_same("b32", h, s, v);
            }
            for &s in SPECIAL {
                if s == 0.0 {
                    continue; // that is the row-1 configuration
                }
                let (lo, hi) = hue_range_for_arm(arm);
                for &h in &[lo, next_down(hi), (lo + hi) * 0.5] {
                    assert_same("b32", h, s, v);
                }
            }
            // 0 * inf -> invalid operation
            for &s in &[f32::INFINITY, f32::NEG_INFINITY] {
                assert_same("b32 0*inf", arm as f32 * 60.0 + 1.0, s, v);
            }
        }
    }
}

#[test]
fn b33_v_special() {
    let mut rng = Rng::new(0x0033);
    let vs: Vec<f32> = SPECIAL.iter().copied().chain(nans()).collect();
    for &arm in &ARMS {
        for &v in &vs {
            for &s in &[0.25f32, 1.0, 1.5, -0.5, f32::INFINITY, 1e-40] {
                for _ in 0..32 {
                    let h = arm_hue(&mut rng, arm);
                    assert_same("b33", h, s, v);
                }
            }
        }
    }
}

// ===========================================================================
// rows 34-40: whole-domain fuzz and the shape / sequencing axes on the main path
// ===========================================================================

#[test]
fn b34_full_random_bitpatterns() {
    // Iteration count and seed are overridable so the same row can be re-run as
    // a long soak: `HSV_FUZZ_ITERS=10000000 HSV_FUZZ_SEED=7 cargo test b34`.
    let iters: u64 = std::env::var("HSV_FUZZ_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300_000);
    let seed: u64 = std::env::var("HSV_FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x0034);
    let mut rng = Rng::new(seed);
    for _ in 0..iters {
        let h = rng.any_f32();
        let s = rng.any_f32();
        let v = rng.any_f32();
        assert_same("b34", h, s, v);
    }
}

#[test]
fn b35_in_place_main_path() {
    let mut rng = Rng::new(0x0035);
    for &arm in &ARMS {
        for _ in 0..2_000 {
            let h = arm_hue(&mut rng, arm);
            let s = s_nonzero(&mut rng);
            let v = rng.range(-2.0, 2.0);
            assert_same_shaped("b35", [h, s, v], 16, 16);
        }
    }
    for _ in 0..20_000 {
        let src = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        assert_same_shaped("b35 fuzz", src, 16, 16);
    }
}

#[test]
fn b36_overlap_main_path() {
    let mut rng = Rng::new(0x0036);
    let pairs = [(16, 20), (20, 16), (16, 24), (24, 16), (16, 28), (28, 16)];
    for &arm in &ARMS {
        for _ in 0..1_000 {
            let h = arm_hue(&mut rng, arm);
            let s = s_nonzero(&mut rng);
            let v = rng.range(-2.0, 2.0);
            for (so, dof) in pairs {
                assert_same_shaped("b36", [h, s, v], so, dof);
            }
        }
    }
    for _ in 0..10_000 {
        let src = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        for (so, dof) in pairs {
            assert_same_shaped("b36 fuzz", src, so, dof);
        }
    }
}

#[test]
fn b37_misaligned_main_path() {
    let mut rng = Rng::new(0x0037);
    for &arm in &ARMS {
        for _ in 0..200 {
            let h = arm_hue(&mut rng, arm);
            let s = s_nonzero(&mut rng);
            let v = rng.range(-2.0, 2.0);
            for src_off in [1usize, 2, 3, 5, 7] {
                for dst_off in [17usize, 18, 19, 20, 33, 35] {
                    assert_same_shaped("b37", [h, s, v], src_off, dst_off);
                    // overlapping *and* misaligned
                    assert_same_shaped("b37 overlap", [h, s, v], src_off, src_off);
                    assert_same_shaped("b37 overlap", [h, s, v], src_off, src_off + 1);
                }
            }
        }
    }
    for _ in 0..5_000 {
        let src = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        assert_same_shaped("b37 fuzz", src, 3, 19);
    }
}

#[test]
fn b38_extent_main_path() {
    let mut rng = Rng::new(0x0038);
    for &arm in &ARMS {
        for _ in 0..2_000 {
            let h = arm_hue(&mut rng, arm);
            let s = s_nonzero(&mut rng);
            let v = rng.range(-2.0, 2.0);
            // canary + src-immutability assertions live inside `run_pair`
            assert_same("b38 canary", h, s, v);
            // exact placement of the three stores
            assert_same_shaped("b38 extent", [h, s, v], 0, 40);
            assert_same_shaped("b38 extent", [h, s, v], 52, 4);
        }
    }
}

#[test]
fn b39_stateless_interleaved() {
    let mut rng = Rng::new(0x0039);
    // Identical inputs must give identical outputs no matter what ran before,
    // and neither object may leave state (e.g. a modified MXCSR) behind.
    let probe = [30.0f32, 0.5, 0.75];
    let baseline = {
        let (cc, rr) = run_pair(probe[0], probe[1], probe[2]);
        assert_eq!(cc, rr);
        cc
    };
    for _ in 0..20_000 {
        let h = rng.any_f32();
        let s = rng.any_f32();
        let v = rng.any_f32();
        // randomize which implementation is called first
        let c_first = rng.next_u32() & 1 == 0;
        let mut co = [0f32; 3];
        let mut ro = [0f32; 3];
        let src = [h, s, v];
        unsafe {
            if c_first {
                c().call(co.as_mut_ptr(), src.as_ptr());
                rust().call(ro.as_mut_ptr(), src.as_ptr());
            } else {
                rust().call(ro.as_mut_ptr(), src.as_ptr());
                c().call(co.as_mut_ptr(), src.as_ptr());
            }
        }
        assert_eq!(
            bits3(&co),
            bits3(&ro),
            "b39: divergence (c_first={c_first}) for h={} s={} v={}",
            show(h),
            show(s),
            show(v)
        );
        // repeat the same call twice in a row: must be idempotent
        let mut co2 = [0f32; 3];
        let mut ro2 = [0f32; 3];
        unsafe {
            c().call(co2.as_mut_ptr(), src.as_ptr());
            rust().call(ro2.as_mut_ptr(), src.as_ptr());
        }
        assert_eq!(bits3(&co2), bits3(&co), "b39: C not idempotent");
        assert_eq!(bits3(&ro2), bits3(&ro), "b39: Rust not idempotent");
        // and the fixed probe still gives the baseline answer
        let (cc, rr) = run_pair(probe[0], probe[1], probe[2]);
        assert_eq!(cc, baseline, "b39: C state leaked");
        assert_eq!(rr, baseline, "b39: Rust state leaked");
    }
}

#[test]
fn b40_grid_sweep() {
    let svs = [0.0f32, 1e-45, 0.25, 0.5, 0.75, 1.0, 1.5];
    let mut h = -720.0f32;
    while h <= 1080.0 {
        for &s in &svs {
            for &v in &svs {
                assert_same("b40", h, s, v);
            }
        }
        h += 0.25;
    }
}

// ===========================================================================
// rows 41-42: extra depth on the two axes where a real divergence was found
// (SSE NaN-operand selection) and on the raw bit-pattern space
// ===========================================================================

/// A wide spread of NaN encodings: canonical, quiet-with-payload, signalling,
/// both signs, minimal and maximal payloads.
fn nan_payloads() -> Vec<f32> {
    let mut v = Vec::new();
    for sign in [0u32, 0x8000_0000] {
        for payload in [
            0x0000_0001u32,
            0x0000_0002,
            0x0000_1234,
            0x0010_0000,
            0x0020_0000,
            0x003F_FFFF, // largest signalling payload
            0x0040_0000, // canonical quiet
            0x0040_0001,
            0x0050_5050,
            0x0060_0000,
            0x007F_FFFF, // largest quiet payload
            0x0055_5555,
        ] {
            v.push(f(sign | 0x7F80_0000 | payload));
        }
    }
    v
}

#[test]
fn b41_nan_payload_cross_product() {
    let nan = nan_payloads();
    assert_eq!(nan.len(), 24);
    // every (s, v) NaN pair against every arm: this is the exact configuration
    // in which `mulss`'s destination-operand-wins rule becomes observable.
    for &arm in &ARMS {
        let h = arm as f32 * 60.0 + 17.0;
        for &s in &nan {
            for &v in &nan {
                assert_same("b41 s,v nan", h, s, v);
            }
        }
    }
    // every (h, s) and (h, v) NaN pair
    for &s in &nan {
        for &v in &nan {
            for h in nan.iter().copied() {
                assert_same("b41 all nan", h, s, v);
            }
        }
    }
    // one NaN at a time, mixed with finite / infinite partners
    let finite = [0.0f32, -0.0, 1e-40, 0.25, 1.0, 1.5, -0.5, f32::INFINITY, f32::NEG_INFINITY];
    for &n in &nan {
        for &arm in &ARMS {
            let h = arm as f32 * 60.0 + 17.0;
            for &x in &finite {
                assert_same("b41 nan h", n, x.max(f32::MIN_POSITIVE), x);
                assert_same("b41 nan s", h, n, x);
                assert_same("b41 nan v", h, x, n);
            }
        }
    }
}

#[test]
fn b42_strided_bitpattern_sweeps() {
    // Walk the whole 2^32 bit-pattern space of one axis with a large prime
    // stride (hits every exponent, both signs, subnormals, infinities and NaNs)
    // while the other two axes are pinned to representative values.
    const STRIDE: u32 = 65_521; // prime
    let others: [(f32, f32); 6] = [
        (0.5, 0.75),
        (1.0, 1.0),
        (1e-40, 1.0),
        (1.5, -0.5),
        (f32::INFINITY, 0.0),
        (f(0x7FC0_1234), f(0xFF80_0001)),
    ];
    // sweep h
    for &(s, v) in &others {
        let mut bits: u32 = 0;
        loop {
            assert_same("b42 h sweep", f(bits), s, v);
            match bits.checked_add(STRIDE) {
                Some(n) => bits = n,
                None => break,
            }
        }
    }
    // sweep s
    for &(h, v) in &[(30.0f32, 0.75f32), (330.0, 0.5), (-30.0, 1.0), (f32::NAN, 0.25)] {
        let mut bits: u32 = 0;
        loop {
            assert_same("b42 s sweep", h, f(bits), v);
            match bits.checked_add(STRIDE) {
                Some(n) => bits = n,
                None => break,
            }
        }
    }
    // sweep v
    for &(h, s) in &[(30.0f32, 0.75f32), (330.0, 1.5), (-30.0, -0.5), (f32::NAN, f32::NAN)] {
        let mut bits: u32 = 0;
        loop {
            assert_same("b42 v sweep", h, s, f(bits));
            match bits.checked_add(STRIDE) {
                Some(n) => bits = n,
                None => break,
            }
        }
    }
}
