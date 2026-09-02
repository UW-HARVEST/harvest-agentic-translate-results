//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH shared objects
//! through their exported `hsv_to_rgb` symbol with many randomized inputs
//! (fixed seed) and compares the three output floats bit-for-bit.

mod common;

use common::{Libs, Rng, SEED, SPECIAL_F32, SPECIAL_HUE, assert_bits_eq, bits3};

/// Randomized samples per row. Kept high enough to catch value-dependent bugs
/// while keeping the whole suite well under the time budget.
const N: usize = 20_000;

fn libs() -> Libs {
    Libs::load()
}

// ---------------------------------------------------------------------------
// C1–C3: the achromatic early-return guard (`s == 0`).
// ---------------------------------------------------------------------------

#[test]
fn c1_saturation_positive_zero() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..N {
        let v = rng.range(-1e6, 1e6);
        l.check("C1", [rng.range(-1e6, 1e6), 0.0, v]);
    }
    for &v in SPECIAL_F32 {
        for &h in SPECIAL_HUE {
            l.check("C1", [h, 0.0, v]);
        }
    }
}

#[test]
fn c2_saturation_negative_zero() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..N {
        l.check("C2", [rng.range(-1e6, 1e6), -0.0, rng.range(-1e6, 1e6)]);
    }
    for &v in SPECIAL_F32 {
        for &h in SPECIAL_HUE {
            l.check("C2", [h, -0.0, v]);
        }
    }
}

#[test]
fn c3_achromatic_all_value_classes() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 3);
    for &s in &[0.0f32, -0.0f32] {
        for &v in SPECIAL_F32 {
            for _ in 0..64 {
                l.check("C3", [rng.any_f32(), s, v]);
            }
            for &h in SPECIAL_HUE {
                l.check("C3", [h, s, v]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C4–C11: every arm of `switch (i)`.
// ---------------------------------------------------------------------------

fn sector_row(row: &str, salt: u64, lo: f32, hi: f32) {
    let l = libs();
    let mut rng = Rng::new(SEED ^ salt);
    for _ in 0..N {
        let h = rng.range(lo, hi);
        // s strictly non-zero so the chromatic path is taken.
        let s = rng.range(f32::EPSILON, 1.0);
        let v = rng.range(0.0, 1.0);
        l.check(row, [h, s, v]);
    }
}

#[test]
fn c4_sector_0() {
    sector_row("C4", 4, 0.0, 60.0);
}

#[test]
fn c5_sector_1() {
    sector_row("C5", 5, 60.0, 120.0);
}

#[test]
fn c6_sector_2() {
    sector_row("C6", 6, 120.0, 180.0);
}

#[test]
fn c7_sector_3() {
    sector_row("C7", 7, 180.0, 240.0);
}

#[test]
fn c8_sector_4() {
    sector_row("C8", 8, 240.0, 300.0);
}

#[test]
fn c9_sector_default_index_5() {
    sector_row("C9", 9, 300.0, 360.0);
}

#[test]
fn c10_sector_default_index_ge_6() {
    sector_row("C10", 10, 360.0, 1e6);
}

#[test]
fn c11_sector_default_negative_index() {
    sector_row("C11", 11, -1e6, -1e-6);
}

// ---------------------------------------------------------------------------
// C12–C13: the `f == 0` boundary and the sector tipping points.
// ---------------------------------------------------------------------------

#[test]
fn c12_exact_sector_boundaries() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 12);
    let boundaries: &[f32] = &[
        0.0, -0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0, 420.0, 480.0, 540.0, 600.0, -60.0,
        -120.0, -180.0, -240.0, -300.0, -360.0,
    ];
    for &h in boundaries {
        for _ in 0..2000 {
            let s = rng.range(f32::EPSILON, 1.0);
            let v = rng.range(0.0, 1.0);
            l.check("C12", [h, s, v]);
        }
        for &s in SPECIAL_F32 {
            for &v in SPECIAL_F32 {
                l.check("C12", [h, s, v]);
            }
        }
    }
}

#[test]
fn c13_one_ulp_around_boundaries() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 13);
    let boundaries: &[f32] = &[
        0.0, 60.0, 120.0, 180.0, 240.0, 300.0, 360.0, 420.0, -60.0, -120.0, -180.0, -240.0,
        -300.0, -360.0,
    ];
    for &b in boundaries {
        // Walk a few ULPs either side of the boundary.
        for k in 0..8i32 {
            for h in [next_after(b, k), next_after(b, -k)] {
                for _ in 0..200 {
                    let s = rng.range(f32::EPSILON, 1.0);
                    let v = rng.range(0.0, 1.0);
                    l.check("C13", [h, s, v]);
                }
                l.check("C13", [h, 1.0, 1.0]);
                l.check("C13", [h, 1.0, 0.0]);
            }
        }
    }
}

