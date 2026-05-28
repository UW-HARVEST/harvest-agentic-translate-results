//! Rust translation of q_math.c (Quake III Arena math routines).
//!
//! All functions are exported with C ABI under their original C names so the
//! resulting cdylib has the same exported interface as the original C
//! shared library.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

use std::os::raw::{c_float, c_int, c_uint};

pub const NUMVERTEXNORMALS: usize = 162;
pub const M_PI: f32 = 3.14159265358979323846_f32;
pub const PITCH: usize = 0;
pub const YAW: usize = 1;
pub const ROLL: usize = 2;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cplane_t {
    pub normal: [f32; 3],
    pub dist: f32,
    pub r#type: u8,
    pub signbits: u8,
    pub pad: [u8; 2],
}

// ===================================================================
// Globals (mirroring those in q_math.c).
// ===================================================================

#[no_mangle]
pub static mut vec3_origin: [f32; 3] = [0.0, 0.0, 0.0];

#[no_mangle]
pub static mut axisDefault: [[f32; 3]; 3] = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
];

#[no_mangle]
pub static mut colorBlack:   [f32; 4] = [0.0, 0.0, 0.0, 1.0];
#[no_mangle]
pub static mut colorRed:     [f32; 4] = [1.0, 0.0, 0.0, 1.0];
#[no_mangle]
pub static mut colorGreen:   [f32; 4] = [0.0, 1.0, 0.0, 1.0];
#[no_mangle]
pub static mut colorBlue:    [f32; 4] = [0.0, 0.0, 1.0, 1.0];
#[no_mangle]
pub static mut colorYellow:  [f32; 4] = [1.0, 1.0, 0.0, 1.0];
#[no_mangle]
pub static mut colorMagenta: [f32; 4] = [1.0, 0.0, 1.0, 1.0];
#[no_mangle]
pub static mut colorCyan:    [f32; 4] = [0.0, 1.0, 1.0, 1.0];
#[no_mangle]
pub static mut colorWhite:   [f32; 4] = [1.0, 1.0, 1.0, 1.0];
#[no_mangle]
pub static mut colorLtGrey:  [f32; 4] = [0.75, 0.75, 0.75, 1.0];
#[no_mangle]
pub static mut colorMdGrey:  [f32; 4] = [0.5, 0.5, 0.5, 1.0];
#[no_mangle]
pub static mut colorDkGrey:  [f32; 4] = [0.25, 0.25, 0.25, 1.0];

#[no_mangle]
pub static mut g_color_table: [[f32; 4]; 8] = [
    [0.0, 0.0, 0.0, 1.0],
    [1.0, 0.0, 0.0, 1.0],
    [0.0, 1.0, 0.0, 1.0],
    [1.0, 1.0, 0.0, 1.0],
    [0.0, 0.0, 1.0, 1.0],
    [0.0, 1.0, 1.0, 1.0],
    [1.0, 0.0, 1.0, 1.0],
    [1.0, 1.0, 1.0, 1.0],
];

