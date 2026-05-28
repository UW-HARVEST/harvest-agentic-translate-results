// Integration tests comparing C and Rust implementations via FFI
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2Raycast {
    pub t: f32,
    pub n: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2Circle {
    pub p: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2AABB {
    pub min: c2v,
    pub max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2Capsule {
    pub a: c2v,
    pub b: c2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2Ray {
    pub p: c2v,
    pub d: c2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct c2m {
    pub x: c2v,
    pub y: c2v,
}

#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum C2_TYPE {
    C2_TYPE_CIRCLE = 0,
    C2_TYPE_AABB = 1,
    C2_TYPE_CAPSULE = 2,
}

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    p
}

fn rust_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Try release first, otherwise debug
    let release = {
        let mut r = p.clone();
        r.push("release");
        r.push("libgen_ray_lib.so");
        r
    };
    if release.exists() {
        return release;
    }
    p.push("debug");
    p.push("libgen_ray_lib.so");
    p
}

fn libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_so_path()).expect("failed to load C .so");
        let r = Library::new(rust_so_path()).expect("failed to load Rust .so");
        (c, r)
    }
}

fn bits_eq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

fn vec_eq(a: c2v, b: c2v) -> bool {
    bits_eq(a.x, b.x) && bits_eq(a.y, b.y)
}

fn raycast_eq(a: c2Raycast, b: c2Raycast) -> bool {
    bits_eq(a.t, b.t) && vec_eq(a.n, b.n)
}

#[test]
fn test_c2V() {
    let (c, r) = libs();
    let test_vals: &[(f32, f32)] = &[
        (0.0, 0.0),
        (1.0, 2.0),
        (-1.5, 3.5),
        (f32::INFINITY, -f32::INFINITY),
        (1e30, -1e-30),
    ];
    type F = unsafe extern "C" fn(f32, f32) -> c2v;
    unsafe {
        let cf: Symbol<F> = c.get(b"c2V").unwrap();
        let rf: Symbol<F> = r.get(b"c2V").unwrap();
        for &(x, y) in test_vals {
            let cr = cf(x, y);
            let rr = rf(x, y);
            assert!(vec_eq(cr, rr), "c2V({},{}): {:?} vs {:?}", x, y, cr, rr);
        }
    }
}

#[test]
fn test_c2Dot() {
    let (c, r) = libs();
    type F = unsafe extern "C" fn(c2v, c2v) -> f32;
    unsafe {
        let cf: Symbol<F> = c.get(b"c2Dot").unwrap();
        let rf: Symbol<F> = r.get(b"c2Dot").unwrap();
        let cases = [
            (c2v { x: 1.0, y: 2.0 }, c2v { x: 3.0, y: 4.0 }),
            (c2v { x: 0.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }),
            (c2v { x: -1.0, y: 2.5 }, c2v { x: 3.5, y: -4.0 }),
        ];
        for (a, b) in cases {
            assert!(bits_eq(cf(a, b), rf(a, b)));
        }
    }
}

#[test]
fn test_c2Len() {
    let (c, r) = libs();
    type F = unsafe extern "C" fn(c2v) -> f32;
    unsafe {
        let cf: Symbol<F> = c.get(b"c2Len").unwrap();
        let rf: Symbol<F> = r.get(b"c2Len").unwrap();
        for v in [
            c2v { x: 3.0, y: 4.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -7.0, y: 24.0 },
        ] {
            assert!(bits_eq(cf(v), rf(v)));
        }
    }
}

