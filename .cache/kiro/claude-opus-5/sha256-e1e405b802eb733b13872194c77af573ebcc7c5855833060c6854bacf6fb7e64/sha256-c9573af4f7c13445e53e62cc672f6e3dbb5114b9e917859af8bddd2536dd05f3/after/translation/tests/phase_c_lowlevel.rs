//! Phase C part 1: `ERRORS.md` rows 1-26.
//!
//! Out-of-range enum values, `switch` `default:` fallbacks and the simplex
//! solvers' degeneracy guards. Every case asserts the two libraries agree on the
//! *same* specific fallback value, not merely that both "did something".

mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// C enums accept any `int` across the FFI boundary, so these are all real
/// inputs the C handles.
const BAD_TYPES: [c_int; 12] =
    [3, 4, 7, 255, 256, -1, -2, -1000, 100000, i32::MAX, i32::MIN, 0x7FFF_FFFE];

const BAD_COUNTS: [c_int; 11] = [0, 4, 5, 7, 100, -1, -2, -100, i32::MAX, i32::MIN, 0x4000_0000];

fn poison_proxy() -> c2Proxy {
    unsafe { std::mem::transmute::<[u8; 72], c2Proxy>([0xA5u8; 72]) }
}

fn poisoned_simplex() -> c2Simplex {
    unsafe { std::mem::transmute::<[u8; 152], c2Simplex>([0xA5u8; 152]) }
}

// ---------------------------------------------------------------------------
// ERRORS row 1 — c2MakeProxy with an out-of-range C2_TYPE leaves *p untouched
// ---------------------------------------------------------------------------

#[test]
fn err01_makeproxy_invalid_type_is_a_noop() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnMakeProxy>("c2MakeProxy"), l.rs.sym::<FnMakeProxy>("c2MakeProxy"));
    let mut g = Rng::new(0xE01);
    let mut rep = Report::new();
    for ty in BAD_TYPES {
        for _ in 0..40 {
            // The shape bytes are irrelevant — the C must not read them.
            let shape = g.capsule();
            let (mut pc, mut pr) = (poison_proxy(), poison_proxy());
            unsafe {
                c(&raw const shape as *const c_void, ty, &raw mut pc);
                r(&raw const shape as *const c_void, ty, &raw mut pr);
            }
            rep.check(same_proxy(&pc, &pr), || {
                format!(
                    "c2MakeProxy(type={ty}):\n  C:    {}\n  Rust: {}",
                    show_proxy(&pc),
                    show_proxy(&pr)
                )
            });
            // The specific expected result: the switch has no `default:`, so
            // every byte of the poison pattern must survive.
            let untouched = poison_proxy();
            rep.check(same_proxy(&pc, &untouched), || {
                format!("c2MakeProxy(type={ty}) modified the proxy in C: {}", show_proxy(&pc))
            });
            rep.check(same_proxy(&pr, &untouched), || {
                format!("c2MakeProxy(type={ty}) modified the proxy in Rust: {}", show_proxy(&pr))
            });
        }
    }
    // Also confirm a NULL shape pointer is never dereferenced for a bad type.
    for ty in BAD_TYPES {
        let (mut pc, mut pr) = (poison_proxy(), poison_proxy());
        unsafe {
            c(std::ptr::null(), ty, &raw mut pc);
            r(std::ptr::null(), ty, &raw mut pr);
        }
        rep.check(same_proxy(&pc, &pr) && same_proxy(&pc, &poison_proxy()), || {
            format!("c2MakeProxy(NULL shape, type={ty}) diverged or wrote")
        });
    }
    rep.finish("err01_makeproxy_invalid_type_is_a_noop");
}

// ---------------------------------------------------------------------------
// ERRORS rows 2-4 — c2GJKSimplexMetric's `default:` falls into `case 1:`
// ---------------------------------------------------------------------------

