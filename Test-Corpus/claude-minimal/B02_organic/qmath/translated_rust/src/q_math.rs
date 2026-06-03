// Translated from q_math.c - Quake III Arena math routines
// Original Copyright (C) 1999-2005 Id Software, Inc. (GPL v2)

#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(clippy::needless_range_loop)]

pub type Vec_t = f32;
pub type Vec3 = [Vec_t; 3];
pub type Vec4 = [Vec_t; 4];
pub type Vec5 = [Vec_t; 5];

pub type Byte = u8;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Qboolean {
    Qfalse = 0,
    Qtrue = 1,
}

pub use Qboolean::*;

// Angle indexes
pub const PITCH: usize = 0;
pub const YAW: usize = 1;
pub const ROLL: usize = 2;

pub const NUMVERTEXNORMALS: usize = 162;

pub const M_PI: f32 = std::f32::consts::PI;

#[inline]
pub fn DEG2RAD(a: f32) -> f32 {
    (a * M_PI) / 180.0_f32
}

#[inline]
pub fn RAD2DEG(a: f32) -> f32 {
    (a * 180.0_f32) / M_PI
}

// Plane types
pub const PLANE_X: i32 = 0;
pub const PLANE_Y: i32 = 1;
pub const PLANE_Z: i32 = 2;
pub const PLANE_NON_AXIAL: i32 = 3;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CplaneT {
    pub normal: Vec3,
    pub dist: f32,
    pub type_: u8,
    pub signbits: u8,
    pub pad: [u8; 2],
}