#[no_mangle]
pub static mut bytedirs: [[f32; 3]; NUMVERTEXNORMALS] = [
    [-0.525731, 0.000000, 0.850651], [-0.442863, 0.238856, 0.864188],
    [-0.295242, 0.000000, 0.955423], [-0.309017, 0.500000, 0.809017],
    [-0.162460, 0.262866, 0.951056], [0.000000, 0.000000, 1.000000],
    [0.000000, 0.850651, 0.525731], [-0.147621, 0.716567, 0.681718],
    [0.147621, 0.716567, 0.681718], [0.000000, 0.525731, 0.850651],
    [0.309017, 0.500000, 0.809017], [0.525731, 0.000000, 0.850651],
    [0.295242, 0.000000, 0.955423], [0.442863, 0.238856, 0.864188],
    [0.162460, 0.262866, 0.951056], [-0.681718, 0.147621, 0.716567],
    [-0.809017, 0.309017, 0.500000], [-0.587785, 0.425325, 0.688191],
    [-0.850651, 0.525731, 0.000000], [-0.864188, 0.442863, 0.238856],
    [-0.716567, 0.681718, 0.147621], [-0.688191, 0.587785, 0.425325],
    [-0.500000, 0.809017, 0.309017], [-0.238856, 0.864188, 0.442863],
    [-0.425325, 0.688191, 0.587785], [-0.716567, 0.681718, -0.147621],
    [-0.500000, 0.809017, -0.309017], [-0.525731, 0.850651, 0.000000],
    [0.000000, 0.850651, -0.525731], [-0.238856, 0.864188, -0.442863],
    [0.000000, 0.955423, -0.295242], [-0.262866, 0.951056, -0.162460],
    [0.000000, 1.000000, 0.000000], [0.000000, 0.955423, 0.295242],
    [-0.262866, 0.951056, 0.162460], [0.238856, 0.864188, 0.442863],
    [0.262866, 0.951056, 0.162460], [0.500000, 0.809017, 0.309017],
    [0.238856, 0.864188, -0.442863], [0.262866, 0.951056, -0.162460],
    [0.500000, 0.809017, -0.309017], [0.850651, 0.525731, 0.000000],
    [0.716567, 0.681718, 0.147621], [0.716567, 0.681718, -0.147621],
    [0.525731, 0.850651, 0.000000], [0.425325, 0.688191, 0.587785],
    [0.864188, 0.442863, 0.238856], [0.688191, 0.587785, 0.425325],
    [0.809017, 0.309017, 0.500000], [0.681718, 0.147621, 0.716567],
    [0.587785, 0.425325, 0.688191], [0.955423, 0.295242, 0.000000],
    [1.000000, 0.000000, 0.000000], [0.951056, 0.162460, 0.262866],
    [0.850651, -0.525731, 0.000000], [0.955423, -0.295242, 0.000000],
    [0.864188, -0.442863, 0.238856], [0.951056, -0.162460, 0.262866],
    [0.809017, -0.309017, 0.500000], [0.681718, -0.147621, 0.716567],
    [0.850651, 0.000000, 0.525731], [0.864188, 0.442863, -0.238856],
    [0.809017, 0.309017, -0.500000], [0.951056, 0.162460, -0.262866],
    [0.525731, 0.000000, -0.850651], [0.681718, 0.147621, -0.716567],
    [0.681718, -0.147621, -0.716567], [0.850651, 0.000000, -0.525731],
    [0.809017, -0.309017, -0.500000], [0.864188, -0.442863, -0.238856],
    [0.951056, -0.162460, -0.262866], [0.147621, 0.716567, -0.681718],
    [0.309017, 0.500000, -0.809017], [0.425325, 0.688191, -0.587785],
    [0.442863, 0.238856, -0.864188], [0.587785, 0.425325, -0.688191],
    [0.688191, 0.587785, -0.425325], [-0.147621, 0.716567, -0.681718],
    [-0.309017, 0.500000, -0.809017], [0.000000, 0.525731, -0.850651],
    [-0.525731, 0.000000, -0.850651], [-0.442863, 0.238856, -0.864188],
    [-0.295242, 0.000000, -0.955423], [-0.162460, 0.262866, -0.951056],
    [0.000000, 0.000000, -1.000000], [0.295242, 0.000000, -0.955423],
    [0.162460, 0.262866, -0.951056], [-0.442863, -0.238856, -0.864188],
    [-0.309017, -0.500000, -0.809017], [-0.162460, -0.262866, -0.951056],
    [0.000000, -0.850651, -0.525731], [-0.147621, -0.716567, -0.681718],
    [0.147621, -0.716567, -0.681718], [0.000000, -0.525731, -0.850651],
    [0.309017, -0.500000, -0.809017], [0.442863, -0.238856, -0.864188],
    [0.162460, -0.262866, -0.951056], [0.238856, -0.864188, -0.442863],
    [0.500000, -0.809017, -0.309017], [0.425325, -0.688191, -0.587785],
    [0.716567, -0.681718, -0.147621], [0.688191, -0.587785, -0.425325],
    [0.587785, -0.425325, -0.688191], [0.000000, -0.955423, -0.295242],
    [0.000000, -1.000000, 0.000000], [0.262866, -0.951056, -0.162460],
    [0.000000, -0.850651, 0.525731], [0.000000, -0.955423, 0.295242],
    [0.238856, -0.864188, 0.442863], [0.262866, -0.951056, 0.162460],
    [0.500000, -0.809017, 0.309017], [0.716567, -0.681718, 0.147621],
    [0.525731, -0.850651, 0.000000], [-0.238856, -0.864188, -0.442863],
    [-0.500000, -0.809017, -0.309017], [-0.262866, -0.951056, -0.162460],
    [-0.850651, -0.525731, 0.000000], [-0.716567, -0.681718, -0.147621],
    [-0.716567, -0.681718, 0.147621], [-0.525731, -0.850651, 0.000000],
    [-0.500000, -0.809017, 0.309017], [-0.238856, -0.864188, 0.442863],
    [-0.262866, -0.951056, 0.162460], [-0.864188, -0.442863, 0.238856],
    [-0.809017, -0.309017, 0.500000], [-0.688191, -0.587785, 0.425325],
    [-0.681718, -0.147621, 0.716567], [-0.442863, -0.238856, 0.864188],
    [-0.587785, -0.425325, 0.688191], [-0.309017, -0.500000, 0.809017],
    [-0.147621, -0.716567, 0.681718], [-0.425325, -0.688191, 0.587785],
    [-0.162460, -0.262866, 0.951056], [0.442863, -0.238856, 0.864188],
    [0.162460, -0.262866, 0.951056], [0.309017, -0.500000, 0.809017],
    [0.147621, -0.716567, 0.681718], [0.000000, -0.525731, 0.850651],
    [0.425325, -0.688191, 0.587785], [0.587785, -0.425325, 0.688191],
    [0.688191, -0.587785, 0.425325], [-0.955423, 0.295242, 0.000000],
    [-0.951056, 0.162460, 0.262866], [-1.000000, 0.000000, 0.000000],
    [-0.850651, 0.000000, 0.525731], [-0.955423, -0.295242, 0.000000],
    [-0.951056, -0.162460, 0.262866], [-0.864188, 0.442863, -0.238856],
    [-0.951056, 0.162460, -0.262866], [-0.809017, 0.309017, -0.500000],
    [-0.864188, -0.442863, -0.238856], [-0.951056, -0.162460, -0.262866],
    [-0.809017, -0.309017, -0.500000], [-0.681718, 0.147621, -0.716567],
    [-0.681718, -0.147621, -0.716567], [-0.850651, 0.000000, -0.525731],
    [-0.688191, 0.587785, -0.425325], [-0.587785, 0.425325, -0.688191],
    [-0.425325, 0.688191, -0.587785], [-0.425325, -0.688191, -0.587785],
    [-0.587785, -0.425325, -0.688191], [-0.688191, -0.587785, -0.425325],
];

// ===================================================================
// Helper macros (as inline fn) — these mirror q_shared.h #defines.
// ===================================================================

#[inline(always)]
fn dot_product(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline(always)]
fn vector_subtract(a: &[f32; 3], b: &[f32; 3], c: &mut [f32; 3]) {
    c[0] = a[0] - b[0];
    c[1] = a[1] - b[1];
    c[2] = a[2] - b[2];
}

#[inline(always)]
fn vector_copy(a: &[f32; 3], b: &mut [f32; 3]) {
    b[0] = a[0];
    b[1] = a[1];
    b[2] = a[2];
}

#[inline(always)]
fn vector_clear(a: &mut [f32; 3]) {
    a[0] = 0.0;
    a[1] = 0.0;
    a[2] = 0.0;
}

#[inline(always)]
fn vector_ma(v: &[f32; 3], s: f32, b: &[f32; 3], o: &mut [f32; 3]) {
    o[0] = v[0] + b[0] * s;
    o[1] = v[1] + b[1] * s;
    o[2] = v[2] + b[2] * s;
}