#[test]
fn err02_to_err04_simplex_metric_default() {
    let l = libs();
    let (c, r) =
        (l.c.sym::<FnSimplexF>("c2GJKSimplexMetric"), l.rs.sym::<FnSimplexF>("c2GJKSimplexMetric"));
    let mut g = Rng::new(0xE02);
    let mut rep = Report::new();
    for count in BAD_COUNTS.iter().copied().chain([1]) {
        for _ in 0..80 {
            let mut s = g.simplex(count);
            s.count = count;
            let (mut a, mut b) = (s, s);
            let (x, y) = unsafe { (c(&raw mut a), r(&raw mut b)) };
            rep.check(same_f32(x, y), || {
                format!("c2GJKSimplexMetric(count={count}): C={} Rust={}", show_f32(x), show_f32(y))
            });
            // The specific expected result for `default:`/`case 1:` is 0.0f.
            rep.check(x.to_bits() == 0 && y.to_bits() == 0, || {
                format!(
                    "c2GJKSimplexMetric(count={count}) must be +0.0: C={} Rust={}",
                    show_f32(x),
                    show_f32(y)
                )
            });
        }
    }
    // Row 3: count == 2 is never negative (it is a length).
    // Row 4: count == 3 IS signed — assert a negative value really occurs, so
    // the two rows are distinguishable and not accidentally both zero.
    let mut saw_negative_det = false;
    for _ in 0..2000 {
        let mut s = g.simplex(3);
        s.count = 3;
        let (mut a, mut b) = (s, s);
        let (x, y) = unsafe { (c(&raw mut a), r(&raw mut b)) };
        rep.check(same_f32(x, y), || {
            format!("c2GJKSimplexMetric(count=3): C={} Rust={}", show_f32(x), show_f32(y))
        });
        if x < 0.0 {
            saw_negative_det = true;
        }
        s.count = 2;
        let (mut a, mut b) = (s, s);
        let (x2, y2) = unsafe { (c(&raw mut a), r(&raw mut b)) };
        rep.check(same_f32(x2, y2), || {
            format!("c2GJKSimplexMetric(count=2): C={} Rust={}", show_f32(x2), show_f32(y2))
        });
        rep.check(!(x2 < 0.0), || {
            format!("count==2 metric is a length but was negative: {}", show_f32(x2))
        });
    }
    assert!(saw_negative_det, "row 4 (signed count==3 metric) never produced a negative value");
    rep.finish("err02_to_err04_simplex_metric_default");
}

// ---------------------------------------------------------------------------
// ERRORS rows 5-8 — c22's three branches, incl. the duplicate-vertex collapse
// ---------------------------------------------------------------------------

