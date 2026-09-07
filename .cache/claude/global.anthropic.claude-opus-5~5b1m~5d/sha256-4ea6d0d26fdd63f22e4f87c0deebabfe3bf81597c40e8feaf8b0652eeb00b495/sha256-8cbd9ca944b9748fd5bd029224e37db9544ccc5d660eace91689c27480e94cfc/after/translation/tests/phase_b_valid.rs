//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//! Every test loads BOTH `.so` files and compares the three output floats
//! bit-for-bit.

mod common;

use common::{Libs, Rng, HUE_REGIONS, L_SHAPES, SEED};

const N: usize = 3_000;

/// Rows 1–3: H1 `h ∈ [0,60)` crossed with L1 / L2 / L3.
#[test]
fn row01_03_h1_by_l_shape() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED);
    for i in 0..N {
        let h = r.range(0.0, 60.0);
        let s = r.range(0.0001, 1.0);
        // L1, L2 (exactly 0.5), L3
        for (tag, l) in [
            ("L1", r.range(0.0, 0.5)),
            ("L2", 0.5f32),
            ("L3", r.range(0.5, 1.0)),
        ] {
            libs.assert_same([h, s, l], &format!("row01_03 {tag} i={i}"));
        }
    }
}

/// Row 4: H2 `h ∈ [60,120)`.
#[test]
fn row04_h2() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 4);
    for i in 0..N {
        let h = r.range(60.0, 120.0);
        let s = r.range(0.0001, 1.0);
        for l in [r.range(0.0, 0.5), 0.5, r.range(0.5, 1.0)] {
            libs.assert_same([h, s, l], &format!("row04 i={i}"));
        }
    }
}

/// Row 5: H3 — negative hues, which the doubled `h < 120.0f` test routes into
/// the third arm. Also exercises X3 (`fmodf` with a negative dividend).
#[test]
fn row05_h3_negative_quirk_arm() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 5);
    for i in 0..N {
        let h = r.range(-1080.0, -0.0);
        let s = r.range(0.0001, 1.0);
        for l in [r.range(0.0, 0.5), 0.5, r.range(0.5, 1.0)] {
            libs.assert_same([h, s, l], &format!("row05 i={i}"));
        }
    }
    // -0.0 is >= 0.0, so it must take H1, not the quirk arm.
    libs.assert_same([-0.0, 0.5, 0.25], "row05 h=-0.0");
    libs.assert_same([-1e-45, 0.5, 0.25], "row05 h=-min_subnormal");
}

/// Row 6: H4 `h ∈ [120,180)` — the arm the C makes unreachable; result must be
/// the grey `m, m, m` of the final `else`.
#[test]
fn row06_h4_unreachable_arm() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 6);
    for i in 0..N {
        let h = r.range(120.0, 180.0);
        let s = r.range(0.0001, 1.0);
        for l in [r.range(0.0, 0.5), 0.5, r.range(0.5, 1.0)] {
            libs.assert_same([h, s, l], &format!("row06 i={i}"));
        }
    }
}

/// Rows 7–9: H5, H6, H7 (each also exercises the X2 `fmodf` wrap).
#[test]
fn row07_09_h5_h6_h7() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 7);
    for (lo, hi) in [(180.0f32, 240.0f32), (240.0, 300.0), (300.0, 360.0)] {
        for i in 0..N {
            let h = r.range(lo, hi);
            let s = r.range(0.0001, 1.0);
            for l in [r.range(0.0, 0.5), 0.5, r.range(0.5, 1.0)] {
                libs.assert_same([h, s, l], &format!("row07_09 [{lo},{hi}) i={i}"));
            }
        }
    }
}

/// Row 10: H8 `h >= 360`.
#[test]
fn row10_h8_ge_360() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 10);
    for i in 0..N {
        let h = r.range(360.0, 1.0e6);
        let s = r.range(0.0001, 1.0);
        for l in [r.range(0.0, 0.5), 0.5, r.range(0.5, 1.0)] {
            libs.assert_same([h, s, l], &format!("row10 i={i}"));
        }
    }
    for h in [360.0f32, 360.00003, 1.0e30, f32::MAX] {
        libs.assert_same([h, 0.7, 0.4], "row10 fixed");
    }
}

/// Rows 11–12: `s == +0.0` and `s == -0.0` short-circuit; the hue is ignored,
/// including NaN/Inf hues.
#[test]
fn row11_12_s_zero_short_circuit() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 11);
    let exotic_h = [
        0.0f32,
        -0.0,
        59.9,
        120.5,
        359.9,
        360.0,
        -12.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
    ];
    for s in [0.0f32, -0.0f32] {
        for &h in &exotic_h {
            for i in 0..200 {
                let l = r.any_f32(); // any bit pattern: l is copied verbatim
                libs.assert_same([h, s, l], &format!("row11_12 s={s} h={h} i={i}"));
            }
            for l in [0.0f32, 1.0, 0.5, -3.5, f32::INFINITY, f32::NAN, -0.0] {
                libs.assert_same([h, s, l], "row11_12 fixed l");
            }
        }
    }
}

