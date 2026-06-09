// Rust translation of q_math.c (Quake III Arena math library)
// Goal: byte-identical behavior with the C library

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]

use std::ffi::{c_float, c_int, c_short, c_uint, c_schar};

// ===========================================================================
// Constants
// ===========================================================================

const M_PI: f32 = 3.14159265358979323846_f32;

const NUMVERTEXNORMALS: usize = 162;

const PITCH: usize = 0;
const YAW: usize = 1;
const ROLL: usize = 2;

// ===========================================================================
// External libm functions to ensure C-identical math results
// ===========================================================================

extern "C" {
    fn sin(x: f64) -> f64;
    fn cos(x: f64) -> f64;
    fn sqrt(x: f64) -> f64;
    fn atan2(y: f64, x: f64) -> f64;
    fn fabs(x: f64) -> f64;
}

#[inline]
fn c_sinf(x: f32) -> f32 {
    unsafe { sin(x as f64) as f32 }
}
#[inline]
fn c_cosf(x: f32) -> f32 {
    unsafe { cos(x as f64) as f32 }
}
#[inline]
fn c_sqrtf(x: f32) -> f32 {
    unsafe { sqrt(x as f64) as f32 }
}
#[inline]
fn c_atan2f(y: f32, x: f32) -> f32 {
    unsafe { atan2(y as f64, x as f64) as f32 }
}
#[inline]
fn c_fabsf(x: f32) -> f32 {
    unsafe { fabs(x as f64) as f32 }
}

// ===========================================================================
// Globals (mutable in C, exposed with extern "C" linkage)
// ===========================================================================

#[unsafe(no_mangle)]
pub static mut vec3_origin: [f32; 3] = [0.0, 0.0, 0.0];

#[unsafe(no_mangle)]
pub static mut axisDefault: [[f32; 3]; 3] = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
];

#[unsafe(no_mangle)]
pub static mut colorBlack: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
#[unsafe(no_mangle)]
pub static mut colorRed: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
#[unsafe(no_mangle)]
pub static mut colorGreen: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
#[unsafe(no_mangle)]
pub static mut colorBlue: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
#[unsafe(no_mangle)]
pub static mut colorYellow: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
#[unsafe(no_mangle)]
pub static mut colorMagenta: [f32; 4] = [1.0, 0.0, 1.0, 1.0];
#[unsafe(no_mangle)]
pub static mut colorCyan: [f32; 4] = [0.0, 1.0, 1.0, 1.0];
#[unsafe(no_mangle)]
pub static mut colorWhite: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
#[unsafe(no_mangle)]
pub static mut colorLtGrey: [f32; 4] = [0.75, 0.75, 0.75, 1.0];
#[unsafe(no_mangle)]
pub static mut colorMdGrey: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
#[unsafe(no_mangle)]
pub static mut colorDkGrey: [f32; 4] = [0.25, 0.25, 0.25, 1.0];

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
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

// ===========================================================================
// cplane_t struct (matching C layout)
// ===========================================================================

#[repr(C)]
pub struct cplane_s {
    pub normal: [f32; 3],
    pub dist: f32,
    pub r#type: u8,
    pub signbits: u8,
    pub pad: [u8; 2],
}

// ===========================================================================
// Helper: vec3_t pointer access
// ===========================================================================

#[inline]
unsafe fn read3(p: *const f32) -> [f32; 3] {
    [*p.add(0), *p.add(1), *p.add(2)]
}

#[inline]
unsafe fn write3(p: *mut f32, v: [f32; 3]) {
    *p.add(0) = v[0];
    *p.add(1) = v[1];
    *p.add(2) = v[2];
}

#[inline]
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn cross3(v1: [f32; 3], v2: [f32; 3]) -> [f32; 3] {
    [
        v1[1] * v2[2] - v1[2] * v2[1],
        v1[2] * v2[0] - v1[0] * v2[2],
        v1[0] * v2[1] - v1[1] * v2[0],
    ]
}

