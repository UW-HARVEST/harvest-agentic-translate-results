use libloading::{Library, Symbol};
use std::os::raw::c_char;

// Mirror C structs with repr(C)
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2GJKCache {
    metric: f32,
    count: i32,
    i_a: [i32; 3],
    i_b: [i32; 3],
    div: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2sv {
    s_a: C2v,
    s_b: C2v,
    p: C2v,
    u: f32,
    i_a: i32,
    i_b: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: f32,
    count: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Proxy {
    radius: f32,
    count: i32,
    verts: [C2v; 8],
}

fn c_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libgjk_cache_lib.so", manifest)
}

fn assert_c2v_eq(label: &str, c: C2v, r: C2v) {
    assert!(
        c.x.to_bits() == r.x.to_bits() && c.y.to_bits() == r.y.to_bits(),
        "{label}: C=({}, {}) [bits: {:08x},{:08x}] vs Rust=({}, {}) [bits: {:08x},{:08x}]",
        c.x, c.y, c.x.to_bits(), c.y.to_bits(),
        r.x, r.y, r.x.to_bits(), r.y.to_bits(),
    );
}

fn assert_f32_eq(label: &str, c: f32, r: f32) {
    assert!(
        c.to_bits() == r.to_bits(),
        "{label}: C={} [bits: {:08x}] vs Rust={} [bits: {:08x}]",
        c, c.to_bits(), r, r.to_bits(),
    );
}

// Load Rust .so dynamically too, so we compare both shared libraries
fn rust_lib_path() -> String {
    // cargo puts the cdylib in target/debug/
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/libgjk_cache_lib.so", manifest)
}

#[test]
fn test_gjk_cache_main() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type GjkCacheFn = unsafe extern "C" fn(
        c_char, *mut C2v, *mut C2v,
        f32, f32, f32, f32,
        f32, f32, f32, f32, f32,
    );

    let c_gjk_cache: Symbol<GjkCacheFn> = unsafe { c_lib.get(b"gjk_cache").unwrap() };
    let r_gjk_cache: Symbol<GjkCacheFn> = unsafe { r_lib.get(b"gjk_cache").unwrap() };

    // Test cases: (reverse, a1..a4, b1..b5)
    let cases: Vec<(i8, [f32; 4], [f32; 5])> = vec![
        (0, [0.0, 0.0, 10.0, 10.0], [20.0, 0.0, 30.0, 10.0, 5.0]),
        (1, [0.0, 0.0, 10.0, 10.0], [20.0, 0.0, 30.0, 10.0, 5.0]),
        (0, [-5.0, -5.0, 5.0, 5.0], [0.0, 0.0, 10.0, 10.0, 2.0]),
        (1, [-5.0, -5.0, 5.0, 5.0], [0.0, 0.0, 10.0, 10.0, 2.0]),
        (0, [100.0, 100.0, 200.0, 200.0], [50.0, 50.0, 60.0, 60.0, 1.0]),
        (0, [0.0, 0.0, 1.0, 1.0], [0.5, 0.5, 0.5, 0.5, 0.1]),
        (1, [0.0, 0.0, 100.0, 100.0], [-50.0, -50.0, 150.0, 150.0, 25.0]),
    ];

    for (i, (rev, a, b)) in cases.iter().enumerate() {
        let mut c_a9 = C2v { x: 0.0, y: 0.0 };
        let mut c_b9 = C2v { x: 0.0, y: 0.0 };
        let mut r_a9 = C2v { x: 0.0, y: 0.0 };
        let mut r_b9 = C2v { x: 0.0, y: 0.0 };

        unsafe {
            c_gjk_cache(
                *rev, &mut c_a9, &mut c_b9,
                a[0], a[1], a[2], a[3],
                b[0], b[1], b[2], b[3], b[4],
            );
            r_gjk_cache(
                *rev, &mut r_a9, &mut r_b9,
                a[0], a[1], a[2], a[3],
                b[0], b[1], b[2], b[3], b[4],
            );
        }

        assert_c2v_eq(&format!("case {i} a9"), c_a9, r_a9);
        assert_c2v_eq(&format!("case {i} b9"), c_b9, r_b9);
    }
}

