use libloading::{Library, Symbol};
use std::ptr;

#[repr(C)]
#[derive(Clone)]
struct CPlane {
    normal: [f32; 3],
    dist: f32,
    type_: u8,
    signbits: u8,
    pad: [u8; 2],
}

struct Libs {
    c: Library,
    rs: Library,
}

fn load_libs() -> Libs {
    let dir = env!("CARGO_MANIFEST_DIR");
    let c_path = format!("{}/c_src/build/libq_math.so", dir);
    let rs_path = format!("{}/target/debug/libdriver.so", dir);
    unsafe {
        Libs {
            c: Library::new(&c_path).expect("Failed to load C library"),
            rs: Library::new(&rs_path).expect("Failed to load Rust library"),
        }
    }
}

fn assert_f32_bits_eq(a: f32, b: f32, ctx: &str) {
    assert_eq!(a.to_bits(), b.to_bits(), "{}: C={} Rust={}", ctx, a, b);
}

fn assert_f32_arr_eq(a: &[f32], b: &[f32], ctx: &str) {
    assert_eq!(a.len(), b.len(), "{}: length mismatch", ctx);
    for i in 0..a.len() {
        assert_f32_bits_eq(a[i], b[i], &format!("{}[{}]", ctx, i));
    }
}

// ==================== Scalar functions ====================

#[test]
fn test_q_rand() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut i32) -> i32> = libs.c.get(b"Q_rand\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut i32) -> i32> = libs.rs.get(b"Q_rand\0").unwrap();
        for &start_seed in &[0i32, 1, -1, 42, 1000000, i32::MAX, i32::MIN] {
            let (mut cs, mut rs) = (start_seed, start_seed);
            for _ in 0..10 {
                let cr = c_fn(&mut cs);
                let rr = rs_fn(&mut rs);
                assert_eq!(cr, rr, "Q_rand mismatch for seed starting at {}", start_seed);
                assert_eq!(cs, rs, "Q_rand seed state mismatch");
            }
        }
    }
}

#[test]
fn test_q_random() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut i32) -> f32> = libs.c.get(b"Q_random\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut i32) -> f32> = libs.rs.get(b"Q_random\0").unwrap();
        for &start_seed in &[0i32, 1, 42, 999, i32::MAX] {
            let (mut cs, mut rs) = (start_seed, start_seed);
            for _ in 0..10 {
                let cr = c_fn(&mut cs);
                let rr = rs_fn(&mut rs);
                assert_f32_bits_eq(cr, rr, &format!("Q_random(seed={})", start_seed));
            }
        }
    }
}

#[test]
fn test_q_crandom() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut i32) -> f32> = libs.c.get(b"Q_crandom\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut i32) -> f32> = libs.rs.get(b"Q_crandom\0").unwrap();
        for &start_seed in &[0i32, 1, 42, 999, i32::MAX] {
            let (mut cs, mut rs) = (start_seed, start_seed);
            for _ in 0..10 {
                let cr = c_fn(&mut cs);
                let rr = rs_fn(&mut rs);
                assert_f32_bits_eq(cr, rr, &format!("Q_crandom(seed={})", start_seed));
            }
        }
    }
}

#[test]
fn test_clamp_char() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(i32) -> i8> = libs.c.get(b"ClampChar\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(i32) -> i8> = libs.rs.get(b"ClampChar\0").unwrap();
        for &v in &[0, 1, -1, 127, -128, 200, -200, i32::MAX, i32::MIN] {
            assert_eq!(c_fn(v), rs_fn(v), "ClampChar({})", v);
        }
    }
}

#[test]
fn test_clamp_short() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(i32) -> i16> = libs.c.get(b"ClampShort\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(i32) -> i16> = libs.rs.get(b"ClampShort\0").unwrap();
        for &v in &[0, 1, -1, 32767, -32768, 40000, -40000, i32::MAX, i32::MIN] {
            assert_eq!(c_fn(v), rs_fn(v), "ClampShort({})", v);
        }
    }
}

#[test]
fn test_q_rsqrt() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = libs.c.get(b"Q_rsqrt\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = libs.rs.get(b"Q_rsqrt\0").unwrap();
        for &v in &[1.0f32, 4.0, 0.25, 100.0, 0.01, 999999.0] {
            assert_f32_bits_eq(c_fn(v), rs_fn(v), &format!("Q_rsqrt({})", v));
        }
    }
}

#[test]
fn test_q_fabs() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = libs.c.get(b"Q_fabs\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = libs.rs.get(b"Q_fabs\0").unwrap();
        for &v in &[0.0f32, 1.0, -1.0, 3.14, -3.14, f32::MAX, f32::MIN] {
            assert_f32_bits_eq(c_fn(v), rs_fn(v), &format!("Q_fabs({})", v));
        }
    }
}

#[test]
fn test_q_log2() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(i32) -> i32> = libs.c.get(b"Q_log2\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(i32) -> i32> = libs.rs.get(b"Q_log2\0").unwrap();
        for &v in &[1, 2, 3, 4, 7, 8, 15, 16, 255, 256, 1024, 65536] {
            assert_eq!(c_fn(v), rs_fn(v), "Q_log2({})", v);
        }
    }
}

