use libloading::{Library, Symbol};
use std::path::PathBuf;

// Mirror C struct layouts exactly
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2v { x: f32, y: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Circle { p: C2v, r: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2AABB { min: C2v, max: C2v }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Capsule { a: C2v, b: C2v, r: f32 }

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libomni_collide_lib.so")
}

// ── omni_collide: the top-level public API ──

#[test]
fn test_omni_collide_all_pairs() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_omni: Symbol<unsafe extern "C" fn(i32,f32,f32,f32,f32,f32,i32,f32,f32,f32,f32,f32) -> i32> =
        unsafe { lib.get(b"omni_collide").unwrap() };

    // C2_TYPE: CAPSULE=0, CIRCLE=1, AABB=2
    // Shape params: circle(cx,cy,r,_,_), aabb(minx,miny,maxx,maxy,_), capsule(ax,ay,bx,by,r)

    struct Case { ta: i32, a: [f32;5], tb: i32, b: [f32;5], desc: &'static str }
    let cases = [
        // Circle vs Circle - overlapping
        Case { ta:1, a:[0.0,0.0,2.0,0.0,0.0], tb:1, b:[1.0,0.0,2.0,0.0,0.0], desc:"circle-circle overlap" },
        // Circle vs Circle - not overlapping
        Case { ta:1, a:[0.0,0.0,1.0,0.0,0.0], tb:1, b:[5.0,0.0,1.0,0.0,0.0], desc:"circle-circle no overlap" },
        // Circle vs AABB - overlapping
        Case { ta:1, a:[0.0,0.0,2.0,0.0,0.0], tb:2, b:[1.0,1.0,3.0,3.0,0.0], desc:"circle-aabb overlap" },
        // Circle vs AABB - not overlapping
        Case { ta:1, a:[0.0,0.0,0.5,0.0,0.0], tb:2, b:[5.0,5.0,6.0,6.0,0.0], desc:"circle-aabb no overlap" },
        // Circle vs Capsule - overlapping
        Case { ta:1, a:[0.0,0.0,1.0,0.0,0.0], tb:0, b:[0.5,0.0,2.0,0.0,0.5], desc:"circle-capsule overlap" },
        // Circle vs Capsule - not overlapping
        Case { ta:1, a:[0.0,0.0,0.1,0.0,0.0], tb:0, b:[5.0,5.0,6.0,6.0,0.1], desc:"circle-capsule no overlap" },
        // AABB vs AABB - overlapping
        Case { ta:2, a:[0.0,0.0,2.0,2.0,0.0], tb:2, b:[1.0,1.0,3.0,3.0,0.0], desc:"aabb-aabb overlap" },
        // AABB vs AABB - not overlapping
        Case { ta:2, a:[0.0,0.0,1.0,1.0,0.0], tb:2, b:[5.0,5.0,6.0,6.0,0.0], desc:"aabb-aabb no overlap" },
        // AABB vs Capsule - overlapping
        Case { ta:2, a:[0.0,0.0,2.0,2.0,0.0], tb:0, b:[1.0,1.0,3.0,3.0,0.5], desc:"aabb-capsule overlap" },
        // AABB vs Capsule - not overlapping
        Case { ta:2, a:[0.0,0.0,1.0,1.0,0.0], tb:0, b:[10.0,10.0,11.0,11.0,0.1], desc:"aabb-capsule no overlap" },
        // Capsule vs Capsule - overlapping
        Case { ta:0, a:[0.0,0.0,2.0,0.0,1.0], tb:0, b:[1.0,0.0,3.0,0.0,1.0], desc:"capsule-capsule overlap" },
        // Capsule vs Capsule - not overlapping
        Case { ta:0, a:[0.0,0.0,1.0,0.0,0.1], tb:0, b:[10.0,10.0,11.0,10.0,0.1], desc:"capsule-capsule no overlap" },
        // Reversed pairs (AABB vs Circle, Capsule vs Circle, Capsule vs AABB)
        Case { ta:2, a:[1.0,1.0,3.0,3.0,0.0], tb:1, b:[0.0,0.0,2.0,0.0,0.0], desc:"aabb-circle overlap" },
        Case { ta:0, a:[0.5,0.0,2.0,0.0,0.5], tb:1, b:[0.0,0.0,1.0,0.0,0.0], desc:"capsule-circle overlap" },
        Case { ta:0, a:[1.0,1.0,3.0,3.0,0.5], tb:2, b:[0.0,0.0,2.0,2.0,0.0], desc:"capsule-aabb overlap" },
    ];

    for c in &cases {
        let c_result = unsafe { c_omni(c.ta, c.a[0],c.a[1],c.a[2],c.a[3],c.a[4], c.tb, c.b[0],c.b[1],c.b[2],c.b[3],c.b[4]) };
        let rust_result = omni_collide_lib::omni_collide(
            unsafe { std::mem::transmute::<i32, omni_collide_lib::C2Type>(c.ta) },
            c.a[0],c.a[1],c.a[2],c.a[3],c.a[4],
            unsafe { std::mem::transmute::<i32, omni_collide_lib::C2Type>(c.tb) },
            c.b[0],c.b[1],c.b[2],c.b[3],c.b[4],
        );
        assert_eq!(c_result, rust_result, "MISMATCH for {}: C={} Rust={}", c.desc, c_result, rust_result);
    }
}

// ── Lower-level collision functions via C .so ──

#[test]
fn test_c2circle_to_circle() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_fn: Symbol<unsafe extern "C" fn(C2Circle, C2Circle) -> i32> =
        unsafe { lib.get(b"c2CircletoCircle").unwrap() };

    let cases = [
        (C2Circle { p: C2v{x:0.0,y:0.0}, r:1.0 }, C2Circle { p: C2v{x:1.5,y:0.0}, r:1.0 }),
        (C2Circle { p: C2v{x:0.0,y:0.0}, r:0.5 }, C2Circle { p: C2v{x:5.0,y:0.0}, r:0.5 }),
        (C2Circle { p: C2v{x:1.0,y:1.0}, r:3.0 }, C2Circle { p: C2v{x:2.0,y:2.0}, r:0.1 }),
    ];
    for (a, b) in &cases {
        let c_r = unsafe { c_fn(*a, *b) };
        // Call Rust omni_collide with circle type=1
        let rust_r = omni_collide_lib::omni_collide(
            unsafe { std::mem::transmute(1i32) }, a.p.x, a.p.y, a.r, 0.0, 0.0,
            unsafe { std::mem::transmute(1i32) }, b.p.x, b.p.y, b.r, 0.0, 0.0,
        );
        assert_eq!(c_r, rust_r, "c2CircletoCircle mismatch");
    }
}