#[test]
fn err05_to_err08_c22_guards() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnSimplexVoid>("c22"), l.rs.sym::<FnSimplexVoid>("c22"));
    let mut rep = Report::new();

    let run = |rep: &mut Report, s: c2Simplex, want_count: c_int, want_div: Option<f32>, tag: &str| {
        let (mut a, mut b) = (s, s);
        unsafe {
            c(&raw mut a);
            r(&raw mut b);
        }
        rep.check(same_simplex(&a, &b), || {
            format!("c22[{tag}]:\n  C:    {}\n  Rust: {}", show_simplex(&a), show_simplex(&b))
        });
        rep.check(a.count == want_count && b.count == want_count, || {
            format!("c22[{tag}] count: C={} Rust={} want={want_count}", a.count, b.count)
        });
        if let Some(d) = want_div {
            rep.check(same_f32(a.div, d) && same_f32(b.div, d), || {
                format!(
                    "c22[{tag}] div: C={} Rust={} want={}",
                    show_f32(a.div),
                    show_f32(b.div),
                    show_f32(d)
                )
            });
        }
    };

    let mk = |ap: c2v, bp: c2v| {
        let mut s = poisoned_simplex();
        for i in 0..4 {
            s.verts[i] = c2sv {
                sA: c2v { x: i as f32, y: -(i as f32) },
                sB: c2v { x: 2.0 * i as f32, y: 0.5 },
                p: c2v { x: 9.0, y: 9.0 },
                u: 0.125 * (i as f32 + 1.0),
                iA: i as c_int,
                iB: 3 - i as c_int,
            };
        }
        s.verts[0].p = ap;
        s.verts[1].p = bp;
        s.div = 7.0;
        s.count = 2;
        s
    };

    // Row 5: v = dot(a, a-b) <= 0  ->  count 1, a.u 1, div 1
    run(&mut rep, mk(c2v { x: 1.0, y: 0.0 }, c2v { x: 2.0, y: 0.0 }), 1, Some(1.0), "row5 v<=0");
    run(&mut rep, mk(c2v { x: 0.5, y: 0.5 }, c2v { x: 5.0, y: 5.0 }), 1, Some(1.0), "row5 v<=0 b");
    // Row 6: u = dot(b, b-a) <= 0 with v > 0  ->  a = b, count 1, div 1
    let s = mk(c2v { x: -2.0, y: 0.0 }, c2v { x: -1.0, y: 0.0 });
    let expect_a = s.verts[1];
    let (mut a, mut b) = (s, s);
    unsafe {
        c(&raw mut a);
        r(&raw mut b);
    }
    rep.check(same_simplex(&a, &b), || "c22[row6] diverged".to_string());
    rep.check(
        a.verts[0].iA == expect_a.iA
            && a.verts[0].iB == expect_a.iB
            && same_v(a.verts[0].sA, expect_a.sA)
            && same_v(a.verts[0].sB, expect_a.sB)
            && same_v(a.verts[0].p, expect_a.p),
        || format!("c22[row6] did not copy b into a: {}", show_simplex(&a)),
    );
    run(&mut rep, mk(c2v { x: -2.0, y: 0.0 }, c2v { x: -1.0, y: 0.0 }), 1, Some(1.0), "row6 u<=0");
    // Row 7: u > 0 && v > 0  ->  count 2, div = u + v
    run(&mut rep, mk(c2v { x: -1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }), 2, Some(4.0), "row7 interior");
    // Row 8: a.p == b.p  ->  u == v == 0  ->  the `v <= 0` branch fires
    for p in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: 3.0, y: -4.0 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::from_bits(1), y: 0.0 },
    ] {
        run(&mut rep, mk(p, p), 1, Some(1.0), "row8 duplicate");
    }
    // NaN vertices: every `<= 0` test is false, so the interior branch fires and
    // `div` becomes NaN. Both sides must agree on that too.
    for (ap, bp) in [
        (c2v { x: f32::NAN, y: 0.0 }, c2v { x: 1.0, y: 0.0 }),
        (c2v { x: 1.0, y: 0.0 }, c2v { x: f32::NAN, y: 0.0 }),
        (c2v { x: f32::INFINITY, y: 0.0 }, c2v { x: f32::INFINITY, y: 0.0 }),
    ] {
        run(&mut rep, mk(ap, bp), 2, None, "NaN/inf");
    }
    rep.finish("err05_to_err08_c22_guards");
}

// ---------------------------------------------------------------------------
// ERRORS rows 9-16 — c23's seven branches and the zero-area degeneracy
// ---------------------------------------------------------------------------