// ==================== Angle functions ====================

#[test]
fn test_lerp_angle() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32, f32) -> f32> = libs.c.get(b"LerpAngle\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(f32, f32, f32) -> f32> = libs.rs.get(b"LerpAngle\0").unwrap();
        let cases = [(0.0, 90.0, 0.5), (350.0, 10.0, 0.5), (0.0, 0.0, 0.0),
                      (0.0, 360.0, 1.0), (90.0, 270.0, 0.5), (-10.0, 10.0, 0.5)];
        for &(from, to, frac) in &cases {
            assert_f32_bits_eq(c_fn(from, to, frac), rs_fn(from, to, frac),
                &format!("LerpAngle({},{},{})", from, to, frac));
        }
    }
}

#[test]
fn test_angle_subtract() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32) -> f32> = libs.c.get(b"AngleSubtract\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(f32, f32) -> f32> = libs.rs.get(b"AngleSubtract\0").unwrap();
        let cases = [(0.0, 0.0), (90.0, 45.0), (350.0, 10.0), (10.0, 350.0),
                      (0.0, 180.0), (180.0, 0.0), (-90.0, 90.0)];
        for &(a, b) in &cases {
            assert_f32_bits_eq(c_fn(a, b), rs_fn(a, b),
                &format!("AngleSubtract({},{})", a, b));
        }
    }
}

#[test]
fn test_angles_subtract() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut f32, *mut f32, *mut f32)> = libs.c.get(b"AnglesSubtract\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut f32, *mut f32, *mut f32)> = libs.rs.get(b"AnglesSubtract\0").unwrap();
        let cases: [([f32;3],[f32;3]); 3] = [
            ([0.0, 0.0, 0.0], [90.0, 45.0, 10.0]),
            ([350.0, 10.0, 180.0], [10.0, 350.0, 0.0]),
            ([-90.0, 270.0, 45.0], [90.0, -270.0, -45.0]),
        ];
        for (v1, v2) in &cases {
            let (mut cv1, mut cv2, mut co) = (*v1, *v2, [0.0f32; 3]);
            let (mut rv1, mut rv2, mut ro) = (*v1, *v2, [0.0f32; 3]);
            c_fn(cv1.as_mut_ptr(), cv2.as_mut_ptr(), co.as_mut_ptr());
            rs_fn(rv1.as_mut_ptr(), rv2.as_mut_ptr(), ro.as_mut_ptr());
            assert_f32_arr_eq(&co, &ro, "AnglesSubtract");
        }
    }
}

#[test]
fn test_angle_mod() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = libs.c.get(b"AngleMod\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = libs.rs.get(b"AngleMod\0").unwrap();
        for &v in &[0.0f32, 90.0, 360.0, 720.0, -90.0, -360.0, 45.5, 999.0] {
            assert_f32_bits_eq(c_fn(v), rs_fn(v), &format!("AngleMod({})", v));
        }
    }
}

#[test]
fn test_angle_normalize_360() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = libs.c.get(b"AngleNormalize360\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = libs.rs.get(b"AngleNormalize360\0").unwrap();
        for &v in &[0.0f32, 90.0, 360.0, 720.0, -90.0, -360.0, 45.5, 999.0] {
            assert_f32_bits_eq(c_fn(v), rs_fn(v), &format!("AngleNormalize360({})", v));
        }
    }
}

#[test]
fn test_angle_normalize_180() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = libs.c.get(b"AngleNormalize180\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = libs.rs.get(b"AngleNormalize180\0").unwrap();
        for &v in &[0.0f32, 90.0, 180.0, 270.0, 360.0, -90.0, -180.0, 720.0] {
            assert_f32_bits_eq(c_fn(v), rs_fn(v), &format!("AngleNormalize180({})", v));
        }
    }
}

#[test]
fn test_angle_delta() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32) -> f32> = libs.c.get(b"AngleDelta\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(f32, f32) -> f32> = libs.rs.get(b"AngleDelta\0").unwrap();
        let cases = [(0.0, 0.0), (90.0, 45.0), (350.0, 10.0), (10.0, 350.0), (180.0, 0.0)];
        for &(a, b) in &cases {
            assert_f32_bits_eq(c_fn(a, b), rs_fn(a, b), &format!("AngleDelta({},{})", a, b));
        }
    }
}

// ==================== Vector functions ====================

#[test]
fn test_dir_to_byte() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32) -> i32> = libs.c.get(b"DirToByte\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32) -> i32> = libs.rs.get(b"DirToByte\0").unwrap();
        let dirs: [[f32;3]; 5] = [
            [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0],
            [-0.5, 0.5, 0.707], [0.33, -0.33, 0.88],
        ];
        for d in &dirs {
            assert_eq!(c_fn(d.as_ptr()), rs_fn(d.as_ptr()), "DirToByte({:?})", d);
        }
    }
}

