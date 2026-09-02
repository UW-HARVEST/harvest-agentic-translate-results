//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH shared objects through `libloading` and calls the
//! exported `to_barycentric` symbol on each, comparing results bit-for-bit
//! across many randomized inputs from a fixed seed.

mod common;

use common::*;

/// Per-row sample count. Every call is two `dlsym`-resolved leaf calls, so this
/// stays far below the time budget while giving property-style coverage.
const N: u32 = 20_000;

/// C1 — non-degenerate triangle, small integer coordinates (exact binary32).
#[test]
fn c01_integer_coords_exact() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 1);
    let mut diff = Diff::new(&d, "C1 integer coords");
    let mut done = 0;
    while done < N {
        let p1 = rng.vec2_small_int(16);
        let p2 = rng.vec2_small_int(16);
        let p3 = rng.vec2_small_int(16);
        // keep it non-degenerate: reject zero-area triples
        let e0 = sub2(p3, p1);
        let e1 = sub2(p2, p1);
        if e0.x * e1.y - e0.y * e1.x == 0.0 {
            continue;
        }
        let p = rng.vec2_small_int(24);
        diff.check(p1, p2, p3, p);
        done += 1;
    }
    diff.finish();
}

/// C2 — non-degenerate triangle, random normals, `p` interior.
#[test]
fn c02_random_triangle_interior_point() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 2);
    let mut diff = Diff::new(&d, "C2 random interior");
    for _ in 0..N {
        let p1 = rng.vec2_unit();
        let p2 = rng.vec2_unit();
        let p3 = rng.vec2_unit();
        // random convex combination -> interior / on boundary
        let (mut a, mut b) = (rng.unit(), rng.unit());
        if a + b > 1.0 {
            a = 1.0 - a;
            b = 1.0 - b;
        }
        let c = 1.0 - a - b;
        let p = Vec2::new(
            p1.x * a + p2.x * b + p3.x * c,
            p1.y * a + p2.y * b + p3.y * c,
        );
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// C3 — `p` exactly at each vertex.
#[test]
fn c03_point_at_vertices() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 3);
    let mut diff = Diff::new(&d, "C3 p at vertex");
    for _ in 0..N {
        let p1 = rng.vec2_unit();
        let p2 = rng.vec2_unit();
        let p3 = rng.vec2_unit();
        diff.check(p1, p2, p3, p1);
        diff.check(p1, p2, p3, p2);
        diff.check(p1, p2, p3, p3);
    }
    diff.finish();
}

/// C4 — `p` on an edge.
#[test]
fn c04_point_on_edge() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 4);
    let mut diff = Diff::new(&d, "C4 p on edge");
    for _ in 0..N {
        let p1 = rng.vec2_unit();
        let p2 = rng.vec2_unit();
        let p3 = rng.vec2_unit();
        let t = rng.unit();
        let (a, b) = match rng.below(3) {
            0 => (p1, p2),
            1 => (p2, p3),
            _ => (p3, p1),
        };
        let p = add2(scale2(a, 1.0 - t), scale2(b, t));
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// C5 — `p` far outside the triangle (extrapolated barycentric coords).
#[test]
fn c05_point_outside() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 5);
    let mut diff = Diff::new(&d, "C5 p outside");
    for _ in 0..N {
        let p1 = rng.vec2_unit();
        let p2 = rng.vec2_unit();
        let p3 = rng.vec2_unit();
        let a = rng.range(-8.0, 8.0);
        let b = rng.range(-8.0, 8.0);
        let c = 1.0 - a - b;
        let p = Vec2::new(
            p1.x * a + p2.x * b + p3.x * c,
            p1.y * a + p2.y * b + p3.y * c,
        );
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// C6 — right angle at `p1` so that `dot01 == 0`.
#[test]
fn c06_right_angle_at_p1() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 6);
    let mut diff = Diff::new(&d, "C6 right angle");
    for _ in 0..N {
        let p1 = rng.vec2_unit();
        let leg = rng.vec2_unit();
        let k = rng.range(-4.0, 4.0);
        // v0 = leg, v1 = k * perp(leg)  => dot(v0, v1) == 0 exactly
        let p3 = add2(p1, leg);
        let p2 = add2(p1, scale2(perp2(leg), k));
        let p = rng.vec2_unit();
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// C7 — mirrored / negative-winding triangle.
#[test]
fn c07_negative_winding() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 7);
    let mut diff = Diff::new(&d, "C7 negative winding");
    for _ in 0..N {
        let p1 = rng.vec2_unit();
        let p2 = rng.vec2_unit();
        let p3 = rng.vec2_unit();
        let p = rng.vec2_unit();
        // swap p2/p3 to reverse the winding, and also mirror in x
        diff.check(p1, p3, p2, p);
        let m = |v: Vec2| Vec2::new(-v.x, v.y);
        diff.check(m(p1), m(p3), m(p2), m(p));
    }
    diff.finish();
}