#[test]
fn test_c2circle_to_aabb() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_fn: Symbol<unsafe extern "C" fn(C2Circle, C2AABB) -> i32> =
        unsafe { lib.get(b"c2CircletoAABB").unwrap() };

    let cases = [
        (C2Circle { p: C2v{x:0.0,y:0.0}, r:2.0 }, C2AABB { min: C2v{x:1.0,y:1.0}, max: C2v{x:3.0,y:3.0} }),
        (C2Circle { p: C2v{x:0.0,y:0.0}, r:0.1 }, C2AABB { min: C2v{x:5.0,y:5.0}, max: C2v{x:6.0,y:6.0} }),
    ];
    for (a, b) in &cases {
        let c_r = unsafe { c_fn(*a, *b) };
        let rust_r = omni_collide_lib::omni_collide(
            unsafe { std::mem::transmute(1i32) }, a.p.x, a.p.y, a.r, 0.0, 0.0,
            unsafe { std::mem::transmute(2i32) }, b.min.x, b.min.y, b.max.x, b.max.y, 0.0,
        );
        assert_eq!(c_r, rust_r, "c2CircletoAABB mismatch");
    }
}

#[test]
fn test_c2circle_to_capsule() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_fn: Symbol<unsafe extern "C" fn(C2Circle, C2Capsule) -> i32> =
        unsafe { lib.get(b"c2CircletoCapsule").unwrap() };

    let cases = [
        (C2Circle { p: C2v{x:0.0,y:0.0}, r:1.0 }, C2Capsule { a: C2v{x:0.5,y:0.0}, b: C2v{x:2.0,y:0.0}, r:0.5 }),
        (C2Circle { p: C2v{x:0.0,y:0.0}, r:0.1 }, C2Capsule { a: C2v{x:5.0,y:5.0}, b: C2v{x:6.0,y:6.0}, r:0.1 }),
    ];
    for (a, b) in &cases {
        let c_r = unsafe { c_fn(*a, *b) };
        let rust_r = omni_collide_lib::omni_collide(
            unsafe { std::mem::transmute(1i32) }, a.p.x, a.p.y, a.r, 0.0, 0.0,
            unsafe { std::mem::transmute(0i32) }, b.a.x, b.a.y, b.b.x, b.b.y, b.r,
        );
        assert_eq!(c_r, rust_r, "c2CircletoCapsule mismatch");
    }
}

