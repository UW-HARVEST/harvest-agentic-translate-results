//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every row drives BOTH `.so`s through `libloading` with many randomised
//! inputs from a fixed-seed splitmix64 PRNG and compares the returned
//! `lm_vec2`s bit-for-bit.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// B1 — canonical unit right triangle, p on a quarter-step lattice
// ---------------------------------------------------------------------------
#[test]
fn b1_unit_right_triangle_quarter_step_point() {
    let mut rng = Rng::new(0xB001_0000_0000_0001);
    let p1 = Vec2::new(0.0, 0.0);
    let p2 = Vec2::new(1.0, 0.0);
    let p3 = Vec2::new(0.0, 1.0);
    for _ in 0..20_000 {
        let p = rng.vec2(|r| r.quarter());
        diff("B1", p1, p2, p3, p);
    }
    // Hand-checked anchors: the C returns (u, v) with u along p3-p1.
    assert_eq!(
        diff_get("B1", p1, p2, p3, Vec2::new(0.25, 0.25)).bits(),
        (0x3E80_0000, 0x3E80_0000)
    );
    assert_eq!(
        diff_get("B1", p1, p2, p3, Vec2::new(2.0, 3.0)).bits(),
        (0x4040_0000, 0x4000_0000) // (3.0, 2.0) — u is the y-ish coordinate
    );
}