#[inline(always)]
fn cross_product(v1: &[f32; 3], v2: &[f32; 3], cross: &mut [f32; 3]) {
    cross[0] = v1[1] * v2[2] - v1[2] * v2[1];
    cross[1] = v1[2] * v2[0] - v1[0] * v2[2];
    cross[2] = v1[0] * v2[1] - v1[1] * v2[0];
}

#[inline(always)]
fn vector_length(v: &[f32; 3]) -> f32 {
    // sqrt is double-precision in C: (float)sqrt(double)
    ((v[0] * v[0] + v[1] * v[1] + v[2] * v[2]) as f64).sqrt() as f32
}

#[inline(always)]
fn deg2rad(a: f32) -> f32 {
    // (a * M_PI) / 180.0F
    a * M_PI / 180.0f32
}

// ===================================================================
// Q_rand / Q_random / Q_crandom
// ===================================================================

/// Return result of next pseudorandom step. Same as C: (69069*seed+1).
#[no_mangle]
pub unsafe extern "C" fn Q_rand(seed: *mut c_int) -> c_int {
    *seed = (69069i32).wrapping_mul(*seed).wrapping_add(1);
    *seed
}

#[no_mangle]
pub unsafe extern "C" fn Q_random(seed: *mut c_int) -> c_float {
    (Q_rand(seed) & 0xffff) as c_float / 0x10000 as c_float
}

#[no_mangle]
pub unsafe extern "C" fn Q_crandom(seed: *mut c_int) -> c_float {
    // 2.0 * (Q_random(seed) - 0.5) — C performs in double, returns float.
    (2.0f64 * (Q_random(seed) as f64 - 0.5f64)) as f32
}

// ===================================================================
// ClampChar / ClampShort
// ===================================================================

#[no_mangle]
pub extern "C" fn ClampChar(i: c_int) -> i8 {
    if i < -128 {
        return -128;
    }
    if i > 127 {
        return 127;
    }
    i as i8
}

#[no_mangle]
pub extern "C" fn ClampShort(i: c_int) -> i16 {
    if i < -32768 {
        return -32768;
    }
    if i > 0x7fff {
        return 0x7fff;
    }
    i as i16
}

// ===================================================================
// DirToByte / ByteToDir
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn DirToByte(dir: *const f32) -> c_int {
    if dir.is_null() {
        return 0;
    }
    let dir = std::slice::from_raw_parts(dir, 3);
    let dir_arr = [dir[0], dir[1], dir[2]];

    let mut bestd: f32 = 0.0;
    let mut best: c_int = 0;
    for i in 0..NUMVERTEXNORMALS {
        let bd = bytedirs[i];
        let d = dir_arr[0] * bd[0] + dir_arr[1] * bd[1] + dir_arr[2] * bd[2];
        if d > bestd {
            bestd = d;
            best = i as c_int;
        }
    }
    best
}

#[no_mangle]
pub unsafe extern "C" fn ByteToDir(b: c_int, dir: *mut f32) {
    let dir_slice = std::slice::from_raw_parts_mut(dir, 3);
    if b < 0 || (b as usize) >= NUMVERTEXNORMALS {
        dir_slice[0] = vec3_origin[0];
        dir_slice[1] = vec3_origin[1];
        dir_slice[2] = vec3_origin[2];
        return;
    }
    let bd = bytedirs[b as usize];
    dir_slice[0] = bd[0];
    dir_slice[1] = bd[1];
    dir_slice[2] = bd[2];
}

// ===================================================================
// ColorBytes3 / ColorBytes4
// ===================================================================

/// Convert a float color in [0,1] to packed unsigned bytes, matching gcc/x86
/// semantics for `(byte)(float * 255)`.
#[inline(always)]
fn color_byte(f: f32) -> u8 {
    // gcc on x86_64 uses cvttss2si then truncation. For values in [0, 255]
    // this matches `as i32 as u8`. Out of range is technically undefined in
    // C; we match the typical gcc behaviour for in-range floats.
    let v = f * 255.0;
    // Avoid Rust's float-to-uint saturation by going through i32. cvttss2si
    // produces int with truncation toward zero; values in (-2^31, 2^31)
    // convert losslessly. Then `as u8` truncates the low 8 bits.
    (v as i32) as u8
}

#[no_mangle]
pub extern "C" fn ColorBytes3(r: c_float, g: c_float, b: c_float) -> c_uint {
    let mut bytes = [0u8; 4];
    bytes[0] = color_byte(r);
    bytes[1] = color_byte(g);
    bytes[2] = color_byte(b);
    // bytes[3] is left as 0 — C `unsigned i;` is uninitialized but the C
    // code only writes 3 bytes. In practice the high byte is whatever the
    // stack happened to contain. We cannot match that exactly so we leave 0.
    // The original C also exhibits this UB.
    u32::from_le_bytes(bytes)
}

#[no_mangle]
pub extern "C" fn ColorBytes4(r: c_float, g: c_float, b: c_float, a: c_float) -> c_uint {
    let mut bytes = [0u8; 4];
    bytes[0] = color_byte(r);
    bytes[1] = color_byte(g);
    bytes[2] = color_byte(b);
    bytes[3] = color_byte(a);
    u32::from_le_bytes(bytes)
}

// ===================================================================
// NormalizeColor
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn NormalizeColor(input: *const f32, out: *mut f32) -> c_float {
    let i = std::slice::from_raw_parts(input, 3);
    let o = std::slice::from_raw_parts_mut(out, 3);

    let mut max = i[0];
    if i[1] > max {
        max = i[1];
    }
    if i[2] > max {
        max = i[2];
    }

    if max == 0.0 {
        o[0] = 0.0;
        o[1] = 0.0;
        o[2] = 0.0;
    } else {
        o[0] = i[0] / max;
        o[1] = i[1] / max;
        o[2] = i[2] / max;
    }
    max
}