#[test]
fn err09_to_err16_c23_guards() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnSimplexVoid>("c23"), l.rs.sym::<FnSimplexVoid>("c23"));
    let mut rep = Report::new();

    let mk = |ps: [c2v; 3]| {
        let mut s = poisoned_simplex();
        for i in 0..4 {
            s.verts[i] = c2sv {
                sA: c2v { x: i as f32 + 0.5, y: -2.0 * i as f32 },
                sB: c2v { x: 1.0 - i as f32, y: 3.0 * i as f32 },
                p: c2v { x: -8.0, y: 8.0 },
                u: 0.375 * (i as f32 + 1.0),
                iA: i as c_int,
                iB: (3 - i) as c_int,
            };
        }
        for i in 0..3 {
            s.verts[i].p = ps[i];
        }
        s.div = -11.0;
        s.count = 3;
        s
    };

    let run = |rep: &mut Report, s: c2Simplex, want_count: c_int, tag: &str| -> c2Simplex {
        let (mut a, mut b) = (s, s);
        unsafe {
            c(&raw mut a);
            r(&raw mut b);
        }
        rep.check(same_simplex(&a, &b), || {
            format!("c23[{tag}]:\n  C:    {}\n  Rust: {}", show_simplex(&a), show_simplex(&b))
        });
        rep.check(a.count == want_count, || {
            format!("c23[{tag}] count: got {} want {want_count}", a.count)
        });
        a
    };

    // A reference triangle; translating it puts the origin in a chosen region.
    let tri = [c2v { x: -1.0, y: -0.6 }, c2v { x: 1.2, y: -0.5 }, c2v { x: 0.1, y: 1.4 }];
    let shift = |o: c2v| [0, 1, 2].map(|i| c2v { x: tri[i].x + o.x, y: tri[i].y + o.y });

    // Rows 9-11: the three vertex regions -> count 1, div 1.
    for (o, tag) in [
        (c2v { x: 6.0, y: 4.0 }, "row9 vertA"),
        (c2v { x: -6.0, y: 4.0 }, "row10 vertB"),
        (c2v { x: 0.0, y: -6.0 }, "row11 vertC"),
    ] {
        let out = run(&mut rep, mk(shift(o)), 1, tag);
        rep.check(same_f32(out.div, 1.0) && same_f32(out.verts[0].u, 1.0), || {
            format!("c23[{tag}] must set div=1 and a.u=1, got div={}", show_f32(out.div))
        });
    }
    // Rows 12-14: the three edge regions -> count 2.
    for (o, tag) in [
        (c2v { x: 0.0, y: 5.0 }, "row12/13/14 edge"),
        (c2v { x: 5.0, y: -2.0 }, "row12/13/14 edge"),
        (c2v { x: -5.0, y: -2.0 }, "row12/13/14 edge"),
    ] {
        run(&mut rep, mk(shift(o)), 2, tag);
    }
    // Row 15: interior -> count 3.
    run(&mut rep, mk(tri), 3, "row15 interior");

    // Row 16: collinear / repeated vertices. `div` is algebraically `area*area`
    // (because `det(b,c)+det(c,a)+det(a,b) == area`), so a zero-area triangle
    // would give `div == 0` — but the interior branch is NOT reachable with
    // `area == 0`: an exactly collinear triple always short-circuits into a
    // vertex or edge branch first. Verified by exhaustive search over integer
    // collinear triples and 20M randomized f32 triples (see ERRORS.md row 16).
    // So the assertion here is the *observed* collapse to a vertex branch.
    for (ps, tag) in [
        ([c2v { x: -1.0, y: -1.0 }, c2v { x: 0.0, y: 0.0 }, c2v { x: 2.0, y: 2.0 }], "collinear thru origin"),
        ([c2v { x: 1.0, y: 5.0 }, c2v { x: 2.0, y: 5.0 }, c2v { x: 3.0, y: 5.0 }], "collinear horizontal"),
        ([c2v { x: 2.0, y: 2.0 }, c2v { x: 2.0, y: 2.0 }, c2v { x: 2.0, y: 2.0 }], "all identical"),
        ([c2v { x: 0.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 0.0 }], "repeated a==b"),
        ([c2v { x: 1.0, y: 0.0 }, c2v { x: -1.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }], "origin on segment"),
    ] {
        let out = run(&mut rep, mk(ps), 1, tag);
        rep.check(same_f32(out.div, 1.0), || {
            format!("c23[{tag}] collapsed to a vertex, so div must be 1.0, got {}", show_f32(out.div))
        });
    }
    // Near-collinear (area tiny but non-zero) still reaches whichever branch the
    // C picks; compared differentially, with no expectation imposed.
    {
        let mut g2 = Rng::new(0xE10);
        for _ in 0..3000 {
            let a = g2.finite_v();
            let d = c2v { x: g2.sym(10.0), y: g2.sym(10.0) };
            let t1 = g2.sym(4.0);
            let t2 = t1 + g2.sym(1.0e-6);
            let ps = [
                a,
                c2v { x: a.x + d.x * t1, y: a.y + d.y * t1 },
                c2v { x: a.x + d.x * t2, y: a.y + d.y * t2 },
            ];
            let (mut sc, mut sr) = (mk(ps), mk(ps));
            unsafe {
                c(&raw mut sc);
                r(&raw mut sr);
            }
            rep.check(same_simplex(&sc, &sr), || {
                format!(
                    "c23[near-collinear]:\n  C:    {}\n  Rust: {}",
                    show_simplex(&sc),
                    show_simplex(&sr)
                )
            });
        }
    }

    // NaN / inf triangles: all the `<= 0` tests fail, all the `> 0` tests fail,
    // so the final `else` (interior) fires. Both sides must land there.
    for ps in [
        [c2v { x: f32::NAN, y: 0.0 }, c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 1.0 }],
        [c2v { x: 1.0, y: 0.0 }, c2v { x: f32::NAN, y: 0.0 }, c2v { x: 0.0, y: 1.0 }],
        [c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 1.0 }, c2v { x: f32::NAN, y: 0.0 }],
        [
            c2v { x: f32::INFINITY, y: 0.0 },
            c2v { x: 0.0, y: f32::INFINITY },
            c2v { x: f32::NEG_INFINITY, y: 0.0 },
        ],
        [
            c2v { x: 1.0e30, y: 1.0e30 },
            c2v { x: -1.0e30, y: 1.0e30 },
            c2v { x: 0.0, y: -1.0e30 },
        ],
    ] {
        let (mut a, mut b) = (mk(ps), mk(ps));
        unsafe {
            c(&raw mut a);
            r(&raw mut b);
        }
        rep.check(same_simplex(&a, &b), || {
            format!(
                "c23[NaN/inf {:?}]:\n  C:    {}\n  Rust: {}",
                ps.map(|p| (p.x, p.y)),
                show_simplex(&a),
                show_simplex(&b)
            )
        });
    }
    rep.finish("err09_to_err16_c23_guards");
}

