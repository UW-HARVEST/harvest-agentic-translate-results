use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libagglom_lib.so")
}

fn rust_lib_path() -> PathBuf {
    // Find the built Rust cdylib
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    dir.join("libagglom_lib.so")
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2v { x: f32, y: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Circle { p: c2v, r: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2AABB { min: c2v, max: c2v }

#[repr(C)]
struct cn_rnd_t { state: [u64; 2] }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct lm_vec2 { x: f32, y: f32 }

// ---- f3 tests ----
#[test]
fn test_f3() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_f3: Symbol<unsafe extern "C" fn(i32, i32) -> i32> = c_lib.get(b"f3").unwrap();
        let r_f3: Symbol<unsafe extern "C" fn(i32, i32) -> i32> = r_lib.get(b"f3").unwrap();

        let cases: &[(i32, i32)] = &[
            (10, 3), (-10, 3), (10, -3), (-10, -3),
            (0, 5), (5, 0), (i32::MIN, 1), (i32::MIN, -1),
            (i32::MIN, i32::MIN), (7, 7), (-7, -7),
            (1, i32::MIN), (i32::MAX, 2), (i32::MIN, 2),
            (100, 7), (-100, 7), (100, -7), (-100, -7),
        ];
        for &(v1, v2) in cases {
            let c_r = c_f3(v1, v2);
            let r_r = r_f3(v1, v2);
            assert_eq!(c_r, r_r, "f3({}, {}): C={}, Rust={}", v1, v2, c_r, r_r);
        }
    }
}

// ---- f5 tests ----
#[test]
fn test_f5() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_f5: Symbol<unsafe extern "C" fn(u32) -> u32> = c_lib.get(b"f5").unwrap();
        let r_f5: Symbol<unsafe extern "C" fn(u32) -> u32> = r_lib.get(b"f5").unwrap();

        let cases: &[u32] = &[0, 1, 0xFFFF, 0xAAAA, 0x5555, 0x1234, 0xABCD, 42];
        for &a in cases {
            let c_r = c_f5(a);
            let r_r = r_f5(a);
            assert_eq!(c_r, r_r, "f5(0x{:04x}): C=0x{:04x}, Rust=0x{:04x}", a, c_r, r_r);
        }
    }
}

// ---- f7 tests ----
#[test]
fn test_f7() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_f7: Symbol<unsafe extern "C" fn(u32, u32, u32) -> u32> = c_lib.get(b"f7").unwrap();
        let r_f7: Symbol<unsafe extern "C" fn(u32, u32, u32) -> u32> = r_lib.get(b"f7").unwrap();

        let cases: &[(u32, u32, u32)] = &[
            (4096, 2, 16), (1024, 1, 24), (4096, 2, 32),
            (256, 6, 16), (1, 1, 8), (4096, 2, 24),
        ];
        for &(bs, ch, bd) in cases {
            let c_r = c_f7(bs, ch, bd);
            let r_r = r_f7(bs, ch, bd);
            assert_eq!(c_r, r_r, "f7({},{},{}): C={}, Rust={}", bs, ch, bd, c_r, r_r);
        }
    }
}

// ---- f4 tests ----
#[test]
fn test_f4() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_f4: Symbol<unsafe extern "C" fn(*mut cn_rnd_t) -> f64> = c_lib.get(b"f4").unwrap();
        let r_f4: Symbol<unsafe extern "C" fn(*mut cn_rnd_t) -> f64> = r_lib.get(b"f4").unwrap();

        let seeds: &[(u64, u64)] = &[
            (12345, 67890), (0, 0), (u64::MAX, u64::MAX), (1, 1),
        ];
        for &(s0, s1) in seeds {
            let mut c_rnd = cn_rnd_t { state: [s0, s1] };
            let mut r_rnd = cn_rnd_t { state: [s0, s1] };
            for i in 0..5 {
                let c_r = c_f4(&mut c_rnd);
                let r_r = r_f4(&mut r_rnd);
                assert_eq!(c_r.to_bits(), r_r.to_bits(),
                    "f4 seed=({},{}), iter={}: C={}, Rust={}", s0, s1, i, c_r, r_r);
            }
        }
    }
}