#[test]
fn test_byte_to_dir() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(i32, *mut f32)> = libs.c.get(b"ByteToDir\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(i32, *mut f32)> = libs.rs.get(b"ByteToDir\0").unwrap();
        for b in [0, 1, 80, 161, 200] {
            let (mut co, mut ro) = ([0.0f32; 3], [0.0f32; 3]);
            c_fn(b, co.as_mut_ptr());
            rs_fn(b, ro.as_mut_ptr());
            assert_f32_arr_eq(&co, &ro, &format!("ByteToDir({})", b));
        }
    }
}

#[test]
fn test_color_bytes3() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32, f32) -> u32> = libs.c.get(b"ColorBytes3\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(f32, f32, f32) -> u32> = libs.rs.get(b"ColorBytes3\0").unwrap();
        let cases = [(1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.5, 0.5, 0.5), (0.0, 0.0, 0.0)];
        for &(r, g, b) in &cases {
            assert_eq!(c_fn(r, g, b), rs_fn(r, g, b), "ColorBytes3({},{},{})", r, g, b);
        }
    }
}

#[test]
fn test_color_bytes4() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32, f32, f32) -> u32> = libs.c.get(b"ColorBytes4\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(f32, f32, f32, f32) -> u32> = libs.rs.get(b"ColorBytes4\0").unwrap();
        let cases = [(1.0, 0.0, 0.0, 1.0), (0.5, 0.5, 0.5, 0.5), (0.0, 0.0, 0.0, 0.0)];
        for &(r, g, b, a) in &cases {
            assert_eq!(c_fn(r, g, b, a), rs_fn(r, g, b, a), "ColorBytes4({},{},{},{})", r, g, b, a);
        }
    }
}

#[test]
fn test_normalize_color() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, *mut f32) -> f32> = libs.c.get(b"NormalizeColor\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, *mut f32) -> f32> = libs.rs.get(b"NormalizeColor\0").unwrap();
        let inputs: [[f32;4]; 3] = [
            [1.0, 2.0, 3.0, 1.0], [0.0, 0.0, 0.0, 1.0], [0.5, 0.5, 0.5, 0.5],
        ];
        for inp in &inputs {
            let (mut co, mut ro) = ([0.0f32; 4], [0.0f32; 4]);
            let cr = c_fn(inp.as_ptr(), co.as_mut_ptr());
            let rr = rs_fn(inp.as_ptr(), ro.as_mut_ptr());
            assert_f32_bits_eq(cr, rr, "NormalizeColor return");
            assert_f32_arr_eq(&co, &ro, "NormalizeColor out");
        }
    }
}

#[test]
fn test_vector_normalize() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut f32) -> f32> = libs.c.get(b"VectorNormalize\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut f32) -> f32> = libs.rs.get(b"VectorNormalize\0").unwrap();
        let inputs: [[f32;3]; 4] = [
            [3.0, 4.0, 0.0], [0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [-5.0, 0.0, 5.0],
        ];
        for inp in &inputs {
            let (mut cv, mut rv) = (*inp, *inp);
            let cr = c_fn(cv.as_mut_ptr());
            let rr = rs_fn(rv.as_mut_ptr());
            assert_f32_bits_eq(cr, rr, "VectorNormalize return");
            assert_f32_arr_eq(&cv, &rv, "VectorNormalize vec");
        }
    }
}

#[test]
fn test_vector_normalize2() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, *mut f32) -> f32> = libs.c.get(b"VectorNormalize2\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, *mut f32) -> f32> = libs.rs.get(b"VectorNormalize2\0").unwrap();
        let inputs: [[f32;3]; 3] = [[3.0, 4.0, 0.0], [0.0, 0.0, 0.0], [1.0, -2.0, 3.0]];
        for inp in &inputs {
            let (mut co, mut ro) = ([0.0f32; 3], [0.0f32; 3]);
            let cr = c_fn(inp.as_ptr(), co.as_mut_ptr());
            let rr = rs_fn(inp.as_ptr(), ro.as_mut_ptr());
            assert_f32_bits_eq(cr, rr, "VectorNormalize2 return");
            assert_f32_arr_eq(&co, &ro, "VectorNormalize2 out");
        }
    }
}

#[test]
fn test_dot_product() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, *const f32) -> f32> = libs.c.get(b"_DotProduct\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, *const f32) -> f32> = libs.rs.get(b"_DotProduct\0").unwrap();
        let cases: [([f32;3],[f32;3]); 3] = [
            ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            ([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]),
            ([-1.0, -2.0, -3.0], [1.0, 2.0, 3.0]),
        ];
        for (a, b) in &cases {
            assert_f32_bits_eq(c_fn(a.as_ptr(), b.as_ptr()), rs_fn(a.as_ptr(), b.as_ptr()), "_DotProduct");
        }
    }
}

#[test]
fn test_vector_subtract() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, *const f32, *mut f32)> = libs.c.get(b"_VectorSubtract\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, *const f32, *mut f32)> = libs.rs.get(b"_VectorSubtract\0").unwrap();
        let a = [1.0f32, 2.0, 3.0];
        let b = [4.0f32, 5.0, 6.0];
        let (mut co, mut ro) = ([0.0f32; 3], [0.0f32; 3]);
        c_fn(a.as_ptr(), b.as_ptr(), co.as_mut_ptr());
        rs_fn(a.as_ptr(), b.as_ptr(), ro.as_mut_ptr());
        assert_f32_arr_eq(&co, &ro, "_VectorSubtract");
    }
}

