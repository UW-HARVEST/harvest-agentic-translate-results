use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2Raycast {
    t: f32,
    n: c2v,
}

impl c2Raycast {
    fn zero() -> Self {
        Self { t: 0.0, n: c2v { x: 0.0, y: 0.0 } }
    }
    fn bytes(&self) -> [u8; 12] {
        let mut b = [0u8; 12];
        b[0..4].copy_from_slice(&self.t.to_ne_bytes());
        b[4..8].copy_from_slice(&self.n.x.to_ne_bytes());
        b[8..12].copy_from_slice(&self.n.y.to_ne_bytes());
        b
    }
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libgen_ray_lib.so");
    p
}

type GenRayFn = unsafe extern "C" fn(
    *mut c2Raycast, *mut c2Raycast, *mut c2Raycast,
    f32, f32, f32, f32,
    f32, f32, f32,
    f32, f32, f32, f32, f32,
    f32, f32, f32, f32,
) -> i32;

type C2VFn = unsafe extern "C" fn(f32, f32) -> c2v;
type C2DotFn = unsafe extern "C" fn(c2v, c2v) -> f32;
type C2LenFn = unsafe extern "C" fn(c2v) -> f32;
type C2BinVFn = unsafe extern "C" fn(c2v, c2v) -> c2v;
type C2MulvsFn = unsafe extern "C" fn(c2v, f32) -> c2v;
type C2DivFn = unsafe extern "C" fn(c2v, f32) -> c2v;
type C2UnaryVFn = unsafe extern "C" fn(c2v) -> c2v;

fn assert_v_eq(label: &str, c: c2v, r: c2v) {
    assert_eq!(c.x.to_bits(), r.x.to_bits(), "{label}: x mismatch c={} r={}", c.x, r.x);
    assert_eq!(c.y.to_bits(), r.y.to_bits(), "{label}: y mismatch c={} r={}", c.y, r.y);
}

#[test]
fn test_low_level_vector_ops() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r_lib = unsafe { Library::new(rust_lib_path()) }.unwrap();

    unsafe {
        // c2V
        let c_c2v: Symbol<C2VFn> = c_lib.get(b"c2V").unwrap();
        let r_c2v: Symbol<C2VFn> = r_lib.get(b"c2V").unwrap();
        for (x, y) in [(0.0f32, 0.0), (1.5, -2.3), (-100.0, 0.001)] {
            assert_v_eq("c2V", c_c2v(x, y), r_c2v(x, y));
        }

        // c2Dot
        let c_dot: Symbol<C2DotFn> = c_lib.get(b"c2Dot").unwrap();
        let r_dot: Symbol<C2DotFn> = r_lib.get(b"c2Dot").unwrap();
        let pairs = [
            (c2v{x:1.0,y:2.0}, c2v{x:3.0,y:4.0}),
            (c2v{x:-1.0,y:0.0}, c2v{x:0.0,y:-1.0}),
        ];
        for (a, b) in pairs {
            assert_eq!(c_dot(a, b).to_bits(), r_dot(a, b).to_bits(), "c2Dot mismatch");
        }

        // c2Len
        let c_len: Symbol<C2LenFn> = c_lib.get(b"c2Len").unwrap();
        let r_len: Symbol<C2LenFn> = r_lib.get(b"c2Len").unwrap();
        for v in [c2v{x:3.0,y:4.0}, c2v{x:0.0,y:0.0}, c2v{x:-1.0,y:1.0}] {
            assert_eq!(c_len(v).to_bits(), r_len(v).to_bits(), "c2Len mismatch");
        }

        // c2Add, c2Sub
        let c_add: Symbol<C2BinVFn> = c_lib.get(b"c2Add").unwrap();
        let r_add: Symbol<C2BinVFn> = r_lib.get(b"c2Add").unwrap();
        let c_sub: Symbol<C2BinVFn> = c_lib.get(b"c2Sub").unwrap();
        let r_sub: Symbol<C2BinVFn> = r_lib.get(b"c2Sub").unwrap();
        let a = c2v{x:1.0,y:2.0};
        let b = c2v{x:3.0,y:-1.0};
        assert_v_eq("c2Add", c_add(a, b), r_add(a, b));
        assert_v_eq("c2Sub", c_sub(a, b), r_sub(a, b));

        // c2Mulvs, c2Div
        let c_mulvs: Symbol<C2MulvsFn> = c_lib.get(b"c2Mulvs").unwrap();
        let r_mulvs: Symbol<C2MulvsFn> = r_lib.get(b"c2Mulvs").unwrap();
        let c_div: Symbol<C2DivFn> = c_lib.get(b"c2Div").unwrap();
        let r_div: Symbol<C2DivFn> = r_lib.get(b"c2Div").unwrap();
        assert_v_eq("c2Mulvs", c_mulvs(a, 2.5), r_mulvs(a, 2.5));
        assert_v_eq("c2Div", c_div(a, 2.5), r_div(a, 2.5));

        // c2Norm, c2Skew, c2Absv
        let c_norm: Symbol<C2UnaryVFn> = c_lib.get(b"c2Norm").unwrap();
        let r_norm: Symbol<C2UnaryVFn> = r_lib.get(b"c2Norm").unwrap();
        let c_skew: Symbol<C2UnaryVFn> = c_lib.get(b"c2Skew").unwrap();
        let r_skew: Symbol<C2UnaryVFn> = r_lib.get(b"c2Skew").unwrap();
        let c_absv: Symbol<C2UnaryVFn> = c_lib.get(b"c2Absv").unwrap();
        let r_absv: Symbol<C2UnaryVFn> = r_lib.get(b"c2Absv").unwrap();
        let v = c2v{x:-3.0,y:4.0};
        assert_v_eq("c2Norm", c_norm(v), r_norm(v));
        assert_v_eq("c2Skew", c_skew(v), r_skew(v));
        assert_v_eq("c2Absv", c_absv(v), r_absv(v));

        // c2Minv, c2Maxv
        let c_minv: Symbol<C2BinVFn> = c_lib.get(b"c2Minv").unwrap();
        let r_minv: Symbol<C2BinVFn> = r_lib.get(b"c2Minv").unwrap();
        let c_maxv: Symbol<C2BinVFn> = c_lib.get(b"c2Maxv").unwrap();
        let r_maxv: Symbol<C2BinVFn> = r_lib.get(b"c2Maxv").unwrap();
        assert_v_eq("c2Minv", c_minv(a, b), r_minv(a, b));
        assert_v_eq("c2Maxv", c_maxv(a, b), r_maxv(a, b));

        // c2CCW90
        let c_ccw: Symbol<C2UnaryVFn> = c_lib.get(b"c2CCW90").unwrap();
        let r_ccw: Symbol<C2UnaryVFn> = r_lib.get(b"c2CCW90").unwrap();
        assert_v_eq("c2CCW90", c_ccw(v), r_ccw(v));
    }
}