/// C8 — collinear, non-coincident vertices: `denom == 0`, numerators non-zero.
#[test]
fn c08_collinear_vertices() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 8);
    let mut diff = Diff::new(&d, "C8 collinear");
    for _ in 0..N {
        let p1 = rng.vec2_unit();
        let dir = rng.vec2_unit();
        // v0 = dir * s, v1 = dir * t  -> exactly parallel
        let s = rng.range(-4.0, 4.0);
        let t = rng.range(-4.0, 4.0);
        let p3 = add2(p1, scale2(dir, s));
        let p2 = add2(p1, scale2(dir, t));
        let p = rng.vec2_unit();
        diff.check(p1, p2, p3, p);
        // integer variant, where the cancellation in denom is exact
        let p1i = rng.vec2_small_int(8);
        let diri = rng.vec2_small_int(4);
        let si = rng.small_int(4);
        let ti = rng.small_int(4);
        diff.check(
            p1i,
            add2(p1i, scale2(diri, ti)),
            add2(p1i, scale2(diri, si)),
            rng.vec2_small_int(8),
        );
    }
    diff.finish();
}

/// C9 — coincident vertices (four sub-cases).
#[test]
fn c09_coincident_vertices() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 9);
    let mut diff = Diff::new(&d, "C9 coincident vertices");
    for _ in 0..N {
        let a = rng.vec2_unit();
        let b = rng.vec2_unit();
        let p = rng.vec2_unit();
        diff.check(a, a, b, p); // p2 == p1
        diff.check(a, b, a, p); // p3 == p1
        diff.check(a, b, b, p); // p3 == p2
        diff.check(a, a, a, p); // all three equal
    }
    diff.finish();
}

/// C10 — tiny magnitudes: dots underflow toward zero / subnormals.
#[test]
fn c10_tiny_magnitudes() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 10);
    let mut diff = Diff::new(&d, "C10 tiny magnitudes");
    let scales = [1e-20f32, 1e-22, 1e-25, 1e-28, 1e-30, 1e-35, 1e-38];
    for _ in 0..N {
        let s = scales[rng.below(scales.len() as u32) as usize];
        let p1 = scale2(rng.vec2_unit(), s);
        let p2 = scale2(rng.vec2_unit(), s);
        let p3 = scale2(rng.vec2_unit(), s);
        let p = scale2(rng.vec2_unit(), s);
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// C11 — huge magnitudes: dots overflow to `+inf`, `invDenom -> 0`.
#[test]
fn c11_huge_magnitudes() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 11);
    let mut diff = Diff::new(&d, "C11 huge magnitudes");
    let scales = [1e15f32, 1e18, 1e20, 1e25, 1e30, 1e35, 3.0e38];
    for _ in 0..N {
        let s = scales[rng.below(scales.len() as u32) as usize];
        let p1 = scale2(rng.vec2_unit(), s);
        let p2 = scale2(rng.vec2_unit(), s);
        let p3 = scale2(rng.vec2_unit(), s);
        let p = scale2(rng.vec2_unit(), s);
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// C12 — mixed magnitude classes in one call (catastrophic cancellation).
#[test]
fn c12_mixed_magnitudes() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 12);
    let mut diff = Diff::new(&d, "C12 mixed magnitudes");
    let scales = [1e-30f32, 1e-15, 1e-5, 1.0, 1e5, 1e15, 1e30];
    for _ in 0..N {
        let mut c = [0f32; 8];
        for slot in c.iter_mut() {
            let s = scales[rng.below(scales.len() as u32) as usize];
            *slot = rng.signed_unit() * s;
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// C13 — sign patterns, including `±0.0`.
#[test]
fn c13_sign_patterns_and_zeros() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 13);
    let mut diff = Diff::new(&d, "C13 sign patterns");
    for _ in 0..N {
        // all-positive / all-negative / mixed
        let mag = |r: &mut Rng| r.range(0.001, 4.0);
        let mut pos = [0f32; 8];
        for s in pos.iter_mut() {
            *s = mag(&mut rng);
        }
        let (a1, a2, a3, a) = from_components(pos);
        diff.check(a1, a2, a3, a);
        let neg = pos.map(|v| -v);
        let (b1, b2, b3, b) = from_components(neg);
        diff.check(b1, b2, b3, b);

        // random ±0.0 in random slots
        let mut c = [0f32; 8];
        for s in c.iter_mut() {
            *s = rng.signed_unit();
        }
        let zeros = 1 + rng.below(8);
        for _ in 0..zeros {
            let i = rng.below(8) as usize;
            c[i] = if rng.bool() { 0.0 } else { -0.0 };
        }
        let (d1, d2, d3, dp) = from_components(c);
        diff.check(d1, d2, d3, dp);
    }
    // the all-zero corner explicitly
    let z = Vec2::new(0.0, 0.0);
    let nz = Vec2::new(-0.0, -0.0);
    for a in [z, nz] {
        for b in [z, nz] {
            for cc in [z, nz] {
                for dd in [z, nz] {
                    diff.check(a, b, cc, dd);
                }
            }
        }
    }
    diff.finish();
}

