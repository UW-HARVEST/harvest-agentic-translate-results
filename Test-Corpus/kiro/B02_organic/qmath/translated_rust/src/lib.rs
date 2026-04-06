#![allow(non_snake_case, non_upper_case_globals)]
use std::f64::consts::PI;

pub type vec_t = f32;
pub type vec3_t = [vec_t; 3];
pub type vec4_t = [vec_t; 4];
pub type qboolean = i32;

const NUMVERTEXNORMALS: usize = 162;
const PITCH: usize = 0;
const YAW: usize = 1;
const ROLL: usize = 2;

#[repr(C)]
pub struct cplane_t {
    pub normal: vec3_t,
    pub dist: f32,
    pub type_: u8,
    pub signbits: u8,
    pub pad: [u8; 2],
}

#[no_mangle]
pub static mut vec3_origin: vec3_t = [0.0, 0.0, 0.0];

#[no_mangle]
pub static mut axisDefault: [vec3_t; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

#[no_mangle]
pub static mut colorBlack: vec4_t = [0.0, 0.0, 0.0, 1.0];
#[no_mangle]
pub static mut colorRed: vec4_t = [1.0, 0.0, 0.0, 1.0];
#[no_mangle]
pub static mut colorGreen: vec4_t = [0.0, 1.0, 0.0, 1.0];
#[no_mangle]
pub static mut colorBlue: vec4_t = [0.0, 0.0, 1.0, 1.0];
#[no_mangle]
pub static mut colorYellow: vec4_t = [1.0, 1.0, 0.0, 1.0];
#[no_mangle]
pub static mut colorMagenta: vec4_t = [1.0, 0.0, 1.0, 1.0];
#[no_mangle]
pub static mut colorCyan: vec4_t = [0.0, 1.0, 1.0, 1.0];
#[no_mangle]
pub static mut colorWhite: vec4_t = [1.0, 1.0, 1.0, 1.0];
#[no_mangle]
pub static mut colorLtGrey: vec4_t = [0.75, 0.75, 0.75, 1.0];
#[no_mangle]
pub static mut colorMdGrey: vec4_t = [0.5, 0.5, 0.5, 1.0];
#[no_mangle]
pub static mut colorDkGrey: vec4_t = [0.25, 0.25, 0.25, 1.0];

#[no_mangle]
pub static mut g_color_table: [vec4_t; 8] = [
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
pub static bytedirs: [vec3_t; NUMVERTEXNORMALS] = [
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

// Helper macros as inline functions
#[inline]
fn dot_product(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn vector_copy(a: &[f32; 3], b: &mut [f32; 3]) {
    b[0] = a[0]; b[1] = a[1]; b[2] = a[2];
}

#[inline]
fn vector_subtract(a: &[f32; 3], b: &[f32; 3], c: &mut [f32; 3]) {
    c[0] = a[0] - b[0]; c[1] = a[1] - b[1]; c[2] = a[2] - b[2];
}

#[inline]
fn vector_add(a: &[f32; 3], b: &[f32; 3], c: &mut [f32; 3]) {
    c[0] = a[0] + b[0]; c[1] = a[1] + b[1]; c[2] = a[2] + b[2];
}

#[inline]
fn vector_scale(v: &[f32; 3], s: f32, o: &mut [f32; 3]) {
    o[0] = v[0] * s; o[1] = v[1] * s; o[2] = v[2] * s;
}

#[inline]
fn vector_ma(v: &[f32; 3], s: f32, b: &[f32; 3], o: &mut [f32; 3]) {
    o[0] = v[0] + b[0] * s; o[1] = v[1] + b[1] * s; o[2] = v[2] + b[2] * s;
}

#[inline]
fn vector_clear(a: &mut [f32; 3]) {
    a[0] = 0.0; a[1] = 0.0; a[2] = 0.0;
}

#[inline]
fn cross_product(v1: &[f32; 3], v2: &[f32; 3], cross: &mut [f32; 3]) {
    cross[0] = v1[1] * v2[2] - v1[2] * v2[1];
    cross[1] = v1[2] * v2[0] - v1[0] * v2[2];
    cross[2] = v1[0] * v2[1] - v1[1] * v2[0];
}

#[inline]
fn vector_length(v: &[f32; 3]) -> f32 {
    ((v[0] * v[0] + v[1] * v[1] + v[2] * v[2]) as f64).sqrt() as f32
}

// ============================================================
// Exported functions matching C API
// ============================================================

#[no_mangle]
pub extern "C" fn Q_rand(seed: &mut i32) -> i32 {
    *seed = (69069i32).wrapping_mul(*seed).wrapping_add(1);
    *seed
}

#[no_mangle]
pub extern "C" fn Q_random(seed: &mut i32) -> f32 {
    (Q_rand(seed) & 0xffff) as f32 / 0x10000 as f32
}

#[no_mangle]
pub extern "C" fn Q_crandom(seed: &mut i32) -> f32 {
    2.0 * (Q_random(seed) - 0.5)
}

#[no_mangle]
pub extern "C" fn ClampChar(i: i32) -> i8 {
    if i < -128 { return -128; }
    if i > 127 { return 127; }
    i as i8
}

#[no_mangle]
pub extern "C" fn ClampShort(i: i32) -> i16 {
    if i < -32768 { return -32768; }
    if i > 0x7fff { return 0x7fff; }
    i as i16
}

#[no_mangle]
pub extern "C" fn DirToByte(dir: *const vec3_t) -> i32 {
    if dir.is_null() { return 0; }
    let dir = unsafe { &*dir };
    let mut bestd: f32 = 0.0;
    let mut best: i32 = 0;
    for i in 0..NUMVERTEXNORMALS {
        let d = dot_product(dir, &bytedirs[i]);
        if d > bestd {
            bestd = d;
            best = i as i32;
        }
    }
    best
}

#[no_mangle]
pub extern "C" fn ByteToDir(b: i32, dir: &mut vec3_t) {
    if b < 0 || b >= NUMVERTEXNORMALS as i32 {
        unsafe { vector_copy(&vec3_origin, dir); }
        return;
    }
    vector_copy(&bytedirs[b as usize], dir);
}

#[no_mangle]
pub extern "C" fn ColorBytes3(r: f32, g: f32, b: f32) -> u32 {
    let mut i: u32 = 0;
    let bytes = unsafe { &mut *(&mut i as *mut u32 as *mut [u8; 4]) };
    bytes[0] = (r * 255.0) as u8;
    bytes[1] = (g * 255.0) as u8;
    bytes[2] = (b * 255.0) as u8;
    i
}

#[no_mangle]
pub extern "C" fn ColorBytes4(r: f32, g: f32, b: f32, a: f32) -> u32 {
    let mut i: u32 = 0;
    let bytes = unsafe { &mut *(&mut i as *mut u32 as *mut [u8; 4]) };
    bytes[0] = (r * 255.0) as u8;
    bytes[1] = (g * 255.0) as u8;
    bytes[2] = (b * 255.0) as u8;
    bytes[3] = (a * 255.0) as u8;
    i
}

#[no_mangle]
pub extern "C" fn NormalizeColor(in_: &vec3_t, out: &mut vec3_t) -> f32 {
    let mut max = in_[0];
    if in_[1] > max { max = in_[1]; }
    if in_[2] > max { max = in_[2]; }
    if max == 0.0 {
        vector_clear(out);
    } else {
        out[0] = in_[0] / max;
        out[1] = in_[1] / max;
        out[2] = in_[2] / max;
    }
    max
}

#[no_mangle]
pub extern "C" fn Q_rsqrt(number: f32) -> f32 {
    let x2 = number * 0.5f32;
    let mut i: u32;
    let mut y = number;
    i = y.to_bits();
    i = 0x5f3759dfu32.wrapping_sub(i >> 1);
    y = f32::from_bits(i);
    y = y * (1.5f32 - (x2 * y * y));
    y
}

#[no_mangle]
pub extern "C" fn Q_fabs(f: f32) -> f32 {
    let mut tmp = f.to_bits() as i32;
    tmp &= 0x7FFFFFFF;
    f32::from_bits(tmp as u32)
}

#[no_mangle]
pub extern "C" fn VectorNormalize(v: &mut vec3_t) -> f32 {
    let length = ((v[0] * v[0] + v[1] * v[1] + v[2] * v[2]) as f64).sqrt() as f32;
    if length != 0.0 {
        let ilength = 1.0 / length;
        v[0] *= ilength;
        v[1] *= ilength;
        v[2] *= ilength;
    }
    length
}

#[no_mangle]
pub extern "C" fn VectorNormalize2(v: &vec3_t, out: &mut vec3_t) -> f32 {
    let length = ((v[0] * v[0] + v[1] * v[1] + v[2] * v[2]) as f64).sqrt() as f32;
    if length != 0.0 {
        let ilength = 1.0 / length;
        out[0] = v[0] * ilength;
        out[1] = v[1] * ilength;
        out[2] = v[2] * ilength;
    } else {
        vector_clear(out);
    }
    length
}

#[no_mangle]
pub extern "C" fn LerpAngle(from: f32, mut to: f32, frac: f32) -> f32 {
    if to - from > 180.0 { to -= 360.0; }
    if to - from < -180.0 { to += 360.0; }
    from + frac * (to - from)
}

#[no_mangle]
pub extern "C" fn AngleSubtract(a1: f32, a2: f32) -> f32 {
    let mut a = a1 - a2;
    while a > 180.0 { a -= 360.0; }
    while a < -180.0 { a += 360.0; }
    a
}

#[no_mangle]
pub extern "C" fn AnglesSubtract(v1: &vec3_t, v2: &vec3_t, v3: &mut vec3_t) {
    v3[0] = AngleSubtract(v1[0], v2[0]);
    v3[1] = AngleSubtract(v1[1], v2[1]);
    v3[2] = AngleSubtract(v1[2], v2[2]);
}

#[no_mangle]
pub extern "C" fn AngleMod(a: f32) -> f32 {
    (360.0f32 / 65536.0) * ((a * (65536.0 / 360.0)) as i32 & 65535) as f32
}

#[no_mangle]
pub extern "C" fn AngleNormalize360(angle: f32) -> f32 {
    (360.0f32 / 65536.0) * ((angle * (65536.0 / 360.0)) as i32 & 65535) as f32
}

#[no_mangle]
pub extern "C" fn AngleNormalize180(angle: f32) -> f32 {
    let mut a = AngleNormalize360(angle);
    if a > 180.0 { a -= 360.0; }
    a
}

#[no_mangle]
pub extern "C" fn AngleDelta(angle1: f32, angle2: f32) -> f32 {
    AngleNormalize180(angle1 - angle2)
}

#[no_mangle]
pub extern "C" fn Q_log2(mut val: i32) -> i32 {
    let mut answer = 0;
    loop {
        val >>= 1;
        if val == 0 { break; }
        answer += 1;
    }
    answer
}

#[no_mangle]
pub extern "C" fn _DotProduct(v1: &vec3_t, v2: &vec3_t) -> f32 {
    v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2]
}

#[no_mangle]
pub extern "C" fn _VectorSubtract(veca: &vec3_t, vecb: &vec3_t, out: &mut vec3_t) {
    out[0] = veca[0] - vecb[0];
    out[1] = veca[1] - vecb[1];
    out[2] = veca[2] - vecb[2];
}

#[no_mangle]
pub extern "C" fn _VectorAdd(veca: &vec3_t, vecb: &vec3_t, out: &mut vec3_t) {
    out[0] = veca[0] + vecb[0];
    out[1] = veca[1] + vecb[1];
    out[2] = veca[2] + vecb[2];
}

#[no_mangle]
pub extern "C" fn _VectorCopy(in_: &vec3_t, out: &mut vec3_t) {
    out[0] = in_[0]; out[1] = in_[1]; out[2] = in_[2];
}

#[no_mangle]
pub extern "C" fn _VectorScale(in_: &vec3_t, scale: f32, out: &mut vec3_t) {
    out[0] = in_[0] * scale;
    out[1] = in_[1] * scale;
    out[2] = in_[2] * scale;
}

#[no_mangle]
pub extern "C" fn _VectorMA(veca: &vec3_t, scale: f32, vecb: &vec3_t, vecc: &mut vec3_t) {
    vecc[0] = veca[0] + scale * vecb[0];
    vecc[1] = veca[1] + scale * vecb[1];
    vecc[2] = veca[2] + scale * vecb[2];
}

#[no_mangle]
pub extern "C" fn Vector4Scale(in_: &vec4_t, scale: f32, out: &mut vec4_t) {
    out[0] = in_[0] * scale;
    out[1] = in_[1] * scale;
    out[2] = in_[2] * scale;
    out[3] = in_[3] * scale;
}

#[no_mangle]
pub extern "C" fn MatrixMultiply(in1: &[[f32; 3]; 3], in2: &[[f32; 3]; 3], out: &mut [[f32; 3]; 3]) {
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
pub extern "C" fn AngleVectors(angles: &vec3_t, forward: *mut vec3_t, right: *mut vec3_t, up: *mut vec3_t) {
    // C code: M_PI is defined as 3.14159265358979323846f (float)
    // angle = angles[YAW] * (M_PI*2 / 360) -- float arithmetic
    // sy = sin(angle) -- float promoted to double for sin(), result stored in float
    let m_pi: f32 = 3.14159265358979323846f32;

    let angle = angles[YAW] * (m_pi * 2.0f32 / 360.0f32);
    let sy = (angle as f64).sin() as f32;
    let cy = (angle as f64).cos() as f32;
    let angle = angles[PITCH] * (m_pi * 2.0f32 / 360.0f32);
    let sp = (angle as f64).sin() as f32;
    let cp = (angle as f64).cos() as f32;
    let angle = angles[ROLL] * (m_pi * 2.0f32 / 360.0f32);
    let sr = (angle as f64).sin() as f32;
    let cr = (angle as f64).cos() as f32;

    if !forward.is_null() {
        let fwd = unsafe { &mut *forward };
        fwd[0] = cp * cy;
        fwd[1] = cp * sy;
        fwd[2] = -sp;
    }
    if !right.is_null() {
        let r = unsafe { &mut *right };
        r[0] = -1.0 * sr * sp * cy + -1.0 * cr * -sy;
        r[1] = -1.0 * sr * sp * sy + -1.0 * cr * cy;
        r[2] = -1.0 * sr * cp;
    }
    if !up.is_null() {
        let u = unsafe { &mut *up };
        u[0] = cr * sp * cy + -sr * -sy;
        u[1] = cr * sp * sy + -sr * cy;
        u[2] = cr * cp;
    }
}

#[no_mangle]
pub extern "C" fn ProjectPointOnPlane(dst: &mut vec3_t, p: &vec3_t, normal: &vec3_t) {
    let inv_denom = 1.0f32 / dot_product(normal, normal);
    let d = dot_product(normal, p) * inv_denom;
    let n = [normal[0] * inv_denom, normal[1] * inv_denom, normal[2] * inv_denom];
    dst[0] = p[0] - d * n[0];
    dst[1] = p[1] - d * n[1];
    dst[2] = p[2] - d * n[2];
}

#[no_mangle]
pub extern "C" fn PerpendicularVector(dst: &mut vec3_t, src: &vec3_t) {
    let mut pos = 0usize;
    let mut minelem = 1.0f32;
    for i in 0..3 {
        if src[i].abs() < minelem {
            pos = i;
            minelem = src[i].abs();
        }
    }
    let mut tempvec: vec3_t = [0.0, 0.0, 0.0];
    tempvec[pos] = 1.0;
    ProjectPointOnPlane(dst, &tempvec, src);
    VectorNormalize(dst);
}

#[no_mangle]
pub extern "C" fn RotatePointAroundVector(dst: &mut vec3_t, dir: &vec3_t, point: &vec3_t, degrees: f32) {
    let vf: vec3_t = [dir[0], dir[1], dir[2]];
    let mut vr: vec3_t = [0.0; 3];
    let mut vup: vec3_t = [0.0; 3];

    PerpendicularVector(&mut vr, dir);
    cross_product(&vr, &vf, &mut vup);

    let mut m: [[f32; 3]; 3] = [[0.0; 3]; 3];
    m[0][0] = vr[0]; m[1][0] = vr[1]; m[2][0] = vr[2];
    m[0][1] = vup[0]; m[1][1] = vup[1]; m[2][1] = vup[2];
    m[0][2] = vf[0]; m[1][2] = vf[1]; m[2][2] = vf[2];

    let mut im = m;
    im[0][1] = m[1][0]; im[0][2] = m[2][0];
    im[1][0] = m[0][1]; im[1][2] = m[2][1];
    im[2][0] = m[0][2]; im[2][1] = m[1][2];

    // C: DEG2RAD(a) = ((a) * M_PI) / 180.0F  where M_PI is float
    let m_pi: f32 = 3.14159265358979323846f32;
    let rad: f32 = (degrees * m_pi) / 180.0f32;
    // C: cos(rad) and sin(rad) - rad is float promoted to double
    let mut zrot: [[f32; 3]; 3] = [[0.0; 3]; 3];
    zrot[0][0] = (rad as f64).cos() as f32;
    zrot[0][1] = (rad as f64).sin() as f32;
    zrot[1][0] = -((rad as f64).sin() as f32);
    zrot[1][1] = (rad as f64).cos() as f32;
    zrot[2][2] = 1.0;

    let mut tmpmat: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut rot: [[f32; 3]; 3] = [[0.0; 3]; 3];
    MatrixMultiply(&m, &zrot, &mut tmpmat);
    MatrixMultiply(&tmpmat, &im, &mut rot);

    for i in 0..3 {
        dst[i] = rot[i][0] * point[0] + rot[i][1] * point[1] + rot[i][2] * point[2];
    }
}

#[no_mangle]
pub extern "C" fn RotateAroundDirection(axis: &mut [vec3_t; 3], yaw: f32) {
    let axis0 = axis[0];
    PerpendicularVector(&mut axis[1], &axis0);
    if yaw != 0.0 {
        let temp = axis[1];
        RotatePointAroundVector(&mut axis[1], &axis0, &temp, yaw);
    }
    let axis0 = axis[0];
    let axis1 = axis[1];
    cross_product(&axis0, &axis1, &mut axis[2]);
}

#[no_mangle]
pub extern "C" fn vectoangles(value1: &vec3_t, angles: &mut vec3_t) {
    let yaw: f32;
    let pitch: f32;

    if value1[1] == 0.0 && value1[0] == 0.0 {
        yaw = 0.0;
        if value1[2] > 0.0 { pitch = 90.0; } else { pitch = 270.0; }
    } else {
        if value1[0] != 0.0 {
            yaw = ((value1[1] as f64).atan2(value1[0] as f64) * 180.0 / PI) as f32;
        } else if value1[1] > 0.0 {
            yaw = 90.0;
        } else {
            yaw = 270.0;
        }
        let mut yaw_val = yaw;
        if yaw_val < 0.0 { yaw_val += 360.0; }

        let forward = ((value1[0] * value1[0] + value1[1] * value1[1]) as f64).sqrt();
        pitch = ((value1[2] as f64).atan2(forward) * 180.0 / PI) as f32;
        let mut pitch_val = pitch;
        if pitch_val < 0.0 { pitch_val += 360.0; }

        angles[PITCH] = -pitch_val;
        angles[YAW] = yaw_val;
        angles[ROLL] = 0.0;
        return;
    }

    angles[PITCH] = -pitch;
    angles[YAW] = yaw;
    angles[ROLL] = 0.0;
}

#[no_mangle]
pub extern "C" fn AnglesToAxis(angles: &vec3_t, axis: &mut [vec3_t; 3]) {
    let mut right: vec3_t = [0.0; 3];
    let mut fwd: vec3_t = [0.0; 3];
    let mut up: vec3_t = [0.0; 3];
    AngleVectors(angles, &mut fwd as *mut _, &mut right as *mut _, &mut up as *mut _);
    axis[0] = fwd;
    axis[2] = up;
    unsafe {
        axis[1][0] = vec3_origin[0] - right[0];
        axis[1][1] = vec3_origin[1] - right[1];
        axis[1][2] = vec3_origin[2] - right[2];
    }
}

#[no_mangle]
pub extern "C" fn AxisClear(axis: &mut [vec3_t; 3]) {
    axis[0] = [1.0, 0.0, 0.0];
    axis[1] = [0.0, 1.0, 0.0];
    axis[2] = [0.0, 0.0, 1.0];
}

#[no_mangle]
pub extern "C" fn AxisCopy(in_: &[vec3_t; 3], out: &mut [vec3_t; 3]) {
    vector_copy(&in_[0], &mut out[0]);
    vector_copy(&in_[1], &mut out[1]);
    vector_copy(&in_[2], &mut out[2]);
}

#[no_mangle]
pub extern "C" fn MakeNormalVectors(forward: &vec3_t, right: &mut vec3_t, up: &mut vec3_t) {
    right[1] = -forward[0];
    right[2] = forward[1];
    right[0] = forward[2];
    let d = dot_product(right, forward);
    // VectorMA(right, -d, forward, right) - but right is both input and output
    right[0] = right[0] + forward[0] * (-d);
    right[1] = right[1] + forward[1] * (-d);
    right[2] = right[2] + forward[2] * (-d);
    VectorNormalize(right);
    cross_product(right, forward, up);
}

#[no_mangle]
pub extern "C" fn VectorRotate(in_: &vec3_t, matrix: &[vec3_t; 3], out: &mut vec3_t) {
    out[0] = dot_product(in_, &matrix[0]);
    out[1] = dot_product(in_, &matrix[1]);
    out[2] = dot_product(in_, &matrix[2]);
}

#[no_mangle]
pub extern "C" fn SetPlaneSignbits(out: &mut cplane_t) {
    let mut bits: i32 = 0;
    for j in 0..3 {
        if out.normal[j] < 0.0 {
            bits |= 1 << j;
        }
    }
    out.signbits = bits as u8;
}

#[no_mangle]
pub extern "C" fn BoxOnPlaneSide(emins: &vec3_t, emaxs: &vec3_t, p: &cplane_t) -> i32 {
    let dist1: f32;
    let dist2: f32;

    if (p.type_ as i32) < 3 {
        if p.dist <= emins[p.type_ as usize] { return 1; }
        if p.dist >= emaxs[p.type_ as usize] { return 2; }
        return 3;
    }

    match p.signbits {
        0 => {
            dist1 = p.normal[0]*emaxs[0] + p.normal[1]*emaxs[1] + p.normal[2]*emaxs[2];
            dist2 = p.normal[0]*emins[0] + p.normal[1]*emins[1] + p.normal[2]*emins[2];
        }
        1 => {
            dist1 = p.normal[0]*emins[0] + p.normal[1]*emaxs[1] + p.normal[2]*emaxs[2];
            dist2 = p.normal[0]*emaxs[0] + p.normal[1]*emins[1] + p.normal[2]*emins[2];
        }
        2 => {
            dist1 = p.normal[0]*emaxs[0] + p.normal[1]*emins[1] + p.normal[2]*emaxs[2];
            dist2 = p.normal[0]*emins[0] + p.normal[1]*emaxs[1] + p.normal[2]*emins[2];
        }
        3 => {
            dist1 = p.normal[0]*emins[0] + p.normal[1]*emins[1] + p.normal[2]*emaxs[2];
            dist2 = p.normal[0]*emaxs[0] + p.normal[1]*emaxs[1] + p.normal[2]*emins[2];
        }
        4 => {
            dist1 = p.normal[0]*emaxs[0] + p.normal[1]*emaxs[1] + p.normal[2]*emins[2];
            dist2 = p.normal[0]*emins[0] + p.normal[1]*emins[1] + p.normal[2]*emaxs[2];
        }
        5 => {
            dist1 = p.normal[0]*emins[0] + p.normal[1]*emaxs[1] + p.normal[2]*emins[2];
            dist2 = p.normal[0]*emaxs[0] + p.normal[1]*emins[1] + p.normal[2]*emaxs[2];
        }
        6 => {
            dist1 = p.normal[0]*emaxs[0] + p.normal[1]*emins[1] + p.normal[2]*emins[2];
            dist2 = p.normal[0]*emins[0] + p.normal[1]*emaxs[1] + p.normal[2]*emaxs[2];
        }
        7 => {
            dist1 = p.normal[0]*emins[0] + p.normal[1]*emins[1] + p.normal[2]*emins[2];
            dist2 = p.normal[0]*emaxs[0] + p.normal[1]*emaxs[1] + p.normal[2]*emaxs[2];
        }
        _ => { dist1 = 0.0; dist2 = 0.0; }
    }

    let mut sides = 0;
    if dist1 >= p.dist { sides = 1; }
    if dist2 < p.dist { sides |= 2; }
    sides
}

#[no_mangle]
pub extern "C" fn RadiusFromBounds(mins: &vec3_t, maxs: &vec3_t) -> f32 {
    let mut corner: vec3_t = [0.0; 3];
    for i in 0..3 {
        let a = (mins[i] as f64).abs() as f32;
        let b = (maxs[i] as f64).abs() as f32;
        corner[i] = if a > b { a } else { b };
    }
    vector_length(&corner)
}

#[no_mangle]
pub extern "C" fn ClearBounds(mins: &mut vec3_t, maxs: &mut vec3_t) {
    mins[0] = 99999.0; mins[1] = 99999.0; mins[2] = 99999.0;
    maxs[0] = -99999.0; maxs[1] = -99999.0; maxs[2] = -99999.0;
}

#[no_mangle]
pub extern "C" fn AddPointToBounds(v: &vec3_t, mins: &mut vec3_t, maxs: &mut vec3_t) {
    for i in 0..3 {
        if v[i] < mins[i] { mins[i] = v[i]; }
        if v[i] > maxs[i] { maxs[i] = v[i]; }
    }
}

#[no_mangle]
pub extern "C" fn PlaneFromPoints(plane: &mut vec4_t, a: &vec3_t, b: &vec3_t, c: &vec3_t) -> qboolean {
    let mut d1: vec3_t = [0.0; 3];
    let mut d2: vec3_t = [0.0; 3];
    vector_subtract(b, a, &mut d1);
    vector_subtract(c, a, &mut d2);
    let mut normal: vec3_t = [0.0; 3];
    cross_product(&d2, &d1, &mut normal);
    if VectorNormalize(&mut normal) == 0.0 {
        return 0; // qfalse
    }
    plane[0] = normal[0];
    plane[1] = normal[1];
    plane[2] = normal[2];
    plane[3] = dot_product(a, &normal);
    1 // qtrue
}
