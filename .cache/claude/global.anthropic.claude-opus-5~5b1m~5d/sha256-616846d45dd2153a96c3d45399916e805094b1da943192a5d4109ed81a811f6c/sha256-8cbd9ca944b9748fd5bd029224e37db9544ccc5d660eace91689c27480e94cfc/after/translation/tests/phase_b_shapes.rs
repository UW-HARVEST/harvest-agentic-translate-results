//! Phase B — differential tests for the level-2 collision routines.
//!
//! `CONFIGS.md` rows 23-44: `c2CircletoCircle`, `c2CircletoAABB`,
//! `c2CircletoCapsule`, driven directly through the `.so` exports.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn circ(g: &mut Rng, m: f32) -> C2Circle {
    C2Circle {
        p: g.v_finite(m),
        r: g.unit() * m,
    }
}

/// Exactly-representable scale factors, so boundary constructions are exact.
const KS: &[f32] = &[
    0.25, 0.5, 1.0, 2.0, 3.0, 4.0, 8.0, 16.0, 32.0, 64.0, 0.125, 128.0,
];

fn cc_row(row: &'static str, n: usize, seed: u64, mk: impl Fn(&mut Rng) -> (C2Circle, C2Circle)) {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(seed);
    let mut case = Case::new(row);
    for _ in 0..n {
        let (a, b) = mk(&mut g);
        let cv = unsafe { (c.c2CircletoCircle)(a, b) };
        let rv = unsafe { (r.c2CircletoCircle)(a, b) };
        case.eq(cv, rv, || format!("c2CircletoCircle({a:?}, {b:?})"));
    }
    case.finish();
}

fn ca_row(row: &'static str, n: usize, seed: u64, mk: impl Fn(&mut Rng) -> (C2Circle, C2Aabb)) {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(seed);
    let mut case = Case::new(row);
    for _ in 0..n {
        let (a, b) = mk(&mut g);
        let cv = unsafe { (c.c2CircletoAABB)(a, b) };
        let rv = unsafe { (r.c2CircletoAABB)(a, b) };
        case.eq(cv, rv, || format!("c2CircletoAABB({a:?}, {b:?})"));
    }
    case.finish();
}

fn cp_row(row: &'static str, n: usize, seed: u64, mk: impl Fn(&mut Rng) -> (C2Circle, C2Capsule)) {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(seed);
    let mut case = Case::new(row);
    for _ in 0..n {
        let (a, b) = mk(&mut g);
        let cv = unsafe { (c.c2CircletoCapsule)(a, b) };
        let rv = unsafe { (r.c2CircletoCapsule)(a, b) };
        case.eq(cv, rv, || format!("c2CircletoCapsule({a:?}, {b:?})"));
    }
    case.finish();
}

/// Which capsule branch (`lib.c:88`/`92`/`95`) a given input takes, computed
/// independently of both libraries via the C `.so`'s own primitives.
fn capsule_branch(A: C2Circle, B: C2Capsule) -> u8 {
    let c = &apis().c;
    unsafe {
        let n = (c.c2Sub)(B.b, B.a);
        let ap = (c.c2Sub)(A.p, B.a);
        let da = (c.c2Dot)(ap, n);
        if da < 0.0 {
            return 0; // branch A
        }
        let db = (c.c2Dot)((c.c2Sub)(A.p, B.b), n);
        if db < 0.0 { 1 } else { 2 } // branch B / C
    }
}

// ===========================================================================
// c2CircletoCircle — rows 23-27
// ===========================================================================

#[test]
fn row23_cc_finite_mixed() {
    // Radii sized against the centre spread so roughly half the cases collide.
    cc_row("23 c2CircletoCircle finite", 30_000, 23, |g| {
        let a = C2Circle {
            p: g.v_finite(50.0),
            r: g.unit() * 40.0,
        };
        let b = C2Circle {
            p: g.v_finite(50.0),
            r: g.unit() * 40.0,
        };
        (a, b)
    });
}

