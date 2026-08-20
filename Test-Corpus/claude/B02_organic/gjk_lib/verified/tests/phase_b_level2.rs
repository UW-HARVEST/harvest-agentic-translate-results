//! Phase B — CONFIGS.md rows 23-35: the simplex primitives.
//!
//! These are the LOWEST-level entry points of the GJK pipeline and they mutate
//! a `c2Simplex` in place, so every test compares the ENTIRE 152-byte simplex
//! byte-for-byte after the call (not just the return value): a wrong field copy
//! inside `c22`/`c23` is only visible that way.

#![allow(non_snake_case)]

#[macro_use]
mod common;

use common::*;

fn p_of(s: &Simplex, i: usize) -> V {
    s.verts[i].p
}

fn same_p(a: V, b: V) -> bool {
    a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits()
}

/// Rows 23/24/25 — `c2GJKSimplexMetric` for count 1, 2, 3.
#[test]
fn row23_25_simplex_metric() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexF>("c2GJKSimplexMetric");
    for &count in &[1i32, 2, 3] {
        let mut g = Rng::new(0x2300 + count as u64);
        for i in 0..20_000 {
            let vg: fn(&mut Rng) -> V = if i % 4 == 0 { Rng::v_mixed } else { Rng::v_coord };
            let mut cs = g.simplex(count, vg);
            // collinear / duplicated variants
            match i % 7 {
                1 => cs.verts[2].p = cs.verts[1].p,
                2 => cs.verts[1].p = cs.verts[0].p,
                3 => {
                    cs.verts[1].p = cs.verts[0].p;
                    cs.verts[2].p = cs.verts[0].p;
                }
                4 => {
                    // exactly collinear: c = a + 2*(b-a)
                    let a = cs.verts[0].p;
                    let b = cs.verts[1].p;
                    cs.verts[2].p = V::new(a.x + 2.0 * (b.x - a.x), a.y + 2.0 * (b.y - a.y));
                }
                _ => {}
            }
            let mut rs = cs;
            let cv = unsafe { c(&mut cs) };
            let rv = unsafe { r(&mut rs) };
            ck_f32!("c2GJKSimplexMetric", cv, rv, "count={count} i={i} s={cs:?}");
            ck_bytes!("c2GJKSimplexMetric simplex untouched", cs, rs, "count={count} i={i}");
        }
    }
}

/// Row 26 — `c22` over randomised 2-simplices, whole-struct compare + arm coverage.
#[test]
fn row26_c22_random() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexVoid>("c22");
    let mut g = Rng::new(0x2601);
    let mut arms = [0usize; 3];
    for i in 0..100_000 {
        let vg: fn(&mut Rng) -> V = match i % 5 {
            0 => Rng::v_mixed,
            1 => Rng::v_grid,
            _ => Rng::v_coord,
        };
        let mut cs = g.simplex(2, vg);
        if i % 11 == 0 {
            cs.verts[1].p = cs.verts[0].p; // duplicate vertex
        }
        let orig = cs;
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        ck_bytes!("c22", cs, rs, "i={i} orig={orig:?}");

        // classify which of the three arms the C took
        let arm = if cs.count == 2 {
            2
        } else if same_p(p_of(&cs, 0), p_of(&orig, 1)) && !same_p(p_of(&orig, 0), p_of(&orig, 1)) {
            1
        } else {
            0
        };
        arms[arm] += 1;
    }
    assert!(arms.iter().all(|&n| n > 0), "c22 arm coverage incomplete: {arms:?}");
    eprintln!("c22 arm hits (v<=0, u<=0, edge) = {arms:?}");
}

