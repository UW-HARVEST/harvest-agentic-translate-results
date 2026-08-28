#![allow(non_snake_case)]
//! Phase B — Group 4: exported GJK simplex machinery.
//! CONFIGS.md rows 37..58.

mod common;
use common::*;
use std::os::raw::c_int;

const N: usize = 4096;

fn rand_sv(rng: &mut Rng) -> c2sv {
    c2sv {
        sA: rng.vec(),
        sB: rng.vec(),
        p: rng.vec(),
        u: rng.coord(),
        iA: rng.below(8) as c_int,
        iB: rng.below(8) as c_int,
    }
}

fn rand_simplex(rng: &mut Rng, count: c_int) -> c2Simplex {
    c2Simplex {
        a: rand_sv(rng),
        b: rand_sv(rng),
        c: rand_sv(rng),
        d: rand_sv(rng),
        div: rng.coord(),
        count,
    }
}

fn special_sv(rng: &mut Rng) -> c2sv {
    c2sv {
        sA: rng.special_vec(),
        sB: rng.special_vec(),
        p: rng.special_vec(),
        u: rng.special(),
        iA: rng.below(8) as c_int,
        iB: rng.below(8) as c_int,
    }
}

fn special_simplex(rng: &mut Rng, count: c_int) -> c2Simplex {
    c2Simplex {
        a: special_sv(rng),
        b: special_sv(rng),
        c: special_sv(rng),
        d: special_sv(rng),
        div: rng.special(),
        count,
    }
}

fn dot(a: c2v, b: c2v) -> f32 {
    a.x * b.x + a.y * b.y
}
fn sub(a: c2v, b: c2v) -> c2v {
    c2v {
        x: a.x - b.x,
        y: a.y - b.y,
    }
}
fn det2(a: c2v, b: c2v) -> f32 {
    a.x * b.y - a.y * b.x
}

/// Which of `c22`'s 3 regions does this simplex fall into? (coverage bookkeeping)
fn c22_region(s: &c2Simplex) -> usize {
    let (a, b) = (s.a.p, s.b.p);
    let u = dot(b, sub(b, a));
    let v = dot(a, sub(a, b));
    if v <= 0.0 {
        0
    } else if u <= 0.0 {
        1
    } else {
        2
    }
}

/// Which of `c23`'s 7 regions? (coverage bookkeeping)
fn c23_region(s: &c2Simplex) -> usize {
    let (a, b, c) = (s.a.p, s.b.p, s.c.p);
    let uAB = dot(b, sub(b, a));
    let vAB = dot(a, sub(a, b));
    let uBC = dot(c, sub(c, b));
    let vBC = dot(b, sub(b, c));
    let uCA = dot(a, sub(a, c));
    let vCA = dot(c, sub(c, a));
    let area = det2(sub(b, a), sub(c, a));
    let uABC = det2(b, c) * area;
    let vABC = det2(c, a) * area;
    let wABC = det2(a, b) * area;
    if vAB <= 0.0 && uCA <= 0.0 {
        0
    } else if uAB <= 0.0 && vBC <= 0.0 {
        1
    } else if uBC <= 0.0 && vCA <= 0.0 {
        2
    } else if uAB > 0.0 && vAB > 0.0 && wABC <= 0.0 {
        3
    } else if uBC > 0.0 && vBC > 0.0 && uABC <= 0.0 {
        4
    } else if uCA > 0.0 && vCA > 0.0 && vABC <= 0.0 {
        5
    } else {
        6
    }
}

// --- rows 37, 38 ---------------------------------------------------------
#[test]
fn cfg_simplex_metric() {
    let mut a = DiffAccum::new("cfg_simplex_metric");
    let mut rng = Rng::new(0x7eed_0001);
    for &count in &[2i32, 3] {
        for i in 0..N {
            let s = rand_simplex(&mut rng, count);
            a.check(format!("count={count} #{i}"), |side| {
                let mut s = s;
                let r = c2GJKSimplexMetric(side, &mut s);
                (r, s)
            });
        }
        for i in 0..N {
            let s = special_simplex(&mut rng, count);
            a.check(format!("special count={count} #{i}"), |side| {
                let mut s = s;
                let r = c2GJKSimplexMetric(side, &mut s);
                (r, s)
            });
        }
    }
    a.finish();
}

