//! Phase C (part 2) — ERRORS.md rows 27–33 and 38–42: the capsule rejection
//! paths, the unguarded divisions in the vector primitives, and `gen_ray`'s
//! degenerate inputs.

#![allow(non_snake_case)]

mod common;
use common::*;

const SEED: u64 = 0xE770_2A12;

fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn call_capsule(ray: c2Ray, c: c2Capsule) -> (i32, c2Raycast, i32, c2Raycast) {
    both_ray(|l, r, s, o| unsafe { (l.c2RaytoCapsule)(r, s, o) }, ray, c)
}

fn sel_lt(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}
fn sel_abs(a: f32) -> f32 {
    if a < 0.0 { -a } else { a }
}

/// The capsule-local quantities the C computes, via the C library's primitives.
struct CapState {
    yBb: c2v,
    yAp: c2v,
    yAe: c2v,
    in_slab: bool,
    in_cap_a: bool,
    in_cap_b: bool,
    enters_outer_if: bool,
    lateral_inside: bool,
}

fn cap_state(A: c2Ray, B: c2Capsule) -> CapState {
    let l = libs();
    unsafe {
        let My = (l.c.c2Norm)((l.c.c2Sub)(B.b, B.a));
        let M = c2m { x: (l.c.c2CCW90)(My), y: My };
        let cap_n = (l.c.c2Sub)(B.b, B.a);
        let yBb = (l.c.c2MulmvT)(M, cap_n);
        let yAp = (l.c.c2MulmvT)(M, (l.c.c2Sub)(A.p, B.a));
        let yAd = (l.c.c2MulmvT)(M, A.d);
        let yAe = (l.c.c2Add)(yAp, (l.c.c2Mulvs)(yAd, A.t));
        let bb = c2AABB { min: (l.c.c2V)(-B.r, 0.0), max: (l.c.c2V)(B.r, yBb.y) };
        CapState {
            yBb,
            yAp,
            yAe,
            in_slab: (l.c.c2AABBtoPoint)(bb, yAp) != 0,
            in_cap_a: (l.c.c2CircleToPoint)(c2Circle { p: B.a, r: B.r }, A.p) != 0,
            in_cap_b: (l.c.c2CircleToPoint)(c2Circle { p: B.b, r: B.r }, A.p) != 0,
            enters_outer_if: yAe.x * yAp.x < 0.0
                || sel_lt(sel_abs(yAe.x), sel_abs(yAp.x)) < B.r,
            lateral_inside: sel_abs(yAp.x) < B.r,
        }
    }
}

fn rand_capsule(rng: &mut Rng) -> c2Capsule {
    let a = v(rng.coord(), rng.coord());
    let ang = rng.range(0.0, 6.283_185_5);
    let len = rng.range(0.5, 60.0);
    c2Capsule {
        a,
        b: v(a.x + len * ang.cos(), a.y + len * ang.sin()),
        r: rng.range(0.05, 30.0),
    }
}

fn rand_ray(rng: &mut Rng, cap: &c2Capsule) -> c2Ray {
    let l = libs();
    let mid = v((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
    let ang = rng.range(0.0, 6.283_185_5);
    let dist = rng.range(0.0, 150.0);
    let origin = v(mid.x + dist * ang.cos(), mid.y + dist * ang.sin());
    let s = rng.range(-0.6, 1.6);
    let target = v(
        cap.a.x + (cap.b.x - cap.a.x) * s + rng.range(-cap.r * 4.0, cap.r * 4.0),
        cap.a.y + (cap.b.y - cap.a.y) * s + rng.range(-cap.r * 4.0, cap.r * 4.0),
    );
    c2Ray {
        p: origin,
        d: unsafe { (l.c.c2Norm)((l.c.c2Sub)(target, origin)) },
        t: rng.range(0.0, 250.0),
    }
}

/// Row 27 — the final `return 0`: neither disjunct of the outer `if` holds.
/// Crucially, `*out` HAS already been written (`out->n = c2Norm(b-a)`,
/// `out->t = 0` run unconditionally at the top of the function), so this test
/// asserts the written-but-rejected output matches bit-for-bit.
#[test]
fn err_27_capsule_fallthrough_out_written() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 27);
    let mut d = Diff::new("row27 capsule fall-through (out IS written)");
    let mut fired = 0usize;
    for _ in 0..60000 {
        let cap = rand_capsule(&mut rng);
        let ray = rand_ray(&mut rng, &cap);
        let st = cap_state(ray, cap);
        if st.in_slab || st.in_cap_a || st.in_cap_b || st.enters_outer_if {
            continue;
        }
        fired += 1;
        let (cr, co, rr, ro) = call_capsule(ray, cap);
        assert_eq!(cr, 0, "fall-through must return 0");
        assert!(
            !rc_eq(co, POISON),
            "c2RaytoCapsule writes *out unconditionally, so it must NOT be the poison value"
        );
        let expected_n = unsafe { (l.c.c2Norm)((l.c.c2Sub)(cap.b, cap.a)) };
        assert!(
            v_eq(co.n, expected_n) && co.t.to_bits() == 0u32,
            "on fall-through *out must be {{t: +0.0, n: c2Norm(b-a)}}, got {}",
            fmt_rc(co)
        );
        d.check_ray(cr, co, rr, ro, || {
            format!("fall-through yAp={} yAe={}", fmt_v(st.yAp), fmt_v(st.yAe))
        });
        if fired >= 8000 {
            break;
        }
    }
    assert!(fired > 2000, "row27 fired only {fired} times");
    eprintln!("    row27: {fired} fall-through cases, *out written in all of them");
    d.finish();
}