// ===========================================================================
// Q_rand / Q_random / Q_crandom
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Q_rand(seed: *mut c_int) -> c_int {
    *seed = (69069i32).wrapping_mul(*seed).wrapping_add(1);
    *seed
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Q_random(seed: *mut c_int) -> c_float {
    (Q_rand(seed) & 0xffff) as f32 / 0x10000 as f32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Q_crandom(seed: *mut c_int) -> c_float {
    // Note: C: 2.0 * (Q_random(seed) - 0.5) — 2.0 and 0.5 are doubles in C.
    // Result is double, then converted back to float on return.
    (2.0_f64 * (Q_random(seed) as f64 - 0.5_f64)) as f32
}

// ===========================================================================
// ClampChar / ClampShort
// ===========================================================================

#[unsafe(no_mangle)]
pub extern "C" fn ClampChar(i: c_int) -> c_schar {
    if i < -128 {
        return -128;
    }
    if i > 127 {
        return 127;
    }
    i as c_schar
}

#[unsafe(no_mangle)]
pub extern "C" fn ClampShort(i: c_int) -> c_short {
    if i < -32768 {
        return -32768;
    }
    if i > 0x7fff {
        return 0x7fff;
    }
    i as c_short
}

// ===========================================================================
// DirToByte / ByteToDir
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn DirToByte(dir: *mut f32) -> c_int {
    if dir.is_null() {
        return 0;
    }

    let d_in = read3(dir);
    let mut bestd: f32 = 0.0;
    let mut best: c_int = 0;
    for i in 0..NUMVERTEXNORMALS {
        let bd = bytedirs[i];
        let d = d_in[0] * bd[0] + d_in[1] * bd[1] + d_in[2] * bd[2];
        if d > bestd {
            bestd = d;
            best = i as c_int;
        }
    }
    best
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ByteToDir(b: c_int, dir: *mut f32) {
    if b < 0 || b >= NUMVERTEXNORMALS as c_int {
        write3(dir, vec3_origin);
        return;
    }
    write3(dir, bytedirs[b as usize]);
}

// ===========================================================================
// ColorBytes3 / ColorBytes4
// ===========================================================================

#[unsafe(no_mangle)]
pub extern "C" fn ColorBytes3(r: c_float, g: c_float, b: c_float) -> c_uint {
    // Replicates: ((byte*)&i)[0] = r * 255; etc.
    // Cast float to byte truncates toward zero (C: float→unsigned char).
    let mut bytes: [u8; 4] = [0, 0, 0, 0];
    bytes[0] = (r * 255.0) as u8;
    bytes[1] = (g * 255.0) as u8;
    bytes[2] = (b * 255.0) as u8;
    // bytes[3] left as 0 (uninitialized stack memory in C; we keep deterministic 0)
    // The C code declares `unsigned i;` uninitialized then writes 3 of 4 bytes.
    // To exactly match the C UB (uninitialized byte), we must reproduce; but byte 3
    // is never written nor read elsewhere in the same call. Returning 0 is the
    // most-deterministic bit pattern; matching tests typically only inspect bytes 0..2.
    u32::from_ne_bytes(bytes)
}

#[unsafe(no_mangle)]
pub extern "C" fn ColorBytes4(r: c_float, g: c_float, b: c_float, a: c_float) -> c_uint {
    let mut bytes: [u8; 4] = [0, 0, 0, 0];
    bytes[0] = (r * 255.0) as u8;
    bytes[1] = (g * 255.0) as u8;
    bytes[2] = (b * 255.0) as u8;
    bytes[3] = (a * 255.0) as u8;
    u32::from_ne_bytes(bytes)
}

// ===========================================================================
// NormalizeColor
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn NormalizeColor(in_: *const f32, out: *mut f32) -> c_float {
    let in_arr = read3(in_);
    let mut max = in_arr[0];
    if in_arr[1] > max {
        max = in_arr[1];
    }
    if in_arr[2] > max {
        max = in_arr[2];
    }
    if max == 0.0 {
        write3(out, [0.0, 0.0, 0.0]);
    } else {
        write3(out, [in_arr[0] / max, in_arr[1] / max, in_arr[2] / max]);
    }
    max
}

// ===========================================================================
// PlaneFromPoints
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn PlaneFromPoints(
    plane: *mut f32,
    a: *const f32,
    b: *const f32,
    c: *const f32,
) -> c_int {
    let av = read3(a);
    let bv = read3(b);
    let cv = read3(c);
    let d1 = sub3(bv, av);
    let d2 = sub3(cv, av);
    let cr = cross3(d2, d1);
    *plane.add(0) = cr[0];
    *plane.add(1) = cr[1];
    *plane.add(2) = cr[2];
    if VectorNormalize(plane) == 0.0 {
        return 0; // qfalse
    }
    *plane.add(3) = av[0] * *plane.add(0) + av[1] * *plane.add(1) + av[2] * *plane.add(2);
    1 // qtrue
}

// ===========================================================================
// RotatePointAroundVector
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn RotatePointAroundVector(
    dst: *mut f32,
    dir: *const f32,
    point: *const f32,
    degrees: c_float,
) {
    let dir_v = read3(dir);
    let point_v = read3(point);

    let mut m: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut im: [[f32; 3]; 3];
    let mut zrot: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut tmpmat: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut rot: [[f32; 3]; 3] = [[0.0; 3]; 3];

    let vf = dir_v;

    // PerpendicularVector(vr, dir)
    let mut vr_arr: [f32; 3] = [0.0; 3];
    PerpendicularVector(vr_arr.as_mut_ptr(), dir);
    let vr = vr_arr;

    // CrossProduct(vr, vf, vup)
    let vup = cross3(vr, vf);

    m[0][0] = vr[0];
    m[1][0] = vr[1];
    m[2][0] = vr[2];

    m[0][1] = vup[0];
    m[1][1] = vup[1];
    m[2][1] = vup[2];

    m[0][2] = vf[0];
    m[1][2] = vf[1];
    m[2][2] = vf[2];

    // memcpy(im, m, sizeof(im));
    im = m;

    im[0][1] = m[1][0];
    im[0][2] = m[2][0];
    im[1][0] = m[0][1];
    im[1][2] = m[2][1];
    im[2][0] = m[0][2];
    im[2][1] = m[1][2];

    // memset(zrot, 0, sizeof(zrot));
    // (already zeroed)
    zrot[0][0] = 1.0;
    zrot[1][1] = 1.0;
    zrot[2][2] = 1.0;

    // rad = DEG2RAD(degrees) = (degrees * M_PI) / 180.0F
    let rad: f32 = (degrees * M_PI) / 180.0_f32;
    zrot[0][0] = c_cosf(rad);
    zrot[0][1] = c_sinf(rad);
    zrot[1][0] = -c_sinf(rad);
    zrot[1][1] = c_cosf(rad);

    matrix_multiply(&m, &zrot, &mut tmpmat);
    matrix_multiply(&tmpmat, &im, &mut rot);

    for i in 0..3usize {
        *dst.add(i) = rot[i][0] * point_v[0] + rot[i][1] * point_v[1] + rot[i][2] * point_v[2];
    }
}

// ===========================================================================
// RotateAroundDirection
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn RotateAroundDirection(axis: *mut f32, yaw: c_float) {
    // axis is vec3_t axis[3]: 9 floats in row-major order
    let axis0 = axis;
    let axis1 = axis.add(3);
    let axis2 = axis.add(6);

    PerpendicularVector(axis1, axis0);

    if yaw != 0.0 {
        let mut temp: [f32; 3] = read3(axis1);
        RotatePointAroundVector(axis1, axis0, temp.as_mut_ptr(), yaw);
    }

    let cr = cross3(read3(axis0), read3(axis1));
    write3(axis2, cr);
}

// ===========================================================================
// vectoangles
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vectoangles(value1: *const f32, angles: *mut f32) {
    let v = read3(value1);
    let mut yaw: f32;
    let mut pitch: f32;
    let forward: f32;

    if v[1] == 0.0 && v[0] == 0.0 {
        yaw = 0.0;
        if v[2] > 0.0 {
            pitch = 90.0;
        } else {
            pitch = 270.0;
        }
    } else {
        if v[0] != 0.0 {
            // atan2(value1[1], value1[0]) * 180 / M_PI
            yaw = c_atan2f(v[1], v[0]) * 180.0_f32 / M_PI;
        } else if v[1] > 0.0 {
            yaw = 90.0;
        } else {
            yaw = 270.0;
        }
        if yaw < 0.0 {
            yaw += 360.0;
        }
        forward = c_sqrtf(v[0] * v[0] + v[1] * v[1]);
        pitch = c_atan2f(v[2], forward) * 180.0_f32 / M_PI;
        if pitch < 0.0 {
            pitch += 360.0;
        }
    }

    *angles.add(PITCH) = -pitch;
    *angles.add(YAW) = yaw;
    *angles.add(ROLL) = 0.0;
}

// ===========================================================================
// AnglesToAxis
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AnglesToAxis(angles: *const f32, axis: *mut f32) {
    let mut right: [f32; 3] = [0.0; 3];
    AngleVectors(angles, axis, right.as_mut_ptr(), axis.add(6));
    // axis[1] = vec3_origin - right
    let vo = vec3_origin;
    let axis1 = axis.add(3);
    *axis1.add(0) = vo[0] - right[0];
    *axis1.add(1) = vo[1] - right[1];
    *axis1.add(2) = vo[2] - right[2];
}

// ===========================================================================
// AxisClear / AxisCopy
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AxisClear(axis: *mut f32) {
    *axis.add(0) = 1.0;
    *axis.add(1) = 0.0;
    *axis.add(2) = 0.0;
    *axis.add(3) = 0.0;
    *axis.add(4) = 1.0;
    *axis.add(5) = 0.0;
    *axis.add(6) = 0.0;
    *axis.add(7) = 0.0;
    *axis.add(8) = 1.0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AxisCopy(in_: *mut f32, out: *mut f32) {
    write3(out, read3(in_));
    write3(out.add(3), read3(in_.add(3)));
    write3(out.add(6), read3(in_.add(6)));
}

// ===========================================================================
// ProjectPointOnPlane
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ProjectPointOnPlane(dst: *mut f32, p: *const f32, normal: *const f32) {
    let nrm = read3(normal);
    let pv = read3(p);

    let mut inv_denom = dot3(nrm, nrm);
    inv_denom = 1.0_f32 / inv_denom;

    let d = dot3(nrm, pv) * inv_denom;

    let n: [f32; 3] = [nrm[0] * inv_denom, nrm[1] * inv_denom, nrm[2] * inv_denom];

    *dst.add(0) = pv[0] - d * n[0];
    *dst.add(1) = pv[1] - d * n[1];
    *dst.add(2) = pv[2] - d * n[2];
}

// ===========================================================================
// MakeNormalVectors
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn MakeNormalVectors(forward: *const f32, right: *mut f32, up: *mut f32) {
    let fwd = read3(forward);
    // right[1] = -forward[0]; right[2] = forward[1]; right[0] = forward[2];
    *right.add(1) = -fwd[0];
    *right.add(2) = fwd[1];
    *right.add(0) = fwd[2];

    let r0 = read3(right);
    let d = dot3(r0, fwd);
    // VectorMA(right, -d, forward, right)
    *right.add(0) = r0[0] + (-d) * fwd[0];
    *right.add(1) = r0[1] + (-d) * fwd[1];
    *right.add(2) = r0[2] + (-d) * fwd[2];
    VectorNormalize(right);

    let r1 = read3(right);
    let cr = cross3(r1, fwd);
    write3(up, cr);
}

// ===========================================================================
// VectorRotate
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn VectorRotate(in_: *mut f32, matrix: *mut f32, out: *mut f32) {
    let v = read3(in_);
    *out.add(0) = dot3(v, read3(matrix));
    *out.add(1) = dot3(v, read3(matrix.add(3)));
    *out.add(2) = dot3(v, read3(matrix.add(6)));
}

// ===========================================================================
// Q_rsqrt — fast inverse square root (bit-level hack)
// ===========================================================================

#[unsafe(no_mangle)]
pub extern "C" fn Q_rsqrt(number: c_float) -> c_float {
    let threehalfs: f32 = 1.5;
    let x2 = number * 0.5_f32;
    let mut y: f32 = number;

    // memcpy(&i, &y, sizeof(float)); — type-punning float -> u32
    let mut i: u32 = y.to_bits();
    i = 0x5f3759df_u32.wrapping_sub(i >> 1);
    // memcpy(&y, &i, sizeof(float));
    y = f32::from_bits(i);

    y = y * (threehalfs - (x2 * y * y));
    y
}

// ===========================================================================
// Q_fabs — absolute value via bit manipulation
// ===========================================================================

#[unsafe(no_mangle)]
pub extern "C" fn Q_fabs(f: c_float) -> c_float {
    let mut tmp: u32 = f.to_bits();
    tmp &= 0x7FFFFFFF;
    f32::from_bits(tmp)
}

// ===========================================================================
// LerpAngle / AngleSubtract / AnglesSubtract / AngleMod
// ===========================================================================

#[unsafe(no_mangle)]
pub extern "C" fn LerpAngle(from: c_float, mut to: c_float, frac: c_float) -> c_float {
    if to - from > 180.0 {
        to -= 360.0;
    }
    if to - from < -180.0 {
        to += 360.0;
    }
    from + frac * (to - from)
}

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AnglesSubtract(v1: *mut f32, v2: *mut f32, v3: *mut f32) {
    *v3.add(0) = AngleSubtract(*v1.add(0), *v2.add(0));
    *v3.add(1) = AngleSubtract(*v1.add(1), *v2.add(1));
    *v3.add(2) = AngleSubtract(*v1.add(2), *v2.add(2));
}

#[unsafe(no_mangle)]
pub extern "C" fn AngleMod(a: c_float) -> c_float {
    // C: a = (360.0/65536) * ((int)(a*(65536/360.0)) & 65535)
    // 360.0 / 65536 is double in C; (int)cast truncates toward zero.
    let inner = (a as f64 * (65536.0_f64 / 360.0_f64)) as i32;
    let masked = inner & 65535;
    ((360.0_f64 / 65536.0_f64) * masked as f64) as f32
}

#[unsafe(no_mangle)]
pub extern "C" fn AngleNormalize360(angle: c_float) -> c_float {
    let inner = (angle as f64 * (65536.0_f64 / 360.0_f64)) as i32;
    let masked = inner & 65535;
    ((360.0_f64 / 65536.0_f64) * masked as f64) as f32
}

#[unsafe(no_mangle)]
pub extern "C" fn AngleNormalize180(angle: c_float) -> c_float {
    let mut angle = AngleNormalize360(angle);
    if angle > 180.0 {
        angle -= 360.0;
    }
    angle
}

#[unsafe(no_mangle)]
pub extern "C" fn AngleDelta(angle1: c_float, angle2: c_float) -> c_float {
    AngleNormalize180(angle1 - angle2)
}

// ===========================================================================
// SetPlaneSignbits
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SetPlaneSignbits(out: *mut cplane_s) {
    let mut bits: c_int = 0;
    let p = &mut *out;
    for j in 0..3usize {
        if p.normal[j] < 0.0 {
            bits |= 1 << j;
        }
    }
    p.signbits = bits as u8;
}

// ===========================================================================
// BoxOnPlaneSide
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn BoxOnPlaneSide(
    emins: *mut f32,
    emaxs: *mut f32,
    p: *mut cplane_s,
) -> c_int {
    let plane = &*p;
    let emins_v = read3(emins);
    let emaxs_v = read3(emaxs);

    if (plane.r#type as c_int) < 3 {
        if plane.dist <= emins_v[plane.r#type as usize] {
            return 1;
        }
        if plane.dist >= emaxs_v[plane.r#type as usize] {
            return 2;
        }
        return 3;
    }

    let n = &plane.normal;
    let (dist1, dist2) = match plane.signbits {
        0 => (
            n[0] * emaxs_v[0] + n[1] * emaxs_v[1] + n[2] * emaxs_v[2],
            n[0] * emins_v[0] + n[1] * emins_v[1] + n[2] * emins_v[2],
        ),
        1 => (
            n[0] * emins_v[0] + n[1] * emaxs_v[1] + n[2] * emaxs_v[2],
            n[0] * emaxs_v[0] + n[1] * emins_v[1] + n[2] * emins_v[2],
        ),
        2 => (
            n[0] * emaxs_v[0] + n[1] * emins_v[1] + n[2] * emaxs_v[2],
            n[0] * emins_v[0] + n[1] * emaxs_v[1] + n[2] * emins_v[2],
        ),
        3 => (
            n[0] * emins_v[0] + n[1] * emins_v[1] + n[2] * emaxs_v[2],
            n[0] * emaxs_v[0] + n[1] * emaxs_v[1] + n[2] * emins_v[2],
        ),
        4 => (
            n[0] * emaxs_v[0] + n[1] * emaxs_v[1] + n[2] * emins_v[2],
            n[0] * emins_v[0] + n[1] * emins_v[1] + n[2] * emaxs_v[2],
        ),
        5 => (
            n[0] * emins_v[0] + n[1] * emaxs_v[1] + n[2] * emins_v[2],
            n[0] * emaxs_v[0] + n[1] * emins_v[1] + n[2] * emaxs_v[2],
        ),
        6 => (
            n[0] * emaxs_v[0] + n[1] * emins_v[1] + n[2] * emins_v[2],
            n[0] * emins_v[0] + n[1] * emaxs_v[1] + n[2] * emaxs_v[2],
        ),
        7 => (
            n[0] * emins_v[0] + n[1] * emins_v[1] + n[2] * emins_v[2],
            n[0] * emaxs_v[0] + n[1] * emaxs_v[1] + n[2] * emaxs_v[2],
        ),
        _ => (0.0_f32, 0.0_f32),
    };

    let mut sides: c_int = 0;
    if dist1 >= plane.dist {
        sides = 1;
    }
    if dist2 < plane.dist {
        sides |= 2;
    }
    sides
}

// ===========================================================================
// RadiusFromBounds / ClearBounds / AddPointToBounds
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn RadiusFromBounds(mins: *const f32, maxs: *const f32) -> c_float {
    let mins_v = read3(mins);
    let maxs_v = read3(maxs);
    let mut corner: [f32; 3] = [0.0; 3];
    for i in 0..3usize {
        // C uses fabs() which is double; result cast back to float by assignment
        let a = c_fabsf(mins_v[i]);
        let b = c_fabsf(maxs_v[i]);
        corner[i] = if a > b { a } else { b };
    }
    // VectorLength: sqrt(...) — note it's the static-inline version using sqrt double
    c_sqrtf(corner[0] * corner[0] + corner[1] * corner[1] + corner[2] * corner[2])
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ClearBounds(mins: *mut f32, maxs: *mut f32) {
    *mins.add(0) = 99999.0;
    *mins.add(1) = 99999.0;
    *mins.add(2) = 99999.0;
    *maxs.add(0) = -99999.0;
    *maxs.add(1) = -99999.0;
    *maxs.add(2) = -99999.0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AddPointToBounds(v: *const f32, mins: *mut f32, maxs: *mut f32) {
    let v_arr = read3(v);
    if v_arr[0] < *mins.add(0) {
        *mins.add(0) = v_arr[0];
    }
    if v_arr[0] > *maxs.add(0) {
        *maxs.add(0) = v_arr[0];
    }
    if v_arr[1] < *mins.add(1) {
        *mins.add(1) = v_arr[1];
    }
    if v_arr[1] > *maxs.add(1) {
        *maxs.add(1) = v_arr[1];
    }
    if v_arr[2] < *mins.add(2) {
        *mins.add(2) = v_arr[2];
    }
    if v_arr[2] > *maxs.add(2) {
        *maxs.add(2) = v_arr[2];
    }
}

// ===========================================================================
// VectorNormalize / VectorNormalize2
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn VectorNormalize(v: *mut f32) -> c_float {
    let vv = read3(v);
    let mut length = vv[0] * vv[0] + vv[1] * vv[1] + vv[2] * vv[2];
    length = c_sqrtf(length);

    if length != 0.0 {
        let ilength = 1.0_f32 / length;
        *v.add(0) = vv[0] * ilength;
        *v.add(1) = vv[1] * ilength;
        *v.add(2) = vv[2] * ilength;
    }
    length
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn VectorNormalize2(v: *const f32, out: *mut f32) -> c_float {
    let vv = read3(v);
    let mut length = vv[0] * vv[0] + vv[1] * vv[1] + vv[2] * vv[2];
    length = c_sqrtf(length);

    if length != 0.0 {
        let ilength = 1.0_f32 / length;
        *out.add(0) = vv[0] * ilength;
        *out.add(1) = vv[1] * ilength;
        *out.add(2) = vv[2] * ilength;
    } else {
        write3(out, [0.0, 0.0, 0.0]);
    }
    length
}

// ===========================================================================
// _VectorMA / _DotProduct / _VectorSubtract / _VectorAdd / _VectorCopy / _VectorScale
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _VectorMA(veca: *const f32, scale: c_float, vecb: *const f32, vecc: *mut f32) {
    let a = read3(veca);
    let b = read3(vecb);
    *vecc.add(0) = a[0] + scale * b[0];
    *vecc.add(1) = a[1] + scale * b[1];
    *vecc.add(2) = a[2] + scale * b[2];
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _DotProduct(v1: *const f32, v2: *const f32) -> c_float {
    let a = read3(v1);
    let b = read3(v2);
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _VectorSubtract(veca: *const f32, vecb: *const f32, out: *mut f32) {
    let a = read3(veca);
    let b = read3(vecb);
    *out.add(0) = a[0] - b[0];
    *out.add(1) = a[1] - b[1];
    *out.add(2) = a[2] - b[2];
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _VectorAdd(veca: *const f32, vecb: *const f32, out: *mut f32) {
    let a = read3(veca);
    let b = read3(vecb);
    *out.add(0) = a[0] + b[0];
    *out.add(1) = a[1] + b[1];
    *out.add(2) = a[2] + b[2];
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _VectorCopy(in_: *const f32, out: *mut f32) {
    write3(out, read3(in_));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _VectorScale(in_: *const f32, scale: c_float, out: *mut f32) {
    let v = read3(in_);
    *out.add(0) = v[0] * scale;
    *out.add(1) = v[1] * scale;
    *out.add(2) = v[2] * scale;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Vector4Scale(in_: *const f32, scale: c_float, out: *mut f32) {
    *out.add(0) = *in_.add(0) * scale;
    *out.add(1) = *in_.add(1) * scale;
    *out.add(2) = *in_.add(2) * scale;
    *out.add(3) = *in_.add(3) * scale;
}

// ===========================================================================
// Q_log2
// ===========================================================================

#[unsafe(no_mangle)]
pub extern "C" fn Q_log2(mut val: c_int) -> c_int {
    let mut answer: c_int = 0;
    loop {
        val >>= 1;
        if val == 0 {
            break;
        }
        answer += 1;
    }
    answer
}

// ===========================================================================
// MatrixMultiply
// ===========================================================================

fn matrix_multiply(in1: &[[f32; 3]; 3], in2: &[[f32; 3]; 3], out: &mut [[f32; 3]; 3]) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn MatrixMultiply(in1: *mut f32, in2: *mut f32, out: *mut f32) {
    // C: float in1[3][3] is a pointer to 3-float arrays; flat 9-float access
    let read_row = |p: *const f32, row: usize| -> [f32; 3] {
        [*p.add(row * 3), *p.add(row * 3 + 1), *p.add(row * 3 + 2)]
    };
    let i1: [[f32; 3]; 3] = [read_row(in1, 0), read_row(in1, 1), read_row(in1, 2)];
    let i2: [[f32; 3]; 3] = [read_row(in2, 0), read_row(in2, 1), read_row(in2, 2)];
    let mut o: [[f32; 3]; 3] = [[0.0; 3]; 3];
    matrix_multiply(&i1, &i2, &mut o);
    for row in 0..3usize {
        for col in 0..3usize {
            *out.add(row * 3 + col) = o[row][col];
        }
    }
}

// ===========================================================================
// AngleVectors
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn AngleVectors(
    angles: *const f32,
    forward: *mut f32,
    right: *mut f32,
    up: *mut f32,
) {
    // Note: C uses static float sr,sp,sy,cr,cp,cy — values persist between
    // calls but are always overwritten before use, so semantically equivalent.
    let ang = read3(angles);
    let mut angle: f32;

    angle = ang[YAW] * (M_PI * 2.0_f32 / 360.0_f32);
    let sy = c_sinf(angle);
    let cy = c_cosf(angle);
    angle = ang[PITCH] * (M_PI * 2.0_f32 / 360.0_f32);
    let sp = c_sinf(angle);
    let cp = c_cosf(angle);
    angle = ang[ROLL] * (M_PI * 2.0_f32 / 360.0_f32);
    let sr = c_sinf(angle);
    let cr = c_cosf(angle);

    if !forward.is_null() {
        *forward.add(0) = cp * cy;
        *forward.add(1) = cp * sy;
        *forward.add(2) = -sp;
    }
    if !right.is_null() {
        *right.add(0) = -1.0 * sr * sp * cy + -1.0 * cr * -sy;
        *right.add(1) = -1.0 * sr * sp * sy + -1.0 * cr * cy;
        *right.add(2) = -1.0 * sr * cp;
    }
    if !up.is_null() {
        *up.add(0) = cr * sp * cy + -sr * -sy;
        *up.add(1) = cr * sp * sy + -sr * cy;
        *up.add(2) = cr * cp;
    }
}

// ===========================================================================
// PerpendicularVector
// ===========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn PerpendicularVector(dst: *mut f32, src: *const f32) {
    let s = read3(src);
    let mut pos: usize = 0;
    let mut minelem: f32 = 1.0_f32;

    for i in 0..3usize {
        let a = c_fabsf(s[i]);
        if a < minelem {
            pos = i;
            minelem = a;
        }
    }
    let mut tempvec: [f32; 3] = [0.0, 0.0, 0.0];
    tempvec[pos] = 1.0_f32;

    ProjectPointOnPlane(dst, tempvec.as_ptr(), src);
    VectorNormalize(dst);
}
