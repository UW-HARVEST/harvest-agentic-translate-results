use libloading::{Library, Symbol};
use std::path::PathBuf;

type Vec3T = [f32; 3];
type Vec4T = [f32; 4];

#[repr(C)]
struct CplaneT {
    normal: Vec3T,
    dist: f32,
    type_: u8,
    signbits: u8,
    pad: [u8; 2],
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libqmath.so")
}

fn load_c_lib() -> Library {
    unsafe { Library::new(c_lib_path()).expect("Failed to load C library") }
}

macro_rules! assert_f32_eq {
    ($a:expr, $b:expr, $name:expr) => {
        assert!($a.to_bits() == $b.to_bits(),
            "{}: C={} (bits {:08x}) vs Rust={} (bits {:08x})",
            $name, $a, $a.to_bits(), $b, $b.to_bits());
    };
}

macro_rules! assert_vec3_eq {
    ($a:expr, $b:expr, $name:expr) => {
        assert_f32_eq!($a[0], $b[0], concat!($name, "[0]"));
        assert_f32_eq!($a[1], $b[1], concat!($name, "[1]"));
        assert_f32_eq!($a[2], $b[2], concat!($name, "[2]"));
    };
}

// ============ Lowest-level functions ============

#[test]
fn test_q_rsqrt() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = unsafe { lib.get(b"Q_rsqrt").unwrap() };
    for &val in &[1.0f32, 4.0, 0.25, 100.0, 0.01] {
        let c_res = unsafe { c_fn(val) };
        let r_res = qmath::Q_rsqrt(val);
        assert_f32_eq!(c_res, r_res, "Q_rsqrt");
    }
}

#[test]
fn test_q_fabs() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = unsafe { lib.get(b"Q_fabs").unwrap() };
    for &val in &[1.0f32, -1.0, 0.0, -0.0, 3.14, -3.14] {
        let c_res = unsafe { c_fn(val) };
        let r_res = qmath::Q_fabs(val);
        assert_f32_eq!(c_res, r_res, "Q_fabs");
    }
}

#[test]
fn test_clamp_char() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32) -> i8> = unsafe { lib.get(b"ClampChar").unwrap() };
    for &val in &[-200, -128, -1, 0, 1, 127, 200] {
        let c_res = unsafe { c_fn(val) };
        let r_res = qmath::ClampChar(val);
        assert_eq!(c_res, r_res, "ClampChar({})", val);
    }
}

#[test]
fn test_clamp_short() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32) -> i16> = unsafe { lib.get(b"ClampShort").unwrap() };
    for &val in &[-40000, -32768, -1, 0, 1, 32767, 40000] {
        let c_res = unsafe { c_fn(val) };
        let r_res = qmath::ClampShort(val);
        assert_eq!(c_res, r_res, "ClampShort({})", val);
    }
}

#[test]
fn test_q_log2() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32) -> i32> = unsafe { lib.get(b"Q_log2").unwrap() };
    for &val in &[1, 2, 3, 4, 7, 8, 15, 16, 255, 256, 1024] {
        let c_res = unsafe { c_fn(val) };
        let r_res = qmath::Q_log2(val);
        assert_eq!(c_res, r_res, "Q_log2({})", val);
    }
}

#[test]
fn test_q_rand() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*mut i32) -> i32> = unsafe { lib.get(b"Q_rand").unwrap() };
    let mut c_seed: i32 = 12345;
    let mut r_seed: i32 = 12345;
    for _ in 0..100 {
        let c_res = unsafe { c_fn(&mut c_seed) };
        let r_res = qmath::Q_rand(&mut r_seed);
        assert_eq!(c_res, r_res, "Q_rand");
        assert_eq!(c_seed, r_seed, "Q_rand seed");
    }
}

#[test]
fn test_q_random() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*mut i32) -> f32> = unsafe { lib.get(b"Q_random").unwrap() };
    let mut c_seed: i32 = 42;
    let mut r_seed: i32 = 42;
    for _ in 0..50 {
        let c_res = unsafe { c_fn(&mut c_seed) };
        let r_res = qmath::Q_random(&mut r_seed);
        assert_f32_eq!(c_res, r_res, "Q_random");
    }
}