/// Row 28 — origin already inside the axis-aligned slab ⇒ early `return 1` with
/// `out->t == 0` (not a real raycast distance).
#[test]
fn err_28_capsule_origin_inside_slab() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 28);
    let mut d = Diff::new("row28 capsule origin inside slab -> 1, t == 0");
    let mut fired = 0usize;
    for _ in 0..60000 {
        let cap = rand_capsule(&mut rng);
        let axis = v(cap.b.x - cap.a.x, cap.b.y - cap.a.y);
        let alen = (axis.x * axis.x + axis.y * axis.y).sqrt();
        let (ux, uy) = (axis.x / alen, axis.y / alen);
        let s = rng.range(0.0, 1.0);
        let lat = rng.range(-cap.r * 0.98, cap.r * 0.98);
        let ray = c2Ray {
            p: v(cap.a.x + axis.x * s + uy * lat, cap.a.y + axis.y * s - ux * lat),
            d: {
                let a = rng.range(0.0, 6.283_185_5);
                v(a.cos(), a.sin())
            },
            t: rng.range(0.0, 200.0),
        };
        let st = cap_state(ray, cap);
        if !st.in_slab {
            continue;
        }
        fired += 1;
        let (cr, co, rr, ro) = call_capsule(ray, cap);
        assert_eq!(cr, 1, "origin in the slab must return 1");
        assert_eq!(co.t.to_bits(), 0u32, "the early return leaves out->t == +0.0");
        let expected_n = unsafe { (l.c.c2Norm)((l.c.c2Sub)(cap.b, cap.a)) };
        assert!(v_eq(co.n, expected_n), "out->n must be c2Norm(b-a)");
        d.check_ray(cr, co, rr, ro, || format!("slab yAp={}", fmt_v(st.yAp)));
        if fired >= 8000 {
            break;
        }
    }
    assert!(fired > 2000, "row28 fired only {fired} times");
    eprintln!("    row28: {fired} origin-in-slab cases");
    d.finish();
}

/// Row 29 — origin strictly inside end-cap A or end-cap B ⇒ early `return 1`.
#[test]
fn err_29_capsule_origin_in_endcap() {
    let mut rng = Rng::new(SEED ^ 29);
    let mut d = Diff::new("row29 capsule origin in an end cap -> 1");
    let (mut na, mut nb) = (0usize, 0usize);
    for _ in 0..120000 {
        let cap = rand_capsule(&mut rng);
        // Just beyond an end, but within r of it: outside the slab, inside a cap.
        let axis = v(cap.b.x - cap.a.x, cap.b.y - cap.a.y);
        let alen = (axis.x * axis.x + axis.y * axis.y).sqrt();
        let (ux, uy) = (axis.x / alen, axis.y / alen);
        let which_a = rng.bool();
        let base = if which_a { cap.a } else { cap.b };
        let outward = if which_a { -1.0f32 } else { 1.0 };
        let along = rng.range(0.001, cap.r * 0.99) * outward;
        let lat = rng.range(-cap.r * 0.5, cap.r * 0.5);
        let ray = c2Ray {
            p: v(base.x + ux * along + uy * lat, base.y + uy * along - ux * lat),
            d: {
                let a = rng.range(0.0, 6.283_185_5);
                v(a.cos(), a.sin())
            },
            t: rng.range(0.0, 200.0),
        };
        let st = cap_state(ray, cap);
        if st.in_slab {
            continue;
        }
        let is_a = st.in_cap_a;
        let is_b = !st.in_cap_a && st.in_cap_b;
        if !is_a && !is_b {
            continue;
        }
        if is_a {
            na += 1;
        } else {
            nb += 1;
        }
        let (cr, co, rr, ro) = call_capsule(ray, cap);
        assert_eq!(cr, 1, "origin in an end cap must return 1");
        assert_eq!(co.t.to_bits(), 0u32, "the early return leaves out->t == +0.0");
        d.check_ray(cr, co, rr, ro, || {
            format!("endcap {} yAp={}", if is_a { "A" } else { "B" }, fmt_v(st.yAp))
        });
        if na >= 4000 && nb >= 4000 {
            break;
        }
    }
    assert!(na > 500 && nb > 500, "row29 cap A fired {na}, cap B fired {nb}");
    eprintln!("    row29: {na} cap-A cases, {nb} cap-B cases");
    d.finish();
}

