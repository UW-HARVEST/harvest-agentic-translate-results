//! Phase B — valid-path differential tests for `f9` (lightmapper barycentric
//! coordinates) and `f10` (half-float decode via lookup tables).
//!
//! Covers `CONFIGS.md` rows C30 … C34.

mod common;

use common::*;

const N: usize = 20_000;

fn chk_f9(p: &Pair, p1: LmVec2, p2: LmVec2, p3: LmVec2, q: LmVec2, tag: &str) {
    same(
        tag,
        (
            (p1.x.to_bits(), p1.y.to_bits()),
            (p2.x.to_bits(), p2.y.to_bits()),
            (p3.x.to_bits(), p3.y.to_bits()),
            (q.x.to_bits(), q.y.to_bits()),
        ),
        unsafe { (p.c.f9)(p1, p2, p3, q) },
        unsafe { (p.rs.f9)(p1, p2, p3, q) },
    );
}

fn v(x: f32, y: f32) -> LmVec2 {
    LmVec2 { x, y }
}

// ---------------------------------------------------------------------------
// C30 — non-degenerate triangles, point inside / on vertex / on edge / outside
// ---------------------------------------------------------------------------

#[test]
fn c30_f9_nondegenerate() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x30);

    let tri = (v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0));
    // vertices, edge midpoints, centroid, outside
    let probes = [
        v(0.0, 0.0),
        v(1.0, 0.0),
        v(0.0, 1.0),
        v(0.5, 0.0),
        v(0.0, 0.5),
        v(0.5, 0.5),
        v(1.0 / 3.0, 1.0 / 3.0),
        v(2.0, 2.0),
        v(-1.0, -1.0),
        v(0.25, 0.25),
    ];
    for q in probes {
        chk_f9(p, tri.0, tri.1, tri.2, q, "f9/unit-tri");
    }

    // randomized well-conditioned triangles + random probe points
    for _ in 0..N {
        let p1 = v(r.finite_f32(10.0), r.finite_f32(10.0));
        let p2 = v(r.finite_f32(10.0), r.finite_f32(10.0));
        let p3 = v(r.finite_f32(10.0), r.finite_f32(10.0));
        let q = v(r.finite_f32(10.0), r.finite_f32(10.0));
        chk_f9(p, p1, p2, p3, q, "f9/random-finite");
    }
    // probe exactly at each vertex of a random triangle
    for _ in 0..N / 4 {
        let p1 = v(r.finite_f32(10.0), r.finite_f32(10.0));
        let p2 = v(r.finite_f32(10.0), r.finite_f32(10.0));
        let p3 = v(r.finite_f32(10.0), r.finite_f32(10.0));
        for q in [p1, p2, p3] {
            chk_f9(p, p1, p2, p3, q, "f9/at-vertex");
        }
    }
}

// ---------------------------------------------------------------------------
// C31 — degenerate triangles: 1.0f / 0.0f is unguarded in the C
// ---------------------------------------------------------------------------

#[test]
fn c31_f9_degenerate() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x31);

    for _ in 0..N / 2 {
        let a = v(r.finite_f32(10.0), r.finite_f32(10.0));
        let b = v(r.finite_f32(10.0), r.finite_f32(10.0));
        let q = v(r.finite_f32(10.0), r.finite_f32(10.0));
        // p1 == p2  -> v1 == 0 -> dot11 == 0 -> denom == 0
        chk_f9(p, a, a, b, q, "f9/p1==p2");
        // p1 == p3  -> v0 == 0 -> dot00 == 0
        chk_f9(p, a, b, a, q, "f9/p1==p3");
        // p2 == p3
        chk_f9(p, a, b, b, q, "f9/p2==p3");
        // all equal
        chk_f9(p, a, a, a, q, "f9/all-equal");
        // collinear: p3 = p1 + 2*(p2 - p1)
        let c = v(
            a.x + 2.0 * (b.x - a.x),
            a.y + 2.0 * (b.y - a.y),
        );
        chk_f9(p, a, b, c, q, "f9/collinear");
        // collinear with a negative multiple
        let d = v(a.x - 3.0 * (b.x - a.x), a.y - 3.0 * (b.y - a.y));
        chk_f9(p, a, b, d, q, "f9/collinear-neg");
    }
    // exactly zero triangle at the origin
    chk_f9(p, v(0.0, 0.0), v(0.0, 0.0), v(0.0, 0.0), v(0.0, 0.0), "f9/zero");
    chk_f9(p, v(0.0, 0.0), v(0.0, 0.0), v(0.0, 0.0), v(1.0, 1.0), "f9/zero-probe");
    // signed-zero variants
    for &s in &[0.0f32, -0.0] {
        chk_f9(p, v(s, s), v(s, s), v(s, s), v(s, s), "f9/signed-zero");
    }
}

