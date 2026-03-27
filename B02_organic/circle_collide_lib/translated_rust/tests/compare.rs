use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct c2v {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct c2Circle {
    p: c2v,
    r: f32,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[derive(Clone, Copy)]
#[repr(C)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libtranslated_rust.so"
    );
    unsafe { Library::new(path).expect("Failed to load C library") }
}

fn rust_lib() -> Library {
    let path = format!(
        "{}/target/debug/libcircle_collide_lib.so",
        env!("CARGO_MANIFEST_DIR")
    );
    unsafe { Library::new(&path).expect("Failed to load Rust library") }
}

// ---- Lowest-level: c2V ----
#[test]
fn test_c2v() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(f32, f32) -> c2v;
    let c_fn: Symbol<F> = unsafe { c.get(b"c2V").unwrap() };
    let r_fn: Symbol<F> = unsafe { r.get(b"c2V").unwrap() };

    let cases = [(0.0, 0.0), (1.5, -2.3), (-100.0, 100.0), (f32::MAX, f32::MIN)];
    for (x, y) in cases {
        let cv = unsafe { c_fn(x, y) };
        let rv = unsafe { r_fn(x, y) };
        assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "c2V x mismatch for ({x}, {y})");
        assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "c2V y mismatch for ({x}, {y})");
    }
}

// ---- c2Mulvs ----
#[test]
fn test_c2mulvs() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2v, f32) -> c2v;
    let c_fn: Symbol<F> = unsafe { c.get(b"c2Mulvs").unwrap() };
    let r_fn: Symbol<F> = unsafe { r.get(b"c2Mulvs").unwrap() };

    let cases = [
        (c2v { x: 1.0, y: 2.0 }, 3.0),
        (c2v { x: -1.0, y: 0.0 }, 0.0),
        (c2v { x: 5.5, y: -3.3 }, -2.0),
    ];
    for (v, s) in cases {
        let cv = unsafe { c_fn(v, s) };
        let rv = unsafe { r_fn(v, s) };
        assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "c2Mulvs x mismatch");
        assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "c2Mulvs y mismatch");
    }
}

// ---- c2Maxv, c2Minv ----
#[test]
fn test_c2maxv_c2minv() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2v, c2v) -> c2v;
    let c_max: Symbol<F> = unsafe { c.get(b"c2Maxv").unwrap() };
    let r_max: Symbol<F> = unsafe { r.get(b"c2Maxv").unwrap() };
    let c_min: Symbol<F> = unsafe { c.get(b"c2Minv").unwrap() };
    let r_min: Symbol<F> = unsafe { r.get(b"c2Minv").unwrap() };

    let a = c2v { x: 1.0, y: -5.0 };
    let b = c2v { x: -2.0, y: 3.0 };

    let cv = unsafe { c_max(a, b) };
    let rv = unsafe { r_max(a, b) };
    assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "c2Maxv x");
    assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "c2Maxv y");

    let cv = unsafe { c_min(a, b) };
    let rv = unsafe { r_min(a, b) };
    assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "c2Minv x");
    assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "c2Minv y");
}

// ---- c2Sub ----
#[test]
fn test_c2sub() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2v, c2v) -> c2v;
    let c_fn: Symbol<F> = unsafe { c.get(b"c2Sub").unwrap() };
    let r_fn: Symbol<F> = unsafe { r.get(b"c2Sub").unwrap() };

    let a = c2v { x: 10.0, y: 20.0 };
    let b = c2v { x: 3.0, y: -7.0 };
    let cv = unsafe { c_fn(a, b) };
    let rv = unsafe { r_fn(a, b) };
    assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "c2Sub x");
    assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "c2Sub y");
}

// ---- c2Dot ----
#[test]
fn test_c2dot() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2v, c2v) -> f32;
    let c_fn: Symbol<F> = unsafe { c.get(b"c2Dot").unwrap() };
    let r_fn: Symbol<F> = unsafe { r.get(b"c2Dot").unwrap() };

    let a = c2v { x: 3.0, y: 4.0 };
    let b = c2v { x: -1.0, y: 2.0 };
    let cv = unsafe { c_fn(a, b) };
    let rv = unsafe { r_fn(a, b) };
    assert_eq!(cv.to_bits(), rv.to_bits(), "c2Dot mismatch");
}

// ---- c2Clampv ----
#[test]
fn test_c2clampv() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
    let c_fn: Symbol<F> = unsafe { c.get(b"c2Clampv").unwrap() };
    let r_fn: Symbol<F> = unsafe { r.get(b"c2Clampv").unwrap() };

    let a = c2v { x: 5.0, y: -10.0 };
    let lo = c2v { x: 0.0, y: -5.0 };
    let hi = c2v { x: 3.0, y: 0.0 };
    let cv = unsafe { c_fn(a, lo, hi) };
    let rv = unsafe { r_fn(a, lo, hi) };
    assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "c2Clampv x");
    assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "c2Clampv y");
}

