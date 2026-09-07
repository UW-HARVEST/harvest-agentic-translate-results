//! Phase B — differential tests for the simplex layer, driven directly on
//! hand-built `c2Simplex` values (these are the lowest-level entry points of
//! the GJK pipeline and are never reachable through `gjk` alone).
//! CONFIGS.md rows 30–48.

mod common;
use common::*;
use std::ffi::c_int;
use std::sync::atomic::{AtomicUsize, Ordering};

const N: usize = 4000;

fn simplex_with_p(ps: &[c2v], div: f32, count: c_int, rng: &mut Rng) -> c2Simplex {
    let mut s = rand_simplex(rng, count, div, false);
    for (i, p) in ps.iter().enumerate() {
        s.verts[i].p = *p;
    }
    s
}

// ---------------------------------------------------------------------------
// rows 30-31: c2GJKSimplexMetric
// ---------------------------------------------------------------------------

fn metric_case(ctx: &str, s: c2Simplex) {
    let (c, r) = (&libs().c, &libs().r);
    let mut cs = s;
    let mut rs = s;
    let cv = unsafe { (c.c2GJKSimplexMetric)(&mut cs) };
    let rv = unsafe { (r.c2GJKSimplexMetric)(&mut rs) };
    assert_feq(&format!("{ctx} return"), cv, rv);
    assert!(
        simplexeq(&cs, &rs),
        "{ctx} simplex mutated differently:\n C={cs:?}\n R={rs:?}"
    );
}

#[test]
fn row30_metric_valid_counts() {
    let mut rng = Rng::new(SEED ^ 30);
    for count in [1i32, 2, 3] {
        for i in 0..N {
            let wild = i % 6 == 0;
            let d = rng.coord(); let s = rand_simplex(&mut rng, count, d, wild);
            metric_case(&format!("row30 metric count={count} #{i}"), s);
        }
    }
}

