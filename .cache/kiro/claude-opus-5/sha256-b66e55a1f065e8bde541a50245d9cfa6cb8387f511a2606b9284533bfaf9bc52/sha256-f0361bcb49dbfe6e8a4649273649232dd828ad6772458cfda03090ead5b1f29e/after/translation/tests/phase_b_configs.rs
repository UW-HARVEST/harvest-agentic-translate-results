//! Phase B — valid-path differential tests, gated on `CONFIGS.md`.
//!
//! One `#[test]` per row of `CONFIGS.md`. Every row drives BOTH shared objects
//! through their exported `hsl_to_rgb` symbol with many randomized inputs
//! (fixed seed) and asserts the three output `f32`s are bit-identical.

mod common;

use common::*;

/// Inputs per randomized row. Kept high enough to explore value-dependent
/// paths, low enough that the whole suite stays well under the time budget.
const N: usize = 20_000;

// ---------------------------------------------------------------------------
// Rows 1-2: the `s == 0` early-return path.
// ---------------------------------------------------------------------------

#[test]
fn row01_s_positive_zero_early_return() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        // `l` over all finite f32, `h` over anything at all.
        let l = rng.any_f32();
        let h = rng.any_f32();
        assert_same(&p, "row01 s=+0.0", [h, 0.0, l]);
    }
    for &l in SPECIALS.iter() {
        for &h in SPECIALS.iter() {
            assert_same(&p, "row01 s=+0.0 specials", [h, 0.0, l]);
        }
    }
}

#[test]
fn row02_s_negative_zero_early_return() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let l = rng.any_f32();
        let h = rng.any_f32();
        assert_same(&p, "row02 s=-0.0", [h, -0.0, l]);
    }
    // `l` = NaN / inf / subnormal must pass through the early return verbatim.
    for &l in SPECIALS.iter() {
        assert_same(&p, "row02 s=-0.0 specials", [123.0, -0.0, l]);
    }
    for neg in [false, true] {
        for _ in 0..1000 {
            let l = rng.nan(neg);
            assert_same(&p, "row02 s=-0.0 l=NaN", [rng.any_f32(), -0.0, l]);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 3-10: each hue region crossed with the nominal s/l ranges.
// ---------------------------------------------------------------------------

fn nominal_region(p: &Pair, label: &str, region: usize) {
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let h = hue_in_region(&mut rng, region);
        // s in (0,1] — reject exactly 0 so the full path runs.
        let mut s = rng.range(0.0, 1.0);
        if s == 0.0 {
            s = 1.0;
        }
        let l = rng.range(0.0, 1.0);
        assert_same(p, label, [h, s, l]);
    }
}

#[test]
fn row03_hue_0_60() {
    nominal_region(&Pair::load(), "row03 h in [0,60)", 1);
}

#[test]
fn row04_hue_60_120() {
    nominal_region(&Pair::load(), "row04 h in [60,120)", 2);
}

#[test]
fn row05_hue_120_180_dead_range() {
    nominal_region(&Pair::load(), "row05 h in [120,180) dead range", 3);
}

#[test]
fn row06_hue_180_240() {
    nominal_region(&Pair::load(), "row06 h in [180,240)", 4);
}

#[test]
fn row07_hue_240_300() {
    nominal_region(&Pair::load(), "row07 h in [240,300)", 5);
}

#[test]
fn row08_hue_300_360() {
    nominal_region(&Pair::load(), "row08 h in [300,360)", 6);
}

#[test]
fn row09_hue_ge_360() {
    nominal_region(&Pair::load(), "row09 h >= 360", 7);
}

#[test]
fn row10_hue_negative_buggy_arm3() {
    nominal_region(&Pair::load(), "row10 h < 0 (buggy arm 3)", 0);
}

// ---------------------------------------------------------------------------
// Row 11: exact sector boundaries and their immediate f32 neighbours.
// ---------------------------------------------------------------------------

#[test]
fn row11_sector_boundaries_and_neighbours() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let mut hues: Vec<f32> = Vec::new();
    for &e in SECTOR_EDGES.iter() {
        hues.push(e);
        hues.push(next_up(e));
        hues.push(next_down(e));
        hues.push(next_up(next_up(e)));
        hues.push(next_down(next_down(e)));
    }
    // A few more edges past the nominal range.
    for e in [420.0f32, 480.0, -60.0, -120.0, 720.0] {
        hues.push(e);
        hues.push(next_up(e));
        hues.push(next_down(e));
    }
    for &h in &hues {
        for _ in 0..200 {
            let mut s = rng.range(0.0, 1.0);
            if s == 0.0 {
                s = 1.0;
            }
            let l = rng.range(0.0, 1.0);
            assert_same(&p, "row11 sector boundary", [h, s, l]);
        }
        // and with the extreme s/l catalogue
        for &s in SPECIALS.iter() {
            for &l in SPECIALS.iter() {
                assert_same(&p, "row11 boundary x specials", [h, s, l]);
            }
        }
    }
}

