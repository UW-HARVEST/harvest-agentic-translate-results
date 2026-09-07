//! Phase B, level 2 — proxy / support / simplex primitives.
//! CONFIGS.md rows 15..=28.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const N: usize = 3000;

/// Row 15 — `c2BBVerts` for every AABB shape, all 4 output verts compared.
/// Also ERRORS.md row 42 (exactly 4 elements written, never more).
#[test]
fn row15_c2BBVerts() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnBBVerts>("c2BBVerts") };
    let mut rng = Rng::new(0x2001);
    let mut d = Diff::new("15: c2BBVerts");
    let sentinel = C2v { x: 1234.5, y: -6789.0 };
    for i in 0..N {
        let mut bb = if i < 64 {
            C2AABB {
                min: C2v { x: SPECIALS[i % SPECIALS.len()], y: SPECIALS[(i / 3) % SPECIALS.len()] },
                max: C2v { x: SPECIALS[(i / 5) % SPECIALS.len()], y: SPECIALS[(i / 7) % SPECIALS.len()] },
            }
        } else {
            aabb(&mut rng, 100.0)
        };
        // 6-slot buffer: slots 4 and 5 must stay untouched in both.
        let mut co = [sentinel; 6];
        let mut ro = [sentinel; 6];
        let mut bb2 = bb;
        unsafe {
            c(co.as_mut_ptr(), &mut bb);
            r(ro.as_mut_ptr(), &mut bb2);
        }
        d.check(bb, co, ro);
        assert!(co[4].beq(&sentinel) && co[5].beq(&sentinel), "C wrote past 4 verts");
        assert!(ro[4].beq(&sentinel) && ro[5].beq(&sentinel), "Rust wrote past 4 verts");
        // the input AABB must not be mutated
        d.check("bb unchanged", bb, bb2);
    }
    d.finish();
}

fn proxy_row(row: &str, ty: c_int, seed: u64) {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnMakeProxy>("c2MakeProxy") };
    let mut rng = Rng::new(seed);
    let mut d = Diff::new(row);
    for _ in 0..N {
        let bytes = shape_bytes(&mut rng, ty, 100.0);
        // Pre-fill both proxies identically with garbage so that any field the
        // C leaves untouched is detectable.
        let fill = C2Proxy {
            radius: -777.25,
            count: -99,
            verts: [C2v { x: 3.5, y: -4.5 }; 8],
        };
        let mut cp = fill;
        let mut rp = fill;
        unsafe {
            c(bytes.as_ptr() as *const c_void, ty, &mut cp);
            r(bytes.as_ptr() as *const c_void, ty, &mut rp);
        }
        d.check((ty_name(ty), &bytes[..]), cp, rp);
    }
    d.finish();
}

#[test]
fn row16_makeproxy_circle() {
    proxy_row("16: c2MakeProxy CIRCLE", C2_TYPE_CIRCLE, 0x2002);
}

#[test]
fn row17_makeproxy_aabb() {
    proxy_row("17: c2MakeProxy AABB", C2_TYPE_AABB, 0x2003);
}

#[test]
fn row18_makeproxy_capsule() {
    proxy_row("18: c2MakeProxy CAPSULE", C2_TYPE_CAPSULE, 0x2004);
}

fn support_row(row: &str, count: c_int, seed: u64, ties: bool) {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnSupport>("c2Support") };
    let mut rng = Rng::new(seed);
    let mut d = Diff::new(row);
    for _ in 0..N * 2 {
        let mut verts = [C2v { x: 0.0, y: 0.0 }; 8];
        for i in 0..count.max(1) as usize {
            verts[i] = if ties && rng.below(3) == 0 {
                // exact-tie generator: reuse an earlier vertex, or use a grid
                if i > 0 && rng.boolean() {
                    verts[rng.below(i)]
                } else {
                    C2v { x: rng.grid(), y: rng.grid() }
                }
            } else {
                vec_mixed(&mut rng, 100.0)
            };
        }
        let dir = if ties && rng.below(4) == 0 {
            // axis-aligned directions maximise ties on AABB vertices
            [
                C2v { x: 1.0, y: 0.0 },
                C2v { x: -1.0, y: 0.0 },
                C2v { x: 0.0, y: 1.0 },
                C2v { x: 0.0, y: -1.0 },
                C2v { x: 0.0, y: 0.0 },
            ][rng.below(5)]
        } else {
            vec_mixed(&mut rng, 100.0)
        };
        unsafe {
            d.check(
                (count, verts, dir),
                c(verts.as_ptr(), count, dir),
                r(verts.as_ptr(), count, dir),
            );
        }
    }
    d.finish();
}