// ===================================================================
// PlaneFromPoints
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn PlaneFromPoints(
    plane: *mut f32,
    a: *const f32,
    b: *const f32,
    c: *const f32,
) -> c_int {
    let plane_slice = std::slice::from_raw_parts_mut(plane, 4);
    let a_arr = [*a.add(0), *a.add(1), *a.add(2)];
    let b_arr = [*b.add(0), *b.add(1), *b.add(2)];
    let c_arr = [*c.add(0), *c.add(1), *c.add(2)];

    let mut d1 = [0f32; 3];
    let mut d2 = [0f32; 3];
    vector_subtract(&b_arr, &a_arr, &mut d1);
    vector_subtract(&c_arr, &a_arr, &mut d2);

    let mut normal = [0f32; 3];
    cross_product(&d2, &d1, &mut normal);

    // VectorNormalize on `normal`
    let length_sq = normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2];
    let length = (length_sq as f64).sqrt() as f32;
    if length != 0.0 {
        let ilength = 1.0 / length;
        normal[0] *= ilength;
        normal[1] *= ilength;
        normal[2] *= ilength;
    }
    plane_slice[0] = normal[0];
    plane_slice[1] = normal[1];
    plane_slice[2] = normal[2];

    if length == 0.0 {
        return 0;
    }
    plane_slice[3] = a_arr[0] * normal[0] + a_arr[1] * normal[1] + a_arr[2] * normal[2];
    1
}

// ===================================================================
// MatrixMultiply
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn MatrixMultiply(
    in1: *const [f32; 3],
    in2: *const [f32; 3],
    out: *mut [f32; 3],
) {
    let a = std::slice::from_raw_parts(in1, 3);
    let b = std::slice::from_raw_parts(in2, 3);
    let o = std::slice::from_raw_parts_mut(out, 3);

    o[0][0] = a[0][0] * b[0][0] + a[0][1] * b[1][0] + a[0][2] * b[2][0];
    o[0][1] = a[0][0] * b[0][1] + a[0][1] * b[1][1] + a[0][2] * b[2][1];
    o[0][2] = a[0][0] * b[0][2] + a[0][1] * b[1][2] + a[0][2] * b[2][2];
    o[1][0] = a[1][0] * b[0][0] + a[1][1] * b[1][0] + a[1][2] * b[2][0];
    o[1][1] = a[1][0] * b[0][1] + a[1][1] * b[1][1] + a[1][2] * b[2][1];
    o[1][2] = a[1][0] * b[0][2] + a[1][1] * b[1][2] + a[1][2] * b[2][2];
    o[2][0] = a[2][0] * b[0][0] + a[2][1] * b[1][0] + a[2][2] * b[2][0];
    o[2][1] = a[2][0] * b[0][1] + a[2][1] * b[1][1] + a[2][2] * b[2][1];
    o[2][2] = a[2][0] * b[0][2] + a[2][1] * b[1][2] + a[2][2] * b[2][2];
}

// ===================================================================
// PerpendicularVector
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn PerpendicularVector(dst: *mut f32, src: *const f32) {
    let s = std::slice::from_raw_parts(src, 3);
    let s_arr = [s[0], s[1], s[2]];

    let mut pos: usize = 0;
    let mut minelem: f32 = 1.0;
    for i in 0..3usize {
        let f = s_arr[i].abs();
        if f < minelem {
            pos = i;
            minelem = f;
        }
    }
    let mut tempvec = [0f32; 3];
    tempvec[pos] = 1.0;

    // Project tempvec onto plane defined by src
    let inv_denom = 1.0f32 / (s_arr[0] * s_arr[0] + s_arr[1] * s_arr[1] + s_arr[2] * s_arr[2]);
    let d = (s_arr[0] * tempvec[0] + s_arr[1] * tempvec[1] + s_arr[2] * tempvec[2]) * inv_denom;
    let n0 = s_arr[0] * inv_denom;
    let n1 = s_arr[1] * inv_denom;
    let n2 = s_arr[2] * inv_denom;
    let mut result = [
        tempvec[0] - d * n0,
        tempvec[1] - d * n1,
        tempvec[2] - d * n2,
    ];
    // VectorNormalize
    let length_sq = result[0] * result[0] + result[1] * result[1] + result[2] * result[2];
    let length = (length_sq as f64).sqrt() as f32;
    if length != 0.0 {
        let ilength = 1.0 / length;
        result[0] *= ilength;
        result[1] *= ilength;
        result[2] *= ilength;
    }
    let dst = std::slice::from_raw_parts_mut(dst, 3);
    dst[0] = result[0];
    dst[1] = result[1];
    dst[2] = result[2];
}

// ===================================================================
// ProjectPointOnPlane
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn ProjectPointOnPlane(dst: *mut f32, p: *const f32, normal: *const f32) {
    let p_slice = std::slice::from_raw_parts(p, 3);
    let n_slice = std::slice::from_raw_parts(normal, 3);
    let dst_slice = std::slice::from_raw_parts_mut(dst, 3);

    let inv_denom = 1.0f32 / (n_slice[0] * n_slice[0] + n_slice[1] * n_slice[1] + n_slice[2] * n_slice[2]);
    let d = (n_slice[0] * p_slice[0] + n_slice[1] * p_slice[1] + n_slice[2] * p_slice[2]) * inv_denom;
    let n0 = n_slice[0] * inv_denom;
    let n1 = n_slice[1] * inv_denom;
    let n2 = n_slice[2] * inv_denom;
    dst_slice[0] = p_slice[0] - d * n0;
    dst_slice[1] = p_slice[1] - d * n1;
    dst_slice[2] = p_slice[2] - d * n2;
}

