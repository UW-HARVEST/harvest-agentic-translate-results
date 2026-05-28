use libloading::{Library, Symbol};
use std::path::PathBuf;
use std::sync::OnceLock;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct LmVec2 {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CnRnd {
    state: [u64; 2],
}

const C_TYPE_CIRCLE: i32 = 0;
const C_TYPE_AABB: i32 = 1;

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    p
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Determine build profile
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    p.push(profile);
    p.push("libagglom_lib.so");
    p
}

struct Libs {
    c: Library,
    rust: Library,
}

fn libs() -> &'static Libs {
    static L: OnceLock<Libs> = OnceLock::new();
    L.get_or_init(|| unsafe {
        let c = Library::new(c_lib_path()).expect("failed to load C .so");
        let rust = Library::new(rust_lib_path()).expect("failed to load Rust .so");
        Libs { c, rust }
    })
}

// ----- c2V -----
#[test]
fn test_c2V() {
    let l = libs();
    type Fn_ = unsafe extern "C" fn(f32, f32) -> C2v;
    let cf: Symbol<Fn_> = unsafe { l.c.get(b"c2V").unwrap() };
    let rf: Symbol<Fn_> = unsafe { l.rust.get(b"c2V").unwrap() };
    let cases: &[(f32, f32)] = &[
        (0.0, 0.0),
        (1.0, -1.0),
        (1e10, -1e-10),
        (f32::INFINITY, f32::NEG_INFINITY),
    ];
    for &(x, y) in cases {
        let cr = unsafe { cf(x, y) };
        let rr = unsafe { rf(x, y) };
        assert_eq!(cr.x.to_bits(), rr.x.to_bits(), "x mismatch for ({}, {})", x, y);
        assert_eq!(cr.y.to_bits(), rr.y.to_bits(), "y mismatch for ({}, {})", x, y);
    }
}

// ----- c2Maxv / c2Minv / c2Clampv / c2Sub / c2Dot -----
#[test]
fn test_c2_vec_ops() {
    let l = libs();
    type Bin = unsafe extern "C" fn(C2v, C2v) -> C2v;
    type Tri = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
    type Dot = unsafe extern "C" fn(C2v, C2v) -> f32;

    let cmax: Symbol<Bin> = unsafe { l.c.get(b"c2Maxv").unwrap() };
    let rmax: Symbol<Bin> = unsafe { l.rust.get(b"c2Maxv").unwrap() };
    let cmin: Symbol<Bin> = unsafe { l.c.get(b"c2Minv").unwrap() };
    let rmin: Symbol<Bin> = unsafe { l.rust.get(b"c2Minv").unwrap() };
    let cclamp: Symbol<Tri> = unsafe { l.c.get(b"c2Clampv").unwrap() };
    let rclamp: Symbol<Tri> = unsafe { l.rust.get(b"c2Clampv").unwrap() };
    let csub: Symbol<Bin> = unsafe { l.c.get(b"c2Sub").unwrap() };
    let rsub: Symbol<Bin> = unsafe { l.rust.get(b"c2Sub").unwrap() };
    let cdot: Symbol<Dot> = unsafe { l.c.get(b"c2Dot").unwrap() };
    let rdot: Symbol<Dot> = unsafe { l.rust.get(b"c2Dot").unwrap() };

    let cases: &[(C2v, C2v)] = &[
        (C2v { x: 0.0, y: 0.0 }, C2v { x: 0.0, y: 0.0 }),
        (C2v { x: 1.0, y: 2.0 }, C2v { x: 3.0, y: -1.0 }),
        (C2v { x: -5.0, y: 7.0 }, C2v { x: 5.0, y: -7.0 }),
        (C2v { x: 1e10, y: -1e10 }, C2v { x: 1e-10, y: -1e-10 }),
    ];
    for &(a, b) in cases {
        let c1 = unsafe { cmax(a, b) };
        let r1 = unsafe { rmax(a, b) };
        assert_eq!(c1.x.to_bits(), r1.x.to_bits());
        assert_eq!(c1.y.to_bits(), r1.y.to_bits());
        let c2 = unsafe { cmin(a, b) };
        let r2 = unsafe { rmin(a, b) };
        assert_eq!(c2.x.to_bits(), r2.x.to_bits());
        assert_eq!(c2.y.to_bits(), r2.y.to_bits());
        let c3 = unsafe { csub(a, b) };
        let r3 = unsafe { rsub(a, b) };
        assert_eq!(c3.x.to_bits(), r3.x.to_bits());
        assert_eq!(c3.y.to_bits(), r3.y.to_bits());
        let c4 = unsafe { cdot(a, b) };
        let r4 = unsafe { rdot(a, b) };
        assert_eq!(c4.to_bits(), r4.to_bits());
    }

    // Clampv
    let lo = C2v { x: -2.0, y: -2.0 };
    let hi = C2v { x: 2.0, y: 2.0 };
    for v in cases.iter().map(|(a, _)| *a) {
        let c1 = unsafe { cclamp(v, lo, hi) };
        let r1 = unsafe { rclamp(v, lo, hi) };
        assert_eq!(c1.x.to_bits(), r1.x.to_bits());
        assert_eq!(c1.y.to_bits(), r1.y.to_bits());
    }
}

