use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_void};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

fn c_lib_path() -> &'static str {
    "c_src/build/libtranslated_rust.so"
}

fn rust_lib_path() -> &'static str {
    "target/debug/libcircle_collide_lib.so"
}

fn load_libs() -> (Library, Library) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        (c_lib, r_lib)
    }
}

// Compare bytes of any T
fn bytes_of<T>(v: &T) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(v as *const T as *const u8, std::mem::size_of::<T>())
    }
}

#[test]
fn test_c2v() {
    let (c_lib, r_lib) = load_libs();
    type Fn_ = unsafe extern "C" fn(f32, f32) -> C2v;
    unsafe {
        let c: Symbol<Fn_> = c_lib.get(b"c2V").unwrap();
        let r: Symbol<Fn_> = r_lib.get(b"c2V").unwrap();
        for &(x, y) in &[(0.0f32, 0.0), (1.0, 2.0), (-3.5, 4.25), (1e10, -1e-10), (f32::NAN, 0.0)] {
            let cv = c(x, y);
            let rv = r(x, y);
            assert_eq!(bytes_of(&cv), bytes_of(&rv), "c2V({}, {})", x, y);
        }
    }
}

#[test]
fn test_c2Mulvs() {
    let (c_lib, r_lib) = load_libs();
    type Fn_ = unsafe extern "C" fn(C2v, f32) -> C2v;
    unsafe {
        let c: Symbol<Fn_> = c_lib.get(b"c2Mulvs").unwrap();
        let r: Symbol<Fn_> = r_lib.get(b"c2Mulvs").unwrap();
        let cases = [
            (C2v { x: 0.0, y: 0.0 }, 0.0f32),
            (C2v { x: 1.0, y: 2.0 }, 3.0),
            (C2v { x: -1.5, y: 2.25 }, -2.0),
            (C2v { x: 1e10, y: -1e-10 }, 1e5),
        ];
        for &(v, s) in &cases {
            assert_eq!(bytes_of(&c(v, s)), bytes_of(&r(v, s)));
        }
    }
}

#[test]
fn test_c2Maxv_c2Minv_c2Clampv_c2Sub() {
    let (c_lib, r_lib) = load_libs();
    type Fn2 = unsafe extern "C" fn(C2v, C2v) -> C2v;
    type Fn3 = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
    unsafe {
        let cmax: Symbol<Fn2> = c_lib.get(b"c2Maxv").unwrap();
        let rmax: Symbol<Fn2> = r_lib.get(b"c2Maxv").unwrap();
        let cmin: Symbol<Fn2> = c_lib.get(b"c2Minv").unwrap();
        let rmin: Symbol<Fn2> = r_lib.get(b"c2Minv").unwrap();
        let cclamp: Symbol<Fn3> = c_lib.get(b"c2Clampv").unwrap();
        let rclamp: Symbol<Fn3> = r_lib.get(b"c2Clampv").unwrap();
        let csub: Symbol<Fn2> = c_lib.get(b"c2Sub").unwrap();
        let rsub: Symbol<Fn2> = r_lib.get(b"c2Sub").unwrap();
        let pts = [
            (C2v { x: 1.0, y: 2.0 }, C2v { x: 3.0, y: -1.0 }),
            (C2v { x: -5.0, y: 10.0 }, C2v { x: 0.0, y: 0.0 }),
            (C2v { x: 1e6, y: -1e6 }, C2v { x: -1e6, y: 1e6 }),
        ];
        for &(a, b) in &pts {
            assert_eq!(bytes_of(&cmax(a, b)), bytes_of(&rmax(a, b)));
            assert_eq!(bytes_of(&cmin(a, b)), bytes_of(&rmin(a, b)));
            assert_eq!(bytes_of(&csub(a, b)), bytes_of(&rsub(a, b)));
        }
        let lo = C2v { x: -1.0, y: -1.0 };
        let hi = C2v { x: 1.0, y: 1.0 };
        for &(a, _) in &pts {
            assert_eq!(bytes_of(&cclamp(a, lo, hi)), bytes_of(&rclamp(a, lo, hi)));
        }
    }
}

#[test]
fn test_c2Dot() {
    let (c_lib, r_lib) = load_libs();
    type Fn_ = unsafe extern "C" fn(C2v, C2v) -> f32;
    unsafe {
        let c: Symbol<Fn_> = c_lib.get(b"c2Dot").unwrap();
        let r: Symbol<Fn_> = r_lib.get(b"c2Dot").unwrap();
        let pts = [
            (C2v { x: 1.0, y: 2.0 }, C2v { x: 3.0, y: 4.0 }),
            (C2v { x: 0.0, y: 0.0 }, C2v { x: 0.0, y: 0.0 }),
            (C2v { x: -1.5, y: 2.5 }, C2v { x: 4.0, y: -2.0 }),
        ];
        for &(a, b) in &pts {
            let cv = c(a, b);
            let rv = r(a, b);
            assert_eq!(bytes_of(&cv), bytes_of(&rv));
        }
    }
}

