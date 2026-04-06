use libloading::{Library, Symbol};
use std::mem::MaybeUninit;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Raycast {
    t: f32,
    n: C2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Ray {
    p: C2v,
    d: C2v,
    t: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct C2m {
    x: C2v,
    y: C2v,
}

fn c_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libgen_ray_lib.so", manifest)
}

fn rust_lib_path() -> String {
    // Find the built Rust cdylib
    let manifest = env!("CARGO_MANIFEST_DIR");
    let target_dir = format!("{}/target/debug", manifest);
    format!("{}/libgen_ray_lib.so", target_dir)
}

fn assert_c2v_eq(label: &str, a: C2v, b: C2v) {
    assert!(
        a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits(),
        "{}: C={{x:{}, y:{}}} vs Rust={{x:{}, y:{}}} (bits: C={{0x{:08x}, 0x{:08x}}} Rust={{0x{:08x}, 0x{:08x}}})",
        label, a.x, a.y, b.x, b.y, a.x.to_bits(), a.y.to_bits(), b.x.to_bits(), b.y.to_bits()
    );
}

fn assert_raycast_eq(label: &str, a: &C2Raycast, b: &C2Raycast) {
    assert!(
        a.t.to_bits() == b.t.to_bits(),
        "{}: t mismatch: C={} (0x{:08x}) vs Rust={} (0x{:08x})",
        label, a.t, a.t.to_bits(), b.t, b.t.to_bits()
    );
    assert_c2v_eq(&format!("{}.n", label), a.n, b.n);
}

// ============ Low-level function tests ============

#[test]
fn test_c2v() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_c2v: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = lib.get(b"c2V").unwrap();
        let inputs = [(0.0f32, 0.0f32), (1.5, -2.3), (-100.0, 100.0), (f32::INFINITY, f32::NEG_INFINITY)];
        for (x, y) in inputs {
            let c = c_c2v(x, y);
            assert_c2v_eq("c2V", c, C2v { x, y });
        }
    }
}

#[test]
fn test_c2_dot() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = lib.get(b"c2Dot").unwrap();
        let cases = [
            (C2v { x: 1.0, y: 0.0 }, C2v { x: 0.0, y: 1.0 }),
            (C2v { x: 3.0, y: 4.0 }, C2v { x: 3.0, y: 4.0 }),
            (C2v { x: -1.5, y: 2.7 }, C2v { x: 0.3, y: -0.8 }),
        ];
        for (a, b) in cases {
            let c_res = c_fn(a, b);
            let rust_res = a.x * b.x + a.y * b.y;
            assert!(c_res.to_bits() == rust_res.to_bits(), "c2Dot mismatch: C={} Rust={}", c_res, rust_res);
        }
    }
}

#[test]
fn test_c2_add_sub_mulvs() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_add: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = lib.get(b"c2Add").unwrap();
        let c_sub: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = lib.get(b"c2Sub").unwrap();
        let c_mulvs: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = lib.get(b"c2Mulvs").unwrap();

        let a = C2v { x: 1.5, y: -2.3 };
        let b = C2v { x: 0.7, y: 3.1 };

        let c_r = c_add(a, b);
        assert_c2v_eq("c2Add", c_r, C2v { x: a.x + b.x, y: a.y + b.y });

        let c_r = c_sub(a, b);
        assert_c2v_eq("c2Sub", c_r, C2v { x: a.x - b.x, y: a.y - b.y });

        let c_r = c_mulvs(a, 2.5);
        assert_c2v_eq("c2Mulvs", c_r, C2v { x: a.x * 2.5, y: a.y * 2.5 });
    }
}

#[test]
fn test_c2_len_div_norm() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_len: Symbol<unsafe extern "C" fn(C2v) -> f32> = lib.get(b"c2Len").unwrap();
        let c_div: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = lib.get(b"c2Div").unwrap();
        let c_norm: Symbol<unsafe extern "C" fn(C2v) -> C2v> = lib.get(b"c2Norm").unwrap();

        let a = C2v { x: 3.0, y: 4.0 };
        let c_r = c_len(a);
        let rust_r = (a.x * a.x + a.y * a.y).sqrt();
        assert!(c_r.to_bits() == rust_r.to_bits(), "c2Len mismatch");

        let c_r = c_div(a, 2.0);
        let s = 1.0f32 / 2.0;
        assert_c2v_eq("c2Div", c_r, C2v { x: a.x * s, y: a.y * s });

        let c_r = c_norm(a);
        let len = (a.x * a.x + a.y * a.y).sqrt();
        let s = 1.0f32 / len;
        assert_c2v_eq("c2Norm", c_r, C2v { x: a.x * s, y: a.y * s });
    }
}

