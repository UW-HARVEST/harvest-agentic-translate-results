use libloading::{Library, Symbol};
use std::ffi::c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;

fn libs() -> (Library, Library) {
    unsafe {
        let c = Library::new("c_build/build/libtranslated_rust.so").unwrap();
        let r = Library::new("target/debug/libcollided_lib.so").unwrap();
        (c, r)
    }
}

fn v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

#[test]
fn test_c2v() {
    let (c, r) = libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = c.get(b"c2V").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = r.get(b"c2V").unwrap();
        for (x, y) in [(0.0, 0.0), (1.5, -2.3), (f32::MAX, f32::MIN), (f32::NAN, 0.0)] {
            let cv = c_fn(x, y);
            let rv = r_fn(x, y);
            assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "c2V x mismatch for ({x},{y})");
            assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "c2V y mismatch for ({x},{y})");
        }
    }
}

macro_rules! test_v2v {
    ($name:ident, $sym:literal) => {
        #[test]
        fn $name() {
            let (c, r) = libs();
            unsafe {
                let c_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get($sym).unwrap();
                let r_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get($sym).unwrap();
                let cases = [
                    (v(0.0, 0.0), v(0.0, 0.0)),
                    (v(1.0, 2.0), v(3.0, 4.0)),
                    (v(-1.0, 5.0), v(2.0, -3.0)),
                    (v(f32::MAX, f32::MIN), v(f32::MIN, f32::MAX)),
                    (v(f32::NAN, 1.0), v(1.0, f32::NAN)),
                ];
                for (a, b) in cases {
                    let cv = c_fn(a, b);
                    let rv = r_fn(a, b);
                    assert_eq!(cv.x.to_bits(), rv.x.to_bits(),
                        "{} x mismatch for ({:?},{:?})", stringify!($name), a, b);
                    assert_eq!(cv.y.to_bits(), rv.y.to_bits(),
                        "{} y mismatch for ({:?},{:?})", stringify!($name), a, b);
                }
            }
        }
    };
}

test_v2v!(test_c2maxv, b"c2Maxv");
test_v2v!(test_c2minv, b"c2Minv");
test_v2v!(test_c2sub, b"c2Sub");

#[test]
fn test_c2clampv() {
    let (c, r) = libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(C2v, C2v, C2v) -> C2v> = c.get(b"c2Clampv").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2v, C2v, C2v) -> C2v> = r.get(b"c2Clampv").unwrap();
        let cases = [
            (v(0.5, 0.5), v(0.0, 0.0), v(1.0, 1.0)),
            (v(-1.0, 2.0), v(0.0, 0.0), v(1.0, 1.0)),
            (v(5.0, -5.0), v(-1.0, -1.0), v(1.0, 1.0)),
        ];
        for (a, lo, hi) in cases {
            let cv = c_fn(a, lo, hi);
            let rv = r_fn(a, lo, hi);
            assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "c2Clampv x mismatch");
            assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "c2Clampv y mismatch");
        }
    }
}

#[test]
fn test_c2dot() {
    let (c, r) = libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = c.get(b"c2Dot").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = r.get(b"c2Dot").unwrap();
        let cases = [
            (v(0.0, 0.0), v(0.0, 0.0)),
            (v(1.0, 0.0), v(0.0, 1.0)),
            (v(3.0, 4.0), v(4.0, 3.0)),
            (v(-1.0, 2.0), v(2.0, -1.0)),
        ];
        for (a, b) in cases {
            let cv = c_fn(a, b);
            let rv = r_fn(a, b);
            assert_eq!(cv.to_bits(), rv.to_bits(), "c2Dot mismatch for ({:?},{:?})", a, b);
        }
    }
}

#[test]
fn test_c2circletocircle() {
    let (c, r) = libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(C2Circle, C2Circle) -> c_int> =
            c.get(b"c2CircletoCircle").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2Circle, C2Circle) -> c_int> =
            r.get(b"c2CircletoCircle").unwrap();
        let cases = [
            // overlapping
            (C2Circle { p: v(0.0, 0.0), r: 5.0 }, C2Circle { p: v(3.0, 0.0), r: 5.0 }),
            // not overlapping
            (C2Circle { p: v(0.0, 0.0), r: 1.0 }, C2Circle { p: v(10.0, 0.0), r: 1.0 }),
            // touching exactly (d2 == r2, should be 0 since < not <=)
            (C2Circle { p: v(0.0, 0.0), r: 5.0 }, C2Circle { p: v(10.0, 0.0), r: 5.0 }),
            // same position
            (C2Circle { p: v(1.0, 1.0), r: 1.0 }, C2Circle { p: v(1.0, 1.0), r: 1.0 }),
            // zero radius
            (C2Circle { p: v(0.0, 0.0), r: 0.0 }, C2Circle { p: v(0.0, 0.0), r: 0.0 }),
        ];
        for (a, b) in &cases {
            let cv = c_fn(*a, *b);
            let rv = r_fn(*a, *b);
            assert_eq!(cv, rv, "c2CircletoCircle mismatch for ({:?},{:?})", a, b);
        }
    }
}