/// Row 13: `s > 1`.
#[test]
fn row13_s_gt_1() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 13);
    for i in 0..N {
        let h = r.range(-500.0, 900.0);
        let s = r.range(1.0, 1.0e6);
        let l = r.range(-2.0, 3.0);
        libs.assert_same([h, s, l], &format!("row13 i={i}"));
    }
    for s in [1.0000001f32, 1e30, f32::MAX] {
        libs.assert_same([30.0, s, 0.5], "row13 fixed");
    }
}

/// Row 14: `s < 0`.
#[test]
fn row14_s_lt_0() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 14);
    for i in 0..N {
        let h = r.range(-500.0, 900.0);
        let s = r.range(-1.0e6, -1.0e-7);
        let l = r.range(-2.0, 3.0);
        libs.assert_same([h, s, l], &format!("row14 i={i}"));
    }
    libs.assert_same([30.0, -1e-45, 0.5], "row14 -min_subnormal");
}

/// Row 15: `s` subnormal but non-zero — must NOT short-circuit.
#[test]
fn row15_s_subnormal() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 15);
    let subs = [
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x0000_FFFF),
        f32::from_bits(0x007F_FFFF),
        f32::from_bits(0x807F_FFFF),
    ];
    for &s in &subs {
        for (tag, lo, hi) in HUE_REGIONS.iter().map(|&(t, a, b)| (t, a, b)) {
            for i in 0..80 {
                let h = r.range(lo, hi);
                let l = r.range(-1.0, 2.0);
                libs.assert_same([h, s, l], &format!("row15 {tag} s={s:e} i={i}"));
            }
        }
    }
}

/// Row 16: `s = ±INF` (incl. `l = 0.5` where `c = 0 * INF` → NaN).
#[test]
fn row16_s_inf() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 16);
    for s in [f32::INFINITY, f32::NEG_INFINITY] {
        for &(tag, lo, hi) in HUE_REGIONS {
            for i in 0..80 {
                let h = r.range(lo, hi);
                let l = r.range(-2.0, 3.0);
                libs.assert_same([h, s, l], &format!("row16 {tag} s={s} i={i}"));
            }
            for l in [0.0f32, 0.5, 1.0, -1.0, f32::INFINITY, f32::NAN] {
                libs.assert_same([r.range(lo, hi), s, l], &format!("row16 {tag} l={l}"));
            }
        }
    }
}

/// Row 17: `s` NaN — `NaN == 0` is false, so the formula runs.
#[test]
fn row17_s_nan() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 17);
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7F80_0001), // signalling
        f32::from_bits(0xFF80_0001), // signalling, negative
        f32::from_bits(0x7FAB_CDEF),
    ];
    for &s in &nans {
        for &(tag, lo, hi) in HUE_REGIONS {
            for i in 0..80 {
                let h = r.range(lo, hi);
                let l = r.range(-2.0, 3.0);
                libs.assert_same([h, s, l], &format!("row17 {tag} s=0x{:08x} i={i}", s.to_bits()));
            }
        }
    }
}

/// Row 18: `l ∈ {0.0, 1.0}` → `c == 0`, `x == 0`, `m == l`.
#[test]
fn row18_l_edges() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 18);
    for l in [0.0f32, 1.0f32, -0.0f32] {
        for &(tag, lo, hi) in HUE_REGIONS {
            for i in 0..150 {
                let h = r.range(lo, hi);
                let s = r.range(0.0001, 1.0);
                libs.assert_same([h, s, l], &format!("row18 {tag} l={l} i={i}"));
            }
        }
    }
}

/// Row 19: `l` outside `[0,1]`.
#[test]
fn row19_l_out_of_range() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 19);
    for &(ltag, llo, lhi) in L_SHAPES {
        for &(htag, hlo, hhi) in HUE_REGIONS {
            for i in 0..80 {
                let h = r.range(hlo, hhi);
                let s = r.range(0.0001, 1.0);
                let l = r.range(llo, lhi);
                libs.assert_same([h, s, l], &format!("row19 {ltag} {htag} i={i}"));
            }
        }
    }
    for l in [-1e30f32, 1e30, f32::MIN, f32::MAX] {
        libs.assert_same([200.0, 0.8, l], "row19 huge l");
    }
}

/// Row 20: `l = ±INF`.
#[test]
fn row20_l_inf() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 20);
    for l in [f32::INFINITY, f32::NEG_INFINITY] {
        for &(tag, lo, hi) in HUE_REGIONS {
            for i in 0..80 {
                let h = r.range(lo, hi);
                let s = r.range(0.0001, 1.0);
                libs.assert_same([h, s, l], &format!("row20 {tag} l={l} i={i}"));
            }
        }
    }
}