#[test]
fn test_vector_add() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, *const f32, *mut f32)> = libs.c.get(b"_VectorAdd\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, *const f32, *mut f32)> = libs.rs.get(b"_VectorAdd\0").unwrap();
        let a = [1.0f32, -2.0, 3.0];
        let b = [4.0f32, 5.0, -6.0];
        let (mut co, mut ro) = ([0.0f32; 3], [0.0f32; 3]);
        c_fn(a.as_ptr(), b.as_ptr(), co.as_mut_ptr());
        rs_fn(a.as_ptr(), b.as_ptr(), ro.as_mut_ptr());
        assert_f32_arr_eq(&co, &ro, "_VectorAdd");
    }
}

#[test]
fn test_vector_copy() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, *mut f32)> = libs.c.get(b"_VectorCopy\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, *mut f32)> = libs.rs.get(b"_VectorCopy\0").unwrap();
        let inp = [1.5f32, -2.5, 3.5];
        let (mut co, mut ro) = ([0.0f32; 3], [0.0f32; 3]);
        c_fn(inp.as_ptr(), co.as_mut_ptr());
        rs_fn(inp.as_ptr(), ro.as_mut_ptr());
        assert_f32_arr_eq(&co, &ro, "_VectorCopy");
    }
}

#[test]
fn test_vector_scale() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, f32, *mut f32)> = libs.c.get(b"_VectorScale\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, f32, *mut f32)> = libs.rs.get(b"_VectorScale\0").unwrap();
        let inp = [1.0f32, 2.0, 3.0];
        for &s in &[0.0f32, 1.0, -1.0, 2.5, 100.0] {
            let (mut co, mut ro) = ([0.0f32; 3], [0.0f32; 3]);
            c_fn(inp.as_ptr(), s, co.as_mut_ptr());
            rs_fn(inp.as_ptr(), s, ro.as_mut_ptr());
            assert_f32_arr_eq(&co, &ro, &format!("_VectorScale({})", s));
        }
    }
}

#[test]
fn test_vector_ma() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, f32, *const f32, *mut f32)> = libs.c.get(b"_VectorMA\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, f32, *const f32, *mut f32)> = libs.rs.get(b"_VectorMA\0").unwrap();
        let a = [1.0f32, 2.0, 3.0];
        let b = [4.0f32, 5.0, 6.0];
        for &s in &[0.0f32, 1.0, -2.0, 0.5] {
            let (mut co, mut ro) = ([0.0f32; 3], [0.0f32; 3]);
            c_fn(a.as_ptr(), s, b.as_ptr(), co.as_mut_ptr());
            rs_fn(a.as_ptr(), s, b.as_ptr(), ro.as_mut_ptr());
            assert_f32_arr_eq(&co, &ro, &format!("_VectorMA({})", s));
        }
    }
}

#[test]
fn test_vector4_scale() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, f32, *mut f32)> = libs.c.get(b"Vector4Scale\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, f32, *mut f32)> = libs.rs.get(b"Vector4Scale\0").unwrap();
        let inp = [1.0f32, 2.0, 3.0, 4.0];
        for &s in &[0.0f32, 1.0, -1.0, 0.5] {
            let (mut co, mut ro) = ([0.0f32; 4], [0.0f32; 4]);
            c_fn(inp.as_ptr(), s, co.as_mut_ptr());
            rs_fn(inp.as_ptr(), s, ro.as_mut_ptr());
            assert_f32_arr_eq(&co, &ro, &format!("Vector4Scale({})", s));
        }
    }
}

#[test]
fn test_vector_rotate() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, *const [f32; 3], *mut f32)> = libs.c.get(b"VectorRotate\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, *const [f32; 3], *mut f32)> = libs.rs.get(b"VectorRotate\0").unwrap();
        let inp = [1.0f32, 0.0, 0.0];
        let matrix: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let (mut co, mut ro) = ([0.0f32; 3], [0.0f32; 3]);
        c_fn(inp.as_ptr(), matrix.as_ptr(), co.as_mut_ptr());
        rs_fn(inp.as_ptr(), matrix.as_ptr(), ro.as_mut_ptr());
        assert_f32_arr_eq(&co, &ro, "VectorRotate identity");

        let matrix2: [[f32; 3]; 3] = [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let inp2 = [1.0f32, 2.0, 3.0];
        let (mut co2, mut ro2) = ([0.0f32; 3], [0.0f32; 3]);
        c_fn(inp2.as_ptr(), matrix2.as_ptr(), co2.as_mut_ptr());
        rs_fn(inp2.as_ptr(), matrix2.as_ptr(), ro2.as_mut_ptr());
        assert_f32_arr_eq(&co2, &ro2, "VectorRotate rotation");
    }
}

// ==================== Bounds functions ====================