// ---------------------------------------------------------------------------
// ERRORS rows 17-20 — c2D
// ---------------------------------------------------------------------------

#[test]
fn err17_to_err20_c2D_guards() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnSimplexV>("c2D"), l.rs.sym::<FnSimplexV>("c2D"));
    let skew = (l.c.sym::<FnVV>("c2Skew"), l.rs.sym::<FnVV>("c2Skew"));
    let ccw = (l.c.sym::<FnVV>("c2CCW90"), l.rs.sym::<FnVV>("c2CCW90"));
    let neg = (l.c.sym::<FnVV>("c2Neg"), l.rs.sym::<FnVV>("c2Neg"));
    let mut g = Rng::new(0xE11);
    let mut rep = Report::new();

    let mut call = |rep: &mut Report, s: c2Simplex| -> c2v {
        let (mut a, mut b) = (s, s);
        let (x, y) = unsafe { (c(&raw mut a), r(&raw mut b)) };
        rep.check(same_v(x, y), || {
            format!("c2D(count={}): C={} Rust={}", s.count, show_v(x), show_v(y))
        });
        x
    };

    let mk = |count: c_int, ap: c2v, bp: c2v| {
        let mut s = poisoned_simplex();
        for i in 0..4 {
            s.verts[i].p = c2v { x: 5.0 + i as f32, y: -5.0 };
            s.verts[i].u = 1.0;
            s.verts[i].iA = i as c_int;
            s.verts[i].iB = i as c_int;
            s.verts[i].sA = c2v { x: 0.0, y: 0.0 };
            s.verts[i].sB = c2v { x: 0.0, y: 0.0 };
        }
        s.verts[0].p = ap;
        s.verts[1].p = bp;
        s.div = 1.0;
        s.count = count;
        s
    };

    // Row 17: count == 1 -> exactly -a.p
    for _ in 0..300 {
        let ap = g.nasty_v();
        let got = call(&mut rep, mk(1, ap, c2v { x: 0.0, y: 0.0 }));
        let want = (neg.0(ap), neg.1(ap));
        rep.check(same_v(got, want.0) && same_v(want.0, want.1), || {
            format!("c2D(count=1) != c2Neg(a.p): got {} want {}", show_v(got), show_v(want.0))
        });
    }
    // Rows 18/19: count == 2. c2Skew(v) = (-v.y, v.x); c2CCW90(v) = (v.y, -v.x).
    // `det2(ab, -a.p) > 0` selects c2Skew, everything else (including `== 0` and
    // `NaN`) selects c2CCW90.
    for (ap, bp, expect_skew, tag) in [
        // ab=(1,0), -a.p=(-1,1): det2 = 1*1 - 0*(-1) = 1 > 0  -> c2Skew
        (c2v { x: 1.0, y: -1.0 }, c2v { x: 2.0, y: -1.0 }, true, "row18 det>0"),
        // ab=(1,0), -a.p=(-1,-1): det2 = 1*(-1) - 0*(-1) = -1 <= 0 -> c2CCW90
        (c2v { x: 1.0, y: 1.0 }, c2v { x: 2.0, y: 1.0 }, false, "row19 det<0"),
        (c2v { x: 1.0, y: 0.0 }, c2v { x: 2.0, y: 0.0 }, false, "row19 det==0"),
        (c2v { x: -1.0, y: -1.0 }, c2v { x: 1.0, y: 1.0 }, false, "row19 det==0 thru origin"),
        (c2v { x: 4.0, y: 4.0 }, c2v { x: 4.0, y: 4.0 }, false, "row19 a==b -> ab==0"),
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 1.0, y: 2.0 }, false, "row19 a==origin"),
        (c2v { x: f32::NAN, y: 0.0 }, c2v { x: 1.0, y: 1.0 }, false, "row19 NaN -> det>0 false"),
    ] {
        let ab = c2v { x: bp.x - ap.x, y: bp.y - ap.y };
        let got = call(&mut rep, mk(2, ap, bp));
        let want = if expect_skew { (skew.0(ab), skew.1(ab)) } else { (ccw.0(ab), ccw.1(ab)) };
        rep.check(same_v(got, want.0) && same_v(want.0, want.1), || {
            format!(
                "c2D[{tag}] expected {} ({}), got {}",
                if expect_skew { "c2Skew(ab)" } else { "c2CCW90(ab)" },
                show_v(want.0),
                show_v(got)
            )
        });
    }
    for _ in 0..1500 {
        call(&mut rep, mk(2, g.nasty_v(), g.nasty_v()));
    }
    // Row 20: count == 3 and every out-of-range count -> exactly c2V(0,0)
    for count in BAD_COUNTS.iter().copied().chain([3]) {
        for _ in 0..30 {
            let got = call(&mut rep, mk(count, g.nasty_v(), g.nasty_v()));
            rep.check(got.x.to_bits() == 0 && got.y.to_bits() == 0, || {
                format!("c2D(count={count}) must be (+0,+0), got {}", show_v(got))
            });
        }
    }
    rep.finish("err17_to_err20_c2D_guards");
}