/// C14 — subnormal components.
#[test]
fn c14_subnormals() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 14);
    let mut diff = Diff::new(&d, "C14 subnormals");
    for _ in 0..N {
        let mut c = [0f32; 8];
        for s in c.iter_mut() {
            *s = rng.subnormal();
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);

        // subnormals spliced into otherwise normal input
        let mut m = [0f32; 8];
        for s in m.iter_mut() {
            *s = rng.signed_unit();
        }
        let k = 1 + rng.below(8);
        for _ in 0..k {
            m[rng.below(8) as usize] = rng.subnormal();
        }
        let (q1, q2, q3, q) = from_components(m);
        diff.check(q1, q2, q3, q);
    }
    // the extreme subnormals
    let tiny = f32::from_bits(1);
    let big_sub = f32::from_bits(0x007F_FFFF);
    diff.check(
        Vec2::new(tiny, -tiny),
        Vec2::new(big_sub, tiny),
        Vec2::new(-big_sub, big_sub),
        Vec2::new(tiny, big_sub),
    );
    diff.finish();
}

/// C15 — `±inf` components in random positions.
#[test]
fn c15_infinities() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 15);
    let mut diff = Diff::new(&d, "C15 infinities");
    for _ in 0..N {
        let mut c = [0f32; 8];
        for s in c.iter_mut() {
            *s = rng.signed_unit();
        }
        let k = 1 + rng.below(8);
        for _ in 0..k {
            c[rng.below(8) as usize] = rng.signed_inf();
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);
    }
    // all-inf sign sweep
    for m in 0u32..256 {
        let mut c = [0f32; 8];
        for (i, s) in c.iter_mut().enumerate() {
            *s = if m >> i & 1 == 1 {
                f32::INFINITY
            } else {
                f32::NEG_INFINITY
            };
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// C16 — quiet NaNs with distinct payloads (SSE destination-operand selection).
#[test]
fn c16_quiet_nans_distinct_payloads() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 16);
    let mut diff = Diff::new(&d, "C16 quiet NaNs");
    for _ in 0..N {
        let mut c = [0f32; 8];
        for s in c.iter_mut() {
            *s = rng.signed_unit();
        }
        let k = 1 + rng.below(8);
        for _ in 0..k {
            c[rng.below(8) as usize] = rng.quiet_nan();
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);
    }
    // every slot NaN, each with a unique payload, so that any two-NaN operand
    // pair inside the dot products is distinguishable
    for base in 0..64u32 {
        let mut c = [0f32; 8];
        for (i, s) in c.iter_mut().enumerate() {
            let payload = 1 + base * 8 + i as u32;
            *s = f32::from_bits(0x7FC0_0000 | payload);
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);
    }
    // negative-sign NaNs too
    for base in 0..64u32 {
        let mut c = [0f32; 8];
        for (i, s) in c.iter_mut().enumerate() {
            let payload = 1 + base * 8 + i as u32;
            *s = f32::from_bits(0xFFC0_0000 | payload);
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// C17 — signalling NaNs (quieted by the first arithmetic operation).
#[test]
fn c17_signalling_nans() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 17);
    let mut diff = Diff::new(&d, "C17 signalling NaNs");
    for _ in 0..N {
        let mut c = [0f32; 8];
        for s in c.iter_mut() {
            *s = rng.signed_unit();
        }
        let k = 1 + rng.below(8);
        for _ in 0..k {
            c[rng.below(8) as usize] = rng.signalling_nan();
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);
    }
    for base in 0..64u32 {
        let mut c = [0f32; 8];
        for (i, s) in c.iter_mut().enumerate() {
            let payload = 1 + base * 8 + i as u32;
            *s = f32::from_bits(0x7F80_0000 | payload);
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// C18 — `±FLT_MAX` / `±FLT_MIN` boundary components.
#[test]
fn c18_float_boundaries() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 18);
    let mut diff = Diff::new(&d, "C18 float boundaries");
    let specials = [
        f32::MAX,
        -f32::MAX,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1.0,
        -1.0,
        f32::EPSILON,
        -f32::EPSILON,
    ];
    for _ in 0..N {
        let mut c = [0f32; 8];
        for s in c.iter_mut() {
            *s = specials[rng.below(specials.len() as u32) as usize];
        }
        let (p1, p2, p3, p) = from_components(c);
        diff.check(p1, p2, p3, p);

        let mut m = [0f32; 8];
        for s in m.iter_mut() {
            *s = rng.signed_unit();
        }
        let k = 1 + rng.below(8);
        for _ in 0..k {
            m[rng.below(8) as usize] = specials[rng.below(specials.len() as u32) as usize];
        }
        let (q1, q2, q3, q) = from_components(m);
        diff.check(q1, q2, q3, q);
    }
    diff.finish();
}

/// C19 — random normals spanning the entire binary32 normal exponent range.
#[test]
fn c19_wide_exponent_normals() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 19);
    let mut diff = Diff::new(&d, "C19 wide normals");
    for _ in 0..N {
        let p1 = rng.vec2_wide();
        let p2 = rng.vec2_wide();
        let p3 = rng.vec2_wide();
        let p = rng.vec2_wide();
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// C20 — unrestricted bit-pattern fuzz over all eight floats.
#[test]
fn c20_unrestricted_bitpattern_fuzz() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 20);
    let mut diff = Diff::new(&d, "C20 unrestricted fuzz");
    for _ in 0..(N * 10) {
        let p1 = rng.vec2_any();
        let p2 = rng.vec2_any();
        let p3 = rng.vec2_any();
        let p = rng.vec2_any();
        diff.check(p1, p2, p3, p);
    }
    diff.finish();
}