/// Row 27 — `c22` with inputs hand-built to force each arm.
#[test]
fn row27_c22_forced_arms() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexVoid>("c22");

    let cases: &[(&str, V, V)] = &[
        ("v<=0 (vertex A region)", V::new(1.0, 0.0), V::new(2.0, 0.0)),
        ("u<=0 (vertex B region)", V::new(2.0, 0.0), V::new(1.0, 0.0)),
        ("edge region", V::new(1.0, -1.0), V::new(1.0, 1.0)),
        ("duplicate a==b", V::new(3.0, 4.0), V::new(3.0, 4.0)),
        ("both at origin", V::new(0.0, 0.0), V::new(0.0, 0.0)),
        ("a at origin", V::new(0.0, 0.0), V::new(1.0, 1.0)),
        ("b at origin", V::new(1.0, 1.0), V::new(0.0, 0.0)),
        ("nan a", V::new(f32::NAN, 0.0), V::new(1.0, 1.0)),
        ("nan b", V::new(1.0, 1.0), V::new(f32::NAN, 0.0)),
        ("inf", V::new(f32::INFINITY, 0.0), V::new(1.0, 1.0)),
        ("-0 vs +0", V::new(-0.0, -0.0), V::new(0.0, 0.0)),
        ("huge", V::new(1e30, 1e30), V::new(-1e30, 1e30)),
        ("tiny", V::new(1e-40, 1e-40), V::new(-1e-40, 1e-40)),
    ];

    let mut g = Rng::new(0x2701);
    for (name, a, b) in cases {
        for k in 0..200 {
            // vary the *other* fields to be sure they are copied identically
            let mut cs = g.simplex(2, Rng::v_coord);
            cs.verts[0].p = *a;
            cs.verts[1].p = *b;
            let mut rs = cs;
            unsafe {
                c(&mut cs);
                r(&mut rs);
            }
            ck_bytes!("c22 forced", cs, rs, "case={name} k={k}");
        }
    }
}

/// Row 28 — `c23` over randomised 3-simplices; all 7 arms must be hit.
#[test]
fn row28_c23_random() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexVoid>("c23");
    let mut g = Rng::new(0x2801);
    let mut arms = [0usize; 8];
    for i in 0..200_000 {
        let vg: fn(&mut Rng) -> V = match i % 6 {
            0 => Rng::v_mixed,
            1 | 2 => Rng::v_grid,
            _ => Rng::v_coord,
        };
        let mut cs = g.simplex(3, vg);
        match i % 13 {
            1 => cs.verts[1].p = cs.verts[0].p,
            2 => cs.verts[2].p = cs.verts[1].p,
            3 => {
                cs.verts[1].p = cs.verts[0].p;
                cs.verts[2].p = cs.verts[0].p;
            }
            4 => {
                // exactly collinear -> area == 0
                let a = cs.verts[0].p;
                let b = cs.verts[1].p;
                cs.verts[2].p = V::new(a.x + 2.0 * (b.x - a.x), a.y + 2.0 * (b.y - a.y));
            }
            _ => {}
        }
        let orig = cs;
        let mut rs = cs;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        ck_bytes!("c23", cs, rs, "i={i} orig={orig:?}");

        // classify (only meaningful when the three p's are distinct)
        let (o0, o1, o2) = (p_of(&orig, 0), p_of(&orig, 1), p_of(&orig, 2));
        let distinct = !same_p(o0, o1) && !same_p(o1, o2) && !same_p(o0, o2);
        if distinct {
            let n0 = p_of(&cs, 0);
            let arm = match cs.count {
                1 => {
                    if same_p(n0, o0) {
                        0
                    } else if same_p(n0, o1) {
                        1
                    } else {
                        2
                    }
                }
                2 => {
                    let n1 = p_of(&cs, 1);
                    if same_p(n0, o0) && same_p(n1, o1) {
                        3 // edge AB
                    } else if same_p(n0, o1) && same_p(n1, o2) {
                        4 // edge BC
                    } else if same_p(n0, o2) && same_p(n1, o0) {
                        5 // edge CA
                    } else {
                        7 // unclassified
                    }
                }
                3 => 6,
                _ => 7,
            };
            arms[arm] += 1;
        }
    }
    assert!(
        arms[0..7].iter().all(|&n| n > 0),
        "c23 arm coverage incomplete: {arms:?}"
    );
    eprintln!("c23 arm hits (A,B,C,AB,BC,CA,interior,other) = {arms:?}");
}