/// `f32::next_up` is not stable on the pinned toolchain; do it by bits.
fn next_up(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(1);
    }
    let b = x.to_bits();
    if b >> 31 == 0 {
        f32::from_bits(b + 1)
    } else {
        f32::from_bits(b - 1)
    }
}

fn next_down(x: f32) -> f32 {
    -next_up(-x)
}

// ---------------------------------------------------------------------------
// Rows 12-15: lightness / saturation special shapes across all hue regions.
// ---------------------------------------------------------------------------

fn across_regions<F: FnMut(&mut Rng) -> (f32, f32)>(p: &Pair, label: &str, mut pick_sl: F) {
    let mut rng = Rng::seeded();
    for region in 0..HUE_REGIONS.len() {
        for _ in 0..(N / 4) {
            let h = hue_in_region(&mut rng, region);
            let (s, l) = pick_sl(&mut rng);
            assert_same(p, label, [h, s, l]);
        }
        // plus the exact sector edges of this region
        for &e in SECTOR_EDGES.iter() {
            let (s, l) = pick_sl(&mut rng);
            assert_same(p, label, [e, s, l]);
        }
    }
}

#[test]
fn row12_lightness_exactly_half() {
    let p = Pair::load();
    across_regions(&p, "row12 l == 0.5", |rng| {
        let mut s = rng.range(-2.0, 2.0);
        if s == 0.0 {
            s = 1.0;
        }
        (s, 0.5)
    });
}

#[test]
fn row13_lightness_zero_and_one() {
    let p = Pair::load();
    across_regions(&p, "row13 l in {0.0,-0.0,1.0}", |rng| {
        let mut s = rng.range(-2.0, 2.0);
        if s == 0.0 {
            s = 1.0;
        }
        let l = rng.pick(&[0.0f32, -0.0, 1.0]);
        (s, l)
    });
}

#[test]
fn row14_lightness_out_of_range() {
    let p = Pair::load();
    across_regions(&p, "row14 l outside [0,1]", |rng| {
        let mut s = rng.range(0.0, 1.0);
        if s == 0.0 {
            s = 1.0;
        }
        let l = if rng.bool() {
            rng.range(-1.0e6, -1.0e-6)
        } else {
            rng.range(1.0 + 1.0e-6, 1.0e6)
        };
        (s, l)
    });
}

#[test]
fn row15_saturation_out_of_range() {
    let p = Pair::load();
    across_regions(&p, "row15 s outside [0,1]", |rng| {
        let s = if rng.bool() {
            rng.range(1.0 + 1.0e-6, 1.0e6)
        } else {
            rng.range(-1.0e6, -1.0e-6)
        };
        (s, rng.range(0.0, 1.0))
    });
}

// ---------------------------------------------------------------------------
// Rows 16-19: the `fmodf` regimes.
// ---------------------------------------------------------------------------

#[test]
fn row16_hue_even_multiple_of_60() {
    // h/60 is an even integer -> fmodf(h/60, 2) == +0.0 -> x == c
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let hues: Vec<f32> = (-40..=40).map(|k| (k * 120) as f32).collect();
    for &h in &hues {
        for _ in 0..500 {
            let mut s = rng.range(-2.0, 2.0);
            if s == 0.0 {
                s = 1.0;
            }
            assert_same(&p, "row16 h = 120k", [h, s, rng.range(-2.0, 2.0)]);
        }
    }
}

#[test]
fn row17_hue_odd_multiple_of_60() {
    // h/60 is an odd integer -> fmodf(h/60, 2) == 1 -> x == 0
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let hues: Vec<f32> = (-40..=40).map(|k| (k * 120 + 60) as f32).collect();
    for &h in &hues {
        for _ in 0..500 {
            let mut s = rng.range(-2.0, 2.0);
            if s == 0.0 {
                s = 1.0;
            }
            assert_same(&p, "row17 h = 120k+60", [h, s, rng.range(-2.0, 2.0)]);
        }
    }
}

#[test]
fn row18_hue_infinite() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for &h in [f32::INFINITY, f32::NEG_INFINITY].iter() {
        for _ in 0..5_000 {
            let mut s = rng.any_f32();
            if s == 0.0 {
                s = 1.0;
            }
            assert_same(&p, "row18 h = +-inf", [h, s, rng.any_f32()]);
        }
        for &s in SPECIALS.iter() {
            for &l in SPECIALS.iter() {
                assert_same(&p, "row18 h = +-inf x specials", [h, s, l]);
            }
        }
    }
}