#[test]
fn test_radius_from_bounds() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, *const f32) -> f32> = libs.c.get(b"RadiusFromBounds\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, *const f32) -> f32> = libs.rs.get(b"RadiusFromBounds\0").unwrap();
        let cases: [([f32;3],[f32;3]); 3] = [
            ([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]),
            ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
            ([-10.0, -5.0, -3.0], [10.0, 5.0, 3.0]),
        ];
        for (mins, maxs) in &cases {
            assert_f32_bits_eq(
                c_fn(mins.as_ptr(), maxs.as_ptr()),
                rs_fn(mins.as_ptr(), maxs.as_ptr()),
                "RadiusFromBounds",
            );
        }
    }
}

#[test]
fn test_clear_bounds() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut f32, *mut f32)> = libs.c.get(b"ClearBounds\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut f32, *mut f32)> = libs.rs.get(b"ClearBounds\0").unwrap();
        let (mut cmins, mut cmaxs) = ([0.0f32; 3], [0.0f32; 3]);
        let (mut rmins, mut rmaxs) = ([0.0f32; 3], [0.0f32; 3]);
        c_fn(cmins.as_mut_ptr(), cmaxs.as_mut_ptr());
        rs_fn(rmins.as_mut_ptr(), rmaxs.as_mut_ptr());
        assert_f32_arr_eq(&cmins, &rmins, "ClearBounds mins");
        assert_f32_arr_eq(&cmaxs, &rmaxs, "ClearBounds maxs");
    }
}

#[test]
fn test_add_point_to_bounds() {
    let libs = load_libs();
    unsafe {
        let c_clear: Symbol<unsafe extern "C" fn(*mut f32, *mut f32)> = libs.c.get(b"ClearBounds\0").unwrap();
        let rs_clear: Symbol<unsafe extern "C" fn(*mut f32, *mut f32)> = libs.rs.get(b"ClearBounds\0").unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, *mut f32, *mut f32)> = libs.c.get(b"AddPointToBounds\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, *mut f32, *mut f32)> = libs.rs.get(b"AddPointToBounds\0").unwrap();

        let (mut cmins, mut cmaxs) = ([0.0f32; 3], [0.0f32; 3]);
        let (mut rmins, mut rmaxs) = ([0.0f32; 3], [0.0f32; 3]);
        c_clear(cmins.as_mut_ptr(), cmaxs.as_mut_ptr());
        rs_clear(rmins.as_mut_ptr(), rmaxs.as_mut_ptr());

        let points: [[f32;3]; 4] = [
            [1.0, 2.0, 3.0], [-1.0, -2.0, -3.0], [5.0, 0.0, -5.0], [0.0, 0.0, 0.0],
        ];
        for p in &points {
            c_fn(p.as_ptr(), cmins.as_mut_ptr(), cmaxs.as_mut_ptr());
            rs_fn(p.as_ptr(), rmins.as_mut_ptr(), rmaxs.as_mut_ptr());
        }
        assert_f32_arr_eq(&cmins, &rmins, "AddPointToBounds mins");
        assert_f32_arr_eq(&cmaxs, &rmaxs, "AddPointToBounds maxs");
    }
}

// ==================== Plane functions ====================

#[test]
fn test_set_plane_signbits() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut CPlane)> = libs.c.get(b"SetPlaneSignbits\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut CPlane)> = libs.rs.get(b"SetPlaneSignbits\0").unwrap();
        let normals: [[f32;3]; 4] = [
            [1.0, 0.0, 0.0], [-1.0, -1.0, -1.0], [0.5, -0.5, 0.5], [-0.3, 0.7, -0.1],
        ];
        for n in &normals {
            let mut cp = CPlane { normal: *n, dist: 0.0, type_: 0, signbits: 0, pad: [0; 2] };
            let mut rp = cp.clone();
            c_fn(&mut cp);
            rs_fn(&mut rp);
            assert_eq!(cp.signbits, rp.signbits, "SetPlaneSignbits({:?})", n);
        }
    }
}

#[test]
fn test_box_on_plane_side() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut f32, *mut f32, *mut CPlane) -> i32> = libs.c.get(b"BoxOnPlaneSide\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut f32, *mut f32, *mut CPlane) -> i32> = libs.rs.get(b"BoxOnPlaneSide\0").unwrap();
        let c_signbits: Symbol<unsafe extern "C" fn(*mut CPlane)> = libs.c.get(b"SetPlaneSignbits\0").unwrap();

        // Test all 8 signbits combos via different normals
        let normals: [[f32;3]; 4] = [
            [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [-1.0, -1.0, 0.0], [0.577, 0.577, 0.577],
        ];
        for n in &normals {
            for &dist in &[0.0f32, 5.0, -5.0] {
                let mut cp = CPlane { normal: *n, dist, type_: 3, signbits: 0, pad: [0; 2] };
                c_signbits(&mut cp);
                let mut rp = cp.clone();
                let mut cemins = [-1.0f32, -1.0, -1.0];
                let mut cemaxs = [1.0f32, 1.0, 1.0];
                let mut remins = cemins;
                let mut remaxs = cemaxs;
                let cr = c_fn(cemins.as_mut_ptr(), cemaxs.as_mut_ptr(), &mut cp);
                let rr = rs_fn(remins.as_mut_ptr(), remaxs.as_mut_ptr(), &mut rp);
                assert_eq!(cr, rr, "BoxOnPlaneSide normal={:?} dist={}", n, dist);
            }
        }
    }
}

