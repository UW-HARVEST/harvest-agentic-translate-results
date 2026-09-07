//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1..C24). Every test drives BOTH the C
//! `.so` and the Rust `.so` through `libloading` and compares the returned
//! `lm_vec2` bit-for-bit, over many randomized inputs with a fixed seed.

mod harness;

use harness::*;

// ---------------------------------------------------------------------------
// helpers for building triangles
// ---------------------------------------------------------------------------

/// A point at barycentric weights `(u, v)` of the triangle, using the SAME
/// convention the C uses: `u` along `p3-p1`, `v` along `p2-p1`.
fn lerp_point(p1: Vec2, p2: Vec2, p3: Vec2, u: f32, v: f32) -> Vec2 {
    Vec2::new(
        p1.x + u * (p3.x - p1.x) + v * (p2.x - p1.x),
        p1.y + u * (p3.y - p1.y) + v * (p2.y - p1.y),
    )
}

fn cross(p1: Vec2, p2: Vec2, p3: Vec2) -> f32 {
    (p2.x - p1.x) * (p3.y - p1.y) - (p2.y - p1.y) * (p3.x - p1.x)
}

/// A well-conditioned triangle: finite, non-degenerate, |signed area| not tiny.
fn nondegenerate_triangle(rng: &mut Rng, want_ccw: bool) -> (Vec2, Vec2, Vec2) {
    loop {
        let p1 = rng.normal_vec();
        let p2 = rng.normal_vec();
        let p3 = rng.normal_vec();
        let c = cross(p1, p2, p3);
        if !c.is_finite() || c.abs() < 1e-3 {
            continue;
        }
        let ccw = c > 0.0;
        if ccw == want_ccw {
            return (p1, p2, p3);
        }
        return (p1, p3, p2); // flip winding
    }
}

// ---------------------------------------------------------------------------
// C1 / C2 — non-degenerate triangles, both windings, interior query point
// ---------------------------------------------------------------------------

fn interior_row(row: &str, seed: u64, ccw: bool, iters: usize) {
    let mut rng = Rng::new(seed);
    for _ in 0..iters {
        let (p1, p2, p3) = nondegenerate_triangle(&mut rng, ccw);
        // strictly inside: u, v > 0 and u + v < 1
        let mut u = rng.unit();
        let mut v = rng.unit();
        if u + v > 1.0 {
            u = 1.0 - u;
            v = 1.0 - v;
        }
        let p = lerp_point(p1, p2, p3, u, v);
        assert_same(row, [p1, p2, p3, p]);
    }
}

#[test]
fn c1_interior_ccw() {
    interior_row("C1", 0x0000_0001, true, 20_000);
}

#[test]
fn c2_interior_cw() {
    interior_row("C2", 0x0000_0002, false, 20_000);
}

// ---------------------------------------------------------------------------
// C3 — canonical axis-aligned reference triangle
// ---------------------------------------------------------------------------

#[test]
fn c3_reference_triangle() {
    let mut rng = Rng::new(0x0000_0003);
    let p1 = Vec2::new(0.0, 0.0);
    let p2 = Vec2::new(1.0, 0.0);
    let p3 = Vec2::new(0.0, 1.0);
    for i in 0..20_000 {
        let p = match i % 4 {
            0 => Vec2::new(rng.unit(), rng.unit()),
            1 => Vec2::new(rng.range(-2.0, 3.0), rng.range(-2.0, 3.0)),
            2 => Vec2::new(
                (rng.below(9) as f32) / 8.0,
                (rng.below(9) as f32) / 8.0,
            ),
            _ => Vec2::new(rng.normal_f32(), rng.normal_f32()),
        };
        assert_same("C3", [p1, p2, p3, p]);
    }
    // exact spot checks of the documented (u along p3-p1, v along p2-p1) order
    let r = assert_same("C3", [p1, p2, p3, Vec2::new(0.25, 0.75)]);
    assert_eq!(r.bits(), (0.75f32.to_bits(), 0.25f32.to_bits()));
}

// ---------------------------------------------------------------------------
// C4 — query point at vertices and edge midpoints (exact weights)
// ---------------------------------------------------------------------------

