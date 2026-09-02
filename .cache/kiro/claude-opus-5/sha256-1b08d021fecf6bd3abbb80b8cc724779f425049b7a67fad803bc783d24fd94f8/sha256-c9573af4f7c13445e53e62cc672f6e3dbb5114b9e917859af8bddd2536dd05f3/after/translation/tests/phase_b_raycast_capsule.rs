//! Phase B rows 43–55: `c2RaytoCapsule`.
//!
//! `c2RaytoCapsule` is the most heavily branched function in the library: nine
//! distinct outcomes, three of which delegate to `c2RaytoCircle`. To prove the
//! rows are not merely *claimed* to cover those branches, every call is also
//! classified by `branch_of()`, which recomputes only the *branch conditions*
//! using the C library's own exported primitives. The tests then assert that
//! each expected branch was actually reached.

mod common;
use common::*;

const N: usize = 20_000;

/// Which of the nine outcomes `c2RaytoCapsule` takes, recomputed with the C's
/// own exports so the classification matches the C's arithmetic exactly.
///
/// `b'a'` … `b'i'` follow the enumeration in `CONFIGS.md`.
fn branch_of(c: &Impl, a: c2Ray, b: c2Capsule) -> u8 {
    unsafe {
        let my = (c.c2Norm)((c.c2Sub)(b.b, b.a));
        let mx = (c.c2CCW90)(my);
        let m = c2m { x: mx, y: my };
        let cap_n = (c.c2Sub)(b.b, b.a);
        let y_bb = (c.c2MulmvT)(m, cap_n);
        let y_ap = (c.c2MulmvT)(m, (c.c2Sub)(a.p, b.a));
        let y_ad = (c.c2MulmvT)(m, a.d);
        let y_ae = (c.c2Add)(y_ap, (c.c2Mulvs)(y_ad, a.t));
        let bb = c2AABB {
            min: (c.c2V)(-b.r, 0.0),
            max: (c.c2V)(b.r, y_bb.y),
        };
        if (c.c2AABBtoPoint)(bb, y_ap) != 0 {
            return b'a';
        }
        if (c.c2CircleToPoint)(c2Circle { p: b.a, r: b.r }, a.p) != 0 {
            return b'b';
        }
        if (c.c2CircleToPoint)(c2Circle { p: b.b, r: b.r }, a.p) != 0 {
            return b'c';
        }
        let tabs = |v: f32| if v < 0.0 { -v } else { v };
        let tmin = |x: f32, y: f32| if x < y { x } else { y };
        if y_ae.x * y_ap.x < 0.0 || tmin(tabs(y_ae.x), tabs(y_ap.x)) < b.r {
            if tabs(y_ap.x) < b.r {
                if y_ap.y < 0.0 {
                    return b'd';
                }
                return b'e';
            }
            let cc = if y_ap.x > 0.0 { b.r } else { -b.r };
            let dd = y_ae.x - y_ap.x;
            let t = (cc - y_ap.x) / dd;
            let y = y_ap.y + (y_ae.y - y_ap.y) * t;
            if y <= 0.0 {
                return b'f';
            }
            if y >= y_bb.y {
                return b'g';
            }
            return b'h';
        }
        b'i'
    }
}

#[derive(Default)]
struct Hist([usize; 9]);