/// Step `n` ULPs away from `x` (sign of `n` gives the direction).
fn next_after(x: f32, n: i32) -> f32 {
    if n == 0 {
        return x;
    }
    let bits = x.to_bits();
    // Map to a monotone signed ordering, step, map back.
    let ord = if bits & 0x8000_0000 != 0 {
        !bits as i64 - i32::MAX as i64
    } else {
        bits as i64
    };
    let ord = ord + n as i64;
    if ord >= 0 {
        f32::from_bits(ord as u32)
    } else {
        f32::from_bits(!((ord + i32::MAX as i64) as u32))
    }
}

// ---------------------------------------------------------------------------
// C14–C15: pathological hue values (float→int conversion boundary and classes).
// ---------------------------------------------------------------------------

#[test]
fn c14_hue_beyond_int_range() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 14);
    // |h/60| >= 2^31  =>  |h| >= 1.288e11
    for _ in 0..N {
        let mag = rng.range(1.29e11, 3.0e38);
        let h = if rng.next_u32() & 1 == 0 { mag } else { -mag };
        let s = rng.range(f32::EPSILON, 1.0);
        let v = rng.range(0.0, 1.0);
        l.check("C14", [h, s, v]);
    }
    for &h in &[
        2147483648.0f32 * 60.0,
        -2147483648.0f32 * 60.0,
        f32::MAX,
        f32::MIN,
        1.29e11,
        -1.29e11,
    ] {
        for &s in SPECIAL_F32 {
            for &v in SPECIAL_F32 {
                l.check("C14", [h, s, v]);
            }
        }
    }
}