// ----- c2CircletoCircle / c2CircletoAABB / c2AABBtoAABB -----
#[test]
fn test_c2_collision() {
    let l = libs();
    type CC = unsafe extern "C" fn(C2Circle, C2Circle) -> i32;
    type CA = unsafe extern "C" fn(C2Circle, C2AABB) -> i32;
    type AA = unsafe extern "C" fn(C2AABB, C2AABB) -> i32;
    let ccc: Symbol<CC> = unsafe { l.c.get(b"c2CircletoCircle").unwrap() };
    let rcc: Symbol<CC> = unsafe { l.rust.get(b"c2CircletoCircle").unwrap() };
    let cca: Symbol<CA> = unsafe { l.c.get(b"c2CircletoAABB").unwrap() };
    let rca: Symbol<CA> = unsafe { l.rust.get(b"c2CircletoAABB").unwrap() };
    let caa: Symbol<AA> = unsafe { l.c.get(b"c2AABBtoAABB").unwrap() };
    let raa: Symbol<AA> = unsafe { l.rust.get(b"c2AABBtoAABB").unwrap() };

    let circles = [
        C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 },
        C2Circle { p: C2v { x: 0.5, y: 0.0 }, r: 1.0 },
        C2Circle { p: C2v { x: 5.0, y: 5.0 }, r: 0.5 },
        C2Circle { p: C2v { x: -3.0, y: 2.0 }, r: 2.0 },
    ];
    let aabbs = [
        C2AABB { min: C2v { x: -1.0, y: -1.0 }, max: C2v { x: 1.0, y: 1.0 } },
        C2AABB { min: C2v { x: 4.0, y: 4.0 }, max: C2v { x: 6.0, y: 6.0 } },
        C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 0.5, y: 0.5 } },
    ];
    for &a in &circles {
        for &b in &circles {
            assert_eq!(unsafe { ccc(a, b) }, unsafe { rcc(a, b) });
        }
        for &b in &aabbs {
            assert_eq!(unsafe { cca(a, b) }, unsafe { rca(a, b) });
        }
    }
    for &a in &aabbs {
        for &b in &aabbs {
            assert_eq!(unsafe { caa(a, b) }, unsafe { raa(a, b) });
        }
    }
}

