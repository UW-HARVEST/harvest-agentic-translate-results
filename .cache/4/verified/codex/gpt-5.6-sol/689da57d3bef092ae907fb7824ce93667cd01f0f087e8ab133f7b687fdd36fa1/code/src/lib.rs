#![allow(non_snake_case)]

use std::ffi::{c_double, c_float, c_int, c_schar, c_short, c_uint, c_void};

const NUMVERTEXNORMALS: usize = 162;
const M_PI: f64 = 3.14159265358979323846_f64;

unsafe extern "C" {
    fn atan2(y: c_double, x: c_double) -> c_double;
    fn cos(x: c_double) -> c_double;
    fn sin(x: c_double) -> c_double;
    fn sqrt(x: c_double) -> c_double;
    fn memset(destination: *mut c_void, value: c_int, count: usize) -> *mut c_void;
}

macro_rules! require_pointers {
    ($($pointer:expr),+ $(,)?) => {
        $(
            if ($pointer).is_null() {
                memset(std::ptr::null_mut(), 0, 1);
            }
        )+
    };
}

#[repr(C)]
pub struct CPlane {
    pub normal: [f32; 3],
    pub dist: f32,
    pub plane_type: u8,
    pub signbits: u8,
    pub pad: [u8; 2],
}

#[no_mangle]
pub static mut vec3_origin: [f32; 3] = [0.0; 3];
#[no_mangle]
pub static mut axisDefault: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

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
pub static mut bytedirs: [[f32; 3]; NUMVERTEXNORMALS] = include!("bytedirs.rs");

#[inline]
unsafe fn read3(ptr: *const f32) -> [f32; 3] {
    [*ptr, *ptr.add(1), *ptr.add(2)]
}

#[inline]
unsafe fn write3(ptr: *mut f32, value: [f32; 3]) {
    *ptr = value[0];
    *ptr.add(1) = value[1];
    *ptr.add(2) = value[2];
}

#[inline]
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
fn c_sqrt(value: f32) -> f32 {
    unsafe { sqrt(value as f64) as f32 }
}

#[inline]
fn matrix_multiply_values(a: [[f32; 3]; 3], b: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut out = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            out[row][column] =
                a[row][0] * b[0][column] + a[row][1] * b[1][column] + a[row][2] * b[2][column];
        }
    }
    out
}

#[no_mangle]
pub unsafe extern "C" fn Q_rand(seed: *mut c_int) -> c_int {
    require_pointers!(seed);
    let next = (*seed as u32).wrapping_mul(69069).wrapping_add(1) as i32;
    *seed = next;
    next
}

#[no_mangle]
pub unsafe extern "C" fn Q_random(seed: *mut c_int) -> c_float {
    (Q_rand(seed) & 0xffff) as f32 / 0x10000 as f32
}

#[no_mangle]
pub unsafe extern "C" fn Q_crandom(seed: *mut c_int) -> c_float {
    2.0 * (Q_random(seed) - 0.5)
}

#[no_mangle]
pub extern "C" fn ClampChar(i: c_int) -> c_schar {
    if i < -128 {
        -128
    } else if i > 127 {
        127
    } else {
        i as i8
    }
}

#[no_mangle]
pub extern "C" fn ClampShort(i: c_int) -> c_short {
    if i < -32768 {
        -32768
    } else if i > 0x7fff {
        0x7fff
    } else {
        i as i16
    }
}

#[no_mangle]
pub unsafe extern "C" fn DirToByte(dir: *const c_float) -> c_int {
    if dir.is_null() {
        return 0;
    }
    let direction = read3(dir);
    let mut bestd = 0.0;
    let mut best = 0;
    for i in 0..NUMVERTEXNORMALS {
        let d = dot(direction, bytedirs[i]);
        if d > bestd {
            bestd = d;
            best = i as i32;
        }
    }
    best
}

#[no_mangle]
pub unsafe extern "C" fn ByteToDir(b: c_int, dir: *mut c_float) {
    require_pointers!(dir);
    if b < 0 || b >= NUMVERTEXNORMALS as i32 {
        write3(dir, [0.0; 3]);
        return;
    }
    write3(dir, bytedirs[b as usize]);
}

#[inline]
fn color_byte(value: f32) -> u8 {
    (value * 255.0) as i32 as u8
}

#[no_mangle]
pub extern "C" fn ColorBytes3(r: c_float, g: c_float, b: c_float) -> c_uint {
    u32::from_ne_bytes([color_byte(r), color_byte(g), color_byte(b), 0])
}