#[test]
fn test_plane_from_points() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut f32, *const f32, *const f32, *const f32) -> i32> = libs.c.get(b"PlaneFromPoints\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut f32, *const f32, *const f32, *const f32) -> i32> = libs.rs.get(b"PlaneFromPoints\0").unwrap();
        let a = [0.0f32, 0.0, 0.0];
        let b = [1.0f32, 0.0, 0.0];
        let c = [0.0f32, 1.0, 0.0];
        let (mut cp, mut rp) = ([0.0f32; 4], [0.0f32; 4]);
        let cr = c_fn(cp.as_mut_ptr(), a.as_ptr(), b.as_ptr(), c.as_ptr());
        let rr = rs_fn(rp.as_mut_ptr(), a.as_ptr(), b.as_ptr(), c.as_ptr());
        assert_eq!(cr, rr, "PlaneFromPoints return");
        assert_f32_arr_eq(&cp, &rp, "PlaneFromPoints plane");

        // Degenerate case
        let d = [0.0f32, 0.0, 0.0];
        let (mut cp2, mut rp2) = ([0.0f32; 4], [0.0f32; 4]);
        let cr2 = c_fn(cp2.as_mut_ptr(), d.as_ptr(), d.as_ptr(), d.as_ptr());
        let rr2 = rs_fn(rp2.as_mut_ptr(), d.as_ptr(), d.as_ptr(), d.as_ptr());
        assert_eq!(cr2, rr2, "PlaneFromPoints degenerate return");
    }
}

// ==================== Complex functions ====================

#[test]
fn test_vectoangles() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, *mut f32)> = libs.c.get(b"vectoangles\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, *mut f32)> = libs.rs.get(b"vectoangles\0").unwrap();
        let inputs: [[f32;3]; 5] = [
            [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0],
            [1.0, 1.0, 0.0], [0.0, 0.0, 0.0],
        ];
        for v in &inputs {
            let (mut co, mut ro) = ([0.0f32; 3], [0.0f32; 3]);
            c_fn(v.as_ptr(), co.as_mut_ptr());
            rs_fn(v.as_ptr(), ro.as_mut_ptr());
            assert_f32_arr_eq(&co, &ro, &format!("vectoangles({:?})", v));
        }
    }
}

#[test]
fn test_angles_to_axis() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, *mut [f32; 3])> = libs.c.get(b"AnglesToAxis\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, *mut [f32; 3])> = libs.rs.get(b"AnglesToAxis\0").unwrap();
        let inputs: [[f32;3]; 3] = [[0.0, 0.0, 0.0], [90.0, 0.0, 0.0], [45.0, 90.0, 30.0]];
        for angles in &inputs {
            let (mut ca, mut ra) = ([[0.0f32; 3]; 3], [[0.0f32; 3]; 3]);
            c_fn(angles.as_ptr(), ca.as_mut_ptr());
            rs_fn(angles.as_ptr(), ra.as_mut_ptr());
            for i in 0..3 {
                assert_f32_arr_eq(&ca[i], &ra[i], &format!("AnglesToAxis({:?})[{}]", angles, i));
            }
        }
    }
}

#[test]
fn test_axis_clear() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut [f32; 3])> = libs.c.get(b"AxisClear\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut [f32; 3])> = libs.rs.get(b"AxisClear\0").unwrap();
        let (mut ca, mut ra) = ([[9.0f32; 3]; 3], [[9.0f32; 3]; 3]);
        c_fn(ca.as_mut_ptr());
        rs_fn(ra.as_mut_ptr());
        for i in 0..3 {
            assert_f32_arr_eq(&ca[i], &ra[i], &format!("AxisClear[{}]", i));
        }
    }
}

#[test]
fn test_axis_copy() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut [f32; 3], *mut [f32; 3])> = libs.c.get(b"AxisCopy\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut [f32; 3], *mut [f32; 3])> = libs.rs.get(b"AxisCopy\0").unwrap();
        let mut src: [[f32; 3]; 3] = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
        let mut src2 = src;
        let (mut co, mut ro) = ([[0.0f32; 3]; 3], [[0.0f32; 3]; 3]);
        c_fn(src.as_mut_ptr(), co.as_mut_ptr());
        rs_fn(src2.as_mut_ptr(), ro.as_mut_ptr());
        for i in 0..3 {
            assert_f32_arr_eq(&co[i], &ro[i], &format!("AxisCopy[{}]", i));
        }
    }
}

#[test]
fn test_project_point_on_plane() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut f32, *const f32, *const f32)> = libs.c.get(b"ProjectPointOnPlane\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut f32, *const f32, *const f32)> = libs.rs.get(b"ProjectPointOnPlane\0").unwrap();
        let p = [1.0f32, 2.0, 3.0];
        let normal = [0.0f32, 0.0, 1.0];
        let (mut cd, mut rd) = ([0.0f32; 3], [0.0f32; 3]);
        c_fn(cd.as_mut_ptr(), p.as_ptr(), normal.as_ptr());
        rs_fn(rd.as_mut_ptr(), p.as_ptr(), normal.as_ptr());
        assert_f32_arr_eq(&cd, &rd, "ProjectPointOnPlane");
    }
}

