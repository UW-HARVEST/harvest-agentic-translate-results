//! Phase B, CONFIGS.md rows 30-38: the simplex machinery (`c2GJKSimplexMetric`,
//! `c22`, `c23`, `c2D`, `c2Witness`, `c2L`).
//!
//! `c22` / `c23` mutate the simplex in place, so each side gets its own copy and the
//! whole 152-byte struct is compared afterwards -- that covers `count`, `div`, every
//! `u`, and the vertex shuffling, not just the "interesting" field.
//!
//! Branch coverage is *asserted*, not hoped for: each test counts how often each of
//! the C function's branches was taken and fails if any is never exercised.
#![allow(non_snake_case)]
#![allow(clippy::unnecessary_cast, clippy::needless_range_loop, clippy::let_and_return)]
#![allow(clippy::field_reassign_with_default)]

mod common;
use common::*;

const N: usize = 6_000;

/// Build a random simplex. `count` is set explicitly; all four `c2sv` slots are
/// filled with random data so that reads past `count` (if any) are still identical.
fn rand_simplex(rng: &mut Rng, count: i32, mag: f32, special: bool) -> c2Simplex {
    let mut s = c2Simplex::default();
    let mk = |rng: &mut Rng| c2sv {
        sA: if special { rng.vec_special() } else { rng.vec_norm(mag) },
        sB: if special { rng.vec_special() } else { rng.vec_norm(mag) },
        p: if special { rng.vec_special() } else { rng.vec_norm(mag) },
        u: if special { rng.f_special() } else { rng.f_norm(mag) },
        iA: (rng.below(4)) as i32,
        iB: (rng.below(4)) as i32,
    };
    s.a = mk(rng);
    s.b = mk(rng);
    s.c = mk(rng);
    s.d = mk(rng);
    s.div = if special { rng.f_special() } else { rng.f_norm(mag) };
    s.count = count;
    s
}

// ---------------------------------------------------------------------------
// Row 30: c2GJKSimplexMetric
// ---------------------------------------------------------------------------