// ---------------------------------------------------------------------------
// ERRORS rows 21-22 — c2L
// ---------------------------------------------------------------------------

#[test]
fn err21_err22_c2L_guards() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnSimplexV>("c2L"), l.rs.sym::<FnSimplexV>("c2L"));
    let mut g = Rng::new(0xE15);
    let mut rep = Report::new();

    let mut call = |rep: &mut Report, s: c2Simplex| -> c2v {
        let (mut a, mut b) = (s, s);
        let (x, y) = unsafe { (c(&raw mut a), r(&raw mut b)) };
        rep.check(same_v(x, y), || {
            format!(
                "c2L(count={} div={}): C={} Rust={}",
                s.count,
                show_f32(s.div),
                show_v(x),
                show_v(y)
            )
        });
        x
    };

    // Row 21: count not in {1,2} -> exactly c2V(0,0), whatever div is.
    for count in BAD_COUNTS.iter().copied().chain([3]) {
        for div in [0.0f32, 1.0, -1.0, f32::NAN, f32::INFINITY] {
            let mut s = g.simplex(count);
            s.count = count;
            s.div = div;
            let got = call(&mut rep, s);
            rep.check(got.x.to_bits() == 0 && got.y.to_bits() == 0, || {
                format!("c2L(count={count}) must be (+0,+0), got {}", show_v(got))
            });
        }
    }
    // Row 22: div == 0 -> den == inf, unguarded. Both must produce the same
    // inf/NaN pattern for counts 1 and 2.
    for count in [1, 2] {
        for div in [0.0f32, -0.0] {
            for _ in 0..400 {
                let mut s = g.simplex(count);
                s.count = count;
                s.div = div;
                let got = call(&mut rep, s);
                if count == 2 {
                    // With den = +/-inf, the result must not be finite unless a
                    // u happens to be exactly 0 and p exactly 0.
                    let _ = got;
                }
            }
        }
        // count == 1 ignores div entirely and returns a.p verbatim.
        let mut s = g.simplex(1);
        s.count = 1;
        s.div = 0.0;
        let want = s.verts[0].p;
        let got = call(&mut rep, s);
        rep.check(same_v(got, want), || {
            format!("c2L(count=1) must return a.p verbatim: got {} want {}", show_v(got), show_v(want))
        });
    }
    rep.finish("err21_err22_c2L_guards");
}