/// Row 30 — `B.r == 0`: the slab degenerates to a segment, `min(|yAe.x|,|yAp.x|)
/// < 0` can never hold, so only the sign-change disjunct can fire.
#[test]
fn err_30_capsule_zero_radius() {
    let mut rng = Rng::new(SEED ^ 30);
    let mut d = Diff::new("row30 capsule r == 0");
    let mut sign_change = 0usize;
    let mut fell_through = 0usize;
    for _ in 0..20000 {
        let base = rand_capsule(&mut rng);
        for r in [0.0f32, -0.0f32] {
            let cap = c2Capsule { a: base.a, b: base.b, r };
            let ray = rand_ray(&mut rng, &base);
            let st = cap_state(ray, cap);
            // With r == 0 the second disjunct `min(...) < 0` is unreachable.
            assert!(
                !(sel_lt(sel_abs(st.yAe.x), sel_abs(st.yAp.x)) < r),
                "with r == 0 the `min(|yAe.x|,|yAp.x|) < r` disjunct must be false"
            );
            if st.enters_outer_if {
                sign_change += 1;
            } else {
                fell_through += 1;
            }
            let (cr, co, rr, ro) = call_capsule(ray, cap);
            d.check_ray(cr, co, rr, ro, || {
                format!("r={} yAp={} yAe={}", fmt_f(r), fmt_v(st.yAp), fmt_v(st.yAe))
            });
        }
    }
    assert!(
        sign_change > 100 && fell_through > 100,
        "row30 coverage: {sign_change} entered / {fell_through} fell through"
    );
    eprintln!("    row30: {sign_change} entered the outer if, {fell_through} fell through");
    d.finish();
}

/// Row 31 — `B.a == B.b`: `c2Norm` of a zero vector makes the whole basis `M`
/// NaN, so `yAp`/`yBb` are NaN.
///
/// NOTE: the ERRORS.md prediction of `return 0` was wrong, and the C is ground
/// truth. With a NaN `yAp` the slab test `c2AABBtoPoint(capsule_bb, yAp)` has all
/// four `<`/`>` comparisons false, so it returns 1 and `c2RaytoCapsule` takes the
/// EARLY `return 1` with `out->t == 0`. This test asserts the branch the C
/// actually takes and that the Rust matches it bit-for-bit.
#[test]
fn err_31_capsule_degenerate_axis() {
    let mut rng = Rng::new(SEED ^ 31);
    let mut d = Diff::new("row31 capsule a == b -> NaN basis");
    let mut nan_basis = 0usize;
    let mut early_one = 0usize;
    for _ in 0..8000 {
        let p = v(rng.coord(), rng.coord());
        for (a, b) in [
            (p, p),
            (v(0.0, 0.0), v(0.0, 0.0)),
            (v(0.0, 0.0), v(-0.0, -0.0)),
            (v(-0.0, 0.0), v(0.0, -0.0)),
            (v(1e30, 1e30), v(1e30, 1e30)),
        ] {
            let cap = c2Capsule { a, b, r: rng.range(0.0, 20.0) };
            let ray = c2Ray {
                p: v(rng.coord(), rng.coord()),
                d: {
                    let ang = rng.range(0.0, 6.283_185_5);
                    v(ang.cos(), ang.sin())
                },
                t: rng.range(0.0, 200.0),
            };
            let st = cap_state(ray, cap);
            let (cr, co, rr, ro) = call_capsule(ray, cap);
            if st.yAp.x.is_nan() || st.yAp.y.is_nan() {
                nan_basis += 1;
                // A NaN yAp makes every comparison in c2AABBtoPoint false, so the
                // slab test returns 1 and the function returns 1 immediately.
                assert!(
                    st.in_slab,
                    "a NaN yAp must make c2AABBtoPoint return 1 (all `<`/`>` false)"
                );
                assert_eq!(cr, 1, "the NaN slab test causes the early `return 1`");
                assert_eq!(co.t.to_bits(), 0u32, "the early return leaves out->t == +0.0");
                assert!(
                    co.n.x.is_nan() && co.n.y.is_nan(),
                    "out->n is c2Norm(b - a) == c2Norm(0,0) == (NaN, NaN), got {}",
                    fmt_v(co.n)
                );
                early_one += 1;
            }
            d.check_ray(cr, co, rr, ro, || {
                format!("a==b yBb={} yAp={}", fmt_v(st.yBb), fmt_v(st.yAp))
            });
        }
    }
    assert!(nan_basis > 1000, "row31 fired only {nan_basis} times");
    eprintln!(
        "    row31: {nan_basis} degenerate-axis cases, {early_one} took the NaN early return 1"
    );
    d.finish();
}