/// Row 21: `l` NaN, several payloads and both signs.
#[test]
fn row21_l_nan() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 21);
    for _ in 0..40 {
        let l = r.any_nan();
        for &(tag, lo, hi) in HUE_REGIONS {
            for i in 0..20 {
                let h = r.range(lo, hi);
                let s = r.range(0.0001, 1.0);
                libs.assert_same([h, s, l], &format!("row21 {tag} l=0x{:08x} i={i}", l.to_bits()));
            }
        }
    }
}

/// Row 22: X4 — `h` an exact multiple of 120 makes `fmodf(h/60, 2)` return `±0`.
#[test]
fn row22_h_multiples_of_120() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 22);
    let mut hs: Vec<f32> = Vec::new();
    let mut k = -12i32;
    while k <= 12 {
        hs.push(120.0 * k as f32);
        hs.push(60.0 * k as f32);
        k += 1;
    }
    for &h in &hs {
        for i in 0..80 {
            let s = r.range(0.0001, 1.0);
            for l in [r.range(0.0, 0.5), 0.5, r.range(0.5, 1.0), 0.0, 1.0] {
                libs.assert_same([h, s, l], &format!("row22 h={h} i={i}"));
            }
        }
    }
}

/// Row 23: the `fmodf` sawtooth swept finely from -720 to +1080.
#[test]
fn row23_sawtooth_sweep() {
    let libs = Libs::load();
    let mut h = -720.0f32;
    let step = 0.37f32;
    let mut n = 0usize;
    while h <= 1080.0 {
        libs.assert_same([h, 1.0, 0.5], &format!("row23 h={h}"));
        libs.assert_same([h, 0.3333333, 0.25], &format!("row23b h={h}"));
        libs.assert_same([h, 0.75, 0.875], &format!("row23c h={h}"));
        h += step;
        n += 1;
    }
    assert!(n > 4_000, "sweep too coarse: {n} points");
}

/// Row 24: X5 — `h = ±INF` makes `fmodf(±INF, 2)` NaN while `c`/`m` stay finite.
#[test]
fn row24_h_inf() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 24);
    for h in [f32::INFINITY, f32::NEG_INFINITY] {
        for i in 0..600 {
            let s = r.range(0.0001, 1.0);
            let l = r.range(-2.0, 3.0);
            libs.assert_same([h, s, l], &format!("row24 h={h} i={i}"));
        }
        for l in [0.0f32, 0.5, 1.0, f32::NAN, f32::INFINITY] {
            libs.assert_same([h, 0.6, l], &format!("row24 fixed h={h} l={l}"));
        }
    }
}

/// Rows 25 / H9: `h` NaN → all comparisons false → final `else`.
#[test]
fn row25_h_nan() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 25);
    for _ in 0..80 {
        let h = r.any_nan();
        for i in 0..20 {
            let s = r.range(0.0001, 1.0);
            let l = r.range(-2.0, 3.0);
            libs.assert_same([h, s, l], &format!("row25 h=0x{:08x} i={i}", h.to_bits()));
        }
    }
}

/// Row 26: `h` exactly on, and one ULP either side of, every dispatch boundary.
#[test]
fn row26_hue_boundaries_ulp() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 26);
    let bounds = [0.0f32, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0];
    let mut probes: Vec<f32> = Vec::new();
    for &b in &bounds {
        probes.push(b);
        probes.push(-b);
        // one ULP either side
        for delta in [-3i32, -2, -1, 1, 2, 3] {
            let bits = b.to_bits();
            let p = if delta < 0 {
                if b == 0.0 {
                    f32::from_bits(0x8000_0000 | (-delta) as u32) // tiny negatives
                } else {
                    f32::from_bits(bits - (-delta) as u32)
                }
            } else {
                f32::from_bits(bits + delta as u32)
            };
            probes.push(p);
            probes.push(-p);
        }
    }
    for &h in &probes {
        for l in [0.0f32, 0.25, 0.5, 0.75, 1.0, -0.3, 1.7] {
            for s in [1.0f32, 0.5, 1e-7, 2.0, -0.5] {
                libs.assert_same([h, s, l], &format!("row26 h=0x{:08x}", h.to_bits()));
            }
        }
        for i in 0..20 {
            libs.assert_same(
                [h, r.range(-1.0, 2.0), r.range(-1.0, 2.0)],
                &format!("row26 rnd h=0x{:08x} i={i}", h.to_bits()),
            );
        }
    }
}

/// Row 27: unconstrained 32-bit-pattern fuzz over all three components.
#[test]
fn row27_full_bitpattern_fuzz() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 27);
    for i in 0..200_000 {
        let src = [r.any_f32(), r.any_f32(), r.any_f32()];
        libs.assert_same(src, &format!("row27 i={i}"));
    }
}

