pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];
pub type Vec4 = [f32; 4];
pub type Vec5 = [f32; 5];

pub const NUMVERTEXNORMALS: usize = 162;

pub const VEC3_ORIGIN: Vec3 = [0.0, 0.0, 0.0];
pub const AXIS_DEFAULT: [Vec3; 3] = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
];

pub const COLOR_BLACK: Vec4 = [0.0, 0.0, 0.0, 1.0];
pub const COLOR_RED: Vec4 = [1.0, 0.0, 0.0, 1.0];
pub const COLOR_GREEN: Vec4 = [0.0, 1.0, 0.0, 1.0];
pub const COLOR_BLUE: Vec4 = [0.0, 0.0, 1.0, 1.0];
pub const COLOR_YELLOW: Vec4 = [1.0, 1.0, 0.0, 1.0];
pub const COLOR_MAGENTA: Vec4 = [1.0, 0.0, 1.0, 1.0];
pub const COLOR_CYAN: Vec4 = [0.0, 1.0, 1.0, 1.0];
pub const COLOR_WHITE: Vec4 = [1.0, 1.0, 1.0, 1.0];
pub const COLOR_LT_GREY: Vec4 = [0.75, 0.75, 0.75, 1.0];
pub const COLOR_MD_GREY: Vec4 = [0.5, 0.5, 0.5, 1.0];
pub const COLOR_DK_GREY: Vec4 = [0.25, 0.25, 0.25, 1.0];

pub const G_COLOR_TABLE: [Vec4; 8] = [
    [0.0, 0.0, 0.0, 1.0],
    [1.0, 0.0, 0.0, 1.0],
    [0.0, 1.0, 0.0, 1.0],
    [1.0, 1.0, 0.0, 1.0],
    [0.0, 0.0, 1.0, 1.0],
    [0.0, 1.0, 1.0, 1.0],
    [1.0, 0.0, 1.0, 1.0],
    [1.0, 1.0, 1.0, 1.0],
];