#[test]
fn test_c2CircletoCircle() {
    let (c_lib, r_lib) = load_libs();
    type Fn_ = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
    unsafe {
        let c: Symbol<Fn_> = c_lib.get(b"c2CircletoCircle").unwrap();
        let r: Symbol<Fn_> = r_lib.get(b"c2CircletoCircle").unwrap();
        let cases = [
            (C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 }, C2Circle { p: C2v { x: 1.0, y: 0.0 }, r: 1.0 }),
            (C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 }, C2Circle { p: C2v { x: 5.0, y: 5.0 }, r: 1.0 }),
            (C2Circle { p: C2v { x: -70.0, y: 0.0 }, r: 20.0 }, C2Circle { p: C2v { x: -70.0, y: 0.0 }, r: 20.0 }),
            (C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 0.0 }, C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 0.0 }),
        ];
        for &(a, b) in &cases {
            assert_eq!(c(a, b), r(a, b));
        }
    }
}

#[test]
fn test_c2CircletoAABB() {
    let (c_lib, r_lib) = load_libs();
    type Fn_ = unsafe extern "C" fn(C2Circle, C2AABB) -> c_int;
    unsafe {
        let c: Symbol<Fn_> = c_lib.get(b"c2CircletoAABB").unwrap();
        let r: Symbol<Fn_> = r_lib.get(b"c2CircletoAABB").unwrap();
        let cases = [
            (C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 }, C2AABB { min: C2v { x: -1.0, y: -1.0 }, max: C2v { x: 1.0, y: 1.0 } }),
            (C2Circle { p: C2v { x: 100.0, y: 100.0 }, r: 1.0 }, C2AABB { min: C2v { x: -1.0, y: -1.0 }, max: C2v { x: 1.0, y: 1.0 } }),
            (C2Circle { p: C2v { x: 2.0, y: 0.0 }, r: 0.5 }, C2AABB { min: C2v { x: -1.0, y: -1.0 }, max: C2v { x: 1.0, y: 1.0 } }),
            (C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 5.0 }, C2AABB { min: C2v { x: -40.0, y: -40.0 }, max: C2v { x: -15.0, y: -15.0 } }),
        ];
        for &(a, b) in &cases {
            assert_eq!(c(a, b), r(a, b));
        }
    }
}

#[test]
fn test_c2CircletoCapsule() {
    let (c_lib, r_lib) = load_libs();
    type Fn_ = unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int;
    unsafe {
        let c: Symbol<Fn_> = c_lib.get(b"c2CircletoCapsule").unwrap();
        let r: Symbol<Fn_> = r_lib.get(b"c2CircletoCapsule").unwrap();
        let cap = C2Capsule {
            a: C2v { x: -40.0, y: 40.0 },
            b: C2v { x: -20.0, y: 100.0 },
            r: 10.0,
        };
        let cases = [
            (C2Circle { p: C2v { x: -40.0, y: 40.0 }, r: 5.0 }, cap),
            (C2Circle { p: C2v { x: -20.0, y: 100.0 }, r: 5.0 }, cap),
            (C2Circle { p: C2v { x: -30.0, y: 70.0 }, r: 5.0 }, cap),
            (C2Circle { p: C2v { x: 100.0, y: 100.0 }, r: 5.0 }, cap),
            (C2Circle { p: C2v { x: -100.0, y: 0.0 }, r: 5.0 }, cap),
            (C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 5.0 }, cap),
        ];
        for &(a, b) in &cases {
            assert_eq!(c(a, b), r(a, b));
        }
    }
}

#[test]
fn test_c2Collided() {
    let (c_lib, r_lib) = load_libs();
    type Fn_ = unsafe extern "C" fn(*const c_void, *const c_void, c_int) -> c_int;
    unsafe {
        let c: Symbol<Fn_> = c_lib.get(b"c2Collided").unwrap();
        let r: Symbol<Fn_> = r_lib.get(b"c2Collided").unwrap();
        let circ_in = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 5.0 };
        let circ = C2Circle { p: C2v { x: 1.0, y: 1.0 }, r: 1.0 };
        let aabb = C2AABB { min: C2v { x: -1.0, y: -1.0 }, max: C2v { x: 1.0, y: 1.0 } };
        let cap = C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 10.0, y: 0.0 }, r: 1.0 };

        let a = &circ_in as *const _ as *const c_void;
        assert_eq!(
            c(a, &circ as *const _ as *const c_void, C2_TYPE_CIRCLE),
            r(a, &circ as *const _ as *const c_void, C2_TYPE_CIRCLE)
        );
        assert_eq!(
            c(a, &aabb as *const _ as *const c_void, C2_TYPE_AABB),
            r(a, &aabb as *const _ as *const c_void, C2_TYPE_AABB)
        );
        assert_eq!(
            c(a, &cap as *const _ as *const c_void, C2_TYPE_CAPSULE),
            r(a, &cap as *const _ as *const c_void, C2_TYPE_CAPSULE)
        );
    }
}

#[test]
fn test_circle_collide() {
    let (c_lib, r_lib) = load_libs();
    type Fn_ = unsafe extern "C" fn(f32, f32, f32) -> c_int;
    unsafe {
        let c: Symbol<Fn_> = c_lib.get(b"circle_collide").unwrap();
        let r: Symbol<Fn_> = r_lib.get(b"circle_collide").unwrap();
        let cases = [
            (0.0f32, 0.0, 1.0),
            (-70.0, 0.0, 20.0),
            (-25.0, -25.0, 5.0),
            (-30.0, 70.0, 5.0),
            (100.0, 100.0, 1.0),
            (-50.0, 0.0, 5.0),
            (-30.0, 50.0, 5.0),
            (-70.0, 0.0, 100.0),
        ];
        for &(x, y, r_val) in &cases {
            assert_eq!(c(x, y, r_val), r(x, y, r_val), "circle_collide({}, {}, {})", x, y, r_val);
        }
    }
}
