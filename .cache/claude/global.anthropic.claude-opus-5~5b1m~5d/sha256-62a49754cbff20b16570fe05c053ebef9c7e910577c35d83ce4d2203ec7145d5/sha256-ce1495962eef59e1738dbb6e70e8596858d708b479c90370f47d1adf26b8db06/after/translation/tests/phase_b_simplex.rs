//! Phase B — valid-path differential tests, `CONFIGS.md` rows 22..=35.
//!
//! Group 3: the simplex internals, driven directly as low-level entry points
//! (`c2Support`, `c2GJKSimplexMetric`, `c22`, `c23`, `c2D`, `c2L`, `c2Witness`)
//! rather than only through `c2GJK`.

mod common;
use common::*;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// Simplex construction helpers
// ---------------------------------------------------------------------------

/// A simplex with every field randomised (including the unused `d` slot and the
/// fields the solver under test does not read), so that any stray write shows up.
fn random_simplex(rng: &mut Rng, count: c_int, scale: f32, extreme: bool) -> c2Simplex {
    let mut s = c2Simplex {
        verts: [c2sv::default(); 4],
        div: if extreme { rng.special_no_nan() } else { rng.ordinary(10.0) },
        count,
    };
    for v in s.verts.iter_mut() {
        if extreme {
            v.sA = rng.v_special_no_nan();
            v.sB = rng.v_special_no_nan();
            v.p = rng.v_special_no_nan();
            v.u = rng.special_no_nan();
        } else {
            v.sA = rng.v_ordinary(scale);
            v.sB = rng.v_ordinary(scale);
            v.p = rng.v_ordinary(scale);
            v.u = rng.ordinary(scale);
        }
        v.iA = rng.below(4) as c_int;
        v.iB = rng.below(4) as c_int;
    }
    s
}

/// Which branch of `c22` the C source takes (used only for coverage counting).
fn classify_c22(s: &c2Simplex) -> usize {
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let u = b.x * (b.x - a.x) + b.y * (b.y - a.y);
    let v = a.x * (a.x - b.x) + a.y * (a.y - b.y);
    if v <= 0.0 {
        0
    } else if u <= 0.0 {
        1
    } else {
        2
    }
}

/// Which of the seven branches of `c23` the C source takes (coverage only).
fn classify_c23(s: &c2Simplex) -> usize {
    let dot = |p: c2v, q: c2v| p.x * q.x + p.y * q.y;
    let sub = |p: c2v, q: c2v| c2v { x: p.x - q.x, y: p.y - q.y };
    let det = |p: c2v, q: c2v| p.x * q.y - p.y * q.x;
    let a = s.verts[0].p;
    let b = s.verts[1].p;
    let c = s.verts[2].p;
    let uab = dot(b, sub(b, a));
    let vab = dot(a, sub(a, b));
    let ubc = dot(c, sub(c, b));
    let vbc = dot(b, sub(b, c));
    let uca = dot(a, sub(a, c));
    let vca = dot(c, sub(c, a));
    let area = det(sub(b, a), sub(c, a));
    let uabc = det(b, c) * area;
    let vabc = det(c, a) * area;
    let wabc = det(a, b) * area;
    if vab <= 0.0 && uca <= 0.0 {
        0
    } else if uab <= 0.0 && vbc <= 0.0 {
        1
    } else if ubc <= 0.0 && vca <= 0.0 {
        2
    } else if uab > 0.0 && vab > 0.0 && wabc <= 0.0 {
        3
    } else if ubc > 0.0 && vbc > 0.0 && uabc <= 0.0 {
        4
    } else if uca > 0.0 && vca > 0.0 && vabc <= 0.0 {
        5
    } else {
        6
    }
}

// ---------------------------------------------------------------------------
// Row 22..=25 — c2Support with 1, 2, 4 and 8 vertices
// ---------------------------------------------------------------------------

fn support_axis(c: &Api, r: &Api, label: &str, row: &str, seed: u64, count: c_int, extreme: bool) {
    let mut rng = Rng::new(seed);
    let mut hit_tie = 0usize;
    for i in 0..N {
        let mut verts = [c2v::default(); 8];
        for k in 0..8 {
            verts[k] = if extreme {
                rng.v_special()
            } else {
                rng.v_ordinary(20.0)
            };
        }
        // Force exact ties in the dot products sometimes (duplicate vertices).
        if count > 1 && rng.below(4) == 0 {
            let src = rng.below(count as u32) as usize;
            let dst = rng.below(count as u32) as usize;
            verts[dst] = verts[src];
            hit_tie += 1;
        }
        let d = match rng.below(6) {
            0 => c2v { x: 1.0, y: 0.0 },
            1 => c2v { x: 0.0, y: 1.0 },
            2 => c2v { x: 0.0, y: 0.0 },
            3 if extreme => rng.v_special(),
            _ => rng.v_ordinary(5.0),
        };
        let ci = unsafe { (c.c2Support)(verts.as_ptr(), count, d) };
        let ri = unsafe { (r.c2Support)(verts.as_ptr(), count, d) };
        assert_eq!(
            ci, ri,
            "{label} {row} c2Support #{i}: count={count} d={} verts={:?} -> C {ci} vs R {ri}",
            fmt_v(d),
            &verts[..count.max(0) as usize]
        );
    }
    if count > 1 {
        assert!(hit_tie > 0, "{label} {row}: no tie cases generated");
    }
}