#[test]
fn test_c2Add_Sub_Mulvs_Div() {
    let (c, r) = libs();
    type FVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
    type FVS = unsafe extern "C" fn(c2v, f32) -> c2v;
    let pairs = [
        (c2v { x: 1.0, y: 2.0 }, c2v { x: 3.0, y: 4.0 }),
        (c2v { x: -1.0, y: -2.0 }, c2v { x: 0.0, y: 7.0 }),
    ];
    let scalar_pairs = [
        (c2v { x: 1.0, y: 2.0 }, 3.0_f32),
        (c2v { x: -2.5, y: 4.5 }, -2.0_f32),
        (c2v { x: 1e6, y: 1e-6 }, 7.0_f32),
    ];
    unsafe {
        for name in ["c2Add", "c2Sub"] {
            let cf: Symbol<FVV> = c.get(name.as_bytes()).unwrap();
            let rf: Symbol<FVV> = r.get(name.as_bytes()).unwrap();
            for (a, b) in pairs {
                assert!(vec_eq(cf(a, b), rf(a, b)), "{}", name);
            }
        }
        for name in ["c2Mulvs", "c2Div"] {
            let cf: Symbol<FVS> = c.get(name.as_bytes()).unwrap();
            let rf: Symbol<FVS> = r.get(name.as_bytes()).unwrap();
            for (a, s) in scalar_pairs {
                assert!(vec_eq(cf(a, s), rf(a, s)), "{}", name);
            }
        }
    }
}

#[test]
fn test_c2Norm() {
    let (c, r) = libs();
    type F = unsafe extern "C" fn(c2v) -> c2v;
    unsafe {
        let cf: Symbol<F> = c.get(b"c2Norm").unwrap();
        let rf: Symbol<F> = r.get(b"c2Norm").unwrap();
        for v in [
            c2v { x: 3.0, y: 4.0 },
            c2v { x: -7.0, y: 24.0 },
            c2v { x: 1.0, y: 0.0 },
        ] {
            assert!(vec_eq(cf(v), rf(v)));
        }
    }
}

#[test]
fn test_c2Min_Max_Skew_Absv_CCW90() {
    let (c, r) = libs();
    type FVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
    type FV = unsafe extern "C" fn(c2v) -> c2v;
    let pairs = [
        (c2v { x: 1.0, y: 5.0 }, c2v { x: 3.0, y: 2.0 }),
        (c2v { x: -1.0, y: -2.0 }, c2v { x: 0.0, y: 7.0 }),
    ];
    let singles = [
        c2v { x: 1.0, y: 2.0 },
        c2v { x: -3.0, y: 4.0 },
        c2v { x: 0.0, y: 0.0 },
    ];
    unsafe {
        for name in ["c2Minv", "c2Maxv"] {
            let cf: Symbol<FVV> = c.get(name.as_bytes()).unwrap();
            let rf: Symbol<FVV> = r.get(name.as_bytes()).unwrap();
            for (a, b) in pairs {
                assert!(vec_eq(cf(a, b), rf(a, b)), "{}", name);
            }
        }
        for name in ["c2Skew", "c2Absv", "c2CCW90"] {
            let cf: Symbol<FV> = c.get(name.as_bytes()).unwrap();
            let rf: Symbol<FV> = r.get(name.as_bytes()).unwrap();
            for v in singles {
                assert!(vec_eq(cf(v), rf(v)), "{}", name);
            }
        }
    }
}

#[test]
fn test_c2MulmvT() {
    let (c, r) = libs();
    type F = unsafe extern "C" fn(c2m, c2v) -> c2v;
    unsafe {
        let cf: Symbol<F> = c.get(b"c2MulmvT").unwrap();
        let rf: Symbol<F> = r.get(b"c2MulmvT").unwrap();
        let m = c2m {
            x: c2v { x: 1.0, y: 2.0 },
            y: c2v { x: 3.0, y: 4.0 },
        };
        for v in [
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: -2.0, y: 3.5 },
        ] {
            assert!(vec_eq(cf(m, v), rf(m, v)));
        }
    }
}