/// Row 32 — the UNGUARDED division `t = (c - yAp.x) / (yAe.x - yAp.x)` in the
/// capsule's else-branch.
///
/// NOTE: the ERRORS.md prediction that this divides by zero was wrong. The
/// else-branch is only reached when `|yAp.x| >= B.r` AND the outer `if` held,
/// which forces `yAe.x != yAp.x`:
///
/// * if `yAe.x * yAp.x < 0` the two have strict opposite signs, so they differ;
/// * otherwise `min(|yAe.x|, |yAp.x|) < B.r <= |yAp.x|`, so the minimum must be
///   `|yAe.x|`, hence `|yAe.x| < |yAp.x|` and again they differ;
/// * a NaN in either makes both disjuncts false (the C's ternary `min` returns
///   its second operand for a NaN first operand), so the branch is not entered.
///
/// So `d == 0` is UNREACHABLE. This test proves that empirically over a sweep
/// that deliberately includes axis-parallel rays (where `yAe.x == yAp.x`), and
/// still differentially checks every case.
#[test]
fn err_32_capsule_division_denominator_never_zero() {
    let mut rng = Rng::new(SEED ^ 32);
    let mut d = Diff::new("row32 capsule else-branch denominator");
    let mut else_branch = 0usize;
    let mut parallel_seen = 0usize;
    let mut zero_denominator = 0usize;

    for _ in 0..40000 {
        let cap = rand_capsule(&mut rng);
        let axis = v(cap.b.x - cap.a.x, cap.b.y - cap.a.y);
        let alen = (axis.x * axis.x + axis.y * axis.y).sqrt();
        let (ux, uy) = (axis.x / alen, axis.y / alen);

        let mut rays = Vec::new();
        // Axis-parallel rays: their lateral travel is exactly 0, so
        // yAe.x == yAp.x -- the only way d could be 0.
        for sign in [1.0f32, -1.0] {
            for latmul in [0.5f32, 1.0, 1.05, 3.0, 12.0] {
                let lat = cap.r * latmul * if rng.bool() { 1.0 } else { -1.0 };
                let s = rng.range(-2.0, 3.0);
                rays.push(c2Ray {
                    p: v(cap.a.x + axis.x * s + uy * lat, cap.a.y + axis.y * s - ux * lat),
                    d: v(ux * sign, uy * sign),
                    t: rng.range(0.0, 200.0),
                });
            }
        }
        // Zero-length rays: yAe == yAp exactly, so d == 0 if reached.
        rays.push(c2Ray {
            p: v(cap.a.x + uy * cap.r * 2.0, cap.a.y - ux * cap.r * 2.0),
            d: v(ux, uy),
            t: 0.0,
        });
        // A general ray for good measure.
        rays.push(rand_ray(&mut rng, &cap));

        for ray in rays {
            let st = cap_state(ray, cap);
            if st.yAe.x - st.yAp.x == 0.0 {
                parallel_seen += 1;
            }
            let reaches_else = !st.in_slab
                && !st.in_cap_a
                && !st.in_cap_b
                && st.enters_outer_if
                && !st.lateral_inside;
            if reaches_else {
                else_branch += 1;
                let denom = st.yAe.x - st.yAp.x;
                if denom == 0.0 {
                    zero_denominator += 1;
                }
            }
            let (cr, co, rr, ro) = call_capsule(ray, cap);
            d.check_ray(cr, co, rr, ro, || {
                format!(
                    "yAp.x={} yAe.x={} r={} reaches_else={reaches_else}",
                    fmt_f(st.yAp.x),
                    fmt_f(st.yAe.x),
                    fmt_f(cap.r)
                )
            });
        }
    }
    assert!(
        else_branch > 2000,
        "the else-branch was reached only {else_branch} times; the test proves nothing"
    );
    assert!(
        parallel_seen > 2000,
        "only {parallel_seen} cases had yAe.x == yAp.x; the /0 candidate was not probed"
    );
    assert_eq!(
        zero_denominator, 0,
        "found {zero_denominator} reachable zero denominators -- the unreachability \
         argument in ERRORS.md row 32 is wrong and the Rust needs an explicit check"
    );
    eprintln!(
        "    row32: {else_branch} else-branch entries, {parallel_seen} with yAe.x == yAp.x, \
         0 reachable zero denominators"
    );
    d.finish();
}