// ----- f2 -----
#[test]
fn test_f2() {
    let l = libs();
    type F2 = unsafe extern "C" fn(*const u8, i32, *const u8, i32) -> i32;
    let cf: Symbol<F2> = unsafe { l.c.get(b"f2").unwrap() };
    let rf: Symbol<F2> = unsafe { l.rust.get(b"f2").unwrap() };

    let circles = [
        C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 },
        C2Circle { p: C2v { x: 5.0, y: 5.0 }, r: 0.5 },
    ];
    let aabbs = [
        C2AABB { min: C2v { x: -1.0, y: -1.0 }, max: C2v { x: 1.0, y: 1.0 } },
        C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 0.5, y: 0.5 } },
    ];

    for c in &circles {
        for c2 in &circles {
            let a = c as *const _ as *const u8;
            let b = c2 as *const _ as *const u8;
            assert_eq!(unsafe { cf(a, C_TYPE_CIRCLE, b, C_TYPE_CIRCLE) }, unsafe {
                rf(a, C_TYPE_CIRCLE, b, C_TYPE_CIRCLE)
            });
        }
        for ab in &aabbs {
            let a = c as *const _ as *const u8;
            let b = ab as *const _ as *const u8;
            assert_eq!(unsafe { cf(a, C_TYPE_CIRCLE, b, C_TYPE_AABB) }, unsafe {
                rf(a, C_TYPE_CIRCLE, b, C_TYPE_AABB)
            });
            assert_eq!(unsafe { cf(b, C_TYPE_AABB, a, C_TYPE_CIRCLE) }, unsafe {
                rf(b, C_TYPE_AABB, a, C_TYPE_CIRCLE)
            });
        }
    }
    for a1 in &aabbs {
        for a2 in &aabbs {
            let a = a1 as *const _ as *const u8;
            let b = a2 as *const _ as *const u8;
            assert_eq!(unsafe { cf(a, C_TYPE_AABB, b, C_TYPE_AABB) }, unsafe {
                rf(a, C_TYPE_AABB, b, C_TYPE_AABB)
            });
        }
    }
}

// ----- f3 -----
#[test]
fn test_f3() {
    let l = libs();
    type F3 = unsafe extern "C" fn(i32, i32) -> i32;
    let cf: Symbol<F3> = unsafe { l.c.get(b"f3").unwrap() };
    let rf: Symbol<F3> = unsafe { l.rust.get(b"f3").unwrap() };

    let cases = [
        (10, 3),
        (10, -3),
        (-10, 3),
        (-10, -3),
        (0, 5),
        (5, 0),
        (i32::MIN, 1),
        (i32::MIN, -1),
        (i32::MIN, i32::MIN),
        (i32::MIN, 2),
        (i32::MAX, 1),
        (i32::MAX, -1),
        (1, i32::MIN),
        (-1, i32::MIN),
        (100, 7),
        (100, -7),
        (-100, 7),
        (-100, -7),
        (i32::MAX, i32::MIN),
    ];
    for &(a, b) in &cases {
        let cr = unsafe { cf(a, b) };
        let rr = unsafe { rf(a, b) };
        assert_eq!(cr, rr, "f3({}, {}) C={} Rust={}", a, b, cr, rr);
    }
}

// ----- f4 -----
#[test]
fn test_f4() {
    let l = libs();
    type F4 = unsafe extern "C" fn(*mut CnRnd) -> f64;
    let cf: Symbol<F4> = unsafe { l.c.get(b"f4").unwrap() };
    let rf: Symbol<F4> = unsafe { l.rust.get(b"f4").unwrap() };

    let seeds = [
        [0u64, 1u64],
        [0xdeadbeef, 0xcafebabe],
        [u64::MAX, u64::MAX],
        [123, 456],
        [1, 1],
        [0, 0xffffffffffffffff],
    ];
    for &s in &seeds {
        let mut c_state = CnRnd { state: s };
        let mut r_state = CnRnd { state: s };
        let cr = unsafe { cf(&mut c_state) };
        let rr = unsafe { rf(&mut r_state) };
        assert_eq!(cr.to_bits(), rr.to_bits(), "f4 seed {:?}", s);
        assert_eq!(c_state.state, r_state.state, "f4 state seed {:?}", s);
    }
}