pub const BYTEDIRS: [Vec3; NUMVERTEXNORMALS] = [
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

pub fn q_rand(seed: &mut i32) -> i32 {
    *seed = 69069 * *seed + 1;
    *seed
}

pub fn q_random(seed: &mut i32) -> f32 {
    ((q_rand(seed) & 0xffff) as f32) / 0x10000 as f32
}

pub fn q_crandom(seed: &mut i32) -> f32 {
    2.0 * (q_random(seed) - 0.5)
}

pub fn clamp_char(i: i32) -> i8 {
    if i < -128 {
        -128
    } else if i > 127 {
        127
    } else {
        i as i8
    }
}

pub fn clamp_short(i: i32) -> i16 {
    if i < -32768 {
        -32768
    } else if i > 0x7fff {
        0x7fff
    } else {
        i as i16
    }
}

pub fn dir_to_byte(dir: Option<&Vec3>) -> i32 {
    let dir = match dir {
        Some(d) => d,
        None => return 0,
    };

    let mut best = 0;
    let mut bestd = 0.0f32;

    for i in 0..NUMVERTEXNORMALS {
        let d = dot_product(dir, &BYTEDIRS[i]);
        if d > bestd {
            bestd = d;
            best = i;
        }
    }

    best as i32
}

pub fn byte_to_dir(b: i32, dir: &mut Vec3) {
    if b < 0 || b >= NUMVERTEXNORMALS as i32 {
        *dir = VEC3_ORIGIN;
        return;
    }
    *dir = BYTEDIRS[b as usize];
}

pub fn color_bytes3(r: f32, g: f32, b: f32) -> u32 {
    let r = (r * 255.0) as u8;
    let g = (g * 255.0) as u8;
    let b = (b * 255.0) as u8;
    ((r as u32) << 0) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub fn color_bytes4(r: f32, g: f32, b: f32, a: f32) -> u32 {
    let r = (r * 255.0) as u8;
    let g = (g * 255.0) as u8;
    let b = (b * 255.0) as u8;
    let a = (a * 255.0) as u8;
    ((r as u32) << 0) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
}

pub fn normalize_color(input: &Vec3, output: &mut Vec3) -> f32 {
    let mut max = input[0];
    if input[1] > max {
        max = input[1];
    }
    if input[2] > max {
        max = input[2];
    }

    if max == 0.0 {
        *output = [0.0, 0.0, 0.0];
    } else {
        output[0] = input[0] / max;
        output[1] = input[1] / max;
        output[2] = input[2] / max;
    }
    max
}

pub fn plane_from_points(plane: &mut Vec4, a: &Vec3, b: &Vec3, c: &Vec3) -> bool {
    let mut d1 = [0.0f32; 3];
    let mut d2 = [0.0f32; 3];

    vector_subtract(b, a, &mut d1);
    vector_subtract(c, a, &mut d2);
    cross_product(&d2, &d1, &mut plane[0..3].try_into().unwrap());

    if vector_normalize(&mut plane[0..3].try_into().unwrap()) == 0.0 {
        return false;
    }

    plane[3] = dot_product(a, &plane[0..3].try_into().unwrap());
    true
}

pub fn rotate_point_around_vector(dst: &mut Vec3, dir: &Vec3, point: &Vec3, degrees: f32) {
    let mut m = [[0.0f32; 3]; 3];
    let mut im = [[0.0f32; 3]; 3];
    let mut zrot = [[0.0f32; 3]; 3];
    let mut tmpmat = [[0.0f32; 3]; 3];
    let mut rot = [[0.0f32; 3]; 3];

    let vf = *dir;

    let mut vr = [0.0f32; 3];
    perpendicular_vector(&mut vr, dir);
    let mut vup = [0.0f32; 3];
    cross_product(&vr, &vf, &mut vup);

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

    let rad = degrees.to_radians();
    zrot[0][0] = rad.cos();
    zrot[0][1] = rad.sin();
    zrot[1][0] = -rad.sin();
    zrot[1][1] = rad.cos();

    matrix_multiply(&m, &zrot, &mut tmpmat);
    matrix_multiply(&tmpmat, &im, &mut rot);

    for i in 0..3 {
        dst[i] = rot[i][0] * point[0] + rot[i][1] * point[1] + rot[i][2] * point[2];
    }
}

pub fn rotate_around_direction(axis: &mut [Vec3; 3], yaw: f32) {
    let mut temp = [0.0f32; 3];
    perpendicular_vector(&mut axis[1], &axis[0]);

    if yaw != 0.0 {
        temp = axis[1];
        rotate_point_around_vector(&mut axis[1], &axis[0], &temp, yaw);
    }

    cross_product(&axis[0], &axis[1], &mut axis[2]);
}

pub fn vectoangles(value1: &Vec3, angles: &mut Vec3) {
    let (yaw, pitch);

    if value1[1] == 0.0 && value1[0] == 0.0 {
        yaw = 0.0;
        if value1[2] > 0.0 {
            pitch = 90.0;
        } else {
            pitch = 270.0;
        }
    } else {
        if value1[0] != 0.0 {
            yaw = value1[1].atan2(value1[0]).to_degrees();
        } else if value1[1] > 0.0 {
            yaw = 90.0;
        } else {
            yaw = 270.0;
        }
        let yaw = if yaw < 0.0 { yaw + 360.0 } else { yaw };

        let forward = (value1[0] * value1[0] + value1[1] * value1[1]).sqrt();
        let mut pitch = value1[2].atan2(forward).to_degrees();
        if pitch < 0.0 {
            pitch += 360.0;
        }
        (yaw, pitch)
    };

    angles[0] = -pitch;
    angles[1] = yaw;
    angles[2] = 0.0;
}

pub fn angles_to_axis(angles: &Vec3, axis: &mut [Vec3; 3]) {
    let mut right = [0.0f32; 3];
    angle_vectors(angles, &mut axis[0], &mut right, &mut axis[2]);
    vector_subtract(&VEC3_ORIGIN, &right, &mut axis[1]);
}

pub fn axis_clear(axis: &mut [Vec3; 3]) {
    axis[0] = [1.0, 0.0, 0.0];
    axis[1] = [0.0, 1.0, 0.0];
    axis[2] = [0.0, 0.0, 1.0];
}

pub fn axis_copy(input: &[Vec3; 3], output: &mut [Vec3; 3]) {
    output[0] = input[0];
    output[1] = input[1];
    output[2] = input[2];
}

pub fn project_point_on_plane(dst: &mut Vec3, p: &Vec3, normal: &Vec3) {
    let inv_denom = 1.0 / dot_product(normal, normal);
    let d = dot_product(normal, p) * inv_denom;

    let n = [
        normal[0] * inv_denom,
        normal[1] * inv_denom,
        normal[2] * inv_denom,
    ];

    dst[0] = p[0] - d * n[0];
    dst[1] = p[1] - d * n[1];
    dst[2] = p[2] - d * n[2];
}

pub fn make_normal_vectors(forward: &Vec3, right: &mut Vec3, up: &mut Vec3) {
    right[1] = -forward[0];
    right[2] = forward[1];
    right[0] = forward[2];

    let d = dot_product(right, forward);
    let mut temp = [0.0f32; 3];
    vector_ma(right, -d, forward, &mut temp);
    *right = temp;
    vector_normalize(right);
    cross_product(right, forward, up);
}

pub fn vector_rotate(input: &Vec3, matrix: &[Vec3; 3], output: &mut Vec3) {
    output[0] = dot_product(input, &matrix[0]);
    output[1] = dot_product(input, &matrix[1]);
    output[2] = dot_product(input, &matrix[2]);
}

pub fn q_rsqrt(number: f32) -> f32 {
    let x2 = number * 0.5f32;
    let mut y = number;
    let mut i: u32 = y.to_bits();
    i = 0x5f3759df - (i >> 1);
    y = f32::from_bits(i);
    y * (1.5 - (x2 * y * y))
}

pub fn q_fabs(f: f32) -> f32 {
    f.abs()
}

pub fn lerp_angle(from: f32, to: f32, frac: f32) -> f32 {
    let mut to = to;
    if to - from > 180.0 {
        to -= 360.0;
    }
    if to - from < -180.0 {
        to += 360.0;
    }
    from + frac * (to - from)
}

pub fn angle_subtract(a1: f32, a2: f32) -> f32 {
    let mut a = a1 - a2;
    while a > 180.0 {
        a -= 360.0;
    }
    while a < -180.0 {
        a += 360.0;
    }
    a
}

pub fn angles_subtract(v1: &Vec3, v2: &Vec3, v3: &mut Vec3) {
    v3[0] = angle_subtract(v1[0], v2[0]);
    v3[1] = angle_subtract(v1[1], v2[1]);
    v3[2] = angle_subtract(v1[2], v2[2]);
}

pub fn angle_mod(a: f32) -> f32 {
    (360.0 / 65536.0) * ((a * (65536.0 / 360.0)) as i32 & 65535) as f32
}

pub fn angle_normalize360(angle: f32) -> f32 {
    (360.0 / 65536.0) * ((angle * (65536.0 / 360.0)) as i32 & 65535) as f32
}

pub fn angle_normalize180(angle: f32) -> f32 {
    let mut angle = angle_normalize360(angle);
    if angle > 180.0 {
        angle -= 360.0;
    }
    angle
}

pub fn angle_delta(angle1: f32, angle2: f32) -> f32 {
    angle_normalize180(angle1 - angle2)
}

pub fn set_plane_signbits(normal: &Vec3) -> u8 {
    let mut bits = 0u8;
    for j in 0..3 {
        if normal[j] < 0.0 {
            bits |= 1 << j;
        }
    }
    bits
}

pub fn radius_from_bounds(mins: &Vec3, maxs: &Vec3) -> f32 {
    let mut corner = [0.0f32; 3];
    for i in 0..3 {
        let a = mins[i].abs();
        let b = maxs[i].abs();
        corner[i] = if a > b { a } else { b };
    }
    vector_length(&corner)
}

pub fn clear_bounds(mins: &mut Vec3, maxs: &mut Vec3) {
    *mins = [99999.0, 99999.0, 99999.0];
    *maxs = [-99999.0, -99999.0, -99999.0];
}

pub fn add_point_to_bounds(v: &Vec3, mins: &mut Vec3, maxs: &mut Vec3) {
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

pub fn vector_normalize(v: &mut Vec3) -> f32 {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length != 0.0 {
        let ilength = 1.0 / length;
        v[0] *= ilength;
        v[1] *= ilength;
        v[2] *= ilength;
    }
    length
}

pub fn vector_normalize2(v: &Vec3, out: &mut Vec3) -> f32 {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length != 0.0 {
        let ilength = 1.0 / length;
        out[0] = v[0] * ilength;
        out[1] = v[1] * ilength;
        out[2] = v[2] * ilength;
    } else {
        *out = [0.0, 0.0, 0.0];
    }
    length
}

pub fn vector_ma(veca: &Vec3, scale: f32, vecb: &Vec3, vecc: &mut Vec3) {
    vecc[0] = veca[0] + scale * vecb[0];
    vecc[1] = veca[1] + scale * vecb[1];
    vecc[2] = veca[2] + scale * vecb[2];
}

pub fn dot_product(v1: &Vec3, v2: &Vec3) -> f32 {
    v1[0] * v2[0] + v1[1] * v2[1] + v1[2] * v2[2]
}

pub fn vector_subtract(veca: &Vec3, vecb: &Vec3, out: &mut Vec3) {
    out[0] = veca[0] - vecb[0];
    out[1] = veca[1] - vecb[1];
    out[2] = veca[2] - vecb[2];
}

pub fn vector_add(veca: &Vec3, vecb: &Vec3, out: &mut Vec3) {
    out[0] = veca[0] + vecb[0];
    out[1] = veca[1] + vecb[1];
    out[2] = veca[2] + vecb[2];
}

pub fn vector_copy(input: &Vec3, output: &mut Vec3) {
    *output = *input;
}

pub fn vector_scale(input: &Vec3, scale: f32, output: &mut Vec3) {
    output[0] = input[0] * scale;
    output[1] = input[1] * scale;
    output[2] = input[2] * scale;
}

pub fn vector4_scale(input: &Vec4, scale: f32, output: &mut Vec4) {
    output[0] = input[0] * scale;
    output[1] = input[1] * scale;
    output[2] = input[2] * scale;
    output[3] = input[3] * scale;
}

pub fn q_log2(val: i32) -> i32 {
    let mut answer = 0;
    let mut val = val;
    while { val >>= 1; val } != 0 {
        answer += 1;
    }
    answer
}

pub fn matrix_multiply(in1: &[[f32; 3]; 3], in2: &[[f32; 3]; 3], out: &mut [[f32; 3]; 3]) {
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

pub fn angle_vectors(angles: &Vec3, forward: &mut Vec3, right: &mut Vec3, up: &mut Vec3) {
    let angle = angles[1] * (std::f32::consts::PI * 2.0 / 360.0);
    let sy = angle.sin();
    let cy = angle.cos();
    let angle = angles[0] * (std::f32::consts::PI * 2.0 / 360.0);
    let sp = angle.sin();
    let cp = angle.cos();
    let angle = angles[2] * (std::f32::consts::PI * 2.0 / 360.0);
    let sr = angle.sin();
    let cr = angle.cos();

    forward[0] = cp * cy;
    forward[1] = cp * sy;
    forward[2] = -sp;

    right[0] = (-1.0 * sr * sp * cy + -1.0 * cr * -sy);
    right[1] = (-1.0 * sr * sp * sy + -1.0 * cr * cy);
    right[2] = -1.0 * sr * cp;

    up[0] = (cr * sp * cy + -sr * -sy);
    up[1] = (cr * sp * sy + -sr * cy);
    up[2] = cr * cp;
}

pub fn perpendicular_vector(dst: &mut Vec3, src: &Vec3) {
    let mut pos = 0;
    let mut minelem = 1.0f32;
    let mut tempvec = [0.0f32; 3];

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

    project_point_on_plane(dst, &tempvec, src);
    vector_normalize(dst);
}

pub fn vector_length(v: &Vec3) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

pub fn vector_length_squared(v: &Vec3) -> f32 {
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}

pub fn distance(p1: &Vec3, p2: &Vec3) -> f32 {
    let mut v = [0.0f32; 3];
    vector_subtract(p2, p1, &mut v);
    vector_length(&v)
}

pub fn distance_squared(p1: &Vec3, p2: &Vec3) -> f32 {
    let mut v = [0.0f32; 3];
    vector_subtract(p2, p1, &mut v);
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}

pub fn vector_normalize_fast(v: &mut Vec3) {
    let ilength = q_rsqrt(dot_product(v, v));
    v[0] *= ilength;
    v[1] *= ilength;
    v[2] *= ilength;
}

pub fn vector_inverse(v: &mut Vec3) {
    v[0] = -v[0];
    v[1] = -v[1];
    v[2] = -v[2];
}

pub fn cross_product(v1: &Vec3, v2: &Vec3, cross: &mut Vec3) {
    cross[0] = v1[1] * v2[2] - v1[2] * v2[1];
    cross[1] = v1[2] * v2[0] - v1[0] * v2[2];
    cross[2] = v1[0] * v2[1] - v1[1] * v2[0];
}

pub fn vector_compare(v1: &Vec3, v2: &Vec3) -> bool {
    v1[0] == v2[0] && v1[1] == v2[1] && v1[2] == v2[2]
}

pub fn vector_clear(v: &mut Vec3) {
    *v = [0.0, 0.0, 0.0];
}

pub fn vector_negate(a: &Vec3, b: &mut Vec3) {
    b[0] = -a[0];
    b[1] = -a[1];
    b[2] = -a[2];
}

pub fn vector_set(v: &mut Vec3, x: f32, y: f32, z: f32) {
    v[0] = x;
    v[1] = y;
    v[2] = z;
}

pub fn vector4_copy(a: &Vec4, b: &mut Vec4) {
    b[0] = a[0];
    b[1] = a[1];
    b[2] = a[2];
    b[3] = a[3];
}

pub fn snap_vector(v: &mut Vec3) {
    v[0] = v[0] as i32 as f32;
    v[1] = v[1] as i32 as f32;
    v[2] = v[2] as i32 as f32;
}