/// C21 — argument aliasing: the same value in several argument slots.
#[test]
fn c21_argument_aliasing() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 21);
    let mut diff = Diff::new(&d, "C21 aliasing");
    for _ in 0..N {
        let a = rng.vec2_unit();
        let b = rng.vec2_unit();
        diff.check(a, a, a, a);
        diff.check(a, a, a, b);
        diff.check(a, b, a, a);
        diff.check(b, a, a, a);
        diff.check(a, a, b, b);
        diff.check(a, b, b, a);
        // same but over arbitrary bit patterns
        let w = rng.vec2_any();
        diff.check(w, w, w, w);
        diff.check(w, w, w, b);
        diff.check(w, b, w, w);
    }
    diff.finish();
}

/// C22 — purity / statelessness: replayed inputs must stay identical.
#[test]
fn c22_purity_replay() {
    let d = Dual::load();
    let mut rng = Rng::new(SEED ^ 22);
    let mut diff = Diff::new(&d, "C22 purity");
    for _ in 0..(N / 64).max(1) {
        let p1 = rng.vec2_any();
        let p2 = rng.vec2_any();
        let p3 = rng.vec2_any();
        let p = rng.vec2_any();
        let first_c = d.call_c(p1, p2, p3, p);
        let first_r = d.call_rust(p1, p2, p3, p);
        assert_eq!(first_c.bits(), first_r.bits(), "C22 initial call diverged");
        for _ in 0..64 {
            // interleave unrelated calls to shake out hidden state
            let n1 = rng.vec2_wide();
            diff.check(n1, p2, p3, p);
            let again_c = d.call_c(p1, p2, p3, p);
            let again_r = d.call_rust(p1, p2, p3, p);
            assert_eq!(
                again_c.bits(),
                first_c.bits(),
                "C is not pure across calls"
            );
            assert_eq!(
                again_r.bits(),
                first_r.bits(),
                "Rust is not pure across calls"
            );
            diff.check(p1, p2, p3, p);
        }
    }
    diff.finish();
}