#[test]
fn test_gen_ray_basic() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r_lib = unsafe { Library::new(rust_lib_path()) }.unwrap();

    let test_cases: Vec<[f32; 16]> = vec![
        // mp_x, mp_y, r_p_x, r_p_y, c_p_x, c_p_y, c_r,
        // cap_a_x, cap_a_y, cap_b_x, cap_b_y, cap_r,
        // bb_min_x, bb_min_y, bb_max_x, bb_max_y
        [10.0, 10.0, 0.0, 0.0, 5.0, 5.0, 1.0,
         3.0, 0.0, 3.0, 6.0, 0.5,
         7.0, 7.0, 9.0, 9.0],
        // Ray misses everything
        [100.0, 0.0, 0.0, 0.0, 50.0, 50.0, 1.0,
         60.0, 60.0, 70.0, 70.0, 0.5,
         80.0, 80.0, 90.0, 90.0],
        // Ray origin inside circle
        [10.0, 10.0, 5.0, 5.0, 5.0, 5.0, 2.0,
         20.0, 20.0, 25.0, 25.0, 0.5,
         30.0, 30.0, 35.0, 35.0],
        // Horizontal ray hitting AABB
        [20.0, 5.0, 0.0, 5.0, 50.0, 50.0, 1.0,
         50.0, 50.0, 55.0, 55.0, 0.5,
         10.0, 3.0, 15.0, 7.0],
        // Close capsule hit
        [6.0, 3.0, 0.0, 3.0, 50.0, 50.0, 1.0,
         4.0, 0.0, 4.0, 6.0, 1.0,
         50.0, 50.0, 55.0, 55.0],
        // All three hits
        [10.0, 5.0, 0.0, 5.0, 3.0, 5.0, 1.0,
         5.0, 3.0, 5.0, 7.0, 1.0,
         7.0, 4.0, 9.0, 6.0],
        // Negative coordinates
        [-5.0, -5.0, 0.0, 0.0, -3.0, -3.0, 1.0,
         -4.0, -6.0, -4.0, -1.0, 0.5,
         -8.0, -8.0, -6.0, -6.0],
    ];

    unsafe {
        let c_fn: Symbol<GenRayFn> = c_lib.get(b"gen_ray").unwrap();
        let r_fn: Symbol<GenRayFn> = r_lib.get(b"gen_ray").unwrap();

        for (i, tc) in test_cases.iter().enumerate() {
            let (mut c1c, mut c2c, mut c3c) = (c2Raycast::zero(), c2Raycast::zero(), c2Raycast::zero());
            let (mut c1r, mut c2r, mut c3r) = (c2Raycast::zero(), c2Raycast::zero(), c2Raycast::zero());

            let c_ret = c_fn(&mut c1c, &mut c2c, &mut c3c,
                tc[0], tc[1], tc[2], tc[3], tc[4], tc[5], tc[6],
                tc[7], tc[8], tc[9], tc[10], tc[11],
                tc[12], tc[13], tc[14], tc[15]);
            let r_ret = r_fn(&mut c1r, &mut c2r, &mut c3r,
                tc[0], tc[1], tc[2], tc[3], tc[4], tc[5], tc[6],
                tc[7], tc[8], tc[9], tc[10], tc[11],
                tc[12], tc[13], tc[14], tc[15]);

            assert_eq!(c_ret, r_ret, "case {i}: return mismatch c={c_ret} r={r_ret}");
            if c_ret & 1 != 0 {
                assert_eq!(c1c.bytes(), c1r.bytes(), "case {i}: cast1 mismatch");
            }
            if c_ret & 2 != 0 {
                assert_eq!(c2c.bytes(), c2r.bytes(), "case {i}: cast2 mismatch");
            }
            if c_ret & 4 != 0 {
                assert_eq!(c3c.bytes(), c3r.bytes(), "case {i}: cast3 mismatch");
            }
        }
    }
}