/// Row 33 — `|yAp.x| < B.r` ⇒ the capsule DELEGATES to `c2RaytoCircle`, so it
/// inherits rows 1–3. The test asserts the delegation is exact: the capsule's
/// result equals a direct `c2RaytoCircle` call on the selected end cap, in both
/// libraries.
#[test]
fn err_33_capsule_delegates_to_circle() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 33);
    let mut d = Diff::new("row33 capsule delegates to c2RaytoCircle");
    let (mut na, mut nb) = (0usize, 0usize);
    let mut delegated_misses = 0usize;
    for _ in 0..120000 {
        let cap = rand_capsule(&mut rng);
        let axis = v(cap.b.x - cap.a.x, cap.b.y - cap.a.y);
        let alen = (axis.x * axis.x + axis.y * axis.y).sqrt();
        let (ux, uy) = (axis.x / alen, axis.y / alen);
        // Laterally inside r, axially outside the slab and outside both caps.
        let along = if rng.bool() {
            rng.range(-4.0, -1.01)
        } else {
            rng.range(1.01, 4.0)
        } * alen;
        let lat = rng.range(-cap.r * 0.99, cap.r * 0.99);
        let ray = c2Ray {
            p: v(cap.a.x + ux * along + uy * lat, cap.a.y + uy * along - ux * lat),
            d: {
                let a = rng.range(0.0, 6.283_185_5);
                v(a.cos(), a.sin())
            },
            t: rng.range(0.0, 300.0),
        };
        let st = cap_state(ray, cap);
        if st.in_slab || st.in_cap_a || st.in_cap_b || !st.enters_outer_if || !st.lateral_inside {
            continue;
        }
        let use_a = st.yAp.y < 0.0;
        if use_a {
            na += 1;
        } else {
            nb += 1;
        }
        let (cr, co, rr, ro) = call_capsule(ray, cap);
        d.check_ray(cr, co, rr, ro, || {
            format!("delegate to cap {} yAp={}", if use_a { "A" } else { "B" }, fmt_v(st.yAp))
        });

        // The delegation must be byte-identical to calling c2RaytoCircle directly
        // on the chosen cap, in each library independently.
        let circ = c2Circle {
            p: if use_a { cap.a } else { cap.b },
            r: cap.r,
        };
        // c2RaytoCapsule pre-writes *out, so seed the direct call the same way.
        let mut dc = c2Raycast {
            t: 0.0,
            n: unsafe { (l.c.c2Norm)((l.c.c2Sub)(cap.b, cap.a)) },
        };
        let mut dr = dc;
        let dcr = unsafe { (l.c.c2RaytoCircle)(ray, circ, &mut dc) };
        let drr = unsafe { (l.r.c2RaytoCircle)(ray, circ, &mut dr) };
        d.check(cr == dcr && rc_eq(co, dc), || {
            format!(
                "C: capsule delegation != direct c2RaytoCircle\n    capsule -> ret={cr} {}\n    circle  -> ret={dcr} {}",
                fmt_rc(co),
                fmt_rc(dc)
            )
        });
        d.check(rr == drr && rc_eq(ro, dr), || {
            "Rust: capsule delegation != direct c2RaytoCircle".into()
        });
        if dcr == 0 {
            delegated_misses += 1;
        }
        if na >= 4000 && nb >= 4000 {
            break;
        }
    }
    assert!(na > 300 && nb > 300, "row33 cap A {na}, cap B {nb}");
    assert!(
        delegated_misses > 100,
        "the delegated c2RaytoCircle never rejected ({delegated_misses}); \
         rows 1-3 are not actually inherited here"
    );
    eprintln!(
        "    row33: {na} -> cap A, {nb} -> cap B, {delegated_misses} delegated rejections"
    );
    d.finish();
}

// ===========================================================================
// rows 38-39 — unguarded division / sqrt in the primitives
// ===========================================================================