#[no_mangle]
pub extern "C" fn ColorBytes4(r: c_float, g: c_float, b: c_float, a: c_float) -> c_uint {
    u32::from_ne_bytes([color_byte(r), color_byte(g), color_byte(b), color_byte(a)])
}

#[no_mangle]
pub unsafe extern "C" fn NormalizeColor(input: *const c_float, out: *mut c_float) -> c_float {
    require_pointers!(input, out);
    let input = read3(input);
    let mut max = input[0];
    if input[1] > max {
        max = input[1];
    }
    if input[2] > max {
        max = input[2];
    }
    if max == 0.0 {
        write3(out, [0.0; 3]);
    } else {
        write3(out, [input[0] / max, input[1] / max, input[2] / max]);
    }
    max
}

#[no_mangle]
pub unsafe extern "C" fn PlaneFromPoints(
    plane: *mut c_float,
    a: *const c_float,
    b: *const c_float,
    c: *const c_float,
) -> c_int {
    require_pointers!(plane, a, b, c);
    let a = read3(a);
    let b = read3(b);
    let c = read3(c);
    let d1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let d2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    write3(plane, cross(d2, d1));
    if VectorNormalize(plane) == 0.0 {
        return 0;
    }
    *plane.add(3) = dot(a, read3(plane));
    1
}

#[no_mangle]
pub unsafe extern "C" fn ProjectPointOnPlane(
    dst: *mut c_float,
    point: *const c_float,
    normal: *const c_float,
) {
    require_pointers!(dst, point, normal);
    let point = read3(point);
    let normal = read3(normal);
    let inv_denom = 1.0 / dot(normal, normal);
    let d = dot(normal, point) * inv_denom;
    let n = [
        normal[0] * inv_denom,
        normal[1] * inv_denom,
        normal[2] * inv_denom,
    ];
    write3(
        dst,
        [
            point[0] - d * n[0],
            point[1] - d * n[1],
            point[2] - d * n[2],
        ],
    );
}

#[no_mangle]
pub unsafe extern "C" fn RotatePointAroundVector(
    dst: *mut c_float,
    dir: *const c_float,
    point: *const c_float,
    degrees: c_float,
) {
    require_pointers!(dst, dir, point);
    let direction = read3(dir);
    let point = read3(point);
    let mut vr = [0.0; 3];
    PerpendicularVector(vr.as_mut_ptr(), direction.as_ptr());
    let vup = cross(vr, direction);

    let m = [
        [vr[0], vup[0], direction[0]],
        [vr[1], vup[1], direction[1]],
        [vr[2], vup[2], direction[2]],
    ];
    let im = [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ];
    let rad = (degrees as f64 * M_PI / 180.0) as f32;
    let sine = sin(rad as f64) as f32;
    let cosine = cos(rad as f64) as f32;
    let zrot = [[cosine, sine, 0.0], [-sine, cosine, 0.0], [0.0, 0.0, 1.0]];
    let tmpmat = matrix_multiply_values(m, zrot);
    let rot = matrix_multiply_values(tmpmat, im);
    write3(
        dst,
        [
            rot[0][0] * point[0] + rot[0][1] * point[1] + rot[0][2] * point[2],
            rot[1][0] * point[0] + rot[1][1] * point[1] + rot[1][2] * point[2],
            rot[2][0] * point[0] + rot[2][1] * point[1] + rot[2][2] * point[2],
        ],
    );
}

#[no_mangle]
pub unsafe extern "C" fn RotateAroundDirection(axis: *mut [c_float; 3], yaw: c_float) {
    require_pointers!(axis);
    let direction = *axis;
    PerpendicularVector((*axis.add(1)).as_mut_ptr(), direction.as_ptr());
    if yaw != 0.0 {
        let temp = *axis.add(1);
        RotatePointAroundVector(
            (*axis.add(1)).as_mut_ptr(),
            direction.as_ptr(),
            temp.as_ptr(),
            yaw,
        );
    }
    *axis.add(2) = cross(direction, *axis.add(1));
}