#[test]
fn row19_hue_nan_subnormal_and_zero() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for neg in [false, true] {
        for _ in 0..5_000 {
            let h = rng.nan(neg);
            let mut s = rng.range(-2.0, 2.0);
            if s == 0.0 {
                s = 1.0;
            }
            assert_same(&p, "row19 h = NaN", [h, s, rng.range(-2.0, 2.0)]);
        }
    }
    // Subnormal and signed-zero hues.
    let tiny = [
        0.0f32,
        -0.0,
        1.0e-45,
        -1.0e-45,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::MIN_POSITIVE / 2.0,
        -f32::MIN_POSITIVE / 2.0,
    ];
    for &h in tiny.iter() {
        for _ in 0..1_000 {
            let mut s = rng.range(-2.0, 2.0);
            if s == 0.0 {
                s = 1.0;
            }
            assert_same(&p, "row19 h subnormal/zero", [h, s, rng.range(-2.0, 2.0)]);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 20-22: infinite / subnormal saturation and lightness.
// ---------------------------------------------------------------------------

#[test]
fn row20_lightness_infinite() {
    let p = Pair::load();
    across_regions(&p, "row20 l = +-inf", |rng| {
        let mut s = rng.range(-2.0, 2.0);
        if s == 0.0 {
            s = 1.0;
        }
        (s, if rng.bool() { f32::INFINITY } else { f32::NEG_INFINITY })
    });
}

#[test]
fn row21_saturation_infinite() {
    let p = Pair::load();
    across_regions(&p, "row21 s = +-inf", |rng| {
        (
            if rng.bool() { f32::INFINITY } else { f32::NEG_INFINITY },
            rng.range(-2.0, 2.0),
        )
    });
}

#[test]
fn row22_subnormal_saturation_and_lightness() {
    let p = Pair::load();
    let subs = [
        1.0e-45f32,
        -1.0e-45,
        f32::MIN_POSITIVE / 2.0,
        -f32::MIN_POSITIVE / 2.0,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(0x0040_0000),
        f32::from_bits(0x8040_0000),
    ];
    across_regions(&p, "row22 subnormal s/l", move |rng| {
        let s = rng.pick(&subs);
        let l = if rng.bool() {
            rng.pick(&subs)
        } else {
            rng.range(0.0, 1.0)
        };
        (s, l)
    });
}

// ---------------------------------------------------------------------------
// Rows 23-26: NaN operand pairing — the axis that pins down every SSE
// destination-operand choice in the C.
// ---------------------------------------------------------------------------

#[test]
fn row23_lightness_nan_probes_add_operand_order() {
    // l = NaN with s = 1.0 makes `x` a POSITIVE NaN (via fabsf) while `m` keeps
    // l's sign, so `add(x, m)` and `add(m, x)` differ observably.
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for region in 0..HUE_REGIONS.len() {
        for neg in [false, true] {
            for _ in 0..2_000 {
                let h = hue_in_region(&mut rng, region);
                let l = rng.nan(neg);
                assert_same(&p, "row23 l=NaN s=1", [h, 1.0, l]);
                let s = rng.range(0.001, 2.0);
                assert_same(&p, "row23 l=NaN s=rand", [h, s, l]);
            }
        }
        // exact edges too
        for &h in SECTOR_EDGES.iter() {
            for neg in [false, true] {
                let l = rng.nan(neg);
                assert_same(&p, "row23 l=NaN at edge", [h, 1.0, l]);
            }
        }
    }
}

#[test]
fn row24_saturation_nan_probes_mul_operand_order() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for region in 0..HUE_REGIONS.len() {
        for neg in [false, true] {
            for _ in 0..2_000 {
                let h = hue_in_region(&mut rng, region);
                let s = rng.nan(neg);
                let l = rng.range(-2.0, 2.0);
                assert_same(&p, "row24 s=NaN", [h, s, l]);
            }
        }
    }
}

#[test]
fn row25_all_nan_combinations() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for hn in [false, true] {
        for sn in [false, true] {
            for ln in [false, true] {
                for _ in 0..3_000 {
                    let (nh, ns, nl) = (rng.bool(), rng.bool(), rng.bool());
                    let h = if hn { rng.nan(nh) } else { rng.range(-400.0, 400.0) };
                    let s = if sn { rng.nan(ns) } else { rng.range(0.001, 2.0) };
                    let l = if ln { rng.nan(nl) } else { rng.range(-2.0, 2.0) };
                    assert_same(&p, "row25 NaN combos", [h, s, l]);
                }
            }
        }
    }
}