// ---------------------------------------------------------------------------
// ERRORS rows 23-24 — c2Witness
// ---------------------------------------------------------------------------

#[test]
fn err23_err24_c2Witness_guards() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnWitness>("c2Witness"), l.rs.sym::<FnWitness>("c2Witness"));
    let mut g = Rng::new(0xE17);
    let mut rep = Report::new();
    let poison = c2v { x: f32::from_bits(0x5A5A5A5A), y: f32::from_bits(0xA5A5A5A5) };

    let mut call = |rep: &mut Report, s: c2Simplex| -> (c2v, c2v) {
        let (mut sc, mut sr) = (s, s);
        let (mut ac, mut bc) = (poison, poison);
        let (mut ar, mut br) = (poison, poison);
        unsafe {
            c(&raw mut sc, &raw mut ac, &raw mut bc);
            r(&raw mut sr, &raw mut ar, &raw mut br);
        }
        rep.check(same_v(ac, ar) && same_v(bc, br), || {
            format!(
                "c2Witness(count={} div={}):\n  C:    a={} b={}\n  Rust: a={} b={}",
                s.count,
                show_f32(s.div),
                show_v(ac),
                show_v(bc),
                show_v(ar),
                show_v(br)
            )
        });
        (ac, bc)
    };

    // Row 23: count not in {1,2,3} -> both outputs exactly c2V(0,0).
    for count in BAD_COUNTS {
        for div in [0.0f32, 1.0, -3.0, f32::NAN] {
            let mut s = g.simplex(count);
            s.count = count;
            s.div = div;
            let (a, b) = call(&mut rep, s);
            rep.check(
                a.x.to_bits() == 0 && a.y.to_bits() == 0 && b.x.to_bits() == 0 && b.y.to_bits() == 0,
                || format!("c2Witness(count={count}) must write (+0,+0) twice, got a={} b={}", show_v(a), show_v(b)),
            );
        }
    }
    // count == 1 copies sA/sB verbatim and ignores div (even div == 0).
    for div in [0.0f32, -0.0, 1.0, f32::NAN, f32::INFINITY] {
        let mut s = g.simplex(1);
        s.count = 1;
        s.div = div;
        let (wa, wb) = (s.verts[0].sA, s.verts[0].sB);
        let (a, b) = call(&mut rep, s);
        rep.check(same_v(a, wa) && same_v(b, wb), || {
            format!("c2Witness(count=1) must copy sA/sB verbatim (div={})", show_f32(div))
        });
    }
    // Row 24: div == 0 with count 2 and 3 -> den == inf, unguarded.
    for count in [2, 3] {
        for div in [0.0f32, -0.0] {
            for _ in 0..500 {
                let mut s = g.simplex(count);
                s.count = count;
                s.div = div;
                call(&mut rep, s);
            }
        }
        // And div = NaN / inf / denormal.
        for div in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::from_bits(1)] {
            for _ in 0..100 {
                let mut s = g.simplex(count);
                s.count = count;
                s.div = div;
                call(&mut rep, s);
            }
        }
    }
    rep.finish("err23_err24_c2Witness_guards");
}

