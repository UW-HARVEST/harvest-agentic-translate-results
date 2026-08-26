use crate::q_shared::*;

pub static mut vec3_origin: vec3_t = [0.0, 0.0, 0.0];
pub static mut axisDefault: [vec3_t; 3] = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
];

pub static mut colorBlack: vec4_t = [0.0, 0.0, 0.0, 1.0];
pub static mut colorRed: vec4_t = [1.0, 0.0, 0.0, 1.0];
pub static mut colorGreen: vec4_t = [0.0, 1.0, 0.0, 1.0];
pub static mut colorBlue: vec4_t = [0.0, 0.0, 1.0, 1.0];
pub static mut colorYellow: vec4_t = [1.0, 1.0, 0.0, 1.0];
pub static mut colorMagenta: vec4_t = [1.0, 0.0, 1.0, 1.0];
pub static mut colorCyan: vec4_t = [0.0, 1.0, 1.0, 1.0];
pub static mut colorWhite: vec4_t = [1.0, 1.0, 1.0, 1.0];
pub static mut colorLtGrey: vec4_t = [0.75, 0.75, 0.75, 1.0];
pub static mut colorMdGrey: vec4_t = [0.5, 0.5, 0.5, 1.0];
pub static mut colorDkGrey: vec4_t = [0.25, 0.25, 0.25, 1.0];

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

pub static mut bytedirs: [vec3_t; NUMVERTEXNORMALS] = [
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
    [-0.587785, -0.425325, -0.688191], [-0.688191, -0.587785, -0.425325]
];

pub fn Q_rand(seed: &mut i32) -> i32 {
    *seed = 69069i32.wrapping_mul(*seed).wrapping_add(1);
    *seed
}

pub fn Q_random(seed: &mut i32) -> f32 {
    (Q_rand(seed) & 0xffff) as f32 / 0x10000 as f32
}

pub fn Q_crandom(seed: &mut i32) -> f32 {
    2.0 * (Q_random(seed) - 0.5)
}

pub fn ClampChar(i: i32) -> i8 {
    if i < -128 {
        -128
    } else if i > 127 {
        127
    } else {
        i as i8
    }
}

pub fn ClampShort(i: i32) -> i16 {
    if i < -32768 {
        -32768
    } else if i > 0x7fff {
        0x7fff
    } else {
        i as i16
    }
}

pub fn DirToByte(dir: &vec3_t) -> i32 {
    let mut best = 0;
    let mut bestd = 0.0;

    unsafe {
        for i in 0..NUMVERTEXNORMALS {
            let d = DotProduct(dir, &bytedirs[i]);
            if d > bestd {
                bestd = d;
                best = i;
            }
        }
    }

    best as i32
}

pub fn ByteToDir(b: i32, dir: &mut vec3_t) {
    if b < 0 || b >= NUMVERTEXNORMALS as i32 {
        unsafe { VectorCopy(&vec3_origin, dir); }
        return;
    }
    unsafe { VectorCopy(&bytedirs[b as usize], dir); }
}

pub fn ColorBytes3(r: f32, g: f32, b: f32) -> u32 {
    let mut i: u32 = 0;
    let bytes = unsafe { std::slice::from_raw_parts_mut(&mut i as *mut u32 as *mut u8, 4) };
    bytes[0] = (r * 255.0) as u8;
    bytes[1] = (g * 255.0) as u8;
    bytes[2] = (b * 255.0) as u8;
    i
}

pub fn ColorBytes4(r: f32, g: f32, b: f32, a: f32) -> u32 {
    let mut i: u32 = 0;
    let bytes = unsafe { std::slice::from_raw_parts_mut(&mut i as *mut u32 as *mut u8, 4) };
    bytes[0] = (r * 255.0) as u8;
    bytes[1] = (g * 255.0) as u8;
    bytes[2] = (b * 255.0) as u8;
    bytes[3] = (a * 255.0) as u8;
    i
}

pub fn NormalizeColor(in_: &vec3_t, out: &mut vec3_t) -> f32 {
    let mut max = in_[0];
    if in_[1] > max {
        max = in_[1];
    }
    if in_[2] > max {
        max = in_[2];
    }

    if max == 0.0 {
        VectorClear(out);
    } else {
        out[0] = in_[0] / max;
        out[1] = in_[1] / max;
        out[2] = in_[2] / max;
    }
    max
}

