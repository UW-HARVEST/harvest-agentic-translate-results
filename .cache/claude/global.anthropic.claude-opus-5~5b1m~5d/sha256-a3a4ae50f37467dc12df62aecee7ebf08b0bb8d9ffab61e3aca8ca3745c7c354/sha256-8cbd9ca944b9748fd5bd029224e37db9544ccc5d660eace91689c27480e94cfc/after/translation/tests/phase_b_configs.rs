//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//! Both libraries are loaded as `.so` and called only through `colourblind`.

mod common;
use common::*;

// ---------------------------------------------------------------- C1..C3
// One row per lowest-level entry point (Protanopia / Deuteranopia /
// Tritanopia), reached through the only exported dispatcher.

fn unit_range_row(ctx: &str, imp: u32, seed: u64) {
    let mut rng = Rng::new(seed);
    for _ in 0..N {
        assert_same(ctx, imp, rng.triple(|r| r.unit()));
    }
    // plus the exact corners of the intended domain
    for &v in &[0.0f32, 1.0, 0.5] {
        assert_same(ctx, imp, [v, v, v]);
    }
}

#[test]
fn cfg_c1_protanopia_unit_range() {
    unit_range_row("C1 Protanopia S1", CB_PROTANOPIA, 0xC1);
}

#[test]
fn cfg_c2_deuteranopia_unit_range() {
    unit_range_row("C2 Deuteranopia S1", CB_DEUTERANOPIA, 0xC2);
}

#[test]
fn cfg_c3_tritanopia_unit_range() {
    unit_range_row("C3 Tritanopia S1", CB_TRITANOPIA, 0xC3);
}

// ---------------------------------------------------------------- C4
#[test]
fn cfg_c4_byte_range() {
    let mut rng = Rng::new(0xC4);
    for &imp in &VALID {
        for _ in 0..N {
            assert_same("C4 S2 byte-range", imp, rng.triple(|r| r.range(0.0, 255.0)));
        }
        for &v in &[0.0f32, 255.0, 128.0, 1.0, 254.0] {
            assert_same("C4 S2 corners", imp, [v, v, v]);
        }
    }
}

// ---------------------------------------------------------------- C5
#[test]
fn cfg_c5_signed_out_of_gamut() {
    let mut rng = Rng::new(0xC5);
    for &imp in &VALID {
        for _ in 0..N {
            assert_same("C5 S3 signed", imp, rng.triple(|r| r.range(-1.0e3, 1.0e3)));
        }
    }
}

// ---------------------------------------------------------------- C6
#[test]
fn cfg_c6_signed_zeros_exhaustive() {
    // all 8 sign combinations of ±0.0, every impairment
    for &imp in &VALID {
        for mask in 0u32..8 {
            let z = |bit: u32| if mask & bit != 0 { -0.0f32 } else { 0.0f32 };
            assert_same("C6 S4 signed zeros", imp, [z(1), z(2), z(4)]);
        }
    }
}

// ---------------------------------------------------------------- C7
#[test]
fn cfg_c7_subnormals() {
    let mut rng = Rng::new(0xC7);
    for &imp in &VALID {
        for _ in 0..N {
            assert_same("C7 S5 subnormal", imp, rng.triple(|r| r.subnormal()));
        }
        for &b in &[1u32, 2, 0x0040_0000, 0x007F_FFFF, 0x8000_0001, 0x807F_FFFF] {
            let v = f32::from_bits(b);
            assert_same("C7 S5 fixed subnormal", imp, [v, v, v]);
            assert_same("C7 S5 mixed", imp, [v, 1.0, -v]);
        }
    }
}

// ---------------------------------------------------------------- C8
#[test]
fn cfg_c8_huge_overflow() {
    let mut rng = Rng::new(0xC8);
    for &imp in &VALID {
        for _ in 0..N {
            let t = rng.triple(|r| {
                let m = r.range(0.5, 1.0);
                let s = if r.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
                s * m * f32::MAX
            });
            assert_same("C8 S6 huge", imp, t);
        }
        for &v in &[f32::MAX, -f32::MAX, f32::MAX / 2.0, 3.0e38, -3.0e38] {
            assert_same("C8 S6 fixed huge", imp, [v, v, v]);
            assert_same("C8 S6 fixed huge mix", imp, [v, -v, v]);
        }
    }
}