#[test]
fn test_c2v_basic_ops() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };

    // c2V
    let c_c2v: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> =
        unsafe { lib.get(b"c2V").unwrap() };
    let c_mulvs: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> =
        unsafe { lib.get(b"c2Mulvs").unwrap() };
    let c_add: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> =
        unsafe { lib.get(b"c2Add").unwrap() };
    let c_sub: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> =
        unsafe { lib.get(b"c2Sub").unwrap() };
    let c_dot: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> =
        unsafe { lib.get(b"c2Dot").unwrap() };
    let c_neg: Symbol<unsafe extern "C" fn(C2v) -> C2v> =
        unsafe { lib.get(b"c2Neg").unwrap() };
    let c_maxv: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> =
        unsafe { lib.get(b"c2Maxv").unwrap() };
    let c_minv: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> =
        unsafe { lib.get(b"c2Minv").unwrap() };
    let c_len: Symbol<unsafe extern "C" fn(C2v) -> f32> =
        unsafe { lib.get(b"c2Len").unwrap() };
    let c_det2: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> =
        unsafe { lib.get(b"c2Det2").unwrap() };
    let c_skew: Symbol<unsafe extern "C" fn(C2v) -> C2v> =
        unsafe { lib.get(b"c2Skew").unwrap() };
    let c_ccw90: Symbol<unsafe extern "C" fn(C2v) -> C2v> =
        unsafe { lib.get(b"c2CCW90").unwrap() };
    let c_norm: Symbol<unsafe extern "C" fn(C2v) -> C2v> =
        unsafe { lib.get(b"c2Norm").unwrap() };
    let c_div: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> =
        unsafe { lib.get(b"c2Div").unwrap() };
    let c_mulrv: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> =
        unsafe { lib.get(b"c2Mulrv").unwrap() };
    let c_mulrvt: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> =
        unsafe { lib.get(b"c2MulrvT").unwrap() };
    let c_mulxv: Symbol<unsafe extern "C" fn(C2x, C2v) -> C2v> =
        unsafe { lib.get(b"c2Mulxv").unwrap() };
    let c_clampv: Symbol<unsafe extern "C" fn(C2v, C2v, C2v) -> C2v> =
        unsafe { lib.get(b"c2Clampv").unwrap() };
    let c_rot_id: Symbol<unsafe extern "C" fn() -> C2r> =
        unsafe { lib.get(b"c2RotIdentity").unwrap() };
    let c_x_id: Symbol<unsafe extern "C" fn() -> C2x> =
        unsafe { lib.get(b"c2xIdentity").unwrap() };

    // Use the Rust internal functions via re-export - we need to call them.
    // Since they're private in lib.rs, we'll replicate the logic inline.
    // Actually, let's just test the public gjk_cache and the C internal functions
    // against each other by calling C functions and comparing with inline Rust equivalents.

    let test_vecs: Vec<(f32, f32)> = vec![
        (0.0, 0.0), (1.0, 2.0), (-3.5, 4.7), (100.0, -200.0),
        (0.001, 0.002), (f32::MAX, f32::MIN),
    ];

    for &(x, y) in &test_vecs {
        let cv = unsafe { c_c2v(x, y) };
        assert_c2v_eq("c2V", cv, C2v { x, y });
    }

    // c2Mulvs
    let a = C2v { x: 3.0, y: -4.0 };
    for &s in &[0.0f32, 1.0, -1.0, 2.5, 0.001] {
        let cv = unsafe { c_mulvs(a, s) };
        let rv = C2v { x: a.x * s, y: a.y * s };
        assert_c2v_eq(&format!("c2Mulvs s={s}"), cv, rv);
    }

    // c2Add, c2Sub
    let pairs = vec![
        (C2v { x: 1.0, y: 2.0 }, C2v { x: 3.0, y: 4.0 }),
        (C2v { x: -1.0, y: 0.0 }, C2v { x: 0.0, y: -1.0 }),
    ];
    for (a, b) in &pairs {
        let ca = unsafe { c_add(*a, *b) };
        let ra = C2v { x: a.x + b.x, y: a.y + b.y };
        assert_c2v_eq("c2Add", ca, ra);

        let cs = unsafe { c_sub(*a, *b) };
        let rs = C2v { x: a.x - b.x, y: a.y - b.y };
        assert_c2v_eq("c2Sub", cs, rs);
    }

    // c2Dot
    let a = C2v { x: 3.0, y: 4.0 };
    let b = C2v { x: 1.0, y: 2.0 };
    let cd = unsafe { c_dot(a, b) };
    assert_f32_eq("c2Dot", cd, a.x * b.x + a.y * b.y);

    // c2Neg
    let cn = unsafe { c_neg(a) };
    assert_c2v_eq("c2Neg", cn, C2v { x: -a.x, y: -a.y });

    // c2Maxv, c2Minv
    let cm = unsafe { c_maxv(a, b) };
    assert_c2v_eq("c2Maxv", cm, C2v { x: 3.0, y: 4.0 });
    let cm = unsafe { c_minv(a, b) };
    assert_c2v_eq("c2Minv", cm, C2v { x: 1.0, y: 2.0 });

    // c2Len
    let cl = unsafe { c_len(C2v { x: 3.0, y: 4.0 }) };
    assert_f32_eq("c2Len", cl, 5.0);

    // c2Det2
    let cd = unsafe { c_det2(C2v { x: 1.0, y: 2.0 }, C2v { x: 3.0, y: 4.0 }) };
    assert_f32_eq("c2Det2", cd, 1.0 * 4.0 - 2.0 * 3.0);

    // c2Skew, c2CCW90
    let v = C2v { x: 3.0, y: 4.0 };
    let cs = unsafe { c_skew(v) };
    assert_c2v_eq("c2Skew", cs, C2v { x: -4.0, y: 3.0 });
    let cc = unsafe { c_ccw90(v) };
    assert_c2v_eq("c2CCW90", cc, C2v { x: 4.0, y: -3.0 });

    // c2Norm
    let cn = unsafe { c_norm(C2v { x: 3.0, y: 4.0 }) };
    let len = 5.0f32;
    assert_c2v_eq("c2Norm", cn, C2v { x: 3.0 / len, y: 4.0 / len });

    // c2Div
    let cd = unsafe { c_div(C2v { x: 6.0, y: 8.0 }, 2.0) };
    assert_c2v_eq("c2Div", cd, C2v { x: 6.0 * (1.0 / 2.0), y: 8.0 * (1.0 / 2.0) });

    // c2Mulrv
    let r = C2r { c: 0.0, s: 1.0 }; // 90 degree rotation
    let v = C2v { x: 1.0, y: 0.0 };
    let cm = unsafe { c_mulrv(r, v) };
    assert_c2v_eq("c2Mulrv", cm, C2v { x: 0.0, y: 1.0 });

    // c2MulrvT
    let cm = unsafe { c_mulrvt(r, v) };
    assert_c2v_eq("c2MulrvT", cm, C2v { x: 0.0, y: -1.0 });

    // c2Mulxv
    let x = C2x { p: C2v { x: 10.0, y: 20.0 }, r: C2r { c: 1.0, s: 0.0 } };
    let v = C2v { x: 1.0, y: 2.0 };
    let cm = unsafe { c_mulxv(x, v) };
    assert_c2v_eq("c2Mulxv", cm, C2v { x: 11.0, y: 22.0 });

    // c2Clampv
    let lo = C2v { x: 0.0, y: 0.0 };
    let hi = C2v { x: 5.0, y: 5.0 };
    let v = C2v { x: -1.0, y: 7.0 };
    let cc = unsafe { c_clampv(v, lo, hi) };
    // clampv = maxv(lo, minv(v, hi))
    let minv = C2v { x: if v.x < hi.x { v.x } else { hi.x }, y: if v.y < hi.y { v.y } else { hi.y } };
    let expected = C2v { x: if lo.x > minv.x { lo.x } else { minv.x }, y: if lo.y > minv.y { lo.y } else { minv.y } };
    assert_c2v_eq("c2Clampv", cc, expected);

    // c2RotIdentity
    let cr = unsafe { c_rot_id() };
    assert_f32_eq("c2RotIdentity.c", cr.c, 1.0);
    assert_f32_eq("c2RotIdentity.s", cr.s, 0.0);

    // c2xIdentity
    let cx = unsafe { c_x_id() };
    assert_c2v_eq("c2xIdentity.p", cx.p, C2v { x: 0.0, y: 0.0 });
    assert_f32_eq("c2xIdentity.r.c", cx.r.c, 1.0);
    assert_f32_eq("c2xIdentity.r.s", cx.r.s, 0.0);
}