/// Row 38 — `c2Div` / `c2Norm` with a zero divisor. `1.0f/0.0f` is `inf`, then
/// `0*inf` is NaN, so `c2Norm(0,0) == (NaN, NaN)` and `c2Norm(0,5) == (NaN, inf)`.
/// The exact NaN/inf bits must match.
#[test]
fn err_38_norm_zero_vector() {
    let l = libs();
    let mut d = Diff::new("row38 c2Div/c2Norm zero divisor");

    let zero_cases = [
        v(0.0, 0.0),
        v(-0.0, -0.0),
        v(0.0, -0.0),
        v(-0.0, 0.0),
    ];
    for a in zero_cases {
        let cn = unsafe { (l.c.c2Norm)(a) };
        let rn = unsafe { (l.r.c2Norm)(a) };
        assert!(
            cn.x.is_nan() && cn.y.is_nan(),
            "c2Norm of a zero vector must be (NaN, NaN), got {}",
            fmt_v(cn)
        );
        d.check_v(cn, rn, || format!("c2Norm({})", fmt_v(a)));
    }
    // One zero component: 0*inf == NaN, nonzero*inf == inf.
    for a in [v(0.0, 5.0), v(5.0, 0.0), v(-0.0, 5.0), v(5.0, -0.0), v(0.0, -5.0)] {
        let cn = unsafe { (l.c.c2Norm)(a) };
        let rn = unsafe { (l.r.c2Norm)(a) };
        d.check_v(cn, rn, || format!("c2Norm({})", fmt_v(a)));
    }
    // c2Div directly with every zero and infinite divisor.
    for b in [0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        for a in [
            v(0.0, 0.0),
            v(-0.0, 0.0),
            v(1.0, -1.0),
            v(f32::INFINITY, 0.0),
            v(f32::NAN, 1.0),
            v(f32::MAX, f32::MIN),
        ] {
            let cv = unsafe { (l.c.c2Div)(a, b) };
            let rv = unsafe { (l.r.c2Div)(a, b) };
            d.check_v(cv, rv, || format!("c2Div({}, {})", fmt_v(a), fmt_f(b)));
        }
    }
    // Huge vector: c2Len overflows to inf, so 1/inf == 0 and the result is 0.
    for a in [v(1e30, 1e30), v(f32::MAX, f32::MAX), v(3.0e38, 3.0e38)] {
        let cn = unsafe { (l.c.c2Norm)(a) };
        let rn = unsafe { (l.r.c2Norm)(a) };
        d.check_v(cn, rn, || format!("c2Norm(huge {})", fmt_v(a)));
    }
    // Tiny vector: c2Dot underflows to 0, so c2Len == 0 and we are back in the
    // zero-divisor case despite a nonzero input.
    for a in [v(1e-30, 1e-30), v(1e-45, 1e-45), v(f32::MIN_POSITIVE, 0.0)] {
        let cn = unsafe { (l.c.c2Norm)(a) };
        let rn = unsafe { (l.r.c2Norm)(a) };
        d.check_v(cn, rn, || format!("c2Norm(tiny {})", fmt_v(a)));
    }
    d.finish();
}

/// Row 39 — `c2Len` of a NaN vector: `sqrtf` of a NaN dot. Sign and payload must
/// match bit-for-bit. Also `sqrtf` of a negative dot, which cannot arise from
/// finite input but can from `inf` components (`inf + -inf == NaN`).
#[test]
fn err_39_len_nan() {
    let l = libs();
    let mut d = Diff::new("row39 c2Len NaN / sqrt of NaN");
    for &nb in NAN_BITS {
        let s = f32::from_bits(nb);
        for a in [v(s, 0.0), v(0.0, s), v(s, s), v(s, 1.0), v(1.0, s)] {
            let cf = unsafe { (l.c.c2Len)(a) };
            let rf = unsafe { (l.r.c2Len)(a) };
            assert!(cf.is_nan(), "c2Len of a NaN vector must be NaN");
            d.check_f(cf, rf, || format!("c2Len({})", fmt_v(a)));
        }
    }
    // inf components: dot(a,a) == inf + inf == inf, so c2Len == inf.
    for a in [
        v(f32::INFINITY, 0.0),
        v(f32::NEG_INFINITY, 0.0),
        v(f32::INFINITY, f32::NEG_INFINITY),
        v(f32::INFINITY, f32::INFINITY),
    ] {
        let cf = unsafe { (l.c.c2Len)(a) };
        let rf = unsafe { (l.r.c2Len)(a) };
        d.check_f(cf, rf, || format!("c2Len({})", fmt_v(a)));
    }
    // inf * 0 inside the dot product -> NaN.
    for a in [v(f32::INFINITY, 0.0), v(0.0, f32::INFINITY)] {
        let cf = unsafe { (l.c.c2Dot)(a, v(0.0, 0.0)) };
        let rf = unsafe { (l.r.c2Dot)(a, v(0.0, 0.0)) };
        d.check_f(cf, rf, || format!("c2Dot({}, 0)", fmt_v(a)));
    }
    d.finish();
}

// ===========================================================================
// rows 40-42 — gen_ray
// ===========================================================================