#[test]
fn row24_cc_exact_touch_boundary() {
    // Pythagorean 3-4-5 scaled by an exact power of two: d2 == r2 exactly, so
    // the strict `<` must reject. 4096 combinations.
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("24 c2CircletoCircle d2==r2");
    let mut g = Rng::seeded(24);
    for &k in KS {
        for i in 0..=16u32 {
            for eps in [0.0f32, -1.0, 1.0] {
                let dist = 5.0 * k; // exact
                let ar = (i as f32) * dist / 16.0;
                let br = dist - ar + eps * k * 0.5;
                let base = g.v_finite(4.0);
                let a = C2Circle { p: base, r: ar };
                let b = C2Circle {
                    p: C2v {
                        x: base.x + 3.0 * k,
                        y: base.y + 4.0 * k,
                    },
                    r: br,
                };
                let cv = unsafe { (c.c2CircletoCircle)(a, b) };
                let rv = unsafe { (r.c2CircletoCircle)(a, b) };
                case.eq(cv, rv, || format!("k={k} i={i} eps={eps} {a:?} {b:?}"));
                // Sanity: with eps == 0 this is the exact-touch case -> reject.
                if eps == 0.0 && base.x == 0.0 && base.y == 0.0 {
                    assert_eq!(cv, 0, "exact touch must not collide: {a:?} {b:?}");
                }
            }
        }
    }
    case.finish();
}

#[test]
fn row25_cc_zero_radii_coincident() {
    cc_row("25 c2CircletoCircle zero r/coincident", 20_000, 25, |g| {
        let p = g.v_finite(20.0);
        let ar = [0.0f32, -0.0, 1.0][g.below(3) as usize];
        let br = [0.0f32, -0.0, 1.0][g.below(3) as usize];
        let bp = if g.next_u32() & 1 == 0 { p } else { g.v_finite(20.0) };
        (C2Circle { p, r: ar }, C2Circle { p: bp, r: br })
    });
}

#[test]
fn row26_cc_negative_and_overflow() {
    cc_row("26 c2CircletoCircle neg/overflow", 20_000, 26, |g| {
        let big = [f32::MAX, -f32::MAX, 1e38, 3.0e38, 1.0];
        let a = C2Circle {
            p: C2v {
                x: big[g.below(5) as usize] * (if g.next_u32() & 1 == 0 { 1.0 } else { -1.0 }),
                y: big[g.below(5) as usize],
            },
            r: -g.unit() * 100.0,
        };
        let b = C2Circle {
            p: C2v {
                x: big[g.below(5) as usize],
                y: big[g.below(5) as usize] * -1.0,
            },
            r: if g.next_u32() & 1 == 0 { f32::MAX } else { -50.0 },
        };
        (a, b)
    });
}

#[test]
fn row27_cc_nan_all_slots() {
    cc_row("27 c2CircletoCircle NaN slots", 20_000, 27, |g| {
        (
            C2Circle {
                p: g.v_spicy(),
                r: g.spicy(),
            },
            C2Circle {
                p: g.v_spicy(),
                r: g.spicy(),
            },
        )
    });
}

// ===========================================================================
// c2CircletoAABB — rows 28-34
// ===========================================================================

/// A valid box (`min <= max` per lane).
fn valid_box(g: &mut Rng) -> C2Aabb {
    let (x0, y0) = (g.sym(50.0), g.sym(50.0));
    C2Aabb {
        min: C2v { x: x0, y: y0 },
        max: C2v {
            x: x0 + g.unit() * 60.0,
            y: y0 + g.unit() * 60.0,
        },
    }
}

#[test]
fn row28_ca_centre_inside() {
    ca_row("28 c2CircletoAABB centre inside", 20_000, 28, |g| {
        let b = valid_box(g);
        let p = C2v {
            x: b.min.x + g.unit() * (b.max.x - b.min.x),
            y: b.min.y + g.unit() * (b.max.y - b.min.y),
        };
        (
            C2Circle {
                p,
                r: g.unit() * 10.0,
            },
            b,
        )
    });
}

