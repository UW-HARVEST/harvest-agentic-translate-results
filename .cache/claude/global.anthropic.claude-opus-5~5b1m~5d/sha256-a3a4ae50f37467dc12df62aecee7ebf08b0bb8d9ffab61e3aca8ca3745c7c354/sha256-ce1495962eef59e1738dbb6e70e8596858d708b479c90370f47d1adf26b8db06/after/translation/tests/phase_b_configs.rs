//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test loads BOTH shared objects with `libloading` and compares their
//! outputs bit-for-bit (`f32::to_bits`), so signed zeros and NaN payloads/signs
//! are all significant. Randomised rows use a fixed-seed SplitMix64 generator,
//! so a failure is always reproducible.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Rows 1-3: one impairment each, in-gamut sRGB-ish data, distinct pointers.
// ---------------------------------------------------------------------------

fn in_gamut_row(imp: u32, ctx: &str) {
    let (c, rust) = both();
    let mut rng = Rng::new(SEED ^ u64::from(imp));
    for _ in 0..20_000 {
        diff(&c, &rust, ctx, imp, [rng.unit(), rng.unit(), rng.unit()]);
    }
    // Deterministic corners of the unit cube on top of the random draws.
    for &r in &[0.0f32, 0.5, 1.0] {
        for &g in &[0.0f32, 0.5, 1.0] {
            for &b in &[0.0f32, 0.5, 1.0] {
                diff(&c, &rust, ctx, imp, [r, g, b]);
            }
        }
    }
}

#[test]
fn cfg_row01_protanopia_in_gamut() {
    in_gamut_row(CB_PROTANOPIA, "row01 protanopia in-gamut");
}

#[test]
fn cfg_row02_deuteranopia_in_gamut() {
    in_gamut_row(CB_DEUTERANOPIA, "row02 deuteranopia in-gamut");
}

#[test]
fn cfg_row03_tritanopia_in_gamut() {
    in_gamut_row(CB_TRITANOPIA, "row03 tritanopia in-gamut");
}

