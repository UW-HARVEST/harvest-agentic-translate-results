//! Phase B rows B57..B61 and Phase C rows E23/E35 for the `c2CastRay`
//! dispatcher (the `C2_TYPE` "mode" flag is the only runtime option in the API).

mod common;
use common::*;

fn both_ct(d: &mut Diff, label: &str, a: c2Ray, buf: &[u8; 20], ty: i32) -> RayResult {
    let (c, r) = apis();
    let rc = call_castray(c, a, buf, ty);
    let rr = call_castray(r, a, buf, ty);
    d.ray(label, || format!("ty={} {:?} buf={:?}", ty, a, buf), rc, rr);
    rc
}

/// B57: `typeB = C2_TYPE_CIRCLE`, and the dispatch must agree with calling
/// `c2RaytoCircle` directly in BOTH libraries.
#[test]
fn b57_dispatch_circle() {
    let (c, r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB57);
    let mut hits = 0;
    for _ in 0..20_000 {
        let a = rng.ray_nice();
        let s = rng.circle_nice();
        let buf = shape_bytes_circle(s);
        let res = both_ct(&mut d, "B57", a, &buf, C2_TYPE_CIRCLE);
        hits += res.0;
        // dispatch == direct call, in each library separately
        let direct_c = call_circle(c, a, s);
        let direct_r = call_circle(r, a, s);
        d.ray("B57/direct-C", || format!("{:?} {:?}", a, s), res, direct_c);
        d.ray(
            "B57/direct-RUST",
            || format!("{:?} {:?}", a, s),
            call_castray(r, a, &buf, C2_TYPE_CIRCLE),
            direct_r,
        );
    }
    assert!(hits > 0, "no circle hits through the dispatcher");
    d.finish("B57 c2CastRay circle");
}

/// B58: `typeB = C2_TYPE_AABB`.
#[test]
fn b58_dispatch_aabb() {
    let (c, r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB58);
    let mut hits = 0;
    for _ in 0..20_000 {
        let a = rng.ray_nice();
        let s = rng.aabb_proper();
        let buf = shape_bytes_aabb(s);
        let res = both_ct(&mut d, "B58", a, &buf, C2_TYPE_AABB);
        hits += res.0;
        d.ray(
            "B58/direct-C",
            || format!("{:?} {:?}", a, s),
            res,
            call_aabb(c, a, s),
        );
        d.ray(
            "B58/direct-RUST",
            || format!("{:?} {:?}", a, s),
            call_castray(r, a, &buf, C2_TYPE_AABB),
            call_aabb(r, a, s),
        );
    }
    assert!(hits > 0, "no AABB hits through the dispatcher");
    d.finish("B58 c2CastRay AABB");
}

/// B59: `typeB = C2_TYPE_CAPSULE`.
#[test]
fn b59_dispatch_capsule() {
    let (c, r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB59);
    let mut hits = 0;
    for _ in 0..20_000 {
        let a = rng.ray_nice();
        let s = rng.capsule_nice();
        let buf = shape_bytes_capsule(s);
        let res = both_ct(&mut d, "B59", a, &buf, C2_TYPE_CAPSULE);
        hits += res.0;
        d.ray(
            "B59/direct-C",
            || format!("{:?} {:?}", a, s),
            res,
            call_capsule(c, a, s),
        );
        d.ray(
            "B59/direct-RUST",
            || format!("{:?} {:?}", a, s),
            call_castray(r, a, &buf, C2_TYPE_CAPSULE),
            call_capsule(r, a, s),
        );
    }
    assert!(hits > 0, "no capsule hits through the dispatcher");
    d.finish("B59 c2CastRay capsule");
}

/// B60: ONE 20-byte buffer reinterpreted under all three `typeB` values.
/// The circle case reads 12 bytes, the AABB 16, the capsule 20 - so this also
/// checks that no library over- or under-reads the shape.
#[test]
fn b60_same_bytes_all_types() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB60);
    for i in 0..20_000 {
        let mut buf = [0u8; 20];
        // Half nice floats, half arbitrary bit patterns.
        for k in 0..5 {
            let v = if i % 2 == 0 {
                rng.nice()
            } else {
                f32::from_bits(rng.next_u32())
            };
            buf[k * 4..k * 4 + 4].copy_from_slice(&v.to_ne_bytes());
        }
        let a = if i % 3 == 0 {
            rng.ray_hostile()
        } else {
            rng.ray_nice()
        };
        for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            both_ct(&mut d, "B60", a, &buf, ty);
        }
    }
    d.finish("B60 c2CastRay shared shape bytes");
}