impl Hist {
    fn add(&mut self, b: u8) {
        if (b'a'..=b'i').contains(&b) {
            self.0[(b - b'a') as usize] += 1;
        }
    }
    fn get(&self, b: u8) -> usize {
        self.0[(b - b'a') as usize]
    }
    fn report(&self) -> String {
        (0..9)
            .map(|i| format!("{}={}", (b'a' + i as u8) as char, self.0[i]))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn drive(
    d: &mut Diff,
    h: &mut Hist,
    c: &Impl,
    r: &Impl,
    ray: c2Ray,
    cap: c2Capsule,
) {
    h.add(branch_of(c, ray, cap));
    cmp_ray_capsule(d, c, r, ray, cap);
}

/// A well-formed capsule with the requested orientation class.
fn capsule(g: &mut Rng, kind: u32) -> c2Capsule {
    let a = g.v(20.0);
    let len = 0.5 + g.unit() * 20.0;
    let b = match kind % 4 {
        0 => c2v { x: a.x, y: a.y + len },          // vertical
        1 => c2v { x: a.x + len, y: a.y },          // horizontal
        2 => {
            let u = g.dir();
            c2v { x: a.x + u.x * len, y: a.y + u.y * len }
        } // diagonal
        _ => c2v { x: a.x, y: a.y - len },          // reversed (b below a)
    };
    c2Capsule {
        a,
        b,
        r: 0.05 + g.unit() * 5.0,
    }
}

// ---------------------------------------------------------------------------
// Rows 43–45: the three "origin is already inside" early returns.
// ---------------------------------------------------------------------------

#[test]
fn row43_45_capsule_origin_inside() {
    let (c, r) = pair();
    let mut d = Diff::new("43/44/45: c2RaytoCapsule origin inside slab / cap a / cap b");
    let mut h = Hist::default();
    let mut g = Rng::new(0x4301);
    for i in 0..N * 2 {
        let cap = capsule(&mut g, i as u32);
        let axis = unsafe { (c.c2Norm)((c.c2Sub)(cap.b, cap.a)) };
        let perp = c2v { x: -axis.y, y: axis.x };
        let len = unsafe { (c.c2Len)((c.c2Sub)(cap.b, cap.a)) };
        let p = match i % 3 {
            // inside the slab: along the axis, |perp offset| < r
            0 => {
                let s = g.unit() * len;
                let o = g.sym(cap.r * 0.95);
                c2v {
                    x: cap.a.x + axis.x * s + perp.x * o,
                    y: cap.a.y + axis.y * s + perp.y * o,
                }
            }
            // inside end-cap a, past the flat end so the slab test fails
            1 => {
                let ang = g.unit() * std::f32::consts::TAU;
                let rr = g.unit() * cap.r * 0.9;
                c2v {
                    x: cap.a.x - axis.x * rr.abs() * 0.5 + ang.cos() * rr * 0.5,
                    y: cap.a.y - axis.y * rr.abs() * 0.5 + ang.sin() * rr * 0.5,
                }
            }
            // inside end-cap b
            _ => {
                let ang = g.unit() * std::f32::consts::TAU;
                let rr = g.unit() * cap.r * 0.9;
                c2v {
                    x: cap.b.x + axis.x * rr.abs() * 0.5 + ang.cos() * rr * 0.5,
                    y: cap.b.y + axis.y * rr.abs() * 0.5 + ang.sin() * rr * 0.5,
                }
            }
        };
        let ray = c2Ray {
            p,
            d: g.dir(),
            t: g.unit() * 40.0,
        };
        drive(&mut d, &mut h, c, r, ray, cap);
    }
    assert!(h.get(b'a') > 100, "branch a under-covered: {}", h.report());
    assert!(h.get(b'b') > 10, "branch b under-covered: {}", h.report());
    assert!(h.get(b'c') > 10, "branch c under-covered: {}", h.report());
    println!("row43_45 branch histogram: {}", h.report());
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 46–51: the straddle branch and its five sub-outcomes, plus the miss.
// ---------------------------------------------------------------------------

#[test]
fn row46_51_capsule_straddle_and_miss() {
    let (c, r) = pair();
    let mut d = Diff::new("46-51: c2RaytoCapsule straddle sub-branches d..i");
    let mut h = Hist::default();
    let mut g = Rng::new(0x4601);
    for i in 0..N * 6 {
        let cap = capsule(&mut g, i as u32);
        let axis = unsafe { (c.c2Norm)((c.c2Sub)(cap.b, cap.a)) };
        let perp = c2v { x: -axis.y, y: axis.x };
        let len = unsafe { (c.c2Len)((c.c2Sub)(cap.b, cap.a)) };
        // Start outside the capsule, at a controlled perpendicular offset and
        // a controlled position along the axis, and aim roughly across it.
        let off = match i % 6 {
            0 => cap.r * (0.2 + g.unit() * 0.7),   // |yAp.x| < r  -> d/e
            1 => cap.r * (1.0 + g.unit() * 4.0),   // |yAp.x| >= r -> f/g/h
            2 => -cap.r * (0.2 + g.unit() * 0.7),
            3 => -cap.r * (1.0 + g.unit() * 4.0),
            4 => cap.r * (1.0 + g.unit() * 0.2),
            _ => g.sym(cap.r * 8.0),
        };
        let along = match i % 5 {
            0 => -len * g.unit(),               // before a
            1 => len * (1.0 + g.unit()),        // past b
            2 => len * g.unit(),                // alongside
            3 => 0.0,
            _ => g.sym(len * 3.0),
        };
        let p = c2v {
            x: cap.a.x + axis.x * along + perp.x * off,
            y: cap.a.y + axis.y * along + perp.y * off,
        };
        // aim across the capsule (so yAe.x flips sign) or along it (so it does not)
        let dir = match i % 4 {
            0 => c2v { x: -perp.x, y: -perp.y },
            1 => c2v { x: perp.x, y: perp.y },
            2 => axis,
            _ => g.dir(),
        };
        let dir = if off > 0.0 && i % 4 == 0 {
            dir
        } else if off < 0.0 && i % 4 == 1 {
            dir
        } else {
            dir
        };
        let ray = c2Ray {
            p,
            d: dir,
            t: (len + cap.r) * (0.1 + g.unit() * 3.0),
        };
        drive(&mut d, &mut h, c, r, ray, cap);
    }
    for br in [b'd', b'e', b'f', b'g', b'h', b'i'] {
        assert!(
            h.get(br) > 20,
            "branch {} under-covered: {}",
            br as char,
            h.report()
        );
    }
    println!("row46_51 branch histogram: {}", h.report());
    d.finish();
}

// ---------------------------------------------------------------------------
// Row 50 specifically: the side-wall hit, both signs of `c`, since
// `out->n = M.x` and `out->n = c2Skew(M.y)` are NOT negatives of each other.
// ---------------------------------------------------------------------------

#[test]
fn row50_capsule_side_wall_both_signs() {
    let (c, r) = pair();
    let mut d = Diff::new("50: c2RaytoCapsule side-wall normal, c>0 (M.x) and c<0 (Skew(M.y))");
    let mut h = Hist::default();
    let mut g = Rng::new(0x5001);
    for i in 0..N * 2 {
        let cap = capsule(&mut g, i as u32);
        let axis = unsafe { (c.c2Norm)((c.c2Sub)(cap.b, cap.a)) };
        let perp = c2v { x: -axis.y, y: axis.x };
        let len = unsafe { (c.c2Len)((c.c2Sub)(cap.b, cap.a)) };
        let sign = if i % 2 == 0 { 1.0f32 } else { -1.0f32 };
        // well outside the slab on one side, aimed straight at the mid-point
        let off = sign * cap.r * (1.5 + g.unit() * 4.0);
        let along = len * (0.2 + g.unit() * 0.6);
        let p = c2v {
            x: cap.a.x + axis.x * along + perp.x * off,
            y: cap.a.y + axis.y * along + perp.y * off,
        };
        let target = c2v {
            x: cap.a.x + axis.x * along,
            y: cap.a.y + axis.y * along,
        };
        let dir = unsafe {
            (c.c2Norm)(c2v {
                x: target.x - p.x,
                y: target.y - p.y,
            })
        };
        let ray = c2Ray {
            p,
            d: dir,
            t: off.abs() * (1.0 + g.unit()),
        };
        drive(&mut d, &mut h, c, r, ray, cap);
    }
    assert!(
        h.get(b'h') > 100,
        "side-wall branch h under-covered: {}",
        h.report()
    );
    println!("row50 branch histogram: {}", h.report());
    d.finish();
}

// ---------------------------------------------------------------------------
// Row 52: orientation sweep, including the reversed capsule (yBb.y < 0, so
// `capsule_bb` is inverted and `c2AABBtoPoint` can never accept).
// ---------------------------------------------------------------------------

#[test]
fn row52_capsule_orientation_sweep() {
    let (c, r) = pair();
    let mut d = Diff::new("52: c2RaytoCapsule orientation sweep incl. reversed a/b");
    let mut h = Hist::default();
    let mut g = Rng::new(0x5201);
    for i in 0..N * 4 {
        // exact axis-aligned and exactly-reversed capsules
        let a = g.v(15.0);
        let len = 0.5 + g.unit() * 15.0;
        let b = match i % 8 {
            0 => c2v { x: a.x, y: a.y + len },
            1 => c2v { x: a.x, y: a.y - len },
            2 => c2v { x: a.x + len, y: a.y },
            3 => c2v { x: a.x - len, y: a.y },
            4 => c2v { x: a.x + len, y: a.y + len },
            5 => c2v { x: a.x - len, y: a.y - len },
            6 => c2v { x: a.x + len, y: a.y - len },
            _ => c2v { x: a.x - len, y: a.y + len },
        };
        let cap = c2Capsule {
            a,
            b,
            r: 0.05 + g.unit() * 4.0,
        };
        let ray = c2Ray {
            p: g.v(30.0),
            d: if g.below(3) == 0 {
                [
                    c2v { x: 1.0, y: 0.0 },
                    c2v { x: 0.0, y: 1.0 },
                    c2v { x: -1.0, y: 0.0 },
                    c2v { x: 0.0, y: -1.0 },
                ][(i / 8) % 4]
            } else {
                g.dir()
            },
            t: g.unit() * 60.0,
        };
        drive(&mut d, &mut h, c, r, ray, cap);
    }
    println!("row52 branch histogram: {}", h.report());
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 53–55: radius classes, fully random, and hostile bit patterns.
// ---------------------------------------------------------------------------

#[test]
fn row53_54_capsule_radius_classes_and_random() {
    let (c, r) = pair();
    let mut d = Diff::new("53/54: c2RaytoCapsule radius classes + uniform random");
    let mut h = Hist::default();
    let mut g = Rng::new(0x5301);
    const RADII: &[f32] = &[
        0.0,
        -0.0,
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        1e-6,
        0.5,
        1.0,
        50.0,
        1e18,
        -1.0,
        -5.0,
    ];
    for i in 0..N * 6 {
        let mut cap = capsule(&mut g, i as u32);
        if i % 2 == 0 {
            cap.r = RADII[(i / 2) % RADII.len()];
        }
        let ray = c2Ray {
            p: g.v(30.0),
            d: if g.below(2) == 0 { g.dir() } else { g.v(2.0) },
            t: g.unit() * 60.0,
        };
        drive(&mut d, &mut h, c, r, ray, cap);
    }
    println!("row53_54 branch histogram: {}", h.report());
    d.finish();
}

#[test]
fn row55_capsule_hostile() {
    let (c, r) = pair();
    let mut d = Diff::new("55: c2RaytoCapsule degenerate / special / random-bit inputs");
    let mut g = Rng::new(0x5501);
    for _ in 0..N * 4 {
        let cap = match g.below(5) {
            0 => {
                // degenerate: a == b  =>  c2Norm(0,0) = (NaN, NaN)
                let q = g.v(10.0);
                c2Capsule { a: q, b: q, r: g.unit() * 5.0 }
            }
            1 => {
                let q = g.v(10.0);
                c2Capsule {
                    a: q,
                    b: c2v { x: q.x, y: q.y },
                    r: 0.0,
                }
            }
            2 => c2Capsule {
                a: g.v_special(),
                b: g.v_special(),
                r: g.special_f32(),
            },
            3 => c2Capsule {
                a: c2v { x: g.any_bits_f32(), y: g.any_bits_f32() },
                b: c2v { x: g.any_bits_f32(), y: g.any_bits_f32() },
                r: g.any_bits_f32(),
            },
            _ => c2Capsule {
                a: g.v_mixed(1e3),
                b: g.v_mixed(1e3),
                r: g.mixed_f32(1e3),
            },
        };
        let ray = match g.below(3) {
            0 => c2Ray {
                p: g.v(20.0),
                d: g.dir(),
                t: g.unit() * 40.0,
            },
            1 => c2Ray {
                p: g.v_special(),
                d: g.v_special(),
                t: g.special_f32(),
            },
            _ => c2Ray {
                p: c2v { x: g.any_bits_f32(), y: g.any_bits_f32() },
                d: c2v { x: g.any_bits_f32(), y: g.any_bits_f32() },
                t: g.any_bits_f32(),
            },
        };
        cmp_ray_capsule(&mut d, c, r, ray, cap);
    }
    d.finish();
}