// --- rows 39..42 ---------------------------------------------------------
#[test]
fn cfg_c22() {
    let mut a = DiffAccum::new("cfg_c22");
    let mut rng = Rng::new(0x7eed_0002);
    let mut hit = [0usize; 3];
    for i in 0..(N * 4) {
        let s = rand_simplex(&mut rng, 2);
        hit[c22_region(&s)] += 1;
        a.check(format!("rand #{i} region={}", c22_region(&s)), |side| {
            let mut s = s;
            c22(side, &mut s);
            s
        });
    }
    // targeted: v <= 0  ⇔  dot(a, a-b) <= 0
    for i in 0..N {
        let mut s = rand_simplex(&mut rng, 2);
        s.a.p = c2v { x: 0.0, y: 0.0 }; // ⇒ v = 0
        hit[c22_region(&s)] += 1;
        a.check(format!("v<=0 #{i}"), |side| {
            let mut s = s;
            c22(side, &mut s);
            s
        });
    }
    // targeted: u <= 0 (b at origin, a away ⇒ u = 0, v > 0)
    for i in 0..N {
        let mut s = rand_simplex(&mut rng, 2);
        s.b.p = c2v { x: 0.0, y: 0.0 };
        s.a.p = c2v {
            x: 1.0 + rng.unit(),
            y: 1.0 + rng.unit(),
        };
        hit[c22_region(&s)] += 1;
        a.check(format!("u<=0 #{i}"), |side| {
            let mut s = s;
            c22(side, &mut s);
            s
        });
    }
    // targeted: interior — origin projects strictly inside segment
    for i in 0..N {
        let mut s = rand_simplex(&mut rng, 2);
        let t = 0.2 + rng.unit() * 0.6;
        let dir = c2v {
            x: 1.0 + rng.unit(),
            y: 0.5 + rng.unit(),
        };
        let off = c2v { x: -dir.y, y: dir.x };
        s.a.p = c2v {
            x: off.x - dir.x * t,
            y: off.y - dir.y * t,
        };
        s.b.p = c2v {
            x: off.x + dir.x * (1.0 - t),
            y: off.y + dir.y * (1.0 - t),
        };
        hit[c22_region(&s)] += 1;
        a.check(format!("interior #{i}"), |side| {
            let mut s = s;
            c22(side, &mut s);
            s
        });
    }
    // non-finite
    for i in 0..N {
        let s = special_simplex(&mut rng, 2);
        a.check(format!("special #{i}"), |side| {
            let mut s = s;
            c22(side, &mut s);
            s
        });
    }
    a.finish();
    eprintln!("cfg_c22 region coverage: {hit:?}");
    assert!(hit.iter().all(|&n| n > 0), "c22 regions not all hit: {hit:?}");
}