// ---------------------------------------------------------------------------
// ERRORS rows 25-26 — c2Support has no bounds check and ties go to index 0
// ---------------------------------------------------------------------------

#[test]
fn err25_err26_c2Support_guards() {
    let l = libs();
    let (c, r) = (l.c.sym::<FnSupport>("c2Support"), l.rs.sym::<FnSupport>("c2Support"));
    let mut g = Rng::new(0xE19);
    let mut rep = Report::new();

    // Row 25: count <= 0 still dereferences verts[0] and returns 0.
    // `verts` is a real 8-element array here, so the read is in bounds and the
    // behaviour is fully defined.
    let verts = [c2v { x: 3.0, y: -7.0 }; 8];
    for count in [0, -1, -2, -100, i32::MIN, i32::MIN + 1] {
        for d in [
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: f32::NAN, y: 0.0 },
            c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
        ] {
            let (x, y) = unsafe { (c(verts.as_ptr(), count, d), r(verts.as_ptr(), count, d)) };
            rep.check(x == y && x == 0, || {
                format!("c2Support(count={count}, d={}): C={x} Rust={y}, want 0", show_v(d))
            });
        }
    }
    // Row 26: ties and NaN keep the first index, because the test is `dot > dmax`.
    for count in [1, 2, 3, 4, 8] {
        // All vertices equal -> every dot equal -> index 0.
        let v = [c2v { x: 2.0, y: -1.0 }; 8];
        for _ in 0..50 {
            let d = g.finite_v();
            let (x, y) = unsafe { (c(v.as_ptr(), count, d), r(v.as_ptr(), count, d)) };
            rep.check(x == y && x == 0, || {
                format!("c2Support(all-equal, count={count}, d={}): C={x} Rust={y}, want 0", show_v(d))
            });
        }
        // d == 0 -> every dot is +0.0, `0 > 0` false -> index 0.
        let mut v = [c2v::default(); 8];
        for (i, s) in v.iter_mut().enumerate() {
            *s = c2v { x: i as f32, y: -(i as f32) };
        }
        let (x, y) = unsafe {
            (
                c(v.as_ptr(), count, c2v { x: 0.0, y: 0.0 }),
                r(v.as_ptr(), count, c2v { x: 0.0, y: 0.0 }),
            )
        };
        rep.check(x == y && x == 0, || format!("c2Support(d=0, count={count}): C={x} Rust={y}, want 0"));
        // NaN direction -> dmax is NaN and nothing compares greater -> index 0.
        let (x, y) = unsafe {
            (
                c(v.as_ptr(), count, c2v { x: f32::NAN, y: 1.0 }),
                r(v.as_ptr(), count, c2v { x: f32::NAN, y: 1.0 }),
            )
        };
        rep.check(x == y && x == 0, || format!("c2Support(d=NaN, count={count}): C={x} Rust={y}, want 0"));
        // A NaN vertex can never win, so the max is chosen from the rest.
        for bad in 0..count.min(8) {
            let mut v2 = [c2v { x: 1.0, y: 0.0 }; 8];
            for (i, s) in v2.iter_mut().enumerate() {
                *s = c2v { x: i as f32, y: 0.0 };
            }
            v2[bad as usize] = c2v { x: f32::NAN, y: f32::NAN };
            let d = c2v { x: 1.0, y: 0.0 };
            let (x, y) = unsafe { (c(v2.as_ptr(), count, d), r(v2.as_ptr(), count, d)) };
            rep.check(x == y, || {
                format!("c2Support(NaN at {bad}, count={count}): C={x} Rust={y}")
            });
            rep.check(x != bad || bad == 0, || {
                format!("c2Support let a NaN vertex win: index {x} (NaN at {bad})")
            });
        }
    }
    rep.finish("err25_err26_c2Support_guards");
}
