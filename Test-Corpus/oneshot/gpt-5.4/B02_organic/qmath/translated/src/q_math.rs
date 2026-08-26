use crate::q_shared::*;

pub static VEC3_ORIGIN: Vec3T = [0.0, 0.0, 0.0];
pub static AXIS_DEFAULT: [Vec3T; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

pub static COLOR_BLACK: Vec4T = [0.0, 0.0, 0.0, 1.0];
pub static COLOR_RED: Vec4T = [1.0, 0.0, 0.0, 1.0];
pub static COLOR_GREEN: Vec4T = [0.0, 1.0, 0.0, 1.0];
pub static COLOR_BLUE: Vec4T = [0.0, 0.0, 1.0, 1.0];
pub static COLOR_YELLOW: Vec4T = [1.0, 1.0, 0.0, 1.0];
pub static COLOR_MAGENTA: Vec4T = [1.0, 0.0, 1.0, 1.0];
pub static COLOR_CYAN: Vec4T = [0.0, 1.0, 1.0, 1.0];
pub static COLOR_WHITE: Vec4T = [1.0, 1.0, 1.0, 1.0];
pub static COLOR_LT_GREY: Vec4T = [0.75, 0.75, 0.75, 1.0];
pub static COLOR_MD_GREY: Vec4T = [0.5, 0.5, 0.5, 1.0];
pub static COLOR_DK_GREY: Vec4T = [0.25, 0.25, 0.25, 1.0];

pub static G_COLOR_TABLE: [Vec4T; 8] = [
    [0.0, 0.0, 0.0, 1.0],
    [1.0, 0.0, 0.0, 1.0],
    [0.0, 1.0, 0.0, 1.0],
    [1.0, 1.0, 0.0, 1.0],
    [0.0, 0.0, 1.0, 1.0],
    [0.0, 1.0, 1.0, 1.0],
    [1.0, 0.0, 1.0, 1.0],
    [1.0, 1.0, 1.0, 1.0],
];

pub static BYTEDIRS: [Vec3T; NUMVERTEXNORMALS] = [
    [-0.525731, 0.0, 0.850651], [-0.442863, 0.238856, 0.864188],
    [-0.295242, 0.0, 0.955423], [-0.309017, 0.5, 0.809017],
    [-0.16246, 0.262866, 0.951056], [0.0, 0.0, 1.0],
    [0.0, 0.850651, 0.525731], [-0.147621, 0.716567, 0.681718],
    [0.147621, 0.716567, 0.681718], [0.0, 0.525731, 0.850651],
    [0.309017, 0.5, 0.809017], [0.525731, 0.0, 0.850651],
    [0.295242, 0.0, 0.955423], [0.442863, 0.238856, 0.864188],
    [0.16246, 0.262866, 0.951056], [-0.681718, 0.147621, 0.716567],
    [-0.809017, 0.309017, 0.5], [-0.587785, 0.425325, 0.688191],
    [-0.850651, 0.525731, 0.0], [-0.864188, 0.442863, 0.238856],
    [-0.716567, 0.681718, 0.147621], [-0.688191, 0.587785, 0.425325],
    [-0.5, 0.809017, 0.309017], [-0.238856, 0.864188, 0.442863],
    [-0.425325, 0.688191, 0.587785], [-0.716567, 0.681718, -0.147621],
    [-0.5, 0.809017, -0.309017], [-0.525731, 0.850651, 0.0],
    [0.0, 0.850651, -0.525731], [-0.238856, 0.864188, -0.442863],
    [0.0, 0.955423, -0.295242], [-0.262866, 0.951056, -0.16246],
    [0.0, 1.0, 0.0], [0.0, 0.955423, 0.295242],
    [-0.262866, 0.951056, 0.16246], [0.238856, 0.864188, 0.442863],
    [0.262866, 0.951056, 0.16246], [0.5, 0.809017, 0.309017],
    [0.238856, 0.864188, -0.442863], [0.262866, 0.951056, -0.16246],
    [0.5, 0.809017, -0.309017], [0.850651, 0.525731, 0.0],
    [0.716567, 0.681718, 0.147621], [0.716567, 0.681718, -0.147621],
    [0.525731, 0.850651, 0.0], [0.425325, 0.688191, 0.587785],
    [0.864188, 0.442863, 0.238856], [0.688191, 0.587785, 0.425325],
    [0.809017, 0.309017, 0.5], [0.681718, 0.147621, 0.716567],
    [0.587785, 0.425325, 0.688191], [0.955423, 0.295242, 0.0],
    [1.0, 0.0, 0.0], [0.951056, 0.16246, 0.262866],
    [0.850651, -0.525731, 0.0], [0.955423, -0.295242, 0.0],
    [0.864188, -0.442863, 0.238856], [0.951056, -0.16246, 0.262866],
    [0.809017, -0.309017, 0.5], [0.681718, -0.147621, 0.716567],
    [0.850651, 0.0, 0.525731], [0.864188, 0.442863, -0.238856],
    [0.809017, 0.309017, -0.5], [0.951056, 0.16246, -0.262866],
    [0.525731, 0.0, -0.850651], [0.681718, 0.147621, -0.716567],
    [0.681718, -0.147621, -0.716567], [0.850651, 0.0, -0.525731],
    [0.809017, -0.309017, -0.5], [0.864188, -0.442863, -0.238856],
    [0.951056, -0.16246, -0.262866], [0.147621, 0.716567, -0.681718],
    [0.309017, 0.5, -0.809017], [0.425325, 0.688191, -0.587785],
    [0.442863, 0.238856, -0.864188], [0.587785, 0.425325, -0.688191],
    [0.688191, 0.587785, -0.425325], [-0.147621, 0.716567, -0.681718],
    [-0.309017, 0.5, -0.809017], [0.0, 0.525731, -0.850651],
    [-0.525731, 0.0, -0.850651], [-0.442863, 0.238856, -0.864188],
    [-0.295242, 0.0, -0.955423], [-0.16246, 0.262866, -0.951056],
    [0.0, 0.0, -1.0], [0.295242, 0.0, -0.955423],
    [0.16246, 0.262866, -0.951056], [-0.442863, -0.238856, -0.864188],
    [-0.309017, -0.5, -0.809017], [-0.16246, -0.262866, -0.951056],
    [0.0, -0.850651, -0.525731], [-0.147621, -0.716567, -0.681718],
    [0.147621, -0.716567, -0.681718], [0.0, -0.525731, -0.850651],
    [0.309017, -0.5, -0.809017], [0.442863, -0.238856, -0.864188],
    [0.16246, -0.262866, -0.951056], [0.238856, -0.864188, -0.442863],
    [0.5, -0.809017, -0.309017], [0.425325, -0.688191, -0.587785],
    [0.716567, -0.681718, -0.147621], [0.688191, -0.587785, -0.425325],
    [0.587785, -0.425325, -0.688191], [0.0, -0.955423, -0.295242],
    [0.0, -1.0, 0.0], [0.262866, -0.951056, -0.16246],
    [0.0, -0.850651, 0.525731], [0.0, -0.955423, 0.295242],
    [0.238856, -0.864188, 0.442863], [0.262866, -0.951056, 0.16246],
    [0.5, -0.809017, 0.309017], [0.716567, -0.681718, 0.147621],
    [0.525731, -0.850651, 0.0], [-0.238856, -0.864188, -0.442863],
    [-0.5, -0.809017, -0.309017], [-0.262866, -0.951056, -0.16246],
    [-0.850651, -0.525731, 0.0], [-0.716567, -0.681718, -0.147621],
    [-0.716567, -0.681718, 0.147621], [-0.525731, -0.850651, 0.0],
    [-0.5, -0.809017, 0.309017], [-0.238856, -0.864188, 0.442863],
    [-0.262866, -0.951056, 0.16246], [-0.864188, -0.442863, 0.238856],
    [-0.809017, -0.309017, 0.5], [-0.688191, -0.587785, 0.425325],
    [-0.681718, -0.147621, 0.716567], [-0.442863, -0.238856, 0.864188],
    [-0.587785, -0.425325, 0.688191], [-0.309017, -0.5, 0.809017],
    [-0.147621, -0.716567, 0.681718], [-0.425325, -0.688191, 0.587785],
    [-0.16246, -0.262866, 0.951056], [0.442863, -0.238856, 0.864188],
    [0.16246, -0.262866, 0.951056], [0.309017, -0.5, 0.809017],
    [0.147621, -0.716567, 0.681718], [0.0, -0.525731, 0.850651],
    [0.425325, -0.688191, 0.587785], [0.587785, -0.425325, 0.688191],
    [0.688191, -0.587785, 0.425325], [-0.955423, 0.295242, 0.0],
    [-0.951056, 0.16246, 0.262866], [-1.0, 0.0, 0.0],
    [-0.850651, 0.0, 0.525731], [-0.955423, -0.295242, 0.0],
    [-0.951056, -0.16246, 0.262866], [-0.864188, 0.442863, -0.238856],
    [-0.951056, 0.16246, -0.262866], [-0.809017, 0.309017, -0.5],
    [-0.864188, -0.442863, -0.238856], [-0.951056, -0.16246, -0.262866],
    [-0.809017, -0.309017, -0.5], [-0.681718, 0.147621, -0.716567],
    [-0.681718, -0.147621, -0.716567], [-0.850651, 0.0, -0.525731],
    [-0.688191, 0.587785, -0.425325], [-0.587785, 0.425325, -0.688191],
    [-0.425325, 0.688191, -0.587785], [-0.425325, -0.688191, -0.587785],
    [-0.587785, -0.425325, -0.688191], [-0.688191, -0.587785, -0.425325],
];

pub fn Q_rand(seed: &mut i32) -> i32 {
    *seed = 69069_i32.wrapping_mul(*seed).wrapping_add(1);
    *seed
}

pub fn Q_random(seed: &mut i32) -> f32 {
    (Q_rand(seed) & 0xffff) as f32 / 65536.0
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

pub fn DirToByte(dir: Option<&Vec3T>) -> i32 {
    let Some(dir) = dir else { return 0; };
    let mut bestd = 0.0f32;
    let mut best = 0i32;
    for (i, n) in BYTEDIRS.iter().enumerate() {
        let d = dot_product(dir, n);
        if d > bestd {
            bestd = d;
            best = i as i32;
        }
    }
    best
}

pub fn ByteToDir(b: i32, dir: &mut Vec3T) {
    if b < 0 || b as usize >= NUMVERTEXNORMALS {
        *dir = VEC3_ORIGIN;
        return;
    }
    *dir = BYTEDIRS[b as usize];
}

pub fn ColorBytes3(r: f32, g: f32, b: f32) -> u32 {
    let rb = (r * 255.0) as u8;
    let gb = (g * 255.0) as u8;
    let bb = (b * 255.0) as u8;
    u32::from_le_bytes([rb, gb, bb, 0])
}

pub fn ColorBytes4(r: f32, g: f32, b: f32, a: f32) -> u32 {
    let rb = (r * 255.0) as u8;
    let gb = (g * 255.0) as u8;
    let bb = (b * 255.0) as u8;
    let ab = (a * 255.0) as u8;
    u32::from_le_bytes([rb, gb, bb, ab])
}

pub fn NormalizeColor(input: &Vec3T, out: &mut Vec3T) -> f32 {
    let mut max = input[0];
    if input[1] > max {
        max = input[1];
    }
    if input[2] > max {
        max = input[2];
    }
    if max == 0.0 {
        vector_clear(out);
    } else {
        out[0] = input[0] / max;
        out[1] = input[1] / max;
        out[2] = input[2] / max;
    }
    max
}

pub fn PlaneFromPoints(plane: &mut Vec4T, a: &Vec3T, b: &Vec3T, c: &Vec3T) -> QBoolean {
    let d1 = vector_subtract(b, a);
    let d2 = vector_subtract(c, a);
    let cross = cross_product(&d2, &d1);
    plane[0] = cross[0];
    plane[1] = cross[1];
    plane[2] = cross[2];
    let mut n = [plane[0], plane[1], plane[2]];
    if VectorNormalize(&mut n) == 0.0 {
        return false;
    }
    plane[0] = n[0];
    plane[1] = n[1];
    plane[2] = n[2];
    plane[3] = dot_product(a, &n);
    true
}

pub fn RotatePointAroundVector(dst: &mut Vec3T, dir: &Vec3T, point: &Vec3T, degrees: f32) {
    let mut m = [[0.0f32; 3]; 3];
    let mut im = [[0.0f32; 3]; 3];
    let mut zrot = [[0.0f32; 3]; 3];
    let mut tmpmat = [[0.0f32; 3]; 3];
    let mut rot = [[0.0f32; 3]; 3];
    let vf = *dir;
    let mut vr = [0.0; 3];
    let mut vup = [0.0; 3];

    PerpendicularVector(&mut vr, dir);
    vup = cross_product(&vr, &vf);

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

    let rad = deg2rad(degrees);
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

pub fn RotateAroundDirection(axis: &mut [Vec3T; 3], yaw: f32) {
    PerpendicularVector(&mut axis[1], &axis[0]);
    if yaw != 0.0 {
        let temp = axis[1];
        RotatePointAroundVector(&mut axis[1], &axis[0], &temp, yaw);
    }
    axis[2] = cross_product(&axis[0], &axis[1]);
}

pub fn vectoangles(value1: &Vec3T, angles: &mut Vec3T) {
    let yaw;
    let pitch;
    if value1[1] == 0.0 && value1[0] == 0.0 {
        if value1[2] > 0.0 {
            pitch = 90.0;
        } else {
            pitch = 270.0;
        }
        angles[PITCH] = -pitch;
        angles[YAW] = 0.0;
        angles[ROLL] = 0.0;
        return;
    }

    let mut y = if value1[0] != 0.0 {
        value1[1].atan2(value1[0]) * 180.0 / M_PI
    } else if value1[1] > 0.0 {
        90.0
    } else {
        270.0
    };
    if y < 0.0 {
        y += 360.0;
    }
    yaw = y;

    let forward = (value1[0] * value1[0] + value1[1] * value1[1]).sqrt();
    let mut p = value1[2].atan2(forward) * 180.0 / M_PI;
    if p < 0.0 {
        p += 360.0;
    }
    pitch = p;

    angles[PITCH] = -pitch;
    angles[YAW] = yaw;
    angles[ROLL] = 0.0;
}

pub fn AnglesToAxis(angles: &Vec3T, axis: &mut [Vec3T; 3]) {
    let mut right = [0.0; 3];
    AngleVectors(angles, Some(&mut axis[0]), Some(&mut right), Some(&mut axis[2]));
    axis[1] = vector_subtract(&VEC3_ORIGIN, &right);
}

pub fn AxisClear(axis: &mut [Vec3T; 3]) {
    axis[0] = [1.0, 0.0, 0.0];
    axis[1] = [0.0, 1.0, 0.0];
    axis[2] = [0.0, 0.0, 1.0];
}

pub fn AxisCopy(input: &[Vec3T; 3], out: &mut [Vec3T; 3]) {
    *out = *input;
}

pub fn ProjectPointOnPlane(dst: &mut Vec3T, p: &Vec3T, normal: &Vec3T) {
    let inv_denom = 1.0 / dot_product(normal, normal);
    let d = dot_product(normal, p) * inv_denom;
    let n = [normal[0] * inv_denom, normal[1] * inv_denom, normal[2] * inv_denom];
    dst[0] = p[0] - d * n[0];
    dst[1] = p[1] - d * n[1];
    dst[2] = p[2] - d * n[2];
}

pub fn MakeNormalVectors(forward: &Vec3T, right: &mut Vec3T, up: &mut Vec3T) {
    right[1] = -forward[0];
    right[2] = forward[1];
    right[0] = forward[2];
    let d = dot_product(right, forward);
    *right = vector_ma(right, -d, forward);
    VectorNormalize(right);
    *up = cross_product(right, forward);
}

pub fn VectorRotate(input: &Vec3T, matrix: &[Vec3T; 3], out: &mut Vec3T) {
    out[0] = dot_product(input, &matrix[0]);
    out[1] = dot_product(input, &matrix[1]);
    out[2] = dot_product(input, &matrix[2]);
}

pub fn Q_rsqrt(number: f32) -> f32 {
    let x2 = number * 0.5f32;
    let mut y = number;
    let mut i = y.to_bits();
    i = 0x5f3759dfu32.wrapping_sub(i >> 1);
    y = f32::from_bits(i);
    y * (1.5f32 - (x2 * y * y))
}

pub fn Q_fabs(f: f32) -> f32 {
    f.abs()
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

pub fn AnglesSubtract(v1: &Vec3T, v2: &Vec3T, v3: &mut Vec3T) {
    v3[0] = AngleSubtract(v1[0], v2[0]);
    v3[1] = AngleSubtract(v1[1], v2[1]);
    v3[2] = AngleSubtract(v1[2], v2[2]);
}

pub fn AngleMod(a: f32) -> f32 {
    (360.0 / 65536.0) * (((a * (65536.0 / 360.0)) as i32 & 65535) as f32)
}

pub fn AngleNormalize360(angle: f32) -> f32 {
    (360.0 / 65536.0) * (((angle * (65536.0 / 360.0)) as i32 & 65535) as f32)
}

pub fn AngleNormalize180(angle: f32) -> f32 {
    let mut angle = AngleNormalize360(angle);
    if angle > 180.0 {
        angle -= 360.0;
    }
    angle
}

pub fn AngleDelta(angle1: f32, angle2: f32) -> f32 {
    AngleNormalize180(angle1 - angle2)
}

pub fn SetPlaneSignbits(out: &mut CPlane) {
    let mut bits = 0u8;
    for j in 0..3 {
        if out.normal[j] < 0.0 {
            bits |= 1 << j;
        }
    }
    out.signbits = bits;
}

pub fn BoxOnPlaneSide(emins: &Vec3T, emaxs: &Vec3T, p: &CPlane) -> i32 {
    if p.type_ < 3 {
        if p.dist <= emins[p.type_ as usize] {
            return 1;
        }
        if p.dist >= emaxs[p.type_ as usize] {
            return 2;
        }
        return 3;
    }

    let (dist1, dist2) = match p.signbits {
        0 => (
            p.normal[0] * emaxs[0] + p.normal[1] * emaxs[1] + p.normal[2] * emaxs[2],
            p.normal[0] * emins[0] + p.normal[1] * emins[1] + p.normal[2] * emins[2],
        ),
        1 => (
            p.normal[0] * emins[0] + p.normal[1] * emaxs[1] + p.normal[2] * emaxs[2],
            p.normal[0] * emaxs[0] + p.normal[1] * emins[1] + p.normal[2] * emins[2],
        ),
        2 => (
            p.normal[0] * emaxs[0] + p.normal[1] * emins[1] + p.normal[2] * emaxs[2],
            p.normal[0] * emins[0] + p.normal[1] * emaxs[1] + p.normal[2] * emins[2],
        ),
        3 => (
            p.normal[0] * emins[0] + p.normal[1] * emins[1] + p.normal[2] * emaxs[2],
            p.normal[0] * emaxs[0] + p.normal[1] * emaxs[1] + p.normal[2] * emins[2],
        ),
        4 => (
            p.normal[0] * emaxs[0] + p.normal[1] * emaxs[1] + p.normal[2] * emins[2],
            p.normal[0] * emins[0] + p.normal[1] * emins[1] + p.normal[2] * emaxs[2],
        ),
        5 => (
            p.normal[0] * emins[0] + p.normal[1] * emaxs[1] + p.normal[2] * emins[2],
            p.normal[0] * emaxs[0] + p.normal[1] * emins[1] + p.normal[2] * emaxs[2],
        ),
        6 => (
            p.normal[0] * emaxs[0] + p.normal[1] * emins[1] + p.normal[2] * emins[2],
            p.normal[0] * emins[0] + p.normal[1] * emaxs[1] + p.normal[2] * emaxs[2],
        ),
        7 => (
            p.normal[0] * emins[0] + p.normal[1] * emins[1] + p.normal[2] * emins[2],
            p.normal[0] * emaxs[0] + p.normal[1] * emaxs[1] + p.normal[2] * emaxs[2],
        ),
        _ => (0.0, 0.0),
    };

    let mut sides = 0;
    if dist1 >= p.dist {
        sides = 1;
    }
    if dist2 < p.dist {
        sides |= 2;
    }
    sides
}

pub fn RadiusFromBounds(mins: &Vec3T, maxs: &Vec3T) -> f32 {
    let mut corner = [0.0; 3];
    for i in 0..3 {
        let a = mins[i].abs();
        let b = maxs[i].abs();
        corner[i] = if a > b { a } else { b };
    }
    vector_length(&corner)
}

pub fn ClearBounds(mins: &mut Vec3T, maxs: &mut Vec3T) {
    *mins = [99999.0, 99999.0, 99999.0];
    *maxs = [-99999.0, -99999.0, -99999.0];
}

pub fn AddPointToBounds(v: &Vec3T, mins: &mut Vec3T, maxs: &mut Vec3T) {
    for i in 0..3 {
        if v[i] < mins[i] {
            mins[i] = v[i];
        }
        if v[i] > maxs[i] {
            maxs[i] = v[i];
        }
    }
}

pub fn VectorNormalize(v: &mut Vec3T) -> VecT {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length != 0.0 {
        let ilength = 1.0 / length;
        v[0] *= ilength;
        v[1] *= ilength;
        v[2] *= ilength;
    }
    length
}

pub fn VectorNormalize2(v: &Vec3T, out: &mut Vec3T) -> VecT {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
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

pub fn _VectorMA(veca: &Vec3T, scale: f32, vecb: &Vec3T, vecc: &mut Vec3T) {
    vecc[0] = veca[0] + scale * vecb[0];
    vecc[1] = veca[1] + scale * vecb[1];
    vecc[2] = veca[2] + scale * vecb[2];
}

pub fn _DotProduct(v1: &Vec3T, v2: &Vec3T) -> VecT {
    dot_product(v1, v2)
}

pub fn _VectorSubtract(veca: &Vec3T, vecb: &Vec3T, out: &mut Vec3T) {
    *out = vector_subtract(veca, vecb);
}

pub fn _VectorAdd(veca: &Vec3T, vecb: &Vec3T, out: &mut Vec3T) {
    *out = vector_add(veca, vecb);
}

pub fn _VectorCopy(input: &Vec3T, out: &mut Vec3T) {
    *out = *input;
}

pub fn _VectorScale(input: &Vec3T, scale: VecT, out: &mut Vec3T) {
    *out = vector_scale(input, scale);
}

pub fn Vector4Scale(input: &Vec4T, scale: VecT, out: &mut Vec4T) {
    out[0] = input[0] * scale;
    out[1] = input[1] * scale;
    out[2] = input[2] * scale;
    out[3] = input[3] * scale;
}

pub fn Q_log2(mut val: i32) -> i32 {
    let mut answer = 0;
    while {
        val >>= 1;
        val != 0
    } {
        answer += 1;
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

pub fn AngleVectors(angles: &Vec3T, forward: Option<&mut Vec3T>, right: Option<&mut Vec3T>, up: Option<&mut Vec3T>) {
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
        right[0] = -sr * sp * cy + cr * sy;
        right[1] = -sr * sp * sy - cr * cy;
        right[2] = -sr * cp;
    }
    if let Some(up) = up {
        up[0] = cr * sp * cy + sr * sy;
        up[1] = cr * sp * sy - sr * cy;
        up[2] = cr * cp;
    }
}

pub fn PerpendicularVector(dst: &mut Vec3T, src: &Vec3T) {
    let mut pos = 0usize;
    let mut minelem = 1.0f32;
    for i in 0..3 {
        if src[i].abs() < minelem {
            pos = i;
            minelem = src[i].abs();
        }
    }
    let mut tempvec = [0.0f32; 3];
    tempvec[pos] = 1.0;
    ProjectPointOnPlane(dst, &tempvec, src);
    VectorNormalize(dst);
}