#[test]
fn test_q_crandom() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*mut i32) -> f32> = unsafe { lib.get(b"Q_crandom").unwrap() };
    let mut c_seed: i32 = 99;
    let mut r_seed: i32 = 99;
    for _ in 0..50 {
        let c_res = unsafe { c_fn(&mut c_seed) };
        let r_res = qmath::Q_crandom(&mut r_seed);
        assert_f32_eq!(c_res, r_res, "Q_crandom");
    }
}

// ============ Vector operations ============

#[test]
fn test_dot_product() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, &Vec3T) -> f32> = unsafe { lib.get(b"_DotProduct").unwrap() };
    let a: Vec3T = [1.0, 2.0, 3.0];
    let b: Vec3T = [4.0, 5.0, 6.0];
    let c_res = unsafe { c_fn(&a, &b) };
    let r_res = qmath::_DotProduct(&a, &b);
    assert_f32_eq!(c_res, r_res, "_DotProduct");
}

#[test]
fn test_vector_subtract() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, &Vec3T, &mut Vec3T)> = unsafe { lib.get(b"_VectorSubtract").unwrap() };
    let a: Vec3T = [1.0, 2.0, 3.0];
    let b: Vec3T = [4.0, 5.0, 6.0];
    let mut c_out = [0.0f32; 3];
    let mut r_out = [0.0f32; 3];
    unsafe { c_fn(&a, &b, &mut c_out) };
    qmath::_VectorSubtract(&a, &b, &mut r_out);
    assert_vec3_eq!(c_out, r_out, "_VectorSubtract");
}

#[test]
fn test_vector_add() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, &Vec3T, &mut Vec3T)> = unsafe { lib.get(b"_VectorAdd").unwrap() };
    let a: Vec3T = [1.0, 2.0, 3.0];
    let b: Vec3T = [4.0, 5.0, 6.0];
    let mut c_out = [0.0f32; 3];
    let mut r_out = [0.0f32; 3];
    unsafe { c_fn(&a, &b, &mut c_out) };
    qmath::_VectorAdd(&a, &b, &mut r_out);
    assert_vec3_eq!(c_out, r_out, "_VectorAdd");
}

#[test]
fn test_vector_copy() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, &mut Vec3T)> = unsafe { lib.get(b"_VectorCopy").unwrap() };
    let a: Vec3T = [1.5, 2.5, 3.5];
    let mut c_out = [0.0f32; 3];
    let mut r_out = [0.0f32; 3];
    unsafe { c_fn(&a, &mut c_out) };
    qmath::_VectorCopy(&a, &mut r_out);
    assert_vec3_eq!(c_out, r_out, "_VectorCopy");
}

#[test]
fn test_vector_scale() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, f32, &mut Vec3T)> = unsafe { lib.get(b"_VectorScale").unwrap() };
    let a: Vec3T = [1.0, 2.0, 3.0];
    let mut c_out = [0.0f32; 3];
    let mut r_out = [0.0f32; 3];
    unsafe { c_fn(&a, 2.5, &mut c_out) };
    qmath::_VectorScale(&a, 2.5, &mut r_out);
    assert_vec3_eq!(c_out, r_out, "_VectorScale");
}

#[test]
fn test_vector_ma() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, f32, &Vec3T, &mut Vec3T)> = unsafe { lib.get(b"_VectorMA").unwrap() };
    let a: Vec3T = [1.0, 2.0, 3.0];
    let b: Vec3T = [4.0, 5.0, 6.0];
    let mut c_out = [0.0f32; 3];
    let mut r_out = [0.0f32; 3];
    unsafe { c_fn(&a, 2.0, &b, &mut c_out) };
    qmath::_VectorMA(&a, 2.0, &b, &mut r_out);
    assert_vec3_eq!(c_out, r_out, "_VectorMA");
}

#[test]
fn test_vector4_scale() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec4T, f32, &mut Vec4T)> = unsafe { lib.get(b"Vector4Scale").unwrap() };
    let a: Vec4T = [1.0, 2.0, 3.0, 4.0];
    let mut c_out = [0.0f32; 4];
    let mut r_out = [0.0f32; 4];
    unsafe { c_fn(&a, 3.0, &mut c_out) };
    qmath::Vector4Scale(&a, 3.0, &mut r_out);
    for i in 0..4 {
        assert_f32_eq!(c_out[i], r_out[i], "Vector4Scale");
    }
}