// ---------------------------------------------------------------------------
// Row 4: all impairments over uniformly random *bit patterns*.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row04_all_impairments_random_bitpatterns() {
    let (c, rust) = both();
    for &imp in &VALID {
        let mut rng = Rng::new(SEED.wrapping_mul(3).wrapping_add(u64::from(imp)));
        for _ in 0..60_000 {
            diff(
                &c,
                &rust,
                "row04 random bit patterns",
                imp,
                [rng.any_f32(), rng.any_f32(), rng.any_f32()],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5: wide normals (log-uniform over the whole normal exponent range).
// ---------------------------------------------------------------------------

#[test]
fn cfg_row05_all_impairments_wide_normals() {
    let (c, rust) = both();
    for &imp in &VALID {
        let mut rng = Rng::new(SEED.rotate_left(7) ^ u64::from(imp));
        for _ in 0..20_000 {
            diff(
                &c,
                &rust,
                "row05 wide normals",
                imp,
                [rng.wide_normal(), rng.wide_normal(), rng.wide_normal()],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6: signed zeros, all 8 sign combinations.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row06_signed_zeros() {
    let (c, rust) = both();
    let zeros = [0.0f32, -0.0f32];
    for &imp in &VALID {
        for &r in &zeros {
            for &g in &zeros {
                for &b in &zeros {
                    diff(&c, &rust, "row06 signed zeros", imp, [r, g, b]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7: subnormals.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row07_subnormals() {
    let (c, rust) = both();
    let smallest = f32::from_bits(1);
    let largest = f32::from_bits(0x007F_FFFF);
    let fixed = [smallest, -smallest, largest, -largest];
    for &imp in &VALID {
        for &r in &fixed {
            for &g in &fixed {
                for &b in &fixed {
                    diff(&c, &rust, "row07 subnormal corners", imp, [r, g, b]);
                }
            }
        }
        let mut rng = Rng::new(SEED ^ 0xABCD ^ u64::from(imp));
        for _ in 0..5_000 {
            diff(
                &c,
                &rust,
                "row07 random subnormals",
                imp,
                [rng.subnormal(), rng.subnormal(), rng.subnormal()],
            );
        }
        // Mixed magnitudes: a subnormal next to a normal forces the add chain
        // to round a tiny addend into a large accumulator.
        let mut rng = Rng::new(SEED ^ 0xDCBA ^ u64::from(imp));
        for _ in 0..5_000 {
            let v = [rng.subnormal(), rng.wide_normal(), rng.subnormal()];
            diff(&c, &rust, "row07 subnormal+normal mix", imp, v);
            diff(&c, &rust, "row07 subnormal+normal mix", imp, [v[1], v[0], v[2]]);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8: near f32::MAX -> overflow to +/-INF.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row08_overflow_to_infinity() {
    let (c, rust) = both();
    let big = [f32::MAX, -f32::MAX, f32::MAX / 2.0, f32::from_bits(0x7F7F_FFFE)];
    for &imp in &VALID {
        for &r in &big {
            for &g in &big {
                for &b in &big {
                    diff(&c, &rust, "row08 near-MAX corners", imp, [r, g, b]);
                }
            }
        }
        let mut rng = Rng::new(SEED ^ 0x0F0F ^ u64::from(imp));
        for _ in 0..5_000 {
            // Random mantissa with the maximum finite exponent.
            let mk = |rng: &mut Rng| {
                let sign = (rng.next_u32() & 1) << 31;
                f32::from_bits(sign | (254u32 << 23) | (rng.next_u32() & 0x007F_FFFF))
            };
            let v = [mk(&mut rng), mk(&mut rng), mk(&mut rng)];
            diff(&c, &rust, "row08 random near-MAX", imp, v);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9: infinities in every position.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row09_infinities_all_positions() {
    let (c, rust) = both();
    let vals = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0f32,
        -0.0f32,
        1.0f32,
        -1.0f32,
        f32::MAX,
    ];
    for &imp in &VALID {
        for &r in &vals {
            for &g in &vals {
                for &b in &vals {
                    diff(&c, &rust, "row09 infinities", imp, [r, g, b]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10: quiet NaN payload + sign propagation.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row10_quiet_nan_payload_propagation() {
    let (c, rust) = both();
    for &imp in &VALID {
        let mut rng = Rng::new(SEED ^ 0x0A0A_1010 ^ u64::from(imp));
        for _ in 0..8_000 {
            // A NaN in exactly one slot.
            let slot = rng.below(3) as usize;
            let mut v = [rng.unit(), rng.unit(), rng.unit()];
            v[slot] = rng.qnan();
            diff(&c, &rust, "row10 single qNaN", imp, v);

            // NaNs in two slots (different payloads/signs).
            let mut v = [rng.wide_normal(), rng.wide_normal(), rng.wide_normal()];
            let a = rng.below(3) as usize;
            let b = (a + 1 + rng.below(2) as usize) % 3;
            v[a] = rng.qnan();
            v[b] = rng.qnan();
            diff(&c, &rust, "row10 two qNaNs", imp, v);

            // All three NaN.
            diff(
                &c,
                &rust,
                "row10 triple qNaN",
                imp,
                [rng.qnan(), rng.qnan(), rng.qnan()],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11: signalling NaN quieting.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row11_signalling_nan_quieting() {
    let (c, rust) = both();
    let snans = [
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFF80_0001),
        f32::from_bits(0x7FBF_FFFF),
        f32::from_bits(0xFFBF_FFFF),
    ];
    for &imp in &VALID {
        for slot in 0usize..3 {
            for &s in &snans {
                let mut v = [0.25f32, 0.5, 0.75];
                v[slot] = s;
                diff(&c, &rust, "row11 sNaN in one slot", imp, v);
            }
        }
        for &s in &snans {
            diff(&c, &rust, "row11 sNaN everywhere", imp, [s, s, s]);
        }
        let mut rng = Rng::new(SEED ^ 0x5171 ^ u64::from(imp));
        for _ in 0..4_000 {
            diff(
                &c,
                &rust,
                "row11 random sNaN triples",
                imp,
                [rng.snan(), rng.snan(), rng.snan()],
            );
            let mut v = [rng.unit(), rng.unit(), rng.unit()];
            v[rng.below(3) as usize] = rng.snan();
            diff(&c, &rust, "row11 random sNaN mixed", imp, v);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12: two NaNs meeting inside one expression -> operand order observable.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row12_nan_vs_nan_operand_order() {
    let (c, rust) = both();
    // Deliberately sign-mismatched, distinct payloads: whichever NaN survives
    // an addss/subss/mulss identifies the destination operand, so this row is
    // what pins the transcribed operand order.
    let nans = [
        f32::from_bits(0x7FC0_0001), // +qNaN payload 1
        f32::from_bits(0xFFC0_0002), // -qNaN payload 2
        f32::from_bits(0x7FD5_5555), // +qNaN payload 0x155555
        f32::from_bits(0xFFEA_AAAA), // -qNaN payload 0x2AAAAA
        f32::from_bits(0x7F80_0003), // +sNaN
        f32::from_bits(0xFF80_0004), // -sNaN
    ];
    for &imp in &VALID {
        for &r in &nans {
            for &g in &nans {
                for &b in &nans {
                    diff(&c, &rust, "row12 NaN vs NaN", imp, [r, g, b]);
                }
            }
        }
        // NaN paired with the values that make the other operand non-NaN in
        // only some of the three sub-expressions.
        for &n in &nans {
            for &x in &[0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY] {
                diff(&c, &rust, "row12 NaN vs value", imp, [n, x, x]);
                diff(&c, &rust, "row12 NaN vs value", imp, [x, n, x]);
                diff(&c, &rust, "row12 NaN vs value", imp, [x, x, n]);
                diff(&c, &rust, "row12 NaN vs value", imp, [n, n, x]);
                diff(&c, &rust, "row12 NaN vs value", imp, [n, x, n]);
                diff(&c, &rust, "row12 NaN vs value", imp, [x, n, n]);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13: exact powers of two and dyadic values (rounding ties).
// ---------------------------------------------------------------------------

#[test]
fn cfg_row13_powers_of_two_and_ties() {
    let (c, rust) = both();
    for &imp in &VALID {
        let mut rng = Rng::new(SEED ^ 0x2222 ^ u64::from(imp));
        for _ in 0..5_000 {
            diff(
                &c,
                &rust,
                "row13 powers of two",
                imp,
                [rng.power_of_two(), rng.power_of_two(), rng.power_of_two()],
            );
        }
        // Values whose products land exactly on a rounding tie boundary is not
        // analytically constructible against these coefficients, so sweep the
        // lowest mantissa bits of 1.0 instead, which walks the tie region.
        for k in 0..2_000u32 {
            let a = f32::from_bits(1.0f32.to_bits() + k);
            let b = f32::from_bits(1.0f32.to_bits() ^ k);
            let d = f32::from_bits(0.5f32.to_bits() + k);
            diff(&c, &rust, "row13 mantissa walk", imp, [a, b, d]);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 14-17: pointer aliasing. The C reads all three components into locals
// before storing, then stores Red, Green, Blue in that order.
// ---------------------------------------------------------------------------

/// Applies the call with a caller-chosen aliasing map: `map[i]` is the index in
/// the 3-element buffer that argument `i` points at.
fn alias_row(ctx: &str, map: [usize; 3], iters: usize, exotic: bool) {
    let (c, rust) = both();
    for &imp in &VALID {
        let mut rng = Rng::new(SEED ^ 0xA11A5 ^ u64::from(imp) ^ (map[0] * 9 + map[1] * 3 + map[2]) as u64);
        for _ in 0..iters {
            let input: [f32; 3] = if exotic {
                [rng.any_f32(), rng.any_f32(), rng.any_f32()]
            } else {
                [rng.unit(), rng.unit(), rng.unit()]
            };
            let mut ca = input;
            let mut ra = input;
            unsafe {
                let p = ca.as_mut_ptr();
                c.call_ptrs(imp, p.add(map[0]), p.add(map[1]), p.add(map[2]));
                let p = ra.as_mut_ptr();
                rust.call_ptrs(imp, p.add(map[0]), p.add(map[1]), p.add(map[2]));
            }
            assert_same(ctx, imp, &input, &ca, &ra);
        }
    }
}

#[test]
fn cfg_row14_alias_r_eq_g() {
    // R and G are the same object, B distinct.
    alias_row("row14 R==G", [0, 0, 1], 6_000, false);
    alias_row("row14 R==G exotic", [0, 0, 1], 6_000, true);
    alias_row("row14 R==G (other slot)", [2, 2, 0], 3_000, true);
}

#[test]
fn cfg_row15_alias_r_eq_b() {
    alias_row("row15 R==B", [0, 1, 0], 6_000, false);
    alias_row("row15 R==B exotic", [0, 1, 0], 6_000, true);
    alias_row("row15 R==B (other slot)", [2, 0, 2], 3_000, true);
}

#[test]
fn cfg_row16_alias_g_eq_b() {
    alias_row("row16 G==B", [0, 1, 1], 6_000, false);
    alias_row("row16 G==B exotic", [0, 1, 1], 6_000, true);
    alias_row("row16 G==B (other slot)", [1, 2, 2], 3_000, true);
}

#[test]
fn cfg_row17_alias_all_three() {
    alias_row("row17 R==G==B", [0, 0, 0], 6_000, false);
    alias_row("row17 R==G==B exotic", [0, 0, 0], 6_000, true);
    alias_row("row17 R==G==B slot1", [1, 1, 1], 3_000, true);
    alias_row("row17 R==G==B slot2", [2, 2, 2], 3_000, true);
}

// ---------------------------------------------------------------------------
// Row 18: all 6 permutations of three distinct pointers.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row18_permuted_pointers() {
    const PERMS: [[usize; 3]; 6] =
        [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];
    for p in PERMS {
        alias_row("row18 permuted pointers", p, 3_000, true);
    }
}

// ---------------------------------------------------------------------------
// Row 19: misaligned pointers (movss has no alignment requirement).
// ---------------------------------------------------------------------------

#[test]
fn cfg_row19_misaligned_layout() {
    let (c, rust) = both();
    for &imp in &VALID {
        for offset in 1usize..=3 {
            let mut rng = Rng::new(SEED ^ 0x5A11 ^ u64::from(imp) ^ offset as u64);
            for _ in 0..3_000 {
                let input = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
                // 16 bytes of scratch so a 3-float window at `offset` fits.
                let mut cbuf = [0u8; 16];
                let mut rbuf = [0u8; 16];
                for i in 0..3 {
                    let le = input[i].to_bits().to_le_bytes();
                    cbuf[offset + 4 * i..offset + 4 * i + 4].copy_from_slice(&le);
                    rbuf[offset + 4 * i..offset + 4 * i + 4].copy_from_slice(&le);
                }
                unsafe {
                    let p = cbuf.as_mut_ptr().add(offset).cast::<f32>();
                    c.call_ptrs(imp, p, p.byte_add(4), p.byte_add(8));
                    let p = rbuf.as_mut_ptr().add(offset).cast::<f32>();
                    rust.call_ptrs(imp, p, p.byte_add(4), p.byte_add(8));
                }
                let read = |buf: &[u8; 16]| {
                    let mut out = [0f32; 3];
                    for i in 0..3 {
                        let mut w = [0u8; 4];
                        w.copy_from_slice(&buf[offset + 4 * i..offset + 4 * i + 4]);
                        out[i] = f32::from_bits(u32::from_le_bytes(w));
                    }
                    out
                };
                assert_same(
                    &format!("row19 misaligned offset {offset}"),
                    imp,
                    &input,
                    &read(&cbuf),
                    &read(&rbuf),
                );
                // Bytes outside the three floats must be untouched.
                assert_eq!(
                    cbuf[..offset],
                    rbuf[..offset],
                    "row19: leading padding diverged"
                );
                assert_eq!(
                    cbuf[offset + 12..],
                    rbuf[offset + 12..],
                    "row19: trailing padding diverged"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20: three separate allocations instead of one contiguous array.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row20_separate_allocations() {
    let (c, rust) = both();
    for &imp in &VALID {
        let mut rng = Rng::new(SEED ^ 0x5EBA_2020 ^ u64::from(imp));
        for _ in 0..3_000 {
            let input = [rng.any_f32(), rng.any_f32(), rng.any_f32()];

            let mut c_boxes = [
                Box::new(input[0]),
                Box::new(input[1]),
                Box::new(input[2]),
            ];
            let mut r_boxes = [
                Box::new(input[0]),
                Box::new(input[1]),
                Box::new(input[2]),
            ];
            unsafe {
                let [a, b, d] = &mut c_boxes;
                c.call_ptrs(imp, &mut **a, &mut **b, &mut **d);
                let [a, b, d] = &mut r_boxes;
                rust.call_ptrs(imp, &mut **a, &mut **b, &mut **d);
            }
            let co = [*c_boxes[0], *c_boxes[1], *c_boxes[2]];
            let ro = [*r_boxes[0], *r_boxes[1], *r_boxes[2]];
            assert_same("row20 separate allocations", imp, &input, &co, &ro);

            // And the same values through a contiguous array must agree with
            // the boxed layout too: the maths is layout-independent.
            let mut contig = input;
            c.call(imp, &mut contig);
            assert_same("row20 boxed vs contiguous (C)", imp, &input, &contig, &co);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 21: repeated in-place application.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row21_repeated_in_place() {
    let (c, rust) = both();
    for &imp in &VALID {
        let mut rng = Rng::new(SEED ^ 0x2121 ^ u64::from(imp));
        for _ in 0..500 {
            let input = [rng.unit(), rng.unit(), rng.unit()];
            let mut a = input;
            let mut b = input;
            for step in 0..100 {
                c.call(imp, &mut a);
                rust.call(imp, &mut b);
                assert_same(
                    &format!("row21 repeated in place, step {step}"),
                    imp,
                    &input,
                    &a,
                    &b,
                );
            }
        }
        // Same, starting from arbitrary bit patterns so the iteration wanders
        // through NaN/INF territory.
        let mut rng = Rng::new(SEED ^ 0x1212 ^ u64::from(imp));
        for _ in 0..200 {
            let input = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
            let mut a = input;
            let mut b = input;
            for step in 0..25 {
                c.call(imp, &mut a);
                rust.call(imp, &mut b);
                assert_same(
                    &format!("row21 repeated in place (exotic), step {step}"),
                    imp,
                    &input,
                    &a,
                    &b,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 22: interleaved impairments on the same buffer (mode switching).
// ---------------------------------------------------------------------------

#[test]
fn cfg_row22_interleaved_impairments() {
    let (c, rust) = both();
    let mut rng = Rng::new(SEED ^ 0x2222_3333);
    for _ in 0..500 {
        let input = [rng.any_f32(), rng.any_f32(), rng.any_f32()];
        let mut a = input;
        let mut b = input;
        for step in 0..50 {
            // Mix valid modes with out-of-range ones, which must be no-ops and
            // must not perturb the sequence.
            let imp = if rng.below(5) == 0 {
                3 + rng.below(1000)
            } else {
                VALID[rng.below(3) as usize]
            };
            c.call(imp, &mut a);
            rust.call(imp, &mut b);
            assert_same(
                &format!("row22 interleaved, step {step}"),
                imp,
                &input,
                &a,
                &b,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 23: full mode x value-class cross product.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row23_mode_by_class_cross_product() {
    let (c, rust) = both();
    let classes: [(&str, f32); 12] = [
        ("+0", 0.0),
        ("-0", -0.0),
        ("smallest subnormal", f32::from_bits(1)),
        ("largest subnormal", f32::from_bits(0x007F_FFFF)),
        ("MIN_POSITIVE", f32::MIN_POSITIVE),
        ("1.0", 1.0),
        ("-1.0", -1.0),
        ("in-gamut 0.375", 0.375),
        ("MAX", f32::MAX),
        ("-MAX", -f32::MAX),
        ("+INF", f32::INFINITY),
        ("qNaN 0x7FC00001", f32::from_bits(0x7FC0_0001)),
    ];
    let extra: [(&str, f32); 3] = [
        ("-INF", f32::NEG_INFINITY),
        ("sNaN 0x7F800001", f32::from_bits(0x7F80_0001)),
        ("-qNaN 0xFFC00001", f32::from_bits(0xFFC0_0001)),
    ];
    for &imp in &VALID {
        for &(_, r) in classes.iter().chain(extra.iter()) {
            for &(_, g) in classes.iter().chain(extra.iter()) {
                for &(_, b) in classes.iter().chain(extra.iter()) {
                    diff(&c, &rust, "row23 mode x class", imp, [r, g, b]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 24: the library's real-world input - 8-bit sRGB channels scaled to [0,1].
// ---------------------------------------------------------------------------

#[test]
fn cfg_row24_srgb_pixel_sweep() {
    let (c, rust) = both();
    for &imp in &VALID {
        // Exhaustive over the 8-bit grey diagonal plus the primary/secondary
        // corners, then a large deterministic sample of the full 24-bit cube.
        for v in 0..=255u32 {
            let f = v as f32 / 255.0;
            diff(&c, &rust, "row24 grey ramp", imp, [f, f, f]);
            diff(&c, &rust, "row24 red ramp", imp, [f, 0.0, 0.0]);
            diff(&c, &rust, "row24 green ramp", imp, [0.0, f, 0.0]);
            diff(&c, &rust, "row24 blue ramp", imp, [0.0, 0.0, f]);
            diff(&c, &rust, "row24 cyan ramp", imp, [0.0, f, f]);
            diff(&c, &rust, "row24 magenta ramp", imp, [f, 0.0, f]);
            diff(&c, &rust, "row24 yellow ramp", imp, [f, f, 0.0]);
        }
        let mut rng = Rng::new(SEED ^ 0x2424 ^ u64::from(imp));
        for _ in 0..200_000 {
            let q = |rng: &mut Rng| (rng.below(256) as f32) / 255.0;
            let v = [q(&mut rng), q(&mut rng), q(&mut rng)];
            diff(&c, &rust, "row24 random sRGB pixel", imp, v);
        }
        // Also the unscaled 0..255 range, in case a consumer feeds raw bytes.
        let mut rng = Rng::new(SEED ^ 0x4242 ^ u64::from(imp));
        for _ in 0..50_000 {
            let v = [
                rng.below(256) as f32,
                rng.below(256) as f32,
                rng.below(256) as f32,
            ];
            diff(&c, &rust, "row24 raw byte range", imp, v);
        }
    }
}
