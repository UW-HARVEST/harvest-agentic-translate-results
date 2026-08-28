//! Scratch probes used while deriving ERRORS.md (kept so the findings can be
//! re-checked).  These print information and assert only what is reproducible.
//!
//! Run with `cargo test --release --test probe_ub -- --nocapture`.

mod common;
use common::*;

/// Search for a configuration that reaches the `iter == 20` cap in `c2GJK`
/// (ERRORS.md E46) and for one that reaches the `a == b => dist = 0` re-check
/// after the radius shrink (E43).
#[test]
fn probe_loop_and_radius_coverage() {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(0x5EA4C4);
    let mut max_it = 0i32;
    let mut best: Option<String> = None;
    let mut forced_zero = 0u32;
    for i in 0..2_000_000u32 {
        let ta = ALL_TYPES[rng.below(3) as usize];
        let tb = ALL_TYPES[rng.below(3) as usize];
        let sa = match ta {
            C2_TYPE_CIRCLE => ShapeBlob::circle(C2Circle {
                p: rng.v_spicy(),
                r: rng.spicy(),
            }),
            C2_TYPE_AABB => ShapeBlob::aabb(C2AABB {
                min: rng.v_spicy(),
                max: rng.v_spicy(),
            }),
            _ => ShapeBlob::capsule(C2Capsule {
                a: rng.v_spicy(),
                b: rng.v_spicy(),
                r: rng.spicy(),
            }),
        };
        let sb = match tb {
            C2_TYPE_CIRCLE => ShapeBlob::circle(C2Circle {
                p: rng.v_spicy(),
                r: rng.spicy(),
            }),
            C2_TYPE_AABB => ShapeBlob::aabb(C2AABB {
                min: rng.v_spicy(),
                max: rng.v_spicy(),
            }),
            _ => ShapeBlob::capsule(C2Capsule {
                a: rng.v_spicy(),
                b: rng.v_spicy(),
                r: rng.spicy(),
            }),
        };
        let ax = rng.x_spicy();
        let bx = rng.x_spicy();
        let (use_ax, use_bx) = (rng.bool(), rng.bool());
        let mut oa = C2v::default();
        let mut ob = C2v::default();
        let mut it = -1i32;
        let dc = unsafe {
            c(
                sa.as_ptr(),
                ta,
                if use_ax { &ax } else { std::ptr::null() },
                sb.as_ptr(),
                tb,
                if use_bx { &bx } else { std::ptr::null() },
                &mut oa,
                &mut ob,
                1,
                &mut it,
                std::ptr::null_mut(),
            )
        };
        let (ca, cb, ci) = (oa, ob, it);
        let dr = unsafe {
            r(
                sa.as_ptr(),
                ta,
                if use_ax { &ax } else { std::ptr::null() },
                sb.as_ptr(),
                tb,
                if use_bx { &bx } else { std::ptr::null() },
                &mut oa,
                &mut ob,
                1,
                &mut it,
                std::ptr::null_mut(),
            )
        };
        assert!(
            f32_same(dc, dr) && v_same(ca, oa) && v_same(cb, ob) && ci == it,
            "divergence at #{i}"
        );
        if ci > max_it {
            max_it = ci;
            best = Some(format!(
                "#{i} ta={} tb={} it={ci} dist={} ax={use_ax} bx={use_bx}",
                type_name(ta),
                type_name(tb),
                fmt_f32(dc)
            ));
        }
        if f32_same(dc, 0.0) && v_same(ca, cb) && ca.x.is_finite() {
            forced_zero += 1;
        }
    }
    println!("max iterations found = {max_it}  ({best:?})");
    println!("dist==0 with a==b (finite): {forced_zero}");
}