#[test]
fn c4_vertices_and_midpoints() {
    let mut rng = Rng::new(0x0000_0004);
    // (u, v) pairs: the three vertices, the three edge midpoints, the centroid
    let weights = [
        (0.0f32, 0.0f32), // p1
        (0.0, 1.0),       // p2
        (1.0, 0.0),       // p3
        (0.0, 0.5),       // mid p1p2
        (0.5, 0.0),       // mid p1p3
        (0.5, 0.5),       // mid p2p3
    ];
    for _ in 0..2_000 {
        let (p1, p2, p3) = nondegenerate_triangle(&mut rng, true);
        for &(u, v) in &weights {
            let p = lerp_point(p1, p2, p3, u, v);
            assert_same("C4", [p1, p2, p3, p]);
        }
    }
}

// ---------------------------------------------------------------------------
// C5 / C6 — query point outside / far outside
// ---------------------------------------------------------------------------

fn outside_row(row: &str, seed: u64, span: f32, iters: usize) {
    let mut rng = Rng::new(seed);
    for _ in 0..iters {
        let ccw = rng.bool();
        let (p1, p2, p3) = nondegenerate_triangle(&mut rng, ccw);
        let u = rng.range(-span, span);
        let v = rng.range(-span, span);
        let p = lerp_point(p1, p2, p3, u, v);
        assert_same(row, [p1, p2, p3, p]);
    }
}

#[test]
fn c5_outside_point() {
    outside_row("C5", 0x0000_0005, 4.0, 20_000);
}

#[test]
fn c6_far_outside_point() {
    outside_row("C6", 0x0000_0006, 1.0e6, 20_000);
}

// ---------------------------------------------------------------------------
// C7 — needle-thin near-degenerate triangle
// ---------------------------------------------------------------------------

#[test]
fn c7_needle_thin_triangle() {
    let mut rng = Rng::new(0x0000_0007);
    for _ in 0..20_000 {
        let p1 = rng.normal_vec();
        let p2 = rng.normal_vec();
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        let t = rng.range(-1.5, 2.5);
        // perturb off the p1p2 line by a tiny amount along the normal
        let eps = rng.scaled_f32(-20, -10).abs();
        let p3 = Vec2::new(p1.x + t * dx - eps * dy, p1.y + t * dy + eps * dx);
        let u = rng.unit();
        let v = rng.unit();
        let p = lerp_point(p1, p2, p3, u, v);
        assert_same("C7", [p1, p2, p3, p]);
    }
}

// ---------------------------------------------------------------------------
// C8 — exactly collinear triangle
// ---------------------------------------------------------------------------

#[test]
fn c8_collinear_triangle() {
    let mut rng = Rng::new(0x0000_0008);
    for i in 0..20_000 {
        let p1 = rng.normal_vec();
        let p2 = rng.normal_vec();
        let t = match i % 3 {
            0 => rng.range(-3.0, 3.0),
            1 => (rng.below(9) as f32) / 4.0, // exact quarters, incl. 0 and 2
            _ => rng.range(0.0, 1.0),
        };
        let p3 = Vec2::new(p1.x + t * (p2.x - p1.x), p1.y + t * (p2.y - p1.y));
        let p = if i % 2 == 0 {
            rng.normal_vec()
        } else {
            // also on the line
            let s = rng.range(-2.0, 2.0);
            Vec2::new(p1.x + s * (p2.x - p1.x), p1.y + s * (p2.y - p1.y))
        };
        assert_same("C8", [p1, p2, p3, p]);
    }
}

// ---------------------------------------------------------------------------
// C9 — coincident vertices
// ---------------------------------------------------------------------------

#[test]
fn c9_coincident_vertices() {
    let mut rng = Rng::new(0x0000_0009);
    for i in 0..20_000 {
        let a = rng.normal_vec();
        let b = rng.normal_vec();
        let p = if i % 5 == 0 { a } else { rng.normal_vec() };
        let args = match i % 4 {
            0 => [a, a, b, p],  // p1 == p2
            1 => [a, b, a, p],  // p1 == p3
            2 => [b, a, a, p],  // p2 == p3
            _ => [a, a, a, p],  // all three
        };
        assert_same("C9", args);
    }
}

// ---------------------------------------------------------------------------
// C10 / C11 / C12 — magnitude regimes
// ---------------------------------------------------------------------------