#[test]
fn row29_ca_centre_outside_all_regions() {
    // Cycles through the 9 Voronoi regions of the box (4 edges, 4 corners,
    // interior), so both clamp lanes hit `lo`, pass-through and `hi`.
    ca_row("29 c2CircletoAABB 9 regions", 30_000, 29, |g| {
        let b = valid_box(g);
        let (w, h) = (b.max.x - b.min.x, b.max.y - b.min.y);
        let pick = |g: &mut Rng, lo: f32, hi: f32, span: f32| match g.below(3) {
            0 => lo - g.unit() * span - 0.5, // below -> clamps to lo
            1 => lo + g.unit() * (hi - lo),  // inside -> passes through
            _ => hi + g.unit() * span + 0.5, // above -> clamps to hi
        };
        let p = C2v {
            x: pick(g, b.min.x, b.max.x, w),
            y: pick(g, b.min.y, b.max.y, h),
        };
        (
            C2Circle {
                p,
                r: g.unit() * 40.0,
            },
            b,
        )
    });
}

#[test]
fn row30_ca_degenerate_box() {
    ca_row("30 c2CircletoAABB degenerate box", 20_000, 30, |g| {
        let c0 = g.v_finite(30.0);
        let b = match g.below(3) {
            0 => C2Aabb { min: c0, max: c0 }, // point box
            1 => C2Aabb {
                min: c0,
                max: C2v {
                    x: c0.x,
                    y: c0.y + g.unit() * 20.0,
                }, // zero width
            },
            _ => C2Aabb {
                min: c0,
                max: C2v {
                    x: c0.x + g.unit() * 20.0,
                    y: c0.y,
                }, // zero height
            },
        };
        (
            C2Circle {
                p: g.v_finite(40.0),
                r: g.unit() * 20.0,
            },
            b,
        )
    });
}

#[test]
fn row31_ca_inverted_box() {
    ca_row("31 c2CircletoAABB inverted box", 20_000, 31, |g| {
        let v = valid_box(g);
        // Invert one lane or both — the C code has no validity check.
        let b = match g.below(3) {
            0 => C2Aabb {
                min: C2v { x: v.max.x, y: v.min.y },
                max: C2v { x: v.min.x, y: v.max.y },
            },
            1 => C2Aabb {
                min: C2v { x: v.min.x, y: v.max.y },
                max: C2v { x: v.max.x, y: v.min.y },
            },
            _ => C2Aabb { min: v.max, max: v.min },
        };
        (
            C2Circle {
                p: g.v_finite(60.0),
                r: g.unit() * 30.0,
            },
            b,
        )
    });
}

#[test]
fn row32_ca_exact_boundary() {
    // Point box at the origin, centre at (3k, 4k), r == 5k -> d2 == r2 exactly.
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("32 c2CircletoAABB d2==r2");
    let mut g = Rng::seeded(32);
    for &k in KS {
        for sx in [1.0f32, -1.0] {
            for sy in [1.0f32, -1.0] {
                for eps in [0.0f32, -0.25, 0.25] {
                    for _ in 0..21 {
                        let o = C2v {
                            x: g.sym(8.0).trunc(),
                            y: g.sym(8.0).trunc(),
                        };
                        let bx = C2Aabb { min: o, max: o };
                        let a = C2Circle {
                            p: C2v {
                                x: o.x + sx * 3.0 * k,
                                y: o.y + sy * 4.0 * k,
                            },
                            r: 5.0 * k + eps * k,
                        };
                        let cv = unsafe { (c.c2CircletoAABB)(a, bx) };
                        let rv = unsafe { (r.c2CircletoAABB)(a, bx) };
                        case.eq(cv, rv, || format!("k={k} eps={eps} {a:?} {bx:?}"));
                        if eps == 0.0 {
                            assert_eq!(cv, 0, "exact touch must not collide: {a:?} {bx:?}");
                        }
                    }
                }
            }
        }
    }
    case.finish();
}