/// Row 29 — `c23` with inputs hand-built to force each of the 7 arms.
#[test]
fn row29_c23_forced_arms() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexVoid>("c23");

    // A triangle far from the origin in various orientations forces the
    // different Voronoi regions; degenerate cases are appended.
    let cases: &[(&str, V, V, V)] = &[
        ("vertex A", V::new(1.0, 0.0), V::new(3.0, 1.0), V::new(3.0, -1.0)),
        ("vertex B", V::new(3.0, 1.0), V::new(1.0, 0.0), V::new(3.0, -1.0)),
        ("vertex C", V::new(3.0, 1.0), V::new(3.0, -1.0), V::new(1.0, 0.0)),
        ("edge AB", V::new(1.0, -1.0), V::new(1.0, 1.0), V::new(3.0, 0.0)),
        ("edge BC", V::new(3.0, 0.0), V::new(1.0, -1.0), V::new(1.0, 1.0)),
        ("edge CA", V::new(1.0, 1.0), V::new(3.0, 0.0), V::new(1.0, -1.0)),
        ("interior/origin inside", V::new(-1.0, -1.0), V::new(2.0, 0.0), V::new(-1.0, 2.0)),
        ("interior ccw", V::new(-1.0, -1.0), V::new(-1.0, 2.0), V::new(2.0, 0.0)),
        ("all identical", V::new(2.0, 2.0), V::new(2.0, 2.0), V::new(2.0, 2.0)),
        ("all at origin", V::new(0.0, 0.0), V::new(0.0, 0.0), V::new(0.0, 0.0)),
        ("collinear", V::new(1.0, 1.0), V::new(2.0, 2.0), V::new(3.0, 3.0)),
        ("collinear through origin", V::new(-1.0, -1.0), V::new(0.0, 0.0), V::new(1.0, 1.0)),
        ("two identical", V::new(1.0, 0.0), V::new(1.0, 0.0), V::new(2.0, 2.0)),
        ("nan vertex", V::new(f32::NAN, 0.0), V::new(1.0, 1.0), V::new(2.0, 2.0)),
        ("all nan", V::new(f32::NAN, f32::NAN), V::new(f32::NAN, f32::NAN), V::new(f32::NAN, f32::NAN)),
        ("inf vertex", V::new(f32::INFINITY, 0.0), V::new(1.0, 1.0), V::new(2.0, 2.0)),
        ("huge", V::new(1e30, 0.0), V::new(0.0, 1e30), V::new(-1e30, -1e30)),
        ("tiny", V::new(1e-40, 0.0), V::new(0.0, 1e-40), V::new(-1e-40, -1e-40)),
        ("-0", V::new(-0.0, -0.0), V::new(0.0, 0.0), V::new(-0.0, 0.0)),
    ];

    let mut g = Rng::new(0x2901);
    for (name, a, b, cc) in cases {
        for k in 0..200 {
            let mut cs = g.simplex(3, Rng::v_coord);
            cs.verts[0].p = *a;
            cs.verts[1].p = *b;
            cs.verts[2].p = *cc;
            let mut rs = cs;
            unsafe {
                c(&mut cs);
                r(&mut rs);
            }
            ck_bytes!("c23 forced", cs, rs, "case={name} k={k}");
        }
    }
}

/// Row 30 — `c2D` for every simplex count and both det branches.
#[test]
fn row30_c2D() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexV>("c2D");
    let mut g = Rng::new(0x3001);
    for &count in &[0i32, 1, 2, 3, 4, -1, i32::MAX, i32::MIN] {
        for i in 0..12_500 {
            let vg: fn(&mut Rng) -> V = if i % 4 == 0 { Rng::v_mixed } else { Rng::v_coord };
            let mut cs = g.simplex(count, vg);
            if i % 9 == 0 {
                // force c2Det2(ab, -a) == 0 : origin collinear with a,b
                let a = cs.verts[0].p;
                cs.verts[1].p = V::new(a.x * 2.0, a.y * 2.0);
            }
            if i % 17 == 0 {
                cs.verts[1].p = cs.verts[0].p; // ab == 0
            }
            let mut rs = cs;
            let cv = unsafe { c(&mut cs) };
            let rv = unsafe { r(&mut rs) };
            ck_v!("c2D", cv, rv, "count={count} i={i} s={cs:?}");
            ck_bytes!("c2D simplex untouched", cs, rs, "count={count} i={i}");
        }
    }
}

/// Row 31 — `c2L` for every simplex count and every `div` class.
#[test]
fn row31_c2L() {
    let l = libs();
    let (c, r) = l.get::<FnSimplexV>("c2L");
    let mut g = Rng::new(0x3101);
    for &count in &[0i32, 1, 2, 3, 4, -1, i32::MAX, i32::MIN] {
        for i in 0..12_500 {
            let vg: fn(&mut Rng) -> V = if i % 4 == 0 { Rng::v_mixed } else { Rng::v_coord };
            let mut cs = g.simplex(count, vg);
            cs.div = match i % 6 {
                0 => 0.0,
                1 => -0.0,
                2 => f32::NAN,
                3 => f32::INFINITY,
                4 => 1e-40,
                _ => cs.div,
            };
            let mut rs = cs;
            let cv = unsafe { c(&mut cs) };
            let rv = unsafe { r(&mut rs) };
            ck_v!("c2L", cv, rv, "count={count} i={i} div={:?} s={cs:?}", cs.div);
            ck_bytes!("c2L simplex untouched", cs, rs, "count={count} i={i}");
        }
    }
}