// --- rows 43..50 ---------------------------------------------------------
#[test]
fn cfg_c23() {
    let mut a = DiffAccum::new("cfg_c23");
    let mut rng = Rng::new(0x7eed_0003);
    let mut hit = [0usize; 7];
    // fully random (row 50)
    for i in 0..(N * 8) {
        let s = rand_simplex(&mut rng, 3);
        hit[c23_region(&s)] += 1;
        a.check(format!("rand #{i} region={}", c23_region(&s)), |side| {
            let mut s = s;
            c23(side, &mut s);
            s
        });
    }
    // vertex regions (rows 43..45): put one vertex at the origin
    for slot in 0..3 {
        for i in 0..N {
            let mut s = rand_simplex(&mut rng, 3);
            let z = c2v { x: 0.0, y: 0.0 };
            match slot {
                0 => s.a.p = z,
                1 => s.b.p = z,
                _ => s.c.p = z,
            }
            hit[c23_region(&s)] += 1;
            a.check(format!("vertex slot={slot} #{i}"), |side| {
                let mut s = s;
                c23(side, &mut s);
                s
            });
        }
    }
    // triangles containing the origin (row 49 / interior) and triangles far away
    for i in 0..N * 2 {
        let mut s = rand_simplex(&mut rng, 3);
        let r = 1.0 + rng.unit() * 3.0;
        let base = rng.unit() * std::f32::consts::TAU;
        let pts: Vec<c2v> = (0..3)
            .map(|k| {
                let t = base + k as f32 * std::f32::consts::TAU / 3.0;
                c2v {
                    x: r * t.cos(),
                    y: r * t.sin(),
                }
            })
            .collect();
        s.a.p = pts[0];
        s.b.p = pts[1];
        s.c.p = pts[2];
        hit[c23_region(&s)] += 1;
        a.check(format!("contains-origin #{i}"), |side| {
            let mut s = s;
            c23(side, &mut s);
            s
        });
        // reversed winding ⇒ negative area ⇒ the other edge branches
        let mut s2 = s;
        s2.b.p = pts[2];
        s2.c.p = pts[1];
        hit[c23_region(&s2)] += 1;
        a.check(format!("contains-origin-cw #{i}"), |side| {
            let mut s = s2;
            c23(side, &mut s);
            s
        });
    }
    // edge regions: origin projects onto one edge, outside the triangle
    for edge in 0..3 {
        for i in 0..N {
            let mut s = rand_simplex(&mut rng, 3);
            // Build an edge straddling the origin's projection, and put the third
            // vertex far on one side.
            let dir = c2v {
                x: 1.0 + rng.unit(),
                y: rng.sym(1.0),
            };
            let nrm = c2v { x: -dir.y, y: dir.x };
            let off = 0.5 + rng.unit();
            let p0 = c2v {
                x: nrm.x * off - dir.x,
                y: nrm.y * off - dir.y,
            };
            let p1 = c2v {
                x: nrm.x * off + dir.x,
                y: nrm.y * off + dir.y,
            };
            let far = c2v {
                x: nrm.x * (off + 3.0 + rng.unit()),
                y: nrm.y * (off + 3.0 + rng.unit()),
            };
            match edge {
                0 => {
                    s.a.p = p0;
                    s.b.p = p1;
                    s.c.p = far;
                }
                1 => {
                    s.b.p = p0;
                    s.c.p = p1;
                    s.a.p = far;
                }
                _ => {
                    s.c.p = p0;
                    s.a.p = p1;
                    s.b.p = far;
                }
            }
            hit[c23_region(&s)] += 1;
            a.check(format!("edge={edge} #{i}"), |side| {
                let mut s = s;
                c23(side, &mut s);
                s
            });
            // flipped winding
            let mut s2 = s;
            std::mem::swap(&mut s2.b, &mut s2.c);
            hit[c23_region(&s2)] += 1;
            a.check(format!("edge={edge} flip #{i}"), |side| {
                let mut s = s2;
                c23(side, &mut s);
                s
            });
        }
    }
    // non-finite
    for i in 0..N {
        let s = special_simplex(&mut rng, 3);
        a.check(format!("special #{i}"), |side| {
            let mut s = s;
            c23(side, &mut s);
            s
        });
    }
    a.finish();
    eprintln!("cfg_c23 region coverage: {hit:?}");
    assert!(hit.iter().all(|&n| n > 0), "c23 regions not all hit: {hit:?}");
}