/// ERRORS.md E43: after the radius shrink the C re-tests `a.x == b.x &&
/// a.y == b.y` and forces `dist` to 0.  Construct that exactly: with
/// `rA = FLT_MAX` and `rB = -FLT_MAX` the sum is `0` (so `dist > rA + rB`
/// holds), and both witness points saturate to `FLT_MAX`.
#[test]
fn probe_forced_zero_after_shrink() {
    let (c, r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let a = ShapeBlob::circle(C2Circle {
        p: C2v { x: 0.0, y: 0.0 },
        r: FLT_MAX,
    });
    let b = ShapeBlob::circle(C2Circle {
        p: C2v { x: 100.0, y: 0.0 },
        r: -FLT_MAX,
    });
    let mut oa = C2v::default();
    let mut ob = C2v::default();
    let mut it = -1i32;
    let dc = unsafe {
        c(
            a.as_ptr(),
            C2_TYPE_CIRCLE,
            std::ptr::null(),
            b.as_ptr(),
            C2_TYPE_CIRCLE,
            std::ptr::null(),
            &mut oa,
            &mut ob,
            1,
            &mut it,
            std::ptr::null_mut(),
        )
    };
    let (ca, cb) = (oa, ob);
    let dr = unsafe {
        r(
            a.as_ptr(),
            C2_TYPE_CIRCLE,
            std::ptr::null(),
            b.as_ptr(),
            C2_TYPE_CIRCLE,
            std::ptr::null(),
            &mut oa,
            &mut ob,
            1,
            &mut it,
            std::ptr::null_mut(),
        )
    };
    println!(
        "E43 probe: C dist={} a={} b={} | Rust dist={} a={} b={}",
        fmt_f32(dc),
        fmt_v(ca),
        fmt_v(cb),
        fmt_f32(dr),
        fmt_v(oa),
        fmt_v(ob)
    );
    assert_f32(dc, dr, "E43 probe");
    assert_v(ca, oa, "E43 probe a");
    assert_v(cb, ob, "E43 probe b");
}

/// Directed hill-climb search for the `iter == 20` cap (ERRORS.md E46).
#[test]
fn probe_hillclimb_iterations() {
    let (c, _r): (FnGJK, FnGJK) = sym(b"c2GJK");
    let mut rng = Rng::new(0x81118);
    let call = |sa: &ShapeBlob, ta: u32, sb: &ShapeBlob, tb: u32, ax: &C2x, bx: &C2x| -> i32 {
        let mut oa = C2v::default();
        let mut ob = C2v::default();
        let mut it = -1i32;
        unsafe {
            c(
                sa.as_ptr(), ta, ax, sb.as_ptr(), tb, bx,
                &mut oa, &mut ob, 1, &mut it, std::ptr::null_mut(),
            )
        };
        it
    };
    let mut best = -1i32;
    let mut best_desc = String::new();
    for round in 0..2000u32 {
        // seed
        let ta = C2_TYPE_AABB;
        let tb = if round % 2 == 0 { C2_TYPE_AABB } else { C2_TYPE_CAPSULE };
        let mut params: Vec<f32> = (0..16).map(|_| rng.spicy()).collect();
        let mut cur = -1i32;
        for _ in 0..400 {
            let mut cand = params.clone();
            let k = rng.below(16) as usize;
            cand[k] = match rng.below(4) {
                0 => rng.spicy(),
                1 => cand[k] * rng.range(0.5, 2.0),
                2 => f32::from_bits(cand[k].to_bits() ^ (1u32 << rng.below(32))),
                _ => rng.finite(),
            };
            let sa = ShapeBlob::aabb(C2AABB {
                min: C2v { x: cand[0], y: cand[1] },
                max: C2v { x: cand[2], y: cand[3] },
            });
            let sb = if tb == C2_TYPE_AABB {
                ShapeBlob::aabb(C2AABB {
                    min: C2v { x: cand[4], y: cand[5] },
                    max: C2v { x: cand[6], y: cand[7] },
                })
            } else {
                ShapeBlob::capsule(C2Capsule {
                    a: C2v { x: cand[4], y: cand[5] },
                    b: C2v { x: cand[6], y: cand[7] },
                    r: cand[8],
                })
            };
            let ax = C2x { p: C2v { x: cand[9], y: cand[10] }, r: C2r { c: cand[11], s: cand[12] } };
            let bx = C2x { p: C2v { x: cand[13], y: cand[14] }, r: C2r { c: cand[15], s: cand[15] } };
            let it = call(&sa, ta, &sb, tb, &ax, &bx);
            if it >= cur {
                cur = it;
                params = cand;
                if it > best {
                    best = it;
                    best_desc = format!("{params:?} ta={} tb={}", type_name(ta), type_name(tb));
                }
            }
        }
    }
    println!("hill-climb best iterations = {best}\n  {best_desc}");
}