/// Row 28: NaN-heavy fuzz — checks NaN sign/payload propagation.
#[test]
fn row28_nan_heavy_fuzz() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 28);
    for i in 0..100_000 {
        let mut src = [0.0f32; 3];
        for c in src.iter_mut() {
            *c = match r.next_u32() % 4 {
                0 | 1 => r.any_nan(),
                2 => r.pick(&[
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    0.0,
                    -0.0,
                    1.0,
                    -1.0,
                    0.5,
                    f32::from_bits(1),
                ]),
                _ => r.range(-400.0, 400.0),
            };
        }
        libs.assert_same(src, &format!("row28 i={i}"));
    }
}

/// Row 29: `dest == src` — full aliasing.
#[test]
fn row29_dest_aliases_src() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 29);
    for i in 0..20_000 {
        let src = [r.any_f32(), r.any_f32(), r.any_f32()];
        let mut cbuf = src;
        let mut rbuf = src;
        unsafe {
            (libs.c)(cbuf.as_mut_ptr(), cbuf.as_ptr());
            (libs.rust)(rbuf.as_mut_ptr(), rbuf.as_ptr());
        }
        assert_eq!(
            common::bits(&cbuf),
            common::bits(&rbuf),
            "row29 i={i} src={} C={} Rust={}",
            common::show(&src),
            common::show(&cbuf),
            common::show(&rbuf)
        );
    }
}

/// Row 30: partially overlapping buffers (`dest == src+1` and `dest == src-1`).
#[test]
fn row30_partial_overlap() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 30);
    for i in 0..20_000 {
        let seed_buf: [f32; 4] = [r.any_f32(), r.any_f32(), r.any_f32(), r.any_f32()];
        // dest = buf+1, src = buf+0
        let mut cb = seed_buf;
        let mut rb = seed_buf;
        unsafe {
            (libs.c)(cb.as_mut_ptr().add(1), cb.as_ptr());
            (libs.rust)(rb.as_mut_ptr().add(1), rb.as_ptr());
        }
        assert_eq!(cb.map(f32::to_bits), rb.map(f32::to_bits), "row30 fwd i={i}");
        // dest = buf+0, src = buf+1
        let mut cb = seed_buf;
        let mut rb = seed_buf;
        unsafe {
            (libs.c)(cb.as_mut_ptr(), cb.as_ptr().add(1));
            (libs.rust)(rb.as_mut_ptr(), rb.as_ptr().add(1));
        }
        assert_eq!(cb.map(f32::to_bits), rb.map(f32::to_bits), "row30 bwd i={i}");
    }
}

/// Row 31: buffers at 4-byte-aligned offsets inside a larger allocation.
#[test]
fn row31_offset_buffers() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 31);
    for i in 0..10_000 {
        let mut big = vec![0.0f32; 16];
        for v in big.iter_mut() {
            *v = r.any_f32();
        }
        let soff = (r.next_u32() % 6) as usize;
        let doff = 8 + (r.next_u32() % 5) as usize;
        let mut cb = big.clone();
        let mut rb = big.clone();
        unsafe {
            (libs.c)(cb.as_mut_ptr().add(doff), cb.as_ptr().add(soff));
            (libs.rust)(rb.as_mut_ptr().add(doff), rb.as_ptr().add(soff));
        }
        let cbits: Vec<u32> = cb.iter().map(|v| v.to_bits()).collect();
        let rbits: Vec<u32> = rb.iter().map(|v| v.to_bits()).collect();
        assert_eq!(cbits, rbits, "row31 i={i} soff={soff} doff={doff}");
    }
}

/// Row 32: no hidden state — repeated and interleaved calls are stable.
#[test]
fn row32_statelessness() {
    let libs = Libs::load();
    let mut r = Rng::new(SEED ^ 32);
    let cases: Vec<[f32; 3]> = (0..500)
        .map(|_| [r.any_f32(), r.any_f32(), r.any_f32()])
        .collect();
    // First pass: record.
    let firsts: Vec<([f32; 3], [f32; 3])> = cases.iter().map(|&s| libs.call_both(s)).collect();
    // Interleave everything, then replay in reverse and compare.
    for &s in cases.iter().rev() {
        libs.assert_same(s, "row32 interleave");
    }
    for (idx, &s) in cases.iter().enumerate().rev() {
        let (c2, r2) = libs.call_both(s);
        assert_eq!(common::bits(&c2), common::bits(&firsts[idx].0), "row32 C drift");
        assert_eq!(common::bits(&r2), common::bits(&firsts[idx].1), "row32 Rust drift");
        assert_eq!(common::bits(&c2), common::bits(&r2), "row32 mismatch");
    }
}
