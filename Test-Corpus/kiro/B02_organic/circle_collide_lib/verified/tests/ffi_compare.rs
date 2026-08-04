use libloading::{Library, Symbol};
use std::os::raw::c_int;

type CircleCollideFn = unsafe extern "C" fn(f32, f32, f32) -> c_int;
type C2VFn = unsafe extern "C" fn(f32, f32) -> [f32; 2];
type C2BinVFn = unsafe extern "C" fn([f32; 2], [f32; 2]) -> [f32; 2];
type C2MulvsFn = unsafe extern "C" fn([f32; 2], f32) -> [f32; 2];
type C2ClampvFn = unsafe extern "C" fn([f32; 2], [f32; 2], [f32; 2]) -> [f32; 2];
type C2DotFn = unsafe extern "C" fn([f32; 2], [f32; 2]) -> f32;

// c2Circle: {c2v p, float r} = {float x, float y, float r}
type C2CircletoCircleFn = unsafe extern "C" fn([f32; 3], [f32; 3]) -> c_int;
// c2AABB: {c2v min, c2v max} = {float, float, float, float}
type C2CircletoAABBFn = unsafe extern "C" fn([f32; 3], [f32; 4]) -> c_int;
// c2Capsule: {c2v a, c2v b, float r} = {float, float, float, float, float}
type C2CircletoCapsuleFn = unsafe extern "C" fn([f32; 3], [f32; 5]) -> c_int;
// c2Collided: (const void*, const void*, int) -> int
type C2CollidedFn = unsafe extern "C" fn(*const u8, *const u8, c_int) -> c_int;

fn load_libs() -> (Library, Library) {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let c_path = format!("{}/c_src/libtranslated_rust.so", manifest);
    let rust_path = format!("{}/target/debug/libcircle_collide_lib.so", manifest);
    unsafe {
        let c_lib = Library::new(&c_path).expect("Failed to load C .so");
        let rust_lib = Library::new(&rust_path).expect("Failed to load Rust .so");
        (c_lib, rust_lib)
    }
}

#[test]
fn test_circle_collide_matches() {
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<CircleCollideFn> =
        unsafe { c_lib.get(b"circle_collide").unwrap() };
    let rust_fn: Symbol<CircleCollideFn> =
        unsafe { rust_lib.get(b"circle_collide").unwrap() };

    let cases: &[(f32, f32, f32)] = &[
        (100.0, 100.0, 1.0), (0.0, 0.0, 1.0),
        (-70.0, 0.0, 1.0), (-50.0, 0.0, 5.0), (-55.0, 0.0, 10.0),
        (-27.0, -27.0, 1.0), (-30.0, -30.0, 5.0), (-15.0, -15.0, 1.0), (-40.0, -40.0, 0.5),
        (-30.0, 70.0, 5.0), (-40.0, 40.0, 1.0), (-20.0, 100.0, 1.0), (-35.0, 50.0, 15.0),
        (-40.0, 0.0, 50.0), (0.0, 0.0, 100.0), (-30.0, -20.0, 30.0),
        (0.0, 0.0, 0.0), (-70.0, 0.0, 0.0),
        (f32::MAX, f32::MAX, 1.0), (f32::MIN, f32::MIN, 1.0),
        (f32::NAN, 0.0, 1.0), (0.0, f32::NAN, 1.0), (0.0, 0.0, f32::NAN),
        (f32::INFINITY, 0.0, 1.0), (f32::NEG_INFINITY, 0.0, 1.0),
        (-20.0, 0.0, 29.9), (-20.0, 0.0, 30.1),
        (-30.0, 30.0, 5.0), (-30.0, 50.0, 5.0), (-30.0, 70.0, 5.0),
        (-30.0, 90.0, 5.0), (-30.0, 110.0, 5.0),
    ];

    for &(x, y, r) in cases {
        let c_r = unsafe { c_fn(x, y, r) };
        let rust_r = unsafe { rust_fn(x, y, r) };
        assert_eq!(c_r, rust_r, "circle_collide({x}, {y}, {r}): C={c_r}, Rust={rust_r}");
    }
}