fn magnitude_row(row: &str, seed: u64, lo: i32, hi: i32, per_slot: bool, iters: usize) {
    let mut rng = Rng::new(seed);
    for _ in 0..iters {
        let mut args = [Vec2::default(); 4];
        if per_slot {
            for s in 0..8 {
                // each coordinate gets its own decade -> mixed scales
                let e = lo + rng.below((hi - lo + 1) as u32) as i32;
                *slot_mut(&mut args, s) = rng.scaled_f32(e, e);
            }
        } else {
            for s in 0..8 {
                *slot_mut(&mut args, s) = rng.scaled_f32(lo, hi);
            }
        }
        assert_same(row, args);
    }
}

#[test]
fn c10_tiny_magnitudes() {
    // 2^-100 .. 2^-66 : products underflow toward zero
    magnitude_row("C10", 0x0000_000a, -100, -66, false, 20_000);
}

#[test]
fn c11_huge_magnitudes() {
    // 2^60 .. 2^100 : products overflow to inf, inf-inf -> nan
    magnitude_row("C11", 0x0000_000b, 60, 100, false, 20_000);
}

#[test]
fn c12_mixed_magnitudes() {
    // every coordinate from an independent decade -> catastrophic cancellation
    magnitude_row("C12", 0x0000_000c, -83, 83, true, 40_000);
}

// ---------------------------------------------------------------------------
// C13 — small integer coordinates (exact arithmetic, many exact degeneracies)
// ---------------------------------------------------------------------------

#[test]
fn c13_integer_coordinates() {
    let mut rng = Rng::new(0x0000_000d);
    for _ in 0..40_000 {
        let mut args = [Vec2::default(); 4];
        for s in 0..8 {
            *slot_mut(&mut args, s) = rng.below(129) as f32 - 64.0;
        }
        assert_same("C13", args);
    }
}

// ---------------------------------------------------------------------------
// C14 — signed zeros in every slot
// ---------------------------------------------------------------------------

#[test]
fn c14_signed_zeros() {
    let mut rng = Rng::new(0x0000_000e);
    for slot in 0..8 {
        for &neg in &[false, true] {
            for _ in 0..2_000 {
                let mut args = [
                    rng.normal_vec(),
                    rng.normal_vec(),
                    rng.normal_vec(),
                    rng.normal_vec(),
                ];
                *slot_mut(&mut args, slot) = if neg { -0.0 } else { 0.0 };
                assert_same(&format!("C14[{}{}]", if neg { "-0@" } else { "+0@" }, SLOT_NAMES[slot]), args);
            }
        }
    }
    // all-zero, every sign combination of the 8 slots
    for mask in 0u32..256 {
        let mut args = [Vec2::default(); 4];
        for s in 0..8 {
            *slot_mut(&mut args, s) = if mask >> s & 1 == 1 { -0.0 } else { 0.0 };
        }
        assert_same("C14[all-zero]", args);
    }
}

// ---------------------------------------------------------------------------
// C15 — subnormals
// ---------------------------------------------------------------------------

#[test]
fn c15_subnormals() {
    let mut rng = Rng::new(0x0000_000f);
    for slot in 0..8 {
        for _ in 0..2_000 {
            let mut args = [
                rng.normal_vec(),
                rng.normal_vec(),
                rng.normal_vec(),
                rng.normal_vec(),
            ];
            *slot_mut(&mut args, slot) = rng.subnormal_f32();
            assert_same("C15[one-slot]", args);
        }
    }
    for _ in 0..4_000 {
        let mut args = [Vec2::default(); 4];
        for s in 0..8 {
            *slot_mut(&mut args, s) = rng.subnormal_f32();
        }
        assert_same("C15[all]", args);
    }
    // the extremal subnormals in every slot
    for slot in 0..8 {
        for &b in &[0x0000_0001u32, 0x007f_ffff, 0x8000_0001, 0x807f_ffff] {
            let mut args = [
                rng.normal_vec(),
                rng.normal_vec(),
                rng.normal_vec(),
                rng.normal_vec(),
            ];
            *slot_mut(&mut args, slot) = f32::from_bits(b);
            assert_same("C15[extremal]", args);
        }
    }
}