// ---- f10 tests ----
#[test]
fn test_f10() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_f10: Symbol<unsafe extern "C" fn(u16) -> f32> = c_lib.get(b"f10").unwrap();
        let r_f10: Symbol<unsafe extern "C" fn(u16) -> f32> = r_lib.get(b"f10").unwrap();

        let cases: &[u16] = &[
            0, 0x3C00, 0x4000, 0x7BFF, 0x0001, 0x8000, 0xFC00, 0x3555,
        ];
        for &h in cases {
            let c_r = c_f10(h);
            let r_r = r_f10(h);
            assert_eq!(c_r.to_bits(), r_r.to_bits(),
                "f10(0x{:04x}): C={} (0x{:08x}), Rust={} (0x{:08x})",
                h, c_r, c_r.to_bits(), r_r, r_r.to_bits());
        }
    }
}

// ---- f9 tests ----
#[test]
fn test_f9() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_f9: Symbol<unsafe extern "C" fn(lm_vec2, lm_vec2, lm_vec2, lm_vec2) -> lm_vec2> =
            c_lib.get(b"f9").unwrap();
        let r_f9: Symbol<unsafe extern "C" fn(lm_vec2, lm_vec2, lm_vec2, lm_vec2) -> lm_vec2> =
            r_lib.get(b"f9").unwrap();

        let cases: &[(lm_vec2, lm_vec2, lm_vec2, lm_vec2)] = &[
            (lm_vec2{x:0.0,y:0.0}, lm_vec2{x:1.0,y:0.0}, lm_vec2{x:0.0,y:1.0}, lm_vec2{x:0.25,y:0.25}),
            (lm_vec2{x:1.0,y:1.0}, lm_vec2{x:4.0,y:1.0}, lm_vec2{x:1.0,y:5.0}, lm_vec2{x:2.0,y:3.0}),
        ];
        for (p1, p2, p3, p) in cases {
            let c_r = c_f9(*p1, *p2, *p3, *p);
            let r_r = r_f9(*p1, *p2, *p3, *p);
            assert_eq!(c_r.x.to_bits(), r_r.x.to_bits(), "f9 x mismatch");
            assert_eq!(c_r.y.to_bits(), r_r.y.to_bits(), "f9 y mismatch");
        }
    }
}

// ---- f11 tests ----
#[test]
fn test_f11() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_f11: Symbol<unsafe extern "C" fn(*mut f32, *const f32)> = c_lib.get(b"f11").unwrap();
        let r_f11: Symbol<unsafe extern "C" fn(*mut f32, *const f32)> = r_lib.get(b"f11").unwrap();

        let cases: &[[f32; 3]] = &[
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.5],
            [120.0, 1.0, 0.5],
            [240.0, 1.0, 0.5],
            [60.0, 0.5, 0.5],
            [180.0, 0.5, 0.5],
            [300.0, 0.5, 0.5],
            [359.0, 1.0, 0.5],
            [30.0, 0.8, 0.3],
            [90.0, 0.6, 0.7],
            [150.0, 0.4, 0.2],
            [210.0, 0.9, 0.4],
            [270.0, 0.3, 0.6],
            [330.0, 0.7, 0.8],
        ];
        for src in cases {
            let mut c_dest = [0.0f32; 3];
            let mut r_dest = [0.0f32; 3];
            c_f11(c_dest.as_mut_ptr(), src.as_ptr());
            r_f11(r_dest.as_mut_ptr(), src.as_ptr());
            for i in 0..3 {
                assert_eq!(c_dest[i].to_bits(), r_dest[i].to_bits(),
                    "f11({:?})[{}]: C={}, Rust={}", src, i, c_dest[i], r_dest[i]);
            }
        }
    }
}

// ---- f12 tests ----
#[test]
fn test_f12() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_f12: Symbol<unsafe extern "C" fn(*mut f32, *const f32)> = c_lib.get(b"f12").unwrap();
        let r_f12: Symbol<unsafe extern "C" fn(*mut f32, *const f32)> = r_lib.get(b"f12").unwrap();

        let cases: &[[f32; 3]] = &[
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 1.0],
            [120.0, 1.0, 1.0],
            [240.0, 1.0, 1.0],
            [60.0, 0.5, 0.8],
            [180.0, 0.5, 0.8],
            [300.0, 0.5, 0.8],
            [359.0, 1.0, 1.0],
            [30.0, 0.8, 0.3],
            [90.0, 0.6, 0.7],
        ];
        for src in cases {
            let mut c_dest = [0.0f32; 3];
            let mut r_dest = [0.0f32; 3];
            c_f12(c_dest.as_mut_ptr(), src.as_ptr());
            r_f12(r_dest.as_mut_ptr(), src.as_ptr());
            for i in 0..3 {
                assert_eq!(c_dest[i].to_bits(), r_dest[i].to_bits(),
                    "f12({:?})[{}]: C={}, Rust={}", src, i, c_dest[i], r_dest[i]);
            }
        }
    }
}