#[test]
fn test_c2v() {
    let (c_lib, rust_lib) = load_libs();
    let c_fn: Symbol<C2VFn> = unsafe { c_lib.get(b"c2V").unwrap() };
    let rust_fn: Symbol<C2VFn> = unsafe { rust_lib.get(b"c2V").unwrap() };

    for &(x, y) in &[(0.0f32, 0.0f32), (1.0, -1.0), (f32::NAN, 3.14), (f32::MAX, f32::MIN)] {
        let c_r = unsafe { c_fn(x, y) };
        let rust_r = unsafe { rust_fn(x, y) };
        assert_eq!(c_r[0].to_bits(), rust_r[0].to_bits(), "c2V x mismatch for ({x},{y})");
        assert_eq!(c_r[1].to_bits(), rust_r[1].to_bits(), "c2V y mismatch for ({x},{y})");
    }
}

fn assert_v_eq(c: [f32; 2], r: [f32; 2], label: &str) {
    assert_eq!(c[0].to_bits(), r[0].to_bits(), "{label} x: C={}, Rust={}", c[0], r[0]);
    assert_eq!(c[1].to_bits(), r[1].to_bits(), "{label} y: C={}, Rust={}", c[1], r[1]);
}

#[test]
fn test_vector_ops() {
    let (c_lib, rust_lib) = load_libs();

    let vecs: &[[f32; 2]] = &[
        [0.0, 0.0], [1.0, 2.0], [-3.0, 4.0], [f32::NAN, 1.0], [f32::MAX, f32::MIN],
    ];

    // c2Sub
    let c_sub: Symbol<C2BinVFn> = unsafe { c_lib.get(b"c2Sub").unwrap() };
    let r_sub: Symbol<C2BinVFn> = unsafe { rust_lib.get(b"c2Sub").unwrap() };
    for a in vecs { for b in vecs {
        assert_v_eq(unsafe { c_sub(*a, *b) }, unsafe { r_sub(*a, *b) }, &format!("c2Sub({a:?},{b:?})"));
    }}

    // c2Maxv
    let c_max: Symbol<C2BinVFn> = unsafe { c_lib.get(b"c2Maxv").unwrap() };
    let r_max: Symbol<C2BinVFn> = unsafe { rust_lib.get(b"c2Maxv").unwrap() };
    for a in vecs { for b in vecs {
        assert_v_eq(unsafe { c_max(*a, *b) }, unsafe { r_max(*a, *b) }, &format!("c2Maxv({a:?},{b:?})"));
    }}

    // c2Minv
    let c_min: Symbol<C2BinVFn> = unsafe { c_lib.get(b"c2Minv").unwrap() };
    let r_min: Symbol<C2BinVFn> = unsafe { rust_lib.get(b"c2Minv").unwrap() };
    for a in vecs { for b in vecs {
        assert_v_eq(unsafe { c_min(*a, *b) }, unsafe { r_min(*a, *b) }, &format!("c2Minv({a:?},{b:?})"));
    }}

    // c2Mulvs
    let c_mul: Symbol<C2MulvsFn> = unsafe { c_lib.get(b"c2Mulvs").unwrap() };
    let r_mul: Symbol<C2MulvsFn> = unsafe { rust_lib.get(b"c2Mulvs").unwrap() };
    for a in vecs { for &s in &[0.0f32, 1.0, -2.5, f32::NAN] {
        assert_v_eq(unsafe { c_mul(*a, s) }, unsafe { r_mul(*a, s) }, &format!("c2Mulvs({a:?},{s})"));
    }}

    // c2Dot
    let c_dot: Symbol<C2DotFn> = unsafe { c_lib.get(b"c2Dot").unwrap() };
    let r_dot: Symbol<C2DotFn> = unsafe { rust_lib.get(b"c2Dot").unwrap() };
    for a in vecs { for b in vecs {
        let c = unsafe { c_dot(*a, *b) };
        let r = unsafe { r_dot(*a, *b) };
        assert_eq!(c.to_bits(), r.to_bits(), "c2Dot({a:?},{b:?}): C={c}, Rust={r}");
    }}

    // c2Clampv
    let c_clamp: Symbol<C2ClampvFn> = unsafe { c_lib.get(b"c2Clampv").unwrap() };
    let r_clamp: Symbol<C2ClampvFn> = unsafe { rust_lib.get(b"c2Clampv").unwrap() };
    let lo = [-1.0f32, -1.0];
    let hi = [1.0f32, 1.0];
    for a in vecs {
        assert_v_eq(unsafe { c_clamp(*a, lo, hi) }, unsafe { r_clamp(*a, lo, hi) }, &format!("c2Clampv({a:?})"));
    }
}

