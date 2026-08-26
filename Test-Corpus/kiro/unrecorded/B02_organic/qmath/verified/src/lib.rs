#![allow(non_upper_case_globals, non_snake_case, unused_assignments)]

use core::ptr;

const M_PI: f32 = 3.14159265358979323846;
const NUMVERTEXNORMALS: usize = 162;
const PITCH: usize = 0;
const YAW: usize = 1;
const ROLL: usize = 2;

#[repr(C)]
pub struct cplane_s {
    pub normal: [f32; 3],
    pub dist: f32,
    pub type_: u8,
    pub signbits: u8,
    pub pad: [u8; 2],
}

// ---- Global data ----

#[no_mangle]
pub static mut vec3_origin: [f32; 3] = [0.0, 0.0, 0.0];

#[no_mangle]
pub static mut axisDefault: [[f32; 3]; 3] = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
];

#[no_mangle]
pub static mut colorBlack: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
#[no_mangle]
pub static mut colorRed: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
#[no_mangle]
pub static mut colorGreen: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
#[no_mangle]
pub static mut colorBlue: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
#[no_mangle]
pub static mut colorYellow: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
#[no_mangle]
pub static mut colorMagenta: [f32; 4] = [1.0, 0.0, 1.0, 1.0];
#[no_mangle]
pub static mut colorCyan: [f32; 4] = [0.0, 1.0, 1.0, 1.0];
#[no_mangle]
pub static mut colorWhite: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
#[no_mangle]
pub static mut colorLtGrey: [f32; 4] = [0.75, 0.75, 0.75, 1.0];
#[no_mangle]
pub static mut colorMdGrey: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
#[no_mangle]
pub static mut colorDkGrey: [f32; 4] = [0.25, 0.25, 0.25, 1.0];

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
pub static bytedirs: [[f32; 3]; NUMVERTEXNORMALS] = [
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

// ---- Inline helpers (not exported, used internally like C macros) ----

#[inline(always)]
unsafe fn dot_product(x: *const f32, y: *const f32) -> f32 {
    *x.add(0) * *y.add(0) + *x.add(1) * *y.add(1) + *x.add(2) * *y.add(2)
}

#[inline(always)]
unsafe fn vector_subtract(a: *const f32, b: *const f32, c: *mut f32) {
    *c.add(0) = *a.add(0) - *b.add(0);
    *c.add(1) = *a.add(1) - *b.add(1);
    *c.add(2) = *a.add(2) - *b.add(2);
}

#[inline(always)]
unsafe fn vector_copy(a: *const f32, b: *mut f32) {
    *b.add(0) = *a.add(0);
    *b.add(1) = *a.add(1);
    *b.add(2) = *a.add(2);
}

#[inline(always)]
unsafe fn vector_clear(a: *mut f32) {
    *a.add(0) = 0.0;
    *a.add(1) = 0.0;
    *a.add(2) = 0.0;
}

#[inline(always)]
unsafe fn vector_ma(v: *const f32, s: f32, b: *const f32, o: *mut f32) {
    *o.add(0) = *v.add(0) + *b.add(0) * s;
    *o.add(1) = *v.add(1) + *b.add(1) * s;
    *o.add(2) = *v.add(2) + *b.add(2) * s;
}

#[inline(always)]
unsafe fn vector_scale(v: *const f32, s: f32, o: *mut f32) {
    *o.add(0) = *v.add(0) * s;
    *o.add(1) = *v.add(1) * s;
    *o.add(2) = *v.add(2) * s;
}

#[inline(always)]
unsafe fn cross_product(v1: *const f32, v2: *const f32, cross: *mut f32) {
    *cross.add(0) = *v1.add(1) * *v2.add(2) - *v1.add(2) * *v2.add(1);
    *cross.add(1) = *v1.add(2) * *v2.add(0) - *v1.add(0) * *v2.add(2);
    *cross.add(2) = *v1.add(0) * *v2.add(1) - *v1.add(1) * *v2.add(0);
}

#[inline(always)]
unsafe fn vector_length(v: *const f32) -> f32 {
    let val = *v.add(0) * *v.add(0) + *v.add(1) * *v.add(1) + *v.add(2) * *v.add(2);
    (val as f64).sqrt() as f32
}

// ---- Exported functions ----

#[no_mangle]
pub unsafe extern "C" fn Q_rand(seed: *mut i32) -> i32 {
    *seed = (69069i32).wrapping_mul(*seed).wrapping_add(1);
    *seed
}

#[no_mangle]
pub unsafe extern "C" fn Q_random(seed: *mut i32) -> f32 {
    (Q_rand(seed) & 0xffff) as f32 / 0x10000 as f32
}

#[no_mangle]
pub unsafe extern "C" fn Q_crandom(seed: *mut i32) -> f32 {
    2.0 * (Q_random(seed) - 0.5)
}

#[no_mangle]
pub unsafe extern "C" fn ClampChar(i: i32) -> i8 {
    if i < -128 { return -128; }
    if i > 127 { return 127; }
    i as i8
}

#[no_mangle]
pub unsafe extern "C" fn ClampShort(i: i32) -> i16 {
    if i < -32768 { return -32768; }
    if i > 0x7fff { return 0x7fff; }
    i as i16
}

#[no_mangle]
pub unsafe extern "C" fn DirToByte(dir: *const f32) -> i32 {
    if dir.is_null() { return 0; }
    let mut bestd: f32 = 0.0;
    let mut best: i32 = 0;
    for i in 0..NUMVERTEXNORMALS {
        let d = dot_product(dir, bytedirs[i].as_ptr());
        if d > bestd {
            bestd = d;
            best = i as i32;
        }
    }
    best
}

#[no_mangle]
pub unsafe extern "C" fn ByteToDir(b: i32, dir: *mut f32) {
    if b < 0 || b >= NUMVERTEXNORMALS as i32 {
        vector_copy(vec3_origin.as_ptr(), dir);
        return;
    }
    vector_copy(bytedirs[b as usize].as_ptr(), dir);
}

#[no_mangle]
pub unsafe extern "C" fn ColorBytes3(r: f32, g: f32, b: f32) -> u32 {
    let mut i: u32 = 0;
    let p = &mut i as *mut u32 as *mut u8;
    *p.add(0) = (r * 255.0) as u8;
    *p.add(1) = (g * 255.0) as u8;
    *p.add(2) = (b * 255.0) as u8;
    i
}

#[no_mangle]
pub unsafe extern "C" fn ColorBytes4(r: f32, g: f32, b: f32, a: f32) -> u32 {
    let mut i: u32 = 0;
    let p = &mut i as *mut u32 as *mut u8;
    *p.add(0) = (r * 255.0) as u8;
    *p.add(1) = (g * 255.0) as u8;
    *p.add(2) = (b * 255.0) as u8;
    *p.add(3) = (a * 255.0) as u8;
    i
}

#[no_mangle]
pub unsafe extern "C" fn NormalizeColor(in_: *const f32, out: *mut f32) -> f32 {
    let mut max = *in_.add(0);
    if *in_.add(1) > max { max = *in_.add(1); }
    if *in_.add(2) > max { max = *in_.add(2); }
    if max == 0.0 {
        vector_clear(out);
    } else {
        *out.add(0) = *in_.add(0) / max;
        *out.add(1) = *in_.add(1) / max;
        *out.add(2) = *in_.add(2) / max;
    }
    max
}

#[no_mangle]
pub unsafe extern "C" fn PlaneFromPoints(
    plane: *mut f32, a: *const f32, b: *const f32, c: *const f32,
) -> i32 {
    let mut d1: [f32; 3] = [0.0; 3];
    let mut d2: [f32; 3] = [0.0; 3];
    vector_subtract(b, a, d1.as_mut_ptr());
    vector_subtract(c, a, d2.as_mut_ptr());
    cross_product(d2.as_ptr(), d1.as_ptr(), plane);
    if VectorNormalize(plane) == 0.0 {
        return 0; // qfalse
    }
    *plane.add(3) = dot_product(a, plane);
    1 // qtrue
}

#[no_mangle]
pub unsafe extern "C" fn RotatePointAroundVector(
    dst: *mut f32, dir: *const f32, point: *const f32, degrees: f32,
) {
    let mut m: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut im: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut zrot: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut tmpmat: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut rot: [[f32; 3]; 3] = [[0.0; 3]; 3];
    let mut vr: [f32; 3] = [0.0; 3];
    let mut vup: [f32; 3] = [0.0; 3];
    let mut vf: [f32; 3] = [0.0; 3];

    vf[0] = *dir.add(0);
    vf[1] = *dir.add(1);
    vf[2] = *dir.add(2);

    PerpendicularVector(vr.as_mut_ptr(), dir);
    cross_product(vr.as_ptr(), vf.as_ptr(), vup.as_mut_ptr());

    m[0][0] = vr[0]; m[1][0] = vr[1]; m[2][0] = vr[2];
    m[0][1] = vup[0]; m[1][1] = vup[1]; m[2][1] = vup[2];
    m[0][2] = vf[0]; m[1][2] = vf[1]; m[2][2] = vf[2];

    ptr::copy_nonoverlapping(m.as_ptr(), im.as_mut_ptr(), 3);

    im[0][1] = m[1][0]; im[0][2] = m[2][0];
    im[1][0] = m[0][1]; im[1][2] = m[2][1];
    im[2][0] = m[0][2]; im[2][1] = m[1][2];

    zrot[0][0] = 1.0; zrot[1][1] = 1.0; zrot[2][2] = 1.0;

    let rad = (degrees * M_PI / 180.0f32) as f64;
    zrot[0][0] = rad.cos() as f32;
    zrot[0][1] = rad.sin() as f32;
    zrot[1][0] = -(rad.sin() as f32);
    zrot[1][1] = rad.cos() as f32;

    MatrixMultiply(
        m.as_mut_ptr() as *mut [f32; 3],
        zrot.as_mut_ptr() as *mut [f32; 3],
        tmpmat.as_mut_ptr() as *mut [f32; 3],
    );
    MatrixMultiply(
        tmpmat.as_mut_ptr() as *mut [f32; 3],
        im.as_mut_ptr() as *mut [f32; 3],
        rot.as_mut_ptr() as *mut [f32; 3],
    );

    for i in 0..3 {
        *dst.add(i) = rot[i][0] * *point.add(0)
            + rot[i][1] * *point.add(1)
            + rot[i][2] * *point.add(2);
    }
}

#[no_mangle]
pub unsafe extern "C" fn RotateAroundDirection(axis: *mut [f32; 3], yaw: f32) {
    PerpendicularVector((*axis.add(1)).as_mut_ptr(), (*axis.add(0)).as_ptr());
    if yaw != 0.0 {
        let mut temp: [f32; 3] = [0.0; 3];
        vector_copy((*axis.add(1)).as_ptr(), temp.as_mut_ptr());
        RotatePointAroundVector(
            (*axis.add(1)).as_mut_ptr(),
            (*axis.add(0)).as_ptr(),
            temp.as_ptr(),
            yaw,
        );
    }
    cross_product(
        (*axis.add(0)).as_ptr(),
        (*axis.add(1)).as_ptr(),
        (*axis.add(2)).as_mut_ptr(),
    );
}

#[no_mangle]
pub unsafe extern "C" fn vectoangles(value1: *const f32, angles: *mut f32) {
    let mut yaw: f32;
    let mut pitch: f32;

    if *value1.add(1) == 0.0 && *value1.add(0) == 0.0 {
        yaw = 0.0;
        if *value1.add(2) > 0.0 {
            pitch = 90.0;
        } else {
            pitch = 270.0;
        }
    } else {
        if *value1.add(0) != 0.0 {
            yaw = ((*value1.add(1) as f64).atan2(*value1.add(0) as f64) * 180.0
                / (M_PI as f64)) as f32;
        } else if *value1.add(1) > 0.0 {
            yaw = 90.0;
        } else {
            yaw = 270.0;
        }
        if yaw < 0.0 {
            yaw += 360.0;
        }
        let forward = ((*value1.add(0) * *value1.add(0)
            + *value1.add(1) * *value1.add(1)) as f64)
            .sqrt() as f32;
        pitch = ((*value1.add(2) as f64).atan2(forward as f64) * 180.0
            / (M_PI as f64)) as f32;
        if pitch < 0.0 {
            pitch += 360.0;
        }
    }

    *angles.add(PITCH) = -pitch;
    *angles.add(YAW) = yaw;
    *angles.add(ROLL) = 0.0;
}

#[no_mangle]
pub unsafe extern "C" fn AnglesToAxis(angles: *const f32, axis: *mut [f32; 3]) {
    let mut right: [f32; 3] = [0.0; 3];
    AngleVectors(
        angles,
        (*axis.add(0)).as_mut_ptr(),
        right.as_mut_ptr(),
        (*axis.add(2)).as_mut_ptr(),
    );
    // VectorSubtract(vec3_origin, right, axis[1])
    (*axis.add(1))[0] = vec3_origin[0] - right[0];
    (*axis.add(1))[1] = vec3_origin[1] - right[1];
    (*axis.add(1))[2] = vec3_origin[2] - right[2];
}

#[no_mangle]
pub unsafe extern "C" fn AxisClear(axis: *mut [f32; 3]) {
    (*axis.add(0))[0] = 1.0; (*axis.add(0))[1] = 0.0; (*axis.add(0))[2] = 0.0;
    (*axis.add(1))[0] = 0.0; (*axis.add(1))[1] = 1.0; (*axis.add(1))[2] = 0.0;
    (*axis.add(2))[0] = 0.0; (*axis.add(2))[1] = 0.0; (*axis.add(2))[2] = 1.0;
}

#[no_mangle]
pub unsafe extern "C" fn AxisCopy(in_: *mut [f32; 3], out: *mut [f32; 3]) {
    vector_copy((*in_.add(0)).as_ptr(), (*out.add(0)).as_mut_ptr());
    vector_copy((*in_.add(1)).as_ptr(), (*out.add(1)).as_mut_ptr());
    vector_copy((*in_.add(2)).as_ptr(), (*out.add(2)).as_mut_ptr());
}

#[no_mangle]
pub unsafe extern "C" fn ProjectPointOnPlane(
    dst: *mut f32, p: *const f32, normal: *const f32,
) {
    let inv_denom = 1.0f32 / dot_product(normal, normal);
    let d = dot_product(normal, p) * inv_denom;
    let mut n: [f32; 3] = [0.0; 3];
    n[0] = *normal.add(0) * inv_denom;
    n[1] = *normal.add(1) * inv_denom;
    n[2] = *normal.add(2) * inv_denom;
    *dst.add(0) = *p.add(0) - d * n[0];
    *dst.add(1) = *p.add(1) - d * n[1];
    *dst.add(2) = *p.add(2) - d * n[2];
}

#[no_mangle]
pub unsafe extern "C" fn MakeNormalVectors(
    forward: *const f32, right: *mut f32, up: *mut f32,
) {
    *right.add(1) = -*forward.add(0);
    *right.add(2) = *forward.add(1);
    *right.add(0) = *forward.add(2);
    let d = dot_product(right, forward);
    vector_ma(right, -d, forward, right);
    VectorNormalize(right);
    cross_product(right, forward, up);
}

#[no_mangle]
pub unsafe extern "C" fn VectorRotate(
    in_: *mut f32, matrix: *mut [f32; 3], out: *mut f32,
) {
    *out.add(0) = dot_product(in_, (*matrix.add(0)).as_ptr());
    *out.add(1) = dot_product(in_, (*matrix.add(1)).as_ptr());
    *out.add(2) = dot_product(in_, (*matrix.add(2)).as_ptr());
}

#[no_mangle]
pub unsafe extern "C" fn Q_rsqrt(number: f32) -> f32 {
    let x2 = number * 0.5f32;
    let mut y = number;
    let mut i: u32 = y.to_bits();
    i = 0x5f3759dfu32.wrapping_sub(i >> 1);
    y = f32::from_bits(i);
    y = y * (1.5f32 - (x2 * y * y));
    y
}

#[no_mangle]
pub unsafe extern "C" fn Q_fabs(f: f32) -> f32 {
    let mut tmp: i32 = f.to_bits() as i32;
    tmp &= 0x7FFFFFFF;
    f32::from_bits(tmp as u32)
}

#[no_mangle]
pub unsafe extern "C" fn LerpAngle(from: f32, mut to: f32, frac: f32) -> f32 {
    if to - from > 180.0 { to -= 360.0; }
    if to - from < -180.0 { to += 360.0; }
    from + frac * (to - from)
}

#[no_mangle]
pub unsafe extern "C" fn AngleSubtract(a1: f32, a2: f32) -> f32 {
    let mut a = a1 - a2;
    while a > 180.0 { a -= 360.0; }
    while a < -180.0 { a += 360.0; }
    a
}

#[no_mangle]
pub unsafe extern "C" fn AnglesSubtract(v1: *mut f32, v2: *mut f32, v3: *mut f32) {
    *v3.add(0) = AngleSubtract(*v1.add(0), *v2.add(0));
    *v3.add(1) = AngleSubtract(*v1.add(1), *v2.add(1));
    *v3.add(2) = AngleSubtract(*v1.add(2), *v2.add(2));
}

#[no_mangle]
pub unsafe extern "C" fn AngleMod(a: f32) -> f32 {
    // C: (360.0/65536) * ((int)(a*(65536/360.0)) & 65535)
    let val = (a as f64) * (65536.0f64 / 360.0f64);
    ((360.0f64 / 65536.0f64) * ((val as i32 & 65535) as f64)) as f32
}

#[no_mangle]
pub unsafe extern "C" fn AngleNormalize360(angle: f32) -> f32 {
    let val = (angle as f64) * (65536.0f64 / 360.0f64);
    ((360.0f64 / 65536.0f64) * ((val as i32 & 65535) as f64)) as f32
}

#[no_mangle]
pub unsafe extern "C" fn AngleNormalize180(angle: f32) -> f32 {
    let mut a = AngleNormalize360(angle);
    if a > 180.0 { a -= 360.0; }
    a
}

#[no_mangle]
pub unsafe extern "C" fn AngleDelta(angle1: f32, angle2: f32) -> f32 {
    AngleNormalize180(angle1 - angle2)
}

#[no_mangle]
pub unsafe extern "C" fn SetPlaneSignbits(out: *mut cplane_s) {
    let mut bits: i32 = 0;
    for j in 0..3 {
        if (*out).normal[j] < 0.0 {
            bits |= 1 << j;
        }
    }
    (*out).signbits = bits as u8;
}

#[no_mangle]
pub unsafe extern "C" fn BoxOnPlaneSide(
    emins: *mut f32, emaxs: *mut f32, p: *mut cplane_s,
) -> i32 {
    let dist1: f32;
    let dist2: f32;

    // fast axial cases
    if (*p).type_ < 3 {
        let t = (*p).type_ as usize;
        if (*p).dist <= *emins.add(t) { return 1; }
        if (*p).dist >= *emaxs.add(t) { return 2; }
        return 3;
    }

    match (*p).signbits {
        0 => {
            dist1 = (*p).normal[0]* *emaxs.add(0) + (*p).normal[1]* *emaxs.add(1) + (*p).normal[2]* *emaxs.add(2);
            dist2 = (*p).normal[0]* *emins.add(0) + (*p).normal[1]* *emins.add(1) + (*p).normal[2]* *emins.add(2);
        }
        1 => {
            dist1 = (*p).normal[0]* *emins.add(0) + (*p).normal[1]* *emaxs.add(1) + (*p).normal[2]* *emaxs.add(2);
            dist2 = (*p).normal[0]* *emaxs.add(0) + (*p).normal[1]* *emins.add(1) + (*p).normal[2]* *emins.add(2);
        }
        2 => {
            dist1 = (*p).normal[0]* *emaxs.add(0) + (*p).normal[1]* *emins.add(1) + (*p).normal[2]* *emaxs.add(2);
            dist2 = (*p).normal[0]* *emins.add(0) + (*p).normal[1]* *emaxs.add(1) + (*p).normal[2]* *emins.add(2);
        }
        3 => {
            dist1 = (*p).normal[0]* *emins.add(0) + (*p).normal[1]* *emins.add(1) + (*p).normal[2]* *emaxs.add(2);
            dist2 = (*p).normal[0]* *emaxs.add(0) + (*p).normal[1]* *emaxs.add(1) + (*p).normal[2]* *emins.add(2);
        }
        4 => {
            dist1 = (*p).normal[0]* *emaxs.add(0) + (*p).normal[1]* *emaxs.add(1) + (*p).normal[2]* *emins.add(2);
            dist2 = (*p).normal[0]* *emins.add(0) + (*p).normal[1]* *emins.add(1) + (*p).normal[2]* *emaxs.add(2);
        }
        5 => {
            dist1 = (*p).normal[0]* *emins.add(0) + (*p).normal[1]* *emaxs.add(1) + (*p).normal[2]* *emins.add(2);
            dist2 = (*p).normal[0]* *emaxs.add(0) + (*p).normal[1]* *emins.add(1) + (*p).normal[2]* *emaxs.add(2);
        }
        6 => {
            dist1 = (*p).normal[0]* *emaxs.add(0) + (*p).normal[1]* *emins.add(1) + (*p).normal[2]* *emins.add(2);
            dist2 = (*p).normal[0]* *emins.add(0) + (*p).normal[1]* *emaxs.add(1) + (*p).normal[2]* *emaxs.add(2);
        }
        7 => {
            dist1 = (*p).normal[0]* *emins.add(0) + (*p).normal[1]* *emins.add(1) + (*p).normal[2]* *emins.add(2);
            dist2 = (*p).normal[0]* *emaxs.add(0) + (*p).normal[1]* *emaxs.add(1) + (*p).normal[2]* *emaxs.add(2);
        }
        _ => {
            dist1 = 0.0;
            dist2 = 0.0;
        }
    }

    let mut sides: i32 = 0;
    if dist1 >= (*p).dist { sides = 1; }
    if dist2 < (*p).dist { sides |= 2; }
    sides
}

#[no_mangle]
pub unsafe extern "C" fn RadiusFromBounds(mins: *const f32, maxs: *const f32) -> f32 {
    let mut corner: [f32; 3] = [0.0; 3];
    for i in 0..3 {
        let a = (*mins.add(i) as f64).abs() as f32;
        let b = (*maxs.add(i) as f64).abs() as f32;
        corner[i] = if a > b { a } else { b };
    }
    vector_length(corner.as_ptr())
}

#[no_mangle]
pub unsafe extern "C" fn ClearBounds(mins: *mut f32, maxs: *mut f32) {
    *mins.add(0) = 99999.0; *mins.add(1) = 99999.0; *mins.add(2) = 99999.0;
    *maxs.add(0) = -99999.0; *maxs.add(1) = -99999.0; *maxs.add(2) = -99999.0;
}

#[no_mangle]
pub unsafe extern "C" fn AddPointToBounds(v: *const f32, mins: *mut f32, maxs: *mut f32) {
    if *v.add(0) < *mins.add(0) { *mins.add(0) = *v.add(0); }
    if *v.add(0) > *maxs.add(0) { *maxs.add(0) = *v.add(0); }
    if *v.add(1) < *mins.add(1) { *mins.add(1) = *v.add(1); }
    if *v.add(1) > *maxs.add(1) { *maxs.add(1) = *v.add(1); }
    if *v.add(2) < *mins.add(2) { *mins.add(2) = *v.add(2); }
    if *v.add(2) > *maxs.add(2) { *maxs.add(2) = *v.add(2); }
}

#[no_mangle]
pub unsafe extern "C" fn VectorNormalize(v: *mut f32) -> f32 {
    let mut length = *v.add(0) * *v.add(0) + *v.add(1) * *v.add(1) + *v.add(2) * *v.add(2);
    length = (length as f64).sqrt() as f32;
    if length != 0.0 {
        let ilength = 1.0f32 / length;
        *v.add(0) *= ilength;
        *v.add(1) *= ilength;
        *v.add(2) *= ilength;
    }
    length
}

#[no_mangle]
pub unsafe extern "C" fn VectorNormalize2(v: *const f32, out: *mut f32) -> f32 {
    let mut length = *v.add(0) * *v.add(0) + *v.add(1) * *v.add(1) + *v.add(2) * *v.add(2);
    length = (length as f64).sqrt() as f32;
    if length != 0.0 {
        let ilength = 1.0f32 / length;
        *out.add(0) = *v.add(0) * ilength;
        *out.add(1) = *v.add(1) * ilength;
        *out.add(2) = *v.add(2) * ilength;
    } else {
        vector_clear(out);
    }
    length
}

#[no_mangle]
pub unsafe extern "C" fn _VectorMA(
    veca: *const f32, scale: f32, vecb: *const f32, vecc: *mut f32,
) {
    *vecc.add(0) = *veca.add(0) + scale * *vecb.add(0);
    *vecc.add(1) = *veca.add(1) + scale * *vecb.add(1);
    *vecc.add(2) = *veca.add(2) + scale * *vecb.add(2);
}

#[no_mangle]
pub unsafe extern "C" fn _DotProduct(v1: *const f32, v2: *const f32) -> f32 {
    *v1.add(0) * *v2.add(0) + *v1.add(1) * *v2.add(1) + *v1.add(2) * *v2.add(2)
}

#[no_mangle]
pub unsafe extern "C" fn _VectorSubtract(
    veca: *const f32, vecb: *const f32, out: *mut f32,
) {
    *out.add(0) = *veca.add(0) - *vecb.add(0);
    *out.add(1) = *veca.add(1) - *vecb.add(1);
    *out.add(2) = *veca.add(2) - *vecb.add(2);
}

#[no_mangle]
pub unsafe extern "C" fn _VectorAdd(
    veca: *const f32, vecb: *const f32, out: *mut f32,
) {
    *out.add(0) = *veca.add(0) + *vecb.add(0);
    *out.add(1) = *veca.add(1) + *vecb.add(1);
    *out.add(2) = *veca.add(2) + *vecb.add(2);
}

#[no_mangle]
pub unsafe extern "C" fn _VectorCopy(in_: *const f32, out: *mut f32) {
    *out.add(0) = *in_.add(0);
    *out.add(1) = *in_.add(1);
    *out.add(2) = *in_.add(2);
}

#[no_mangle]
pub unsafe extern "C" fn _VectorScale(in_: *const f32, scale: f32, out: *mut f32) {
    *out.add(0) = *in_.add(0) * scale;
    *out.add(1) = *in_.add(1) * scale;
    *out.add(2) = *in_.add(2) * scale;
}

#[no_mangle]
pub unsafe extern "C" fn Vector4Scale(in_: *const f32, scale: f32, out: *mut f32) {
    *out.add(0) = *in_.add(0) * scale;
    *out.add(1) = *in_.add(1) * scale;
    *out.add(2) = *in_.add(2) * scale;
    *out.add(3) = *in_.add(3) * scale;
}

#[no_mangle]
pub unsafe extern "C" fn Q_log2(mut val: i32) -> i32 {
    let mut answer: i32 = 0;
    val >>= 1;
    while val != 0 {
        answer += 1;
        val >>= 1;
    }
    answer
}

#[no_mangle]
pub unsafe extern "C" fn MatrixMultiply(
    in1: *mut [f32; 3], in2: *mut [f32; 3], out: *mut [f32; 3],
) {
    (*out.add(0))[0] = (*in1.add(0))[0] * (*in2.add(0))[0] + (*in1.add(0))[1] * (*in2.add(1))[0] + (*in1.add(0))[2] * (*in2.add(2))[0];
    (*out.add(0))[1] = (*in1.add(0))[0] * (*in2.add(0))[1] + (*in1.add(0))[1] * (*in2.add(1))[1] + (*in1.add(0))[2] * (*in2.add(2))[1];
    (*out.add(0))[2] = (*in1.add(0))[0] * (*in2.add(0))[2] + (*in1.add(0))[1] * (*in2.add(1))[2] + (*in1.add(0))[2] * (*in2.add(2))[2];
    (*out.add(1))[0] = (*in1.add(1))[0] * (*in2.add(0))[0] + (*in1.add(1))[1] * (*in2.add(1))[0] + (*in1.add(1))[2] * (*in2.add(2))[0];
    (*out.add(1))[1] = (*in1.add(1))[0] * (*in2.add(0))[1] + (*in1.add(1))[1] * (*in2.add(1))[1] + (*in1.add(1))[2] * (*in2.add(2))[1];
    (*out.add(1))[2] = (*in1.add(1))[0] * (*in2.add(0))[2] + (*in1.add(1))[1] * (*in2.add(1))[2] + (*in1.add(1))[2] * (*in2.add(2))[2];
    (*out.add(2))[0] = (*in1.add(2))[0] * (*in2.add(0))[0] + (*in1.add(2))[1] * (*in2.add(1))[0] + (*in1.add(2))[2] * (*in2.add(2))[0];
    (*out.add(2))[1] = (*in1.add(2))[0] * (*in2.add(0))[1] + (*in1.add(2))[1] * (*in2.add(1))[1] + (*in1.add(2))[2] * (*in2.add(2))[1];
    (*out.add(2))[2] = (*in1.add(2))[0] * (*in2.add(0))[2] + (*in1.add(2))[1] * (*in2.add(1))[2] + (*in1.add(2))[2] * (*in2.add(2))[2];
}

#[no_mangle]
pub unsafe extern "C" fn AngleVectors(
    angles: *const f32, forward: *mut f32, right: *mut f32, up: *mut f32,
) {
    let angle_yaw = *angles.add(YAW) * (M_PI * 2.0 / 360.0);
    let sy = (angle_yaw as f64).sin() as f32;
    let cy = (angle_yaw as f64).cos() as f32;
    let angle_pitch = *angles.add(PITCH) * (M_PI * 2.0 / 360.0);
    let sp = (angle_pitch as f64).sin() as f32;
    let cp = (angle_pitch as f64).cos() as f32;
    let angle_roll = *angles.add(ROLL) * (M_PI * 2.0 / 360.0);
    let sr = (angle_roll as f64).sin() as f32;
    let cr = (angle_roll as f64).cos() as f32;

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

#[no_mangle]
pub unsafe extern "C" fn PerpendicularVector(dst: *mut f32, src: *const f32) {
    let mut pos: usize = 0;
    let mut minelem: f32 = 1.0;
    let mut tempvec: [f32; 3] = [0.0; 3];

    for i in 0..3 {
        if ((*src.add(i) as f64).abs() as f32) < minelem {
            pos = i;
            minelem = (*src.add(i) as f64).abs() as f32;
        }
    }
    tempvec[pos] = 1.0;

    ProjectPointOnPlane(dst, tempvec.as_ptr(), src);
    VectorNormalize(dst);
}