#[test]
fn test_vector_normalize() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&mut Vec3T) -> f32> = unsafe { lib.get(b"VectorNormalize").unwrap() };
    for v in &[[3.0f32, 4.0, 0.0], [1.0, 1.0, 1.0], [0.0, 0.0, 0.0], [0.0, 5.0, 0.0]] {
        let mut c_v = *v;
        let mut r_v = *v;
        let c_len = unsafe { c_fn(&mut c_v) };
        let r_len = qmath::VectorNormalize(&mut r_v);
        assert_f32_eq!(c_len, r_len, "VectorNormalize len");
        assert_vec3_eq!(c_v, r_v, "VectorNormalize");
    }
}

#[test]
fn test_vector_normalize2() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, &mut Vec3T) -> f32> = unsafe { lib.get(b"VectorNormalize2").unwrap() };
    let v: Vec3T = [3.0, 4.0, 0.0];
    let mut c_out = [0.0f32; 3];
    let mut r_out = [0.0f32; 3];
    let c_len = unsafe { c_fn(&v, &mut c_out) };
    let r_len = qmath::VectorNormalize2(&v, &mut r_out);
    assert_f32_eq!(c_len, r_len, "VectorNormalize2 len");
    assert_vec3_eq!(c_out, r_out, "VectorNormalize2");
}

// ============ Angle functions ============

#[test]
fn test_lerp_angle() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f32, f32, f32) -> f32> = unsafe { lib.get(b"LerpAngle").unwrap() };
    for &(from, to, frac) in &[(0.0f32, 90.0, 0.5), (350.0, 10.0, 0.5), (10.0, 350.0, 0.5), (0.0, 180.0, 0.25)] {
        let c_res = unsafe { c_fn(from, to, frac) };
        let r_res = qmath::LerpAngle(from, to, frac);
        assert_f32_eq!(c_res, r_res, "LerpAngle");
    }
}

#[test]
fn test_angle_subtract() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f32, f32) -> f32> = unsafe { lib.get(b"AngleSubtract").unwrap() };
    for &(a1, a2) in &[(90.0f32, 45.0), (10.0, 350.0), (350.0, 10.0), (0.0, 180.0)] {
        let c_res = unsafe { c_fn(a1, a2) };
        let r_res = qmath::AngleSubtract(a1, a2);
        assert_f32_eq!(c_res, r_res, "AngleSubtract");
    }
}

#[test]
fn test_angles_subtract() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, &Vec3T, &mut Vec3T)> = unsafe { lib.get(b"AnglesSubtract").unwrap() };
    let v1: Vec3T = [90.0, 180.0, 270.0];
    let v2: Vec3T = [45.0, 350.0, 10.0];
    let mut c_out = [0.0f32; 3];
    let mut r_out = [0.0f32; 3];
    unsafe { c_fn(&v1, &v2, &mut c_out) };
    qmath::AnglesSubtract(&v1, &v2, &mut r_out);
    assert_vec3_eq!(c_out, r_out, "AnglesSubtract");
}

#[test]
fn test_angle_mod() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = unsafe { lib.get(b"AngleMod").unwrap() };
    for &val in &[0.0f32, 90.0, 180.0, 360.0, 720.0, -90.0, -360.0, 45.5] {
        let c_res = unsafe { c_fn(val) };
        let r_res = qmath::AngleMod(val);
        assert_f32_eq!(c_res, r_res, "AngleMod");
    }
}

#[test]
fn test_angle_normalize_360() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = unsafe { lib.get(b"AngleNormalize360").unwrap() };
    for &val in &[0.0f32, 90.0, 360.0, 720.0, -90.0, -360.0, 45.5] {
        let c_res = unsafe { c_fn(val) };
        let r_res = qmath::AngleNormalize360(val);
        assert_f32_eq!(c_res, r_res, "AngleNormalize360");
    }
}

