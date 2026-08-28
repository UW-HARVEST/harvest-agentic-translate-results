//! Level 4: manifold generation, `c2Collide`, `ptr_from_parts` and `omni_manifold`.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

/// Non-zero seed pattern so that fields the C never writes are still compared.
fn seed_manifold() -> c2Manifold {
    c2Manifold {
        count: -1_431_655_766,
        depths: [-77.5, 88.25],
        contact_points: [
            c2v { x: 1.5, y: -2.5 },
            c2v { x: 3.5, y: -4.5 },
        ],
        n: c2v { x: -5.5, y: 6.5 },
    }
}

fn gen_circle(rng: &mut Rng, wild: bool) -> c2Circle {
    if wild {
        c2Circle {
            p: rng.vec_wild(),
            r: rng.wild(),
        }
    } else {
        c2Circle {
            p: rng.vec_tame(),
            r: rng.radius(),
        }
    }
}

fn gen_aabb(rng: &mut Rng, wild: bool) -> c2AABB {
    if wild {
        c2AABB {
            min: rng.vec_wild(),
            max: rng.vec_wild(),
        }
    } else {
        let (a, b) = (rng.vec_tame(), rng.vec_tame());
        c2AABB {
            min: c2v {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            },
            max: c2v {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            },
        }
    }
}

fn gen_capsule(rng: &mut Rng, wild: bool) -> c2Capsule {
    if wild {
        c2Capsule {
            a: rng.vec_wild(),
            b: rng.vec_wild(),
            r: rng.wild(),
        }
    } else {
        c2Capsule {
            a: rng.vec_tame(),
            b: rng.vec_tame(),
            r: rng.radius(),
        }
    }
}

macro_rules! manifold_test {
    ($name:ident, $sym:literal, $ty:ty, $genA:ident, $genB:ident, $iters:expr) => {
        #[test]
        fn $name() {
            let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
            let (cf, rf) = l.pair::<unsafe extern "C" fn($ty, $ty, *mut c2Manifold) -> ()>($sym);
            let mut rng = Rng::new(0x4000 + $sym.len() as u64);
            for it in 0..$iters {
                for wild in [false, true] {
                    let A = $genA(&mut rng, wild);
                    let B = $genB(&mut rng, wild);
                    let mut mc = seed_manifold();
                    let mut mr = seed_manifold();
                    scrub_stack();
                    unsafe { cf(A, B, &mut mc) };
                    unsafe { rf(A, B, &mut mr) };
                    assert_same_lazy(&mc, &mr, || format!("{} #{it} wild={wild} A={A:?} B={B:?}", $sym));
                }
            }
        }
    };
}

manifold_test!(
    circle_to_circle,
    "c2CircletoCircleManifold",
    c2Circle,
    gen_circle,
    gen_circle,
    50_000
);
manifold_test!(
    aabb_to_aabb,
    "c2AABBtoAABBManifold",
    c2AABB,
    gen_aabb,
    gen_aabb,
    50_000
);
manifold_test!(
    capsule_to_capsule,
    "c2CapsuletoCapsuleManifold",
    c2Capsule,
    gen_capsule,
    gen_capsule,
    50_000
);

#[test]
fn circle_to_aabb() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) =
        l.pair::<unsafe extern "C" fn(c2Circle, c2AABB, *mut c2Manifold) -> ()>(
            "c2CircletoAABBManifold",
        );
    let mut rng = Rng::new(401);
    for it in 0..50_000 {
        for wild in [false, true] {
            let A = gen_circle(&mut rng, wild);
            let B = gen_aabb(&mut rng, wild);
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            scrub_stack();
            unsafe { cf(A, B, &mut mc) };
            unsafe { rf(A, B, &mut mr) };
            assert_same_lazy(&mc, &mr, || format!("c2CircletoAABBManifold #{it} wild={wild} A={A:?} B={B:?}"));
        }
    }
}

#[test]
fn circle_to_capsule() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) =
        l.pair::<unsafe extern "C" fn(c2Circle, c2Capsule, *mut c2Manifold) -> ()>(
            "c2CircletoCapsuleManifold",
        );
    let mut rng = Rng::new(403);
    for it in 0..50_000 {
        for wild in [false, true] {
            let A = gen_circle(&mut rng, wild);
            let B = gen_capsule(&mut rng, wild);
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            scrub_stack();
            unsafe { cf(A, B, &mut mc) };
            unsafe { rf(A, B, &mut mr) };
            assert_same_lazy(&mc, &mr, || format!("c2CircletoCapsuleManifold #{it} wild={wild} A={A:?} B={B:?}"));
        }
    }
}

#[test]
fn aabb_to_capsule() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) = l.pair::<unsafe extern "C" fn(c2AABB, c2Capsule, *mut c2Manifold) -> ()>(
        "c2AABBtoCapsuleManifold",
    );
    let mut rng = Rng::new(405);
    for it in 0..50_000 {
        for wild in [false, true] {
            let A = gen_aabb(&mut rng, wild);
            let B = gen_capsule(&mut rng, wild);
            let mut mc = seed_manifold();
            let mut mr = seed_manifold();
            scrub_stack();
            unsafe { cf(A, B, &mut mc) };
            unsafe { rf(A, B, &mut mr) };
            assert_same_lazy(&mc, &mr, || format!("c2AABBtoCapsuleManifold #{it} wild={wild} A={A:?} B={B:?}"));
        }
    }
}