#[test]
fn test_c2circletoaabb() {
    let (c, r) = libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(C2Circle, C2AABB) -> c_int> =
            c.get(b"c2CircletoAABB").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2Circle, C2AABB) -> c_int> =
            r.get(b"c2CircletoAABB").unwrap();
        let cases = [
            // circle inside aabb
            (C2Circle { p: v(0.5, 0.5), r: 0.1 }, C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }),
            // circle outside
            (C2Circle { p: v(5.0, 5.0), r: 0.1 }, C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }),
            // circle overlapping edge
            (C2Circle { p: v(1.5, 0.5), r: 1.0 }, C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }),
            // circle center on corner
            (C2Circle { p: v(0.0, 0.0), r: 0.5 }, C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }),
        ];
        for (a, b) in &cases {
            let cv = c_fn(*a, *b);
            let rv = r_fn(*a, *b);
            assert_eq!(cv, rv, "c2CircletoAABB mismatch for ({:?},{:?})", a, b);
        }
    }
}

#[test]
fn test_c2aabbtoaabb() {
    let (c, r) = libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> =
            c.get(b"c2AABBtoAABB").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> =
            r.get(b"c2AABBtoAABB").unwrap();
        let cases = [
            // overlapping
            (C2AABB { min: v(0.0, 0.0), max: v(2.0, 2.0) }, C2AABB { min: v(1.0, 1.0), max: v(3.0, 3.0) }),
            // not overlapping
            (C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }, C2AABB { min: v(5.0, 5.0), max: v(6.0, 6.0) }),
            // touching edge (should be 1 since !(0|0|0|0))
            (C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }, C2AABB { min: v(1.0, 0.0), max: v(2.0, 1.0) }),
            // identical
            (C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }, C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }),
            // one inside other
            (C2AABB { min: v(0.0, 0.0), max: v(10.0, 10.0) }, C2AABB { min: v(1.0, 1.0), max: v(2.0, 2.0) }),
        ];
        for (a, b) in &cases {
            let cv = c_fn(*a, *b);
            let rv = r_fn(*a, *b);
            assert_eq!(cv, rv, "c2AABBtoAABB mismatch for ({:?},{:?})", a, b);
        }
    }
}

#[test]
fn test_collided() {
    let (c, r) = libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const u8, c_int, *const u8, c_int) -> c_int> =
            c.get(b"collided").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const u8, c_int, *const u8, c_int) -> c_int> =
            r.get(b"collided").unwrap();

        // Circle vs Circle
        let ca = C2Circle { p: v(0.0, 0.0), r: 5.0 };
        let cb = C2Circle { p: v(3.0, 0.0), r: 5.0 };
        let cv = c_fn(&ca as *const _ as *const u8, C2_TYPE_CIRCLE, &cb as *const _ as *const u8, C2_TYPE_CIRCLE);
        let rv = r_fn(&ca as *const _ as *const u8, C2_TYPE_CIRCLE, &cb as *const _ as *const u8, C2_TYPE_CIRCLE);
        assert_eq!(cv, rv, "collided circle-circle mismatch");

        // Circle vs AABB
        let aabb = C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) };
        let cv = c_fn(&ca as *const _ as *const u8, C2_TYPE_CIRCLE, &aabb as *const _ as *const u8, C2_TYPE_AABB);
        let rv = r_fn(&ca as *const _ as *const u8, C2_TYPE_CIRCLE, &aabb as *const _ as *const u8, C2_TYPE_AABB);
        assert_eq!(cv, rv, "collided circle-aabb mismatch");

        // AABB vs Circle
        let cv = c_fn(&aabb as *const _ as *const u8, C2_TYPE_AABB, &ca as *const _ as *const u8, C2_TYPE_CIRCLE);
        let rv = r_fn(&aabb as *const _ as *const u8, C2_TYPE_AABB, &ca as *const _ as *const u8, C2_TYPE_CIRCLE);
        assert_eq!(cv, rv, "collided aabb-circle mismatch");

        // AABB vs AABB
        let aabb2 = C2AABB { min: v(0.5, 0.5), max: v(1.5, 1.5) };
        let cv = c_fn(&aabb as *const _ as *const u8, C2_TYPE_AABB, &aabb2 as *const _ as *const u8, C2_TYPE_AABB);
        let rv = r_fn(&aabb as *const _ as *const u8, C2_TYPE_AABB, &aabb2 as *const _ as *const u8, C2_TYPE_AABB);
        assert_eq!(cv, rv, "collided aabb-aabb mismatch");

        // Invalid type
        let cv = c_fn(&ca as *const _ as *const u8, 99, &cb as *const _ as *const u8, C2_TYPE_CIRCLE);
        let rv = r_fn(&ca as *const _ as *const u8, 99, &cb as *const _ as *const u8, C2_TYPE_CIRCLE);
        assert_eq!(cv, rv, "collided invalid type mismatch");
    }
}
