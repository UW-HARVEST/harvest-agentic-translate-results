extern "C" {
    fn atan2(__y: ::core::ffi::c_double, __x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn cos(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn sin(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn sqrt(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn fabs(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type __uint32_t = u32;
pub type size_t = usize;
pub type byte = ::core::ffi::c_uchar;
pub type qboolean = ::core::ffi::c_uint;
pub const qtrue: qboolean = 1;
pub const qfalse: qboolean = 0;
pub type vec_t = ::core::ffi::c_float;
pub type vec3_t = [vec_t; 3];
pub type vec4_t = [vec_t; 4];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cplane_s {
    pub normal: vec3_t,
    pub dist: ::core::ffi::c_float,
    pub type_0: byte,
    pub signbits: byte,
    pub pad: [byte; 2],
}
pub type uint32_t = __uint32_t;
pub type cplane_t = cplane_s;
pub const M_PI: ::core::ffi::c_double = 3.14159265358979323846f64;
pub const PITCH: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const YAW: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ROLL: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const NUMVERTEXNORMALS: ::core::ffi::c_int = 162 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn VectorLength(mut v: *const vec_t) -> vec_t {
    return sqrt(
        (*v.offset(0 as ::core::ffi::c_int as isize) * *v.offset(0 as ::core::ffi::c_int as isize)
            + *v.offset(1 as ::core::ffi::c_int as isize)
                * *v.offset(1 as ::core::ffi::c_int as isize)
            + *v.offset(2 as ::core::ffi::c_int as isize)
                * *v.offset(2 as ::core::ffi::c_int as isize)) as ::core::ffi::c_double,
    ) as vec_t;
}
#[inline]
unsafe extern "C" fn CrossProduct(
    mut v1: *const vec_t,
    mut v2: *const vec_t,
    mut cross: *mut vec_t,
) {
    *cross.offset(0 as ::core::ffi::c_int as isize) = *v1.offset(1 as ::core::ffi::c_int as isize)
        * *v2.offset(2 as ::core::ffi::c_int as isize)
        - *v1.offset(2 as ::core::ffi::c_int as isize)
            * *v2.offset(1 as ::core::ffi::c_int as isize);
    *cross.offset(1 as ::core::ffi::c_int as isize) = *v1.offset(2 as ::core::ffi::c_int as isize)
        * *v2.offset(0 as ::core::ffi::c_int as isize)
        - *v1.offset(0 as ::core::ffi::c_int as isize)
            * *v2.offset(2 as ::core::ffi::c_int as isize);
    *cross.offset(2 as ::core::ffi::c_int as isize) = *v1.offset(0 as ::core::ffi::c_int as isize)
        * *v2.offset(1 as ::core::ffi::c_int as isize)
        - *v1.offset(1 as ::core::ffi::c_int as isize)
            * *v2.offset(0 as ::core::ffi::c_int as isize);
}
#[no_mangle]
pub static mut vec3_origin: vec3_t = [
    0 as ::core::ffi::c_int as vec_t,
    0 as ::core::ffi::c_int as vec_t,
    0 as ::core::ffi::c_int as vec_t,
];
#[no_mangle]
pub static mut axisDefault: [vec3_t; 3] = [
    [
        1 as ::core::ffi::c_int as vec_t,
        0 as ::core::ffi::c_int as vec_t,
        0 as ::core::ffi::c_int as vec_t,
    ],
    [
        0 as ::core::ffi::c_int as vec_t,
        1 as ::core::ffi::c_int as vec_t,
        0 as ::core::ffi::c_int as vec_t,
    ],
    [
        0 as ::core::ffi::c_int as vec_t,
        0 as ::core::ffi::c_int as vec_t,
        1 as ::core::ffi::c_int as vec_t,
    ],
];
#[no_mangle]
pub static mut colorBlack: vec4_t = [
    0 as ::core::ffi::c_int as vec_t,
    0 as ::core::ffi::c_int as vec_t,
    0 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
];
#[no_mangle]
pub static mut colorRed: vec4_t = [
    1 as ::core::ffi::c_int as vec_t,
    0 as ::core::ffi::c_int as vec_t,
    0 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
];
#[no_mangle]
pub static mut colorGreen: vec4_t = [
    0 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
    0 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
];
#[no_mangle]
pub static mut colorBlue: vec4_t = [
    0 as ::core::ffi::c_int as vec_t,
    0 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
];
#[no_mangle]
pub static mut colorYellow: vec4_t = [
    1 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
    0 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
];
#[no_mangle]
pub static mut colorMagenta: vec4_t = [
    1 as ::core::ffi::c_int as vec_t,
    0 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
];
#[no_mangle]
pub static mut colorCyan: vec4_t = [
    0 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
];
#[no_mangle]
pub static mut colorWhite: vec4_t = [
    1 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
    1 as ::core::ffi::c_int as vec_t,
];
#[no_mangle]
pub static mut colorLtGrey: vec4_t = [
    0.75f64 as vec_t,
    0.75f64 as vec_t,
    0.75f64 as vec_t,
    1 as ::core::ffi::c_int as vec_t,
];
#[no_mangle]
pub static mut colorMdGrey: vec4_t = [
    0.5f64 as vec_t,
    0.5f64 as vec_t,
    0.5f64 as vec_t,
    1 as ::core::ffi::c_int as vec_t,
];
#[no_mangle]
pub static mut colorDkGrey: vec4_t = [
    0.25f64 as vec_t,
    0.25f64 as vec_t,
    0.25f64 as vec_t,
    1 as ::core::ffi::c_int as vec_t,
];
#[no_mangle]
pub static mut g_color_table: [vec4_t; 8] = [
    [
        0.0f64 as vec_t,
        0.0f64 as vec_t,
        0.0f64 as vec_t,
        1.0f64 as vec_t,
    ],
    [
        1.0f64 as vec_t,
        0.0f64 as vec_t,
        0.0f64 as vec_t,
        1.0f64 as vec_t,
    ],
    [
        0.0f64 as vec_t,
        1.0f64 as vec_t,
        0.0f64 as vec_t,
        1.0f64 as vec_t,
    ],
    [
        1.0f64 as vec_t,
        1.0f64 as vec_t,
        0.0f64 as vec_t,
        1.0f64 as vec_t,
    ],
    [
        0.0f64 as vec_t,
        0.0f64 as vec_t,
        1.0f64 as vec_t,
        1.0f64 as vec_t,
    ],
    [
        0.0f64 as vec_t,
        1.0f64 as vec_t,
        1.0f64 as vec_t,
        1.0f64 as vec_t,
    ],
    [
        1.0f64 as vec_t,
        0.0f64 as vec_t,
        1.0f64 as vec_t,
        1.0f64 as vec_t,
    ],
    [
        1.0f64 as vec_t,
        1.0f64 as vec_t,
        1.0f64 as vec_t,
        1.0f64 as vec_t,
    ],
];
#[no_mangle]
pub static mut bytedirs: [vec3_t; 162] = [
    [-0.525731f32, 0.000000f32, 0.850651f32],
    [-0.442863f32, 0.238856f32, 0.864188f32],
    [-0.295242f32, 0.000000f32, 0.955423f32],
    [-0.309017f32, 0.500000f32, 0.809017f32],
    [-0.162460f32, 0.262866f32, 0.951056f32],
    [0.000000f32, 0.000000f32, 1.000000f32],
    [0.000000f32, 0.850651f32, 0.525731f32],
    [-0.147621f32, 0.716567f32, 0.681718f32],
    [0.147621f32, 0.716567f32, 0.681718f32],
    [0.000000f32, 0.525731f32, 0.850651f32],
    [0.309017f32, 0.500000f32, 0.809017f32],
    [0.525731f32, 0.000000f32, 0.850651f32],
    [0.295242f32, 0.000000f32, 0.955423f32],
    [0.442863f32, 0.238856f32, 0.864188f32],
    [0.162460f32, 0.262866f32, 0.951056f32],
    [-0.681718f32, 0.147621f32, 0.716567f32],
    [-0.809017f32, 0.309017f32, 0.500000f32],
    [-0.587785f32, 0.425325f32, 0.688191f32],
    [-0.850651f32, 0.525731f32, 0.000000f32],
    [-0.864188f32, 0.442863f32, 0.238856f32],
    [-0.716567f32, 0.681718f32, 0.147621f32],
    [-0.688191f32, 0.587785f32, 0.425325f32],
    [-0.500000f32, 0.809017f32, 0.309017f32],
    [-0.238856f32, 0.864188f32, 0.442863f32],
    [-0.425325f32, 0.688191f32, 0.587785f32],
    [-0.716567f32, 0.681718f32, -0.147621f32],
    [-0.500000f32, 0.809017f32, -0.309017f32],
    [-0.525731f32, 0.850651f32, 0.000000f32],
    [0.000000f32, 0.850651f32, -0.525731f32],
    [-0.238856f32, 0.864188f32, -0.442863f32],
    [0.000000f32, 0.955423f32, -0.295242f32],
    [-0.262866f32, 0.951056f32, -0.162460f32],
    [0.000000f32, 1.000000f32, 0.000000f32],
    [0.000000f32, 0.955423f32, 0.295242f32],
    [-0.262866f32, 0.951056f32, 0.162460f32],
    [0.238856f32, 0.864188f32, 0.442863f32],
    [0.262866f32, 0.951056f32, 0.162460f32],
    [0.500000f32, 0.809017f32, 0.309017f32],
    [0.238856f32, 0.864188f32, -0.442863f32],
    [0.262866f32, 0.951056f32, -0.162460f32],
    [0.500000f32, 0.809017f32, -0.309017f32],
    [0.850651f32, 0.525731f32, 0.000000f32],
    [0.716567f32, 0.681718f32, 0.147621f32],
    [0.716567f32, 0.681718f32, -0.147621f32],
    [0.525731f32, 0.850651f32, 0.000000f32],
    [0.425325f32, 0.688191f32, 0.587785f32],
    [0.864188f32, 0.442863f32, 0.238856f32],
    [0.688191f32, 0.587785f32, 0.425325f32],
    [0.809017f32, 0.309017f32, 0.500000f32],
    [0.681718f32, 0.147621f32, 0.716567f32],
    [0.587785f32, 0.425325f32, 0.688191f32],
    [0.955423f32, 0.295242f32, 0.000000f32],
    [1.000000f32, 0.000000f32, 0.000000f32],
    [0.951056f32, 0.162460f32, 0.262866f32],
    [0.850651f32, -0.525731f32, 0.000000f32],
    [0.955423f32, -0.295242f32, 0.000000f32],
    [0.864188f32, -0.442863f32, 0.238856f32],
    [0.951056f32, -0.162460f32, 0.262866f32],
    [0.809017f32, -0.309017f32, 0.500000f32],
    [0.681718f32, -0.147621f32, 0.716567f32],
    [0.850651f32, 0.000000f32, 0.525731f32],
    [0.864188f32, 0.442863f32, -0.238856f32],
    [0.809017f32, 0.309017f32, -0.500000f32],
    [0.951056f32, 0.162460f32, -0.262866f32],
    [0.525731f32, 0.000000f32, -0.850651f32],
    [0.681718f32, 0.147621f32, -0.716567f32],
    [0.681718f32, -0.147621f32, -0.716567f32],
    [0.850651f32, 0.000000f32, -0.525731f32],
    [0.809017f32, -0.309017f32, -0.500000f32],
    [0.864188f32, -0.442863f32, -0.238856f32],
    [0.951056f32, -0.162460f32, -0.262866f32],
    [0.147621f32, 0.716567f32, -0.681718f32],
    [0.309017f32, 0.500000f32, -0.809017f32],
    [0.425325f32, 0.688191f32, -0.587785f32],
    [0.442863f32, 0.238856f32, -0.864188f32],
    [0.587785f32, 0.425325f32, -0.688191f32],
    [0.688191f32, 0.587785f32, -0.425325f32],
    [-0.147621f32, 0.716567f32, -0.681718f32],
    [-0.309017f32, 0.500000f32, -0.809017f32],
    [0.000000f32, 0.525731f32, -0.850651f32],
    [-0.525731f32, 0.000000f32, -0.850651f32],
    [-0.442863f32, 0.238856f32, -0.864188f32],
    [-0.295242f32, 0.000000f32, -0.955423f32],
    [-0.162460f32, 0.262866f32, -0.951056f32],
    [0.000000f32, 0.000000f32, -1.000000f32],
    [0.295242f32, 0.000000f32, -0.955423f32],
    [0.162460f32, 0.262866f32, -0.951056f32],
    [-0.442863f32, -0.238856f32, -0.864188f32],
    [-0.309017f32, -0.500000f32, -0.809017f32],
    [-0.162460f32, -0.262866f32, -0.951056f32],
    [0.000000f32, -0.850651f32, -0.525731f32],
    [-0.147621f32, -0.716567f32, -0.681718f32],
    [0.147621f32, -0.716567f32, -0.681718f32],
    [0.000000f32, -0.525731f32, -0.850651f32],
    [0.309017f32, -0.500000f32, -0.809017f32],
    [0.442863f32, -0.238856f32, -0.864188f32],
    [0.162460f32, -0.262866f32, -0.951056f32],
    [0.238856f32, -0.864188f32, -0.442863f32],
    [0.500000f32, -0.809017f32, -0.309017f32],
    [0.425325f32, -0.688191f32, -0.587785f32],
    [0.716567f32, -0.681718f32, -0.147621f32],
    [0.688191f32, -0.587785f32, -0.425325f32],
    [0.587785f32, -0.425325f32, -0.688191f32],
    [0.000000f32, -0.955423f32, -0.295242f32],
    [0.000000f32, -1.000000f32, 0.000000f32],
    [0.262866f32, -0.951056f32, -0.162460f32],
    [0.000000f32, -0.850651f32, 0.525731f32],
    [0.000000f32, -0.955423f32, 0.295242f32],
    [0.238856f32, -0.864188f32, 0.442863f32],
    [0.262866f32, -0.951056f32, 0.162460f32],
    [0.500000f32, -0.809017f32, 0.309017f32],
    [0.716567f32, -0.681718f32, 0.147621f32],
    [0.525731f32, -0.850651f32, 0.000000f32],
    [-0.238856f32, -0.864188f32, -0.442863f32],
    [-0.500000f32, -0.809017f32, -0.309017f32],
    [-0.262866f32, -0.951056f32, -0.162460f32],
    [-0.850651f32, -0.525731f32, 0.000000f32],
    [-0.716567f32, -0.681718f32, -0.147621f32],
    [-0.716567f32, -0.681718f32, 0.147621f32],
    [-0.525731f32, -0.850651f32, 0.000000f32],
    [-0.500000f32, -0.809017f32, 0.309017f32],
    [-0.238856f32, -0.864188f32, 0.442863f32],
    [-0.262866f32, -0.951056f32, 0.162460f32],
    [-0.864188f32, -0.442863f32, 0.238856f32],
    [-0.809017f32, -0.309017f32, 0.500000f32],
    [-0.688191f32, -0.587785f32, 0.425325f32],
    [-0.681718f32, -0.147621f32, 0.716567f32],
    [-0.442863f32, -0.238856f32, 0.864188f32],
    [-0.587785f32, -0.425325f32, 0.688191f32],
    [-0.309017f32, -0.500000f32, 0.809017f32],
    [-0.147621f32, -0.716567f32, 0.681718f32],
    [-0.425325f32, -0.688191f32, 0.587785f32],
    [-0.162460f32, -0.262866f32, 0.951056f32],
    [0.442863f32, -0.238856f32, 0.864188f32],
    [0.162460f32, -0.262866f32, 0.951056f32],
    [0.309017f32, -0.500000f32, 0.809017f32],
    [0.147621f32, -0.716567f32, 0.681718f32],
    [0.000000f32, -0.525731f32, 0.850651f32],
    [0.425325f32, -0.688191f32, 0.587785f32],
    [0.587785f32, -0.425325f32, 0.688191f32],
    [0.688191f32, -0.587785f32, 0.425325f32],
    [-0.955423f32, 0.295242f32, 0.000000f32],
    [-0.951056f32, 0.162460f32, 0.262866f32],
    [-1.000000f32, 0.000000f32, 0.000000f32],
    [-0.850651f32, 0.000000f32, 0.525731f32],
    [-0.955423f32, -0.295242f32, 0.000000f32],
    [-0.951056f32, -0.162460f32, 0.262866f32],
    [-0.864188f32, 0.442863f32, -0.238856f32],
    [-0.951056f32, 0.162460f32, -0.262866f32],
    [-0.809017f32, 0.309017f32, -0.500000f32],
    [-0.864188f32, -0.442863f32, -0.238856f32],
    [-0.951056f32, -0.162460f32, -0.262866f32],
    [-0.809017f32, -0.309017f32, -0.500000f32],
    [-0.681718f32, 0.147621f32, -0.716567f32],
    [-0.681718f32, -0.147621f32, -0.716567f32],
    [-0.850651f32, 0.000000f32, -0.525731f32],
    [-0.688191f32, 0.587785f32, -0.425325f32],
    [-0.587785f32, 0.425325f32, -0.688191f32],
    [-0.425325f32, 0.688191f32, -0.587785f32],
    [-0.425325f32, -0.688191f32, -0.587785f32],
    [-0.587785f32, -0.425325f32, -0.688191f32],
    [-0.688191f32, -0.587785f32, -0.425325f32],
];
#[no_mangle]
pub unsafe extern "C" fn Q_rand(mut seed: *mut ::core::ffi::c_int) -> ::core::ffi::c_int {
    *seed = 69069 as ::core::ffi::c_int * *seed + 1 as ::core::ffi::c_int;
    return *seed;
}
#[no_mangle]
pub unsafe extern "C" fn Q_random(mut seed: *mut ::core::ffi::c_int) -> ::core::ffi::c_float {
    return (Q_rand(seed) & 0xffff as ::core::ffi::c_int) as ::core::ffi::c_float
        / 0x10000 as ::core::ffi::c_int as ::core::ffi::c_float;
}
#[no_mangle]
pub unsafe extern "C" fn Q_crandom(mut seed: *mut ::core::ffi::c_int) -> ::core::ffi::c_float {
    return (2.0f64 * (Q_random(seed) as ::core::ffi::c_double - 0.5f64)) as ::core::ffi::c_float;
}
#[no_mangle]
pub unsafe extern "C" fn ClampChar(mut i: ::core::ffi::c_int) -> ::core::ffi::c_schar {
    if i < -(128 as ::core::ffi::c_int) {
        return -(128 as ::core::ffi::c_int) as ::core::ffi::c_schar;
    }
    if i > 127 as ::core::ffi::c_int {
        return 127 as ::core::ffi::c_schar;
    }
    return i as ::core::ffi::c_schar;
}
#[no_mangle]
pub unsafe extern "C" fn ClampShort(mut i: ::core::ffi::c_int) -> ::core::ffi::c_short {
    if i < -(32768 as ::core::ffi::c_int) {
        return -(32768 as ::core::ffi::c_int) as ::core::ffi::c_short;
    }
    if i > 0x7fff as ::core::ffi::c_int {
        return 0x7fff as ::core::ffi::c_short;
    }
    return i as ::core::ffi::c_short;
}
#[no_mangle]
pub unsafe extern "C" fn DirToByte(mut dir: *mut vec_t) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_int = 0;
    let mut best: ::core::ffi::c_int = 0;
    let mut d: ::core::ffi::c_float = 0.;
    let mut bestd: ::core::ffi::c_float = 0.;
    if dir.is_null() {
        return 0 as ::core::ffi::c_int;
    }
    bestd = 0 as ::core::ffi::c_int as ::core::ffi::c_float;
    best = 0 as ::core::ffi::c_int;
    i = 0 as ::core::ffi::c_int;
    while i < NUMVERTEXNORMALS {
        d = (*dir.offset(0 as ::core::ffi::c_int as isize)
            * bytedirs[i as usize][0 as ::core::ffi::c_int as usize]
            + *dir.offset(1 as ::core::ffi::c_int as isize)
                * bytedirs[i as usize][1 as ::core::ffi::c_int as usize]
            + *dir.offset(2 as ::core::ffi::c_int as isize)
                * bytedirs[i as usize][2 as ::core::ffi::c_int as usize])
            as ::core::ffi::c_float;
        if d > bestd {
            bestd = d;
            best = i;
        }
        i += 1;
    }
    return best;
}
#[no_mangle]
pub unsafe extern "C" fn ByteToDir(mut b: ::core::ffi::c_int, mut dir: *mut vec_t) {
    if b < 0 as ::core::ffi::c_int || b >= NUMVERTEXNORMALS {
        *dir.offset(0 as ::core::ffi::c_int as isize) =
            vec3_origin[0 as ::core::ffi::c_int as usize];
        *dir.offset(1 as ::core::ffi::c_int as isize) =
            vec3_origin[1 as ::core::ffi::c_int as usize];
        *dir.offset(2 as ::core::ffi::c_int as isize) =
            vec3_origin[2 as ::core::ffi::c_int as usize];
        return;
    }
    *dir.offset(0 as ::core::ffi::c_int as isize) =
        bytedirs[b as usize][0 as ::core::ffi::c_int as usize];
    *dir.offset(1 as ::core::ffi::c_int as isize) =
        bytedirs[b as usize][1 as ::core::ffi::c_int as usize];
    *dir.offset(2 as ::core::ffi::c_int as isize) =
        bytedirs[b as usize][2 as ::core::ffi::c_int as usize];
}
#[no_mangle]
pub unsafe extern "C" fn ColorBytes3(
    mut r: ::core::ffi::c_float,
    mut g: ::core::ffi::c_float,
    mut b: ::core::ffi::c_float,
) -> ::core::ffi::c_uint {
    let mut i: ::core::ffi::c_uint = 0;
    *(&raw mut i as *mut byte).offset(0 as ::core::ffi::c_int as isize) =
        (r * 255 as ::core::ffi::c_int as ::core::ffi::c_float) as byte;
    *(&raw mut i as *mut byte).offset(1 as ::core::ffi::c_int as isize) =
        (g * 255 as ::core::ffi::c_int as ::core::ffi::c_float) as byte;
    *(&raw mut i as *mut byte).offset(2 as ::core::ffi::c_int as isize) =
        (b * 255 as ::core::ffi::c_int as ::core::ffi::c_float) as byte;
    return i;
}
#[no_mangle]
pub unsafe extern "C" fn ColorBytes4(
    mut r: ::core::ffi::c_float,
    mut g: ::core::ffi::c_float,
    mut b: ::core::ffi::c_float,
    mut a: ::core::ffi::c_float,
) -> ::core::ffi::c_uint {
    let mut i: ::core::ffi::c_uint = 0;
    *(&raw mut i as *mut byte).offset(0 as ::core::ffi::c_int as isize) =
        (r * 255 as ::core::ffi::c_int as ::core::ffi::c_float) as byte;
    *(&raw mut i as *mut byte).offset(1 as ::core::ffi::c_int as isize) =
        (g * 255 as ::core::ffi::c_int as ::core::ffi::c_float) as byte;
    *(&raw mut i as *mut byte).offset(2 as ::core::ffi::c_int as isize) =
        (b * 255 as ::core::ffi::c_int as ::core::ffi::c_float) as byte;
    *(&raw mut i as *mut byte).offset(3 as ::core::ffi::c_int as isize) =
        (a * 255 as ::core::ffi::c_int as ::core::ffi::c_float) as byte;
    return i;
}
#[no_mangle]
pub unsafe extern "C" fn NormalizeColor(
    mut in_0: *const vec_t,
    mut out: *mut vec_t,
) -> ::core::ffi::c_float {
    let mut max: ::core::ffi::c_float = 0.;
    max = *in_0.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_float;
    if *in_0.offset(1 as ::core::ffi::c_int as isize) > max {
        max = *in_0.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_float;
    }
    if *in_0.offset(2 as ::core::ffi::c_int as isize) > max {
        max = *in_0.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_float;
    }
    if max == 0. {
        let ref mut fresh0 = *out.offset(2 as ::core::ffi::c_int as isize);
        *fresh0 = 0 as ::core::ffi::c_int as vec_t;
        let ref mut fresh1 = *out.offset(1 as ::core::ffi::c_int as isize);
        *fresh1 = *fresh0;
        *out.offset(0 as ::core::ffi::c_int as isize) = *fresh1;
    } else {
        *out.offset(0 as ::core::ffi::c_int as isize) =
            (*in_0.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_float / max) as vec_t;
        *out.offset(1 as ::core::ffi::c_int as isize) =
            (*in_0.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_float / max) as vec_t;
        *out.offset(2 as ::core::ffi::c_int as isize) =
            (*in_0.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_float / max) as vec_t;
    }
    return max;
}
#[no_mangle]
pub unsafe extern "C" fn PlaneFromPoints(
    mut plane: *mut vec_t,
    mut a: *const vec_t,
    mut b: *const vec_t,
    mut c: *const vec_t,
) -> qboolean {
    let mut d1: vec3_t = [0.; 3];
    let mut d2: vec3_t = [0.; 3];
    d1[0 as ::core::ffi::c_int as usize] =
        *b.offset(0 as ::core::ffi::c_int as isize) - *a.offset(0 as ::core::ffi::c_int as isize);
    d1[1 as ::core::ffi::c_int as usize] =
        *b.offset(1 as ::core::ffi::c_int as isize) - *a.offset(1 as ::core::ffi::c_int as isize);
    d1[2 as ::core::ffi::c_int as usize] =
        *b.offset(2 as ::core::ffi::c_int as isize) - *a.offset(2 as ::core::ffi::c_int as isize);
    d2[0 as ::core::ffi::c_int as usize] =
        *c.offset(0 as ::core::ffi::c_int as isize) - *a.offset(0 as ::core::ffi::c_int as isize);
    d2[1 as ::core::ffi::c_int as usize] =
        *c.offset(1 as ::core::ffi::c_int as isize) - *a.offset(1 as ::core::ffi::c_int as isize);
    d2[2 as ::core::ffi::c_int as usize] =
        *c.offset(2 as ::core::ffi::c_int as isize) - *a.offset(2 as ::core::ffi::c_int as isize);
    CrossProduct(
        &raw mut d2 as *mut vec_t as *const vec_t,
        &raw mut d1 as *mut vec_t as *const vec_t,
        plane as *mut vec_t,
    );
    if VectorNormalize(plane as *mut vec_t) == 0 as ::core::ffi::c_int as ::core::ffi::c_float {
        return qfalse;
    }
    *plane.offset(3 as ::core::ffi::c_int as isize) = *a.offset(0 as ::core::ffi::c_int as isize)
        * *plane.offset(0 as ::core::ffi::c_int as isize)
        + *a.offset(1 as ::core::ffi::c_int as isize)
            * *plane.offset(1 as ::core::ffi::c_int as isize)
        + *a.offset(2 as ::core::ffi::c_int as isize)
            * *plane.offset(2 as ::core::ffi::c_int as isize);
    return qtrue;
}
#[no_mangle]
pub unsafe extern "C" fn RotatePointAroundVector(
    mut dst: *mut vec_t,
    mut dir: *const vec_t,
    mut point: *const vec_t,
    mut degrees: ::core::ffi::c_float,
) {
    let mut m: [[::core::ffi::c_float; 3]; 3] = [[0.; 3]; 3];
    let mut im: [[::core::ffi::c_float; 3]; 3] = [[0.; 3]; 3];
    let mut zrot: [[::core::ffi::c_float; 3]; 3] = [[0.; 3]; 3];
    let mut tmpmat: [[::core::ffi::c_float; 3]; 3] = [[0.; 3]; 3];
    let mut rot: [[::core::ffi::c_float; 3]; 3] = [[0.; 3]; 3];
    let mut i: ::core::ffi::c_int = 0;
    let mut vr: vec3_t = [0.; 3];
    let mut vup: vec3_t = [0.; 3];
    let mut vf: vec3_t = [0.; 3];
    let mut rad: ::core::ffi::c_float = 0.;
    vf[0 as ::core::ffi::c_int as usize] = *dir.offset(0 as ::core::ffi::c_int as isize);
    vf[1 as ::core::ffi::c_int as usize] = *dir.offset(1 as ::core::ffi::c_int as isize);
    vf[2 as ::core::ffi::c_int as usize] = *dir.offset(2 as ::core::ffi::c_int as isize);
    PerpendicularVector(&raw mut vr as *mut vec_t, dir);
    CrossProduct(
        &raw mut vr as *mut vec_t as *const vec_t,
        &raw mut vf as *mut vec_t as *const vec_t,
        &raw mut vup as *mut vec_t,
    );
    m[0 as ::core::ffi::c_int as usize][0 as ::core::ffi::c_int as usize] =
        vr[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_float;
    m[1 as ::core::ffi::c_int as usize][0 as ::core::ffi::c_int as usize] =
        vr[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_float;
    m[2 as ::core::ffi::c_int as usize][0 as ::core::ffi::c_int as usize] =
        vr[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_float;
    m[0 as ::core::ffi::c_int as usize][1 as ::core::ffi::c_int as usize] =
        vup[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_float;
    m[1 as ::core::ffi::c_int as usize][1 as ::core::ffi::c_int as usize] =
        vup[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_float;
    m[2 as ::core::ffi::c_int as usize][1 as ::core::ffi::c_int as usize] =
        vup[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_float;
    m[0 as ::core::ffi::c_int as usize][2 as ::core::ffi::c_int as usize] =
        vf[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_float;
    m[1 as ::core::ffi::c_int as usize][2 as ::core::ffi::c_int as usize] =
        vf[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_float;
    m[2 as ::core::ffi::c_int as usize][2 as ::core::ffi::c_int as usize] =
        vf[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_float;
    memcpy(
        &raw mut im as *mut [::core::ffi::c_float; 3] as *mut ::core::ffi::c_void,
        &raw mut m as *mut [::core::ffi::c_float; 3] as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[[::core::ffi::c_float; 3]; 3]>() as size_t,
    );
    im[0 as ::core::ffi::c_int as usize][1 as ::core::ffi::c_int as usize] =
        m[1 as ::core::ffi::c_int as usize][0 as ::core::ffi::c_int as usize];
    im[0 as ::core::ffi::c_int as usize][2 as ::core::ffi::c_int as usize] =
        m[2 as ::core::ffi::c_int as usize][0 as ::core::ffi::c_int as usize];
    im[1 as ::core::ffi::c_int as usize][0 as ::core::ffi::c_int as usize] =
        m[0 as ::core::ffi::c_int as usize][1 as ::core::ffi::c_int as usize];
    im[1 as ::core::ffi::c_int as usize][2 as ::core::ffi::c_int as usize] =
        m[2 as ::core::ffi::c_int as usize][1 as ::core::ffi::c_int as usize];
    im[2 as ::core::ffi::c_int as usize][0 as ::core::ffi::c_int as usize] =
        m[0 as ::core::ffi::c_int as usize][2 as ::core::ffi::c_int as usize];
    im[2 as ::core::ffi::c_int as usize][1 as ::core::ffi::c_int as usize] =
        m[1 as ::core::ffi::c_int as usize][2 as ::core::ffi::c_int as usize];
    memset(
        &raw mut zrot as *mut [::core::ffi::c_float; 3] as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<[[::core::ffi::c_float; 3]; 3]>() as size_t,
    );
    zrot[2 as ::core::ffi::c_int as usize][2 as ::core::ffi::c_int as usize] = 1.0f32;
    zrot[1 as ::core::ffi::c_int as usize][1 as ::core::ffi::c_int as usize] =
        zrot[2 as ::core::ffi::c_int as usize][2 as ::core::ffi::c_int as usize];
    zrot[0 as ::core::ffi::c_int as usize][0 as ::core::ffi::c_int as usize] =
        zrot[1 as ::core::ffi::c_int as usize][1 as ::core::ffi::c_int as usize];
    rad = (degrees as ::core::ffi::c_double * M_PI / 180.0f64) as ::core::ffi::c_float;
    zrot[0 as ::core::ffi::c_int as usize][0 as ::core::ffi::c_int as usize] =
        cos(rad as ::core::ffi::c_double) as ::core::ffi::c_float;
    zrot[0 as ::core::ffi::c_int as usize][1 as ::core::ffi::c_int as usize] =
        sin(rad as ::core::ffi::c_double) as ::core::ffi::c_float;
    zrot[1 as ::core::ffi::c_int as usize][0 as ::core::ffi::c_int as usize] =
        -sin(rad as ::core::ffi::c_double) as ::core::ffi::c_float;
    zrot[1 as ::core::ffi::c_int as usize][1 as ::core::ffi::c_int as usize] =
        cos(rad as ::core::ffi::c_double) as ::core::ffi::c_float;
    MatrixMultiply(
        &raw mut m as *mut [::core::ffi::c_float; 3],
        &raw mut zrot as *mut [::core::ffi::c_float; 3],
        &raw mut tmpmat as *mut [::core::ffi::c_float; 3],
    );
    MatrixMultiply(
        &raw mut tmpmat as *mut [::core::ffi::c_float; 3],
        &raw mut im as *mut [::core::ffi::c_float; 3],
        &raw mut rot as *mut [::core::ffi::c_float; 3],
    );
    i = 0 as ::core::ffi::c_int;
    while i < 3 as ::core::ffi::c_int {
        *dst.offset(i as isize) = rot[i as usize][0 as ::core::ffi::c_int as usize]
            * *point.offset(0 as ::core::ffi::c_int as isize)
            + rot[i as usize][1 as ::core::ffi::c_int as usize]
                * *point.offset(1 as ::core::ffi::c_int as isize)
            + rot[i as usize][2 as ::core::ffi::c_int as usize]
                * *point.offset(2 as ::core::ffi::c_int as isize);
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn RotateAroundDirection(
    mut axis: *mut vec3_t,
    mut yaw: ::core::ffi::c_float,
) {
    PerpendicularVector(
        &raw mut *axis.offset(1 as ::core::ffi::c_int as isize) as *mut vec_t,
        &raw mut *axis.offset(0 as ::core::ffi::c_int as isize) as *mut vec_t as *const vec_t,
    );
    if yaw != 0. {
        let mut temp: vec3_t = [0.; 3];
        temp[0 as ::core::ffi::c_int as usize] =
            (*axis.offset(1 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize];
        temp[1 as ::core::ffi::c_int as usize] =
            (*axis.offset(1 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize];
        temp[2 as ::core::ffi::c_int as usize] =
            (*axis.offset(1 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize];
        RotatePointAroundVector(
            &raw mut *axis.offset(1 as ::core::ffi::c_int as isize) as *mut vec_t,
            &raw mut *axis.offset(0 as ::core::ffi::c_int as isize) as *mut vec_t as *const vec_t,
            &raw mut temp as *mut vec_t as *const vec_t,
            yaw,
        );
    }
    CrossProduct(
        &raw mut *axis.offset(0 as ::core::ffi::c_int as isize) as *mut vec_t as *const vec_t,
        &raw mut *axis.offset(1 as ::core::ffi::c_int as isize) as *mut vec_t as *const vec_t,
        &raw mut *axis.offset(2 as ::core::ffi::c_int as isize) as *mut vec_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn vectoangles(mut value1: *const vec_t, mut angles: *mut vec_t) {
    let mut forward: ::core::ffi::c_float = 0.;
    let mut yaw: ::core::ffi::c_float = 0.;
    let mut pitch: ::core::ffi::c_float = 0.;
    if *value1.offset(1 as ::core::ffi::c_int as isize)
        == 0 as ::core::ffi::c_int as ::core::ffi::c_float
        && *value1.offset(0 as ::core::ffi::c_int as isize)
            == 0 as ::core::ffi::c_int as ::core::ffi::c_float
    {
        yaw = 0 as ::core::ffi::c_int as ::core::ffi::c_float;
        if *value1.offset(2 as ::core::ffi::c_int as isize)
            > 0 as ::core::ffi::c_int as ::core::ffi::c_float
        {
            pitch = 90 as ::core::ffi::c_int as ::core::ffi::c_float;
        } else {
            pitch = 270 as ::core::ffi::c_int as ::core::ffi::c_float;
        }
    } else {
        if *value1.offset(0 as ::core::ffi::c_int as isize) != 0. {
            yaw = (atan2(
                *value1.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_double,
                *value1.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_double,
            ) * 180 as ::core::ffi::c_int as ::core::ffi::c_double
                / M_PI) as ::core::ffi::c_float;
        } else if *value1.offset(1 as ::core::ffi::c_int as isize)
            > 0 as ::core::ffi::c_int as ::core::ffi::c_float
        {
            yaw = 90 as ::core::ffi::c_int as ::core::ffi::c_float;
        } else {
            yaw = 270 as ::core::ffi::c_int as ::core::ffi::c_float;
        }
        if yaw < 0 as ::core::ffi::c_int as ::core::ffi::c_float {
            yaw += 360 as ::core::ffi::c_int as ::core::ffi::c_float;
        }
        forward = sqrt(
            (*value1.offset(0 as ::core::ffi::c_int as isize)
                * *value1.offset(0 as ::core::ffi::c_int as isize)
                + *value1.offset(1 as ::core::ffi::c_int as isize)
                    * *value1.offset(1 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_double,
        ) as ::core::ffi::c_float;
        pitch = (atan2(
            *value1.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_double,
            forward as ::core::ffi::c_double,
        ) * 180 as ::core::ffi::c_int as ::core::ffi::c_double
            / M_PI) as ::core::ffi::c_float;
        if pitch < 0 as ::core::ffi::c_int as ::core::ffi::c_float {
            pitch += 360 as ::core::ffi::c_int as ::core::ffi::c_float;
        }
    }
    *angles.offset(PITCH as isize) = -pitch as vec_t;
    *angles.offset(YAW as isize) = yaw as vec_t;
    *angles.offset(ROLL as isize) = 0 as ::core::ffi::c_int as vec_t;
}
#[no_mangle]
pub unsafe extern "C" fn AnglesToAxis(mut angles: *const vec_t, mut axis: *mut vec3_t) {
    let mut right: vec3_t = [0.; 3];
    AngleVectors(
        angles,
        &raw mut *axis.offset(0 as ::core::ffi::c_int as isize) as *mut vec_t,
        &raw mut right as *mut vec_t,
        &raw mut *axis.offset(2 as ::core::ffi::c_int as isize) as *mut vec_t,
    );
    (*axis.offset(1 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize] =
        vec3_origin[0 as ::core::ffi::c_int as usize] - right[0 as ::core::ffi::c_int as usize];
    (*axis.offset(1 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize] =
        vec3_origin[1 as ::core::ffi::c_int as usize] - right[1 as ::core::ffi::c_int as usize];
    (*axis.offset(1 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize] =
        vec3_origin[2 as ::core::ffi::c_int as usize] - right[2 as ::core::ffi::c_int as usize];
}
#[no_mangle]
pub unsafe extern "C" fn AxisClear(mut axis: *mut vec3_t) {
    (*axis.offset(0 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize] =
        1 as ::core::ffi::c_int as vec_t;
    (*axis.offset(0 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize] =
        0 as ::core::ffi::c_int as vec_t;
    (*axis.offset(0 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize] =
        0 as ::core::ffi::c_int as vec_t;
    (*axis.offset(1 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize] =
        0 as ::core::ffi::c_int as vec_t;
    (*axis.offset(1 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize] =
        1 as ::core::ffi::c_int as vec_t;
    (*axis.offset(1 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize] =
        0 as ::core::ffi::c_int as vec_t;
    (*axis.offset(2 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize] =
        0 as ::core::ffi::c_int as vec_t;
    (*axis.offset(2 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize] =
        0 as ::core::ffi::c_int as vec_t;
    (*axis.offset(2 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize] =
        1 as ::core::ffi::c_int as vec_t;
}
#[no_mangle]
pub unsafe extern "C" fn AxisCopy(mut in_0: *mut vec3_t, mut out: *mut vec3_t) {
    (*out.offset(0 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize] =
        (*in_0.offset(0 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize];
    (*out.offset(0 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize] =
        (*in_0.offset(0 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize];
    (*out.offset(0 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize] =
        (*in_0.offset(0 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize];
    (*out.offset(1 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize] =
        (*in_0.offset(1 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize];
    (*out.offset(1 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize] =
        (*in_0.offset(1 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize];
    (*out.offset(1 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize] =
        (*in_0.offset(1 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize];
    (*out.offset(2 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize] =
        (*in_0.offset(2 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize];
    (*out.offset(2 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize] =
        (*in_0.offset(2 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize];
    (*out.offset(2 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize] =
        (*in_0.offset(2 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize];
}
#[no_mangle]
pub unsafe extern "C" fn ProjectPointOnPlane(
    mut dst: *mut vec_t,
    mut p: *const vec_t,
    mut normal: *const vec_t,
) {
    let mut d: ::core::ffi::c_float = 0.;
    let mut n: vec3_t = [0.; 3];
    let mut inv_denom: ::core::ffi::c_float = 0.;
    inv_denom = (*normal.offset(0 as ::core::ffi::c_int as isize)
        * *normal.offset(0 as ::core::ffi::c_int as isize)
        + *normal.offset(1 as ::core::ffi::c_int as isize)
            * *normal.offset(1 as ::core::ffi::c_int as isize)
        + *normal.offset(2 as ::core::ffi::c_int as isize)
            * *normal.offset(2 as ::core::ffi::c_int as isize))
        as ::core::ffi::c_float;
    inv_denom = 1.0f32 / inv_denom;
    d = (*normal.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_float
        * *p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_float
        + *normal.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_float
            * *p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_float
        + *normal.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_float
            * *p.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_float)
        * inv_denom;
    n[0 as ::core::ffi::c_int as usize] = (*normal.offset(0 as ::core::ffi::c_int as isize)
        as ::core::ffi::c_float
        * inv_denom) as vec_t;
    n[1 as ::core::ffi::c_int as usize] = (*normal.offset(1 as ::core::ffi::c_int as isize)
        as ::core::ffi::c_float
        * inv_denom) as vec_t;
    n[2 as ::core::ffi::c_int as usize] = (*normal.offset(2 as ::core::ffi::c_int as isize)
        as ::core::ffi::c_float
        * inv_denom) as vec_t;
    *dst.offset(0 as ::core::ffi::c_int as isize) = *p.offset(0 as ::core::ffi::c_int as isize)
        - d as vec_t * n[0 as ::core::ffi::c_int as usize];
    *dst.offset(1 as ::core::ffi::c_int as isize) = *p.offset(1 as ::core::ffi::c_int as isize)
        - d as vec_t * n[1 as ::core::ffi::c_int as usize];
    *dst.offset(2 as ::core::ffi::c_int as isize) = *p.offset(2 as ::core::ffi::c_int as isize)
        - d as vec_t * n[2 as ::core::ffi::c_int as usize];
}
#[no_mangle]
pub unsafe extern "C" fn MakeNormalVectors(
    mut forward: *const vec_t,
    mut right: *mut vec_t,
    mut up: *mut vec_t,
) {
    let mut d: ::core::ffi::c_float = 0.;
    *right.offset(1 as ::core::ffi::c_int as isize) =
        -*forward.offset(0 as ::core::ffi::c_int as isize);
    *right.offset(2 as ::core::ffi::c_int as isize) =
        *forward.offset(1 as ::core::ffi::c_int as isize);
    *right.offset(0 as ::core::ffi::c_int as isize) =
        *forward.offset(2 as ::core::ffi::c_int as isize);
    d = (*right.offset(0 as ::core::ffi::c_int as isize)
        * *forward.offset(0 as ::core::ffi::c_int as isize)
        + *right.offset(1 as ::core::ffi::c_int as isize)
            * *forward.offset(1 as ::core::ffi::c_int as isize)
        + *right.offset(2 as ::core::ffi::c_int as isize)
            * *forward.offset(2 as ::core::ffi::c_int as isize)) as ::core::ffi::c_float;
    *right.offset(0 as ::core::ffi::c_int as isize) =
        (*right.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_float
            + *forward.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_float * -d)
            as vec_t;
    *right.offset(1 as ::core::ffi::c_int as isize) =
        (*right.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_float
            + *forward.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_float * -d)
            as vec_t;
    *right.offset(2 as ::core::ffi::c_int as isize) =
        (*right.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_float
            + *forward.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_float * -d)
            as vec_t;
    VectorNormalize(right);
    CrossProduct(right as *const vec_t, forward, up);
}
#[no_mangle]
pub unsafe extern "C" fn VectorRotate(
    mut in_0: *mut vec_t,
    mut matrix: *mut vec3_t,
    mut out: *mut vec_t,
) {
    *out.offset(0 as ::core::ffi::c_int as isize) = *in_0.offset(0 as ::core::ffi::c_int as isize)
        * (*matrix.offset(0 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        + *in_0.offset(1 as ::core::ffi::c_int as isize)
            * (*matrix.offset(0 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
        + *in_0.offset(2 as ::core::ffi::c_int as isize)
            * (*matrix.offset(0 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize];
    *out.offset(1 as ::core::ffi::c_int as isize) = *in_0.offset(0 as ::core::ffi::c_int as isize)
        * (*matrix.offset(1 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        + *in_0.offset(1 as ::core::ffi::c_int as isize)
            * (*matrix.offset(1 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
        + *in_0.offset(2 as ::core::ffi::c_int as isize)
            * (*matrix.offset(1 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize];
    *out.offset(2 as ::core::ffi::c_int as isize) = *in_0.offset(0 as ::core::ffi::c_int as isize)
        * (*matrix.offset(2 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        + *in_0.offset(1 as ::core::ffi::c_int as isize)
            * (*matrix.offset(2 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
        + *in_0.offset(2 as ::core::ffi::c_int as isize)
            * (*matrix.offset(2 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize];
}
#[no_mangle]
pub unsafe extern "C" fn Q_rsqrt(mut number: ::core::ffi::c_float) -> ::core::ffi::c_float {
    let mut i: uint32_t = 0;
    let mut x2: ::core::ffi::c_float = 0.;
    let mut y: ::core::ffi::c_float = 0.;
    let threehalfs: ::core::ffi::c_float = 1.5f32;
    x2 = number * 0.5f32;
    y = number;
    memcpy(
        &raw mut i as *mut ::core::ffi::c_void,
        &raw mut y as *const ::core::ffi::c_void,
        ::core::mem::size_of::<::core::ffi::c_float>() as size_t,
    );
    i = (0x5f3759df as uint32_t).wrapping_sub(i >> 1 as ::core::ffi::c_int);
    memcpy(
        &raw mut y as *mut ::core::ffi::c_void,
        &raw mut i as *const ::core::ffi::c_void,
        ::core::mem::size_of::<::core::ffi::c_float>() as size_t,
    );
    y = y * (threehalfs - x2 * y * y);
    return y;
}
#[no_mangle]
pub unsafe extern "C" fn Q_fabs(mut f: ::core::ffi::c_float) -> ::core::ffi::c_float {
    let mut tmp: ::core::ffi::c_int = *(&raw mut f as *mut ::core::ffi::c_int);
    tmp &= 0x7fffffff as ::core::ffi::c_int;
    return *(&raw mut tmp as *mut ::core::ffi::c_float);
}
#[no_mangle]
pub unsafe extern "C" fn LerpAngle(
    mut from: ::core::ffi::c_float,
    mut to: ::core::ffi::c_float,
    mut frac: ::core::ffi::c_float,
) -> ::core::ffi::c_float {
    let mut a: ::core::ffi::c_float = 0.;
    if to - from > 180 as ::core::ffi::c_int as ::core::ffi::c_float {
        to -= 360 as ::core::ffi::c_int as ::core::ffi::c_float;
    }
    if to - from < -(180 as ::core::ffi::c_int) as ::core::ffi::c_float {
        to += 360 as ::core::ffi::c_int as ::core::ffi::c_float;
    }
    a = from + frac * (to - from);
    return a;
}
#[no_mangle]
pub unsafe extern "C" fn AngleSubtract(
    mut a1: ::core::ffi::c_float,
    mut a2: ::core::ffi::c_float,
) -> ::core::ffi::c_float {
    let mut a: ::core::ffi::c_float = 0.;
    a = a1 - a2;
    while a > 180 as ::core::ffi::c_int as ::core::ffi::c_float {
        a -= 360 as ::core::ffi::c_int as ::core::ffi::c_float;
    }
    while a < -(180 as ::core::ffi::c_int) as ::core::ffi::c_float {
        a += 360 as ::core::ffi::c_int as ::core::ffi::c_float;
    }
    return a;
}
#[no_mangle]
pub unsafe extern "C" fn AnglesSubtract(
    mut v1: *mut vec_t,
    mut v2: *mut vec_t,
    mut v3: *mut vec_t,
) {
    *v3.offset(0 as ::core::ffi::c_int as isize) = AngleSubtract(
        *v1.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_float,
        *v2.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_float,
    ) as vec_t;
    *v3.offset(1 as ::core::ffi::c_int as isize) = AngleSubtract(
        *v1.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_float,
        *v2.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_float,
    ) as vec_t;
    *v3.offset(2 as ::core::ffi::c_int as isize) = AngleSubtract(
        *v1.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_float,
        *v2.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_float,
    ) as vec_t;
}
#[no_mangle]
pub unsafe extern "C" fn AngleMod(mut a: ::core::ffi::c_float) -> ::core::ffi::c_float {
    a = (360.0f64 / 65536 as ::core::ffi::c_int as ::core::ffi::c_double
        * ((a as ::core::ffi::c_double
            * (65536 as ::core::ffi::c_int as ::core::ffi::c_double / 360.0f64))
            as ::core::ffi::c_int
            & 65535 as ::core::ffi::c_int) as ::core::ffi::c_double)
        as ::core::ffi::c_float;
    return a;
}
#[no_mangle]
pub unsafe extern "C" fn AngleNormalize360(
    mut angle: ::core::ffi::c_float,
) -> ::core::ffi::c_float {
    return (360.0f64 / 65536 as ::core::ffi::c_int as ::core::ffi::c_double
        * ((angle as ::core::ffi::c_double
            * (65536 as ::core::ffi::c_int as ::core::ffi::c_double / 360.0f64))
            as ::core::ffi::c_int
            & 65535 as ::core::ffi::c_int) as ::core::ffi::c_double)
        as ::core::ffi::c_float;
}
#[no_mangle]
pub unsafe extern "C" fn AngleNormalize180(
    mut angle: ::core::ffi::c_float,
) -> ::core::ffi::c_float {
    angle = AngleNormalize360(angle);
    if angle as ::core::ffi::c_double > 180.0f64 {
        angle = (angle as ::core::ffi::c_double - 360.0f64) as ::core::ffi::c_float;
    }
    return angle;
}
#[no_mangle]
pub unsafe extern "C" fn AngleDelta(
    mut angle1: ::core::ffi::c_float,
    mut angle2: ::core::ffi::c_float,
) -> ::core::ffi::c_float {
    return AngleNormalize180(angle1 - angle2);
}
#[no_mangle]
pub unsafe extern "C" fn SetPlaneSignbits(mut out: *mut cplane_t) {
    let mut bits: ::core::ffi::c_int = 0;
    let mut j: ::core::ffi::c_int = 0;
    bits = 0 as ::core::ffi::c_int;
    j = 0 as ::core::ffi::c_int;
    while j < 3 as ::core::ffi::c_int {
        if (*out).normal[j as usize] < 0 as ::core::ffi::c_int as ::core::ffi::c_float {
            bits |= (1 as ::core::ffi::c_int) << j;
        }
        j += 1;
    }
    (*out).signbits = bits as byte;
}
#[no_mangle]
pub unsafe extern "C" fn BoxOnPlaneSide(
    mut emins: *mut vec_t,
    mut emaxs: *mut vec_t,
    mut p: *mut cplane_s,
) -> ::core::ffi::c_int {
    let mut dist1: ::core::ffi::c_float = 0.;
    let mut dist2: ::core::ffi::c_float = 0.;
    let mut sides: ::core::ffi::c_int = 0;
    if ((*p).type_0 as ::core::ffi::c_int) < 3 as ::core::ffi::c_int {
        if (*p).dist <= *emins.offset((*p).type_0 as isize) {
            return 1 as ::core::ffi::c_int;
        }
        if (*p).dist >= *emaxs.offset((*p).type_0 as isize) {
            return 2 as ::core::ffi::c_int;
        }
        return 3 as ::core::ffi::c_int;
    }
    match (*p).signbits as ::core::ffi::c_int {
        0 => {
            dist1 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emaxs.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
            dist2 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emins.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emins.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emins.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
        }
        1 => {
            dist1 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emins.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
            dist2 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emaxs.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emins.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emins.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
        }
        2 => {
            dist1 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emaxs.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emins.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
            dist2 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emins.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emins.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
        }
        3 => {
            dist1 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emins.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emins.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
            dist2 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emaxs.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emins.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
        }
        4 => {
            dist1 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emaxs.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emins.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
            dist2 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emins.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emins.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
        }
        5 => {
            dist1 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emins.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emins.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
            dist2 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emaxs.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emins.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
        }
        6 => {
            dist1 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emaxs.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emins.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emins.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
            dist2 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emins.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
        }
        7 => {
            dist1 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emins.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emins.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emins.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
            dist2 = ((*p).normal[0 as ::core::ffi::c_int as usize]
                * *emaxs.offset(0 as ::core::ffi::c_int as isize)
                + (*p).normal[1 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(1 as ::core::ffi::c_int as isize)
                + (*p).normal[2 as ::core::ffi::c_int as usize]
                    * *emaxs.offset(2 as ::core::ffi::c_int as isize))
                as ::core::ffi::c_float;
        }
        _ => {
            dist2 = 0 as ::core::ffi::c_int as ::core::ffi::c_float;
            dist1 = dist2;
        }
    }
    sides = 0 as ::core::ffi::c_int;
    if dist1 >= (*p).dist {
        sides = 1 as ::core::ffi::c_int;
    }
    if dist2 < (*p).dist {
        sides |= 2 as ::core::ffi::c_int;
    }
    return sides;
}
#[no_mangle]
pub unsafe extern "C" fn RadiusFromBounds(
    mut mins: *const vec_t,
    mut maxs: *const vec_t,
) -> ::core::ffi::c_float {
    let mut i: ::core::ffi::c_int = 0;
    let mut corner: vec3_t = [0.; 3];
    let mut a: ::core::ffi::c_float = 0.;
    let mut b: ::core::ffi::c_float = 0.;
    i = 0 as ::core::ffi::c_int;
    while i < 3 as ::core::ffi::c_int {
        a = fabs(*mins.offset(i as isize) as ::core::ffi::c_double) as ::core::ffi::c_float;
        b = fabs(*maxs.offset(i as isize) as ::core::ffi::c_double) as ::core::ffi::c_float;
        corner[i as usize] = (if a > b { a } else { b }) as vec_t;
        i += 1;
    }
    return VectorLength(&raw mut corner as *mut vec_t as *const vec_t) as ::core::ffi::c_float;
}
#[no_mangle]
pub unsafe extern "C" fn ClearBounds(mut mins: *mut vec_t, mut maxs: *mut vec_t) {
    let ref mut fresh2 = *mins.offset(2 as ::core::ffi::c_int as isize);
    *fresh2 = 99999 as ::core::ffi::c_int as vec_t;
    let ref mut fresh3 = *mins.offset(1 as ::core::ffi::c_int as isize);
    *fresh3 = *fresh2;
    *mins.offset(0 as ::core::ffi::c_int as isize) = *fresh3;
    let ref mut fresh4 = *maxs.offset(2 as ::core::ffi::c_int as isize);
    *fresh4 = -(99999 as ::core::ffi::c_int) as vec_t;
    let ref mut fresh5 = *maxs.offset(1 as ::core::ffi::c_int as isize);
    *fresh5 = *fresh4;
    *maxs.offset(0 as ::core::ffi::c_int as isize) = *fresh5;
}
#[no_mangle]
pub unsafe extern "C" fn AddPointToBounds(
    mut v: *const vec_t,
    mut mins: *mut vec_t,
    mut maxs: *mut vec_t,
) {
    if *v.offset(0 as ::core::ffi::c_int as isize) < *mins.offset(0 as ::core::ffi::c_int as isize)
    {
        *mins.offset(0 as ::core::ffi::c_int as isize) =
            *v.offset(0 as ::core::ffi::c_int as isize);
    }
    if *v.offset(0 as ::core::ffi::c_int as isize) > *maxs.offset(0 as ::core::ffi::c_int as isize)
    {
        *maxs.offset(0 as ::core::ffi::c_int as isize) =
            *v.offset(0 as ::core::ffi::c_int as isize);
    }
    if *v.offset(1 as ::core::ffi::c_int as isize) < *mins.offset(1 as ::core::ffi::c_int as isize)
    {
        *mins.offset(1 as ::core::ffi::c_int as isize) =
            *v.offset(1 as ::core::ffi::c_int as isize);
    }
    if *v.offset(1 as ::core::ffi::c_int as isize) > *maxs.offset(1 as ::core::ffi::c_int as isize)
    {
        *maxs.offset(1 as ::core::ffi::c_int as isize) =
            *v.offset(1 as ::core::ffi::c_int as isize);
    }
    if *v.offset(2 as ::core::ffi::c_int as isize) < *mins.offset(2 as ::core::ffi::c_int as isize)
    {
        *mins.offset(2 as ::core::ffi::c_int as isize) =
            *v.offset(2 as ::core::ffi::c_int as isize);
    }
    if *v.offset(2 as ::core::ffi::c_int as isize) > *maxs.offset(2 as ::core::ffi::c_int as isize)
    {
        *maxs.offset(2 as ::core::ffi::c_int as isize) =
            *v.offset(2 as ::core::ffi::c_int as isize);
    }
}
#[no_mangle]
pub unsafe extern "C" fn VectorNormalize(mut v: *mut vec_t) -> vec_t {
    let mut length: ::core::ffi::c_float = 0.;
    let mut ilength: ::core::ffi::c_float = 0.;
    length = (*v.offset(0 as ::core::ffi::c_int as isize)
        * *v.offset(0 as ::core::ffi::c_int as isize)
        + *v.offset(1 as ::core::ffi::c_int as isize) * *v.offset(1 as ::core::ffi::c_int as isize)
        + *v.offset(2 as ::core::ffi::c_int as isize) * *v.offset(2 as ::core::ffi::c_int as isize))
        as ::core::ffi::c_float;
    length = sqrt(length as ::core::ffi::c_double) as ::core::ffi::c_float;
    if length != 0. {
        ilength = 1 as ::core::ffi::c_int as ::core::ffi::c_float / length;
        let ref mut fresh6 = *v.offset(0 as ::core::ffi::c_int as isize);
        *fresh6 *= ilength;
        let ref mut fresh7 = *v.offset(1 as ::core::ffi::c_int as isize);
        *fresh7 *= ilength;
        let ref mut fresh8 = *v.offset(2 as ::core::ffi::c_int as isize);
        *fresh8 *= ilength;
    }
    return length as vec_t;
}
#[no_mangle]
pub unsafe extern "C" fn VectorNormalize2(mut v: *const vec_t, mut out: *mut vec_t) -> vec_t {
    let mut length: ::core::ffi::c_float = 0.;
    let mut ilength: ::core::ffi::c_float = 0.;
    length = (*v.offset(0 as ::core::ffi::c_int as isize)
        * *v.offset(0 as ::core::ffi::c_int as isize)
        + *v.offset(1 as ::core::ffi::c_int as isize) * *v.offset(1 as ::core::ffi::c_int as isize)
        + *v.offset(2 as ::core::ffi::c_int as isize) * *v.offset(2 as ::core::ffi::c_int as isize))
        as ::core::ffi::c_float;
    length = sqrt(length as ::core::ffi::c_double) as ::core::ffi::c_float;
    if length != 0. {
        ilength = 1 as ::core::ffi::c_int as ::core::ffi::c_float / length;
        *out.offset(0 as ::core::ffi::c_int as isize) = (*v.offset(0 as ::core::ffi::c_int as isize)
            as ::core::ffi::c_float
            * ilength) as vec_t;
        *out.offset(1 as ::core::ffi::c_int as isize) = (*v.offset(1 as ::core::ffi::c_int as isize)
            as ::core::ffi::c_float
            * ilength) as vec_t;
        *out.offset(2 as ::core::ffi::c_int as isize) = (*v.offset(2 as ::core::ffi::c_int as isize)
            as ::core::ffi::c_float
            * ilength) as vec_t;
    } else {
        let ref mut fresh9 = *out.offset(2 as ::core::ffi::c_int as isize);
        *fresh9 = 0 as ::core::ffi::c_int as vec_t;
        let ref mut fresh10 = *out.offset(1 as ::core::ffi::c_int as isize);
        *fresh10 = *fresh9;
        *out.offset(0 as ::core::ffi::c_int as isize) = *fresh10;
    }
    return length as vec_t;
}
#[no_mangle]
pub unsafe extern "C" fn _VectorMA(
    mut veca: *const vec_t,
    mut scale: ::core::ffi::c_float,
    mut vecb: *const vec_t,
    mut vecc: *mut vec_t,
) {
    *vecc.offset(0 as ::core::ffi::c_int as isize) = *veca.offset(0 as ::core::ffi::c_int as isize)
        + scale as vec_t * *vecb.offset(0 as ::core::ffi::c_int as isize);
    *vecc.offset(1 as ::core::ffi::c_int as isize) = *veca.offset(1 as ::core::ffi::c_int as isize)
        + scale as vec_t * *vecb.offset(1 as ::core::ffi::c_int as isize);
    *vecc.offset(2 as ::core::ffi::c_int as isize) = *veca.offset(2 as ::core::ffi::c_int as isize)
        + scale as vec_t * *vecb.offset(2 as ::core::ffi::c_int as isize);
}
#[no_mangle]
pub unsafe extern "C" fn _DotProduct(mut v1: *const vec_t, mut v2: *const vec_t) -> vec_t {
    return *v1.offset(0 as ::core::ffi::c_int as isize)
        * *v2.offset(0 as ::core::ffi::c_int as isize)
        + *v1.offset(1 as ::core::ffi::c_int as isize)
            * *v2.offset(1 as ::core::ffi::c_int as isize)
        + *v1.offset(2 as ::core::ffi::c_int as isize)
            * *v2.offset(2 as ::core::ffi::c_int as isize);
}
#[no_mangle]
pub unsafe extern "C" fn _VectorSubtract(
    mut veca: *const vec_t,
    mut vecb: *const vec_t,
    mut out: *mut vec_t,
) {
    *out.offset(0 as ::core::ffi::c_int as isize) = *veca.offset(0 as ::core::ffi::c_int as isize)
        - *vecb.offset(0 as ::core::ffi::c_int as isize);
    *out.offset(1 as ::core::ffi::c_int as isize) = *veca.offset(1 as ::core::ffi::c_int as isize)
        - *vecb.offset(1 as ::core::ffi::c_int as isize);
    *out.offset(2 as ::core::ffi::c_int as isize) = *veca.offset(2 as ::core::ffi::c_int as isize)
        - *vecb.offset(2 as ::core::ffi::c_int as isize);
}
#[no_mangle]
pub unsafe extern "C" fn _VectorAdd(
    mut veca: *const vec_t,
    mut vecb: *const vec_t,
    mut out: *mut vec_t,
) {
    *out.offset(0 as ::core::ffi::c_int as isize) = *veca.offset(0 as ::core::ffi::c_int as isize)
        + *vecb.offset(0 as ::core::ffi::c_int as isize);
    *out.offset(1 as ::core::ffi::c_int as isize) = *veca.offset(1 as ::core::ffi::c_int as isize)
        + *vecb.offset(1 as ::core::ffi::c_int as isize);
    *out.offset(2 as ::core::ffi::c_int as isize) = *veca.offset(2 as ::core::ffi::c_int as isize)
        + *vecb.offset(2 as ::core::ffi::c_int as isize);
}
#[no_mangle]
pub unsafe extern "C" fn _VectorCopy(mut in_0: *const vec_t, mut out: *mut vec_t) {
    *out.offset(0 as ::core::ffi::c_int as isize) = *in_0.offset(0 as ::core::ffi::c_int as isize);
    *out.offset(1 as ::core::ffi::c_int as isize) = *in_0.offset(1 as ::core::ffi::c_int as isize);
    *out.offset(2 as ::core::ffi::c_int as isize) = *in_0.offset(2 as ::core::ffi::c_int as isize);
}
#[no_mangle]
pub unsafe extern "C" fn _VectorScale(
    mut in_0: *const vec_t,
    mut scale: vec_t,
    mut out: *mut vec_t,
) {
    *out.offset(0 as ::core::ffi::c_int as isize) =
        *in_0.offset(0 as ::core::ffi::c_int as isize) * scale;
    *out.offset(1 as ::core::ffi::c_int as isize) =
        *in_0.offset(1 as ::core::ffi::c_int as isize) * scale;
    *out.offset(2 as ::core::ffi::c_int as isize) =
        *in_0.offset(2 as ::core::ffi::c_int as isize) * scale;
}
#[no_mangle]
pub unsafe extern "C" fn Vector4Scale(
    mut in_0: *const vec_t,
    mut scale: vec_t,
    mut out: *mut vec_t,
) {
    *out.offset(0 as ::core::ffi::c_int as isize) =
        *in_0.offset(0 as ::core::ffi::c_int as isize) * scale;
    *out.offset(1 as ::core::ffi::c_int as isize) =
        *in_0.offset(1 as ::core::ffi::c_int as isize) * scale;
    *out.offset(2 as ::core::ffi::c_int as isize) =
        *in_0.offset(2 as ::core::ffi::c_int as isize) * scale;
    *out.offset(3 as ::core::ffi::c_int as isize) =
        *in_0.offset(3 as ::core::ffi::c_int as isize) * scale;
}
#[no_mangle]
pub unsafe extern "C" fn Q_log2(mut val: ::core::ffi::c_int) -> ::core::ffi::c_int {
    let mut answer: ::core::ffi::c_int = 0;
    answer = 0 as ::core::ffi::c_int;
    loop {
        val >>= 1 as ::core::ffi::c_int;
        if !(val != 0 as ::core::ffi::c_int) {
            break;
        }
        answer += 1;
    }
    return answer;
}
#[no_mangle]
pub unsafe extern "C" fn MatrixMultiply(
    mut in1: *mut [::core::ffi::c_float; 3],
    mut in2: *mut [::core::ffi::c_float; 3],
    mut out: *mut [::core::ffi::c_float; 3],
) {
    (*out.offset(0 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize] = (*in1
        .offset(0 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        * (*in2.offset(0 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        + (*in1.offset(0 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
            * (*in2.offset(1 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        + (*in1.offset(0 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
            * (*in2.offset(2 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize];
    (*out.offset(0 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize] = (*in1
        .offset(0 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        * (*in2.offset(0 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
        + (*in1.offset(0 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
            * (*in2.offset(1 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
        + (*in1.offset(0 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
            * (*in2.offset(2 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize];
    (*out.offset(0 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize] = (*in1
        .offset(0 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        * (*in2.offset(0 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
        + (*in1.offset(0 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
            * (*in2.offset(1 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
        + (*in1.offset(0 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
            * (*in2.offset(2 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize];
    (*out.offset(1 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize] = (*in1
        .offset(1 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        * (*in2.offset(0 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        + (*in1.offset(1 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
            * (*in2.offset(1 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        + (*in1.offset(1 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
            * (*in2.offset(2 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize];
    (*out.offset(1 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize] = (*in1
        .offset(1 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        * (*in2.offset(0 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
        + (*in1.offset(1 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
            * (*in2.offset(1 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
        + (*in1.offset(1 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
            * (*in2.offset(2 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize];
    (*out.offset(1 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize] = (*in1
        .offset(1 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        * (*in2.offset(0 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
        + (*in1.offset(1 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
            * (*in2.offset(1 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
        + (*in1.offset(1 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
            * (*in2.offset(2 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize];
    (*out.offset(2 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize] = (*in1
        .offset(2 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        * (*in2.offset(0 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        + (*in1.offset(2 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
            * (*in2.offset(1 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        + (*in1.offset(2 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
            * (*in2.offset(2 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize];
    (*out.offset(2 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize] = (*in1
        .offset(2 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        * (*in2.offset(0 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
        + (*in1.offset(2 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
            * (*in2.offset(1 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
        + (*in1.offset(2 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
            * (*in2.offset(2 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize];
    (*out.offset(2 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize] = (*in1
        .offset(2 as ::core::ffi::c_int as isize))[0 as ::core::ffi::c_int as usize]
        * (*in2.offset(0 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
        + (*in1.offset(2 as ::core::ffi::c_int as isize))[1 as ::core::ffi::c_int as usize]
            * (*in2.offset(1 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
        + (*in1.offset(2 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize]
            * (*in2.offset(2 as ::core::ffi::c_int as isize))[2 as ::core::ffi::c_int as usize];
}
#[no_mangle]
pub unsafe extern "C" fn AngleVectors(
    mut angles: *const vec_t,
    mut forward: *mut vec_t,
    mut right: *mut vec_t,
    mut up: *mut vec_t,
) {
    let mut angle: ::core::ffi::c_float = 0.;
    static mut sr: ::core::ffi::c_float = 0.;
    static mut sp: ::core::ffi::c_float = 0.;
    static mut sy: ::core::ffi::c_float = 0.;
    static mut cr: ::core::ffi::c_float = 0.;
    static mut cp: ::core::ffi::c_float = 0.;
    static mut cy: ::core::ffi::c_float = 0.;
    angle = (*angles.offset(YAW as isize) as ::core::ffi::c_double
        * (M_PI * 2 as ::core::ffi::c_int as ::core::ffi::c_double
            / 360 as ::core::ffi::c_int as ::core::ffi::c_double))
        as ::core::ffi::c_float;
    sy = sin(angle as ::core::ffi::c_double) as ::core::ffi::c_float;
    cy = cos(angle as ::core::ffi::c_double) as ::core::ffi::c_float;
    angle = (*angles.offset(PITCH as isize) as ::core::ffi::c_double
        * (M_PI * 2 as ::core::ffi::c_int as ::core::ffi::c_double
            / 360 as ::core::ffi::c_int as ::core::ffi::c_double))
        as ::core::ffi::c_float;
    sp = sin(angle as ::core::ffi::c_double) as ::core::ffi::c_float;
    cp = cos(angle as ::core::ffi::c_double) as ::core::ffi::c_float;
    angle = (*angles.offset(ROLL as isize) as ::core::ffi::c_double
        * (M_PI * 2 as ::core::ffi::c_int as ::core::ffi::c_double
            / 360 as ::core::ffi::c_int as ::core::ffi::c_double))
        as ::core::ffi::c_float;
    sr = sin(angle as ::core::ffi::c_double) as ::core::ffi::c_float;
    cr = cos(angle as ::core::ffi::c_double) as ::core::ffi::c_float;
    if !forward.is_null() {
        *forward.offset(0 as ::core::ffi::c_int as isize) = (cp * cy) as vec_t;
        *forward.offset(1 as ::core::ffi::c_int as isize) = (cp * sy) as vec_t;
        *forward.offset(2 as ::core::ffi::c_int as isize) = -sp as vec_t;
    }
    if !right.is_null() {
        *right.offset(0 as ::core::ffi::c_int as isize) =
            (-(1 as ::core::ffi::c_int) as ::core::ffi::c_float * sr * sp * cy
                + -(1 as ::core::ffi::c_int) as ::core::ffi::c_float * cr * -sy)
                as vec_t;
        *right.offset(1 as ::core::ffi::c_int as isize) =
            (-(1 as ::core::ffi::c_int) as ::core::ffi::c_float * sr * sp * sy
                + -(1 as ::core::ffi::c_int) as ::core::ffi::c_float * cr * cy)
                as vec_t;
        *right.offset(2 as ::core::ffi::c_int as isize) =
            (-(1 as ::core::ffi::c_int) as ::core::ffi::c_float * sr * cp) as vec_t;
    }
    if !up.is_null() {
        *up.offset(0 as ::core::ffi::c_int as isize) = (cr * sp * cy + -sr * -sy) as vec_t;
        *up.offset(1 as ::core::ffi::c_int as isize) = (cr * sp * sy + -sr * cy) as vec_t;
        *up.offset(2 as ::core::ffi::c_int as isize) = (cr * cp) as vec_t;
    }
}
#[no_mangle]
pub unsafe extern "C" fn PerpendicularVector(mut dst: *mut vec_t, mut src: *const vec_t) {
    let mut pos: ::core::ffi::c_int = 0;
    let mut i: ::core::ffi::c_int = 0;
    let mut minelem: ::core::ffi::c_float = 1.0f32;
    let mut tempvec: vec3_t = [0.; 3];
    pos = 0 as ::core::ffi::c_int;
    i = 0 as ::core::ffi::c_int;
    while i < 3 as ::core::ffi::c_int {
        if fabs(*src.offset(i as isize) as ::core::ffi::c_double) < minelem as ::core::ffi::c_double
        {
            pos = i;
            minelem =
                fabs(*src.offset(i as isize) as ::core::ffi::c_double) as ::core::ffi::c_float;
        }
        i += 1;
    }
    tempvec[2 as ::core::ffi::c_int as usize] = 0.0f32 as vec_t;
    tempvec[1 as ::core::ffi::c_int as usize] = tempvec[2 as ::core::ffi::c_int as usize];
    tempvec[0 as ::core::ffi::c_int as usize] = tempvec[1 as ::core::ffi::c_int as usize];
    tempvec[pos as usize] = 1.0f32 as vec_t;
    ProjectPointOnPlane(dst, &raw mut tempvec as *mut vec_t as *const vec_t, src);
    VectorNormalize(dst);
}