#[test]
fn test_angle_normalize_180() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f32) -> f32> = unsafe { lib.get(b"AngleNormalize180").unwrap() };
    for &val in &[0.0f32, 90.0, 180.0, 270.0, 360.0, -90.0, -180.0] {
        let c_res = unsafe { c_fn(val) };
        let r_res = qmath::AngleNormalize180(val);
        assert_f32_eq!(c_res, r_res, "AngleNormalize180");
    }
}

#[test]
fn test_angle_delta() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f32, f32) -> f32> = unsafe { lib.get(b"AngleDelta").unwrap() };
    for &(a1, a2) in &[(90.0f32, 45.0), (10.0, 350.0), (0.0, 180.0)] {
        let c_res = unsafe { c_fn(a1, a2) };
        let r_res = qmath::AngleDelta(a1, a2);
        assert_f32_eq!(c_res, r_res, "AngleDelta");
    }
}

// ============ Color functions ============

#[test]
fn test_color_bytes3() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f32, f32, f32) -> u32> = unsafe { lib.get(b"ColorBytes3").unwrap() };
    for &(r, g, b) in &[(1.0f32, 0.0, 0.0), (0.0, 1.0, 0.0), (0.5, 0.5, 0.5), (0.0, 0.0, 0.0)] {
        let c_res = unsafe { c_fn(r, g, b) };
        let r_res = qmath::ColorBytes3(r, g, b);
        assert_eq!(c_res, r_res, "ColorBytes3({},{},{}): C={:#x} Rust={:#x}", r, g, b, c_res, r_res);
    }
}

#[test]
fn test_color_bytes4() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f32, f32, f32, f32) -> u32> = unsafe { lib.get(b"ColorBytes4").unwrap() };
    let c_res = unsafe { c_fn(1.0, 0.5, 0.25, 1.0) };
    let r_res = qmath::ColorBytes4(1.0, 0.5, 0.25, 1.0);
    assert_eq!(c_res, r_res, "ColorBytes4: C={:#x} Rust={:#x}", c_res, r_res);
}

#[test]
fn test_normalize_color() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, &mut Vec3T) -> f32> = unsafe { lib.get(b"NormalizeColor").unwrap() };
    let input: Vec3T = [0.5, 0.8, 0.3];
    let mut c_out = [0.0f32; 3];
    let mut r_out = [0.0f32; 3];
    let c_max = unsafe { c_fn(&input, &mut c_out) };
    let r_max = qmath::NormalizeColor(&input, &mut r_out);
    assert_f32_eq!(c_max, r_max, "NormalizeColor max");
    assert_vec3_eq!(c_out, r_out, "NormalizeColor");
}

// ============ Dir/Byte functions ============

#[test]
fn test_dir_to_byte() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const Vec3T) -> i32> = unsafe { lib.get(b"DirToByte").unwrap() };
    let dirs: [Vec3T; 4] = [
        [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.577, 0.577, 0.577],
    ];
    for d in &dirs {
        let c_res = unsafe { c_fn(d as *const _) };
        let r_res = qmath::DirToByte(d as *const _);
        assert_eq!(c_res, r_res, "DirToByte({:?})", d);
    }
}

#[test]
fn test_byte_to_dir() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32, &mut Vec3T)> = unsafe { lib.get(b"ByteToDir").unwrap() };
    for b in [0, 1, 50, 100, 161, -1, 200] {
        let mut c_out = [0.0f32; 3];
        let mut r_out = [0.0f32; 3];
        unsafe { c_fn(b, &mut c_out) };
        qmath::ByteToDir(b, &mut r_out);
        assert_vec3_eq!(c_out, r_out, "ByteToDir");
    }
}

// ============ Bounds functions ============

#[test]
fn test_clear_bounds() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&mut Vec3T, &mut Vec3T)> = unsafe { lib.get(b"ClearBounds").unwrap() };
    let mut c_mins = [0.0f32; 3]; let mut c_maxs = [0.0f32; 3];
    let mut r_mins = [0.0f32; 3]; let mut r_maxs = [0.0f32; 3];
    unsafe { c_fn(&mut c_mins, &mut c_maxs) };
    qmath::ClearBounds(&mut r_mins, &mut r_maxs);
    assert_vec3_eq!(c_mins, r_mins, "ClearBounds mins");
    assert_vec3_eq!(c_maxs, r_maxs, "ClearBounds maxs");
}