/// B61: `out` is pre-poisoned; the SAME fields must be written (or left alone)
/// by both libraries on hit and on miss.
#[test]
fn b61_out_write_parity() {
    let (c, r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB61);
    let mut untouched = 0;
    let mut written = 0;
    for i in 0..30_000 {
        let a = rng.ray_nice();
        let (buf, ty) = match i % 3 {
            0 => (shape_bytes_circle(rng.circle_nice()), C2_TYPE_CIRCLE),
            1 => (shape_bytes_aabb(rng.aabb_proper()), C2_TYPE_AABB),
            _ => (shape_bytes_capsule(rng.capsule_nice()), C2_TYPE_CAPSULE),
        };
        let rc = call_castray(c, a, &buf, ty);
        let rr = call_castray(r, a, &buf, ty);
        d.ray("B61", || format!("ty={} {:?}", ty, a), rc, rr);
        // Was `out` touched at all?  Must be the same answer for both.
        let c_touched = !cast_eq(rc.1, POISON);
        let r_touched = !cast_eq(rr.1, POISON);
        d.check(c_touched == r_touched, || {
            format!(
                "out-written mismatch (ty={ty}): C touched={c_touched} RUST touched={r_touched}"
            )
        });
        if c_touched {
            written += 1;
        } else {
            untouched += 1;
        }
    }
    assert!(written > 0 && untouched > 0, "need both write and no-write cases");
    d.finish("B61 c2CastRay out-parameter write parity");
}

/// E23 + E35: out-of-range `C2_TYPE` values.
///
/// The C `switch` has no `default:` and no trailing `return`, so control falls
/// off the end of a non-void function: **undefined behaviour**.  The `-O0`
/// reference artifact never writes `%rax` on that path, so the returned `int`
/// is whatever the caller happened to leave in `%rax` - it is not a function of
/// the arguments and cannot be reproduced by any implementation.  What IS
/// defined and observable is asserted here:
///   * neither library writes to `*out`;
///   * neither library crashes, for any `int` value including `INT_MIN`;
///   * every in-range value (`0`,`1`,`2`) still dispatches identically, and the
///     `ja` in the `-O0` artifact makes the comparison UNSIGNED, so `-1` is
///     rejected exactly like `3`.
#[test]
fn e23_e35_out_of_range_enum() {
    let (c, r) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xE23);
    let mut tys: Vec<i32> = vec![-1, 3, 4, 5, 7, 99, 1000, i32::MIN, i32::MAX, -2, -1000];
    for _ in 0..2000 {
        let v = rng.next_u32() as i32;
        if !(0..=2).contains(&v) {
            tys.push(v);
        }
    }
    for &ty in &tys {
        for _ in 0..4 {
            let a = rng.ray_nice();
            let buf = shape_bytes_capsule(rng.capsule_nice());
            let mut oc = POISON;
            let mut orr = POISON;
            let _rc = unsafe {
                (c.c2CastRay)(
                    a,
                    buf.as_ptr() as *const std::ffi::c_void,
                    ty,
                    &mut oc,
                )
            };
            let _rr = unsafe {
                (r.c2CastRay)(
                    a,
                    buf.as_ptr() as *const std::ffi::c_void,
                    ty,
                    &mut orr,
                )
            };
            // Defined behaviour: `*out` is not touched by either library.
            d.check(cast_eq(oc, POISON), || {
                format!("C wrote to *out for ty={ty}: {}", fmt_cast(oc))
            });
            d.check(cast_eq(orr, POISON), || {
                format!("RUST wrote to *out for ty={ty}: {}", fmt_cast(orr))
            });
            d.check(cast_eq(oc, orr), || {
                format!(
                    "out mismatch for ty={ty}: C={} RUST={}",
                    fmt_cast(oc),
                    fmt_cast(orr)
                )
            });
        }
    }
    // In-range values must still dispatch identically (unsigned compare edge).
    for ty in [0, 1, 2] {
        for _ in 0..2000 {
            let a = rng.ray_nice();
            let buf = shape_bytes_capsule(rng.capsule_nice());
            both_ct(&mut d, "E35/in-range", a, &buf, ty);
        }
    }
    d.finish("E23/E35 c2CastRay out-of-range C2_TYPE");
}