#[test]
fn row26_signalling_nans() {
    let p = Pair::load();
    let snans = [
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xff80_0001),
        f32::from_bits(0x7fbf_ffff),
        f32::from_bits(0xffbf_ffff),
        f32::from_bits(0x7f80_4000),
        f32::from_bits(0xff80_4000),
    ];
    let mut rng = Rng::seeded();
    for &v in snans.iter() {
        for region in 0..HUE_REGIONS.len() {
            let h = hue_in_region(&mut rng, region);
            let s = rng.range(0.001, 2.0);
            let l = rng.range(-2.0, 2.0);
            assert_same(&p, "row26 sNaN in h", [v, s, l]);
            assert_same(&p, "row26 sNaN in s", [h, v, l]);
            assert_same(&p, "row26 sNaN in l", [h, s, v]);
            assert_same(&p, "row26 sNaN in s and l", [h, v, v]);
            assert_same(&p, "row26 sNaN everywhere", [v, v, v]);
        }
        for &e in SECTOR_EDGES.iter() {
            assert_same(&p, "row26 sNaN at edge", [e, v, v]);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 27: unrestricted 32-bit-pattern fuzz.
// ---------------------------------------------------------------------------

#[test]
fn row27_unrestricted_bit_pattern_fuzz() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..200_000 {
        let src = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        assert_same(&p, "row27 full fuzz", src);
    }
}

// ---------------------------------------------------------------------------
// Rows 28-31: buffer geometry.
// ---------------------------------------------------------------------------

/// Run both implementations with `dest` and `src` at the given offsets inside a
/// scratch buffer, and compare the whole buffer afterwards.
fn geometry_case(p: &Pair, label: &str, src_off: usize, dest_off: usize, src: [f32; 3]) {
    const LEN: usize = 12;
    let build = || {
        let mut buf = [f32::from_bits(0xDEAD_BEEF); LEN];
        buf[src_off] = src[0];
        buf[src_off + 1] = src[1];
        buf[src_off + 2] = src[2];
        buf
    };
    let mut cb = build();
    let mut rb = build();
    unsafe {
        (p.c)(cb.as_mut_ptr().add(dest_off), cb.as_ptr().add(src_off));
        (p.rust)(rb.as_mut_ptr().add(dest_off), rb.as_ptr().add(src_off));
    }
    let cbits: Vec<u32> = cb.iter().map(|v| v.to_bits()).collect();
    let rbits: Vec<u32> = rb.iter().map(|v| v.to_bits()).collect();
    assert_eq!(
        cbits, rbits,
        "\n[{label}] divergence (src_off={src_off}, dest_off={dest_off})\n  input: {}\n  C   : {cbits:08x?}\n  Rust: {rbits:08x?}\n",
        show_in(src)
    );
}

fn geometry_inputs(rng: &mut Rng) -> [f32; 3] {
    match rng.next_u32() % 4 {
        0 => [rng.range(-400.0, 400.0), rng.range(-2.0, 2.0), rng.range(-2.0, 2.0)],
        1 => [rng.any_f32(), rng.any_f32(), rng.any_f32()],
        2 => [rng.range(-400.0, 400.0), 0.0, rng.any_f32()],
        _ => {
            let neg = rng.bool();
            [rng.range(-400.0, 400.0), 1.0, rng.nan(neg)]
        }
    }
}

#[test]
fn row28_dest_aliases_src_exactly() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..20_000 {
        let src = geometry_inputs(&mut rng);
        geometry_case(&p, "row28 dest == src", 4, 4, src);
    }
}

#[test]
fn row29_dest_partially_overlaps_src() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..20_000 {
        let src = geometry_inputs(&mut rng);
        geometry_case(&p, "row29 dest == src+1", 4, 5, src);
        geometry_case(&p, "row29 dest == src-1", 4, 3, src);
        geometry_case(&p, "row29 dest == src+2", 4, 6, src);
        geometry_case(&p, "row29 dest == src-2", 4, 2, src);
    }
}