pub static mut vec3_origin: Vec3 = [0.0, 0.0, 0.0];
pub static mut axisDefault: [Vec3; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

pub static mut colorBlack: Vec4 = [0.0, 0.0, 0.0, 1.0];
pub static mut colorRed: Vec4 = [1.0, 0.0, 0.0, 1.0];
pub static mut colorGreen: Vec4 = [0.0, 1.0, 0.0, 1.0];
pub static mut colorBlue: Vec4 = [0.0, 0.0, 1.0, 1.0];
pub static mut colorYellow: Vec4 = [1.0, 1.0, 0.0, 1.0];
pub static mut colorMagenta: Vec4 = [1.0, 0.0, 1.0, 1.0];
pub static mut colorCyan: Vec4 = [0.0, 1.0, 1.0, 1.0];
pub static mut colorWhite: Vec4 = [1.0, 1.0, 1.0, 1.0];
pub static mut colorLtGrey: Vec4 = [0.75, 0.75, 0.75, 1.0];
pub static mut colorMdGrey: Vec4 = [0.5, 0.5, 0.5, 1.0];
pub static mut colorDkGrey: Vec4 = [0.25, 0.25, 0.25, 1.0];

pub static mut g_color_table: [Vec4; 8] = [
    [0.0, 0.0, 0.0, 1.0],
    [1.0, 0.0, 0.0, 1.0],
    [0.0, 1.0, 0.0, 1.0],
    [1.0, 1.0, 0.0, 1.0],
    [0.0, 0.0, 1.0, 1.0],
    [0.0, 1.0, 1.0, 1.0],
    [1.0, 0.0, 1.0, 1.0],
    [1.0, 1.0, 1.0, 1.0],
];

pub static bytedirs: [Vec3; NUMVERTEXNORMALS] = [
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

// ==============================================================
// Inline helpers (translated from C macros)

#[inline]
pub fn dot_product(x: &Vec3, y: &Vec3) -> f32 {
    x[0] * y[0] + x[1] * y[1] + x[2] * y[2]
}

#[inline]
pub fn vector_subtract(a: &Vec3, b: &Vec3, c: &mut Vec3) {
    c[0] = a[0] - b[0];
    c[1] = a[1] - b[1];
    c[2] = a[2] - b[2];
}

#[inline]
pub fn vector_add(a: &Vec3, b: &Vec3, c: &mut Vec3) {
    c[0] = a[0] + b[0];
    c[1] = a[1] + b[1];
    c[2] = a[2] + b[2];
}

#[inline]
pub fn vector_copy(a: &Vec3, b: &mut Vec3) {
    b[0] = a[0];
    b[1] = a[1];
    b[2] = a[2];
}

#[inline]
pub fn vector_scale(v: &Vec3, s: f32, o: &mut Vec3) {
    o[0] = v[0] * s;
    o[1] = v[1] * s;
    o[2] = v[2] * s;
}

#[inline]
pub fn vector_ma(v: &Vec3, s: f32, b: &Vec3, o: &mut Vec3) {
    o[0] = v[0] + b[0] * s;
    o[1] = v[1] + b[1] * s;
    o[2] = v[2] + b[2] * s;
}

#[inline]
pub fn vector_clear(a: &mut Vec3) {
    a[0] = 0.0;
    a[1] = 0.0;
    a[2] = 0.0;
}

#[inline]
pub fn vector_negate(a: &Vec3, b: &mut Vec3) {
    b[0] = -a[0];
    b[1] = -a[1];
    b[2] = -a[2];
}

#[inline]
pub fn vector_set(v: &mut Vec3, x: f32, y: f32, z: f32) {
    v[0] = x;
    v[1] = y;
    v[2] = z;
}

#[inline]
pub fn vector4_copy(a: &Vec4, b: &mut Vec4) {
    b[0] = a[0];
    b[1] = a[1];
    b[2] = a[2];
    b[3] = a[3];
}

#[inline]
pub fn VectorCompare(v1: &Vec3, v2: &Vec3) -> i32 {
    if v1[0] != v2[0] || v1[1] != v2[1] || v1[2] != v2[2] {
        0
    } else {
        1
    }
}

#[inline]
pub fn VectorLength(v: &Vec3) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

#[inline]
pub fn VectorLengthSquared(v: &Vec3) -> f32 {
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}

#[inline]
pub fn Distance(p1: &Vec3, p2: &Vec3) -> f32 {
    let mut v: Vec3 = [0.0; 3];
    vector_subtract(p2, p1, &mut v);
    VectorLength(&v)
}

#[inline]
pub fn DistanceSquared(p1: &Vec3, p2: &Vec3) -> f32 {
    let mut v: Vec3 = [0.0; 3];
    vector_subtract(p2, p1, &mut v);
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}

#[inline]
pub fn VectorNormalizeFast(v: &mut Vec3) {
    let ilength = Q_rsqrt(dot_product(v, v));
    v[0] *= ilength;
    v[1] *= ilength;
    v[2] *= ilength;
}

#[inline]
pub fn VectorInverse(v: &mut Vec3) {
    v[0] = -v[0];
    v[1] = -v[1];
    v[2] = -v[2];
}

#[inline]
pub fn CrossProduct(v1: &Vec3, v2: &Vec3, cross: &mut Vec3) {
    cross[0] = v1[1] * v2[2] - v1[2] * v2[1];
    cross[1] = v1[2] * v2[0] - v1[0] * v2[2];
    cross[2] = v1[0] * v2[1] - v1[1] * v2[0];
}

// ==============================================================

pub fn Q_rand(seed: &mut i32) -> i32 {
    *seed = (69069i32).wrapping_mul(*seed).wrapping_add(1);
    *seed
}

pub fn Q_random(seed: &mut i32) -> f32 {
    (Q_rand(seed) & 0xffff) as f32 / 0x10000 as f32
}

pub fn Q_crandom(seed: &mut i32) -> f32 {
    2.0 * (Q_random(seed) - 0.5)
}

// =======================================================

pub fn ClampChar(i: i32) -> i8 {
    if i < -128 {
        return -128;
    }
    if i > 127 {
        return 127;
    }
    i as i8
}

pub fn ClampShort(i: i32) -> i16 {
    if i < -32768 {
        return -32768;
    }
    if i > 0x7fff {
        return 0x7fff;
    }
    i as i16
}

// this isn't a real cheap function to call!
pub fn DirToByte(dir: Option<&Vec3>) -> i32 {
    let dir = match dir {
        Some(d) => d,
        None => return 0,
    };

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

pub fn ByteToDir(b: i32, dir: &mut Vec3) {
    if b < 0 || (b as usize) >= NUMVERTEXNORMALS {
        unsafe {
            vector_copy(&vec3_origin, dir);
        }
        return;
    }
    vector_copy(&bytedirs[b as usize], dir);
}

pub fn ColorBytes3(r: f32, g: f32, b: f32) -> u32 {
    let b0 = (r * 255.0) as u8;
    let b1 = (g * 255.0) as u8;
    let b2 = (b * 255.0) as u8;
    let b3: u8 = 0;
    u32::from_le_bytes([b0, b1, b2, b3])
}

pub fn ColorBytes4(r: f32, g: f32, b: f32, a: f32) -> u32 {
    let b0 = (r * 255.0) as u8;
    let b1 = (g * 255.0) as u8;
    let b2 = (b * 255.0) as u8;
    let b3 = (a * 255.0) as u8;
    u32::from_le_bytes([b0, b1, b2, b3])
}

pub fn NormalizeColor(in_v: &Vec3, out: &mut Vec3) -> f32 {
    let mut max = in_v[0];
    if in_v[1] > max {
        max = in_v[1];
    }
    if in_v[2] > max {
        max = in_v[2];
    }

    if max == 0.0 {
        vector_clear(out);
    } else {
        out[0] = in_v[0] / max;
        out[1] = in_v[1] / max;
        out[2] = in_v[2] / max;
    }
    max
}

/// Returns false if the triangle is degenerate.
/// The normal will point out of the clock for clockwise ordered points.
pub fn PlaneFromPoints(plane: &mut Vec4, a: &Vec3, b: &Vec3, c: &Vec3) -> Qboolean {
    let mut d1: Vec3 = [0.0; 3];
    let mut d2: Vec3 = [0.0; 3];

    vector_subtract(b, a, &mut d1);
    vector_subtract(c, a, &mut d2);

    let mut normal: Vec3 = [0.0; 3];
    CrossProduct(&d2, &d1, &mut normal);
    if VectorNormalize(&mut normal) == 0.0 {
        plane[0] = normal[0];
        plane[1] = normal[1];
        plane[2] = normal[2];
        return Qfalse;
    }
    plane[0] = normal[0];
    plane[1] = normal[1];
    plane[2] = normal[2];

    plane[3] = dot_product(a, &normal);
    Qtrue
}

pub fn RotatePointAroundVector(dst: &mut Vec3, dir: &Vec3, point: &Vec3, degrees: f32) {
    let mut m: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut im: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut zrot: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut tmpmat: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut rot: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut vr: Vec3 = [0.0; 3];
    let mut vup: Vec3 = [0.0; 3];
    let mut vf: Vec3 = [0.0; 3];

    vf[0] = dir[0];
    vf[1] = dir[1];
    vf[2] = dir[2];

    PerpendicularVector(&mut vr, dir);
    CrossProduct(&vr, &vf, &mut vup);

    m[0][0] = vr[0];
    m[1][0] = vr[1];
    m[2][0] = vr[2];

    m[0][1] = vup[0];
    m[1][1] = vup[1];
    m[2][1] = vup[2];

    m[0][2] = vf[0];
    m[1][2] = vf[1];
    m[2][2] = vf[2];

    im = m;

    im[0][1] = m[1][0];
    im[0][2] = m[2][0];
    im[1][0] = m[0][1];
    im[1][2] = m[2][1];
    im[2][0] = m[0][2];
    im[2][1] = m[1][2];

    for i in 0..3 {
        for j in 0..3 {
            zrot[i][j] = 0.0;
        }
    }
    zrot[0][0] = 1.0;
    zrot[1][1] = 1.0;
    zrot[2][2] = 1.0;

    let rad = DEG2RAD(degrees);
    zrot[0][0] = rad.cos();
    zrot[0][1] = rad.sin();
    zrot[1][0] = -(rad.sin());
    zrot[1][1] = rad.cos();

    MatrixMultiply(&m, &zrot, &mut tmpmat);
    MatrixMultiply(&tmpmat, &im, &mut rot);

    for i in 0..3 {
        dst[i] = rot[i][0] * point[0] + rot[i][1] * point[1] + rot[i][2] * point[2];
    }
}

pub fn RotateAroundDirection(axis: &mut [Vec3; 3], yaw: f32) {
    // Create an arbitrary axis[1]
    let axis0_copy = axis[0];
    PerpendicularVector(&mut axis[1], &axis0_copy);

    // Rotate it around axis[0] by yaw
    if yaw != 0.0 {
        let temp: Vec3 = axis[1];
        let axis0 = axis[0];
        RotatePointAroundVector(&mut axis[1], &axis0, &temp, yaw);
    }

    // Cross to get axis[2]
    let a0 = axis[0];
    let a1 = axis[1];
    CrossProduct(&a0, &a1, &mut axis[2]);
}

pub fn vectoangles(value1: &Vec3, angles: &mut Vec3) {
    let yaw: f32;
    let mut pitch: f32;

    if value1[1] == 0.0 && value1[0] == 0.0 {
        yaw = 0.0;
        if value1[2] > 0.0 {
            pitch = 90.0;
        } else {
            pitch = 270.0;
        }
    } else {
        let mut yaw_calc: f32;
        if value1[0] != 0.0 {
            yaw_calc = value1[1].atan2(value1[0]) * 180.0 / M_PI;
        } else if value1[1] > 0.0 {
            yaw_calc = 90.0;
        } else {
            yaw_calc = 270.0;
        }
        if yaw_calc < 0.0 {
            yaw_calc += 360.0;
        }
        yaw = yaw_calc;

        let forward = (value1[0] * value1[0] + value1[1] * value1[1]).sqrt();
        pitch = value1[2].atan2(forward) * 180.0 / M_PI;
        if pitch < 0.0 {
            pitch += 360.0;
        }
    }

    angles[PITCH] = -pitch;
    angles[YAW] = yaw;
    angles[ROLL] = 0.0;
}

pub fn AnglesToAxis(angles: &Vec3, axis: &mut [Vec3; 3]) {
    let mut right: Vec3 = [0.0; 3];

    // Split axis borrows
    let (axis0, axis1, axis2) = {
        let (first, rest) = axis.split_at_mut(1);
        let (mid, last) = rest.split_at_mut(1);
        (&mut first[0], &mut mid[0], &mut last[0])
    };

    AngleVectors(angles, Some(axis0), Some(&mut right), Some(axis2));
    unsafe {
        vector_subtract(&vec3_origin, &right, axis1);
    }
}

pub fn AxisClear(axis: &mut [Vec3; 3]) {
    axis[0][0] = 1.0;
    axis[0][1] = 0.0;
    axis[0][2] = 0.0;
    axis[1][0] = 0.0;
    axis[1][1] = 1.0;
    axis[1][2] = 0.0;
    axis[2][0] = 0.0;
    axis[2][1] = 0.0;
    axis[2][2] = 1.0;
}

pub fn AxisCopy(in_v: &[Vec3; 3], out: &mut [Vec3; 3]) {
    vector_copy(&in_v[0], &mut out[0]);
    vector_copy(&in_v[1], &mut out[1]);
    vector_copy(&in_v[2], &mut out[2]);
}

pub fn ProjectPointOnPlane(dst: &mut Vec3, p: &Vec3, normal: &Vec3) {
    let mut inv_denom = dot_product(normal, normal);
    inv_denom = 1.0 / inv_denom;

    let d = dot_product(normal, p) * inv_denom;

    let mut n: Vec3 = [0.0; 3];
    n[0] = normal[0] * inv_denom;
    n[1] = normal[1] * inv_denom;
    n[2] = normal[2] * inv_denom;

    dst[0] = p[0] - d * n[0];
    dst[1] = p[1] - d * n[1];
    dst[2] = p[2] - d * n[2];
}

/// Given a normalized forward vector, create two
/// other perpendicular vectors.
pub fn MakeNormalVectors(forward: &Vec3, right: &mut Vec3, up: &mut Vec3) {
    // This rotate and negate guarantees a vector
    // not colinear with the original
    right[1] = -forward[0];
    right[2] = forward[1];
    right[0] = forward[2];

    let d = dot_product(right, forward);
    let right_copy = *right;
    vector_ma(&right_copy, -d, forward, right);
    VectorNormalize(right);
    CrossProduct(right, forward, up);
}

pub fn VectorRotate(in_v: &Vec3, matrix: &[Vec3; 3], out: &mut Vec3) {
    out[0] = dot_product(in_v, &matrix[0]);
    out[1] = dot_product(in_v, &matrix[1]);
    out[2] = dot_product(in_v, &matrix[2]);
}

// ============================================================================

/// Quake III's famous fast inverse square root.
pub fn Q_rsqrt(number: f32) -> f32 {
    let threehalfs: f32 = 1.5;
    let x2 = number * 0.5;
    let mut y = number;

    let mut i: u32 = y.to_bits();
    i = 0x5f3759df_u32.wrapping_sub(i >> 1);
    y = f32::from_bits(i);

    y = y * (threehalfs - (x2 * y * y));
    y
}

pub fn Q_fabs(f: f32) -> f32 {
    let tmp: u32 = f.to_bits() & 0x7FFFFFFF;
    f32::from_bits(tmp)
}

// ============================================================

pub fn LerpAngle(from: f32, mut to: f32, frac: f32) -> f32 {
    if to - from > 180.0 {
        to -= 360.0;
    }
    if to - from < -180.0 {
        to += 360.0;
    }
    from + frac * (to - from)
}

/// Always returns a value from -180 to 180
pub fn AngleSubtract(a1: f32, a2: f32) -> f32 {
    let mut a = a1 - a2;
    while a > 180.0 {
        a -= 360.0;
    }
    while a < -180.0 {
        a += 360.0;
    }
    a
}

pub fn AnglesSubtract(v1: &Vec3, v2: &Vec3, v3: &mut Vec3) {
    v3[0] = AngleSubtract(v1[0], v2[0]);
    v3[1] = AngleSubtract(v1[1], v2[1]);
    v3[2] = AngleSubtract(v1[2], v2[2]);
}

pub fn AngleMod(a: f32) -> f32 {
    (360.0 / 65536.0) * (((a * (65536.0 / 360.0)) as i32) & 65535) as f32
}

/// returns angle normalized to the range [0 <= angle < 360]
pub fn AngleNormalize360(angle: f32) -> f32 {
    (360.0 / 65536.0) * (((angle * (65536.0 / 360.0)) as i32) & 65535) as f32
}

/// returns angle normalized to the range [-180 < angle <= 180]
pub fn AngleNormalize180(angle: f32) -> f32 {
    let mut angle = AngleNormalize360(angle);
    if angle > 180.0 {
        angle -= 360.0;
    }
    angle
}

/// returns the normalized delta from angle1 to angle2
pub fn AngleDelta(angle1: f32, angle2: f32) -> f32 {
    AngleNormalize180(angle1 - angle2)
}

// ============================================================

pub fn SetPlaneSignbits(out: &mut CplaneT) {
    let mut bits: u8 = 0;
    for j in 0..3 {
        if out.normal[j] < 0.0 {
            bits |= 1 << j;
        }
    }
    out.signbits = bits;
}

/// Returns 1, 2, or 1 + 2
pub fn BoxOnPlaneSide(emins: &Vec3, emaxs: &Vec3, p: &CplaneT) -> i32 {
    let dist1: f32;
    let dist2: f32;

    // Fast axial cases
    if p.type_ < 3 {
        let idx = p.type_ as usize;
        if p.dist <= emins[idx] {
            return 1;
        }
        if p.dist >= emaxs[idx] {
            return 2;
        }
        return 3;
    }

    // General case
    match p.signbits {
        0 => {
            dist1 = p.normal[0] * emaxs[0] + p.normal[1] * emaxs[1] + p.normal[2] * emaxs[2];
            dist2 = p.normal[0] * emins[0] + p.normal[1] * emins[1] + p.normal[2] * emins[2];
        }
        1 => {
            dist1 = p.normal[0] * emins[0] + p.normal[1] * emaxs[1] + p.normal[2] * emaxs[2];
            dist2 = p.normal[0] * emaxs[0] + p.normal[1] * emins[1] + p.normal[2] * emins[2];
        }
        2 => {
            dist1 = p.normal[0] * emaxs[0] + p.normal[1] * emins[1] + p.normal[2] * emaxs[2];
            dist2 = p.normal[0] * emins[0] + p.normal[1] * emaxs[1] + p.normal[2] * emins[2];
        }
        3 => {
            dist1 = p.normal[0] * emins[0] + p.normal[1] * emins[1] + p.normal[2] * emaxs[2];
            dist2 = p.normal[0] * emaxs[0] + p.normal[1] * emaxs[1] + p.normal[2] * emins[2];
        }
        4 => {
            dist1 = p.normal[0] * emaxs[0] + p.normal[1] * emaxs[1] + p.normal[2] * emins[2];
            dist2 = p.normal[0] * emins[0] + p.normal[1] * emins[1] + p.normal[2] * emaxs[2];
        }
        5 => {
            dist1 = p.normal[0] * emins[0] + p.normal[1] * emaxs[1] + p.normal[2] * emins[2];
            dist2 = p.normal[0] * emaxs[0] + p.normal[1] * emins[1] + p.normal[2] * emaxs[2];
        }
        6 => {
            dist1 = p.normal[0] * emaxs[0] + p.normal[1] * emins[1] + p.normal[2] * emins[2];
            dist2 = p.normal[0] * emins[0] + p.normal[1] * emaxs[1] + p.normal[2] * emaxs[2];
        }
        7 => {
            dist1 = p.normal[0] * emins[0] + p.normal[1] * emins[1] + p.normal[2] * emins[2];
            dist2 = p.normal[0] * emaxs[0] + p.normal[1] * emaxs[1] + p.normal[2] * emaxs[2];
        }
        _ => {
            dist1 = 0.0;
            dist2 = 0.0;
        }
    }

    let mut sides = 0;
    if dist1 >= p.dist {
        sides = 1;
    }
    if dist2 < p.dist {
        sides |= 2;
    }
    sides
}

pub fn RadiusFromBounds(mins: &Vec3, maxs: &Vec3) -> f32 {
    let mut corner: Vec3 = [0.0; 3];
    for i in 0..3 {
        let a = mins[i].abs();
        let b = maxs[i].abs();
        corner[i] = if a > b { a } else { b };
    }
    VectorLength(&corner)
}

pub fn ClearBounds(mins: &mut Vec3, maxs: &mut Vec3) {
    mins[0] = 99999.0;
    mins[1] = 99999.0;
    mins[2] = 99999.0;
    maxs[0] = -99999.0;
    maxs[1] = -99999.0;
    maxs[2] = -99999.0;
}

pub fn AddPointToBounds(v: &Vec3, mins: &mut Vec3, maxs: &mut Vec3) {
    if v[0] < mins[0] {
        mins[0] = v[0];
    }
    if v[0] > maxs[0] {
        maxs[0] = v[0];
    }

    if v[1] < mins[1] {
        mins[1] = v[1];
    }
    if v[1] > maxs[1] {
        maxs[1] = v[1];
    }

    if v[2] < mins[2] {
        mins[2] = v[2];
    }
    if v[2] > maxs[2] {
        maxs[2] = v[2];
    }
}

pub fn VectorNormalize(v: &mut Vec3) -> f32 {
    let mut length = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    length = length.sqrt();

    if length != 0.0 {
        let ilength = 1.0 / length;
        v[0] *= ilength;
        v[1] *= ilength;
        v[2] *= ilength;
    }
    length
}

pub fn VectorNormalize2(v: &Vec3, out: &mut Vec3) -> f32 {
    let mut length = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    length = length.sqrt();

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

pub fn _VectorMA(veca: &Vec3, scale: f32, vecb: &Vec3, vecc: &mut Vec3) {
    vecc[0] = veca[0] + scale * vecb[0];
    vecc[1] = veca[1] + scale * vecb[1];
    vecc[2] = veca[2] + scale * vecb[2];
}

pub fn _DotProduct(v1: &Vec3, v2: &Vec3) -> f32 {
    v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2]
}

pub fn _VectorSubtract(veca: &Vec3, vecb: &Vec3, out: &mut Vec3) {
    out[0] = veca[0] - vecb[0];
    out[1] = veca[1] - vecb[1];
    out[2] = veca[2] - vecb[2];
}

pub fn _VectorAdd(veca: &Vec3, vecb: &Vec3, out: &mut Vec3) {
    out[0] = veca[0] + vecb[0];
    out[1] = veca[1] + vecb[1];
    out[2] = veca[2] + vecb[2];
}

pub fn _VectorCopy(in_v: &Vec3, out: &mut Vec3) {
    out[0] = in_v[0];
    out[1] = in_v[1];
    out[2] = in_v[2];
}

pub fn _VectorScale(in_v: &Vec3, scale: f32, out: &mut Vec3) {
    out[0] = in_v[0] * scale;
    out[1] = in_v[1] * scale;
    out[2] = in_v[2] * scale;
}

pub fn Vector4Scale(in_v: &Vec4, scale: f32, out: &mut Vec4) {
    out[0] = in_v[0] * scale;
    out[1] = in_v[1] * scale;
    out[2] = in_v[2] * scale;
    out[3] = in_v[3] * scale;
}

pub fn Q_log2(mut val: i32) -> i32 {
    let mut answer = 0;
    val >>= 1;
    while val != 0 {
        answer += 1;
        val >>= 1;
    }
    answer
}

pub fn MatrixMultiply(in1: &[[f32; 3]; 3], in2: &[[f32; 3]; 3], out: &mut [[f32; 3]; 3]) {
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

pub fn AngleVectors(
    angles: &Vec3,
    forward: Option<&mut Vec3>,
    right: Option<&mut Vec3>,
    up: Option<&mut Vec3>,
) {
    let angle = angles[YAW] * (M_PI * 2.0 / 360.0);
    let sy = angle.sin();
    let cy = angle.cos();
    let angle = angles[PITCH] * (M_PI * 2.0 / 360.0);
    let sp = angle.sin();
    let cp = angle.cos();
    let angle = angles[ROLL] * (M_PI * 2.0 / 360.0);
    let sr = angle.sin();
    let cr = angle.cos();

    if let Some(forward) = forward {
        forward[0] = cp * cy;
        forward[1] = cp * sy;
        forward[2] = -sp;
    }
    if let Some(right) = right {
        right[0] = -1.0 * sr * sp * cy + -1.0 * cr * -sy;
        right[1] = -1.0 * sr * sp * sy + -1.0 * cr * cy;
        right[2] = -1.0 * sr * cp;
    }
    if let Some(up) = up {
        up[0] = cr * sp * cy + -sr * -sy;
        up[1] = cr * sp * sy + -sr * cy;
        up[2] = cr * cp;
    }
}

/// Assumes "src" is normalized.
pub fn PerpendicularVector(dst: &mut Vec3, src: &Vec3) {
    let mut pos: usize = 0;
    let mut minelem: f32 = 1.0;
    let mut tempvec: Vec3 = [0.0; 3];

    // Find the smallest magnitude axially aligned vector
    for i in 0..3 {
        if src[i].abs() < minelem {
            pos = i;
            minelem = src[i].abs();
        }
    }
    tempvec[0] = 0.0;
    tempvec[1] = 0.0;
    tempvec[2] = 0.0;
    tempvec[pos] = 1.0;

    // Project the point onto the plane defined by src
    ProjectPointOnPlane(dst, &tempvec, src);

    // Normalize the result
    VectorNormalize(dst);
}