// ---- f13 tests ----
#[test]
fn test_f13() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_f13: Symbol<unsafe extern "C" fn(*mut f32, *const f32)> = c_lib.get(b"f13").unwrap();
        let r_f13: Symbol<unsafe extern "C" fn(*mut f32, *const f32)> = r_lib.get(b"f13").unwrap();

        let cases: &[[f32; 3]] = &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.5, 0.3, 0.1],
            [0.2, 0.8, 0.4],
            [0.9, 0.1, 0.7],
        ];
        for src in cases {
            let mut c_dest = [0.0f32; 3];
            let mut r_dest = [0.0f32; 3];
            c_f13(c_dest.as_mut_ptr(), src.as_ptr());
            r_f13(r_dest.as_mut_ptr(), src.as_ptr());
            for i in 0..3 {
                assert_eq!(c_dest[i].to_bits(), r_dest[i].to_bits(),
                    "f13({:?})[{}]: C={}, Rust={}", src, i, c_dest[i], r_dest[i]);
            }
        }
    }
}

// ---- f2 tests ----
#[test]
fn test_f2() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_f2: Symbol<unsafe extern "C" fn(*const u8, i32, *const u8, i32) -> i32> =
            c_lib.get(b"f2").unwrap();
        let r_f2: Symbol<unsafe extern "C" fn(*const u8, i32, *const u8, i32) -> i32> =
            r_lib.get(b"f2").unwrap();

        // Circle vs AABB
        let circle = c2Circle { p: c2v { x: 1.0, y: 1.0 }, r: 2.0 };
        let aabb = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 3.0, y: 3.0 } };

        let c_r = c_f2(&circle as *const _ as *const u8, 0, &aabb as *const _ as *const u8, 1);
        let r_r = r_f2(&circle as *const _ as *const u8, 0, &aabb as *const _ as *const u8, 1);
        assert_eq!(c_r, r_r, "f2 circle-aabb: C={}, Rust={}", c_r, r_r);

        // Circle vs Circle
        let c2 = c2Circle { p: c2v { x: 10.0, y: 10.0 }, r: 1.0 };
        let c_r = c_f2(&circle as *const _ as *const u8, 0, &c2 as *const _ as *const u8, 0);
        let r_r = r_f2(&circle as *const _ as *const u8, 0, &c2 as *const _ as *const u8, 0);
        assert_eq!(c_r, r_r, "f2 circle-circle: C={}, Rust={}", c_r, r_r);

        // AABB vs AABB
        let aabb2 = c2AABB { min: c2v { x: 5.0, y: 5.0 }, max: c2v { x: 8.0, y: 8.0 } };
        let c_r = c_f2(&aabb as *const _ as *const u8, 1, &aabb2 as *const _ as *const u8, 1);
        let r_r = r_f2(&aabb as *const _ as *const u8, 1, &aabb2 as *const _ as *const u8, 1);
        assert_eq!(c_r, r_r, "f2 aabb-aabb: C={}, Rust={}", c_r, r_r);
    }
}