// ---------------------------------------------------------------------------
// B2 — dyadic coordinates: every intermediate product/sum is exact
// ---------------------------------------------------------------------------
#[test]
fn b2_dyadic_exact_arithmetic() {
    let mut rng = Rng::new(0xB002_0000_0000_0002);
    for _ in 0..50_000 {
        let p1 = rng.vec2(|r| r.dyadic());
        let p2 = rng.vec2(|r| r.dyadic());
        let p3 = rng.vec2(|r| r.dyadic());
        let p = rng.vec2(|r| r.dyadic());
        diff("B2", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B3 — full-mantissa normals, moderate exponents
// ---------------------------------------------------------------------------
#[test]
fn b3_random_normals_moderate_exponents() {
    let mut rng = Rng::new(0xB003_0000_0000_0003);
    for _ in 0..200_000 {
        let p1 = rng.vec2(|r| r.normal_in(-20, 20));
        let p2 = rng.vec2(|r| r.normal_in(-20, 20));
        let p3 = rng.vec2(|r| r.normal_in(-20, 20));
        let p = rng.vec2(|r| r.normal_in(-20, 20));
        diff("B3", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B4 — p constructed strictly inside the triangle
// ---------------------------------------------------------------------------
#[test]
fn b4_point_inside_by_construction() {
    let mut rng = Rng::new(0xB004_0000_0000_0004);
    for _ in 0..50_000 {
        let p1 = rng.vec2(|r| r.normal_in(-6, 6));
        let p2 = rng.vec2(|r| r.normal_in(-6, 6));
        let p3 = rng.vec2(|r| r.normal_in(-6, 6));
        // Random barycentric weights with u + v <= 1.
        let mut u = rng.unit();
        let mut v = rng.unit();
        if u + v > 1.0 {
            u = 1.0 - u;
            v = 1.0 - v;
        }
        // p = p1 + v*(p2-p1) + u*(p3-p1)
        let p = Vec2::new(
            p1.x + v * (p2.x - p1.x) + u * (p3.x - p1.x),
            p1.y + v * (p2.y - p1.y) + u * (p3.y - p1.y),
        );
        diff("B4", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B5 — p exactly on a vertex
// ---------------------------------------------------------------------------
#[test]
fn b5_point_on_vertex() {
    let mut rng = Rng::new(0xB005_0000_0000_0005);
    for i in 0..30_000 {
        let p1 = rng.vec2(|r| r.normal_in(-10, 10));
        let p2 = rng.vec2(|r| r.normal_in(-10, 10));
        let p3 = rng.vec2(|r| r.normal_in(-10, 10));
        let p = match i % 3 {
            0 => p1,
            1 => p2,
            _ => p3,
        };
        diff("B5", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B6 — p exactly on an edge
// ---------------------------------------------------------------------------
#[test]
fn b6_point_on_edge() {
    let mut rng = Rng::new(0xB006_0000_0000_0006);
    for i in 0..30_000 {
        let p1 = rng.vec2(|r| r.dyadic());
        let p2 = rng.vec2(|r| r.dyadic());
        let p3 = rng.vec2(|r| r.dyadic());
        let t = rng.unit();
        let lerp = |a: Vec2, b: Vec2| Vec2::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y));
        let p = match i % 3 {
            0 => lerp(p1, p2),
            1 => lerp(p2, p3),
            _ => lerp(p3, p1),
        };
        diff("B6", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B7 — p2 == p3  =>  v0 == v1  =>  determinant cancels to zero
// ---------------------------------------------------------------------------
#[test]
fn b7_p2_equals_p3_determinant_cancels() {
    let mut rng = Rng::new(0xB007_0000_0000_0007);
    for _ in 0..20_000 {
        let p1 = rng.vec2(|r| r.normal_in(-10, 10));
        let q = rng.vec2(|r| r.normal_in(-10, 10));
        let p = rng.vec2(|r| r.normal_in(-10, 10));
        diff("B7", p1, q, q, p);
    }
}

// ---------------------------------------------------------------------------
// B8 — orthogonal edges: dot01 is (signed) zero
// ---------------------------------------------------------------------------
#[test]
fn b8_orthogonal_edges_zero_cross_term() {
    let mut rng = Rng::new(0xB008_0000_0000_0008);
    for _ in 0..20_000 {
        let p1 = rng.vec2(|r| r.dyadic());
        let a = rng.normal_in(-8, 8);
        let b = rng.normal_in(-8, 8);
        // v1 = (a, 0), v0 = (0, b)
        let p2 = Vec2::new(p1.x + a, p1.y);
        let p3 = Vec2::new(p1.x, p1.y + b);
        let p = rng.vec2(|r| r.dyadic());
        diff("B8", p1, p2, p3, p);
        // and the mirrored assignment
        diff("B8", p1, p3, p2, p);
    }
}

// ---------------------------------------------------------------------------
// B9 — needle triangles: catastrophic cancellation in the Gram determinant
// ---------------------------------------------------------------------------
#[test]
fn b9_needle_triangle_cancellation() {
    let mut rng = Rng::new(0xB009_0000_0000_0009);
    for _ in 0..50_000 {
        let p1 = rng.vec2(|r| r.normal_in(-4, 4));
        let dx = rng.normal_in(-2, 2);
        let dy = rng.normal_in(-2, 2);
        let len1 = rng.pow2(-6, 6).abs();
        let len2 = rng.pow2(-6, 6).abs();
        // v0 nearly parallel to v1, with a tiny perpendicular perturbation.
        let eps = rng.pow2(-24, -8);
        let p2 = Vec2::new(p1.x + dx * len1, p1.y + dy * len1);
        let p3 = Vec2::new(
            p1.x + (dx + eps * dy) * len2,
            p1.y + (dy - eps * dx) * len2,
        );
        let p = rng.vec2(|r| r.normal_in(-4, 4));
        diff("B9", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B10 — huge magnitudes: overflow in lm_sub2 / lm_dot2
// ---------------------------------------------------------------------------
#[test]
fn b10_huge_magnitudes_overflow() {
    let mut rng = Rng::new(0xB010_0000_0000_0010);
    for _ in 0..50_000 {
        let p1 = rng.vec2(|r| r.normal_in(60, 127));
        let p2 = rng.vec2(|r| r.normal_in(60, 127));
        let p3 = rng.vec2(|r| r.normal_in(60, 127));
        let p = rng.vec2(|r| r.normal_in(60, 127));
        diff("B10", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B11 — tiny magnitudes: subnormals, squares flushing to zero
// ---------------------------------------------------------------------------
#[test]
fn b11_tiny_magnitudes_underflow() {
    let mut rng = Rng::new(0xB011_0000_0000_0011);
    for i in 0..50_000 {
        let gen = |r: &mut Rng| {
            if i % 3 == 0 {
                r.subnormal()
            } else {
                r.normal_in(-126, -70)
            }
        };
        let p1 = rng.vec2(gen);
        let p2 = rng.vec2(gen);
        let p3 = rng.vec2(gen);
        let p = rng.vec2(gen);
        diff("B11", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B12 — mixed magnitude classes, independently per float
// ---------------------------------------------------------------------------
#[test]
fn b12_mixed_magnitude_classes() {
    let mut rng = Rng::new(0xB012_0000_0000_0012);
    fn mixed(r: &mut Rng) -> f32 {
        match r.below(5) {
            0 => r.subnormal(),
            1 => r.normal_in(-120, -60),
            2 => r.normal_in(-3, 3),
            3 => r.normal_in(60, 100),
            _ => r.normal_in(100, 127),
        }
    }
    for _ in 0..100_000 {
        let p1 = rng.vec2(mixed);
        let p2 = rng.vec2(mixed);
        let p3 = rng.vec2(mixed);
        let p = rng.vec2(mixed);
        diff("B12", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B13 — all 256 signed-zero combinations (exhaustive)
// ---------------------------------------------------------------------------
#[test]
fn b13_signed_zeros_exhaustive() {
    for mask in 0u32..256 {
        let z = |bit: u32| {
            if mask & (1 << bit) == 0 {
                0.0f32
            } else {
                -0.0f32
            }
        };
        let p1 = Vec2::new(z(0), z(1));
        let p2 = Vec2::new(z(2), z(3));
        let p3 = Vec2::new(z(4), z(5));
        let p = Vec2::new(z(6), z(7));
        diff("B13", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B14 — cross product of the 24-entry special-value table
// ---------------------------------------------------------------------------
#[test]
fn b14_special_value_cross_product() {
    let mut rng = Rng::new(0xB014_0000_0000_0014);
    for _ in 0..200_000 {
        let p1 = rng.vec2(|r| r.special());
        let p2 = rng.vec2(|r| r.special());
        let p3 = rng.vec2(|r| r.special());
        let p = rng.vec2(|r| r.special());
        diff("B14", p1, p2, p3, p);
    }
    // Plus a deterministic sweep: one special in each slot, 1.0 elsewhere.
    for slot in 0..8usize {
        for &bits in SPECIALS.iter() {
            let mut f = [1.0f32; 8];
            f[slot] = f32::from_bits(bits);
            diff(
                "B14-sweep",
                Vec2::new(f[0], f[1]),
                Vec2::new(f[2], f[3]),
                Vec2::new(f[4], f[5]),
                Vec2::new(f[6], f[7]),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// B15 — fully random 32-bit patterns
// ---------------------------------------------------------------------------
#[test]
fn b15_fully_random_bit_patterns() {
    let mut rng = Rng::new(0xB015_0000_0000_0015);
    for _ in 0..300_000 {
        let p1 = rng.vec2(|r| r.any_bits());
        let p2 = rng.vec2(|r| r.any_bits());
        let p3 = rng.vec2(|r| r.any_bits());
        let p = rng.vec2(|r| r.any_bits());
        diff("B15", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B16 — NaN-heavy (quiet), 50 % per float
// ---------------------------------------------------------------------------
#[test]
fn b16_qnan_heavy() {
    let mut rng = Rng::new(0xB016_0000_0000_0016);
    for _ in 0..300_000 {
        let gen = |r: &mut Rng| {
            if r.chance(50) {
                r.qnan()
            } else {
                r.normal_in(-10, 10)
            }
        };
        let p1 = rng.vec2(gen);
        let p2 = rng.vec2(gen);
        let p3 = rng.vec2(gen);
        let p = rng.vec2(gen);
        diff("B16", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B17 — signalling-NaN-heavy: exercises SNaN -> QNaN quieting at every op
// ---------------------------------------------------------------------------
#[test]
fn b17_snan_heavy() {
    let mut rng = Rng::new(0xB017_0000_0000_0017);
    for _ in 0..300_000 {
        let gen = |r: &mut Rng| {
            if r.chance(50) {
                r.snan()
            } else {
                r.normal_in(-10, 10)
            }
        };
        let p1 = rng.vec2(gen);
        let p2 = rng.vec2(gen);
        let p3 = rng.vec2(gen);
        let p = rng.vec2(gen);
        diff("B17", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B18 — exactly one NaN, position swept: isolates the winning destination
//       operand at each SSE site
// ---------------------------------------------------------------------------
#[test]
fn b18_single_nan_position_sweep() {
    let mut rng = Rng::new(0xB018_0000_0000_0018);
    for slot in 0..8usize {
        for i in 0..20_000 {
            let mut f = [0.0f32; 8];
            for k in 0..8 {
                f[k] = rng.normal_in(-10, 10);
            }
            f[slot] = if i % 2 == 0 { rng.qnan() } else { rng.snan() };
            diff(
                "B18",
                Vec2::new(f[0], f[1]),
                Vec2::new(f[2], f[3]),
                Vec2::new(f[4], f[5]),
                Vec2::new(f[6], f[7]),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// B19 — inf-heavy: inf-inf, 0*inf, inf+(-inf), inf/inf all reachable
// ---------------------------------------------------------------------------
#[test]
fn b19_inf_heavy() {
    let mut rng = Rng::new(0xB019_0000_0000_0019);
    for _ in 0..200_000 {
        let gen = |r: &mut Rng| match r.below(10) {
            0..=3 => r.inf(),
            4 => {
                if r.chance(50) {
                    0.0
                } else {
                    -0.0
                }
            }
            _ => r.normal_in(-10, 10),
        };
        let p1 = rng.vec2(gen);
        let p2 = rng.vec2(gen);
        let p3 = rng.vec2(gen);
        let p = rng.vec2(gen);
        diff("B19", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B20 — all 24 permutations of the four arguments (ABI slot shuffle)
// ---------------------------------------------------------------------------
fn permutations4() -> Vec<[usize; 4]> {
    let mut out = Vec::with_capacity(24);
    for a in 0..4 {
        for b in 0..4 {
            if b == a {
                continue;
            }
            for c in 0..4 {
                if c == a || c == b {
                    continue;
                }
                for d in 0..4 {
                    if d == a || d == b || d == c {
                        continue;
                    }
                    out.push([a, b, c, d]);
                }
            }
        }
    }
    out
}

#[test]
fn b20_argument_slot_permutations() {
    let perms = permutations4();
    assert_eq!(perms.len(), 24);
    let mut rng = Rng::new(0xB020_0000_0000_0020);
    for perm in &perms {
        for _ in 0..5_000 {
            let pts = [
                rng.vec2(|r| r.normal_in(-10, 10)),
                rng.vec2(|r| r.normal_in(-10, 10)),
                rng.vec2(|r| r.normal_in(-10, 10)),
                rng.vec2(|r| r.normal_in(-10, 10)),
            ];
            diff(
                "B20",
                pts[perm[0]],
                pts[perm[1]],
                pts[perm[2]],
                pts[perm[3]],
            );
        }
    }
}

// ---------------------------------------------------------------------------
// B21 — repeat-call determinism / no hidden state, interleaved C/Rust
// ---------------------------------------------------------------------------
#[test]
fn b21_no_hidden_state_interleaved_calls() {
    let l = libs();
    let mut rng = Rng::new(0xB021_0000_0000_0021);
    for _ in 0..20_000 {
        let p1 = rng.vec2(|r| r.any_bits());
        let p2 = rng.vec2(|r| r.any_bits());
        let p3 = rng.vec2(|r| r.any_bits());
        let p = rng.vec2(|r| r.any_bits());

        let c1 = unsafe { (l.c)(p1, p2, p3, p) };
        let r1 = unsafe { (l.rust)(p1, p2, p3, p) };
        let c2 = unsafe { (l.c)(p1, p2, p3, p) };
        let r2 = unsafe { (l.rust)(p1, p2, p3, p) };

        assert_eq!(c1.bits(), c2.bits(), "C is not deterministic?!");
        assert_eq!(
            r1.bits(),
            r2.bits(),
            "Rust is not deterministic: hidden state in the translation"
        );
        assert_eq!(
            c1.bits(),
            r1.bits(),
            "[B21] divergence p1={p1:?} p2={p2:?} p3={p3:?} p={p:?} C={c1:?} Rust={r1:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// B22 — exact integer lattice
// ---------------------------------------------------------------------------
#[test]
fn b22_integer_lattice() {
    let mut rng = Rng::new(0xB022_0000_0000_0022);
    for _ in 0..100_000 {
        let p1 = rng.vec2(|r| r.lattice_int());
        let p2 = rng.vec2(|r| r.lattice_int());
        let p3 = rng.vec2(|r| r.lattice_int());
        let p = rng.vec2(|r| r.lattice_int());
        diff("B22", p1, p2, p3, p);
    }
    // Exhaustive small lattice for the triangle, random p: 9^6 is too much, so
    // sweep the two edge vectors over a 5x5x5x5 grid with p1 fixed at origin.
    let mut rng2 = Rng::new(0xB022_FFFF);
    for ax in -2..=2 {
        for ay in -2..=2 {
            for bx in -2..=2 {
                for by in -2..=2 {
                    let p2 = Vec2::new(ax as f32, ay as f32);
                    let p3 = Vec2::new(bx as f32, by as f32);
                    for _ in 0..8 {
                        let p = rng2.vec2(|r| r.small_int());
                        diff("B22-grid", P_ZERO, p2, p3, p);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// B23 — powers of two with maximal exponent spread inside one dot product
// ---------------------------------------------------------------------------
#[test]
fn b23_power_of_two_exponent_spread() {
    let mut rng = Rng::new(0xB023_0000_0000_0023);
    for _ in 0..100_000 {
        let p1 = rng.vec2(|r| r.pow2(-60, 60));
        let p2 = rng.vec2(|r| r.pow2(-60, 60));
        let p3 = rng.vec2(|r| r.pow2(-60, 60));
        let p = rng.vec2(|r| r.pow2(-60, 60));
        diff("B23", p1, p2, p3, p);
    }
    // Extreme spread: near the exponent limits.
    for _ in 0..50_000 {
        let p1 = rng.vec2(|r| r.pow2(-126, 127));
        let p2 = rng.vec2(|r| r.pow2(-126, 127));
        let p3 = rng.vec2(|r| r.pow2(-126, 127));
        let p = rng.vec2(|r| r.pow2(-126, 127));
        diff("B23-extreme", p1, p2, p3, p);
    }
}

// ---------------------------------------------------------------------------
// B24 — degenerate-family sweep (valid inputs that hit the unguarded divide)
// ---------------------------------------------------------------------------
#[test]
fn b24_degenerate_family_sweep() {
    let mut rng = Rng::new(0xB024_0000_0000_0024);

    // (a) all three triangle vertices coincident
    for _ in 0..20_000 {
        let q = rng.vec2(|r| r.normal_in(-10, 10));
        let p = rng.vec2(|r| r.normal_in(-10, 10));
        diff("B24a", q, q, q, p);
    }
    // (b) p2 == p1  (v1 == 0)
    for _ in 0..20_000 {
        let p1 = rng.vec2(|r| r.normal_in(-10, 10));
        let p3 = rng.vec2(|r| r.normal_in(-10, 10));
        let p = rng.vec2(|r| r.normal_in(-10, 10));
        diff("B24b", p1, p1, p3, p);
    }
    // (c) p3 == p1  (v0 == 0)
    for _ in 0..20_000 {
        let p1 = rng.vec2(|r| r.normal_in(-10, 10));
        let p2 = rng.vec2(|r| r.normal_in(-10, 10));
        let p = rng.vec2(|r| r.normal_in(-10, 10));
        diff("B24c", p1, p2, p1, p);
    }
    // (d) exactly collinear: p3 - p1 = t * (p2 - p1) with t a power of two so
    //     the multiply is exact and the determinant is exactly zero.
    for _ in 0..20_000 {
        let p1 = rng.vec2(|r| r.dyadic());
        let d = rng.vec2(|r| r.dyadic());
        let t = rng.pow2(-8, 8);
        let p2 = Vec2::new(p1.x + d.x, p1.y + d.y);
        let p3 = Vec2::new(p1.x + d.x * t, p1.y + d.y * t);
        let p = rng.vec2(|r| r.dyadic());
        diff("B24d", p1, p2, p3, p);
    }
}