#[test]
fn test_c2AABBtoAABB() {
    let (c, r) = libs();
    type F = unsafe extern "C" fn(c2AABB, c2AABB) -> i32;
    unsafe {
        let cf: Symbol<F> = c.get(b"c2AABBtoAABB").unwrap();
        let rf: Symbol<F> = r.get(b"c2AABBtoAABB").unwrap();
        let aabb = |min_x, min_y, max_x, max_y| c2AABB {
            min: c2v { x: min_x, y: min_y },
            max: c2v { x: max_x, y: max_y },
        };
        let cases = [
            (aabb(0.0, 0.0, 1.0, 1.0), aabb(0.5, 0.5, 1.5, 1.5)),
            (aabb(0.0, 0.0, 1.0, 1.0), aabb(2.0, 2.0, 3.0, 3.0)),
            (aabb(0.0, 0.0, 1.0, 1.0), aabb(0.0, 2.0, 1.0, 3.0)),
        ];
        for (a, b) in cases {
            assert_eq!(cf(a, b), rf(a, b));
        }
    }
}

#[test]
fn test_c2AABBtoPoint() {
    let (c, r) = libs();
    type F = unsafe extern "C" fn(c2AABB, c2v) -> i32;
    unsafe {
        let cf: Symbol<F> = c.get(b"c2AABBtoPoint").unwrap();
        let rf: Symbol<F> = r.get(b"c2AABBtoPoint").unwrap();
        let bb = c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 1.0, y: 1.0 },
        };
        for v in [
            c2v { x: 0.5, y: 0.5 },
            c2v { x: -1.0, y: 0.5 },
            c2v { x: 1.5, y: 0.5 },
            c2v { x: 0.5, y: 1.5 },
        ] {
            assert_eq!(cf(bb, v), rf(bb, v));
        }
    }
}

#[test]
fn test_c2CircleToPoint() {
    let (c, r) = libs();
    type F = unsafe extern "C" fn(c2Circle, c2v) -> i32;
    unsafe {
        let cf: Symbol<F> = c.get(b"c2CircleToPoint").unwrap();
        let rf: Symbol<F> = r.get(b"c2CircleToPoint").unwrap();
        let circle = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r: 1.0,
        };
        for v in [
            c2v { x: 0.5, y: 0.5 },
            c2v { x: 1.5, y: 0.0 },
            c2v { x: 0.0, y: 0.0 },
        ] {
            assert_eq!(cf(circle, v), rf(circle, v));
        }
    }
}