// ----- f5 -----
#[test]
fn test_f5() {
    let l = libs();
    type F5 = unsafe extern "C" fn(u32) -> u32;
    let cf: Symbol<F5> = unsafe { l.c.get(b"f5").unwrap() };
    let rf: Symbol<F5> = unsafe { l.rust.get(b"f5").unwrap() };

    let cases = [0u32, 1, 0xFFFF, 0x1234, 0xABCD, 0x8000, 0xFFFFFFFF, 0xAAAA, 0x5555, 0xFF00FF00];
    for &v in &cases {
        assert_eq!(unsafe { cf(v) }, unsafe { rf(v) }, "f5({})", v);
    }
}

// ----- f7 -----
#[test]
fn test_f7() {
    let l = libs();
    type F7 = unsafe extern "C" fn(u32, u32, u32) -> u32;
    let cf: Symbol<F7> = unsafe { l.c.get(b"f7").unwrap() };
    let rf: Symbol<F7> = unsafe { l.rust.get(b"f7").unwrap() };
    let cases = [
        (4096u32, 1u32, 16u32),
        (4096, 2, 16),
        (4096, 2, 24),
        (4096, 2, 32),
        (4096, 4, 16),
        (4096, 8, 32),
        (1, 1, 1),
        (0, 0, 0),
        (256, 6, 24),
    ];
    for &(a, b, c) in &cases {
        assert_eq!(unsafe { cf(a, b, c) }, unsafe { rf(a, b, c) }, "f7({},{},{})", a, b, c);
    }
}

// ----- f9 -----
#[test]
fn test_f9() {
    let l = libs();
    type F9 = unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;
    let cf: Symbol<F9> = unsafe { l.c.get(b"f9").unwrap() };
    let rf: Symbol<F9> = unsafe { l.rust.get(b"f9").unwrap() };
    let cases = [
        (
            LmVec2 { x: 0.0, y: 0.0 },
            LmVec2 { x: 1.0, y: 0.0 },
            LmVec2 { x: 0.0, y: 1.0 },
            LmVec2 { x: 0.5, y: 0.5 },
        ),
        (
            LmVec2 { x: -1.0, y: -1.0 },
            LmVec2 { x: 1.0, y: -1.0 },
            LmVec2 { x: 0.0, y: 1.0 },
            LmVec2 { x: 0.0, y: 0.0 },
        ),
        (
            LmVec2 { x: 5.0, y: 5.0 },
            LmVec2 { x: 10.0, y: 5.0 },
            LmVec2 { x: 5.0, y: 10.0 },
            LmVec2 { x: 7.0, y: 7.0 },
        ),
    ];
    for &(p1, p2, p3, p) in &cases {
        let cr = unsafe { cf(p1, p2, p3, p) };
        let rr = unsafe { rf(p1, p2, p3, p) };
        // Use bit-exact comparison; both should produce same output
        if cr.x.is_nan() {
            assert!(rr.x.is_nan());
        } else {
            assert_eq!(cr.x.to_bits(), rr.x.to_bits());
        }
        if cr.y.is_nan() {
            assert!(rr.y.is_nan());
        } else {
            assert_eq!(cr.y.to_bits(), rr.y.to_bits());
        }
    }
}

// ----- f10 -----
#[test]
fn test_f10() {
    let l = libs();
    type F10 = unsafe extern "C" fn(u16) -> f32;
    let cf: Symbol<F10> = unsafe { l.c.get(b"f10").unwrap() };
    let rf: Symbol<F10> = unsafe { l.rust.get(b"f10").unwrap() };
    // Test all 65536 possible u16 inputs
    for h in 0u32..=u16::MAX as u32 {
        let h = h as u16;
        let cr = unsafe { cf(h) };
        let rr = unsafe { rf(h) };
        if cr.is_nan() {
            assert!(rr.is_nan(), "f10({:#06x}) C=NaN, Rust={:?}", h, rr);
        } else {
            assert_eq!(
                cr.to_bits(),
                rr.to_bits(),
                "f10({:#06x}) C={:?} Rust={:?}",
                h,
                cr,
                rr
            );
        }
    }
}