#[allow(clippy::too_many_arguments)]
fn gen_ray_both(p: [f32; 16]) -> (i32, [c2Raycast; 3], i32, [c2Raycast; 3]) {
    let l = libs();
    let mut co = [POISON; 3];
    let mut ro = [POISON; 3];
    let cr = unsafe {
        (l.c.gen_ray)(
            &mut co[0], &mut co[1], &mut co[2], p[0], p[1], p[2], p[3], p[4],
            p[5], p[6], p[7], p[8], p[9], p[10], p[11], p[12], p[13], p[14], p[15],
        )
    };
    let rr = unsafe {
        (l.r.gen_ray)(
            &mut ro[0], &mut ro[1], &mut ro[2], p[0], p[1], p[2], p[3], p[4],
            p[5], p[6], p[7], p[8], p[9], p[10], p[11], p[12], p[13], p[14], p[15],
        )
    };
    (cr, co, rr, ro)
}

fn gen_check(d: &mut Diff, p: [f32; 16]) -> (i32, [c2Raycast; 3]) {
    let (cr, co, rr, ro) = gen_ray_both(p);
    let ok = cr == rr
        && rc_eq(co[0], ro[0])
        && rc_eq(co[1], ro[1])
        && rc_eq(co[2], ro[2]);
    d.check(ok, || {
        format!(
            "gen_ray({p:?})\n    C   -> {cr} {} {} {}\n    Rust-> {rr} {} {} {}",
            fmt_rc(co[0]), fmt_rc(co[1]), fmt_rc(co[2]),
            fmt_rc(ro[0]), fmt_rc(ro[1]), fmt_rc(ro[2])
        )
    });
    (cr, co)
}

/// Row 40 — `mp == ray.p` ⇒ `c2Norm(0)` ⇒ NaN `ray.d` and NaN `ray.t`. All three
/// shapes then run with a NaN ray.
///
/// NOTE: the ERRORS.md prediction of a blanket `ret == 0` was wrong, and the C is
/// ground truth. The capsule's slab test uses `yAp`, which depends only on `A.p`
/// and the (finite) capsule basis — not on the NaN direction — so when the ray
/// ORIGIN happens to lie in the capsule's slab or an end cap, the capsule leg
/// still returns 1 and `gen_ray` returns 2. The circle and box legs do always
/// miss, because a NaN `A.d` makes their `t`/`tN` NaN.
#[test]
fn err_40_gen_ray_degenerate_ray() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 40);
    let mut d = Diff::new("row40 gen_ray degenerate ray (mp == ray.p)");
    let mut fired = 0usize;
    let mut ret_hist = [0usize; 8];
    for _ in 0..8000 {
        let px = rng.coord();
        let py = rng.coord();
        let p: [f32; 16] = [
            px, py, px, py, // mp == ray.p exactly
            rng.coord(), rng.coord(), rng.range(0.1, 30.0),
            rng.coord(), rng.coord(), rng.coord(), rng.coord(), rng.range(0.1, 20.0),
            rng.coord(), rng.coord(), rng.coord(), rng.coord(),
        ];
        let (cr, co) = gen_check(&mut d, p);
        fired += 1;
        if (0..8).contains(&cr) {
            ret_hist[cr as usize] += 1;
        }
        // The ray direction really is NaN.
        let dir = unsafe { (l.c.c2Norm)((l.c.c2Sub)(v(px, py), v(px, py))) };
        assert!(dir.x.is_nan() && dir.y.is_nan(), "ray.d must be NaN here");
        // Only the CIRCLE leg is guaranteed to miss: `t = -b - sqrtf(disc)` is
        // NaN, so `t >= 0` is false. The BOX leg can still "hit": a NaN `p1`
        // makes the ray bbox NaN, `c2AABBtoAABB` then returns 1 (all `<` false),
        // the SAT `d` is NaN so `d > 0` is false, and `p0` is finite so at least
        // one plane has `da < 0` ⇒ `tN == 0` ⇒ `hit`. The capsule leg can hit via
        // its slab test, which also only depends on `A.p`.
        assert_eq!(
            cr & 0b001,
            0,
            "a NaN direction must make the circle leg miss, got ret={cr}"
        );
        assert!(
            rc_eq(co[0], POISON),
            "cast1 must be untouched (the circle leg cannot hit here)"
        );
        // cast2 is always written (the capsule pre-writes it unconditionally).
        assert!(
            !rc_eq(co[1], POISON),
            "the capsule leg writes cast2 unconditionally"
        );
        // cast3 is written exactly when the box leg reported a hit.
        assert_eq!(
            (cr & 0b100 != 0),
            !rc_eq(co[2], POISON),
            "cast3 written-ness must track the box hit bit"
        );
    }
    // The signed-zero flavours of coincidence.
    for _ in 0..2000 {
        let mut p = [0f32; 16];
        for i in 0..16 {
            p[i] = rng.coord();
        }
        p[0] = 0.0;
        p[2] = -0.0;
        p[1] = -0.0;
        p[3] = 0.0;
        gen_check(&mut d, p);
    }
    assert!(fired > 5000);
    // Only even return codes are achievable: bit 0 (circle) can never be set.
    for odd in [1usize, 3, 5, 7] {
        assert_eq!(
            ret_hist[odd], 0,
            "return code {odd} implies a circle hit, which is impossible with a \
             NaN direction (histogram {ret_hist:?})"
        );
    }
    assert!(
        ret_hist[0] > 50 && ret_hist[2] > 0 && ret_hist[4] > 100 && ret_hist[6] > 0,
        "expected all four achievable codes {{0, 2, 4, 6}}, got {ret_hist:?}"
    );
    eprintln!("    row40: {fired} degenerate-ray cases, return histogram {ret_hist:?}");
    d.finish();
}