#[test]
fn c15_hue_special_classes() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 15);
    let hues: &[f32] = &[
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0x7F80_0001),
        0.0,
        -0.0,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
    ];
    for &h in hues {
        for _ in 0..1000 {
            let s = rng.range(f32::EPSILON, 1.0);
            let v = rng.range(-10.0, 10.0);
            l.check("C15", [h, s, v]);
        }
        for &s in SPECIAL_F32 {
            for &v in SPECIAL_F32 {
                l.check("C15", [h, s, v]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C16–C19: the saturation axis crossed with every sector.
// ---------------------------------------------------------------------------

/// One representative hue per `switch` arm, including both `default` flavours.
const SECTOR_HUES: &[f32] = &[30.0, 90.0, 150.0, 210.0, 270.0, 330.0, 400.0, -30.0];

fn sat_row(row: &str, salt: u64, sats: &[f32]) {
    let l = libs();
    let mut rng = Rng::new(SEED ^ salt);
    for &s in sats {
        for &h0 in SECTOR_HUES {
            for _ in 0..400 {
                let h = h0 + rng.range(-29.0, 29.0);
                let v = rng.range(-10.0, 10.0);
                l.check(row, [h, s, v]);
            }
            for &v in SPECIAL_F32 {
                l.check(row, [h0, s, v]);
            }
        }
    }
}

#[test]
fn c16_saturation_small_to_one() {
    sat_row(
        "C16",
        16,
        &[
            f32::from_bits(1),
            f32::MIN_POSITIVE,
            1e-30,
            1e-7,
            0.25,
            0.5,
            0.999_999_9,
            1.0,
        ],
    );
}

#[test]
fn c17_saturation_above_one() {
    sat_row("C17", 17, &[1.000_000_1, 1.5, 2.0, 255.0, 1e10, 1e30, f32::MAX]);
}

#[test]
fn c18_saturation_negative() {
    sat_row(
        "C18",
        18,
        &[
            -f32::from_bits(1),
            -1e-30,
            -0.5,
            -1.0,
            -2.0,
            -1e30,
            f32::MIN,
        ],
    );
}

#[test]
fn c19_saturation_inf_nan() {
    sat_row(
        "C19",
        19,
        &[
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            -f32::NAN,
            f32::from_bits(0x7FC0_1234),
            f32::from_bits(0x7F80_0001),
        ],
    );
}

// ---------------------------------------------------------------------------
// C20–C21: the value axis crossed with every sector.
// ---------------------------------------------------------------------------

fn val_row(row: &str, salt: u64, vals: &[f32]) {
    let l = libs();
    let mut rng = Rng::new(SEED ^ salt);
    for &v in vals {
        for &h0 in SECTOR_HUES {
            for _ in 0..400 {
                let h = h0 + rng.range(-29.0, 29.0);
                let s = rng.range(f32::EPSILON, 1.0);
                l.check(row, [h, s, v]);
            }
            for &s in SPECIAL_F32 {
                l.check(row, [h0, s, v]);
            }
        }
    }
}

#[test]
fn c20_value_finite_classes() {
    val_row(
        "C20",
        20,
        &[
            0.0,
            -0.0,
            f32::from_bits(1),
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            0.5,
            1.0,
            -1.0,
            255.0,
            1e30,
            -1e30,
            f32::MAX,
            f32::MIN,
        ],
    );
}

#[test]
fn c21_value_inf_nan() {
    val_row(
        "C21",
        21,
        &[
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NAN,
            -f32::NAN,
            f32::from_bits(0x7FC0_1234),
            f32::from_bits(0x7F80_0001),
        ],
    );
}

// ---------------------------------------------------------------------------
// C22–C24: pointer aliasing. The C reads all three inputs into locals before
// its first store, so in-place conversion is well defined and must match.
// ---------------------------------------------------------------------------

/// Run `hsv_to_rgb` with `dest` pointing `off` floats into the same buffer that
/// holds `src`, and return the whole buffer's bits so surrounding bytes are
/// checked too.
fn aliased_call(f: common::HsvToRgb, src: [f32; 3], off: usize, buf_len: usize) -> Vec<u32> {
    const POISON: f32 = f32::from_bits(0xDEAD_BEEF);
    let mut buf = vec![POISON; buf_len];
    buf[0] = src[0];
    buf[1] = src[1];
    buf[2] = src[2];
    unsafe {
        let base = buf.as_mut_ptr();
        f(base.add(off), base as *const f32);
    }
    buf.iter().map(|x| x.to_bits()).collect()
}

fn alias_row(row: &str, salt: u64, off: usize, zero_sat: bool) {
    let l = libs();
    let mut rng = Rng::new(SEED ^ salt);
    let buf_len = 8;
    let one = |src: [f32; 3]| {
        let c = aliased_call(l.c, src, off, buf_len);
        let r = aliased_call(l.rust, src, off, buf_len);
        assert_eq!(
            c,
            r,
            "[{row}] aliasing divergence at dest=src+{off} for src={:?} \
             (bits {:08x?})\n  C    = {:08x?}\n  Rust = {:08x?}",
            src,
            [src[0].to_bits(), src[1].to_bits(), src[2].to_bits()],
            c,
            r
        );
    };
    for _ in 0..N / 4 {
        let h = rng.range(-400.0, 760.0);
        let s = if zero_sat {
            if rng.next_u32() & 1 == 0 { 0.0 } else { -0.0 }
        } else {
            rng.range(f32::EPSILON, 1.0)
        };
        let v = rng.range(0.0, 1.0);
        one([h, s, v]);
    }
    for &h in SPECIAL_HUE {
        for &v in SPECIAL_F32 {
            let s = if zero_sat { 0.0 } else { 0.7 };
            one([h, s, v]);
        }
    }
}

#[test]
fn c22_in_place_chromatic() {
    alias_row("C22", 22, 0, false);
}

#[test]
fn c23_in_place_achromatic() {
    alias_row("C23", 23, 0, true);
}

#[test]
fn c24_partial_overlap() {
    alias_row("C24a", 240, 1, false);
    alias_row("C24b", 241, 2, false);
    alias_row("C24c", 242, 1, true);
    alias_row("C24d", 243, 2, true);
    // Non-overlapping but adjacent, to confirm no 4th element is touched.
    alias_row("C24e", 244, 3, false);
    alias_row("C24f", 245, 4, true);
}

// ---------------------------------------------------------------------------
// C25: buffer offsets — exactly three floats are written, nothing else.
// ---------------------------------------------------------------------------

#[test]
fn c25_offsets_and_write_extent() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 25);
    const POISON: f32 = f32::from_bits(0xDEAD_BEEF);
    const LEN: usize = 16;

    for src_off in 0..5usize {
        for dst_off in 8..13usize {
            for _ in 0..500 {
                let src = [
                    rng.range(-400.0, 760.0),
                    if rng.next_u32() % 5 == 0 {
                        0.0
                    } else {
                        rng.range(-2.0, 2.0)
                    },
                    rng.range(-2.0, 2.0),
                ];
                let run = |f: common::HsvToRgb| -> Vec<u32> {
                    let mut buf = [POISON; LEN];
                    buf[src_off] = src[0];
                    buf[src_off + 1] = src[1];
                    buf[src_off + 2] = src[2];
                    unsafe {
                        let base = buf.as_mut_ptr();
                        f(base.add(dst_off), base.add(src_off) as *const f32);
                    }
                    buf.iter().map(|x| x.to_bits()).collect()
                };
                let c = run(l.c);
                let r = run(l.rust);
                assert_eq!(
                    c, r,
                    "[C25] divergence at src_off={src_off} dst_off={dst_off} src={src:?}"
                );
                // Exactly three floats written: everything outside
                // dst_off..dst_off+3 (and the src slots) is untouched poison.
                for (i, &w) in c.iter().enumerate() {
                    let in_dst = i >= dst_off && i < dst_off + 3;
                    let in_src = i >= src_off && i < src_off + 3;
                    if !in_dst && !in_src {
                        assert_eq!(
                            w,
                            POISON.to_bits(),
                            "[C25] C wrote outside dest at index {i}"
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C26–C28: fuzz and grid sweeps.
// ---------------------------------------------------------------------------

#[test]
fn c26_unrestricted_bit_pattern_fuzz() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..400_000 {
        l.check("C26", [rng.any_f32(), rng.any_f32(), rng.any_f32()]);
    }
}

#[test]
fn c27_canonical_range_fuzz() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..400_000 {
        let h = rng.range(0.0, 360.0);
        let s = rng.unit();
        let v = rng.unit();
        l.check("C27", [h, s, v]);
    }
}

#[test]
fn c28_deterministic_grid_sweep() {
    let l = libs();
    let sats: &[f32] = &[0.0, -0.0, 1e-7, 0.25, 0.5, 1.0, 2.0, -0.5];
    let vals: &[f32] = &[0.0, -0.0, 0.5, 1.0, 255.0, -1.0];
    // h from -720 to 1080 in 0.37 steps ~= 4865 hues.
    let mut h = -720.0f32;
    while h <= 1080.0 {
        for &s in sats {
            for &v in vals {
                l.check("C28", [h, s, v]);
            }
        }
        h += 0.37;
    }
}

// ---------------------------------------------------------------------------
// Sanity: both libraries really are loaded from distinct files and export the
// symbol (guards against accidentally testing one library against itself).
// ---------------------------------------------------------------------------

#[test]
fn harness_loads_two_distinct_libraries() {
    let l = libs();
    assert_ne!(
        l.c_path.canonicalize().unwrap(),
        l.rust_path.canonicalize().unwrap(),
        "C and Rust .so paths must differ"
    );
    assert!(l.c_path.to_string_lossy().contains("c_src"));
    assert!(l.rust_path.to_string_lossy().contains("hsv_to_rgb_lib"));
    // Smoke: a known conversion (pure red).
    let mut d = [0.0f32; 3];
    unsafe { (l.c)(d.as_mut_ptr(), [0.0f32, 1.0, 1.0].as_ptr()) };
    assert_eq!(bits3(&d), bits3(&[1.0, 0.0, 0.0]));
    let mut d2 = [0.0f32; 3];
    unsafe { (l.rust)(d2.as_mut_ptr(), [0.0f32, 1.0, 1.0].as_ptr()) };
    assert_bits_eq("smoke", [0.0, 1.0, 1.0], bits3(&d), bits3(&d2));
}