#[test]
fn test_add_point_to_bounds() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, &mut Vec3T, &mut Vec3T)> = unsafe { lib.get(b"AddPointToBounds").unwrap() };
    let mut c_mins = [99999.0f32; 3]; let mut c_maxs = [-99999.0f32; 3];
    let mut r_mins = [99999.0f32; 3]; let mut r_maxs = [-99999.0f32; 3];
    let points = [[1.0f32, -2.0, 3.0], [-1.0, 5.0, -3.0], [0.0, 0.0, 0.0]];
    for p in &points {
        unsafe { c_fn(p, &mut c_mins, &mut c_maxs) };
        qmath::AddPointToBounds(p, &mut r_mins, &mut r_maxs);
    }
    assert_vec3_eq!(c_mins, r_mins, "AddPointToBounds mins");
    assert_vec3_eq!(c_maxs, r_maxs, "AddPointToBounds maxs");
}

#[test]
fn test_radius_from_bounds() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, &Vec3T) -> f32> = unsafe { lib.get(b"RadiusFromBounds").unwrap() };
    let mins: Vec3T = [-3.0, -4.0, -5.0];
    let maxs: Vec3T = [1.0, 2.0, 3.0];
    let c_res = unsafe { c_fn(&mins, &maxs) };
    let r_res = qmath::RadiusFromBounds(&mins, &maxs);
    assert_f32_eq!(c_res, r_res, "RadiusFromBounds");
}

// ============ Matrix and complex functions ============

#[test]
fn test_matrix_multiply() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&[[f32; 3]; 3], &[[f32; 3]; 3], &mut [[f32; 3]; 3])> =
        unsafe { lib.get(b"MatrixMultiply").unwrap() };
    let in1 = [[1.0f32, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
    let in2 = [[9.0f32, 8.0, 7.0], [6.0, 5.0, 4.0], [3.0, 2.0, 1.0]];
    let mut c_out = [[0.0f32; 3]; 3];
    let mut r_out = [[0.0f32; 3]; 3];
    unsafe { c_fn(&in1, &in2, &mut c_out) };
    qmath::MatrixMultiply(&in1, &in2, &mut r_out);
    for i in 0..3 {
        for j in 0..3 {
            assert_f32_eq!(c_out[i][j], r_out[i][j], "MatrixMultiply");
        }
    }
}

#[test]
fn test_angle_vectors() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, *mut Vec3T, *mut Vec3T, *mut Vec3T)> =
        unsafe { lib.get(b"AngleVectors").unwrap() };
    let angles: Vec3T = [30.0, 45.0, 60.0];
    let mut c_fwd = [0.0f32; 3]; let mut c_right = [0.0f32; 3]; let mut c_up = [0.0f32; 3];
    let mut r_fwd = [0.0f32; 3]; let mut r_right = [0.0f32; 3]; let mut r_up = [0.0f32; 3];
    unsafe { c_fn(&angles, &mut c_fwd, &mut c_right, &mut c_up) };
    qmath::AngleVectors(&angles, &mut r_fwd, &mut r_right, &mut r_up);
    assert_vec3_eq!(c_fwd, r_fwd, "AngleVectors fwd");
    assert_vec3_eq!(c_right, r_right, "AngleVectors right");
    assert_vec3_eq!(c_up, r_up, "AngleVectors up");
}

#[test]
fn test_perpendicular_vector() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&mut Vec3T, &Vec3T)> = unsafe { lib.get(b"PerpendicularVector").unwrap() };
    let src: Vec3T = [0.0, 0.0, 1.0];
    let mut c_dst = [0.0f32; 3];
    let mut r_dst = [0.0f32; 3];
    unsafe { c_fn(&mut c_dst, &src) };
    qmath::PerpendicularVector(&mut r_dst, &src);
    assert_vec3_eq!(c_dst, r_dst, "PerpendicularVector");
}

