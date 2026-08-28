//! Loading / ABI smoke test: every one of the 22 exported symbols is resolvable
//! in BOTH shared libraries and returns the same thing for one simple input.
mod common;
use common::*;

#[test]
fn smoke_all_symbols_resolve_and_agree() {
    let p = apis();
    let mut ck = Checker::new("smoke");
    let ctx = || "fixed smoke input".to_string();
    unsafe {
        ck.vec("c2V", (p.c.c2V)(1.5, -2.5), (p.r.c2V)(1.5, -2.5), &ctx);
        ck.f32(
            "c2Dot",
            (p.c.c2Dot)(v(1.5, 2.0), v(-3.0, 4.0)),
            (p.r.c2Dot)(v(1.5, 2.0), v(-3.0, 4.0)),
            &ctx,
        );
        ck.f32("c2Len", (p.c.c2Len)(v(3.0, 4.0)), (p.r.c2Len)(v(3.0, 4.0)), &ctx);
        ck.vec("c2Add", (p.c.c2Add)(v(1.0, 2.0), v(3.0, 4.0)), (p.r.c2Add)(v(1.0, 2.0), v(3.0, 4.0)), &ctx);
        ck.vec("c2Sub", (p.c.c2Sub)(v(1.0, 2.0), v(3.0, 4.0)), (p.r.c2Sub)(v(1.0, 2.0), v(3.0, 4.0)), &ctx);
        ck.vec("c2Mulvs", (p.c.c2Mulvs)(v(1.0, 2.0), 3.0), (p.r.c2Mulvs)(v(1.0, 2.0), 3.0), &ctx);
        ck.vec("c2Div", (p.c.c2Div)(v(1.0, 2.0), 3.0), (p.r.c2Div)(v(1.0, 2.0), 3.0), &ctx);
        ck.vec("c2Norm", (p.c.c2Norm)(v(3.0, 4.0)), (p.r.c2Norm)(v(3.0, 4.0)), &ctx);
        ck.vec("c2Minv", (p.c.c2Minv)(v(1.0, 5.0), v(2.0, 4.0)), (p.r.c2Minv)(v(1.0, 5.0), v(2.0, 4.0)), &ctx);
        ck.vec("c2Maxv", (p.c.c2Maxv)(v(1.0, 5.0), v(2.0, 4.0)), (p.r.c2Maxv)(v(1.0, 5.0), v(2.0, 4.0)), &ctx);
        ck.vec("c2Skew", (p.c.c2Skew)(v(1.0, 2.0)), (p.r.c2Skew)(v(1.0, 2.0)), &ctx);
        ck.vec("c2Absv", (p.c.c2Absv)(v(-1.0, 2.0)), (p.r.c2Absv)(v(-1.0, 2.0)), &ctx);
        ck.vec("c2CCW90", (p.c.c2CCW90)(v(1.0, 2.0)), (p.r.c2CCW90)(v(1.0, 2.0)), &ctx);
        let m = C2m { x: v(1.0, 2.0), y: v(3.0, 4.0) };
        ck.vec("c2MulmvT", (p.c.c2MulmvT)(m, v(5.0, 6.0)), (p.r.c2MulmvT)(m, v(5.0, 6.0)), &ctx);

        let ray = C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: 100.0 };
        let circle = C2Circle { p: v(0.0, 0.0), r: 2.0 };
        let mut oc = POISON;
        let mut or_ = POISON;
        ck.int("c2RaytoCircle", (p.c.c2RaytoCircle)(ray, circle, &mut oc), (p.r.c2RaytoCircle)(ray, circle, &mut or_), &ctx);
        ck.cast("c2RaytoCircle.out", oc, or_, &ctx);

        let a = C2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) };
        let b = C2AABB { min: v(0.0, 0.0), max: v(2.0, 2.0) };
        ck.int("c2AABBtoAABB", (p.c.c2AABBtoAABB)(a, b), (p.r.c2AABBtoAABB)(a, b), &ctx);
        let mut oc = POISON;
        let mut or_ = POISON;
        ck.int("c2RaytoAABB", (p.c.c2RaytoAABB)(ray, a, &mut oc), (p.r.c2RaytoAABB)(ray, a, &mut or_), &ctx);
        ck.cast("c2RaytoAABB.out", oc, or_, &ctx);
        ck.int("c2AABBtoPoint", (p.c.c2AABBtoPoint)(a, v(0.5, 0.5)), (p.r.c2AABBtoPoint)(a, v(0.5, 0.5)), &ctx);
        ck.int("c2CircleToPoint", (p.c.c2CircleToPoint)(circle, v(0.5, 0.5)), (p.r.c2CircleToPoint)(circle, v(0.5, 0.5)), &ctx);

        let cap = C2Capsule { a: v(0.0, -3.0), b: v(0.0, 3.0), r: 1.0 };
        let mut oc = POISON;
        let mut or_ = POISON;
        ck.int("c2RaytoCapsule", (p.c.c2RaytoCapsule)(ray, cap, &mut oc), (p.r.c2RaytoCapsule)(ray, cap, &mut or_), &ctx);
        ck.cast("c2RaytoCapsule.out", oc, or_, &ctx);

        let mut oc = POISON;
        let mut or_ = POISON;
        ck.int(
            "c2CastRay",
            (p.c.c2CastRay)(ray, &circle as *const C2Circle as *const _, C2_TYPE_CIRCLE, &mut oc),
            (p.r.c2CastRay)(ray, &circle as *const C2Circle as *const _, C2_TYPE_CIRCLE, &mut or_),
            &ctx,
        );
        ck.cast("c2CastRay.out", oc, or_, &ctx);

        let mut oc = POISON;
        let mut or_ = POISON;
        ck.int(
            "spec_ray",
            (p.c.spec_ray)(&mut oc, 5.0, 5.0, 0.0, 0.0, 2.0, -5.0, -5.0),
            (p.r.spec_ray)(&mut or_, 5.0, 5.0, 0.0, 0.0, 2.0, -5.0, -5.0),
            &ctx,
        );
        ck.cast("spec_ray.out", oc, or_, &ctx);
    }
    ck.finish();
}

/// The Rust `.so` must export every symbol the C `.so` exports (Phase D, done
/// again here from inside the harness so a regression fails the test suite).
#[test]
fn symbol_parity() {
    let p = apis();
    let c_syms = nm_exports(&p.c.path);
    let r_syms = nm_exports(&p.r.path);
    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    eprintln!("C exports {} symbols, rust exports {}", c_syms.len(), r_syms.len());
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );
    for s in ALL_SYMBOLS {
        assert!(c_syms.contains(&s.to_string()), "C .so lost symbol {s}");
        assert!(r_syms.contains(&s.to_string()), "rust .so lost symbol {s}");
    }
    assert_eq!(c_syms.len(), 22, "unexpected C export count: {c_syms:?}");
}

fn nm_exports(path: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", path.display());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Only real code exports; skip GLIBC/ITM/gmon weak data symbols.
            if kind == "T" && !name.starts_with('_') {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}