// ----- f11 (HSL->RGB) -----
#[test]
fn test_f11() {
    let l = libs();
    type F11 = unsafe extern "C" fn(*mut f32, *const f32);
    let cf: Symbol<F11> = unsafe { l.c.get(b"f11").unwrap() };
    let rf: Symbol<F11> = unsafe { l.rust.get(b"f11").unwrap() };

    let cases: &[[f32; 3]] = &[
        [0.0, 0.0, 0.5],   // s=0, gray
        [30.0, 0.5, 0.5],  // < 60
        [90.0, 0.7, 0.4],  // 60..120
        [150.0, 0.7, 0.4], // 120..180 (note original C bug)
        [210.0, 0.5, 0.5], // 180..240
        [270.0, 0.5, 0.5], // 240..300
        [330.0, 0.5, 0.5], // 300..360
        [400.0, 0.5, 0.5], // out of range
        [-30.0, 0.5, 0.5], // negative
        [0.0, 1.0, 1.0],
        [60.0, 1.0, 0.5],
    ];
    for src in cases {
        let mut cdest = [0.0f32; 3];
        let mut rdest = [0.0f32; 3];
        unsafe { cf(cdest.as_mut_ptr(), src.as_ptr()) };
        unsafe { rf(rdest.as_mut_ptr(), src.as_ptr()) };
        for i in 0..3 {
            if cdest[i].is_nan() {
                assert!(rdest[i].is_nan());
            } else {
                assert_eq!(
                    cdest[i].to_bits(),
                    rdest[i].to_bits(),
                    "f11 src={:?} idx={} C={:?} R={:?}",
                    src,
                    i,
                    cdest,
                    rdest
                );
            }
        }
    }
}

// ----- f12 (HSV->RGB) -----
#[test]
fn test_f12() {
    let l = libs();
    type F12 = unsafe extern "C" fn(*mut f32, *const f32);
    let cf: Symbol<F12> = unsafe { l.c.get(b"f12").unwrap() };
    let rf: Symbol<F12> = unsafe { l.rust.get(b"f12").unwrap() };

    let cases: &[[f32; 3]] = &[
        [0.0, 0.0, 0.5],
        [30.0, 0.5, 0.5],
        [90.0, 0.7, 0.4],
        [150.0, 0.7, 0.4],
        [210.0, 0.5, 0.5],
        [270.0, 0.5, 0.5],
        [330.0, 0.5, 0.5],
        [60.0, 1.0, 1.0],
        [120.0, 1.0, 1.0],
        [180.0, 1.0, 1.0],
        [240.0, 1.0, 1.0],
        [300.0, 1.0, 1.0],
        [359.99, 1.0, 1.0],
    ];
    for src in cases {
        let mut cdest = [0.0f32; 3];
        let mut rdest = [0.0f32; 3];
        unsafe { cf(cdest.as_mut_ptr(), src.as_ptr()) };
        unsafe { rf(rdest.as_mut_ptr(), src.as_ptr()) };
        for i in 0..3 {
            if cdest[i].is_nan() {
                assert!(rdest[i].is_nan());
            } else {
                assert_eq!(cdest[i].to_bits(), rdest[i].to_bits());
            }
        }
    }
}