#[test]
fn row30_simplex_metric() {
    let l = libs();
    let (cf, rf) = l.get::<FnSimplexF>("c2GJKSimplexMetric");
    let mut rng = Rng::new(30);
    for &count in [0i32, 1, 2, 3, 4, -1, 7, i32::MIN, i32::MAX].iter() {
        for i in 0..N {
            let s = rand_simplex(&mut rng, count, 100.0, i % 4 == 0);
            let (mut cs, mut rs) = (s, s);
            let (c, r) = unsafe { (cf(&mut cs), rf(&mut rs)) };
            let ctx = format!("count={count} s={s:?}");
            eq_f32("c2GJKSimplexMetric", &ctx, c, r);
            eq("c2GJKSimplexMetric (simplex must not change)", &ctx, &cs, &rs);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 31-32: c22
// ---------------------------------------------------------------------------

#[test]
fn row31_32_c22() {
    let l = libs();
    let (cf, rf) = l.get::<FnSimplexVoid>("c22");
    let mut rng = Rng::new(31);
    // branch tally: [vertex-A (v<=0), vertex-B (u<=0), edge]
    let mut hits = [0u32; 3];
    for i in 0..N * 4 {
        let mut s = rand_simplex(&mut rng, 2, 100.0, i % 7 == 0);
        // Steer the geometry so all three branches are reached:
        match i % 5 {
            0 => {
                // a == b -> u == v == 0 -> vertex-A branch
                s.b.p = s.a.p;
            }
            1 => {
                // origin beyond a: a.p and b.p on the same ray, |a| < |b|
                let dir = rng.vec_norm(1.0);
                s.a.p = v(dir.x, dir.y);
                s.b.p = v(dir.x * 3.0, dir.y * 3.0);
            }
            2 => {
                // origin beyond b
                let dir = rng.vec_norm(1.0);
                s.a.p = v(dir.x * 3.0, dir.y * 3.0);
                s.b.p = v(dir.x, dir.y);
            }
            3 => {
                // origin between: a and b on opposite sides
                let dir = rng.vec_norm(3.0);
                s.a.p = dir;
                s.b.p = v(-dir.x, -dir.y);
            }
            _ => {}
        }
        let (mut cs, mut rs) = (s, s);
        unsafe {
            cf(&mut cs);
            rf(&mut rs);
        }
        eq("c22", &format!("in={s:?}"), &cs, &rs);
        match (cs.count, cs.a.p == s.a.p) {
            (1, true) => hits[0] += 1,
            (1, false) => hits[1] += 1,
            _ => hits[2] += 1,
        }
    }
    println!("c22 branch hits: vertexA(or A-collapse)={} vertexB={} edge={}", hits[0], hits[1], hits[2]);
    assert!(hits[0] > 0 && hits[1] > 0 && hits[2] > 0, "c22 branches not all covered: {hits:?}");
}

// ---------------------------------------------------------------------------
// Rows 33-34: c23
// ---------------------------------------------------------------------------

#[test]
fn row33_34_c23() {
    let l = libs();
    let (cf, rf) = l.get::<FnSimplexVoid>("c23");
    let mut rng = Rng::new(33);
    // tally by resulting count: 1 (vertex region), 2 (edge region), 3 (interior)
    let mut by_count = [0u32; 4];
    for i in 0..N * 8 {
        let mut s = rand_simplex(&mut rng, 3, 100.0, i % 11 == 0);
        match i % 8 {
            0 => {
                // triangle containing the origin -> interior branch
                let r = rng.f_pos(20.0) + 1.0;
                let ph = rng.f_pos(std::f32::consts::TAU);
                for (k, sv) in [&mut s.a, &mut s.b, &mut s.c].into_iter().enumerate() {
                    let t = ph + k as f32 * std::f32::consts::TAU / 3.0;
                    sv.p = v(r * t.cos(), r * t.sin());
                }
            }
            1 => {
                // triangle far from the origin -> a vertex region
                let off = v(rng.f_norm(50.0) + 60.0, rng.f_norm(50.0) + 60.0);
                for (k, sv) in [&mut s.a, &mut s.b, &mut s.c].into_iter().enumerate() {
                    let t = k as f32 * 2.0;
                    sv.p = v(off.x + t.cos() * 3.0, off.y + t.sin() * 3.0);
                }
            }
            2 => {
                // collinear -> area == 0
                let (o, d) = (rng.vec_norm(10.0), rng.vec_norm(5.0));
                s.a.p = o;
                s.b.p = v(o.x + d.x, o.y + d.y);
                s.c.p = v(o.x + 2.0 * d.x, o.y + 2.0 * d.y);
            }
            3 => {
                // duplicated vertices
                s.b.p = s.a.p;
            }
            4 => {
                s.c.p = s.b.p;
            }
            5 => {
                // one edge straddling the origin -> edge region
                let d = rng.vec_norm(10.0);
                s.a.p = d;
                s.b.p = v(-d.x, -d.y);
                s.c.p = v(d.y * 5.0, -d.x * 5.0);
            }
            6 => {
                // integer lattice: exact zeros in the barycentric terms
                s.a.p = rng.vec_lattice(3);
                s.b.p = rng.vec_lattice(3);
                s.c.p = rng.vec_lattice(3);
            }
            _ => {}
        }
        let (mut cs, mut rs) = (s, s);
        unsafe {
            cf(&mut cs);
            rf(&mut rs);
        }
        eq("c23", &format!("in={s:?}"), &cs, &rs);
        by_count[cs.count.clamp(0, 3) as usize] += 1;
    }
    println!("c23 result counts: {by_count:?}");
    assert!(by_count[1] > 0, "c23 never collapsed to a vertex");
    assert!(by_count[2] > 0, "c23 never collapsed to an edge");
    assert!(by_count[3] > 0, "c23 never kept the interior");
}

/// Directly target each of `c23`'s seven branches with hand-built simplices, so the
/// per-branch `div` operand order is checked individually rather than statistically.
#[test]
fn row33_c23_all_seven_branches() {
    let l = libs();
    let (cf, rf) = l.get::<FnSimplexVoid>("c23");
    let mut rng = Rng::new(3333);
    // (name, a, b, c) chosen so that the origin sits in the named Voronoi region
    // of triangle (a, b, c).
    let cases: [(&str, c2v, c2v, c2v); 7] = [
        ("vertex A", v(1.0, 0.0), v(5.0, 1.0), v(5.0, -1.0)),
        ("vertex B", v(5.0, 1.0), v(1.0, 0.0), v(5.0, -1.0)),
        ("vertex C", v(5.0, 1.0), v(5.0, -1.0), v(1.0, 0.0)),
        ("edge AB", v(1.0, 2.0), v(1.0, -2.0), v(6.0, 0.0)),
        ("edge BC", v(6.0, 0.0), v(1.0, 2.0), v(1.0, -2.0)),
        ("edge CA", v(1.0, -2.0), v(6.0, 0.0), v(1.0, 2.0)),
        ("interior", v(-2.0, -2.0), v(3.0, -1.0), v(0.0, 4.0)),
    ];
    for (name, pa, pb, pc) in cases {
        for trial in 0..2_000 {
            let mut s = rand_simplex(&mut rng, 3, 10.0, false);
            // scale/rotate the whole configuration but keep the region
            let k = 1.0 + rng.f_pos(20.0);
            let th = rng.f_pos(std::f32::consts::TAU);
            let (co, si) = (th.cos(), th.sin());
            let rot = |p: c2v| v(k * (co * p.x - si * p.y), k * (si * p.x + co * p.y));
            s.a.p = rot(pa);
            s.b.p = rot(pb);
            s.c.p = rot(pc);
            let (mut cs, mut rs) = (s, s);
            unsafe {
                cf(&mut cs);
                rf(&mut rs);
            }
            eq("c23 branch", &format!("{name} trial={trial} in={s:?}"), &cs, &rs);
        }
        // check the un-rotated case lands in the region we intended
        let mut s = rand_simplex(&mut rng, 3, 10.0, false);
        s.a.p = pa;
        s.b.p = pb;
        s.c.p = pc;
        let (mut cs, mut rs) = (s, s);
        unsafe {
            cf(&mut cs);
            rf(&mut rs);
        }
        eq("c23 branch base", name, &cs, &rs);
        let want = if name.starts_with("vertex") {
            1
        } else if name.starts_with("edge") {
            2
        } else {
            3
        };
        assert_eq!(cs.count, want, "case `{name}` did not land in its intended region");
    }
}

// ---------------------------------------------------------------------------
// Row 35: c2D
// ---------------------------------------------------------------------------

#[test]
fn row35_c2D() {
    let l = libs();
    let (cf, rf) = l.get::<FnSimplexV>("c2D");
    let mut rng = Rng::new(35);
    let mut det_pos = 0u32;
    let mut det_nonpos = 0u32;
    for &count in [0i32, 1, 2, 3, 4, -5].iter() {
        for i in 0..N {
            let mut s = rand_simplex(&mut rng, count, 100.0, i % 6 == 0);
            if count == 2 {
                // force both signs of c2Det2(ab, -a.p)
                let d = rng.vec_norm(10.0);
                s.a.p = d;
                s.b.p = if i % 2 == 0 {
                    v(d.x + d.y, d.y - d.x)
                } else {
                    v(d.x - d.y, d.y + d.x)
                };
            }
            let (mut cs, mut rs) = (s, s);
            let (c, r) = unsafe { (cf(&mut cs), rf(&mut rs)) };
            let ctx = format!("count={count} s={s:?}");
            eq("c2D", &ctx, &c, &r);
            eq("c2D (simplex must not change)", &ctx, &cs, &rs);
            if count == 2 {
                let ab = v(s.b.p.x - s.a.p.x, s.b.p.y - s.a.p.y);
                let det = ab.x * -s.a.p.y - ab.y * -s.a.p.x;
                if det > 0.0 {
                    det_pos += 1;
                } else {
                    det_nonpos += 1;
                }
            }
        }
    }
    println!("c2D count==2: det>0 hits={det_pos}, det<=0 hits={det_nonpos}");
    assert!(det_pos > 0 && det_nonpos > 0, "c2D count==2 did not cover both det signs");
}

// ---------------------------------------------------------------------------
// Rows 36-37: c2Witness
// ---------------------------------------------------------------------------

#[test]
fn row36_37_c2Witness() {
    let l = libs();
    let (cf, rf) = l.get::<FnWitness>("c2Witness");
    let mut rng = Rng::new(36);
    for &count in [0i32, 1, 2, 3, 4, -2, 99].iter() {
        for i in 0..N {
            let mut s = rand_simplex(&mut rng, count, 100.0, i % 5 == 0);
            // exercise the div variants explicitly
            s.div = match i % 6 {
                0 => 0.0,
                1 => -0.0,
                2 => f32::from_bits(1), // denormal
                3 => f32::NAN,
                4 => f32::INFINITY,
                _ => s.div,
            };
            let (mut cs, mut rs) = (s, s);
            let (mut ca, mut cb) = (poison_v(1), poison_v(2));
            let (mut ra, mut rb) = (poison_v(1), poison_v(2));
            unsafe {
                cf(&mut cs, &mut ca, &mut cb);
                rf(&mut rs, &mut ra, &mut rb);
            }
            let ctx = format!("count={count} div=0x{:08x} s={s:?}", s.div.to_bits());
            eq("c2Witness outA", &ctx, &ca, &ra);
            eq("c2Witness outB", &ctx, &cb, &rb);
            eq("c2Witness (simplex must not change)", &ctx, &cs, &rs);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 38: c2L
// ---------------------------------------------------------------------------

#[test]
fn row38_c2L() {
    let l = libs();
    let (cf, rf) = l.get::<FnSimplexV>("c2L");
    let mut rng = Rng::new(38);
    for &count in [0i32, 1, 2, 3, 4, -7].iter() {
        for i in 0..N {
            let mut s = rand_simplex(&mut rng, count, 100.0, i % 5 == 0);
            s.div = match i % 7 {
                0 => 0.0,
                1 => -0.0,
                2 => f32::from_bits(1),
                3 => f32::NAN,
                4 => f32::INFINITY,
                5 => 1.0,
                _ => s.div,
            };
            let (mut cs, mut rs) = (s, s);
            let (c, r) = unsafe { (cf(&mut cs), rf(&mut rs)) };
            let ctx = format!("count={count} div=0x{:08x} s={s:?}", s.div.to_bits());
            eq("c2L", &ctx, &c, &r);
            eq("c2L (simplex must not change)", &ctx, &cs, &rs);
        }
    }
}
