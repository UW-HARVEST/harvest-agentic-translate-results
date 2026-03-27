use libloading::{Library, Symbol};
use std::os::raw::c_int;

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

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libcollided_lib.so"
    );
    unsafe { Library::new(path).expect("Failed to load C library") }
}

fn rust_lib() -> Library {
    // Find the Rust cdylib in target/debug/
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    let path = dir.join("libcollided_lib.so");
    unsafe { Library::new(path).expect("Failed to load Rust library") }
}

macro_rules! assert_c2v_eq {
    ($a:expr, $b:expr, $name:expr) => {
        assert_eq!($a.x.to_bits(), $b.x.to_bits(), "{}: x mismatch", $name);
        assert_eq!($a.y.to_bits(), $b.y.to_bits(), "{}: y mismatch", $name);
    };
}

#[test]
fn test_c2v() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = c.get(b"c2V").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = r.get(b"c2V").unwrap();
        for (x, y) in [(0.0f32, 0.0), (1.5, -2.3), (f32::MAX, f32::MIN), (f32::NAN, 0.0)] {
            let cv = c_fn(x, y);
            let rv = r_fn(x, y);
            assert_c2v_eq!(cv, rv, format!("c2V({x}, {y})"));
        }
    }
}

#[test]
fn test_c2maxv() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get(b"c2Maxv").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get(b"c2Maxv").unwrap();
        let a = C2v { x: 1.0, y: -3.0 };
        let b = C2v { x: -2.0, y: 5.0 };
        assert_c2v_eq!(c_fn(a, b), r_fn(a, b), "c2Maxv");
    }
}

#[test]
fn test_c2minv() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get(b"c2Minv").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get(b"c2Minv").unwrap();
        let a = C2v { x: 1.0, y: -3.0 };
        let b = C2v { x: -2.0, y: 5.0 };
        assert_c2v_eq!(c_fn(a, b), r_fn(a, b), "c2Minv");
    }
}

#[test]
fn test_c2clampv() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(C2v, C2v, C2v) -> C2v> =
            c.get(b"c2Clampv").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2v, C2v, C2v) -> C2v> =
            r.get(b"c2Clampv").unwrap();
        let a = C2v { x: 5.0, y: -1.0 };
        let lo = C2v { x: 0.0, y: 0.0 };
        let hi = C2v { x: 3.0, y: 3.0 };
        assert_c2v_eq!(c_fn(a, lo, hi), r_fn(a, lo, hi), "c2Clampv");
    }
}

#[test]
fn test_c2sub() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get(b"c2Sub").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get(b"c2Sub").unwrap();
        let a = C2v { x: 3.0, y: 7.0 };
        let b = C2v { x: 1.0, y: 2.0 };
        assert_c2v_eq!(c_fn(a, b), r_fn(a, b), "c2Sub");
    }
}

#[test]
fn test_c2dot() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = c.get(b"c2Dot").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = r.get(b"c2Dot").unwrap();
        let a = C2v { x: 3.0, y: 4.0 };
        let b = C2v { x: 1.0, y: 2.0 };
        assert_eq!(c_fn(a, b).to_bits(), r_fn(a, b).to_bits(), "c2Dot mismatch");
    }
}

#[test]
fn test_c2circle_to_circle() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(C2Circle, C2Circle) -> c_int> =
            c.get(b"c2CircletoCircle").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2Circle, C2Circle) -> c_int> =
            r.get(b"c2CircletoCircle").unwrap();
        // overlapping
        let a = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 5.0 };
        let b = C2Circle { p: C2v { x: 3.0, y: 0.0 }, r: 5.0 };
        assert_eq!(c_fn(a, b), r_fn(a, b), "CircletoCircle overlapping");
        // not overlapping
        let b2 = C2Circle { p: C2v { x: 100.0, y: 0.0 }, r: 1.0 };
        assert_eq!(c_fn(a, b2), r_fn(a, b2), "CircletoCircle non-overlapping");
    }
}

#[test]
fn test_c2circle_to_aabb() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(C2Circle, C2AABB) -> c_int> =
            c.get(b"c2CircletoAABB").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2Circle, C2AABB) -> c_int> =
            r.get(b"c2CircletoAABB").unwrap();
        let circ = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 2.0 };
        let aabb = C2AABB { min: C2v { x: 1.0, y: 1.0 }, max: C2v { x: 3.0, y: 3.0 } };
        assert_eq!(c_fn(circ, aabb), r_fn(circ, aabb), "CircletoAABB");
        let far = C2AABB { min: C2v { x: 10.0, y: 10.0 }, max: C2v { x: 12.0, y: 12.0 } };
        assert_eq!(c_fn(circ, far), r_fn(circ, far), "CircletoAABB far");
    }
}

#[test]
fn test_c2aabb_to_aabb() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> =
            c.get(b"c2AABBtoAABB").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> =
            r.get(b"c2AABBtoAABB").unwrap();
        let a = C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 5.0, y: 5.0 } };
        let b = C2AABB { min: C2v { x: 3.0, y: 3.0 }, max: C2v { x: 8.0, y: 8.0 } };
        assert_eq!(c_fn(a, b), r_fn(a, b), "AABBtoAABB overlapping");
        let far = C2AABB { min: C2v { x: 10.0, y: 10.0 }, max: C2v { x: 12.0, y: 12.0 } };
        assert_eq!(c_fn(a, far), r_fn(a, far), "AABBtoAABB non-overlapping");
    }
}

#[test]
fn test_collided() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const u8, c_int, *const u8, c_int) -> c_int> =
            c.get(b"collided").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const u8, c_int, *const u8, c_int) -> c_int> =
            r.get(b"collided").unwrap();

        // Circle vs Circle
        let ca = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 5.0 };
        let cb = C2Circle { p: C2v { x: 3.0, y: 0.0 }, r: 5.0 };
        let pa = &ca as *const C2Circle as *const u8;
        let pb = &cb as *const C2Circle as *const u8;
        assert_eq!(c_fn(pa, 0, pb, 0), r_fn(pa, 0, pb, 0), "collided circle-circle");

        // Circle vs AABB
        let aabb = C2AABB { min: C2v { x: 1.0, y: 1.0 }, max: C2v { x: 3.0, y: 3.0 } };
        let pb2 = &aabb as *const C2AABB as *const u8;
        assert_eq!(c_fn(pa, 0, pb2, 1), r_fn(pa, 0, pb2, 1), "collided circle-aabb");

        // AABB vs Circle
        assert_eq!(c_fn(pb2, 1, pa, 0), r_fn(pb2, 1, pa, 0), "collided aabb-circle");

        // AABB vs AABB
        let aabb2 = C2AABB { min: C2v { x: 2.0, y: 2.0 }, max: C2v { x: 6.0, y: 6.0 } };
        let pb3 = &aabb2 as *const C2AABB as *const u8;
        assert_eq!(c_fn(pb2, 1, pb3, 1), r_fn(pb2, 1, pb3, 1), "collided aabb-aabb");
    }
}