#[test]
fn test_c2_support() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_support: Symbol<unsafe extern "C" fn(*const C2v, i32, C2v) -> i32> =
        unsafe { lib.get(b"c2Support").unwrap() };

    let verts = [
        C2v { x: 0.0, y: 0.0 },
        C2v { x: 10.0, y: 0.0 },
        C2v { x: 5.0, y: 10.0 },
        C2v { x: -5.0, y: 5.0 },
    ];

    let dirs = [
        C2v { x: 1.0, y: 0.0 },
        C2v { x: 0.0, y: 1.0 },
        C2v { x: -1.0, y: 0.0 },
        C2v { x: 0.0, y: -1.0 },
        C2v { x: 1.0, y: 1.0 },
    ];

    for d in &dirs {
        let ci = unsafe { c_support(verts.as_ptr(), 4, *d) };
        // Replicate Rust logic
        let mut imax = 0i32;
        let mut dmax = verts[0].x * d.x + verts[0].y * d.y;
        for i in 1..4i32 {
            let dot = verts[i as usize].x * d.x + verts[i as usize].y * d.y;
            if dot > dmax {
                imax = i;
                dmax = dot;
            }
        }
        assert_eq!(ci, imax, "c2Support dir=({},{})", d.x, d.y);
    }
}

#[test]
fn test_c2_bb_verts() {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_bb_verts: Symbol<unsafe extern "C" fn(*mut C2v, *const C2AABB)> =
        unsafe { lib.get(b"c2BBVerts").unwrap() };

    let bb = C2AABB {
        min: C2v { x: 1.0, y: 2.0 },
        max: C2v { x: 5.0, y: 6.0 },
    };
    let mut c_out = [C2v { x: 0.0, y: 0.0 }; 4];
    unsafe { c_bb_verts(c_out.as_mut_ptr(), &bb) };

    let expected = [
        bb.min,
        C2v { x: bb.max.x, y: bb.min.y },
        bb.max,
        C2v { x: bb.min.x, y: bb.max.y },
    ];
    for i in 0..4 {
        assert_c2v_eq(&format!("c2BBVerts[{i}]"), c_out[i], expected[i]);
    }
}