#[test]
fn test_make_normal_vectors() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, *mut f32, *mut f32)> = libs.c.get(b"MakeNormalVectors\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, *mut f32, *mut f32)> = libs.rs.get(b"MakeNormalVectors\0").unwrap();
        let inputs: [[f32;3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.577, 0.577, 0.577]];
        for fwd in &inputs {
            let (mut cr, mut rr) = ([0.0f32; 3], [0.0f32; 3]);
            let (mut cu, mut ru) = ([0.0f32; 3], [0.0f32; 3]);
            c_fn(fwd.as_ptr(), cr.as_mut_ptr(), cu.as_mut_ptr());
            rs_fn(fwd.as_ptr(), rr.as_mut_ptr(), ru.as_mut_ptr());
            assert_f32_arr_eq(&cr, &rr, "MakeNormalVectors right");
            assert_f32_arr_eq(&cu, &ru, "MakeNormalVectors up");
        }
    }
}

#[test]
fn test_rotate_point_around_vector() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut f32, *const f32, *const f32, f32)> = libs.c.get(b"RotatePointAroundVector\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut f32, *const f32, *const f32, f32)> = libs.rs.get(b"RotatePointAroundVector\0").unwrap();
        let dir = [0.0f32, 0.0, 1.0];
        let point = [1.0f32, 0.0, 0.0];
        for &deg in &[0.0f32, 90.0, 180.0, 270.0, 45.0, -90.0] {
            let (mut co, mut ro) = ([0.0f32; 3], [0.0f32; 3]);
            c_fn(co.as_mut_ptr(), dir.as_ptr(), point.as_ptr(), deg);
            rs_fn(ro.as_mut_ptr(), dir.as_ptr(), point.as_ptr(), deg);
            assert_f32_arr_eq(&co, &ro, &format!("RotatePointAroundVector({})", deg));
        }
    }
}

#[test]
fn test_rotate_around_direction() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut [f32; 3], f32)> = libs.c.get(b"RotateAroundDirection\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut [f32; 3], f32)> = libs.rs.get(b"RotateAroundDirection\0").unwrap();
        for &yaw in &[0.0f32, 90.0, 180.0, 45.0] {
            let mut ca: [[f32; 3]; 3] = [[0.0, 0.0, 1.0], [0.0; 3], [0.0; 3]];
            let mut ra = ca;
            c_fn(ca.as_mut_ptr(), yaw);
            rs_fn(ra.as_mut_ptr(), yaw);
            for i in 0..3 {
                assert_f32_arr_eq(&ca[i], &ra[i], &format!("RotateAroundDirection({})[{}]", yaw, i));
            }
        }
    }
}

#[test]
fn test_angle_vectors() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*const f32, *mut f32, *mut f32, *mut f32)> = libs.c.get(b"AngleVectors\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*const f32, *mut f32, *mut f32, *mut f32)> = libs.rs.get(b"AngleVectors\0").unwrap();
        let inputs: [[f32;3]; 3] = [[0.0, 0.0, 0.0], [90.0, 0.0, 0.0], [30.0, 60.0, 90.0]];
        for angles in &inputs {
            let (mut cf, mut rf) = ([0.0f32; 3], [0.0f32; 3]);
            let (mut cr, mut rr) = ([0.0f32; 3], [0.0f32; 3]);
            let (mut cu, mut ru) = ([0.0f32; 3], [0.0f32; 3]);
            c_fn(angles.as_ptr(), cf.as_mut_ptr(), cr.as_mut_ptr(), cu.as_mut_ptr());
            rs_fn(angles.as_ptr(), rf.as_mut_ptr(), rr.as_mut_ptr(), ru.as_mut_ptr());
            assert_f32_arr_eq(&cf, &rf, &format!("AngleVectors({:?}) fwd", angles));
            assert_f32_arr_eq(&cr, &rr, &format!("AngleVectors({:?}) right", angles));
            assert_f32_arr_eq(&cu, &ru, &format!("AngleVectors({:?}) up", angles));
        }
        // Test with NULL pointers for right/up
        let angles = [45.0f32, 45.0, 0.0];
        let (mut cf, mut rf) = ([0.0f32; 3], [0.0f32; 3]);
        c_fn(angles.as_ptr(), cf.as_mut_ptr(), ptr::null_mut(), ptr::null_mut());
        rs_fn(angles.as_ptr(), rf.as_mut_ptr(), ptr::null_mut(), ptr::null_mut());
        assert_f32_arr_eq(&cf, &rf, "AngleVectors forward-only");
    }
}