// ---------------------------------------------------------------------------
// C16 — infinities
// ---------------------------------------------------------------------------

#[test]
fn c16_infinities() {
    let mut rng = Rng::new(0x0000_0010);
    for slot in 0..8 {
        for &neg in &[false, true] {
            for _ in 0..2_000 {
                let mut args = [
                    rng.normal_vec(),
                    rng.normal_vec(),
                    rng.normal_vec(),
                    rng.normal_vec(),
                ];
                *slot_mut(&mut args, slot) = if neg { f32::NEG_INFINITY } else { f32::INFINITY };
                assert_same("C16[one-slot]", args);
            }
        }
    }
    for mask in 0u32..256 {
        let mut args = [Vec2::default(); 4];
        for s in 0..8 {
            *slot_mut(&mut args, s) = if mask >> s & 1 == 1 {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            };
        }
        assert_same("C16[all-inf]", args);
    }
}

// ---------------------------------------------------------------------------
// C17 — a single quiet NaN with a random payload in every slot
// ---------------------------------------------------------------------------

#[test]
fn c17_single_qnan_per_slot() {
    let mut rng = Rng::new(0x0000_0011);
    for slot in 0..8 {
        for _ in 0..8_000 {
            let mut args = [
                rng.normal_vec(),
                rng.normal_vec(),
                rng.normal_vec(),
                rng.normal_vec(),
            ];
            *slot_mut(&mut args, slot) = rng.qnan_f32();
            assert_same(&format!("C17[{}]", SLOT_NAMES[slot]), args);
        }
    }
}

// ---------------------------------------------------------------------------
// C18 — several simultaneous NaNs with distinct payloads (payload race)
// ---------------------------------------------------------------------------

#[test]
fn c18_multiple_nan_payload_race() {
    let mut rng = Rng::new(0x0000_0012);
    for _ in 0..200_000 {
        let mut args = [
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
        ];
        let count = 2 + rng.below(7); // 2..=8 NaN slots
        let mut placed = 0;
        let mut mask = 0u32;
        while placed < count {
            let slot = rng.below(8) as usize;
            if mask >> slot & 1 == 1 {
                continue;
            }
            mask |= 1 << slot;
            *slot_mut(&mut args, slot) = rng.qnan_f32();
            placed += 1;
        }
        assert_same("C18", args);
    }
}

// ---------------------------------------------------------------------------
// C19 — signaling NaNs
// ---------------------------------------------------------------------------

#[test]
fn c19_signaling_nans() {
    let mut rng = Rng::new(0x0000_0013);
    for i in 0..100_000 {
        let mut args = [
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
        ];
        let count = 1 + rng.below(4);
        for _ in 0..count {
            let slot = rng.below(8) as usize;
            *slot_mut(&mut args, slot) = rng.snan_f32();
        }
        // half the time mix in quiet NaNs too, so quiet-vs-signaling ordering matters
        if i % 2 == 0 {
            let slot = rng.below(8) as usize;
            *slot_mut(&mut args, slot) = rng.qnan_f32();
        }
        assert_same("C19", args);
    }
    // the minimal / maximal signaling payloads in every slot
    for slot in 0..8 {
        for &b in &[0x7f80_0001u32, 0x7fbf_ffff, 0xff80_0001, 0xffbf_ffff] {
            let mut args = [
                rng.normal_vec(),
                rng.normal_vec(),
                rng.normal_vec(),
                rng.normal_vec(),
            ];
            *slot_mut(&mut args, slot) = f32::from_bits(b);
            assert_same("C19[extremal-snan]", args);
        }
    }
}

// ---------------------------------------------------------------------------
// C20 — fully unstructured random bit patterns
// ---------------------------------------------------------------------------

#[test]
fn c20_random_bit_patterns() {
    let mut rng = Rng::new(0x0000_0014);
    for _ in 0..400_000 {
        let args = [rng.any_vec(), rng.any_vec(), rng.any_vec(), rng.any_vec()];
        assert_same("C20", args);
    }
}

// ---------------------------------------------------------------------------
// C21 — exponent sweep: every one of the 256 f32 exponents, in every slot
// ---------------------------------------------------------------------------