#[test]
fn test_c2_minv_maxv_skew_absv() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_minv: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = lib.get(b"c2Minv").unwrap();
        let c_maxv: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = lib.get(b"c2Maxv").unwrap();
        let c_skew: Symbol<unsafe extern "C" fn(C2v) -> C2v> = lib.get(b"c2Skew").unwrap();
        let c_absv: Symbol<unsafe extern "C" fn(C2v) -> C2v> = lib.get(b"c2Absv").unwrap();

        let a = C2v { x: 1.0, y: -3.0 };
        let b = C2v { x: -2.0, y: 5.0 };

        assert_c2v_eq("c2Minv", c_minv(a, b), C2v { x: -2.0, y: -3.0 });
        assert_c2v_eq("c2Maxv", c_maxv(a, b), C2v { x: 1.0, y: 5.0 });
        assert_c2v_eq("c2Skew", c_skew(a), C2v { x: 3.0, y: 1.0 });
        assert_c2v_eq("c2Absv", c_absv(a), C2v { x: 1.0, y: 3.0 });
    }
}

#[test]
fn test_c2_ccw90_mulmvt() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_ccw90: Symbol<unsafe extern "C" fn(C2v) -> C2v> = lib.get(b"c2CCW90").unwrap();
        let c_mulmvt: Symbol<unsafe extern "C" fn(C2m, C2v) -> C2v> = lib.get(b"c2MulmvT").unwrap();

        let a = C2v { x: 1.0, y: 2.0 };
        assert_c2v_eq("c2CCW90", c_ccw90(a), C2v { x: 2.0, y: -1.0 });

        let m = C2m {
            x: C2v { x: 1.0, y: 2.0 },
            y: C2v { x: 3.0, y: 4.0 },
        };
        let b = C2v { x: 5.0, y: 6.0 };
        let expected = C2v {
            x: 1.0 * 5.0 + 2.0 * 6.0,
            y: 3.0 * 5.0 + 4.0 * 6.0,
        };
        assert_c2v_eq("c2MulmvT", c_mulmvt(m, b), expected);
    }
}

#[test]
fn test_c2_aabb_to_aabb() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> i32> = lib.get(b"c2AABBtoAABB").unwrap();

        // Overlapping
        let a = C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 2.0, y: 2.0 } };
        let b = C2AABB { min: C2v { x: 1.0, y: 1.0 }, max: C2v { x: 3.0, y: 3.0 } };
        assert_eq!(c_fn(a, b), 1);

        // Non-overlapping
        let b2 = C2AABB { min: C2v { x: 5.0, y: 5.0 }, max: C2v { x: 6.0, y: 6.0 } };
        assert_eq!(c_fn(a, b2), 0);
    }
}

#[test]
fn test_c2_aabb_to_point_and_circle_to_point() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_aabb_pt: Symbol<unsafe extern "C" fn(C2AABB, C2v) -> i32> = lib.get(b"c2AABBtoPoint").unwrap();
        let c_circ_pt: Symbol<unsafe extern "C" fn(C2Circle, C2v) -> i32> = lib.get(b"c2CircleToPoint").unwrap();

        let aabb = C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 2.0, y: 2.0 } };
        assert_eq!(c_aabb_pt(aabb, C2v { x: 1.0, y: 1.0 }), 1);
        assert_eq!(c_aabb_pt(aabb, C2v { x: 5.0, y: 5.0 }), 0);

        let circ = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 };
        assert_eq!(c_circ_pt(circ, C2v { x: 0.5, y: 0.0 }), 1);
        assert_eq!(c_circ_pt(circ, C2v { x: 5.0, y: 0.0 }), 0);
    }
}

// ============ Ray-cast function tests ============

#[test]
fn test_c2_ray_to_circle() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> i32> = lib.get(b"c2RaytoCircle").unwrap();

        let ray = C2Ray {
            p: C2v { x: -5.0, y: 0.0 },
            d: C2v { x: 1.0, y: 0.0 },
            t: 100.0,
        };
        let circle = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 };

        let mut c_out = MaybeUninit::<C2Raycast>::zeroed().assume_init();
        let c_hit = c_fn(ray, circle, &mut c_out);
        assert_eq!(c_hit, 1, "C should hit circle");

        // Load Rust lib and compare
        let rlib = Library::new(rust_lib_path()).unwrap();
        // Rust only exports gen_ray, so we test via gen_ray below
        // For now just verify C result is sane
        assert!(c_out.t > 0.0);
    }
}