#[test]
fn row33_ca_radius_specials() {
    ca_row("33 c2CircletoAABB radius specials", 20_000, 33, |g| {
        let rr = [
            0.0f32,
            -0.0,
            -7.5,
            f32::MAX,
            1e38,
            f32::MIN_POSITIVE,
            f32::from_bits(1),
            f32::from_bits(0x7fc0_0000),
            f32::INFINITY,
        ][g.below(9) as usize];
        (
            C2Circle {
                p: g.v_finite(40.0),
                r: rr,
            },
            valid_box(g),
        )
    });
}

#[test]
fn row34_ca_nan_inf_all_slots() {
    ca_row("34 c2CircletoAABB NaN/inf slots", 20_000, 34, |g| {
        (
            C2Circle {
                p: g.v_spicy(),
                r: g.spicy(),
            },
            C2Aabb {
                min: g.v_spicy(),
                max: g.v_spicy(),
            },
        )
    });
}

// ===========================================================================
// c2CircletoCapsule — rows 35-44
// ===========================================================================

fn valid_capsule(g: &mut Rng) -> C2Capsule {
    let a = g.v_finite(50.0);
    C2Capsule {
        a,
        b: C2v {
            x: a.x + g.sym(60.0),
            y: a.y + g.sym(60.0),
        },
        r: g.unit() * 20.0,
    }
}

/// Rejection-samples until the input lands on the wanted capsule branch.
fn cp_branch_row(row: &'static str, n: usize, seed: u64, want: u8) {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(seed);
    let mut case = Case::new(row);
    let mut made = 0usize;
    let mut tries = 0usize;
    while made < n && tries < n * 200 {
        tries += 1;
        let cap = valid_capsule(&mut g);
        // Sample the centre relative to the segment so all branches are cheap
        // to hit: t < 0 -> branch A, 0..1 -> B, > 1 -> C, plus perpendicular
        // offset.
        let t = match want {
            0 => -g.unit() * 2.0 - 0.05,
            1 => g.unit(),
            _ => 1.0 + g.unit() * 2.0 + 0.05,
        };
        let nx = cap.b.x - cap.a.x;
        let ny = cap.b.y - cap.a.y;
        let off = g.sym(30.0);
        let len = (nx * nx + ny * ny).sqrt();
        let (px, py) = if len > 0.0 {
            (
                cap.a.x + nx * t - ny / len * off,
                cap.a.y + ny * t + nx / len * off,
            )
        } else {
            (cap.a.x + off, cap.a.y + off)
        };
        let a = C2Circle {
            p: C2v { x: px, y: py },
            r: g.unit() * 25.0,
        };
        if capsule_branch(a, cap) != want {
            continue;
        }
        made += 1;
        let cv = unsafe { (c.c2CircletoCapsule)(a, cap) };
        let rv = unsafe { (r.c2CircletoCapsule)(a, cap) };
        case.eq(cv, rv, || format!("branch={want} {a:?} {cap:?}"));
    }
    assert!(
        made >= n / 2,
        "row {row}: only produced {made}/{n} inputs on branch {want}"
    );
    case.finish();
}

#[test]
fn row35_cp_branch_a() {
    cp_branch_row("35 c2CircletoCapsule branch A (da<0)", 20_000, 35, 0);
}

#[test]
fn row36_cp_branch_b() {
    cp_branch_row("36 c2CircletoCapsule branch B (segment)", 30_000, 36, 1);
}

#[test]
fn row37_cp_branch_c() {
    cp_branch_row("37 c2CircletoCapsule branch C (da>=0,db>=0)", 20_000, 37, 2);
}

#[test]
fn row38_cp_all_branches_from_random() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(38);
    let mut case = Case::new("38 c2CircletoCapsule random, all branches");
    let mut hits = [0usize; 3];
    for _ in 0..30_000 {
        let cap = valid_capsule(&mut g);
        let a = circ(&mut g, 80.0);
        hits[capsule_branch(a, cap) as usize] += 1;
        let cv = unsafe { (c.c2CircletoCapsule)(a, cap) };
        let rv = unsafe { (r.c2CircletoCapsule)(a, cap) };
        case.eq(cv, rv, || format!("{a:?} {cap:?}"));
    }
    assert!(
        hits.iter().all(|&h| h > 100),
        "not all capsule branches exercised: {hits:?}"
    );
    eprintln!("  row 38 branch hits A/B/C = {hits:?}");
    case.finish();
}