#[test]
fn row22_support_count1() {
    for_each_pair(|c, r, label| support_axis(c, r, label, "row22", 0x0017, 1, false));
}

#[test]
fn row23_support_count2() {
    for_each_pair(|c, r, label| support_axis(c, r, label, "row23", 0x0018, 2, false));
}

#[test]
fn row24_support_count4() {
    for_each_pair(|c, r, label| support_axis(c, r, label, "row24", 0x0019, 4, false));
}

#[test]
fn row25_support_count8_and_nan() {
    for_each_pair(|c, r, label| {
        support_axis(c, r, label, "row25a", 0x001A, 8, false);
        support_axis(c, r, label, "row25b", 0x001B, 8, true);
        support_axis(c, r, label, "row25c", 0x001C, 4, true);
    });
}

// ---------------------------------------------------------------------------
// Row 26 — c2GJKSimplexMetric for count 1, 2, 3
// ---------------------------------------------------------------------------

#[test]
fn row26_gjk_simplex_metric() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x001D);
        for count in [1, 2, 3] {
            for i in 0..N {
                let extreme = rng.below(4) == 0;
                let mut cs = random_simplex(&mut rng, count, 50.0, extreme);
                let mut rs = cs;
                let cm = unsafe { (c.c2GJKSimplexMetric)(&mut cs) };
                let rm = unsafe { (r.c2GJKSimplexMetric)(&mut rs) };
                assert!(
                    f32_same(cm, rm),
                    "{label} row26 #{i} count={count}: C {} vs R {}\n  {}",
                    fmt_f32(cm),
                    fmt_f32(rm),
                    fmt_simplex(&cs)
                );
                assert!(
                    simplex_same(&cs, &rs),
                    "{label} row26 #{i} count={count}: simplex mutated differently"
                );
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Row 27..=30 — c22
// ---------------------------------------------------------------------------

#[test]
fn row27to29_c22_targeted_branches() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x001E);
        let mut seen = [0usize; 3];
        let mut tested = 0usize;
        // Rejection-sample until every branch has plenty of samples.
        while tested < N * 3 && seen.iter().any(|&n| n < 400) {
            let mut cs = random_simplex(&mut rng, 2, 8.0, false);
            // Sometimes place p's so that the origin is clearly outside the
            // segment on one side (forces the u<=0 / v<=0 collapses).
            match rng.below(4) {
                0 => {
                    let dir = rng.v_ordinary(1.0);
                    cs.verts[0].p = c2v { x: dir.x + 3.0, y: dir.y + 3.0 };
                    cs.verts[1].p = c2v { x: dir.x + 6.0, y: dir.y + 6.0 };
                }
                1 => {
                    let dir = rng.v_ordinary(1.0);
                    cs.verts[0].p = c2v { x: dir.x - 6.0, y: dir.y - 6.0 };
                    cs.verts[1].p = c2v { x: dir.x - 3.0, y: dir.y - 3.0 };
                }
                2 => {
                    // origin between the two points -> interior branch
                    let dir = rng.v_ordinary(4.0);
                    cs.verts[0].p = dir;
                    cs.verts[1].p = c2v { x: -dir.x, y: -dir.y };
                }
                _ => {}
            }
            let br = classify_c22(&cs);
            seen[br] += 1;
            tested += 1;
            let mut rs = cs;
            unsafe {
                (c.c22)(&mut cs);
                (r.c22)(&mut rs);
            }
            assert!(
                simplex_same(&cs, &rs),
                "{label} c22 branch {br}: divergence\n  C: {}\n  R: {}",
                fmt_simplex(&cs),
                fmt_simplex(&rs)
            );
        }
        assert!(
            seen.iter().all(|&n| n >= 400),
            "{label} rows27-29: insufficient c22 branch coverage {seen:?}"
        );
        println!("{label} c22 branch coverage: {seen:?}");
    });
}

