//! Phase C — error/boundary-path differential tests, one per `ERRORS.md` row.
//!
//! `hsl_to_rgb` is `void` and validates nothing, so there is no error code to
//! compare; the "same rejection" assertion is therefore "the same three output
//! words, bit-for-bit", which is the function's entire observable result. Rows
//! E16/E17 (null / too-short buffers) are undefined behaviour in the C and are
//! deliberately NOT invoked — see `ERRORS.md`.

mod common;

use common::{Libs, Rng, SEED};

/// E1: `s == 0.0f` takes the early `return` and ignores the hue completely.
#[test]
fn err_s_zero_short_circuits() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xE1);
    for i in 0..5_000 {
        let h = r.any_f32(); // must be irrelevant
        let l = r.any_f32();
        let (c, rs) = libs.call_both([h, 0.0, l]);
        assert_eq!(common::bits(&c), common::bits(&rs), "E1 i={i}");
        // And it really is the pass-through of l in both.
        assert_eq!(
            common::bits(&c),
            [l.to_bits(); 3],
            "E1 C did not store l,l,l (i={i})"
        );
    }
}

/// E2: `s == -0.0f` compares equal to zero, so it short-circuits too.
#[test]
fn err_s_negative_zero() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xE2);
    for i in 0..5_000 {
        let h = r.any_f32();
        let l = r.any_f32();
        libs.assert_same([h, -0.0, l], &format!("E2 i={i}"));
    }
    for l in [0.0f32, -0.0, 1.0, f32::NAN, f32::INFINITY] {
        let (c, _) = libs.call_both([12.0, -0.0, l]);
        assert_eq!(common::bits(&c), [l.to_bits(); 3], "E2 short-circuit");
    }
}

/// E3: the smallest non-zero subnormal saturation must NOT short-circuit.
#[test]
fn err_s_smallest_subnormal() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xE3);
    // Note on observability: for a *subnormal* `s`, `c = (1-|2l-1|)*s` has a
    // magnitude of at most `s`, so `0.5*c` underflows relative to any normal `l`
    // and `m` rounds back to `l`; `x = (1-|fmod-1|)*c` likewise rounds to `±0`.
    // The formula therefore yields the same `l,l,l` bits that the `s == 0`
    // short-circuit would — indistinguishable by result. What IS observable is
    // that a *normal* tiny `s` does not short-circuit, so pin that first, then
    // run the differential sweep over the subnormal saturations.
    let tiny_normal = f32::MIN_POSITIVE; // 1.175e-38, smallest normal
    let (cn, _) = libs.call_both([30.0, tiny_normal, 0.5]);
    assert_eq!(
        cn.map(f32::to_bits),
        [0.5f32.to_bits(); 3],
        "E3 baseline: c underflows away, so the C also yields l,l,l here"
    );
    // A saturation big enough to be visible must NOT give the l,l,l pattern.
    let (cv, _) = libs.call_both([30.0, 1.0e-6, 0.5]);
    assert_ne!(
        cv.map(f32::to_bits),
        [0.5f32.to_bits(); 3],
        "E3 a visible non-zero s must not behave like the s==0 short-circuit"
    );
    for s in [
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x0000_FFFF),
        f32::from_bits(0x007F_FFFF),
        f32::from_bits(0x807F_FFFF),
    ] {
        for i in 0..3_000 {
            let h = r.range(-500.0, 900.0);
            let l = r.range(-2.0, 3.0);
            libs.assert_same([h, s, l], &format!("E3 s=0x{:08x} i={i}", s.to_bits()));
        }
    }
}

/// E4: `s` NaN — `NaN == 0` is false, so the full formula runs.
#[test]
fn err_s_nan_no_short_circuit() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xE4);
    for _ in 0..200 {
        let s = r.any_nan();
        for i in 0..30 {
            let h = r.range(-500.0, 900.0);
            let l = r.range(-2.0, 3.0);
            libs.assert_same([h, s, l], &format!("E4 s=0x{:08x} i={i}", s.to_bits()));
        }
    }
    // Explicit: a NaN s must not produce the l,l,l short-circuit pattern.
    let (c, _) = libs.call_both([10.0, f32::NAN, 0.25]);
    assert_ne!(common::bits(&c), [0.25f32.to_bits(); 3], "E4 short-circuited");
}

/// E5: `h` NaN → every comparison false → final `else` → `m, m, m`.
#[test]
fn err_h_nan_falls_to_else() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xE5);
    for _ in 0..200 {
        let h = r.any_nan();
        for i in 0..30 {
            let s = r.range(0.0001, 2.0);
            let l = r.range(-2.0, 3.0);
            let (c, rs) = libs.call_both([h, s, l]);
            assert_eq!(common::bits(&c), common::bits(&rs), "E5 i={i}");
            assert_eq!(c[0].to_bits(), c[1].to_bits(), "E5 C not grey");
            assert_eq!(c[1].to_bits(), c[2].to_bits(), "E5 C not grey");
        }
    }
}