#[test]
fn test_perpendicular_vector() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut f32, *const f32)> = libs.c.get(b"PerpendicularVector\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut f32, *const f32)> = libs.rs.get(b"PerpendicularVector\0").unwrap();
        let inputs: [[f32;3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.577, 0.577, 0.577]];
        for src in &inputs {
            let (mut co, mut ro) = ([0.0f32; 3], [0.0f32; 3]);
            c_fn(co.as_mut_ptr(), src.as_ptr());
            rs_fn(ro.as_mut_ptr(), src.as_ptr());
            assert_f32_arr_eq(&co, &ro, &format!("PerpendicularVector({:?})", src));
        }
    }
}

#[test]
fn test_matrix_multiply() {
    let libs = load_libs();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut [f32; 3], *mut [f32; 3], *mut [f32; 3])> = libs.c.get(b"MatrixMultiply\0").unwrap();
        let rs_fn: Symbol<unsafe extern "C" fn(*mut [f32; 3], *mut [f32; 3], *mut [f32; 3])> = libs.rs.get(b"MatrixMultiply\0").unwrap();
        let mut in1: [[f32; 3]; 3] = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
        let mut in2: [[f32; 3]; 3] = [[9.0, 8.0, 7.0], [6.0, 5.0, 4.0], [3.0, 2.0, 1.0]];
        let mut in1r = in1;
        let mut in2r = in2;
        let (mut co, mut ro) = ([[0.0f32; 3]; 3], [[0.0f32; 3]; 3]);
        c_fn(in1.as_mut_ptr(), in2.as_mut_ptr(), co.as_mut_ptr());
        rs_fn(in1r.as_mut_ptr(), in2r.as_mut_ptr(), ro.as_mut_ptr());
        for i in 0..3 {
            assert_f32_arr_eq(&co[i], &ro[i], &format!("MatrixMultiply[{}]", i));
        }

        // Identity test
        let mut id: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let mut id2 = id;
        let mut m: [[f32; 3]; 3] = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
        let mut m2 = m;
        let (mut co2, mut ro2) = ([[0.0f32; 3]; 3], [[0.0f32; 3]; 3]);
        c_fn(id.as_mut_ptr(), m.as_mut_ptr(), co2.as_mut_ptr());
        rs_fn(id2.as_mut_ptr(), m2.as_mut_ptr(), ro2.as_mut_ptr());
        for i in 0..3 {
            assert_f32_arr_eq(&co2[i], &ro2[i], &format!("MatrixMultiply identity[{}]", i));
        }
    }
}

// ==================== Data symbols ====================

#[test]
fn test_vec3_origin() {
    let libs = load_libs();
    unsafe {
        let c_ptr: Symbol<*const [f32; 3]> = libs.c.get(b"vec3_origin\0").unwrap();
        let rs_ptr: Symbol<*const [f32; 3]> = libs.rs.get(b"vec3_origin\0").unwrap();
        assert_f32_arr_eq(&**c_ptr, &**rs_ptr, "vec3_origin");
    }
}

#[test]
fn test_axis_default() {
    let libs = load_libs();
    unsafe {
        let c_ptr: Symbol<*const [[f32; 3]; 3]> = libs.c.get(b"axisDefault\0").unwrap();
        let rs_ptr: Symbol<*const [[f32; 3]; 3]> = libs.rs.get(b"axisDefault\0").unwrap();
        for i in 0..3 {
            assert_f32_arr_eq(&(**c_ptr)[i], &(**rs_ptr)[i], &format!("axisDefault[{}]", i));
        }
    }
}

#[test]
fn test_bytedirs() {
    let libs = load_libs();
    unsafe {
        let c_ptr: Symbol<*const [[f32; 3]; 162]> = libs.c.get(b"bytedirs\0").unwrap();
        let rs_ptr: Symbol<*const [[f32; 3]; 162]> = libs.rs.get(b"bytedirs\0").unwrap();
        for i in 0..162 {
            assert_f32_arr_eq(&(**c_ptr)[i], &(**rs_ptr)[i], &format!("bytedirs[{}]", i));
        }
    }
}

#[test]
fn test_color_data() {
    let libs = load_libs();
    unsafe {
        let names = [
            "colorBlack", "colorRed", "colorGreen", "colorBlue",
            "colorYellow", "colorMagenta", "colorCyan", "colorWhite",
            "colorLtGrey", "colorMdGrey", "colorDkGrey",
        ];
        for name in &names {
            let sym = format!("{}\0", name);
            let c_ptr: Symbol<*const [f32; 4]> = libs.c.get(sym.as_bytes()).unwrap();
            let rs_ptr: Symbol<*const [f32; 4]> = libs.rs.get(sym.as_bytes()).unwrap();
            assert_f32_arr_eq(&**c_ptr, &**rs_ptr, name);
        }
    }
}

#[test]
fn test_g_color_table() {
    let libs = load_libs();
    unsafe {
        let c_ptr: Symbol<*const [[f32; 4]; 8]> = libs.c.get(b"g_color_table\0").unwrap();
        let rs_ptr: Symbol<*const [[f32; 4]; 8]> = libs.rs.get(b"g_color_table\0").unwrap();
        for i in 0..8 {
            assert_f32_arr_eq(&(**c_ptr)[i], &(**rs_ptr)[i], &format!("g_color_table[{}]", i));
        }
    }
}