#[test]
fn row19_support_count1() {
    support_row("19: c2Support count=1", 1, 0x2005, false);
}

#[test]
fn row20_support_count2() {
    support_row("20: c2Support count=2", 2, 0x2006, true);
}

#[test]
fn row21_support_count4() {
    support_row("21: c2Support count=4", 4, 0x2007, true);
}

#[test]
fn row22_support_count8() {
    support_row("22: c2Support count=8", 8, 0x2008, true);
}

/// Row 23 — `c2GJKSimplexMetric` for count 1/2/3 (and ERRORS.md row 8's
/// out-of-range counts live in errors.rs).
#[test]
fn row23_simplex_metric() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnSimplexF>("c2GJKSimplexMetric") };
    let mut rng = Rng::new(0x2009);
    let mut d = Diff::new("23: c2GJKSimplexMetric count=1|2|3");
    for count in [1, 2, 3] {
        for _ in 0..N {
            let s = simplex(&mut rng, count, 100.0);
            let mut cs = s;
            let mut rs = s;
            unsafe {
                let cv = c(&mut cs);
                let rv = r(&mut rs);
                d.check((count, s.verts[0].p, s.verts[1].p, s.verts[2].p), cv, rv);
            }
            // the function must not mutate the simplex
            d.check("no mutation", cs, rs);
        }
    }
    d.finish();
}

/// Rows 24 & 25 — `c22` / `c23`. Whole 152-byte simplex compared afterwards,
/// and branch coverage is measured so the test fails if a branch is never hit.
#[test]
fn row24_c22_all_branches() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnSimplexVoid>("c22") };
    let mut rng = Rng::new(0x200a);
    let mut d = Diff::new("24: c22");
    // count how often each resulting shape occurred, as a branch-coverage proxy
    let mut hits = [0usize; 3]; // count==1 via v<=0, count==1 via u<=0, count==2
    for i in 0..N * 6 {
        let mut s = simplex(&mut rng, 2, 20.0);
        // Deliberately steer towards each branch: the sign of dot(b, b-a) and
        // dot(a, a-b) is what selects them.
        match i % 3 {
            0 => {
                // make a and b nearly identical -> both u and v tiny
                s.verts[1].p = s.verts[0].p;
            }
            1 => {
                // put the origin outside the segment past `a`
                let p = s.verts[0].p;
                s.verts[1].p = C2v { x: p.x * 2.0, y: p.y * 2.0 };
            }
            _ => {}
        }
        let mut cs = s;
        let mut rs = s;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        d.check(s, cs, rs);
        if cs.count == 1 {
            if cs.verts[0].p.beq(&s.verts[0].p) {
                hits[0] += 1;
            } else {
                hits[1] += 1;
            }
        } else {
            hits[2] += 1;
        }
    }
    d.finish();
    assert!(hits.iter().all(|&h| h > 0), "c22 branch coverage gap: {hits:?}");
}