pub fn PlaneFromPoints(plane: &mut vec4_t, a: &vec3_t, b: &vec3_t, c: &vec3_t) -> qboolean {
    let mut d1 = [0.0; 3];
    let mut d2 = [0.0; 3];

    VectorSubtract(b, a, &mut d1);
    VectorSubtract(c, a, &mut d2);
    let mut p3 = [0.0; 3];
    CrossProduct(&d2, &d1, &mut p3);
    plane[0] = p3[0];
    plane[1] = p3[1];
    plane[2] = p3[2];
    if VectorNormalize(&mut p3) == 0.0 {
        return qfalse;
    }
    plane[0] = p3[0];
    plane[1] = p3[1];
    plane[2] = p3[2];

    plane[3] = DotProduct(a, &p3);
    qtrue
}

pub fn RotatePointAroundVector(dst: &mut vec3_t, dir: &vec3_t, point: &vec3_t, degrees: f32) {
    let mut m = [[0.0; 3]; 3];
    let mut im = [[0.0; 3]; 3];
    let mut zrot = [[0.0; 3]; 3];
    let mut tmpmat = [[0.0; 3]; 3];
    let mut rot = [[0.0; 3]; 3];
    let mut vr = [0.0; 3];
    let mut vup = [0.0; 3];
    let mut vf = [0.0; 3];

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

    zrot[0][0] = 1.0;
    zrot[1][1] = 1.0;
    zrot[2][2] = 1.0;

    let rad = DEG2RAD(degrees);
    zrot[0][0] = rad.cos();
    zrot[0][1] = rad.sin();
    zrot[1][0] = -rad.sin();
    zrot[1][1] = rad.cos();

    MatrixMultiply(&m, &zrot, &mut tmpmat);
    MatrixMultiply(&tmpmat, &im, &mut rot);

    for i in 0..3 {
        dst[i] = rot[i][0] * point[0] + rot[i][1] * point[1] + rot[i][2] * point[2];
    }
}

pub fn RotateAroundDirection(axis: &mut [vec3_t; 3], yaw: f32) {
    let mut temp = [0.0; 3];
    PerpendicularVector(&mut temp, &axis[0]);
    axis[1] = temp;

    if yaw != 0.0 {
        let mut temp2 = [0.0; 3];
        VectorCopy(&axis[1], &mut temp2);
        RotatePointAroundVector(&mut axis[1], &axis[0], &temp2, yaw);
    }

    let mut temp3 = [0.0; 3];
    CrossProduct(&axis[0], &axis[1], &mut temp3);
    axis[2] = temp3;
}