#[test]
fn row39_cp_degenerate_capsule() {
    // a == b  =>  n == (0,0), c2Dot(n,n) == 0. `da` is then ±0, which is NOT
    // `< 0`, and `db` likewise, so the C code takes branch C and the unguarded
    // division is never reached.
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(39);
    let mut case = Case::new("39 c2CircletoCapsule a==b");
    for _ in 0..20_000 {
        let pt = g.v_finite(50.0);
        let cap = C2Capsule {
            a: pt,
            b: pt,
            r: g.unit() * 20.0,
        };
        let a = circ(&mut g, 60.0);
        assert_eq!(
            capsule_branch(a, cap),
            2,
            "degenerate capsule should take branch C"
        );
        let cv = unsafe { (c.c2CircletoCapsule)(a, cap) };
        let rv = unsafe { (r.c2CircletoCapsule)(a, cap) };
        case.eq(cv, rv, || format!("{a:?} {cap:?}"));
    }
    case.finish();
}

#[test]
fn row40_cp_near_degenerate_underflow() {
    // Capsules whose `c2Dot(n,n)` underflows to zero or to a subnormal, i.e.
    // the neighbourhood of the unguarded `da / c2Dot(n,n)` division.
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(40);
    let mut case = Case::new("40 c2CircletoCapsule underflowing n");
    let mut div_by_zero_on_branch_b = 0usize;
    let scales = [1e-23f32, 1e-22, 1e-20, 3e-24, f32::from_bits(1), 1e-30];
    for _ in 0..20_000 {
        let s = scales[g.below(scales.len() as u32) as usize];
        let a0 = if g.next_u32() & 1 == 0 {
            C2v { x: 0.0, y: 0.0 }
        } else {
            C2v {
                x: g.sym(1.0) * s,
                y: g.sym(1.0) * s,
            }
        };
        let cap = C2Capsule {
            a: a0,
            b: C2v {
                x: a0.x + g.sym(1.0) * s,
                y: a0.y + g.sym(1.0) * s,
            },
            r: g.unit() * s,
        };
        // Centre either at the same tiny scale (can reach branch B) or far away.
        let p = if g.next_u32() & 1 == 0 {
            C2v {
                x: g.sym(1.0) * s,
                y: g.sym(1.0) * s,
            }
        } else {
            g.v_finite(10.0)
        };
        let a = C2Circle {
            p,
            r: g.unit() * s,
        };
        if capsule_branch(a, cap) == 1 {
            let n = unsafe { (c.c2Sub)(cap.b, cap.a) };
            if unsafe { (c.c2Dot)(n, n) } == 0.0 {
                div_by_zero_on_branch_b += 1;
            }
        }
        let cv = unsafe { (c.c2CircletoCapsule)(a, cap) };
        let rv = unsafe { (r.c2CircletoCapsule)(a, cap) };
        case.eq(cv, rv, || format!("s={s:e} {a:?} {cap:?}"));
    }
    eprintln!("  row 40: division-by-zero reached on branch B {div_by_zero_on_branch_b} times");
    case.finish();
}

#[test]
fn row41_cp_overflowing_n() {
    // `c2Dot(n,n)` overflows to +inf while `da` stays finite, so the quotient
    // is 0 and `c2Mulvs(n, 0)` computes `inf * 0`-adjacent values; also the
    // `inf / inf -> NaN` variant.
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(41);
    let mut case = Case::new("41 c2CircletoCapsule overflowing n");
    let mut seen_b = 0usize;
    for _ in 0..20_000 {
        let big = [1e30f32, 1e25, f32::MAX, 1e20, 1e38][g.below(5) as usize];
        let cap = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v {
                x: big,
                y: if g.next_u32() & 1 == 0 { 0.0 } else { big },
            },
            r: g.unit() * 50.0,
        };
        // Small `ap` keeps `da` finite; large `ap` makes it inf.
        let m = [1.0f32, 10.0, 1e8, 5e29, 1e15][g.below(5) as usize];
        let a = C2Circle {
            p: C2v {
                x: m * g.unit(),
                y: m * g.sym(1.0),
            },
            r: g.unit() * 50.0,
        };
        if capsule_branch(a, cap) == 1 {
            seen_b += 1;
        }
        let cv = unsafe { (c.c2CircletoCapsule)(a, cap) };
        let rv = unsafe { (r.c2CircletoCapsule)(a, cap) };
        case.eq(cv, rv, || format!("{a:?} {cap:?}"));
    }
    assert!(seen_b > 0, "row 41 never reached branch B");
    eprintln!("  row 41: branch B reached {seen_b} times with overflowing n");
    case.finish();
}