#[test]
fn test_c2RaytoCircle() {
    let (c, r) = libs();
    type F = unsafe extern "C" fn(c2Ray, c2Circle, *mut c2Raycast) -> i32;
    unsafe {
        let cf: Symbol<F> = c.get(b"c2RaytoCircle").unwrap();
        let rf: Symbol<F> = r.get(b"c2RaytoCircle").unwrap();
        let circle = c2Circle {
            p: c2v { x: 5.0, y: 0.0 },
            r: 1.0,
        };
        let cases = [
            // hit case
            c2Ray {
                p: c2v { x: 0.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            // miss case
            c2Ray {
                p: c2v { x: 0.0, y: 5.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            // parallel miss
            c2Ray {
                p: c2v { x: 0.0, y: 0.0 },
                d: c2v { x: 0.0, y: 1.0 },
                t: 10.0,
            },
        ];
        for ray in cases {
            let mut a = c2Raycast {
                t: 0.0,
                n: c2v { x: 0.0, y: 0.0 },
            };
            let mut b = c2Raycast {
                t: 0.0,
                n: c2v { x: 0.0, y: 0.0 },
            };
            let ra = cf(ray, circle, &mut a);
            let rb = rf(ray, circle, &mut b);
            assert_eq!(ra, rb);
            if ra != 0 {
                assert!(raycast_eq(a, b));
            }
        }
    }
}

#[test]
fn test_c2RaytoAABB() {
    let (c, r) = libs();
    type F = unsafe extern "C" fn(c2Ray, c2AABB, *mut c2Raycast) -> i32;
    unsafe {
        let cf: Symbol<F> = c.get(b"c2RaytoAABB").unwrap();
        let rf: Symbol<F> = r.get(b"c2RaytoAABB").unwrap();
        let bb = c2AABB {
            min: c2v { x: 4.0, y: -1.0 },
            max: c2v { x: 6.0, y: 1.0 },
        };
        let cases = [
            c2Ray {
                p: c2v { x: 0.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Ray {
                p: c2v { x: 0.0, y: 5.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Ray {
                p: c2v { x: 5.0, y: -5.0 },
                d: c2v { x: 0.0, y: 1.0 },
                t: 10.0,
            },
        ];
        for ray in cases {
            let mut a = c2Raycast {
                t: 0.0,
                n: c2v { x: 0.0, y: 0.0 },
            };
            let mut b = c2Raycast {
                t: 0.0,
                n: c2v { x: 0.0, y: 0.0 },
            };
            let ra = cf(ray, bb, &mut a);
            let rb = rf(ray, bb, &mut b);
            assert_eq!(ra, rb);
            if ra != 0 {
                assert!(raycast_eq(a, b), "{:?} vs {:?}", a, b);
            }
        }
    }
}

#[test]
fn test_c2RaytoCapsule() {
    let (c, r) = libs();
    type F = unsafe extern "C" fn(c2Ray, c2Capsule, *mut c2Raycast) -> i32;
    unsafe {
        let cf: Symbol<F> = c.get(b"c2RaytoCapsule").unwrap();
        let rf: Symbol<F> = r.get(b"c2RaytoCapsule").unwrap();
        let cap = c2Capsule {
            a: c2v { x: 4.0, y: -2.0 },
            b: c2v { x: 4.0, y: 2.0 },
            r: 0.5,
        };
        let cases = [
            c2Ray {
                p: c2v { x: 0.0, y: 0.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Ray {
                p: c2v { x: 0.0, y: 5.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Ray {
                p: c2v { x: 0.0, y: -5.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
            c2Ray {
                p: c2v { x: 0.0, y: 10.0 },
                d: c2v { x: 1.0, y: 0.0 },
                t: 10.0,
            },
        ];
        for ray in cases {
            let mut a = c2Raycast {
                t: 0.0,
                n: c2v { x: 0.0, y: 0.0 },
            };
            let mut b = c2Raycast {
                t: 0.0,
                n: c2v { x: 0.0, y: 0.0 },
            };
            let ra = cf(ray, cap, &mut a);
            let rb = rf(ray, cap, &mut b);
            assert_eq!(ra, rb);
            assert!(raycast_eq(a, b), "{:?} vs {:?}", a, b);
        }
    }
}

#[test]
fn test_c2CastRay() {
    let (c, r) = libs();
    type F = unsafe extern "C" fn(c2Ray, *const core::ffi::c_void, C2_TYPE, *mut c2Raycast) -> i32;
    unsafe {
        let cf: Symbol<F> = c.get(b"c2CastRay").unwrap();
        let rf: Symbol<F> = r.get(b"c2CastRay").unwrap();
        let ray = c2Ray {
            p: c2v { x: 0.0, y: 0.0 },
            d: c2v { x: 1.0, y: 0.0 },
            t: 10.0,
        };
        let circle = c2Circle {
            p: c2v { x: 5.0, y: 0.0 },
            r: 1.0,
        };
        let bb = c2AABB {
            min: c2v { x: 4.0, y: -1.0 },
            max: c2v { x: 6.0, y: 1.0 },
        };
        let cap = c2Capsule {
            a: c2v { x: 4.0, y: -2.0 },
            b: c2v { x: 4.0, y: 2.0 },
            r: 0.5,
        };

        let mut a = c2Raycast {
            t: 0.0,
            n: c2v { x: 0.0, y: 0.0 },
        };
        let mut b = c2Raycast {
            t: 0.0,
            n: c2v { x: 0.0, y: 0.0 },
        };
        let ra = cf(
            ray,
            &circle as *const c2Circle as *const core::ffi::c_void,
            C2_TYPE::C2_TYPE_CIRCLE,
            &mut a,
        );
        let rb = rf(
            ray,
            &circle as *const c2Circle as *const core::ffi::c_void,
            C2_TYPE::C2_TYPE_CIRCLE,
            &mut b,
        );
        assert_eq!(ra, rb);
        assert!(raycast_eq(a, b));

        let ra = cf(
            ray,
            &bb as *const c2AABB as *const core::ffi::c_void,
            C2_TYPE::C2_TYPE_AABB,
            &mut a,
        );
        let rb = rf(
            ray,
            &bb as *const c2AABB as *const core::ffi::c_void,
            C2_TYPE::C2_TYPE_AABB,
            &mut b,
        );
        assert_eq!(ra, rb);
        assert!(raycast_eq(a, b));

        let ra = cf(
            ray,
            &cap as *const c2Capsule as *const core::ffi::c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
            &mut a,
        );
        let rb = rf(
            ray,
            &cap as *const c2Capsule as *const core::ffi::c_void,
            C2_TYPE::C2_TYPE_CAPSULE,
            &mut b,
        );
        assert_eq!(ra, rb);
        assert!(raycast_eq(a, b));
    }
}

#[test]
fn test_gen_ray() {
    let (c, r) = libs();
    type F = unsafe extern "C" fn(
        *mut c2Raycast,
        *mut c2Raycast,
        *mut c2Raycast,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
    ) -> i32;
    unsafe {
        let cf: Symbol<F> = c.get(b"gen_ray").unwrap();
        let rf: Symbol<F> = r.get(b"gen_ray").unwrap();
        let cases: &[[f32; 16]] = &[
            // mp_x, mp_y, r_p_x, r_p_y, c_p_x, c_p_y, c_r,
            // cap_a_x, cap_a_y, cap_b_x, cap_b_y, cap_r,
            // bb_min_x, bb_min_y, bb_max_x, bb_max_y
            [10.0, 0.0, 0.0, 0.0, 5.0, 0.0, 1.0, 4.0, -2.0, 4.0, 2.0, 0.5, 4.0, -1.0, 6.0, 1.0],
            [10.0, 5.0, 0.0, 5.0, 5.0, 0.0, 1.0, 4.0, -2.0, 4.0, 2.0, 0.5, 4.0, -1.0, 6.0, 1.0],
            [-10.0, -10.0, 0.0, 0.0, 5.0, 0.0, 1.0, 4.0, -2.0, 4.0, 2.0, 0.5, 4.0, -1.0, 6.0, 1.0],
        ];
        for case in cases {
            let mut ca1 = c2Raycast { t: 0.0, n: c2v { x: 0.0, y: 0.0 } };
            let mut ca2 = c2Raycast { t: 0.0, n: c2v { x: 0.0, y: 0.0 } };
            let mut ca3 = c2Raycast { t: 0.0, n: c2v { x: 0.0, y: 0.0 } };
            let mut ra1 = c2Raycast { t: 0.0, n: c2v { x: 0.0, y: 0.0 } };
            let mut ra2 = c2Raycast { t: 0.0, n: c2v { x: 0.0, y: 0.0 } };
            let mut ra3 = c2Raycast { t: 0.0, n: c2v { x: 0.0, y: 0.0 } };
            let cr = cf(
                &mut ca1, &mut ca2, &mut ca3,
                case[0], case[1], case[2], case[3], case[4], case[5], case[6],
                case[7], case[8], case[9], case[10], case[11], case[12], case[13],
                case[14], case[15],
            );
            let rr = rf(
                &mut ra1, &mut ra2, &mut ra3,
                case[0], case[1], case[2], case[3], case[4], case[5], case[6],
                case[7], case[8], case[9], case[10], case[11], case[12], case[13],
                case[14], case[15],
            );
            assert_eq!(cr, rr);
            // Compare casts only when corresponding hit bit is set
            if cr & 1 != 0 {
                assert!(raycast_eq(ca1, ra1));
            }
            if cr & 2 != 0 {
                assert!(raycast_eq(ca2, ra2));
            }
            if cr & 4 != 0 {
                assert!(raycast_eq(ca3, ra3));
            }
        }
    }
}