#[test]
fn row30_c22_fully_random() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x001F);
        for i in 0..N {
            let extreme = rng.below(3) == 0;
            let mut cs = random_simplex(&mut rng, 2, 1.0e3, extreme);
            let mut rs = cs;
            unsafe {
                (c.c22)(&mut cs);
                (r.c22)(&mut rs);
            }
            assert!(
                simplex_same(&cs, &rs),
                "{label} row30 c22 #{i}: divergence\n  C: {}\n  R: {}",
                fmt_simplex(&cs),
                fmt_simplex(&rs)
            );
        }
        // count values other than 2 must be handled identically too: c22 does
        // not look at s->count at all, it only writes it.
        for count in [-1, 0, 1, 3, 4, 99] {
            let mut rng2 = Rng::new(0x0020u64.wrapping_add(count as i64 as u64));
            for _ in 0..64 {
                let mut cs = random_simplex(&mut rng2, count, 10.0, false);
                let mut rs = cs;
                unsafe {
                    (c.c22)(&mut cs);
                    (r.c22)(&mut rs);
                }
                assert!(
                    simplex_same(&cs, &rs),
                    "{label} row30 c22 count={count}: divergence"
                );
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Row 31 / 32 — c23
// ---------------------------------------------------------------------------

#[test]
fn row31_c23_all_seven_branches() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0021);
        let mut seen = [0usize; 7];
        let mut tested = 0usize;
        while tested < N * 8 && seen.iter().any(|&n| n < 200) {
            let mut cs = random_simplex(&mut rng, 3, 8.0, false);
            match rng.below(6) {
                // Triangle containing the origin -> interior branch.
                0 => {
                    let t = rng.unit() * std::f32::consts::PI;
                    let rad = 1.0 + rng.unit().abs() * 4.0;
                    for k in 0..3 {
                        let ang = t + k as f32 * 2.094_395_1;
                        cs.verts[k].p = c2v {
                            x: rad * ang.cos(),
                            y: rad * ang.sin(),
                        };
                    }
                }
                // Triangle pushed far away along a random direction -> vertex /
                // edge region depending on the offset.
                1..=3 => {
                    let off = rng.v_ordinary(12.0);
                    let t = rng.unit() * std::f32::consts::PI;
                    let rad = 0.5 + rng.unit().abs() * 3.0;
                    for k in 0..3 {
                        let ang = t + k as f32 * 2.094_395_1;
                        cs.verts[k].p = c2v {
                            x: off.x + rad * ang.cos(),
                            y: off.y + rad * ang.sin(),
                        };
                    }
                }
                // Degenerate / collinear triangles (area == 0).
                4 => {
                    let a = rng.v_ordinary(6.0);
                    let b = rng.v_ordinary(6.0);
                    let t = rng.unit();
                    cs.verts[0].p = a;
                    cs.verts[1].p = b;
                    cs.verts[2].p = c2v {
                        x: a.x + (b.x - a.x) * t,
                        y: a.y + (b.y - a.y) * t,
                    };
                }
                _ => {}
            }
            let br = classify_c23(&cs);
            seen[br] += 1;
            tested += 1;
            let mut rs = cs;
            unsafe {
                (c.c23)(&mut cs);
                (r.c23)(&mut rs);
            }
            assert!(
                simplex_same(&cs, &rs),
                "{label} row31 c23 branch {br}: divergence\n  C: {}\n  R: {}",
                fmt_simplex(&cs),
                fmt_simplex(&rs)
            );
        }
        assert!(
            seen.iter().all(|&n| n >= 200),
            "{label} row31: insufficient c23 branch coverage {seen:?} after {tested} samples"
        );
        println!("{label} c23 branch coverage: {seen:?}");
    });
}