#[test]
fn test_c2_ray_to_aabb() {
    unsafe {
        let lib = Library::new(c_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2Ray, C2AABB, *mut C2Raycast) -> i32> = lib.get(b"c2RaytoAABB").unwrap();

        let ray = C2Ray {
            p: C2v { x: -5.0, y: 0.5 },
            d: C2v { x: 1.0, y: 0.0 },
            t: 100.0,
        };
        let aabb = C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 2.0, y: 2.0 } };

        let mut c_out = MaybeUninit::<C2Raycast>::zeroed().assume_init();
        let c_hit = c_fn(ray, aabb, &mut c_out);
        assert_eq!(c_hit, 1, "C should hit AABB");
    }
}

// ============ gen_ray end-to-end test ============

type GenRayFn = unsafe extern "C" fn(
    *mut C2Raycast, *mut C2Raycast, *mut C2Raycast,
    f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32,
) -> i32;

fn call_gen_ray(lib: &Library, params: &[f32; 16]) -> (i32, C2Raycast, C2Raycast, C2Raycast) {
    unsafe {
        let func: Symbol<GenRayFn> = lib.get(b"gen_ray").unwrap();
        let mut c1 = MaybeUninit::<C2Raycast>::zeroed().assume_init();
        let mut c2 = MaybeUninit::<C2Raycast>::zeroed().assume_init();
        let mut c3 = MaybeUninit::<C2Raycast>::zeroed().assume_init();
        let ret = func(
            &mut c1, &mut c2, &mut c3,
            params[0], params[1], params[2], params[3],
            params[4], params[5], params[6],
            params[7], params[8], params[9], params[10], params[11],
            params[12], params[13], params[14], params[15],
        );
        (ret, c1, c2, c3)
    }
}

#[test]
fn test_gen_ray_compare() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let r_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    // Test cases: [mp_x, mp_y, r_p_x, r_p_y, c_p_x, c_p_y, c_r,
    //              cap_a_x, cap_a_y, cap_b_x, cap_b_y, cap_r,
    //              bb_min_x, bb_min_y, bb_max_x, bb_max_y]
    let test_cases: &[[f32; 16]] = &[
        // Ray from left hitting circle at origin, capsule, and AABB
        [10.0, 0.0, -10.0, 0.0, 0.0, 0.0, 1.0, 3.0, -1.0, 3.0, 1.0, 0.5, 5.0, -1.0, 7.0, 1.0],
        // Ray going up
        [0.0, 10.0, 0.0, -10.0, 0.0, 0.0, 2.0, -1.0, 5.0, 1.0, 5.0, 1.0, -1.0, 7.0, 1.0, 9.0],
        // Ray missing everything
        [100.0, 100.0, -100.0, -100.0, 50.0, 0.0, 1.0, 60.0, 0.0, 70.0, 0.0, 0.5, 80.0, 0.0, 90.0, 1.0],
        // Diagonal ray
        [5.0, 5.0, -5.0, -5.0, 0.0, 0.0, 1.5, 2.0, 2.0, 3.0, 3.0, 0.5, 1.0, 1.0, 4.0, 4.0],
        // Ray origin inside circle
        [5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 3.0, -1.0, 3.0, 1.0, 0.5, 4.0, -1.0, 6.0, 1.0],
    ];

    for (i, params) in test_cases.iter().enumerate() {
        let (c_ret, c_c1, c_c2, c_c3) = call_gen_ray(&c_lib, params);
        let (r_ret, r_c1, r_c2, r_c3) = call_gen_ray(&r_lib, params);

        assert_eq!(c_ret, r_ret, "case {}: return value mismatch: C={} Rust={}", i, c_ret, r_ret);

        // Only compare raycast outputs for hits
        if c_ret & 1 != 0 {
            assert_raycast_eq(&format!("case {} cast1", i), &c_c1, &r_c1);
        }
        if c_ret & 2 != 0 {
            assert_raycast_eq(&format!("case {} cast2", i), &c_c2, &r_c2);
        }
        if c_ret & 4 != 0 {
            assert_raycast_eq(&format!("case {} cast3", i), &c_c3, &r_c3);
        }
    }
}