// ---------------------------------------------------------------- C9
#[test]
fn cfg_c9_tiny_underflow() {
    let mut rng = Rng::new(0xC9);
    for &imp in &VALID {
        for _ in 0..N {
            let t = rng.triple(|r| {
                let m = r.range(1.0, 2.0);
                let s = if r.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
                s * m * f32::MIN_POSITIVE
            });
            assert_same("C9 S7 tiny", imp, t);
        }
        for &v in &[f32::MIN_POSITIVE, -f32::MIN_POSITIVE, 1.0e-40, -1.0e-40, 1.0e-45] {
            assert_same("C9 S7 fixed tiny", imp, [v, v, v]);
        }
    }
}

// ---------------------------------------------------------------- C10
#[test]
fn cfg_c10_infinities_exhaustive() {
    const P: f32 = f32::INFINITY;
    const M: f32 = f32::NEG_INFINITY;
    // every channel independently one of {+Inf, -Inf, 1.0, 0.0, -1.0}: 125 combos
    let vals = [P, M, 1.0f32, 0.0f32, -1.0f32];
    for &imp in &VALID {
        for &r in &vals {
            for &g in &vals {
                for &b in &vals {
                    assert_same("C10 S8 inf", imp, [r, g, b]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------- C11
#[test]
fn cfg_c11_single_nan() {
    let payloads: [u32; 6] = [1, 2, 0x0040_0000, 0x0055_5555, 0x007F_FFFF, 0x0000_1234];
    let others: [f32; 5] = [0.0, -0.0, 1.0, -1.0, f32::INFINITY];
    for &imp in &VALID {
        for &p in &payloads {
            for &sign in &[0u32, 0x8000_0000] {
                let nan = f32::from_bits(sign | 0x7FC0_0000 | p);
                for pos in 0..3 {
                    for &o in &others {
                        let mut t = [o, o, o];
                        t[pos] = nan;
                        assert_same("C11 S9 single NaN", imp, t);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------- C12
// The operand-order-sensitive case: multiple NaNs with differing signs and
// payloads in one expression. ADDSS/SUBSS/MULSS keep the *destination*
// operand, so the surviving bits expose the compiler's operand order.
#[test]
fn cfg_c12_multi_nan_differing_signs_and_payloads() {
    let nans: [f32; 6] = [
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0xFFC0_0001),
        f32::from_bits(0x7FC0_0002),
        f32::from_bits(0xFFC0_0002),
        f32::from_bits(0x7FFF_FFFF),
        f32::from_bits(0xFFAA_5555),
    ];
    let finite: [f32; 3] = [1.0, -0.0, f32::INFINITY];

    for &imp in &VALID {
        // exhaustive: all 3 channels drawn from the NaN set (216 combos)
        for &r in &nans {
            for &g in &nans {
                for &b in &nans {
                    assert_same("C12 S10 triple NaN", imp, [r, g, b]);
                }
            }
        }
        // two NaNs + one finite, every channel subset
        for &n1 in &nans {
            for &n2 in &nans {
                for &f in &finite {
                    assert_same("C12 S10 NaN,NaN,fin", imp, [n1, n2, f]);
                    assert_same("C12 S10 NaN,fin,NaN", imp, [n1, f, n2]);
                    assert_same("C12 S10 fin,NaN,NaN", imp, [f, n1, n2]);
                }
            }
        }
    }
    // randomized NaN triples on top of the exhaustive grid
    let mut rng = Rng::new(0xC12);
    for &imp in &VALID {
        for _ in 0..N {
            assert_same("C12 S10 random NaN triple", imp, rng.triple(|r| r.nan()));
        }
    }
}

// ---------------------------------------------------------------- C13
#[test]
fn cfg_c13_signalling_nan() {
    let snans: [f32; 6] = [
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFF80_0001),
        f32::from_bits(0x7F80_0002),
        f32::from_bits(0xFF80_0002),
        f32::from_bits(0x7FBF_FFFF),
        f32::from_bits(0xFFBF_FFFF),
    ];
    for &imp in &VALID {
        for &s in &snans {
            for pos in 0..3 {
                let mut t = [1.0f32, 2.0, 3.0];
                t[pos] = s;
                assert_same("C13 S11 sNaN one channel", imp, t);
            }
            assert_same("C13 S11 sNaN all", imp, [s, s, s]);
        }
        // sNaN meeting qNaN in the same expression
        let q = f32::from_bits(0xFFC0_0000);
        for &s in &snans {
            assert_same("C13 S11 sNaN+qNaN", imp, [s, q, 1.0]);
            assert_same("C13 S11 sNaN+qNaN", imp, [q, s, 1.0]);
            assert_same("C13 S11 sNaN+qNaN", imp, [1.0, s, q]);
        }
    }
}

// ---------------------------------------------------------------- C14
#[test]
fn cfg_c14_fully_random_bit_patterns() {
    let mut rng = Rng::new(0xC14);
    for &imp in &VALID {
        for _ in 0..(N * 5) {
            assert_same("C14 S12 random bits", imp, rng.triple(|r| r.any_f32()));
        }
    }
}

// ---------------------------------------------------------------- C15..C18
fn aliasing_row(ctx: &str, alias: Alias, seed: u64) {
    let mut rng = Rng::new(seed);
    for &imp in &VALID {
        for _ in 0..N {
            assert_same_aliased(ctx, imp, rng.triple(|r| r.unit()), alias);
        }
        for _ in 0..N {
            assert_same_aliased(ctx, imp, rng.triple(|r| r.any_f32()), alias);
        }
        // deterministic distinguishable values so a wrong store order shows up
        assert_same_aliased(ctx, imp, [1.0, 2.0, 3.0], alias);
        assert_same_aliased(ctx, imp, [-1.0, 0.0, 255.0], alias);
    }
}

#[test]
fn cfg_c15_alias_r_eq_g() {
    aliasing_row("C15 A2 R==G", A2_R_EQ_G, 0xC15);
}

#[test]
fn cfg_c16_alias_r_eq_b() {
    aliasing_row("C16 A3 R==B", A3_R_EQ_B, 0xC16);
}

#[test]
fn cfg_c17_alias_g_eq_b() {
    aliasing_row("C17 A4 G==B", A4_G_EQ_B, 0xC17);
}

#[test]
fn cfg_c18_alias_all_same() {
    aliasing_row("C18 A5 R==G==B", A5_ALL_SAME, 0xC18);
}

// ---------------------------------------------------------------- C19
// L2: three adjacent elements of one array. `assert_same_aliased` already
// surrounds the payload with guard words, so this also proves nothing is
// written outside the triple.
#[test]
fn cfg_c19_contiguous_array_with_guards() {
    let mut rng = Rng::new(0xC19);
    for &imp in &VALID {
        for alias in ALL_ALIASES {
            for _ in 0..500 {
                assert_same_aliased("C19 L2 contiguous+guards", imp, rng.triple(|r| r.any_f32()), alias);
            }
        }
    }
}

// ---------------------------------------------------------------- C20
// L3: a real consumer pattern — a heap pixel buffer, function applied per
// pixel, whole buffer compared at the end.
#[test]
fn cfg_c20_heap_pixel_buffer() {
    const PIXELS: usize = 4096;
    let c = c_fn();
    let rust = rust_fn();
    for &imp in &VALID {
        let mut rng = Rng::new(0xC20 ^ u64::from(imp));
        let src: Vec<f32> = (0..PIXELS * 3)
            .map(|i| if i % 7 == 0 { rng.any_f32() } else { rng.unit() })
            .collect();
        let mut buf_c = src.clone();
        let mut buf_rust = src.clone();
        unsafe {
            for p in 0..PIXELS {
                let q = buf_c.as_mut_ptr().add(p * 3);
                c(imp, q, q.add(1), q.add(2));
            }
            for p in 0..PIXELS {
                let q = buf_rust.as_mut_ptr().add(p * 3);
                rust(imp, q, q.add(1), q.add(2));
            }
        }
        let mismatch = buf_c
            .iter()
            .zip(buf_rust.iter())
            .enumerate()
            .find(|(_, (a, b))| a.to_bits() != b.to_bits());
        assert!(
            mismatch.is_none(),
            "C20 heap buffer divergence at index {:?} (imp={imp}): C=0x{:08X} Rust=0x{:08X}, input=0x{:08X}",
            mismatch.map(|(i, _)| i),
            mismatch.map(|(_, (a, _))| a.to_bits()).unwrap_or(0),
            mismatch.map(|(_, (_, b))| b.to_bits()).unwrap_or(0),
            mismatch.map(|(i, _)| src[i].to_bits()).unwrap_or(0),
        );
    }
}

// ---------------------------------------------------------------- C21
// L4: repeated application of the same impairment (the transform is a
// projection, so it should converge; either way C and Rust must agree at
// every iteration).
#[test]
fn cfg_c21_repeated_application() {
    let c = c_fn();
    let rust = rust_fn();
    let mut rng = Rng::new(0xC21);
    for &imp in &VALID {
        for _ in 0..500 {
            let start = rng.triple(|r| r.range(-4.0, 4.0));
            let mut vc = start;
            let mut vr = start;
            for iter in 0..10 {
                unsafe {
                    c(imp, &mut vc[0], &mut vc[1], &mut vc[2]);
                    rust(imp, &mut vr[0], &mut vr[1], &mut vr[2]);
                }
                for k in 0..3 {
                    assert_eq!(
                        vc[k].to_bits(),
                        vr[k].to_bits(),
                        "C21 divergence at iteration {iter}, channel {k}, imp={imp}, start={start:?}: C=0x{:08X} Rust=0x{:08X}",
                        vc[k].to_bits(),
                        vr[k].to_bits()
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------- C22
// Mixed impairment sequence in one process — catches cross-helper state leakage
// (e.g. a cached matrix or a dirtied FP control word).
#[test]
fn cfg_c22_mixed_impairment_sequence() {
    let c = c_fn();
    let rust = rust_fn();
    let mut rng = Rng::new(0xC22);
    for _ in 0..1000 {
        let start = rng.triple(|r| r.any_f32());
        let mut vc = start;
        let mut vr = start;
        for step in 0..12usize {
            let imp = VALID[step % 3];
            unsafe {
                c(imp, &mut vc[0], &mut vc[1], &mut vc[2]);
                rust(imp, &mut vr[0], &mut vr[1], &mut vr[2]);
            }
            for k in 0..3 {
                assert_eq!(
                    vc[k].to_bits(),
                    vr[k].to_bits(),
                    "C22 divergence at step {step} (imp={imp}), channel {k}, start bits {:08X?}",
                    start.map(f32::to_bits)
                );
            }
        }
    }
}

// ---------------------------------------------------------------- C23
#[test]
fn cfg_c23_boundary_constants_every_position() {
    let consts: [f32; 9] = [0.0, -0.0, 1.0, -1.0, 0.5, 255.0, -255.0, f32::EPSILON, 2.0];
    let mut rng = Rng::new(0xC23);
    for &imp in &VALID {
        for &k in &consts {
            for pos in 0..3 {
                for _ in 0..100 {
                    let mut t = rng.triple(|r| r.unit());
                    t[pos] = k;
                    assert_same("C23 boundary const", imp, t);
                }
            }
        }
        // whole-triple combinations of the constants (729 combos)
        for &r in &consts {
            for &g in &consts {
                for &b in &consts {
                    assert_same("C23 const triple", imp, [r, g, b]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------- C24
// Values around powers of two, where a sum lands on an exact round-to-nearest-
// even tie: the most rounding-sensitive valid inputs.
#[test]
fn cfg_c24_rounding_ties_near_powers_of_two() {
    let mut rng = Rng::new(0xC24);
    for &imp in &VALID {
        for k in -30i32..=30 {
            let base = (2.0f32).powi(k);
            for _ in 0..40 {
                let jitter = |r: &mut Rng| {
                    let steps = (r.next_u32() % 9) as i32 - 4;
                    let mut v = base;
                    for _ in 0..steps.abs() {
                        v = if steps > 0 { next_up(v) } else { next_down(v) };
                    }
                    if r.next_u32() & 1 == 0 { v } else { -v }
                };
                let t = [jitter(&mut rng), jitter(&mut rng), jitter(&mut rng)];
                assert_same("C24 rounding ties", imp, t);
            }
            // exact powers of two, and ±1 ulp, in all three channels
            assert_same("C24 exact 2^k", imp, [base, base, base]);
            assert_same("C24 2^k ulp", imp, [next_up(base), base, next_down(base)]);
            assert_same("C24 2^k signed", imp, [-base, base, -next_up(base)]);
        }
    }
}

fn next_up(x: f32) -> f32 {
    if x.is_nan() || x == f32::INFINITY {
        return x;
    }
    let b = x.to_bits();
    if x == 0.0 {
        f32::from_bits(1)
    } else if x > 0.0 {
        f32::from_bits(b + 1)
    } else {
        f32::from_bits(b - 1)
    }
}

fn next_down(x: f32) -> f32 {
    if x.is_nan() || x == f32::NEG_INFINITY {
        return x;
    }
    let b = x.to_bits();
    if x == 0.0 {
        f32::from_bits(0x8000_0001)
    } else if x > 0.0 {
        f32::from_bits(b - 1)
    } else {
        f32::from_bits(b + 1)
    }
}

// ---------------------------------------------------------------- sanity
#[test]
fn zz_report_libraries_under_test() {
    // Fails loudly if either .so is missing, and records what was compared.
    println!("C   .so: {}", c_so_path().display());
    println!("Rust.so: {}", rust_so_path().display());
    assert!(c_so_path().is_file());
    assert!(rust_so_path().is_file());
}