/// Rows 32/33/34 — `c2Witness` for count 1, 2, 3 with all `div` classes.
#[test]
fn row32_34_witness() {
    let l = libs();
    let (c, r) = l.get::<FnWitness>("c2Witness");
    for &count in &[1i32, 2, 3] {
        let mut g = Rng::new(0x3200 + count as u64);
        for i in 0..50_000 {
            let vg: fn(&mut Rng) -> V = if i % 4 == 0 { Rng::v_mixed } else { Rng::v_coord };
            let mut cs = g.simplex(count, vg);
            cs.div = match i % 8 {
                0 => 0.0,
                1 => -0.0,
                2 => f32::NAN,
                3 => f32::INFINITY,
                4 => 1e-40,
                5 => 1e30,
                _ => cs.div,
            };
            let mut rs = cs;
            let mut ca = V::new(f32::from_bits(0xA5A5_A5A5), f32::from_bits(0xA5A5_A5A5));
            let mut cb = ca;
            let mut ra = ca;
            let mut rb = ca;
            unsafe {
                c(&mut cs, &mut ca, &mut cb);
                r(&mut rs, &mut ra, &mut rb);
            }
            ck_v!("c2Witness a", ca, ra, "count={count} i={i} div={:?} s={cs:?}", cs.div);
            ck_v!("c2Witness b", cb, rb, "count={count} i={i} div={:?} s={cs:?}", cs.div);
            ck_bytes!("c2Witness simplex untouched", cs, rs, "count={count} i={i}");
        }
    }
}

/// Row 35 — the COMPOSED pipeline: random simplex -> `c22`/`c23` -> `c2D` ->
/// `c2L` -> `c2Witness`, exactly as `c2GJK` chains them. Per-function tests
/// cannot see bugs that only appear in the composition.
#[test]
fn row35_simplex_pipeline() {
    let l = libs();
    let (c22c, c22r) = l.get::<FnSimplexVoid>("c22");
    let (c23c, c23r) = l.get::<FnSimplexVoid>("c23");
    let (cdc, cdr) = l.get::<FnSimplexV>("c2D");
    let (clc, clr) = l.get::<FnSimplexV>("c2L");
    let (cwc, cwr) = l.get::<FnWitness>("c2Witness");
    let (cmc, cmr) = l.get::<FnSimplexF>("c2GJKSimplexMetric");

    let mut g = Rng::new(0x3501);
    for i in 0..100_000 {
        let count = 2 + (i % 2) as i32; // 2 or 3
        let vg: fn(&mut Rng) -> V = match i % 5 {
            0 => Rng::v_mixed,
            1 | 2 => Rng::v_grid,
            _ => Rng::v_coord,
        };
        let mut cs = g.simplex(count, vg);
        let mut rs = cs;

        unsafe {
            if count == 2 {
                c22c(&mut cs);
                c22r(&mut rs);
            } else {
                c23c(&mut cs);
                c23r(&mut rs);
            }
        }
        ck_bytes!("pipeline after reduce", cs, rs, "i={i} count={count}");

        let (cd, rd) = unsafe { (cdc(&mut cs), cdr(&mut rs)) };
        ck_v!("pipeline c2D", cd, rd, "i={i}");

        let (cl, rl) = unsafe { (clc(&mut cs), clr(&mut rs)) };
        ck_v!("pipeline c2L", cl, rl, "i={i}");

        let mut ca = V::default();
        let mut cb = V::default();
        let mut ra = V::default();
        let mut rb = V::default();
        unsafe {
            cwc(&mut cs, &mut ca, &mut cb);
            cwr(&mut rs, &mut ra, &mut rb);
        }
        ck_v!("pipeline witness a", ca, ra, "i={i}");
        ck_v!("pipeline witness b", cb, rb, "i={i}");

        let (cm, rm) = unsafe { (cmc(&mut cs), cmr(&mut rs)) };
        ck_f32!("pipeline metric", cm, rm, "i={i}");
        ck_bytes!("pipeline final simplex", cs, rs, "i={i}");
    }
}