// ===================================================================
// MakeNormalVectors
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn MakeNormalVectors(forward: *const f32, right: *mut f32, up: *mut f32) {
    let f = std::slice::from_raw_parts(forward, 3);
    let f_arr = [f[0], f[1], f[2]];

    let r_slice = std::slice::from_raw_parts_mut(right, 3);
    r_slice[1] = -f_arr[0];
    r_slice[2] = f_arr[1];
    r_slice[0] = f_arr[2];

    let mut r_arr = [r_slice[0], r_slice[1], r_slice[2]];
    let d = r_arr[0] * f_arr[0] + r_arr[1] * f_arr[1] + r_arr[2] * f_arr[2];
    // VectorMA(right, -d, forward, right) -> right[i] = right[i] + forward[i] * (-d)
    r_arr[0] = r_arr[0] + f_arr[0] * (-d);
    r_arr[1] = r_arr[1] + f_arr[1] * (-d);
    r_arr[2] = r_arr[2] + f_arr[2] * (-d);

    // VectorNormalize(right)
    let length_sq = r_arr[0] * r_arr[0] + r_arr[1] * r_arr[1] + r_arr[2] * r_arr[2];
    let length = (length_sq as f64).sqrt() as f32;
    if length != 0.0 {
        let ilength = 1.0 / length;
        r_arr[0] *= ilength;
        r_arr[1] *= ilength;
        r_arr[2] *= ilength;
    }
    r_slice[0] = r_arr[0];
    r_slice[1] = r_arr[1];
    r_slice[2] = r_arr[2];

    // CrossProduct(right, forward, up)
    let up_slice = std::slice::from_raw_parts_mut(up, 3);
    up_slice[0] = r_arr[1] * f_arr[2] - r_arr[2] * f_arr[1];
    up_slice[1] = r_arr[2] * f_arr[0] - r_arr[0] * f_arr[2];
    up_slice[2] = r_arr[0] * f_arr[1] - r_arr[1] * f_arr[0];
}

// ===================================================================
// VectorRotate
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn VectorRotate(
    inp: *const f32,
    matrix: *const [f32; 3],
    out: *mut f32,
) {
    let i = std::slice::from_raw_parts(inp, 3);
    let m = std::slice::from_raw_parts(matrix, 3);
    let o = std::slice::from_raw_parts_mut(out, 3);
    o[0] = i[0] * m[0][0] + i[1] * m[0][1] + i[2] * m[0][2];
    o[1] = i[0] * m[1][0] + i[1] * m[1][1] + i[2] * m[1][2];
    o[2] = i[0] * m[2][0] + i[1] * m[2][1] + i[2] * m[2][2];
}

// ===================================================================
// Q_rsqrt / Q_fabs
// ===================================================================

#[no_mangle]
pub extern "C" fn Q_rsqrt(number: c_float) -> c_float {
    let threehalfs: f32 = 1.5;
    let x2 = number * 0.5f32;
    let mut y = number;

    let mut i: u32 = y.to_bits();
    i = 0x5f3759dfu32.wrapping_sub(i >> 1);
    y = f32::from_bits(i);

    y = y * (threehalfs - (x2 * y * y));
    y
}

#[no_mangle]
pub extern "C" fn Q_fabs(f: c_float) -> c_float {
    let bits = f.to_bits();
    f32::from_bits(bits & 0x7FFF_FFFF)
}

// ===================================================================
// LerpAngle / AngleSubtract / AnglesSubtract
// ===================================================================

#[no_mangle]
pub extern "C" fn LerpAngle(from: c_float, to: c_float, frac: c_float) -> c_float {
    let mut to = to;
    if to - from > 180.0 {
        to -= 360.0;
    }
    if to - from < -180.0 {
        to += 360.0;
    }
    from + frac * (to - from)
}

#[no_mangle]
pub extern "C" fn AngleSubtract(a1: c_float, a2: c_float) -> c_float {
    let mut a = a1 - a2;
    while a > 180.0 {
        a -= 360.0;
    }
    while a < -180.0 {
        a += 360.0;
    }
    a
}

#[no_mangle]
pub unsafe extern "C" fn AnglesSubtract(v1: *const f32, v2: *const f32, v3: *mut f32) {
    let v3 = std::slice::from_raw_parts_mut(v3, 3);
    v3[0] = AngleSubtract(*v1.add(0), *v2.add(0));
    v3[1] = AngleSubtract(*v1.add(1), *v2.add(1));
    v3[2] = AngleSubtract(*v1.add(2), *v2.add(2));
}

#[no_mangle]
pub extern "C" fn AngleMod(a: c_float) -> c_float {
    // a = (360.0/65536) * ((int)(a*(65536/360.0)) & 65535);
    // Note: in C, 360.0 and 65536/360.0 are double — operations promoted.
    let v = (a as f64) * (65536.0f64 / 360.0f64);
    let masked = (v as i32) & 65535;
    ((360.0f64 / 65536.0f64) * masked as f64) as f32
}

#[no_mangle]
pub extern "C" fn AngleNormalize360(angle: c_float) -> c_float {
    let v = (angle as f64) * (65536.0f64 / 360.0f64);
    let masked = (v as i32) & 65535;
    ((360.0f64 / 65536.0f64) * masked as f64) as f32
}

#[no_mangle]
pub extern "C" fn AngleNormalize180(angle: c_float) -> c_float {
    let mut a = AngleNormalize360(angle);
    if a > 180.0 {
        a -= 360.0;
    }
    a
}

#[no_mangle]
pub extern "C" fn AngleDelta(angle1: c_float, angle2: c_float) -> c_float {
    AngleNormalize180(angle1 - angle2)
}

// ===================================================================
// SetPlaneSignbits / BoxOnPlaneSide
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn SetPlaneSignbits(out: *mut cplane_t) {
    let o = &mut *out;
    let mut bits: u8 = 0;
    for j in 0..3usize {
        if o.normal[j] < 0.0 {
            bits |= 1 << j;
        }
    }
    o.signbits = bits;
}