#[test]
fn test_c2_gjk_full() {
    // Test c2GJK through the C library vs calling gjk_cache which exercises it
    let lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };

    type C2GJKFn = unsafe extern "C" fn(
        *const u8, i32, *const C2x,
        *const u8, i32, *const C2x,
        *mut C2v, *mut C2v,
        i32, *mut i32, *mut C2GJKCache,
    ) -> f32;

    let c_gjk: Symbol<C2GJKFn> = unsafe { lib.get(b"c2GJK").unwrap() };

    // Test 1: Circle vs Capsule (same as gjk_cache internal)
    let circle = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 15.0 };
    let capsule = C2Capsule { a: C2v { x: 100.0, y: -25.0 }, b: C2v { x: 75.0, y: 100.0 }, r: 10.0 };

    let mut c_a = C2v { x: 0.0, y: 0.0 };
    let mut c_b = C2v { x: 0.0, y: 0.0 };
    let mut c_iter = 0i32;
    let mut c_cache = C2GJKCache { metric: 0.0, count: 0, i_a: [0; 3], i_b: [0; 3], div: 0.0 };

    let c_dist = unsafe {
        c_gjk(
            &circle as *const C2Circle as *const u8, 0, std::ptr::null(),
            &capsule as *const C2Capsule as *const u8, 2, std::ptr::null(),
            &mut c_a, &mut c_b, 1, &mut c_iter, &mut c_cache,
        )
    };

    // Now call Rust gjk_cache which does the same internally and compare
    // We can't directly call c2_gjk from Rust (it's private), but gjk_cache
    // exercises it. Let's just verify the C function works and compare
    // the gjk_cache outputs.

    // Test 2: AABB vs Capsule
    let bb = C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 10.0, y: 10.0 } };
    let cap = C2Capsule { a: C2v { x: 20.0, y: 0.0 }, b: C2v { x: 30.0, y: 10.0 }, r: 5.0 };

    let mut c_a2 = C2v { x: 0.0, y: 0.0 };
    let mut c_b2 = C2v { x: 0.0, y: 0.0 };
    let c_dist2 = unsafe {
        c_gjk(
            &bb as *const C2AABB as *const u8, 1, std::ptr::null(),
            &cap as *const C2Capsule as *const u8, 2, std::ptr::null(),
            &mut c_a2, &mut c_b2, 1, std::ptr::null_mut(), std::ptr::null_mut(),
        )
    };

    // Verify C returns reasonable values (non-NaN, non-negative distance)
    assert!(!c_dist.is_nan(), "c2GJK dist1 is NaN");
    assert!(!c_dist2.is_nan(), "c2GJK dist2 is NaN");
    assert!(c_dist >= 0.0, "c2GJK dist1 negative");
    assert!(c_dist2 >= 0.0, "c2GJK dist2 negative");

    // The real byte-for-byte comparison happens in test_gjk_cache_main
    // which calls both C and Rust gjk_cache with the same inputs.
    println!("c2GJK circle-capsule: dist={c_dist}, a=({},{}), b=({},{}), iter={c_iter}",
             c_a.x, c_a.y, c_b.x, c_b.y);
    println!("c2GJK aabb-capsule: dist={c_dist2}, a=({},{}), b=({},{})",
             c_a2.x, c_a2.y, c_b2.x, c_b2.y);
}