#[test]
fn c21_exponent_sweep() {
    let mut rng = Rng::new(0x0000_0015);
    for exp in 0u32..256 {
        for _ in 0..512 {
            let mut args = [Vec2::default(); 4];
            for s in 0..8 {
                // every slot uses this exponent; mantissa and sign random
                let mant = rng.next_u32() & 0x007f_ffff;
                let sign = (rng.next_u32() & 1) << 31;
                *slot_mut(&mut args, s) = f32::from_bits(sign | (exp << 23) | mant);
            }
            assert_same(&format!("C21[exp=0x{exp:02x}]"), args);
        }
    }
}

// ---------------------------------------------------------------------------
// C22 — ABI stress: dirty XMM registers, field-by-field readback
// ---------------------------------------------------------------------------

/// Perturbs the caller's float register state before the FFI call so a wrong
/// argument/return register assignment cannot accidentally look correct.
#[inline(never)]
fn dirty_xmm(rng: &mut Rng) -> f32 {
    let mut acc = 0.0f32;
    for _ in 0..8 {
        acc = acc * 1.000_001 + rng.normal_f32();
    }
    acc
}

#[test]
fn c22_abi_register_stress() {
    let mut rng = Rng::new(0x0000_0016);
    let mut sink = 0.0f32;
    for _ in 0..20_000 {
        let args = [
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
            rng.normal_vec(),
        ];
        sink += dirty_xmm(&mut rng);
        let c = call_c(args[0], args[1], args[2], args[3]);
        sink += dirty_xmm(&mut rng);
        let r = call_rust(args[0], args[1], args[2], args[3]);
        sink += dirty_xmm(&mut rng);
        // read fields individually, not as a tuple, to catch swapped halves
        assert_eq!(
            c.x.to_bits(),
            r.x.to_bits(),
            "C22 .x mismatch for {args:?}"
        );
        assert_eq!(
            c.y.to_bits(),
            r.y.to_bits(),
            "C22 .y mismatch for {args:?}"
        );
        // the C's (u, v) order must be reproduced: swapping would break this
        assert_ne!(std::ptr::addr_of!(c.x), std::ptr::addr_of!(c.y));
    }
    assert!(sink.is_finite() || sink.is_nan()); // keep `sink` alive
}

// ---------------------------------------------------------------------------
// C23 — argument aliasing: one value shared by several parameters
// ---------------------------------------------------------------------------

#[test]
fn c23_argument_aliasing() {
    let mut rng = Rng::new(0x0000_0017);
    for mask in 1u32..16 {
        for _ in 0..2_000 {
            let shared = rng.normal_vec();
            let mut args = [
                rng.normal_vec(),
                rng.normal_vec(),
                rng.normal_vec(),
                rng.normal_vec(),
            ];
            for i in 0..4 {
                if mask >> i & 1 == 1 {
                    args[i] = shared;
                }
            }
            assert_same(&format!("C23[mask={mask:#x}]"), args);
        }
    }
}

// ---------------------------------------------------------------------------
// C24 — statelessness: repeated calls interleaved with unrelated calls
// ---------------------------------------------------------------------------

#[test]
fn c24_statelessness() {
    let mut rng = Rng::new(0x0000_0018);
    let fixed = [
        Vec2::new(-5.5, 2.25),
        Vec2::new(7.125, -3.75),
        Vec2::new(0.5, 9.5),
        Vec2::new(1.0, 1.0),
    ];
    let baseline_c = call_c(fixed[0], fixed[1], fixed[2], fixed[3]);
    let baseline_r = call_rust(fixed[0], fixed[1], fixed[2], fixed[3]);
    assert_eq!(baseline_c.bits(), baseline_r.bits(), "C24 baseline");

    for _ in 0..1_000 {
        // an unrelated (often NaN/inf-producing) call in between
        let noise = [rng.any_vec(), rng.any_vec(), rng.any_vec(), rng.any_vec()];
        assert_same("C24[noise]", noise);

        let c = call_c(fixed[0], fixed[1], fixed[2], fixed[3]);
        let r = call_rust(fixed[0], fixed[1], fixed[2], fixed[3]);
        assert_eq!(c.bits(), baseline_c.bits(), "C24: C is not stateless");
        assert_eq!(r.bits(), baseline_r.bits(), "C24: Rust is not stateless");
    }
}
