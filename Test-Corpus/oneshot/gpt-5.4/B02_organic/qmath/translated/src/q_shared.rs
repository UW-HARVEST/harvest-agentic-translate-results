pub type Byte = u8;
pub type QBoolean = bool;
pub type VecT = f32;
pub type Vec2T = [VecT; 2];
pub type Vec3T = [VecT; 3];
pub type Vec4T = [VecT; 4];
pub type Vec5T = [VecT; 5];

pub const PITCH: usize = 0;
pub const YAW: usize = 1;
pub const ROLL: usize = 2;
pub const NUMVERTEXNORMALS: usize = 162;
pub const M_PI: f32 = std::f32::consts::PI;
pub const PLANE_X: u8 = 0;
pub const PLANE_Y: u8 = 1;
pub const PLANE_Z: u8 = 2;
pub const PLANE_NON_AXIAL: u8 = 3;

#[derive(Clone, Copy, Debug, Default)]
pub struct CPlane {
    pub normal: Vec3T,
    pub dist: f32,
    pub type_: Byte,
    pub signbits: Byte,
    pub pad: [Byte; 2],
}

pub fn dot_product(x: &Vec3T, y: &Vec3T) -> f32 {
    x[0] * y[0] + x[1] * y[1] + x[2] * y[2]
}

pub fn vector_subtract(a: &Vec3T, b: &Vec3T) -> Vec3T {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn vector_add(a: &Vec3T, b: &Vec3T) -> Vec3T {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn vector_copy(a: &Vec3T) -> Vec3T {
    *a
}

pub fn vector_scale(v: &Vec3T, s: f32) -> Vec3T {
    [v[0] * s, v[1] * s, v[2] * s]
}

pub fn vector_ma(v: &Vec3T, s: f32, b: &Vec3T) -> Vec3T {
    [v[0] + b[0] * s, v[1] + b[1] * s, v[2] + b[2] * s]
}

pub fn vector_clear(a: &mut Vec3T) {
    *a = [0.0, 0.0, 0.0];
}

pub fn vector_negate(a: &Vec3T) -> Vec3T {
    [-a[0], -a[1], -a[2]]
}

pub fn vector_set(v: &mut Vec3T, x: f32, y: f32, z: f32) {
    *v = [x, y, z];
}

pub fn vector4_copy(a: &Vec4T) -> Vec4T {
    *a
}

pub fn vector_compare(v1: &Vec3T, v2: &Vec3T) -> i32 {
    if v1[0] != v2[0] || v1[1] != v2[1] || v1[2] != v2[2] {
        0
    } else {
        1
    }
}

pub fn vector_length(v: &Vec3T) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

pub fn vector_length_squared(v: &Vec3T) -> f32 {
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}

pub fn distance(p1: &Vec3T, p2: &Vec3T) -> f32 {
    vector_length(&vector_subtract(p2, p1))
}

pub fn distance_squared(p1: &Vec3T, p2: &Vec3T) -> f32 {
    let v = vector_subtract(p2, p1);
    v[0] * v[0] + v[1] * v[1] + v[2] * v[2]
}

pub fn VectorNormalizeFast(v: &mut Vec3T) {
    let ilength = Q_rsqrt(dot_product(v, v));
    v[0] *= ilength;
    v[1] *= ilength;
    v[2] *= ilength;
}

pub fn vector_inverse(v: &mut Vec3T) {
    v[0] = -v[0];
    v[1] = -v[1];
    v[2] = -v[2];
}

pub fn cross_product(v1: &Vec3T, v2: &Vec3T) -> Vec3T {
    [
        v1[1] * v2[2] - v1[2] * v2[1],
        v1[2] * v2[0] - v1[0] * v2[2],
        v1[0] * v2[1] - v1[1] * v2[0],
    ]
}

pub fn deg2rad(a: f32) -> f32 {
    (a * M_PI) / 180.0
}

pub fn plane_type_for_normal(x: &Vec3T) -> u8 {
    if x[0] == 1.0 {
        PLANE_X
    } else if x[1] == 1.0 {
        PLANE_Y
    } else if x[2] == 1.0 {
        PLANE_Z
    } else {
        PLANE_NON_AXIAL
    }
}

pub fn Q_rsqrt(number: f32) -> f32 {
    crate::q_math::Q_rsqrt(number)
}