#[test]
fn test_c2aabb_to_aabb() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_fn: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> i32> =
        unsafe { lib.get(b"c2AABBtoAABB").unwrap() };

    let cases = [
        (C2AABB { min: C2v{x:0.0,y:0.0}, max: C2v{x:2.0,y:2.0} }, C2AABB { min: C2v{x:1.0,y:1.0}, max: C2v{x:3.0,y:3.0} }),
        (C2AABB { min: C2v{x:0.0,y:0.0}, max: C2v{x:1.0,y:1.0} }, C2AABB { min: C2v{x:5.0,y:5.0}, max: C2v{x:6.0,y:6.0} }),
    ];
    for (a, b) in &cases {
        let c_r = unsafe { c_fn(*a, *b) };
        let rust_r = omni_collide_lib::omni_collide(
            unsafe { std::mem::transmute(2i32) }, a.min.x, a.min.y, a.max.x, a.max.y, 0.0,
            unsafe { std::mem::transmute(2i32) }, b.min.x, b.min.y, b.max.x, b.max.y, 0.0,
        );
        assert_eq!(c_r, rust_r, "c2AABBtoAABB mismatch");
    }
}

#[test]
fn test_c2aabb_to_capsule() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_fn: Symbol<unsafe extern "C" fn(C2AABB, C2Capsule) -> i32> =
        unsafe { lib.get(b"c2AABBtoCapsule").unwrap() };

    let cases = [
        (C2AABB { min: C2v{x:0.0,y:0.0}, max: C2v{x:2.0,y:2.0} }, C2Capsule { a: C2v{x:1.0,y:1.0}, b: C2v{x:3.0,y:3.0}, r:0.5 }),
        (C2AABB { min: C2v{x:0.0,y:0.0}, max: C2v{x:1.0,y:1.0} }, C2Capsule { a: C2v{x:10.0,y:10.0}, b: C2v{x:11.0,y:11.0}, r:0.1 }),
    ];
    for (a, b) in &cases {
        let c_r = unsafe { c_fn(*a, *b) };
        let rust_r = omni_collide_lib::omni_collide(
            unsafe { std::mem::transmute(2i32) }, a.min.x, a.min.y, a.max.x, a.max.y, 0.0,
            unsafe { std::mem::transmute(0i32) }, b.a.x, b.a.y, b.b.x, b.b.y, b.r,
        );
        assert_eq!(c_r, rust_r, "c2AABBtoCapsule mismatch");
    }
}