/// E6: `h` one ULP past each dispatch boundary, both directions.
#[test]
fn err_h_boundary_nextafter() {
    let libs = Libs::load();
    let bounds = [0.0f32, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0];
    for &b in &bounds {
        let mut probes = vec![b];
        if b != 0.0 {
            probes.push(f32::from_bits(b.to_bits() - 1)); // nextafter toward 0
            probes.push(f32::from_bits(b.to_bits() + 1)); // nextafter toward +inf
            probes.push(-b);
            probes.push(f32::from_bits((-b).to_bits() - 1));
            probes.push(f32::from_bits((-b).to_bits() + 1));
        } else {
            probes.push(-0.0);
            probes.push(f32::from_bits(1)); // +min subnormal
            probes.push(f32::from_bits(0x8000_0001)); // -min subnormal
        }
        for h in probes {
            for s in [1.0f32, 0.5, 0.001, 2.5, -1.0, f32::INFINITY, f32::NAN] {
                for l in [0.0f32, 0.5, 1.0, -0.75, 1.75, f32::NAN, f32::INFINITY] {
                    libs.assert_same(
                        [h, s, l],
                        &format!("E6 b={b} h=0x{:08x} s={s} l={l}", h.to_bits()),
                    );
                }
            }
        }
    }
}

/// E7: negative hues take the doubled-`h<120` quirk arm (`m, c+m, x+m`),
/// not the final `else`.
#[test]
fn err_h_negative_takes_quirk_branch() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xE7);
    // Structural check on the C: for a negative hue the first output equals m
    // and the second equals c+m, i.e. it is NOT the all-grey else arm.
    let (c, _) = libs.call_both([-30.0, 1.0, 0.5]);
    assert_ne!(c[0].to_bits(), c[1].to_bits(), "E7 C took the grey else arm");
    for i in 0..10_000 {
        let h = r.range(-1.0e6, -1.0e-6);
        let s = r.range(-2.0, 2.0);
        let l = r.range(-2.0, 3.0);
        libs.assert_same([h, s, l], &format!("E7 i={i}"));
    }
    for h in [-1e-45f32, -0.0, -f32::MIN_POSITIVE, -360.0, -720.0, f32::MIN] {
        libs.assert_same([h, 0.9, 0.4], &format!("E7 fixed h={h}"));
    }
}

/// E8: `h >= 360` and `h = +INF` fall to the final `else` → `m, m, m`.
#[test]
fn err_h_ge_360_and_inf() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xE8);
    for h in [360.0f32, f32::from_bits(360.0f32.to_bits() + 1), 1e30, f32::MAX, f32::INFINITY] {
        for i in 0..1_000 {
            let s = r.range(0.0001, 2.0);
            let l = r.range(-2.0, 3.0);
            let (c, rs) = libs.call_both([h, s, l]);
            assert_eq!(common::bits(&c), common::bits(&rs), "E8 h={h} i={i}");
            assert_eq!(c[0].to_bits(), c[1].to_bits(), "E8 C not grey h={h}");
            assert_eq!(c[1].to_bits(), c[2].to_bits(), "E8 C not grey h={h}");
        }
    }
}

/// E9: `h = -INF` takes the quirk arm.
#[test]
fn err_h_neg_inf() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xE9);
    for i in 0..3_000 {
        let s = r.range(-2.0, 2.0);
        let l = r.range(-2.0, 3.0);
        libs.assert_same([f32::NEG_INFINITY, s, l], &format!("E9 i={i}"));
    }
    for s in [1.0f32, -1.0, f32::INFINITY, f32::NAN, f32::from_bits(1)] {
        for l in [0.0f32, 0.5, 1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            libs.assert_same([f32::NEG_INFINITY, s, l], "E9 fixed");
        }
    }
}

/// E10: `l = ±INF` / `l` NaN — unchecked, propagates.
#[test]
fn err_l_inf_and_nan() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xE10);
    let mut ls = vec![f32::INFINITY, f32::NEG_INFINITY];
    for _ in 0..40 {
        ls.push(r.any_nan());
    }
    for &l in &ls {
        for i in 0..200 {
            let h = r.range(-500.0, 900.0);
            let s = r.range(-2.0, 2.0);
            libs.assert_same([h, s, l], &format!("E10 l=0x{:08x} i={i}", l.to_bits()));
        }
        // s == 0 still short-circuits and copies l verbatim, NaN payload included.
        let (c, rs) = libs.call_both([37.0, 0.0, l]);
        assert_eq!(common::bits(&c), common::bits(&rs), "E10 short-circuit");
        assert_eq!(common::bits(&c), [l.to_bits(); 3], "E10 l not copied");
    }
}

/// E11: `s = ±INF` — including `l = 0.5` where `c = 0 * INF` = NaN.
#[test]
fn err_s_inf() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xE11);
    for s in [f32::INFINITY, f32::NEG_INFINITY] {
        // l == 0.5 → 1 - |2*0.5-1| == 0 → 0 * INF → NaN in the C.
        let (c, rs) = libs.call_both([30.0, s, 0.5]);
        assert_eq!(common::bits(&c), common::bits(&rs), "E11 l=0.5 s={s}");
        assert!(c[0].is_nan(), "E11 expected NaN from 0*INF, got {:?}", c[0]);
        for i in 0..3_000 {
            let h = r.range(-500.0, 900.0);
            let l = r.range(-2.0, 3.0);
            libs.assert_same([h, s, l], &format!("E11 s={s} i={i}"));
        }
    }
}