#[no_mangle]
pub unsafe extern "C" fn BoxOnPlaneSide(
    emins: *const f32,
    emaxs: *const f32,
    p: *const cplane_t,
) -> c_int {
    let emins = std::slice::from_raw_parts(emins, 3);
    let emaxs = std::slice::from_raw_parts(emaxs, 3);
    let p = &*p;

    if p.r#type < 3 {
        let t = p.r#type as usize;
        if p.dist <= emins[t] {
            return 1;
        }
        if p.dist >= emaxs[t] {
            return 2;
        }
        return 3;
    }

    let n = &p.normal;
    let (dist1, dist2) = match p.signbits {
        0 => (
            n[0] * emaxs[0] + n[1] * emaxs[1] + n[2] * emaxs[2],
            n[0] * emins[0] + n[1] * emins[1] + n[2] * emins[2],
        ),
        1 => (
            n[0] * emins[0] + n[1] * emaxs[1] + n[2] * emaxs[2],
            n[0] * emaxs[0] + n[1] * emins[1] + n[2] * emins[2],
        ),
        2 => (
            n[0] * emaxs[0] + n[1] * emins[1] + n[2] * emaxs[2],
            n[0] * emins[0] + n[1] * emaxs[1] + n[2] * emins[2],
        ),
        3 => (
            n[0] * emins[0] + n[1] * emins[1] + n[2] * emaxs[2],
            n[0] * emaxs[0] + n[1] * emaxs[1] + n[2] * emins[2],
        ),
        4 => (
            n[0] * emaxs[0] + n[1] * emaxs[1] + n[2] * emins[2],
            n[0] * emins[0] + n[1] * emins[1] + n[2] * emaxs[2],
        ),
        5 => (
            n[0] * emins[0] + n[1] * emaxs[1] + n[2] * emins[2],
            n[0] * emaxs[0] + n[1] * emins[1] + n[2] * emaxs[2],
        ),
        6 => (
            n[0] * emaxs[0] + n[1] * emins[1] + n[2] * emins[2],
            n[0] * emins[0] + n[1] * emaxs[1] + n[2] * emaxs[2],
        ),
        7 => (
            n[0] * emins[0] + n[1] * emins[1] + n[2] * emins[2],
            n[0] * emaxs[0] + n[1] * emaxs[1] + n[2] * emaxs[2],
        ),
        _ => (0.0, 0.0),
    };

    let mut sides: c_int = 0;
    if dist1 >= p.dist {
        sides = 1;
    }
    if dist2 < p.dist {
        sides |= 2;
    }
    sides
}

// ===================================================================
// RadiusFromBounds / ClearBounds / AddPointToBounds
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn RadiusFromBounds(mins: *const f32, maxs: *const f32) -> c_float {
    let mins = std::slice::from_raw_parts(mins, 3);
    let maxs = std::slice::from_raw_parts(maxs, 3);
    let mut corner = [0f32; 3];
    for i in 0..3usize {
        // C uses fabs() (double). Promote then truncate back to f32.
        let a = (mins[i] as f64).abs() as f32;
        let b = (maxs[i] as f64).abs() as f32;
        corner[i] = if a > b { a } else { b };
    }
    vector_length(&corner)
}

#[no_mangle]
pub unsafe extern "C" fn ClearBounds(mins: *mut f32, maxs: *mut f32) {
    let mins = std::slice::from_raw_parts_mut(mins, 3);
    let maxs = std::slice::from_raw_parts_mut(maxs, 3);
    mins[0] = 99999.0;
    mins[1] = 99999.0;
    mins[2] = 99999.0;
    maxs[0] = -99999.0;
    maxs[1] = -99999.0;
    maxs[2] = -99999.0;
}

#[no_mangle]
pub unsafe extern "C" fn AddPointToBounds(v: *const f32, mins: *mut f32, maxs: *mut f32) {
    let v = std::slice::from_raw_parts(v, 3);
    let mins = std::slice::from_raw_parts_mut(mins, 3);
    let maxs = std::slice::from_raw_parts_mut(maxs, 3);
    if v[0] < mins[0] { mins[0] = v[0]; }
    if v[0] > maxs[0] { maxs[0] = v[0]; }
    if v[1] < mins[1] { mins[1] = v[1]; }
    if v[1] > maxs[1] { maxs[1] = v[1]; }
    if v[2] < mins[2] { mins[2] = v[2]; }
    if v[2] > maxs[2] { maxs[2] = v[2]; }
}

// ===================================================================
// VectorNormalize / VectorNormalize2
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn VectorNormalize(v: *mut f32) -> c_float {
    let v = std::slice::from_raw_parts_mut(v, 3);
    let length_sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    let length = (length_sq as f64).sqrt() as f32;

    if length != 0.0 {
        let ilength = 1.0 / length;
        v[0] *= ilength;
        v[1] *= ilength;
        v[2] *= ilength;
    }
    length
}

#[no_mangle]
pub unsafe extern "C" fn VectorNormalize2(v: *const f32, out: *mut f32) -> c_float {
    let v = std::slice::from_raw_parts(v, 3);
    let out = std::slice::from_raw_parts_mut(out, 3);
    let length_sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    let length = (length_sq as f64).sqrt() as f32;

    if length != 0.0 {
        let ilength = 1.0 / length;
        out[0] = v[0] * ilength;
        out[1] = v[1] * ilength;
        out[2] = v[2] * ilength;
    } else {
        out[0] = 0.0;
        out[1] = 0.0;
        out[2] = 0.0;
    }
    length
}

// ===================================================================
// _VectorMA / _DotProduct / _VectorSubtract / _VectorAdd / _VectorCopy /
// _VectorScale / Vector4Scale
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn _VectorMA(veca: *const f32, scale: c_float, vecb: *const f32, vecc: *mut f32) {
    let a = std::slice::from_raw_parts(veca, 3);
    let b = std::slice::from_raw_parts(vecb, 3);
    let c = std::slice::from_raw_parts_mut(vecc, 3);
    c[0] = a[0] + scale * b[0];
    c[1] = a[1] + scale * b[1];
    c[2] = a[2] + scale * b[2];
}

#[no_mangle]
pub unsafe extern "C" fn _DotProduct(v1: *const f32, v2: *const f32) -> c_float {
    let v1 = std::slice::from_raw_parts(v1, 3);
    let v2 = std::slice::from_raw_parts(v2, 3);
    v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2]
}

#[no_mangle]
pub unsafe extern "C" fn _VectorSubtract(veca: *const f32, vecb: *const f32, out: *mut f32) {
    let a = std::slice::from_raw_parts(veca, 3);
    let b = std::slice::from_raw_parts(vecb, 3);
    let o = std::slice::from_raw_parts_mut(out, 3);
    o[0] = a[0] - b[0];
    o[1] = a[1] - b[1];
    o[2] = a[2] - b[2];
}