// ----- f13 (RGB->HSV) -----
#[test]
fn test_f13() {
    let l = libs();
    type F13 = unsafe extern "C" fn(*mut f32, *const f32);
    let cf: Symbol<F13> = unsafe { l.c.get(b"f13").unwrap() };
    let rf: Symbol<F13> = unsafe { l.rust.get(b"f13").unwrap() };

    let cases: &[[f32; 3]] = &[
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.5, 0.5, 0.5],
        [0.7, 0.3, 0.1],
        [0.1, 0.7, 0.3],
        [0.3, 0.1, 0.7],
        [0.5, 0.0, 0.5],
    ];
    for src in cases {
        let mut cdest = [0.0f32; 3];
        let mut rdest = [0.0f32; 3];
        unsafe { cf(cdest.as_mut_ptr(), src.as_ptr()) };
        unsafe { rf(rdest.as_mut_ptr(), src.as_ptr()) };
        for i in 0..3 {
            if cdest[i].is_nan() {
                assert!(rdest[i].is_nan());
            } else {
                assert_eq!(
                    cdest[i].to_bits(),
                    rdest[i].to_bits(),
                    "f13 src={:?} idx={} C={:?} R={:?}",
                    src,
                    i,
                    cdest,
                    rdest
                );
            }
        }
    }
}

// ----- agglom (top-level) -----
#[test]
fn test_agglom() {
    let l = libs();
    type Agglom = unsafe extern "C" fn(
        f32, f32, f32, f32, f32, f32, f32,
        i32, i32,
        u64, u64,
        u32,
        u32, u32, u32,
        f32, f32, f32, f32, f32, f32, f32, f32,
        u16,
        f32, f32, f32,
        f32, f32, f32,
        f32, f32, f32,
    ) -> f64;
    let cf: Symbol<Agglom> = unsafe { l.c.get(b"agglom").unwrap() };
    let rf: Symbol<Agglom> = unsafe { l.rust.get(b"agglom").unwrap() };

    let test_args: &[(f32, f32, f32, f32, f32, f32, f32,
                      i32, i32,
                      u64, u64,
                      u32,
                      u32, u32, u32,
                      f32, f32, f32, f32, f32, f32, f32, f32,
                      u16,
                      f32, f32, f32,
                      f32, f32, f32,
                      f32, f32, f32)] = &[
        (
            0.0, 0.0, 1.0, -1.0, -1.0, 1.0, 1.0,
            10, 3,
            0xdeadbeef, 0xcafebabe,
            0x1234,
            4096, 2, 24,
            0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.5, 0.5,
            0x3c00,
            30.0, 0.5, 0.5,
            120.0, 1.0, 0.5,
            0.7, 0.3, 0.1,
        ),
        (
            5.0, 5.0, 0.5, 4.0, 4.0, 6.0, 6.0,
            -100, 7,
            0, 1,
            0xFFFFFFFF,
            256, 6, 24,
            0.0, 0.0, 2.0, 0.0, 0.0, 2.0, 1.0, 1.0,
            0x7BFF,
            240.0, 1.0, 0.3,
            60.0, 1.0, 1.0,
            0.1, 0.5, 0.9,
        ),
    ];
    for args in test_args {
        let cr = unsafe {
            cf(args.0, args.1, args.2, args.3, args.4, args.5, args.6,
               args.7, args.8,
               args.9, args.10,
               args.11,
               args.12, args.13, args.14,
               args.15, args.16, args.17, args.18, args.19, args.20, args.21, args.22,
               args.23,
               args.24, args.25, args.26,
               args.27, args.28, args.29,
               args.30, args.31, args.32)
        };
        let rr = unsafe {
            rf(args.0, args.1, args.2, args.3, args.4, args.5, args.6,
               args.7, args.8,
               args.9, args.10,
               args.11,
               args.12, args.13, args.14,
               args.15, args.16, args.17, args.18, args.19, args.20, args.21, args.22,
               args.23,
               args.24, args.25, args.26,
               args.27, args.28, args.29,
               args.30, args.31, args.32)
        };
        assert_eq!(cr.to_bits(), rr.to_bits(), "agglom mismatch C={} R={}", cr, rr);
    }
}