// ---- c2CircletoCircle ----
#[test]
fn test_c2circletocircle() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2Circle, c2Circle) -> c_int;
    let c_fn: Symbol<F> = unsafe { c.get(b"c2CircletoCircle").unwrap() };
    let r_fn: Symbol<F> = unsafe { r.get(b"c2CircletoCircle").unwrap() };

    let cases = [
        // overlapping
        (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 5.0 },
         c2Circle { p: c2v { x: 3.0, y: 0.0 }, r: 5.0 }),
        // not overlapping
        (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
         c2Circle { p: c2v { x: 100.0, y: 0.0 }, r: 1.0 }),
        // touching exactly
        (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 5.0 },
         c2Circle { p: c2v { x: 10.0, y: 0.0 }, r: 5.0 }),
    ];
    for (i, (a, b)) in cases.iter().enumerate() {
        let cv = unsafe { c_fn(*a, *b) };
        let rv = unsafe { r_fn(*a, *b) };
        assert_eq!(cv, rv, "c2CircletoCircle mismatch case {i}");
    }
}

// ---- c2CircletoAABB ----
#[test]
fn test_c2circletoaabb() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2Circle, c2AABB) -> c_int;
    let c_fn: Symbol<F> = unsafe { c.get(b"c2CircletoAABB").unwrap() };
    let r_fn: Symbol<F> = unsafe { r.get(b"c2CircletoAABB").unwrap() };

    let cases = [
        (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 5.0 },
         c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } }),
        (c2Circle { p: c2v { x: 100.0, y: 100.0 }, r: 1.0 },
         c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } }),
    ];
    for (i, (a, b)) in cases.iter().enumerate() {
        let cv = unsafe { c_fn(*a, *b) };
        let rv = unsafe { r_fn(*a, *b) };
        assert_eq!(cv, rv, "c2CircletoAABB mismatch case {i}");
    }
}

// ---- c2CircletoCapsule ----
#[test]
fn test_c2circletocapsule() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2Circle, c2Capsule) -> c_int;
    let c_fn: Symbol<F> = unsafe { c.get(b"c2CircletoCapsule").unwrap() };
    let r_fn: Symbol<F> = unsafe { r.get(b"c2CircletoCapsule").unwrap() };

    let cases = [
        // near segment start
        (c2Circle { p: c2v { x: -35.0, y: 40.0 }, r: 5.0 },
         c2Capsule { a: c2v { x: -40.0, y: 40.0 }, b: c2v { x: -20.0, y: 100.0 }, r: 10.0 }),
        // far away
        (c2Circle { p: c2v { x: 500.0, y: 500.0 }, r: 1.0 },
         c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 10.0 }, r: 1.0 }),
        // near segment middle
        (c2Circle { p: c2v { x: 5.0, y: 5.0 }, r: 3.0 },
         c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 10.0 }, r: 3.0 }),
    ];
    for (i, (a, b)) in cases.iter().enumerate() {
        let cv = unsafe { c_fn(*a, *b) };
        let rv = unsafe { r_fn(*a, *b) };
        assert_eq!(cv, rv, "c2CircletoCapsule mismatch case {i}");
    }
}

// ---- c2Collided ----
#[test]
fn test_c2collided() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(*const u8, *const u8, c_int) -> c_int;
    let c_fn: Symbol<F> = unsafe { c.get(b"c2Collided").unwrap() };
    let r_fn: Symbol<F> = unsafe { r.get(b"c2Collided").unwrap() };

    let circle_a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 5.0 };
    let circle_b = c2Circle { p: c2v { x: 3.0, y: 0.0 }, r: 5.0 };
    let aabb = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
    let capsule = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 10.0 }, r: 3.0 };

    // Circle vs Circle (type 0)
    let cv = unsafe { c_fn(&circle_a as *const _ as *const u8, &circle_b as *const _ as *const u8, 0) };
    let rv = unsafe { r_fn(&circle_a as *const _ as *const u8, &circle_b as *const _ as *const u8, 0) };
    assert_eq!(cv, rv, "c2Collided circle-circle");

    // Circle vs AABB (type 1)
    let cv = unsafe { c_fn(&circle_a as *const _ as *const u8, &aabb as *const _ as *const u8, 1) };
    let rv = unsafe { r_fn(&circle_a as *const _ as *const u8, &aabb as *const _ as *const u8, 1) };
    assert_eq!(cv, rv, "c2Collided circle-aabb");

    // Circle vs Capsule (type 2)
    let cv = unsafe { c_fn(&circle_a as *const _ as *const u8, &capsule as *const _ as *const u8, 2) };
    let rv = unsafe { r_fn(&circle_a as *const _ as *const u8, &capsule as *const _ as *const u8, 2) };
    assert_eq!(cv, rv, "c2Collided circle-capsule");
}

// ---- circle_collide (top-level) ----
#[test]
fn test_circle_collide() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(f32, f32, f32) -> c_int;
    let c_fn: Symbol<F> = unsafe { c.get(b"circle_collide").unwrap() };
    let r_fn: Symbol<F> = unsafe { r.get(b"circle_collide").unwrap() };

    let cases = [
        (0.0_f32, 0.0_f32, 1.0_f32),
        (-70.0, 0.0, 25.0),   // near the circle obstacle
        (-30.0, -30.0, 10.0), // near the AABB
        (-30.0, 70.0, 15.0),  // near the capsule
        (500.0, 500.0, 1.0),  // far away
        (-50.0, 0.0, 50.0),   // large radius, hits multiple
        (-40.0, 40.0, 5.0),   // on capsule start
    ];
    for (x, y, rad) in cases {
        let cv = unsafe { c_fn(x, y, rad) };
        let rv = unsafe { r_fn(x, y, rad) };
        assert_eq!(cv, rv, "circle_collide mismatch for ({x}, {y}, {rad}): C={cv}, Rust={rv}");
    }
}