// ---------------------------------------------------------------------------
// c2Collide / ptr_from_parts / omni_manifold
// ---------------------------------------------------------------------------

const TYPES: [C2_TYPE; 4] = [C2_TYPE_CAPSULE, C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_POLY];

enum Shape {
    Circle(c2Circle),
    Aabb(c2AABB),
    Capsule(c2Capsule),
}

impl Shape {
    fn ptr(&self) -> *const c_void {
        match self {
            Shape::Circle(c) => c as *const _ as *const c_void,
            Shape::Aabb(c) => c as *const _ as *const c_void,
            Shape::Capsule(c) => c as *const _ as *const c_void,
        }
    }
    fn make(t: C2_TYPE, rng: &mut Rng, wild: bool) -> Shape {
        match t {
            C2_TYPE_CIRCLE => Shape::Circle(gen_circle(rng, wild)),
            C2_TYPE_AABB => Shape::Aabb(gen_aabb(rng, wild)),
            _ => Shape::Capsule(gen_capsule(rng, wild)),
        }
    }
}

#[test]
fn collide_all_type_pairs() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) = l.pair::<unsafe extern "C" fn(
        *const c_void,
        C2_TYPE,
        *const c_void,
        C2_TYPE,
        *mut c2Manifold,
    ) -> ()>("c2Collide");
    let mut rng = Rng::new(407);
    for it in 0..8_000 {
        for wild in [false, true] {
            for &ta in &TYPES {
                for &tb in &TYPES {
                    // A polygon operand is never dereferenced by c2Collide (no case
                    // handles it), but pass a real object anyway.
                    let A = Shape::make(ta, &mut rng, wild);
                    let B = Shape::make(tb, &mut rng, wild);
                    let mut mc = seed_manifold();
                    let mut mr = seed_manifold();
                    scrub_stack();
                    unsafe { cf(A.ptr(), ta, B.ptr(), tb, &mut mc) };
                    unsafe { rf(A.ptr(), ta, B.ptr(), tb, &mut mr) };
                    assert_same_lazy(&mc, &mr, || format!("c2Collide #{it} wild={wild} ta={ta} tb={tb}"));
                }
            }
        }
    }
}

#[test]
fn ptr_from_parts_matches() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) = l
        .pair::<unsafe extern "C" fn(C2_TYPE, f32, f32, f32, f32, f32) -> *mut c_void>(
            "ptr_from_parts",
        );
    let mut rng = Rng::new(409);
    for _ in 0..20_000 {
        let t = TYPES[rng.below(3) as usize + 1]; // CIRCLE / AABB / POLY -> skip POLY below
        let t = if t == C2_TYPE_POLY { C2_TYPE_CAPSULE } else { t };
        let v: [f32; 5] = [rng.wild(), rng.wild(), rng.wild(), rng.wild(), rng.wild()];
        unsafe {
            let pc = cf(t, v[0], v[1], v[2], v[3], v[4]);
            let pr = rf(t, v[0], v[1], v[2], v[3], v[4]);
            assert!(!pc.is_null() && !pr.is_null(), "ptr_from_parts returned null");
            let n = match t {
                C2_TYPE_CIRCLE => std::mem::size_of::<c2Circle>(),
                C2_TYPE_AABB => std::mem::size_of::<c2AABB>(),
                _ => std::mem::size_of::<c2Capsule>(),
            };
            let bc = std::slice::from_raw_parts(pc as *const u8, n);
            let br = std::slice::from_raw_parts(pr as *const u8, n);
            assert_eq!(
                hex(bc),
                hex(br),
                "ptr_from_parts(type={t}, {v:?}) payload differs"
            );
        }
    }
}

#[test]
fn omni_manifold_all_type_pairs() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    type F = unsafe extern "C" fn(
        *mut c2Manifold,
        C2_TYPE,
        f32,
        f32,
        f32,
        f32,
        f32,
        C2_TYPE,
        f32,
        f32,
        f32,
        f32,
        f32,
    );
    let (cf, rf) = l.pair::<F>("omni_manifold");
    let mut rng = Rng::new(411);
    for it in 0..6_000 {
        for wild in [false, true] {
            for &ta in &TYPES {
                for &tb in &TYPES {
                    let g = |r: &mut Rng| if wild { r.wild() } else { r.tame() };
                    let a: [f32; 5] = [g(&mut rng), g(&mut rng), g(&mut rng), g(&mut rng), g(&mut rng)];
                    let b: [f32; 5] = [g(&mut rng), g(&mut rng), g(&mut rng), g(&mut rng), g(&mut rng)];
                    let mut mc = seed_manifold();
                    let mut mr = seed_manifold();
                    scrub_stack();
                    unsafe { cf(&mut mc, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]) };
                    unsafe { rf(&mut mr, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]) };
                    assert_same_lazy(&mc, &mr, || format!(
                            "omni_manifold #{it} wild={wild} ta={ta} a={a:?} tb={tb} b={b:?}"
                        ));
                }
            }
        }
    }
}