#[test]
fn test_project_point_on_plane() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&mut Vec3T, &Vec3T, &Vec3T)> = unsafe { lib.get(b"ProjectPointOnPlane").unwrap() };
    let p: Vec3T = [1.0, 2.0, 3.0];
    let normal: Vec3T = [0.0, 0.0, 1.0];
    let mut c_dst = [0.0f32; 3];
    let mut r_dst = [0.0f32; 3];
    unsafe { c_fn(&mut c_dst, &p, &normal) };
    qmath::ProjectPointOnPlane(&mut r_dst, &p, &normal);
    assert_vec3_eq!(c_dst, r_dst, "ProjectPointOnPlane");
}

#[test]
fn test_make_normal_vectors() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, &mut Vec3T, &mut Vec3T)> = unsafe { lib.get(b"MakeNormalVectors").unwrap() };
    let fwd: Vec3T = [0.0, 0.0, 1.0];
    let mut c_right = [0.0f32; 3]; let mut c_up = [0.0f32; 3];
    let mut r_right = [0.0f32; 3]; let mut r_up = [0.0f32; 3];
    unsafe { c_fn(&fwd, &mut c_right, &mut c_up) };
    qmath::MakeNormalVectors(&fwd, &mut r_right, &mut r_up);
    assert_vec3_eq!(c_right, r_right, "MakeNormalVectors right");
    assert_vec3_eq!(c_up, r_up, "MakeNormalVectors up");
}

#[test]
fn test_vector_rotate() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, &[Vec3T; 3], &mut Vec3T)> = unsafe { lib.get(b"VectorRotate").unwrap() };
    let v: Vec3T = [1.0, 0.0, 0.0];
    let matrix: [Vec3T; 3] = [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
    let mut c_out = [0.0f32; 3];
    let mut r_out = [0.0f32; 3];
    unsafe { c_fn(&v, &matrix, &mut c_out) };
    qmath::VectorRotate(&v, &matrix, &mut r_out);
    assert_vec3_eq!(c_out, r_out, "VectorRotate");
}

#[test]
fn test_rotate_point_around_vector() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&mut Vec3T, &Vec3T, &Vec3T, f32)> =
        unsafe { lib.get(b"RotatePointAroundVector").unwrap() };
    let dir: Vec3T = [0.0, 0.0, 1.0];
    let point: Vec3T = [1.0, 0.0, 0.0];
    let mut c_dst = [0.0f32; 3];
    let mut r_dst = [0.0f32; 3];
    unsafe { c_fn(&mut c_dst, &dir, &point, 90.0) };
    qmath::RotatePointAroundVector(&mut r_dst, &dir, &point, 90.0);
    assert_vec3_eq!(c_dst, r_dst, "RotatePointAroundVector");
}

#[test]
fn test_rotate_around_direction() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&mut [Vec3T; 3], f32)> =
        unsafe { lib.get(b"RotateAroundDirection").unwrap() };
    let mut c_axis: [Vec3T; 3] = [[0.0, 0.0, 1.0], [0.0; 3], [0.0; 3]];
    let mut r_axis: [Vec3T; 3] = [[0.0, 0.0, 1.0], [0.0; 3], [0.0; 3]];
    unsafe { c_fn(&mut c_axis, 45.0) };
    qmath::RotateAroundDirection(&mut r_axis, 45.0);
    for i in 0..3 {
        assert_vec3_eq!(c_axis[i], r_axis[i], "RotateAroundDirection");
    }
}

#[test]
fn test_vectoangles() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, &mut Vec3T)> = unsafe { lib.get(b"vectoangles").unwrap() };
    let inputs: [Vec3T; 4] = [
        [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0, 1.0],
    ];
    for v in &inputs {
        let mut c_out = [0.0f32; 3];
        let mut r_out = [0.0f32; 3];
        unsafe { c_fn(v, &mut c_out) };
        qmath::vectoangles(v, &mut r_out);
        assert_vec3_eq!(c_out, r_out, "vectoangles");
    }
}

