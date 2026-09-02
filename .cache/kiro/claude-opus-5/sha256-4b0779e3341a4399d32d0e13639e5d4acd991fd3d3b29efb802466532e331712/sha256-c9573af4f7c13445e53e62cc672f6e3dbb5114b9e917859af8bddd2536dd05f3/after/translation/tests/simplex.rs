//! Phase B — CONFIGS.md rows 24..40: the simplex solvers `c22`, `c23`, and the
//! readers `c2D`, `c2L`, `c2Witness`.
//!
//! Every `c2sv` field (`sA`, `sB`, `p`, `u`, `iA`, `iB`) is filled with a
//! distinct value so that the whole-struct copies the C performs
//! (`s->a = s->b`, `s->b = s->c`, ...) are verified, not just the barycentric
//! weights. The complete `c2Simplex` is compared after each call.
//!
//! Rather than hand-building an input per branch, random simplices are
//! classified with the *same predicates the C uses* and the test asserts that
//! every arm was reached (so a row is only "covered" if it demonstrably ran).

mod common;
use common::*;

const N: usize = 40_000;

type FnSimplex1 = unsafe extern "C" fn(*mut C2Simplex);
type FnD = unsafe extern "C" fn(*mut C2Simplex) -> C2v;
type FnWitness = unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v);

fn dot(a: C2v, b: C2v) -> f32 {
    a.x * b.x + a.y * b.y
}
fn sub(a: C2v, b: C2v) -> C2v {
    C2v { x: a.x - b.x, y: a.y - b.y }
}
fn det2(a: C2v, b: C2v) -> f32 {
    a.x * b.y - a.y * b.x
}

/// Fill every field of every `c2sv` slot with a distinguishable value.
fn full_simplex(rng: &mut Rng, count: i32, ps: [C2v; 4]) -> C2Simplex {
    let mut s = C2Simplex::default();
    s.count = count;
    s.div = rng.range(0.1, 10.0);
    for k in 0..4 {
        s.v[k] = C2sv {
            sA: rng.v_tame(),
            sB: rng.v_tame(),
            p: ps[k],
            u: rng.range(-5.0, 5.0),
            iA: (k as i32) * 7 + 1,
            iB: (k as i32) * 11 + 2,
        };
    }
    s
}

/// Which arm of `c22` the C takes (mirrors lib.c lines 175..195).
fn c22_arm(s: &C2Simplex) -> usize {
    let a = s.v[0].p;
    let b = s.v[1].p;
    let u = dot(b, sub(b, a));
    let v = dot(a, sub(a, b));
    if v <= 0.0 {
        0 // keep a
    } else if u <= 0.0 {
        1 // keep b
    } else {
        2 // edge
    }
}