#[test]
fn row25_c23_all_branches() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnSimplexVoid>("c23") };
    let mut rng = Rng::new(0x200b);
    let mut d = Diff::new("25: c23");
    let mut by_count = [0usize; 4];
    for i in 0..N * 10 {
        let mut s = simplex(&mut rng, 3, 20.0);
        // steer towards the various vertex / edge / interior regions
        match i % 5 {
            0 => {
                // degenerate: all three points equal
                s.verts[1].p = s.verts[0].p;
                s.verts[2].p = s.verts[0].p;
            }
            1 => {
                // collinear
                let a = s.verts[0].p;
                let b = s.verts[1].p;
                s.verts[2].p = C2v {
                    x: a.x + (b.x - a.x) * 3.0,
                    y: a.y + (b.y - a.y) * 3.0,
                };
            }
            2 => {
                // triangle containing the origin -> interior (count 3)
                s.verts[0].p = C2v { x: -1.0 - rng.unit(), y: -1.0 - rng.unit() };
                s.verts[1].p = C2v { x: 1.0 + rng.unit(), y: -1.0 - rng.unit() };
                s.verts[2].p = C2v { x: rng.sym(0.5), y: 1.0 + rng.unit() };
            }
            3 => {
                // small triangle far from the origin -> vertex/edge regions
                let o = vec_tame(&mut rng, 50.0);
                for k in 0..3 {
                    s.verts[k].p = C2v { x: o.x + rng.sym(0.5), y: o.y + rng.sym(0.5) };
                }
            }
            _ => {}
        }
        let mut cs = s;
        let mut rs = s;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        d.check(s, cs, rs);
        if (0..4).contains(&cs.count) {
            by_count[cs.count as usize] += 1;
        }
    }
    d.finish();
    assert!(by_count[1] > 0 && by_count[2] > 0 && by_count[3] > 0,
            "c23 result-count coverage gap: {by_count:?}");
}

/// Row 26 — `c2D` for count 1, 2 (both sub-branches) and 3.
#[test]
fn row26_c2D() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnSimplexV>("c2D") };
    let mut rng = Rng::new(0x200c);
    let mut d = Diff::new("26: c2D count=1|2|3");
    let mut skew = 0usize;
    let mut ccw = 0usize;
    for count in [1, 2, 3] {
        for _ in 0..N * 2 {
            let s = simplex(&mut rng, count, 20.0);
            let mut cs = s;
            let mut rs = s;
            unsafe {
                let cv = c(&mut cs);
                let rv = r(&mut rs);
                d.check((count, s.verts[0].p, s.verts[1].p), cv, rv);
            }
            d.check("no mutation", cs, rs);
            if count == 2 {
                let ab = C2v {
                    x: s.verts[1].p.x - s.verts[0].p.x,
                    y: s.verts[1].p.y - s.verts[0].p.y,
                };
                let det = ab.x * -s.verts[0].p.y - ab.y * -s.verts[0].p.x;
                if det > 0.0 { skew += 1 } else { ccw += 1 }
            }
        }
    }
    d.finish();
    assert!(skew > 0 && ccw > 0, "c2D count=2 sub-branch gap: skew={skew} ccw={ccw}");
}

/// Row 27 — `c2L` for count 1 / 2 (random `div`, including 0) / other.
#[test]
fn row27_c2L() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnSimplexV>("c2L") };
    let mut rng = Rng::new(0x200d);
    let mut d = Diff::new("27: c2L count=1|2|other");
    for count in [1, 2, 3] {
        for _ in 0..N * 2 {
            let mut s = simplex(&mut rng, count, 20.0);
            if rng.below(6) == 0 {
                s.div = 0.0;
            }
            let mut cs = s;
            let mut rs = s;
            unsafe {
                let cv = c(&mut cs);
                let rv = r(&mut rs);
                d.check((count, s.div, s.verts[0].u, s.verts[1].u), cv, rv);
            }
            d.check("no mutation", cs, rs);
        }
    }
    d.finish();
}

/// Row 28 — `c2Witness` for count 1 / 2 / 3 / out-of-range.
#[test]
fn row28_c2Witness() {
    let l = libs();
    let (c, r) = unsafe { l.pair::<FnWitness>("c2Witness") };
    let mut rng = Rng::new(0x200e);
    let mut d = Diff::new("28: c2Witness count=1|2|3|oor");
    for count in [1, 2, 3, 0, 4, 7, -1] {
        for _ in 0..N {
            let mut s = simplex(&mut rng, count, 20.0);
            if rng.below(6) == 0 {
                s.div = 0.0;
            }
            let mut cs = s;
            let mut rs = s;
            let mut ca = C2v { x: 111.0, y: 222.0 };
            let mut cb = C2v { x: 333.0, y: 444.0 };
            let mut ra = ca;
            let mut rb = cb;
            unsafe {
                c(&mut cs, &mut ca, &mut cb);
                r(&mut rs, &mut ra, &mut rb);
            }
            d.check((count, s.div), (ca, cb), (ra, rb));
            d.check("no mutation", cs, rs);
        }
    }
    d.finish();
}