#[test]
fn test_angles_to_axis() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, &mut [Vec3T; 3])> = unsafe { lib.get(b"AnglesToAxis").unwrap() };
    let angles: Vec3T = [30.0, 45.0, 0.0];
    let mut c_axis = [[0.0f32; 3]; 3];
    let mut r_axis = [[0.0f32; 3]; 3];
    unsafe { c_fn(&angles, &mut c_axis) };
    qmath::AnglesToAxis(&angles, &mut r_axis);
    for i in 0..3 {
        assert_vec3_eq!(c_axis[i], r_axis[i], "AnglesToAxis");
    }
}

#[test]
fn test_axis_clear() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&mut [Vec3T; 3])> = unsafe { lib.get(b"AxisClear").unwrap() };
    let mut c_axis = [[9.0f32; 3]; 3];
    let mut r_axis = [[9.0f32; 3]; 3];
    unsafe { c_fn(&mut c_axis) };
    qmath::AxisClear(&mut r_axis);
    for i in 0..3 {
        assert_vec3_eq!(c_axis[i], r_axis[i], "AxisClear");
    }
}

#[test]
fn test_axis_copy() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&[Vec3T; 3], &mut [Vec3T; 3])> = unsafe { lib.get(b"AxisCopy").unwrap() };
    let src: [Vec3T; 3] = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
    let mut c_out = [[0.0f32; 3]; 3];
    let mut r_out = [[0.0f32; 3]; 3];
    unsafe { c_fn(&src, &mut c_out) };
    qmath::AxisCopy(&src, &mut r_out);
    for i in 0..3 {
        assert_vec3_eq!(c_out[i], r_out[i], "AxisCopy");
    }
}

#[test]
fn test_set_plane_signbits() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&mut CplaneT)> = unsafe { lib.get(b"SetPlaneSignbits").unwrap() };
    let normals: [Vec3T; 4] = [
        [1.0, 1.0, 1.0], [-1.0, 1.0, 1.0], [-1.0, -1.0, 1.0], [-1.0, -1.0, -1.0],
    ];
    for n in &normals {
        let mut c_plane = CplaneT { normal: *n, dist: 0.0, type_: 3, signbits: 0, pad: [0; 2] };
        let mut r_plane = qmath::cplane_t { normal: *n, dist: 0.0, type_: 3, signbits: 0, pad: [0; 2] };
        unsafe { c_fn(&mut c_plane) };
        qmath::SetPlaneSignbits(&mut r_plane);
        assert_eq!(c_plane.signbits, r_plane.signbits, "SetPlaneSignbits({:?})", n);
    }
}

#[test]
fn test_box_on_plane_side() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&Vec3T, &Vec3T, &CplaneT) -> i32> =
        unsafe { lib.get(b"BoxOnPlaneSide").unwrap() };
    let emins: Vec3T = [-1.0, -1.0, -1.0];
    let emaxs: Vec3T = [1.0, 1.0, 1.0];
    // Test non-axial case
    let plane = CplaneT { normal: [0.577, 0.577, 0.577], dist: 0.0, type_: 3, signbits: 0, pad: [0; 2] };
    let r_plane = qmath::cplane_t { normal: [0.577, 0.577, 0.577], dist: 0.0, type_: 3, signbits: 0, pad: [0; 2] };
    let c_res = unsafe { c_fn(&emins, &emaxs, &plane) };
    let r_res = qmath::BoxOnPlaneSide(&emins, &emaxs, &r_plane);
    assert_eq!(c_res, r_res, "BoxOnPlaneSide");
}

#[test]
fn test_plane_from_points() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(&mut Vec4T, &Vec3T, &Vec3T, &Vec3T) -> i32> =
        unsafe { lib.get(b"PlaneFromPoints").unwrap() };
    let a: Vec3T = [0.0, 0.0, 0.0];
    let b: Vec3T = [1.0, 0.0, 0.0];
    let c: Vec3T = [0.0, 1.0, 0.0];
    let mut c_plane = [0.0f32; 4];
    let mut r_plane = [0.0f32; 4];
    let c_res = unsafe { c_fn(&mut c_plane, &a, &b, &c) };
    let r_res = qmath::PlaneFromPoints(&mut r_plane, &a, &b, &c);
    assert_eq!(c_res, r_res, "PlaneFromPoints return");
    for i in 0..4 {
        assert_f32_eq!(c_plane[i], r_plane[i], "PlaneFromPoints");
    }
}