#[no_mangle]
pub unsafe extern "C" fn vectoangles(value1: *const c_float, angles: *mut c_float) {
    require_pointers!(value1, angles);
    let value = read3(value1);
    let (mut yaw, mut pitch);
    if value[1] == 0.0 && value[0] == 0.0 {
        yaw = 0.0;
        pitch = if value[2] > 0.0 { 90.0 } else { 270.0 };
    } else {
        if value[0] != 0.0 {
            yaw = (atan2(value[1] as f64, value[0] as f64) * 180.0 / M_PI) as f32;
        } else if value[1] > 0.0 {
            yaw = 90.0;
        } else {
            yaw = 270.0;
        }
        if yaw < 0.0 {
            yaw += 360.0;
        }
        let forward = c_sqrt(value[0] * value[0] + value[1] * value[1]);
        pitch = (atan2(value[2] as f64, forward as f64) * 180.0 / M_PI) as f32;
        if pitch < 0.0 {
            pitch += 360.0;
        }
    }
    write3(angles, [-pitch, yaw, 0.0]);
}

#[no_mangle]
pub unsafe extern "C" fn AnglesToAxis(angles: *const c_float, axis: *mut [c_float; 3]) {
    require_pointers!(angles, axis);
    let mut right = [0.0; 3];
    AngleVectors(
        angles,
        (*axis).as_mut_ptr(),
        right.as_mut_ptr(),
        (*axis.add(2)).as_mut_ptr(),
    );
    *axis.add(1) = [-right[0], -right[1], -right[2]];
}

#[no_mangle]
pub unsafe extern "C" fn AxisClear(axis: *mut [c_float; 3]) {
    require_pointers!(axis);
    *axis = [1.0, 0.0, 0.0];
    *axis.add(1) = [0.0, 1.0, 0.0];
    *axis.add(2) = [0.0, 0.0, 1.0];
}

#[no_mangle]
pub unsafe extern "C" fn AxisCopy(input: *const [c_float; 3], out: *mut [c_float; 3]) {
    require_pointers!(input, out);
    *out = *input;
    *out.add(1) = *input.add(1);
    *out.add(2) = *input.add(2);
}

#[no_mangle]
pub unsafe extern "C" fn MakeNormalVectors(
    forward: *const c_float,
    right: *mut c_float,
    up: *mut c_float,
) {
    require_pointers!(forward, right, up);
    let forward = read3(forward);
    let mut result = [forward[2], -forward[0], forward[1]];
    let d = dot(result, forward);
    result = [
        result[0] + -d * forward[0],
        result[1] + -d * forward[1],
        result[2] + -d * forward[2],
    ];
    VectorNormalize(result.as_mut_ptr());
    write3(right, result);
    write3(up, cross(result, forward));
}

#[no_mangle]
pub unsafe extern "C" fn VectorRotate(
    input: *const c_float,
    matrix: *const [c_float; 3],
    out: *mut c_float,
) {
    require_pointers!(input, matrix, out);
    let input = read3(input);
    write3(
        out,
        [
            dot(input, *matrix),
            dot(input, *matrix.add(1)),
            dot(input, *matrix.add(2)),
        ],
    );
}

#[no_mangle]
pub extern "C" fn Q_rsqrt(number: c_float) -> c_float {
    let x2 = number * 0.5;
    let mut y = number;
    let bits = 0x5f3759df_u32.wrapping_sub(y.to_bits() >> 1);
    y = f32::from_bits(bits);
    y = y * (1.5 - x2 * y * y);
    y
}

#[no_mangle]
pub extern "C" fn Q_fabs(f: c_float) -> c_float {
    f32::from_bits(f.to_bits() & 0x7fff_ffff)
}