/// E12: all three inputs NaN with distinct payloads/signs — the surviving NaN's
/// sign and payload must match bit-for-bit.
#[test]
fn err_nan_payload_propagation() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xE12);
    let fixed = [
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFF80_0001),
        f32::from_bits(0x7FAA_AAAA),
        f32::from_bits(0xFF55_5555),
        f32::from_bits(0x7FFF_FFFF),
        f32::from_bits(0xFFFF_FFFF),
    ];
    for &a in &fixed {
        for &b in &fixed {
            for &c in &fixed {
                libs.assert_same(
                    [a, b, c],
                    &format!("E12 fixed 0x{:08x}/0x{:08x}/0x{:08x}", a.to_bits(), b.to_bits(), c.to_bits()),
                );
            }
        }
    }
    for i in 0..50_000 {
        let src = [r.any_nan(), r.any_nan(), r.any_nan()];
        libs.assert_same(src, &format!("E12 rnd i={i}"));
    }
    // Mixed: exactly one component NaN, in each position.
    for pos in 0..3 {
        for i in 0..5_000 {
            let mut src = [r.range(-400.0, 400.0), r.range(0.0001, 2.0), r.range(-2.0, 3.0)];
            src[pos] = r.any_nan();
            libs.assert_same(src, &format!("E12 pos={pos} i={i}"));
        }
    }
}

/// E13: `dest == src` — all loads precede all stores, so aliasing is benign.
#[test]
fn err_dest_aliases_src() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xE13);
    for i in 0..10_000 {
        let src = [r.any_f32(), r.any_f32(), r.any_f32()];
        let mut cb = src;
        let mut rb = src;
        unsafe {
            (libs.c)(cb.as_mut_ptr(), cb.as_ptr());
            (libs.rust)(rb.as_mut_ptr(), rb.as_ptr());
        }
        assert_eq!(cb.map(f32::to_bits), rb.map(f32::to_bits), "E13 i={i}");
        // Aliased result must equal the disjoint result.
        let (cd, _) = libs.call_both(src);
        assert_eq!(cb.map(f32::to_bits), cd.map(f32::to_bits), "E13 aliasing changed C");
    }
}

/// E14: partial overlap in both directions.
#[test]
fn err_partial_overlap() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xE14);
    for i in 0..10_000 {
        let base: [f32; 5] = [
            r.any_f32(),
            r.any_f32(),
            r.any_f32(),
            r.any_f32(),
            r.any_f32(),
        ];
        for (doff, soff) in [(1usize, 0usize), (0, 1), (2, 1), (1, 2), (0, 2), (2, 0)] {
            let mut cb = base;
            let mut rb = base;
            unsafe {
                (libs.c)(cb.as_mut_ptr().add(doff), cb.as_ptr().add(soff));
                (libs.rust)(rb.as_mut_ptr().add(doff), rb.as_ptr().add(soff));
            }
            assert_eq!(
                cb.map(f32::to_bits),
                rb.map(f32::to_bits),
                "E14 i={i} doff={doff} soff={soff}"
            );
        }
    }
}

/// E15: there is no `enum`, flag or `int` parameter in this API, so there is no
/// out-of-range enum value to pass across the FFI boundary. What the API *does*
/// accept is an arbitrary 32-bit word per component; that entire space is the
/// "invalid value" surface and it is swept in `row27_full_bitpattern_fuzz`.
/// This test pins the claim so the row is not silently forgotten.
#[test]
fn err_no_enum_surface_exhaustive_exponent_sweep() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 0xE15);
    // Sweep every one of the 256 exponent fields, both signs, for each argument
    // position — this covers zero, all subnormals, all normals, Inf and NaN
    // classes for values that no sane caller would pass.
    for pos in 0..3usize {
        for exp in 0u32..256 {
            for sign in [0u32, 0x8000_0000] {
                let mant = r.next_u32() & 0x007F_FFFF;
                let v = f32::from_bits(sign | (exp << 23) | mant);
                let mut src = [30.0f32, 0.75, 0.4];
                src[pos] = v;
                libs.assert_same(src, &format!("E15 pos={pos} exp={exp} sign={sign:#x}"));
            }
        }
    }
}

/// E16 / E17 are undefined behaviour in the C (unconditional dereference with no
/// length parameter), so they cannot be differentially tested. This test only
/// records the contract: the C header takes bare `float *` with no size and no
/// null check, so the Rust must impose the same `unsafe` contract and is not
/// expected to be more defensive.
#[test]
fn err_null_and_short_buffers_are_ub_not_tested() {
    // Documented in ERRORS.md rows E16/E17; intentionally not invoked.
    let libs = Libs::load();
    // Sanity: the function pointers really were resolved from the two .so files.
    assert!(!(libs.c as usize == 0));
    assert!(!(libs.rust as usize == 0));
    assert_ne!(
        libs.c as usize, libs.rust as usize,
        "C and Rust symbols must be distinct implementations"
    );
}