/// A dense sweep of plausible physics inputs (nothing wild), which is what a
/// consumer of `omni_manifold` would actually feed it.
#[test]
fn omni_manifold_grid() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    type F = unsafe extern "C" fn(
        *mut c2Manifold,
        C2_TYPE,
        f32,
        f32,
        f32,
        f32,
        f32,
        C2_TYPE,
        f32,
        f32,
        f32,
        f32,
        f32,
    );
    let (cf, rf) = l.pair::<F>("omni_manifold");
    let steps: [f32; 9] = [-2.0, -1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0];
    for &ta in &TYPES {
        for &tb in &TYPES {
            for &dx in &steps {
                for &dy in &steps {
                    for &r in &[0.0f32, 0.5, 1.0] {
                        let a = [-1.0f32, 0.0, 1.0, 0.5, r];
                        let b = [dx - 1.0, dy, dx + 1.0, dy + 0.5, r];
                        let mut mc = seed_manifold();
                        let mut mr = seed_manifold();
                        scrub_stack();
                        unsafe { cf(&mut mc, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]) };
                        unsafe { rf(&mut mr, ta, a[0], a[1], a[2], a[3], a[4], tb, b[0], b[1], b[2], b[3], b[4]) };
                        assert_same_lazy(&mc, &mr, || format!("omni grid ta={ta} tb={tb} dx={dx} dy={dy} r={r}"));
                    }
                }
            }
        }
    }
}

/// Degenerate AABBs (min == max) make every polygon normal NaN, which is the
/// case the `c2Poly` "preceding word" reproduction in `c2AABBtoCapsuleManifold`
/// exists for.
#[test]
fn degenerate_aabb_capsule() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) = l.pair::<unsafe extern "C" fn(c2AABB, c2Capsule, *mut c2Manifold) -> ()>(
        "c2AABBtoCapsuleManifold",
    );
    let vals: [f32; 7] = [-2.0, -1.0, -0.25, 0.0, 0.25, 1.0, 2.0];
    let mut n = 0usize;
    for &x in &vals {
        for &y in &vals {
            let A = c2AABB {
                min: c2v { x, y },
                max: c2v { x, y },
            };
            for &cx in &vals {
                for &cy in &vals {
                    for &r in &[0.0f32, 0.5, 1.0, 2.0] {
                        let B = c2Capsule {
                            a: c2v { x: cx, y: cy },
                            b: c2v { x: cx + 1.0, y: cy },
                            r,
                        };
                        let mut mc = seed_manifold();
                        let mut mr = seed_manifold();
                        scrub_stack();
                        unsafe { cf(A, B, &mut mc) };
                        unsafe { rf(A, B, &mut mr) };
                        assert_same_lazy(&mc, &mr, || format!("degenerate aabb {A:?} {B:?}"));
                        n += 1;
                    }
                }
            }
        }
    }
    assert!(n > 1000);
}

#[test]
fn capsule_to_poly_matches() {
    let _serial = serialize();
    let l = Libs::load();
    l.warm_up();
    let (cf, rf) =
        l.pair::<unsafe extern "C" fn(c2Capsule, *const c2Poly, *const c2x, *mut c2Manifold) -> ()>(
            "c2CapsuletoPolyManifold",
        );
    let (cn, _rn) = l.pair::<unsafe extern "C" fn(*mut c2v, *mut c2v, c_int) -> ()>("c2Norms");
    let mut rng = Rng::new(413);
    for it in 0..30_000 {
        // Convex polygon: a regular n-gon, jittered, kept convex by construction.
        let count = 3 + rng.below(6) as c_int;
        let mut p = c2Poly::default();
        p.count = count;
        let rad = 0.25 + (rng.next_u32() as f64 / u32::MAX as f64) as f32 * 2.0;
        for i in 0..count {
            let ang = i as f32 / count as f32 * 6.283_185_5;
            p.verts[i as usize] = c2v {
                x: rad * ang.cos(),
                y: rad * ang.sin(),
            };
        }
        unsafe { cn(p.verts.as_mut_ptr(), p.norms.as_mut_ptr(), count) };
        let A = gen_capsule(&mut rng, false);
        let bx = if rng.below(2) == 0 {
            let ang = (rng.next_u32() as f64 / u32::MAX as f64) as f32 * 6.283_185_5;
            Some(c2x {
                p: rng.vec_tame(),
                r: c2r {
                    c: ang.cos(),
                    s: ang.sin(),
                },
            })
        } else {
            None
        };
        let bxp = bx.as_ref().map_or(std::ptr::null(), |x| x as *const c2x);
        let mut mc = seed_manifold();
        let mut mr = seed_manifold();
        scrub_stack();
        unsafe { cf(A, &p, bxp, &mut mc) };
        unsafe { rf(A, &p, bxp, &mut mr) };
        assert_same_lazy(&mc, &mr, || format!("c2CapsuletoPolyManifold #{it} A={A:?} count={count} bx={bx:?}"));
    }
}