#[test]
fn test_collision_functions() {
    let (c_lib, rust_lib) = load_libs();

    // c2CircletoCircle
    let c_cc: Symbol<C2CircletoCircleFn> = unsafe { c_lib.get(b"c2CircletoCircle").unwrap() };
    let r_cc: Symbol<C2CircletoCircleFn> = unsafe { rust_lib.get(b"c2CircletoCircle").unwrap() };

    let circles: &[[f32; 3]] = &[
        [0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [10.0, 10.0, 0.5], [-70.0, 0.0, 20.0],
    ];
    for a in circles { for b in circles {
        let c = unsafe { c_cc(*a, *b) };
        let r = unsafe { r_cc(*a, *b) };
        assert_eq!(c, r, "c2CircletoCircle({a:?},{b:?}): C={c}, Rust={r}");
    }}

    // c2CircletoAABB
    let c_ca: Symbol<C2CircletoAABBFn> = unsafe { c_lib.get(b"c2CircletoAABB").unwrap() };
    let r_ca: Symbol<C2CircletoAABBFn> = unsafe { rust_lib.get(b"c2CircletoAABB").unwrap() };

    let aabbs: &[[f32; 4]] = &[
        [-1.0, -1.0, 1.0, 1.0], [-40.0, -40.0, -15.0, -15.0], [0.0, 0.0, 10.0, 10.0],
    ];
    for circ in circles { for aabb in aabbs {
        let c = unsafe { c_ca(*circ, *aabb) };
        let r = unsafe { r_ca(*circ, *aabb) };
        assert_eq!(c, r, "c2CircletoAABB({circ:?},{aabb:?}): C={c}, Rust={r}");
    }}

    // c2CircletoCapsule
    let c_ccap: Symbol<C2CircletoCapsuleFn> = unsafe { c_lib.get(b"c2CircletoCapsule").unwrap() };
    let r_ccap: Symbol<C2CircletoCapsuleFn> = unsafe { rust_lib.get(b"c2CircletoCapsule").unwrap() };

    let capsules: &[[f32; 5]] = &[
        [-40.0, 40.0, -20.0, 100.0, 10.0], [0.0, 0.0, 10.0, 0.0, 5.0],
    ];
    for circ in circles { for cap in capsules {
        let c = unsafe { c_ccap(*circ, *cap) };
        let r = unsafe { r_ccap(*circ, *cap) };
        assert_eq!(c, r, "c2CircletoCapsule({circ:?},{cap:?}): C={c}, Rust={r}");
    }}

    // c2Collided
    let c_coll: Symbol<C2CollidedFn> = unsafe { c_lib.get(b"c2Collided").unwrap() };
    let r_coll: Symbol<C2CollidedFn> = unsafe { rust_lib.get(b"c2Collided").unwrap() };

    let circ_a: [f32; 3] = [0.0, 0.0, 50.0];
    let circ_b: [f32; 3] = [-70.0, 0.0, 20.0];
    let aabb_b: [f32; 4] = [-40.0, -40.0, -15.0, -15.0];
    let cap_b: [f32; 5] = [-40.0, 40.0, -20.0, 100.0, 10.0];

    for &(b_ptr, b_type) in &[
        (circ_b.as_ptr() as *const u8, 0),
        (aabb_b.as_ptr() as *const u8, 1),
        (cap_b.as_ptr() as *const u8, 2),
    ] {
        let c = unsafe { c_coll(circ_a.as_ptr() as *const u8, b_ptr, b_type) };
        let r = unsafe { r_coll(circ_a.as_ptr() as *const u8, b_ptr, b_type) };
        assert_eq!(c, r, "c2Collided type={b_type}: C={c}, Rust={r}");
    }
}