pub fn vectoangles(value1: &vec3_t, angles: &mut vec3_t) {
    let mut yaw;
    let mut pitch;

    if value1[1] == 0.0 && value1[0] == 0.0 {
        yaw = 0.0;
        if value1[2] > 0.0 {
            pitch = 90.0;
        } else {
            pitch = 270.0;
        }
    } else {
        if value1[0] != 0.0 {
            yaw = value1[1].atan2(value1[0]) * 180.0 / M_PI;
        } else if value1[1] > 0.0 {
            yaw = 90.0;
        } else {
            yaw = 270.0;
        }
        if yaw < 0.0 {
            yaw += 360.0;
        }

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

pub fn AnglesToAxis(angles: &vec3_t, axis: &mut [vec3_t; 3]) {
    let mut right = [0.0; 3];
    AngleVectors(angles, Some(&mut axis[0]), Some(&mut right), Some(&mut axis[2]));
    unsafe { VectorSubtract(&vec3_origin, &right, &mut axis[1]); }
}

pub fn AxisClear(axis: &mut [vec3_t; 3]) {
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

pub fn AxisCopy(in_: &[vec3_t; 3], out: &mut [vec3_t; 3]) {
    VectorCopy(&in_[0], &mut out[0]);
    VectorCopy(&in_[1], &mut out[1]);
    VectorCopy(&in_[2], &mut out[2]);
}

pub fn ProjectPointOnPlane(dst: &mut vec3_t, p: &vec3_t, normal: &vec3_t) {
    let mut inv_denom = DotProduct(normal, normal);
    inv_denom = 1.0 / inv_denom;

    let d = DotProduct(normal, p) * inv_denom;

    let mut n = [0.0; 3];
    n[0] = normal[0] * inv_denom;
    n[1] = normal[1] * inv_denom;
    n[2] = normal[2] * inv_denom;

    dst[0] = p[0] - d * n[0];
    dst[1] = p[1] - d * n[1];
    dst[2] = p[2] - d * n[2];
}

pub fn MakeNormalVectors(forward: &vec3_t, right: &mut vec3_t, up: &mut vec3_t) {
    right[1] = -forward[0];
    right[2] = forward[1];
    right[0] = forward[2];

    let d = DotProduct(right, forward);
    let mut temp = [0.0; 3];
    VectorMA(right, -d, forward, &mut temp);
    VectorCopy(&temp, right);
    VectorNormalize(right);
    CrossProduct(right, forward, up);
}

pub fn VectorRotate(in_: &vec3_t, matrix: &[vec3_t; 3], out: &mut vec3_t) {
    out[0] = DotProduct(in_, &matrix[0]);
    out[1] = DotProduct(in_, &matrix[1]);
    out[2] = DotProduct(in_, &matrix[2]);
}

pub fn Q_rsqrt(number: f32) -> f32 {
    let x2 = number * 0.5;
    let mut y = number;
    let threehalfs = 1.5;

    let mut i = y.to_bits();
    i = 0x5f3759df - (i >> 1);
    y = f32::from_bits(i);

    y = y * (threehalfs - (x2 * y * y));
    y
}

pub fn Q_fabs(f: f32) -> f32 {
    let mut tmp = f.to_bits();
    tmp &= 0x7FFFFFFF;
    f32::from_bits(tmp)
}

pub fn LerpAngle(from: f32, mut to: f32, frac: f32) -> f32 {
    if to - from > 180.0 {
        to -= 360.0;
    }
    if to - from < -180.0 {
        to += 360.0;
    }
    from + frac * (to - from)
}

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

pub fn AnglesSubtract(v1: &vec3_t, v2: &vec3_t, v3: &mut vec3_t) {
    v3[0] = AngleSubtract(v1[0], v2[0]);
    v3[1] = AngleSubtract(v1[1], v2[1]);
    v3[2] = AngleSubtract(v1[2], v2[2]);
}

pub fn AngleMod(mut a: f32) -> f32 {
    a = (360.0 / 65536.0) * (((a * (65536.0 / 360.0)) as i32) & 65535) as f32;
    a
}

pub fn AngleNormalize360(angle: f32) -> f32 {
    (360.0 / 65536.0) * (((angle * (65536.0 / 360.0)) as i32) & 65535) as f32
}

pub fn AngleNormalize180(mut angle: f32) -> f32 {
    angle = AngleNormalize360(angle);
    if angle > 180.0 {
        angle -= 360.0;
    }
    angle
}

pub fn AngleDelta(angle1: f32, angle2: f32) -> f32 {
    AngleNormalize180(angle1 - angle2)
}

pub fn SetPlaneSignbits(out: &mut cplane_t) {
    let mut bits = 0;
    for j in 0..3 {
        if out.normal[j] < 0.0 {
            bits |= 1 << j;
        }
    }
    out.signbits = bits;
}

pub fn BoxOnPlaneSide(emins: &vec3_t, emaxs: &vec3_t, p: &cplane_t) -> i32 {
    let mut dist1 = 0.0;
    let mut dist2 = 0.0;

    if p.type_ < 3 {
        if p.dist <= emins[p.type_ as usize] {
            return 1;
        }
        if p.dist >= emaxs[p.type_ as usize] {
            return 2;
        }
        return 3;
    }

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
        _ => {}
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

pub fn RadiusFromBounds(mins: &vec3_t, maxs: &vec3_t) -> f32 {
    let mut corner = [0.0; 3];

    for i in 0..3 {
        let a = mins[i].abs();
        let b = maxs[i].abs();
        corner[i] = if a > b { a } else { b };
    }

    VectorLength(&corner)
}

pub fn ClearBounds(mins: &mut vec3_t, maxs: &mut vec3_t) {
    mins[0] = 99999.0;
    mins[1] = 99999.0;
    mins[2] = 99999.0;
    maxs[0] = -99999.0;
    maxs[1] = -99999.0;
    maxs[2] = -99999.0;
}

pub fn AddPointToBounds(v: &vec3_t, mins: &mut vec3_t, maxs: &mut vec3_t) {
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

pub fn VectorNormalize(v: &mut vec3_t) -> f32 {
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

pub fn VectorNormalize2(v: &vec3_t, out: &mut vec3_t) -> f32 {
    let mut length = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
    length = length.sqrt();

    if length != 0.0 {
        let ilength = 1.0 / length;
        out[0] = v[0] * ilength;
        out[1] = v[1] * ilength;
        out[2] = v[2] * ilength;
    } else {
        VectorClear(out);
    }

    length
}

pub fn _VectorMA(veca: &vec3_t, scale: f32, vecb: &vec3_t, vecc: &mut vec3_t) {
    vecc[0] = veca[0] + scale * vecb[0];
    vecc[1] = veca[1] + scale * vecb[1];
    vecc[2] = veca[2] + scale * vecb[2];
}

pub fn _DotProduct(v1: &vec3_t, v2: &vec3_t) -> f32 {
    v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2]
}

pub fn _VectorSubtract(veca: &vec3_t, vecb: &vec3_t, out: &mut vec3_t) {
    out[0] = veca[0] - vecb[0];
    out[1] = veca[1] - vecb[1];
    out[2] = veca[2] - vecb[2];
}

pub fn _VectorAdd(veca: &vec3_t, vecb: &vec3_t, out: &mut vec3_t) {
    out[0] = veca[0] + vecb[0];
    out[1] = veca[1] + vecb[1];
    out[2] = veca[2] + vecb[2];
}

pub fn _VectorCopy(in_: &vec3_t, out: &mut vec3_t) {
    out[0] = in_[0];
    out[1] = in_[1];
    out[2] = in_[2];
}

pub fn _VectorScale(in_: &vec3_t, scale: f32, out: &mut vec3_t) {
    out[0] = in_[0] * scale;
    out[1] = in_[1] * scale;
    out[2] = in_[2] * scale;
}

pub fn Vector4Scale(in_: &vec4_t, scale: f32, out: &mut vec4_t) {
    out[0] = in_[0] * scale;
    out[1] = in_[1] * scale;
    out[2] = in_[2] * scale;
    out[3] = in_[3] * scale;
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

pub fn AngleVectors(angles: &vec3_t, forward: Option<&mut vec3_t>, right: Option<&mut vec3_t>, up: Option<&mut vec3_t>) {
    let mut angle = angles[YAW] * (M_PI * 2.0 / 360.0);
    let sy = angle.sin();
    let cy = angle.cos();
    angle = angles[PITCH] * (M_PI * 2.0 / 360.0);
    let sp = angle.sin();
    let cp = angle.cos();
    angle = angles[ROLL] * (M_PI * 2.0 / 360.0);
    let sr = angle.sin();
    let cr = angle.cos();

    if let Some(f) = forward {
        f[0] = cp * cy;
        f[1] = cp * sy;
        f[2] = -sp;
    }
    if let Some(r) = right {
        r[0] = -1.0 * sr * sp * cy + -1.0 * cr * -sy;
        r[1] = -1.0 * sr * sp * sy + -1.0 * cr * cy;
        r[2] = -1.0 * sr * cp;
    }
    if let Some(u) = up {
        u[0] = cr * sp * cy + -sr * -sy;
        u[1] = cr * sp * sy + -sr * cy;
        u[2] = cr * cp;
    }
}

pub fn PerpendicularVector(dst: &mut vec3_t, src: &vec3_t) {
    let mut pos = 0;
    let mut minelem = 1.0;
    let mut tempvec = [0.0; 3];

    for i in 0..3 {
        if src[i].abs() < minelem {
            pos = i;
            minelem = src[i].abs();
        }
    }
    tempvec[pos] = 1.0;

    ProjectPointOnPlane(dst, &tempvec, src);
    VectorNormalize(dst);
}