#[no_mangle]
pub unsafe extern "C" fn _VectorAdd(veca: *const f32, vecb: *const f32, out: *mut f32) {
    let a = std::slice::from_raw_parts(veca, 3);
    let b = std::slice::from_raw_parts(vecb, 3);
    let o = std::slice::from_raw_parts_mut(out, 3);
    o[0] = a[0] + b[0];
    o[1] = a[1] + b[1];
    o[2] = a[2] + b[2];
}

#[no_mangle]
pub unsafe extern "C" fn _VectorCopy(input: *const f32, out: *mut f32) {
    let i = std::slice::from_raw_parts(input, 3);
    let o = std::slice::from_raw_parts_mut(out, 3);
    o[0] = i[0];
    o[1] = i[1];
    o[2] = i[2];
}

#[no_mangle]
pub unsafe extern "C" fn _VectorScale(input: *const f32, scale: c_float, out: *mut f32) {
    let i = std::slice::from_raw_parts(input, 3);
    let o = std::slice::from_raw_parts_mut(out, 3);
    o[0] = i[0] * scale;
    o[1] = i[1] * scale;
    o[2] = i[2] * scale;
}

#[no_mangle]
pub unsafe extern "C" fn Vector4Scale(input: *const f32, scale: c_float, out: *mut f32) {
    let i = std::slice::from_raw_parts(input, 4);
    let o = std::slice::from_raw_parts_mut(out, 4);
    o[0] = i[0] * scale;
    o[1] = i[1] * scale;
    o[2] = i[2] * scale;
    o[3] = i[3] * scale;
}

// ===================================================================
// Q_log2
// ===================================================================

#[no_mangle]
pub extern "C" fn Q_log2(val: c_int) -> c_int {
    let mut val = val;
    let mut answer = 0;
    loop {
        // signed arithmetic shift (matches gcc on x86)
        val >>= 1;
        if val == 0 {
            break;
        }
        answer += 1;
    }
    answer
}

// ===================================================================
// vectoangles
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn vectoangles(value1: *const f32, angles: *mut f32) {
    let v = std::slice::from_raw_parts(value1, 3);
    let a = std::slice::from_raw_parts_mut(angles, 3);

    let yaw: f32;
    let mut pitch: f32;

    if v[1] == 0.0 && v[0] == 0.0 {
        yaw = 0.0;
        if v[2] > 0.0 {
            pitch = 90.0;
        } else {
            pitch = 270.0;
        }
    } else {
        let mut y: f32;
        if v[0] != 0.0 {
            // atan2(v[1], v[0]) * 180 / M_PI in double (M_PI is float, promoted)
            let tmp = (v[1] as f64).atan2(v[0] as f64) * 180.0f64 / (M_PI as f64);
            y = tmp as f32;
        } else if v[1] > 0.0 {
            y = 90.0;
        } else {
            y = 270.0;
        }
        if y < 0.0 {
            y += 360.0;
        }
        yaw = y;

        // forward = sqrt(v[0]^2 + v[1]^2) — sqrt is double in C.
        let forward = ((v[0] * v[0] + v[1] * v[1]) as f64).sqrt() as f32;
        let p_double = (v[2] as f64).atan2(forward as f64) * 180.0f64 / (M_PI as f64);
        pitch = p_double as f32;
        if pitch < 0.0 {
            pitch += 360.0;
        }
    }

    a[PITCH] = -pitch;
    a[YAW] = yaw;
    a[ROLL] = 0.0;
}

// ===================================================================
// AngleVectors
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn AngleVectors(
    angles: *const f32,
    forward: *mut f32,
    right: *mut f32,
    up: *mut f32,
) {
    let a = std::slice::from_raw_parts(angles, 3);

    // C uses sin/cos (double) on float promoted to double, then assigns
    // back to float. Replicate by converting through f64.
    let angle_yaw = (a[YAW] as f64) * (std::f64::consts::PI * 2.0 / 360.0);
    let sy = angle_yaw.sin() as f32;
    let cy = angle_yaw.cos() as f32;

    let angle_pitch = (a[PITCH] as f64) * (std::f64::consts::PI * 2.0 / 360.0);
    let sp = angle_pitch.sin() as f32;
    let cp = angle_pitch.cos() as f32;

    let angle_roll = (a[ROLL] as f64) * (std::f64::consts::PI * 2.0 / 360.0);
    let sr = angle_roll.sin() as f32;
    let cr = angle_roll.cos() as f32;

    if !forward.is_null() {
        let f = std::slice::from_raw_parts_mut(forward, 3);
        f[0] = cp * cy;
        f[1] = cp * sy;
        f[2] = -sp;
    }
    if !right.is_null() {
        let r = std::slice::from_raw_parts_mut(right, 3);
        r[0] = -1.0 * sr * sp * cy + -1.0 * cr * -sy;
        r[1] = -1.0 * sr * sp * sy + -1.0 * cr * cy;
        r[2] = -1.0 * sr * cp;
    }
    if !up.is_null() {
        let u = std::slice::from_raw_parts_mut(up, 3);
        u[0] = cr * sp * cy + -sr * -sy;
        u[1] = cr * sp * sy + -sr * cy;
        u[2] = cr * cp;
    }
}

// ===================================================================
// AngleVectors helper: M_PI*2/360 in C is computed as float because M_PI is
// a float literal. To match gcc's behaviour exactly we should compute the
// constant the same way. We stay in f64 for `sin`/`cos` arguments because
// libm's sin/cos are double precision.
// ===================================================================

// ===================================================================
// AnglesToAxis / AxisClear / AxisCopy
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn AnglesToAxis(angles: *const f32, axis: *mut [f32; 3]) {
    let mut right = [0f32; 3];
    let axis_slice = std::slice::from_raw_parts_mut(axis, 3);
    AngleVectors(
        angles,
        axis_slice[0].as_mut_ptr(),
        right.as_mut_ptr(),
        axis_slice[2].as_mut_ptr(),
    );
    axis_slice[1][0] = vec3_origin[0] - right[0];
    axis_slice[1][1] = vec3_origin[1] - right[1];
    axis_slice[1][2] = vec3_origin[2] - right[2];
}