// ---------------------------------------------------------------------------
// C32 — extreme magnitudes
// ---------------------------------------------------------------------------

#[test]
fn c32_f9_extreme_magnitudes() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x32);

    let mags = [
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        1e-30f32,
        1e-10,
        1.0,
        1e10,
        1e30,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &m in &mags {
        chk_f9(p, v(0.0, 0.0), v(m, 0.0), v(0.0, m), v(m, m), "f9/mag-tri");
        chk_f9(p, v(m, m), v(-m, m), v(m, -m), v(0.0, 0.0), "f9/mag-tri2");
        chk_f9(p, v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(m, m), "f9/mag-probe");
    }
    // mixed magnitudes so dot products overflow and underflow
    for _ in 0..N {
        let pick = |r: &mut Rng| {
            let m = mags[(r.below(mags.len() as u32)) as usize];
            let s = if r.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
            m * s
        };
        let p1 = v(pick(&mut r), pick(&mut r));
        let p2 = v(pick(&mut r), pick(&mut r));
        let p3 = v(pick(&mut r), pick(&mut r));
        let q = v(pick(&mut r), pick(&mut r));
        chk_f9(p, p1, p2, p3, q, "f9/mixed-mag");
    }
    // full cross product of the special corpus applied uniformly
    for &bits in SPECIAL_F32 {
        let s = f32::from_bits(bits);
        chk_f9(p, v(s, s), v(s, 1.0), v(1.0, s), v(s, -s), "f9/special-uniform");
        chk_f9(p, v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(s, s), "f9/special-probe");
        chk_f9(p, v(s, 0.0), v(0.0, s), v(s, s), v(1.0, 1.0), "f9/special-tri");
    }
}

// ---------------------------------------------------------------------------
// C33 — fully random 32-bit patterns in all eight coordinates
// ---------------------------------------------------------------------------

#[test]
fn c33_f9_raw_bit_patterns() {
    let p = pair();
    let mut r = Rng::new(SEED ^ 0x33);
    for _ in 0..N {
        let p1 = r.raw_lmv();
        let p2 = r.raw_lmv();
        let p3 = r.raw_lmv();
        let q = r.raw_lmv();
        chk_f9(p, p1, p2, p3, q, "f9/raw");
    }
    // "nice" generator: mixes specials in at a high rate, so NaN-operand
    // selection inside every one of the five dot products is exercised
    for _ in 0..N {
        let p1 = r.lmv(4.0);
        let p2 = r.lmv(4.0);
        let p3 = r.lmv(4.0);
        let q = r.lmv(4.0);
        chk_f9(p, p1, p2, p3, q, "f9/nice");
    }
    // pairwise NaN sign combinations: these decide which operand `mulss` and
    // `addss` propagate
    let nans = [
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0xFFFF_FFFF),
        f32::from_bits(0x7F80_0001),
    ];
    for &a in &nans {
        for &b in &nans {
            chk_f9(p, v(a, b), v(b, a), v(a, a), v(b, b), "f9/nan-mix");
            chk_f9(p, v(a, 1.0), v(2.0, b), v(b, 3.0), v(4.0, a), "f9/nan-sparse");
            chk_f9(p, v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(a, b), "f9/nan-probe");
        }
    }
}

// ---------------------------------------------------------------------------
// C34 — f10: exhaustive over the whole uint16_t domain
// ---------------------------------------------------------------------------

#[test]
fn c34_f10_exhaustive() {
    let p = pair();
    for h in 0u16..=u16::MAX {
        same("f10/exhaustive", h, unsafe { (p.c.f10)(h) }, unsafe {
            (p.rs.f10)(h)
        });
        if h == u16::MAX {
            break;
        }
    }
    // spot-check the extreme table indices explicitly (n == 0 and n == 63,
    // low bits 0 and 0x3ff -> m__mantissa[0] and m__mantissa[2047])
    for h in [0u16, 0x03FF, 0x0400, 0xFC00, 0xFFFF, 0x7C00, 0xFC00, 0x7E00, 0xFE00] {
        same("f10/boundary", h, unsafe { (p.c.f10)(h) }, unsafe {
            (p.rs.f10)(h)
        });
    }
}