// ---- agglom (top-level) test ----
#[test]
fn test_agglom() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        type AgglomFn = unsafe extern "C" fn(
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

        let c_agglom: Symbol<AgglomFn> = c_lib.get(b"agglom").unwrap();
        let r_agglom: Symbol<AgglomFn> = r_lib.get(b"agglom").unwrap();

        let test_cases: &[(f32,f32,f32,f32,f32,f32,f32, i32,i32, u64,u64, u32, u32,u32,u32, f32,f32,f32,f32,f32,f32,f32,f32, u16, f32,f32,f32, f32,f32,f32, f32,f32,f32)] = &[
            // Basic case
            (1.0,1.0,2.0, 0.0,0.0,3.0,3.0, 10,3, 12345,67890, 0x1234, 4096,2,16, 0.0,0.0,1.0,0.0,0.0,1.0,0.25,0.25, 0x3C00, 0.0,1.0,0.5, 0.0,1.0,1.0, 1.0,0.0,0.0),
            // Another case
            (5.0,5.0,1.0, 2.0,2.0,8.0,8.0, -100,7, 0,0, 0xABCD, 1024,1,24, 1.0,1.0,4.0,1.0,1.0,5.0,2.0,3.0, 0x4000, 120.0,0.5,0.5, 60.0,0.5,0.8, 0.5,0.3,0.1),
        ];

        for (i, args) in test_cases.iter().enumerate() {
            let c_r = c_agglom(
                args.0,args.1,args.2,args.3,args.4,args.5,args.6,
                args.7,args.8, args.9,args.10, args.11,
                args.12,args.13,args.14,
                args.15,args.16,args.17,args.18,args.19,args.20,args.21,args.22,
                args.23, args.24,args.25,args.26,
                args.27,args.28,args.29, args.30,args.31,args.32,
            );
            let r_r = r_agglom(
                args.0,args.1,args.2,args.3,args.4,args.5,args.6,
                args.7,args.8, args.9,args.10, args.11,
                args.12,args.13,args.14,
                args.15,args.16,args.17,args.18,args.19,args.20,args.21,args.22,
                args.23, args.24,args.25,args.26,
                args.27,args.28,args.29, args.30,args.31,args.32,
            );
            assert_eq!(c_r.to_bits(), r_r.to_bits(),
                "agglom case {}: C={}, Rust={}", i, c_r, r_r);
        }
    }
}

// ---- c2V, c2Maxv, c2Minv, c2Sub, c2Dot, c2Clampv tests ----
#[test]
fn test_c2_helpers() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        // c2V
        let c_c2V: Symbol<unsafe extern "C" fn(f32, f32) -> c2v> = c_lib.get(b"c2V").unwrap();
        let r_c2V: Symbol<unsafe extern "C" fn(f32, f32) -> c2v> = r_lib.get(b"c2V").unwrap();
        let c_r = c_c2V(1.5, 2.5);
        let r_r = r_c2V(1.5, 2.5);
        assert_eq!(c_r.x.to_bits(), r_r.x.to_bits());
        assert_eq!(c_r.y.to_bits(), r_r.y.to_bits());

        // c2Dot
        let c_dot: Symbol<unsafe extern "C" fn(c2v, c2v) -> f32> = c_lib.get(b"c2Dot").unwrap();
        let r_dot: Symbol<unsafe extern "C" fn(c2v, c2v) -> f32> = r_lib.get(b"c2Dot").unwrap();
        let a = c2v { x: 3.0, y: 4.0 };
        let b = c2v { x: 1.0, y: 2.0 };
        assert_eq!(c_dot(a, b).to_bits(), r_dot(a, b).to_bits());

        // c2Sub
        let c_sub: Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = c_lib.get(b"c2Sub").unwrap();
        let r_sub: Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = r_lib.get(b"c2Sub").unwrap();
        let c_r = c_sub(a, b);
        let r_r = r_sub(a, b);
        assert_eq!(c_r.x.to_bits(), r_r.x.to_bits());
        assert_eq!(c_r.y.to_bits(), r_r.y.to_bits());

        // c2CircletoCircle
        let c_cc: Symbol<unsafe extern "C" fn(c2Circle, c2Circle) -> i32> = c_lib.get(b"c2CircletoCircle").unwrap();
        let r_cc: Symbol<unsafe extern "C" fn(c2Circle, c2Circle) -> i32> = r_lib.get(b"c2CircletoCircle").unwrap();
        let ca = c2Circle { p: c2v{x:0.0,y:0.0}, r: 5.0 };
        let cb = c2Circle { p: c2v{x:3.0,y:4.0}, r: 2.0 };
        assert_eq!(c_cc(ca, cb), r_cc(ca, cb));

        // c2AABBtoAABB
        let c_aa: Symbol<unsafe extern "C" fn(c2AABB, c2AABB) -> i32> = c_lib.get(b"c2AABBtoAABB").unwrap();
        let r_aa: Symbol<unsafe extern "C" fn(c2AABB, c2AABB) -> i32> = r_lib.get(b"c2AABBtoAABB").unwrap();
        let a1 = c2AABB { min: c2v{x:0.0,y:0.0}, max: c2v{x:2.0,y:2.0} };
        let a2 = c2AABB { min: c2v{x:1.0,y:1.0}, max: c2v{x:3.0,y:3.0} };
        assert_eq!(c_aa(a1, a2), r_aa(a1, a2));
    }
}