#[no_mangle]
pub unsafe extern "C" fn AxisClear(axis: *mut [f32; 3]) {
    let a = std::slice::from_raw_parts_mut(axis, 3);
    a[0][0] = 1.0; a[0][1] = 0.0; a[0][2] = 0.0;
    a[1][0] = 0.0; a[1][1] = 1.0; a[1][2] = 0.0;
    a[2][0] = 0.0; a[2][1] = 0.0; a[2][2] = 1.0;
}

#[no_mangle]
pub unsafe extern "C" fn AxisCopy(input: *const [f32; 3], out: *mut [f32; 3]) {
    let i = std::slice::from_raw_parts(input, 3);
    let o = std::slice::from_raw_parts_mut(out, 3);
    o[0] = i[0];
    o[1] = i[1];
    o[2] = i[2];
}

// ===================================================================
// RotatePointAroundVector / RotateAroundDirection
// ===================================================================

#[no_mangle]
pub unsafe extern "C" fn RotatePointAroundVector(
    dst: *mut f32,
    dir: *const f32,
    point: *const f32,
    degrees: c_float,
) {
    let dir_arr = [*dir.add(0), *dir.add(1), *dir.add(2)];
    let p = [*point.add(0), *point.add(1), *point.add(2)];

    let mut vr = [0f32; 3];
    PerpendicularVector(vr.as_mut_ptr(), dir_arr.as_ptr());

    let mut vup = [0f32; 3];
    cross_product(&vr, &dir_arr, &mut vup);

    let vf = dir_arr;

    let mut m = [[0f32; 3]; 3];
    m[0][0] = vr[0]; m[1][0] = vr[1]; m[2][0] = vr[2];
    m[0][1] = vup[0]; m[1][1] = vup[1]; m[2][1] = vup[2];
    m[0][2] = vf[0]; m[1][2] = vf[1]; m[2][2] = vf[2];

    // memcpy(im, m) then transpose-ish:
    let mut im = m;
    im[0][1] = m[1][0];
    im[0][2] = m[2][0];
    im[1][0] = m[0][1];
    im[1][2] = m[2][1];
    im[2][0] = m[0][2];
    im[2][1] = m[1][2];

    let mut zrot = [[0f32; 3]; 3];
    zrot[0][0] = 1.0; zrot[1][1] = 1.0; zrot[2][2] = 1.0;

    // DEG2RAD: ((a) * M_PI) / 180.0F — float arithmetic
    let rad: f32 = degrees * M_PI / 180.0f32;
    // C uses cos(double)/sin(double). Promote to double, then truncate.
    zrot[0][0] = (rad as f64).cos() as f32;
    zrot[0][1] = (rad as f64).sin() as f32;
    zrot[1][0] = -((rad as f64).sin() as f32);
    zrot[1][1] = (rad as f64).cos() as f32;

    let mut tmpmat = [[0f32; 3]; 3];
    matrix_mul(&m, &zrot, &mut tmpmat);
    let mut rot = [[0f32; 3]; 3];
    matrix_mul(&tmpmat, &im, &mut rot);

    let dst_slice = std::slice::from_raw_parts_mut(dst, 3);
    for i in 0..3usize {
        dst_slice[i] = rot[i][0] * p[0] + rot[i][1] * p[1] + rot[i][2] * p[2];
    }
}

fn matrix_mul(in1: &[[f32; 3]; 3], in2: &[[f32; 3]; 3], out: &mut [[f32; 3]; 3]) {
    out[0][0] = in1[0][0] * in2[0][0] + in1[0][1] * in2[1][0] + in1[0][2] * in2[2][0];
    out[0][1] = in1[0][0] * in2[0][1] + in1[0][1] * in2[1][1] + in1[0][2] * in2[2][1];
    out[0][2] = in1[0][0] * in2[0][2] + in1[0][1] * in2[1][2] + in1[0][2] * in2[2][2];
    out[1][0] = in1[1][0] * in2[0][0] + in1[1][1] * in2[1][0] + in1[1][2] * in2[2][0];
    out[1][1] = in1[1][0] * in2[0][1] + in1[1][1] * in2[1][1] + in1[1][2] * in2[2][1];
    out[1][2] = in1[1][0] * in2[0][2] + in1[1][1] * in2[1][2] + in1[1][2] * in2[2][2];
    out[2][0] = in1[2][0] * in2[0][0] + in1[2][1] * in2[1][0] + in1[2][2] * in2[2][0];
    out[2][1] = in1[2][0] * in2[0][1] + in1[2][1] * in2[1][1] + in1[2][2] * in2[2][1];
    out[2][2] = in1[2][0] * in2[0][2] + in1[2][1] * in2[1][2] + in1[2][2] * in2[2][2];
}

#[no_mangle]
pub unsafe extern "C" fn RotateAroundDirection(axis: *mut [f32; 3], yaw: c_float) {
    let axis_slice = std::slice::from_raw_parts_mut(axis, 3);
    PerpendicularVector(axis_slice[1].as_mut_ptr(), axis_slice[0].as_ptr());

    if yaw != 0.0 {
        let temp = axis_slice[1];
        let axis0 = axis_slice[0];
        RotatePointAroundVector(
            axis_slice[1].as_mut_ptr(),
            axis0.as_ptr(),
            temp.as_ptr(),
            yaw,
        );
    }
    // CrossProduct(axis[0], axis[1], axis[2])
    let a0 = axis_slice[0];
    let a1 = axis_slice[1];
    axis_slice[2][0] = a0[1] * a1[2] - a0[2] * a1[1];
    axis_slice[2][1] = a0[2] * a1[0] - a0[0] * a1[2];
    axis_slice[2][2] = a0[0] * a1[1] - a0[1] * a1[0];
}

// Re-export so main.rs can call our internal helpers.
#[doc(hidden)]
pub fn _vector_length_pub(v: &[f32; 3]) -> f32 {
    vector_length(v)
}