#[test]
fn row42_cp_radius_specials() {
    cp_row("42 c2CircletoCapsule radius specials", 20_000, 42, |g| {
        let pick = |g: &mut Rng| {
            [
                0.0f32,
                -0.0,
                -12.0,
                f32::MAX,
                1e38,
                f32::MIN_POSITIVE,
                f32::from_bits(0x7fc0_0000),
                f32::INFINITY,
                f32::NEG_INFINITY,
            ][g.below(9) as usize]
        };
        let mut cap = valid_capsule(g);
        cap.r = pick(g);
        (
            C2Circle {
                p: g.v_finite(60.0),
                r: pick(g),
            },
            cap,
        )
    });
}

#[test]
fn row43_cp_exact_boundary() {
    // Branch C: capsule (-L,0)-(0,0), centre (3k,4k) => d2 == (5k)^2; set
    // A.r + B.r == 5k exactly. Branch B: capsule (0,0)-(0,10), centre (3k,5)
    // => d2 == (3k)^2; set A.r + B.r == 3k exactly.
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("43 c2CircletoCapsule d2==r*r");
    for &k in KS {
        for i in 0..=16u32 {
            for eps in [0.0f32, -0.25, 0.25] {
                for which in [0u8, 1] {
                    let (a, cap, want_branch) = if which == 0 {
                        let total = 5.0 * k;
                        let ar = (i as f32) * total / 16.0;
                        (
                            C2Circle {
                                p: C2v { x: 3.0 * k, y: 4.0 * k },
                                r: ar,
                            },
                            C2Capsule {
                                a: C2v { x: -64.0, y: 0.0 },
                                b: C2v { x: 0.0, y: 0.0 },
                                r: total - ar + eps * k,
                            },
                            2u8,
                        )
                    } else {
                        let total = 3.0 * k;
                        let ar = (i as f32) * total / 16.0;
                        (
                            C2Circle {
                                p: C2v { x: 3.0 * k, y: 5.0 },
                                r: ar,
                            },
                            C2Capsule {
                                a: C2v { x: 0.0, y: 0.0 },
                                b: C2v { x: 0.0, y: 10.0 },
                                r: total - ar + eps * k,
                            },
                            1u8,
                        )
                    };
                    assert_eq!(
                        capsule_branch(a, cap),
                        want_branch,
                        "boundary case took the wrong branch: {a:?} {cap:?}"
                    );
                    let cv = unsafe { (c.c2CircletoCapsule)(a, cap) };
                    let rv = unsafe { (r.c2CircletoCapsule)(a, cap) };
                    case.eq(cv, rv, || format!("k={k} i={i} eps={eps} {a:?} {cap:?}"));
                    if eps == 0.0 {
                        assert_eq!(cv, 0, "exact touch must not collide: {a:?} {cap:?}");
                    }
                }
            }
        }
    }
    case.finish();
}

#[test]
fn row44_cp_nan_inf_all_slots() {
    cp_row("44 c2CircletoCapsule NaN/inf slots", 30_000, 44, |g| {
        (
            C2Circle {
                p: g.v_spicy(),
                r: g.spicy(),
            },
            C2Capsule {
                a: g.v_spicy(),
                b: g.v_spicy(),
                r: g.spicy(),
            },
        )
    });
}