/// Which arm of `c23` the C takes (mirrors lib.c lines 197..250).
fn c23_arm(s: &C2Simplex) -> usize {
    let a = s.v[0].p;
    let b = s.v[1].p;
    let c = s.v[2].p;
    let uab = dot(b, sub(b, a));
    let vab = dot(a, sub(a, b));
    let ubc = dot(c, sub(c, b));
    let vbc = dot(b, sub(b, c));
    let uca = dot(a, sub(a, c));
    let vca = dot(c, sub(c, a));
    let area = det2(sub(b, a), sub(c, a));
    let uabc = det2(b, c) * area;
    let vabc = det2(c, a) * area;
    let wabc = det2(a, b) * area;
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

/// Generate p-values with a shape mix that reaches every solver arm.
fn gen_ps(rng: &mut Rng, i: usize) -> [C2v; 4] {
    let mut ps = [C2v::default(); 4];
    match i % 8 {
        // Triangle centred on the origin -> origin usually inside (arm 6).
        0 => {
            let t: Vec<C2v> = (0..3).map(|_| rng.v_tame()).collect();
            let cx = (t[0].x + t[1].x + t[2].x) / 3.0;
            let cy = (t[0].y + t[1].y + t[2].y) / 3.0;
            for k in 0..3 {
                ps[k] = C2v { x: t[k].x - cx, y: t[k].y - cy };
            }
            ps[3] = rng.v_tame();
        }
        // Small triangle far from the origin -> vertex/edge arms.
        1 => {
            let o = C2v { x: rng.range(-200.0, 200.0), y: rng.range(-200.0, 200.0) };
            for k in 0..4 {
                ps[k] = C2v { x: o.x + rng.range(-3.0, 3.0), y: o.y + rng.range(-3.0, 3.0) };
            }
        }
        // Collinear (area == 0).
        2 => {
            let a = rng.v_tame();
            let d = rng.v_tame();
            for k in 0..4 {
                let t = k as f32;
                ps[k] = C2v { x: a.x + t * d.x, y: a.y + t * d.y };
            }
        }
        // All coincident.
        3 => {
            let a = rng.v_tame();
            ps = [a; 4];
        }
        // Straddling the origin on one axis.
        4 => {
            for k in 0..4 {
                ps[k] = C2v { x: rng.range(-1.0, 1.0), y: rng.range(-100.0, 100.0) };
            }
        }
        // Huge magnitudes -> overflow in the area/det products.
        5 => {
            for k in 0..4 {
                ps[k] = C2v { x: rng.range(-1.0e20, 1.0e20), y: rng.range(-1.0e20, 1.0e20) };
            }
        }
        6 => {
            for k in 0..4 {
                ps[k] = rng.v_mixed();
            }
        }
        _ => {
            for k in 0..4 {
                ps[k] = rng.v_tame();
            }
        }
    }
    ps
}

/// rows 24,25,26,27 — c22, with all three arms proven to be hit
#[test]
fn row24_27_c22() {
    let l = libs();
    let (c, r) = l.pair::<FnSimplex1>("c22");
    let mut rng = Rng::new(0x51_0024);
    let mut hits = [0usize; 3];

    for i in 0..N {
        let ps = gen_ps(&mut rng, i);
        let s = full_simplex(&mut rng, 2, ps);
        hits[c22_arm(&s)] += 1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        same("c22", &(c22_arm(&s), ps[0], ps[1]), &cs, &rs);
    }
    // Fully arbitrary bit patterns (NaN predicates make all arms false ->
    // the trailing `else`).
    for _ in 0..N / 4 {
        let ps = [rng.v_any(), rng.v_any(), rng.v_any(), rng.v_any()];
        let s = full_simplex(&mut rng, 2, ps);
        let mut cs = s;
        let mut rs = s;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        same("c22 arbitrary", &(ps[0], ps[1]), &cs, &rs);
    }
    assert!(hits.iter().all(|&h| h > 0), "c22 arm coverage gap: {hits:?}");
    eprintln!("c22 arm hits: keep-a={} keep-b={} edge={}", hits[0], hits[1], hits[2]);
}

/// rows 28..35 — c23, with all seven arms proven to be hit
#[test]
fn row28_35_c23() {
    let l = libs();
    let (c, r) = l.pair::<FnSimplex1>("c23");
    let mut rng = Rng::new(0x51_0028);
    let mut hits = [0usize; 7];

    for i in 0..N {
        let ps = gen_ps(&mut rng, i);
        let s = full_simplex(&mut rng, 3, ps);
        hits[c23_arm(&s)] += 1;
        let mut cs = s;
        let mut rs = s;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        same("c23", &(c23_arm(&s), ps[0], ps[1], ps[2]), &cs, &rs);
    }
    for _ in 0..N / 4 {
        let ps = [rng.v_any(), rng.v_any(), rng.v_any(), rng.v_any()];
        let s = full_simplex(&mut rng, 3, ps);
        let mut cs = s;
        let mut rs = s;
        unsafe {
            c(&mut cs);
            r(&mut rs);
        }
        same("c23 arbitrary", &(ps[0], ps[1], ps[2]), &cs, &rs);
    }
    assert!(hits.iter().all(|&h| h > 0), "c23 arm coverage gap: {hits:?}");
    eprintln!("c23 arm hits: A={} B={} C={} AB={} BC={} CA={} interior={}",
        hits[0], hits[1], hits[2], hits[3], hits[4], hits[5], hits[6]);
}

/// row 36 — c2D at count 1, 2 (both det branches), 3 and out-of-range
#[test]
fn row36_c2d() {
    let l = libs();
    let (c, r) = l.pair::<FnD>("c2D");
    let mut rng = Rng::new(0x51_0036);
    let mut skew = 0usize;
    let mut ccw = 0usize;

    for count in [0i32, 1, 2, 3, 4, -1, 99] {
        for i in 0..N / 4 {
            let ps = gen_ps(&mut rng, i);
            let s = full_simplex(&mut rng, count, ps);
            if count == 2 {
                let ab = sub(ps[1], ps[0]);
                let neg = C2v { x: -ps[0].x, y: -ps[0].y };
                if det2(ab, neg) > 0.0 {
                    skew += 1;
                } else {
                    ccw += 1;
                }
            }
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
            same("c2D", &(count, ps[0], ps[1]), &cv, &rv);
            same("c2D no-mutate", &count, &cs, &rs);
        }
    }
    assert!(skew > 0 && ccw > 0, "c2D branch coverage gap: skew={skew} ccw={ccw}");
}

/// row 37 — c2L at count 1, 2 and out-of-range, incl. div == 0
#[test]
fn row37_c2l() {
    let l = libs();
    let (c, r) = l.pair::<FnD>("c2L");
    let mut rng = Rng::new(0x51_0037);

    for count in [0i32, 1, 2, 3, 4, -1, 99] {
        for i in 0..N / 4 {
            let ps = gen_ps(&mut rng, i);
            let mut s = full_simplex(&mut rng, count, ps);
            s.div = match i % 6 {
                0 => 0.0,
                1 => -0.0,
                2 => rng.f32_mixed(),
                3 => f32::MIN_POSITIVE,
                _ => s.div,
            };
            for k in 0..4 {
                s.v[k].u = if i % 5 == 0 { rng.f32_mixed() } else { s.v[k].u };
            }
            let mut cs = s;
            let mut rs = s;
            let (cv, rv) = unsafe { (c(&mut cs), r(&mut rs)) };
            same("c2L", &(count, s.div, ps[0], ps[1]), &cv, &rv);
            same("c2L no-mutate", &count, &cs, &rs);
        }
    }
}

/// rows 38,39,40 — c2Witness at count 1, 2, 3 and out-of-range
#[test]
fn row38_40_c2witness() {
    let l = libs();
    let (c, r) = l.pair::<FnWitness>("c2Witness");
    let mut rng = Rng::new(0x51_0038);

    for count in [0i32, 1, 2, 3, 4, -1, 99] {
        for i in 0..N / 4 {
            let ps = gen_ps(&mut rng, i);
            let mut s = full_simplex(&mut rng, count, ps);
            s.div = match i % 7 {
                0 => 0.0,
                1 => -0.0,
                2 => rng.f32_mixed(),
                3 => f32::MIN_POSITIVE,
                4 => f32::INFINITY,
                _ => s.div,
            };
            for k in 0..4 {
                s.v[k].u = match i % 4 {
                    0 => rng.f32_mixed(),
                    1 => 0.0,
                    _ => s.v[k].u,
                };
                if i % 9 == 0 {
                    s.v[k].sA = rng.v_mixed();
                    s.v[k].sB = rng.v_mixed();
                }
            }
            let poison = C2v { x: f32::from_bits(0x5eed_face), y: f32::from_bits(0x5eed_beef) };
            let (mut ca, mut cb) = (poison, poison);
            let (mut ra, mut rb) = (poison, poison);
            let mut cs = s;
            let mut rs = s;
            unsafe {
                c(&mut cs, &mut ca, &mut cb);
                r(&mut rs, &mut ra, &mut rb);
            }
            same("c2Witness a", &(count, s.div), &ca, &ra);
            same("c2Witness b", &(count, s.div), &cb, &rb);
            same("c2Witness no-mutate", &count, &cs, &rs);
        }
    }
}