#[test]
fn row31_metric_out_of_range_counts() {
    let mut rng = Rng::new(SEED ^ 31);
    for count in [0i32, -1, -999, 4, 5, 99, i32::MIN, i32::MAX] {
        for i in 0..200 {
            let d = rng.coord(); let s = rand_simplex(&mut rng, count, d, i % 3 == 0);
            metric_case(&format!("row31 metric count={count} #{i}"), s);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 32-35: c22
// ---------------------------------------------------------------------------

static C22_BRANCH: [AtomicUsize; 3] = [
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
];

fn c22_case(ctx: &str, s: c2Simplex) {
    let (c, r) = (&libs().c, &libs().r);
    let mut cs = s;
    let mut rs = s;
    unsafe {
        (c.c22)(&mut cs);
        (r.c22)(&mut rs);
    }
    assert!(
        simplexeq(&cs, &rs),
        "{ctx}:\n in={s:?}\n C={cs:?}\n R={rs:?}"
    );
    // classify which of the three C branches was taken, from the C result
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let ab = c2v { x: b.x - a.x, y: b.y - a.y };
    let v = a.x * (a.x - b.x) + a.y * (a.y - b.y);
    let u = b.x * ab.x + b.y * ab.y;
    let idx = if !(v > 0.0) {
        0
    } else if !(u > 0.0) {
        1
    } else {
        2
    };
    C22_BRANCH[idx].fetch_add(1, Ordering::Relaxed);
}

#[test]
fn row32_to_row35_c22_all_branches() {
    let mut rng = Rng::new(SEED ^ 32);
    // uniformly random p pairs
    for i in 0..N {
        let d = rng.coord(); let s = rand_simplex(&mut rng, 2, d, i % 8 == 0);
        c22_case(&format!("row32-34 c22 random #{i}"), s);
    }
    // segments deliberately straddling the origin (interior branch)
    for i in 0..N {
        let dir = rng.coord_v();
        let t = rng.unit();
        let a = c2v { x: -dir.x * t, y: -dir.y * t };
        let b = c2v {
            x: dir.x * (1.0 - t),
            y: dir.y * (1.0 - t),
        };
        let s = simplex_with_p(&[a, b], rng.coord(), 2, &mut rng);
        c22_case(&format!("row34 c22 straddle #{i}"), s);
    }
    // row 35: duplicate points, point at origin, NaN
    for i in 0..N {
        let p = rng.coord_v();
        let s = simplex_with_p(&[p, p], rng.coord(), 2, &mut rng);
        c22_case(&format!("row35 c22 dup #{i}"), s);
        let z = c2v { x: 0.0, y: 0.0 };
        let s = simplex_with_p(&[z, rng.coord_v()], rng.coord(), 2, &mut rng);
        c22_case(&format!("row35 c22 a-at-origin #{i}"), s);
        let s = simplex_with_p(&[rng.coord_v(), z], rng.coord(), 2, &mut rng);
        c22_case(&format!("row35 c22 b-at-origin #{i}"), s);
        let s = simplex_with_p(&[rng.wild_v(), rng.wild_v()], rng.wild(), 2, &mut rng);
        c22_case(&format!("row35 c22 wild #{i}"), s);
    }
    // signed zeros / infinities
    for x in [0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY] {
        for y in [0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY] {
            let s = simplex_with_p(
                &[c2v { x, y }, c2v { x: y, y: x }],
                1.0,
                2,
                &mut rng,
            );
            c22_case("row35 c22 special", s);
        }
    }
    let hits: Vec<usize> = C22_BRANCH.iter().map(|a| a.load(Ordering::Relaxed)).collect();
    assert!(
        hits.iter().all(|&h| h > 0),
        "not all three c22 branches were covered: {hits:?}"
    );
}

// ---------------------------------------------------------------------------
// rows 36-39: c23
// ---------------------------------------------------------------------------

static C23_BRANCH: [AtomicUsize; 7] = [
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
    AtomicUsize::new(0),
];

fn dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}
fn sub(a: c2v, b: c2v) -> c2v {
    c2v { x: a.x - b.x, y: a.y - b.y }
}
fn det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

/// Mirror of the C branch ladder purely for coverage accounting.
fn classify_c23(s: &c2Simplex) -> usize {
    let (a, b, c) = (s.verts[0].p, s.verts[1].p, s.verts[2].p);
    let u_ab = dot(b, sub(b, a));
    let v_ab = dot(a, sub(a, b));
    let u_bc = dot(c, sub(c, b));
    let v_bc = dot(b, sub(b, c));
    let u_ca = dot(a, sub(a, c));
    let v_ca = dot(c, sub(c, a));
    let area = det2(sub(b, a), sub(c, a));
    let u_abc = det2(b, c) * area;
    let v_abc = det2(c, a) * area;
    let w_abc = det2(a, b) * area;
    if !(v_ab > 0.0) && !(u_ca > 0.0) {
        0
    } else if !(u_ab > 0.0) && !(v_bc > 0.0) {
        1
    } else if !(u_bc > 0.0) && !(v_ca > 0.0) {
        2
    } else if u_ab > 0.0 && v_ab > 0.0 && !(w_abc > 0.0) {
        3
    } else if u_bc > 0.0 && v_bc > 0.0 && !(u_abc > 0.0) {
        4
    } else if u_ca > 0.0 && v_ca > 0.0 && !(v_abc > 0.0) {
        5
    } else {
        6
    }
}

fn c23_case(ctx: &str, s: c2Simplex) {
    let (c, r) = (&libs().c, &libs().r);
    let mut cs = s;
    let mut rs = s;
    unsafe {
        (c.c23)(&mut cs);
        (r.c23)(&mut rs);
    }
    assert!(
        simplexeq(&cs, &rs),
        "{ctx}:\n in={s:?}\n C={cs:?}\n R={rs:?}"
    );
    C23_BRANCH[classify_c23(&s)].fetch_add(1, Ordering::Relaxed);
}

#[test]
fn row36_to_row39_c23_all_branches() {
    let mut rng = Rng::new(SEED ^ 36);
    // row 36: uniformly random triples
    for i in 0..N * 2 {
        let d = rng.coord(); let s = rand_simplex(&mut rng, 3, d, i % 9 == 0);
        c23_case(&format!("row36 c23 random #{i}"), s);
    }
    // row 37: triangles that contain the origin
    for i in 0..N {
        let mut ps = [c2v::default(); 3];
        for k in 0..3 {
            let ang = (k as f32) * std::f32::consts::TAU / 3.0 + rng.unit();
            let rad = 0.5 + rng.unit() * 4.0;
            ps[k] = c2v {
                x: ang.cos() * rad,
                y: ang.sin() * rad,
            };
        }
        let s = simplex_with_p(&ps, rng.coord(), 3, &mut rng);
        c23_case(&format!("row37 c23 contains-origin #{i}"), s);
        // reversed winding => negative area
        let s = simplex_with_p(&[ps[2], ps[1], ps[0]], rng.coord(), 3, &mut rng);
        c23_case(&format!("row37 c23 contains-origin rev #{i}"), s);
    }
    // row 38: collinear triples (area == 0)
    for i in 0..N {
        let o = rng.coord_v();
        let d = rng.coord_v();
        let (t0, t1, t2) = (rng.sym(5.0), rng.sym(5.0), rng.sym(5.0));
        let mk = |t: f32| c2v {
            x: o.x + d.x * t,
            y: o.y + d.y * t,
        };
        let s = simplex_with_p(&[mk(t0), mk(t1), mk(t2)], rng.coord(), 3, &mut rng);
        c23_case(&format!("row38 c23 collinear #{i}"), s);
    }
    // row 39: coincident points and points at the origin
    for i in 0..N {
        let p = rng.coord_v();
        let q = rng.coord_v();
        let z = c2v { x: 0.0, y: 0.0 };
        for (tag, ps) in [
            ("aa", [p, p, q]),
            ("bb", [q, p, p]),
            ("ac", [p, q, p]),
            ("all", [p, p, p]),
            ("z0", [z, p, q]),
            ("z1", [p, z, q]),
            ("z2", [p, q, z]),
            ("zz", [z, z, z]),
        ] {
            let s = simplex_with_p(&ps, rng.coord(), 3, &mut rng);
            c23_case(&format!("row39 c23 {tag} #{i}"), s);
        }
    }
    // wild floats
    for i in 0..N {
        let d = rng.wild(); let s = rand_simplex(&mut rng, 3, d, true);
        c23_case(&format!("row39 c23 wild #{i}"), s);
    }
    let hits: Vec<usize> = C23_BRANCH.iter().map(|a| a.load(Ordering::Relaxed)).collect();
    assert!(
        hits.iter().all(|&h| h > 0),
        "not all seven c23 branches were covered: {hits:?}"
    );
    eprintln!("c23 branch coverage: {hits:?}");
}

// ---------------------------------------------------------------------------
// rows 40-43: c2D
// ---------------------------------------------------------------------------

fn c2d_case(ctx: &str, s: c2Simplex) {
    let (c, r) = (&libs().c, &libs().r);
    let mut cs = s;
    let mut rs = s;
    let cv = unsafe { (c.c2D)(&mut cs) };
    let rv = unsafe { (r.c2D)(&mut rs) };
    assert_veq(&format!("{ctx} return (in={s:?})"), cv, rv);
    assert!(simplexeq(&cs, &rs), "{ctx} simplex mutated differently");
}

#[test]
fn row40_to_row43_c2D() {
    let mut rng = Rng::new(SEED ^ 40);
    for count in [1i32, 2, 3, 0, -1, 4, 77] {
        for i in 0..N {
            let d = rng.coord(); let s = rand_simplex(&mut rng, count, d, i % 7 == 0);
            c2d_case(&format!("row40-43 c2D count={count} #{i}"), s);
        }
    }
    // row 42: count==2 with det exactly 0 (a, b and the origin collinear)
    for i in 0..N {
        let d = rng.coord_v();
        let (t0, t1) = (rng.sym(5.0), rng.sym(5.0));
        let a = c2v { x: d.x * t0, y: d.y * t0 };
        let b = c2v { x: d.x * t1, y: d.y * t1 };
        let s = simplex_with_p(&[a, b], rng.coord(), 2, &mut rng);
        c2d_case(&format!("row42 c2D collinear-with-origin #{i}"), s);
    }
    // a.p exactly at the origin => -a.p is (±0,±0) => det == ±0
    for i in 0..N {
        let z = c2v { x: 0.0, y: 0.0 };
        let s = simplex_with_p(&[z, rng.coord_v()], rng.coord(), 2, &mut rng);
        c2d_case(&format!("row42 c2D a-at-origin #{i}"), s);
    }
}

// ---------------------------------------------------------------------------
// rows 44-47: c2Witness
// ---------------------------------------------------------------------------

fn witness_case(ctx: &str, s: c2Simplex) {
    let (c, r) = (&libs().c, &libs().r);
    let mut cs = s;
    let mut rs = s;
    let poison = c2v { x: -424242.0, y: 313131.0 };
    let (mut ca, mut cb) = (poison, poison);
    let (mut ra, mut rb) = (poison, poison);
    unsafe {
        (c.c2Witness)(&mut cs, &mut ca, &mut cb);
        (r.c2Witness)(&mut rs, &mut ra, &mut rb);
    }
    assert_veq(&format!("{ctx} outA (in={s:?})"), ca, ra);
    assert_veq(&format!("{ctx} outB (in={s:?})"), cb, rb);
    assert!(simplexeq(&cs, &rs), "{ctx} simplex mutated differently");
}

#[test]
fn row44_to_row47_c2Witness() {
    let mut rng = Rng::new(SEED ^ 44);
    let divs = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        f32::INFINITY,
        f32::NAN,
    ];
    for count in [1i32, 2, 3, 0, -1, 4, 88] {
        for i in 0..N {
            let div = if i % 4 == 0 {
                divs[i % divs.len()]
            } else {
                rng.coord()
            };
            let s = rand_simplex(&mut rng, count, div, i % 9 == 0);
            witness_case(&format!("row44-47 c2Witness count={count} #{i}"), s);
        }
        // every special div, exhaustively, for this count
        for (k, &div) in divs.iter().enumerate() {
            let s = rand_simplex(&mut rng, count, div, false);
            witness_case(
                &format!("row44-47 c2Witness count={count} div#{k}={div:?}"),
                s,
            );
        }
    }
}

/// `c2Witness` writes `*a` and then reads the simplex again for `*b`, so an
/// alias between the out-pointers and the simplex is observable.
#[test]
fn row44_c2Witness_aliasing_outputs() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 45);
    for count in [1i32, 2, 3] {
        for i in 0..500 {
            let d = rng.coord(); let s = rand_simplex(&mut rng, count, d, false);
            // a and b alias the same c2v
            let mut cs = s;
            let mut rs = s;
            let mut cv = c2v { x: 1.0, y: 2.0 };
            let mut rv = cv;
            unsafe {
                (c.c2Witness)(&mut cs, &mut cv, &mut cv);
                (r.c2Witness)(&mut rs, &mut rv, &mut rv);
            }
            assert_veq(
                &format!("row44 c2Witness alias a==b count={count} #{i}"),
                cv,
                rv,
            );
            // a and b point into the simplex itself (verts[0].sA / verts[3].sB)
            let mut cs = s;
            let mut rs = s;
            unsafe {
                let cp = std::ptr::addr_of_mut!(cs.verts[0].sA);
                let cq = std::ptr::addr_of_mut!(cs.verts[3].sB);
                (c.c2Witness)(&mut cs, cp, cq);
                let rp = std::ptr::addr_of_mut!(rs.verts[0].sA);
                let rq = std::ptr::addr_of_mut!(rs.verts[3].sB);
                (r.c2Witness)(&mut rs, rp, rq);
            }
            assert!(
                simplexeq(&cs, &rs),
                "row44 c2Witness alias-into-simplex count={count} #{i}:\n C={cs:?}\n R={rs:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// row 48: c2L
// ---------------------------------------------------------------------------

#[test]
fn row48_c2L() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 48);
    let divs = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::INFINITY,
        f32::NAN,
    ];
    for count in [1i32, 2, 3, 0, -1, 4, 55] {
        for i in 0..N {
            let div = if i % 4 == 0 {
                divs[i % divs.len()]
            } else {
                rng.coord()
            };
            let s = rand_simplex(&mut rng, count, div, i % 9 == 0);
            let mut cs = s;
            let mut rs = s;
            let cv = unsafe { (c.c2L)(&mut cs) };
            let rv = unsafe { (r.c2L)(&mut rs) };
            assert_veq(
                &format!("row48 c2L count={count} #{i} (in={s:?})"),
                cv,
                rv,
            );
            assert!(simplexeq(&cs, &rs), "row48 c2L simplex mutated differently");
        }
    }
}