// --- rows 51..53 ---------------------------------------------------------
#[test]
fn cfg_c2d() {
    let mut a = DiffAccum::new("cfg_c2d");
    let mut rng = Rng::new(0x7eed_0004);
    let mut skew_branch = 0usize;
    let mut ccw_branch = 0usize;
    for i in 0..N {
        let s = rand_simplex(&mut rng, 1);
        a.check(format!("count=1 #{i}"), |side| {
            let mut s = s;
            let r = c2D(side, &mut s);
            (r, s)
        });
    }
    for i in 0..(N * 4) {
        let s = rand_simplex(&mut rng, 2);
        let ab = sub(s.b.p, s.a.p);
        let na = c2v {
            x: -s.a.p.x,
            y: -s.a.p.y,
        };
        if det2(ab, na) > 0.0 {
            skew_branch += 1;
        } else {
            ccw_branch += 1;
        }
        a.check(format!("count=2 #{i}"), |side| {
            let mut s = s;
            let r = c2D(side, &mut s);
            (r, s)
        });
    }
    for i in 0..N {
        let s = special_simplex(&mut rng, 2);
        a.check(format!("special count=2 #{i}"), |side| {
            let mut s = s;
            let r = c2D(side, &mut s);
            (r, s)
        });
    }
    a.finish();
    eprintln!("cfg_c2d: skew={skew_branch} ccw90={ccw_branch}");
    assert!(skew_branch > 0 && ccw_branch > 0);
}

// --- rows 54, 55 ---------------------------------------------------------
#[test]
fn cfg_c2l() {
    let mut a = DiffAccum::new("cfg_c2l");
    let mut rng = Rng::new(0x7eed_0005);
    for &count in &[1i32, 2] {
        for i in 0..(N * 2) {
            let s = rand_simplex(&mut rng, count);
            a.check(format!("count={count} #{i}"), |side| {
                let mut s = s;
                let r = c2L(side, &mut s);
                (r, s)
            });
        }
        for i in 0..N {
            let s = special_simplex(&mut rng, count);
            a.check(format!("special count={count} #{i}"), |side| {
                let mut s = s;
                let r = c2L(side, &mut s);
                (r, s)
            });
        }
        // div == 0 ⇒ den = inf
        for i in 0..N {
            let mut s = rand_simplex(&mut rng, count);
            s.div = 0.0;
            a.check(format!("div=0 count={count} #{i}"), |side| {
                let mut s = s;
                let r = c2L(side, &mut s);
                (r, s)
            });
        }
    }
    a.finish();
}

// --- rows 56..58 ---------------------------------------------------------
#[test]
fn cfg_witness() {
    let mut a = DiffAccum::new("cfg_witness");
    let mut rng = Rng::new(0x7eed_0006);
    for &count in &[1i32, 2, 3] {
        for i in 0..(N * 2) {
            let s = rand_simplex(&mut rng, count);
            a.check(format!("count={count} #{i}"), |side| {
                let mut s = s;
                let mut oa = c2v { x: 9.0, y: -9.0 };
                let mut ob = c2v { x: -9.0, y: 9.0 };
                c2Witness(side, &mut s, &mut oa, &mut ob);
                (oa, ob, s)
            });
        }
        for i in 0..N {
            let s = special_simplex(&mut rng, count);
            a.check(format!("special count={count} #{i}"), |side| {
                let mut s = s;
                let mut oa = c2v { x: 9.0, y: -9.0 };
                let mut ob = c2v { x: -9.0, y: 9.0 };
                c2Witness(side, &mut s, &mut oa, &mut ob);
                (oa, ob, s)
            });
        }
        // div == 0 ⇒ den = inf
        for i in 0..N {
            let mut s = rand_simplex(&mut rng, count);
            s.div = 0.0;
            a.check(format!("div=0 count={count} #{i}"), |side| {
                let mut s = s;
                let mut oa = c2v { x: 9.0, y: -9.0 };
                let mut ob = c2v { x: -9.0, y: 9.0 };
                c2Witness(side, &mut s, &mut oa, &mut ob);
                (oa, ob, s)
            });
        }
    }
    a.finish();
}