#[no_mangle]
pub extern "C" fn LerpAngle(from: c_float, mut to: c_float, frac: c_float) -> c_float {
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
pub unsafe extern "C" fn AnglesSubtract(v1: *const c_float, v2: *const c_float, v3: *mut c_float) {
    require_pointers!(v1, v2, v3);
    let v1 = read3(v1);
    let v2 = read3(v2);
    write3(
        v3,
        [
            AngleSubtract(v1[0], v2[0]),
            AngleSubtract(v1[1], v2[1]),
            AngleSubtract(v1[2], v2[2]),
        ],
    );
}

#[inline]
fn angle_to_short_domain(angle: f32) -> i32 {
    (angle as f64 * (65536.0_f64 / 360.0_f64)) as i32
}

#[no_mangle]
pub extern "C" fn AngleMod(a: c_float) -> c_float {
    ((360.0_f64 / 65536.0_f64) * ((angle_to_short_domain(a) & 65535) as f64)) as f32
}

#[no_mangle]
pub extern "C" fn AngleNormalize360(angle: c_float) -> c_float {
    ((360.0_f64 / 65536.0_f64) * ((angle_to_short_domain(angle) & 65535) as f64)) as f32
}

#[no_mangle]
pub extern "C" fn AngleNormalize180(angle: c_float) -> c_float {
    let mut angle = AngleNormalize360(angle);
    if angle > 180.0 {
        angle -= 360.0;
    }
    angle
}

#[no_mangle]
pub extern "C" fn AngleDelta(angle1: c_float, angle2: c_float) -> c_float {
    AngleNormalize180(angle1 - angle2)
}

#[no_mangle]
pub unsafe extern "C" fn SetPlaneSignbits(out: *mut CPlane) {
    require_pointers!(out);
    let mut bits = 0;
    for j in 0..3 {
        if (*out).normal[j] < 0.0 {
            bits |= 1 << j;
        }
    }
    (*out).signbits = bits;
}

#[no_mangle]
pub unsafe extern "C" fn BoxOnPlaneSide(
    emins: *const c_float,
    emaxs: *const c_float,
    plane: *const CPlane,
) -> c_int {
    require_pointers!(emins, emaxs, plane);
    let emins = read3(emins);
    let emaxs = read3(emaxs);
    let plane = &*plane;

    if plane.plane_type < 3 {
        let axis = plane.plane_type as usize;
        if plane.dist <= emins[axis] {
            return 1;
        }
        if plane.dist >= emaxs[axis] {
            return 2;
        }
        return 3;
    }

    let n = plane.normal;
    let (dist1, dist2) = match plane.signbits {
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

    let mut sides = 0;
    if dist1 >= plane.dist {
        sides = 1;
    }
    if dist2 < plane.dist {
        sides |= 2;
    }
    sides
}

#[no_mangle]
pub unsafe extern "C" fn RadiusFromBounds(mins: *const c_float, maxs: *const c_float) -> c_float {
    require_pointers!(mins, maxs);
    let mins = read3(mins);
    let maxs = read3(maxs);
    let mut corner = [0.0; 3];
    for i in 0..3 {
        let a = mins[i].abs();
        let b = maxs[i].abs();
        corner[i] = if a > b { a } else { b };
    }
    c_sqrt(dot(corner, corner))
}

#[no_mangle]
pub unsafe extern "C" fn ClearBounds(mins: *mut c_float, maxs: *mut c_float) {
    require_pointers!(mins, maxs);
    write3(mins, [99999.0; 3]);
    write3(maxs, [-99999.0; 3]);
}

#[no_mangle]
pub unsafe extern "C" fn AddPointToBounds(
    value: *const c_float,
    mins: *mut c_float,
    maxs: *mut c_float,
) {
    require_pointers!(value, mins, maxs);
    let value = read3(value);
    for i in 0..3 {
        if value[i] < *mins.add(i) {
            *mins.add(i) = value[i];
        }
        if value[i] > *maxs.add(i) {
            *maxs.add(i) = value[i];
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn VectorNormalize(value: *mut c_float) -> c_float {
    require_pointers!(value);
    let mut vector = read3(value);
    let length = c_sqrt(dot(vector, vector));
    if length != 0.0 {
        let ilength = 1.0 / length;
        vector[0] *= ilength;
        vector[1] *= ilength;
        vector[2] *= ilength;
        write3(value, vector);
    }
    length
}

#[no_mangle]
pub unsafe extern "C" fn VectorNormalize2(value: *const c_float, out: *mut c_float) -> c_float {
    require_pointers!(value, out);
    let value = read3(value);
    let length = c_sqrt(dot(value, value));
    if length != 0.0 {
        let ilength = 1.0 / length;
        write3(
            out,
            [value[0] * ilength, value[1] * ilength, value[2] * ilength],
        );
    } else {
        write3(out, [0.0; 3]);
    }
    length
}

#[no_mangle]
pub unsafe extern "C" fn _VectorMA(
    veca: *const c_float,
    scale: c_float,
    vecb: *const c_float,
    vecc: *mut c_float,
) {
    require_pointers!(veca, vecb, vecc);
    let a = read3(veca);
    let b = read3(vecb);
    write3(
        vecc,
        [
            a[0] + scale * b[0],
            a[1] + scale * b[1],
            a[2] + scale * b[2],
        ],
    );
}

#[no_mangle]
pub unsafe extern "C" fn _DotProduct(v1: *const c_float, v2: *const c_float) -> c_float {
    require_pointers!(v1, v2);
    dot(read3(v1), read3(v2))
}

#[no_mangle]
pub unsafe extern "C" fn _VectorSubtract(
    veca: *const c_float,
    vecb: *const c_float,
    out: *mut c_float,
) {
    require_pointers!(veca, vecb, out);
    let a = read3(veca);
    let b = read3(vecb);
    write3(out, [a[0] - b[0], a[1] - b[1], a[2] - b[2]]);
}

#[no_mangle]
pub unsafe extern "C" fn _VectorAdd(veca: *const c_float, vecb: *const c_float, out: *mut c_float) {
    require_pointers!(veca, vecb, out);
    let a = read3(veca);
    let b = read3(vecb);
    write3(out, [a[0] + b[0], a[1] + b[1], a[2] + b[2]]);
}

#[no_mangle]
pub unsafe extern "C" fn _VectorCopy(input: *const c_float, out: *mut c_float) {
    require_pointers!(input, out);
    write3(out, read3(input));
}

#[no_mangle]
pub unsafe extern "C" fn _VectorScale(input: *const c_float, scale: c_float, out: *mut c_float) {
    require_pointers!(input, out);
    let input = read3(input);
    write3(out, [input[0] * scale, input[1] * scale, input[2] * scale]);
}

#[no_mangle]
pub unsafe extern "C" fn Vector4Scale(input: *const c_float, scale: c_float, out: *mut c_float) {
    require_pointers!(input, out);
    for i in 0..4 {
        *out.add(i) = *input.add(i) * scale;
    }
}

#[no_mangle]
pub extern "C" fn Q_log2(mut val: c_int) -> c_int {
    let mut answer = 0;
    loop {
        val >>= 1;
        if val == 0 {
            break;
        }
        answer += 1;
    }
    answer
}

#[no_mangle]
pub unsafe extern "C" fn MatrixMultiply(
    in1: *const [c_float; 3],
    in2: *const [c_float; 3],
    out: *mut [c_float; 3],
) {
    require_pointers!(in1, in2, out);
    let a = [*in1, *in1.add(1), *in1.add(2)];
    let b = [*in2, *in2.add(1), *in2.add(2)];
    let result = matrix_multiply_values(a, b);
    *out = result[0];
    *out.add(1) = result[1];
    *out.add(2) = result[2];
}

#[no_mangle]
pub unsafe extern "C" fn AngleVectors(
    angles: *const c_float,
    forward: *mut c_float,
    right: *mut c_float,
    up: *mut c_float,
) {
    require_pointers!(angles);
    let angles = read3(angles);
    let scale = M_PI * 2.0 / 360.0;

    let angle = (angles[1] as f64 * scale) as f32;
    let sy = sin(angle as f64) as f32;
    let cy = cos(angle as f64) as f32;
    let angle = (angles[0] as f64 * scale) as f32;
    let sp = sin(angle as f64) as f32;
    let cp = cos(angle as f64) as f32;
    let angle = (angles[2] as f64 * scale) as f32;
    let sr = sin(angle as f64) as f32;
    let cr = cos(angle as f64) as f32;

    if !forward.is_null() {
        write3(forward, [cp * cy, cp * sy, -sp]);
    }
    if !right.is_null() {
        write3(
            right,
            [
                -1.0 * sr * sp * cy + -1.0 * cr * -sy,
                -1.0 * sr * sp * sy + -1.0 * cr * cy,
                -1.0 * sr * cp,
            ],
        );
    }
    if !up.is_null() {
        write3(
            up,
            [cr * sp * cy + -sr * -sy, cr * sp * sy + -sr * cy, cr * cp],
        );
    }
}

#[no_mangle]
pub unsafe extern "C" fn PerpendicularVector(dst: *mut c_float, src: *const c_float) {
    require_pointers!(dst, src);
    let src = read3(src);
    let mut pos = 0;
    let mut minelem = 1.0_f32;
    for i in 0..3 {
        let magnitude = (src[i] as f64).abs();
        if magnitude < minelem as f64 {
            pos = i;
            minelem = magnitude as f32;
        }
    }
    let mut tempvec = [0.0; 3];
    tempvec[pos] = 1.0;
    ProjectPointOnPlane(dst, tempvec.as_ptr(), src.as_ptr());
    VectorNormalize(dst);
}