#[test]
fn row32_c23_fully_random() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0022);
        for i in 0..N {
            let extreme = rng.below(3) == 0;
            let mut cs = random_simplex(&mut rng, 3, 1.0e3, extreme);
            let mut rs = cs;
            unsafe {
                (c.c23)(&mut cs);
                (r.c23)(&mut rs);
            }
            assert!(
                simplex_same(&cs, &rs),
                "{label} row32 c23 #{i}: divergence\n  C: {}\n  R: {}",
                fmt_simplex(&cs),
                fmt_simplex(&rs)
            );
        }
        for count in [-1, 0, 1, 2, 4, 99] {
            let mut rng2 = Rng::new(0x0023u64.wrapping_add(count as i64 as u64));
            for _ in 0..64 {
                let mut cs = random_simplex(&mut rng2, count, 10.0, false);
                let mut rs = cs;
                unsafe {
                    (c.c23)(&mut cs);
                    (r.c23)(&mut rs);
                }
                assert!(
                    simplex_same(&cs, &rs),
                    "{label} row32 c23 count={count}: divergence"
                );
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Row 33 — c2D
// ---------------------------------------------------------------------------

#[test]
fn row33_c2d() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0024);
        let mut skew = 0usize;
        let mut ccw = 0usize;
        for count in [1, 2, 3] {
            for i in 0..N {
                let extreme = rng.below(4) == 0;
                let mut cs = random_simplex(&mut rng, count, 30.0, extreme);
                let mut rs = cs;
                if count == 2 {
                    let a = cs.verts[0].p;
                    let b = cs.verts[1].p;
                    let ab = c2v { x: b.x - a.x, y: b.y - a.y };
                    let det = ab.x * (-a.y) - ab.y * (-a.x);
                    if det > 0.0 {
                        skew += 1;
                    } else {
                        ccw += 1;
                    }
                }
                let cd = unsafe { (c.c2D)(&mut cs) };
                let rd = unsafe { (r.c2D)(&mut rs) };
                assert!(
                    v_same(cd, rd),
                    "{label} row33 c2D #{i} count={count}: C {} vs R {}\n  {}",
                    fmt_v(cd),
                    fmt_v(rd),
                    fmt_simplex(&cs)
                );
                assert!(
                    simplex_same(&cs, &rs),
                    "{label} row33 c2D #{i}: simplex mutated differently"
                );
            }
        }
        assert!(skew > 100 && ccw > 100, "{label} row33: c2D sign coverage skew={skew} ccw={ccw}");
        // Also the out-of-range counts.
        for count in [-1, 0, 4, 99] {
            let mut rng2 = Rng::new(0x0025u64.wrapping_add(count as i64 as u64));
            for _ in 0..64 {
                let mut cs = random_simplex(&mut rng2, count, 10.0, false);
                let mut rs = cs;
                let cd = unsafe { (c.c2D)(&mut cs) };
                let rd = unsafe { (r.c2D)(&mut rs) };
                assert!(v_same(cd, rd), "{label} row33 c2D count={count}");
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Row 34 — c2L
// ---------------------------------------------------------------------------

#[test]
fn row34_c2l() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0026);
        for count in [1, 2, 3] {
            for i in 0..N {
                let extreme = rng.below(4) == 0;
                let mut cs = random_simplex(&mut rng, count, 30.0, extreme);
                // occasionally force div to a nasty value
                cs.div = match rng.below(8) {
                    0 => 0.0,
                    1 => -0.0,
                    2 => 1.0,
                    3 => f32::MIN_POSITIVE,
                    _ => cs.div,
                };
                let mut rs = cs;
                let cd = unsafe { (c.c2L)(&mut cs) };
                let rd = unsafe { (r.c2L)(&mut rs) };
                assert!(
                    v_same(cd, rd),
                    "{label} row34 c2L #{i} count={count}: C {} vs R {}\n  {}",
                    fmt_v(cd),
                    fmt_v(rd),
                    fmt_simplex(&cs)
                );
                assert!(
                    simplex_same(&cs, &rs),
                    "{label} row34 c2L #{i}: simplex mutated differently"
                );
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Row 35 — c2Witness
// ---------------------------------------------------------------------------

#[test]
fn row35_c2witness() {
    for_each_pair(|c, r, label| {
        let mut rng = Rng::new(0x0027);
        for count in [1, 2, 3] {
            for i in 0..N {
                let extreme = rng.below(4) == 0;
                let mut cs = random_simplex(&mut rng, count, 30.0, extreme);
                cs.div = match rng.below(8) {
                    0 => 0.0,
                    1 => -0.0,
                    2 => 1.0,
                    _ => cs.div,
                };
                let mut rs = cs;
                let poison = c2v { x: -1.0e-30, y: 4.25e12 };
                let (mut ca, mut cb) = (poison, poison);
                let (mut ra, mut rb) = (poison, poison);
                unsafe {
                    (c.c2Witness)(&mut cs, &mut ca, &mut cb);
                    (r.c2Witness)(&mut rs, &mut ra, &mut rb);
                }
                assert!(
                    v_same(ca, ra) && v_same(cb, rb),
                    "{label} row35 c2Witness #{i} count={count}:\n  C a={} b={}\n  R a={} b={}\n  {}",
                    fmt_v(ca),
                    fmt_v(cb),
                    fmt_v(ra),
                    fmt_v(rb),
                    fmt_simplex(&cs)
                );
                assert!(
                    simplex_same(&cs, &rs),
                    "{label} row35 c2Witness #{i}: simplex mutated differently"
                );
            }
        }
    });
}