/// Row 41 — null out-params: `cast1` and `cast3` are safe when their shape
/// misses, `cast2` is not (tested for SIGSEGV parity in `phase_c_crash.rs`).
/// This covers the SAFE subset in-process.
#[test]
fn err_41_gen_ray_null_outs_safe_subset() {
    let l = libs();
    let mut d = Diff::new("row41 gen_ray null cast1/cast3 with a missing shape");
    let mut cast2 = POISON;
    let mut cast2r = POISON;
    // Circle and box parked far off the ray so their legs return before writing.
    let args: [f32; 16] = [
        10.0, 0.0, -10.0, 0.0,       // ray along +x through the origin
        0.0, 1.0e6, 1.0,             // circle far above
        0.0, -5.0, 0.0, 5.0, 2.0,    // capsule across the ray
        0.0, 1.0e6, 1.0, 1.0e6 + 1.0 // box far above
    ];
    let cr = unsafe {
        (l.c.gen_ray)(
            std::ptr::null_mut(), &mut cast2, std::ptr::null_mut(),
            args[0], args[1], args[2], args[3], args[4], args[5], args[6],
            args[7], args[8], args[9], args[10], args[11],
            args[12], args[13], args[14], args[15],
        )
    };
    let rr = unsafe {
        (l.r.gen_ray)(
            std::ptr::null_mut(), &mut cast2r, std::ptr::null_mut(),
            args[0], args[1], args[2], args[3], args[4], args[5], args[6],
            args[7], args[8], args[9], args[10], args[11],
            args[12], args[13], args[14], args[15],
        )
    };
    d.check_i(cr, rr, || "gen_ray(cast1=NULL, cast3=NULL)".into());
    d.check(rc_eq(cast2, cast2r), || {
        format!("cast2 mismatch: C {} vs Rust {}", fmt_rc(cast2), fmt_rc(cast2r))
    });
    assert_eq!(cr & 0b101, 0, "the circle and box legs must have missed");
    eprintln!("    row41: gen_ray with null cast1/cast3 returned {cr} from both");
    d.finish();
}

/// Row 42 — inverted `bb`: never validated, flows straight into rows 10 and 15.
#[test]
fn err_42_gen_ray_inverted_bb() {
    let mut rng = Rng::new(SEED ^ 42);
    let mut d = Diff::new("row42 gen_ray inverted bb");
    let mut hit_box = 0usize;
    for _ in 0..20000 {
        let mut p = [0f32; 16];
        p[0] = rng.coord();
        p[1] = rng.coord();
        p[2] = rng.coord();
        p[3] = rng.coord();
        p[4] = rng.coord();
        p[5] = rng.coord();
        p[6] = rng.range(0.1, 20.0);
        p[7] = rng.coord();
        p[8] = rng.coord();
        p[9] = rng.coord();
        p[10] = rng.coord();
        p[11] = rng.range(0.1, 20.0);
        let bx = rng.coord();
        let by = rng.coord();
        let w = rng.range(0.1, 40.0);
        let h = rng.range(0.1, 40.0);
        // min > max on both axes.
        p[12] = bx + w;
        p[13] = by + h;
        p[14] = bx;
        p[15] = by;
        let (cr, _) = gen_check(&mut d, p);
        if cr & 0b100 != 0 {
            hit_box += 1;
        }
        // Inverted on one axis only.
        p[12] = bx + w;
        p[14] = bx;
        p[13] = by;
        p[15] = by + h;
        gen_check(&mut d, p);
        // Degenerate (min == max).
        p[12] = bx;
        p[13] = by;
        p[14] = bx;
        p[15] = by;
        gen_check(&mut d, p);
    }
    eprintln!("    row42: the inverted box reported a hit in {hit_box} cases");
    d.finish();
}