#[test]
fn row30_guard_sentinels_bound_the_access() {
    // Disjoint buffers with poison guards on both sides: proves neither
    // implementation writes outside dest[0..3], and — by running each input
    // twice with *different* bytes beyond src[2] — that neither reads past
    // src[2].
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..20_000 {
        let src = geometry_inputs(&mut rng);
        // dest at offset 5, src at offset 1: guards everywhere else.
        geometry_case(&p, "row30 guarded disjoint", 1, 5, src);

        // Independence from bytes past src[2].
        let mut a = [0f32; 6];
        let mut b = [0f32; 6];
        for (i, v) in src.iter().enumerate() {
            a[i] = *v;
            b[i] = *v;
        }
        a[3] = f32::from_bits(0x0000_0000);
        a[4] = f32::from_bits(0x0000_0000);
        a[5] = f32::from_bits(0x0000_0000);
        b[3] = f32::from_bits(0xFFFF_FFFF);
        b[4] = f32::from_bits(0x7F7F_7F7F);
        b[5] = f32::from_bits(0xA5A5_A5A5);
        let mut da = [f32::from_bits(0xDEAD_BEEF); 6];
        let mut db = [f32::from_bits(0xDEAD_BEEF); 6];
        let mut ra = [f32::from_bits(0xDEAD_BEEF); 6];
        let mut rb = [f32::from_bits(0xDEAD_BEEF); 6];
        unsafe {
            (p.c)(da.as_mut_ptr(), a.as_ptr());
            (p.c)(db.as_mut_ptr(), b.as_ptr());
            (p.rust)(ra.as_mut_ptr(), a.as_ptr());
            (p.rust)(rb.as_mut_ptr(), b.as_ptr());
        }
        let bits = |x: &[f32; 6]| x.iter().map(|v| v.to_bits()).collect::<Vec<u32>>();
        assert_eq!(bits(&da), bits(&db), "C read past src[2] for {}", show_in(src));
        assert_eq!(bits(&ra), bits(&rb), "Rust read past src[2] for {}", show_in(src));
        assert_eq!(bits(&da), bits(&ra), "divergence for {}", show_in(src));
        // Guards on dest must be untouched.
        for i in 3..6 {
            assert_eq!(da[i].to_bits(), 0xDEAD_BEEF, "C wrote dest[{i}]");
            assert_eq!(ra[i].to_bits(), 0xDEAD_BEEF, "Rust wrote dest[{i}]");
        }
    }
}

#[test]
fn row31_underaligned_buffers() {
    // 4-byte aligned but deliberately not 8/16-byte aligned.
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let mut backing = vec![0u8; 128];
    for _ in 0..20_000 {
        let src = geometry_inputs(&mut rng);
        for (soff, doff) in [(4usize, 36usize), (12, 44), (20, 52), (4, 4), (4, 8)] {
            assert_eq!(soff % 4, 0);
            assert_eq!(doff % 4, 0);
            unsafe {
                let base = backing.as_mut_ptr();
                // force a 4-mod-8 start so the f32 pointers are not 8-aligned
                let skew = (base as usize) % 8;
                let start = base.add((4 + 8 - skew) % 8);
                let sp = start.add(soff) as *mut f32;
                let dp = start.add(doff) as *mut f32;
                assert_eq!(sp as usize % 4, 0);
                std::ptr::write(sp, src[0]);
                std::ptr::write(sp.add(1), src[1]);
                std::ptr::write(sp.add(2), src[2]);
                let mut cout = [0u32; 3];
                let mut rout = [0u32; 3];
                for k in 0..3 {
                    std::ptr::write(dp.add(k), f32::from_bits(0xDEAD_BEEF));
                }
                (p.c)(dp, sp as *const f32);
                for k in 0..3 {
                    cout[k] = std::ptr::read(dp.add(k)).to_bits();
                }
                // restore src (it may have been clobbered by aliasing)
                std::ptr::write(sp, src[0]);
                std::ptr::write(sp.add(1), src[1]);
                std::ptr::write(sp.add(2), src[2]);
                for k in 0..3 {
                    std::ptr::write(dp.add(k), f32::from_bits(0xDEAD_BEEF));
                }
                (p.rust)(dp, sp as *const f32);
                for k in 0..3 {
                    rout[k] = std::ptr::read(dp.add(k)).to_bits();
                }
                assert_eq!(
                    cout, rout,
                    "\n[row31 underaligned soff={soff} doff={doff}] {}\n  C   : {}\n  Rust: {}\n",
                    show_in(src),
                    show3(cout),
                    show3(rout)
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 32: build configuration. Reported, not asserted here — the runner script
// re-runs this whole file for every feature combination.
// ---------------------------------------------------------------------------

#[test]
fn row32_report_loaded_objects() {
    let p = Pair::load();
    println!("C    .so: {}", p.c_path.display());
    println!("Rust .so: {}", p.rust_path.display());
    // sanity: both symbols resolved and are distinct code addresses
    let c_addr = p.c as usize;
    let r_addr = p.rust as usize;
    assert_ne!(c_addr, r_addr, "loaded the same object twice");
}
