//! Phase B — rows C25..C28: `f9` (barycentric coords) and `f10` (half->float).

mod harness;

use harness::*;

fn chk_f9(p: &Pair, p1: LmVec2, p2: LmVec2, p3: LmVec2, q: LmVec2) {
    let (c, r) = unsafe { ((p.c.f9)(p1, p2, p3, q), (p.r.f9)(p1, p2, p3, q)) };
    assert_eq!(
        (c.x.to_bits(), c.y.to_bits()),
        (r.x.to_bits(), r.y.to_bits()),
        "f9 diverged for p1={:?} p2={:?} p3={:?} p={:?}\n  C   =({:#010x},{:#010x})\n  Rust=({:#010x},{:#010x})",
        p1,
        p2,
        p3,
        q,
        c.x.to_bits(),
        c.y.to_bits(),
        r.x.to_bits(),
        r.y.to_bits()
    );
}

fn v(x: f32, y: f32) -> LmVec2 {
    LmVec2 { x, y }
}

// ------------------------------------------------------------------------ C25
#[test]
fn c25_f9_non_degenerate_triangles() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..100_000 {
        let p1 = v(rng.tame_f32(20.0), rng.tame_f32(20.0));
        let p2 = v(rng.tame_f32(20.0), rng.tame_f32(20.0));
        let p3 = v(rng.tame_f32(20.0), rng.tame_f32(20.0));
        // Probe: random, at a vertex, at a centroid, and far outside.
        chk_f9(&p, p1, p2, p3, v(rng.tame_f32(20.0), rng.tame_f32(20.0)));
        chk_f9(&p, p1, p2, p3, p1);
        chk_f9(&p, p1, p2, p3, p2);
        chk_f9(&p, p1, p2, p3, p3);
        chk_f9(
            &p,
            p1,
            p2,
            p3,
            v(
                (p1.x + p2.x + p3.x) / 3.0,
                (p1.y + p2.y + p3.y) / 3.0,
            ),
        );
        chk_f9(&p, p1, p2, p3, v(1e6, -1e6));
    }
    // Unit reference triangle with a dense probe grid.
    let (a, b, c) = (v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0));
    for i in -10i32..=20 {
        for j in -10i32..=20 {
            chk_f9(&p, a, b, c, v(i as f32 * 0.1, j as f32 * 0.1));
        }
    }
}

// ------------------------------------------------------------------------ C26
#[test]
fn c26_f9_degenerate_triangles() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..20_000 {
        let a = v(rng.tame_f32(10.0), rng.tame_f32(10.0));
        let q = v(rng.tame_f32(10.0), rng.tame_f32(10.0));
        let d = v(rng.tame_f32(10.0), rng.tame_f32(10.0));
        // all three coincident -> denom == 0 -> invDenom == +/-inf
        chk_f9(&p, a, a, a, q);
        // p2 == p1  (v1 == 0)
        chk_f9(&p, a, a, d, q);
        // p3 == p1  (v0 == 0)
        chk_f9(&p, a, d, a, q);
        // collinear: p3 = p1 + 2*(p2 - p1)
        let col = v(a.x + 2.0 * (d.x - a.x), a.y + 2.0 * (d.y - a.y));
        chk_f9(&p, a, d, col, q);
        // probe coincident with p1 on a degenerate triangle
        chk_f9(&p, a, a, a, a);
    }
    // Exact zeros and signed zeros.
    let z = v(0.0, 0.0);
    let nz = v(-0.0, -0.0);
    for &p1 in &[z, nz] {
        for &p2 in &[z, nz] {
            for &p3 in &[z, nz] {
                for &q in &[z, nz, v(1.0, 1.0)] {
                    chk_f9(&p, p1, p2, p3, q);
                }
            }
        }
    }
}

// ------------------------------------------------------------------------ C27
#[test]
fn c27_f9_special_floats() {
    let p = load();
    let mut rng = Rng::new(SEED ^ 27);
    let sp = all_special_f32();

    // Full-bit-space random: NaN payloads, infinities and subnormals appear
    // naturally and in every argument position.
    for _ in 0..200_000 {
        chk_f9(
            &p,
            v(rng.any_f32(), rng.any_f32()),
            v(rng.any_f32(), rng.any_f32()),
            v(rng.any_f32(), rng.any_f32()),
            v(rng.any_f32(), rng.any_f32()),
        );
    }
    // One special injected into each of the 8 scalar slots at a time, over a
    // tame base triangle.
    for &s in &sp {
        for slot in 0..8 {
            let mut c = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.25, 0.25];
            c[slot] = s;
            chk_f9(
                &p,
                v(c[0], c[1]),
                v(c[2], c[3]),
                v(c[4], c[5]),
                v(c[6], c[7]),
            );
        }
    }
    // Two specials at a time (all pairs of slots, all pairs of values).
    for &s1 in &sp {
        for &s2 in &sp {
            for slot1 in 0..8 {
                let slot2 = (slot1 + 3) % 8;
                let mut c = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 0.25, 0.25];
                c[slot1] = s1;
                c[slot2] = s2;
                chk_f9(
                    &p,
                    v(c[0], c[1]),
                    v(c[2], c[3]),
                    v(c[4], c[5]),
                    v(c[6], c[7]),
                );
            }
        }
    }
    // All eight slots drawn from the special set (sampled).
    for _ in 0..200_000 {
        let mut c = [0.0f32; 8];
        for k in 0..8 {
            c[k] = rng.pick(&sp);
        }
        chk_f9(
            &p,
            v(c[0], c[1]),
            v(c[2], c[3]),
            v(c[4], c[5]),
            v(c[6], c[7]),
        );
    }
}

// ------------------------------------------------------------------------ C28
#[test]
fn c28_f10_exhaustive_all_65536_inputs() {
    let p = load();
    for h in 0u16..=u16::MAX {
        let (c, r) = unsafe { ((p.c.f10)(h), (p.r.f10)(h)) };
        assert_eq!(
            c.to_bits(),
            r.to_bits(),
            "f10({h:#06x}) diverged: C={:#010x} Rust={:#010x}",
            c.to_bits(),
            r.to_bits()
        );
    }
}