#[test]
fn test_c2capsule_to_capsule() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_fn: Symbol<unsafe extern "C" fn(C2Capsule, C2Capsule) -> i32> =
        unsafe { lib.get(b"c2CapsuletoCapsule").unwrap() };

    let cases = [
        (C2Capsule { a: C2v{x:0.0,y:0.0}, b: C2v{x:2.0,y:0.0}, r:1.0 }, C2Capsule { a: C2v{x:1.0,y:0.0}, b: C2v{x:3.0,y:0.0}, r:1.0 }),
        (C2Capsule { a: C2v{x:0.0,y:0.0}, b: C2v{x:1.0,y:0.0}, r:0.1 }, C2Capsule { a: C2v{x:10.0,y:10.0}, b: C2v{x:11.0,y:10.0}, r:0.1 }),
    ];
    for (a, b) in &cases {
        let c_r = unsafe { c_fn(*a, *b) };
        let rust_r = omni_collide_lib::omni_collide(
            unsafe { std::mem::transmute(0i32) }, a.a.x, a.a.y, a.b.x, a.b.y, a.r,
            unsafe { std::mem::transmute(0i32) }, b.a.x, b.a.y, b.b.x, b.b.y, b.r,
        );
        assert_eq!(c_r, rust_r, "c2CapsuletoCapsule mismatch");
    }
}

// ── Vector helper tests (call C directly, compare with Rust via omni_collide indirectly) ──

#[test]
fn test_c2v_helpers() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };

    // c2V
    let c2v_fn: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = unsafe { lib.get(b"c2V").unwrap() };
    let v = unsafe { c2v_fn(3.0, 4.0) };
    assert_eq!((v.x, v.y), (3.0, 4.0));

    // c2Add
    let c2add: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = unsafe { lib.get(b"c2Add").unwrap() };
    let r = unsafe { c2add(C2v{x:1.0,y:2.0}, C2v{x:3.0,y:4.0}) };
    assert_eq!((r.x, r.y), (4.0, 6.0));

    // c2Sub
    let c2sub: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = unsafe { lib.get(b"c2Sub").unwrap() };
    let r = unsafe { c2sub(C2v{x:5.0,y:7.0}, C2v{x:2.0,y:3.0}) };
    assert_eq!((r.x, r.y), (3.0, 4.0));

    // c2Dot
    let c2dot: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = unsafe { lib.get(b"c2Dot").unwrap() };
    let d = unsafe { c2dot(C2v{x:1.0,y:2.0}, C2v{x:3.0,y:4.0}) };
    assert_eq!(d, 11.0);

    // c2Len
    let c2len: Symbol<unsafe extern "C" fn(C2v) -> f32> = unsafe { lib.get(b"c2Len").unwrap() };
    let l = unsafe { c2len(C2v{x:3.0,y:4.0}) };
    assert_eq!(l, 5.0);

    // c2Det2
    let c2det2: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = unsafe { lib.get(b"c2Det2").unwrap() };
    let d = unsafe { c2det2(C2v{x:1.0,y:2.0}, C2v{x:3.0,y:4.0}) };
    assert_eq!(d, -2.0);

    // c2Mulvs
    let c2mulvs: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = unsafe { lib.get(b"c2Mulvs").unwrap() };
    let r = unsafe { c2mulvs(C2v{x:2.0,y:3.0}, 4.0) };
    assert_eq!((r.x, r.y), (8.0, 12.0));

    // c2Neg
    let c2neg: Symbol<unsafe extern "C" fn(C2v) -> C2v> = unsafe { lib.get(b"c2Neg").unwrap() };
    let r = unsafe { c2neg(C2v{x:1.0,y:-2.0}) };
    assert_eq!((r.x, r.y), (-1.0, 2.0));

    // c2Skew
    let c2skew: Symbol<unsafe extern "C" fn(C2v) -> C2v> = unsafe { lib.get(b"c2Skew").unwrap() };
    let r = unsafe { c2skew(C2v{x:1.0,y:2.0}) };
    assert_eq!((r.x, r.y), (-2.0, 1.0));

    // c2CCW90
    let c2ccw90: Symbol<unsafe extern "C" fn(C2v) -> C2v> = unsafe { lib.get(b"c2CCW90").unwrap() };
    let r = unsafe { c2ccw90(C2v{x:1.0,y:2.0}) };
    assert_eq!((r.x, r.y), (2.0, -1.0));
}
