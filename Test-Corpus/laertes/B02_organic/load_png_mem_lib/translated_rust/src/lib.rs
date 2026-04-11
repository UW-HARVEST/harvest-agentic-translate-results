extern "C" {
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn memset(
        __s: *mut libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn memcmp(
        __s1: *const libc::c_void,
        __s2: *const libc::c_void,
        __n: size_t,
    ) -> libc::c_int;
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn abs(__x: libc::c_int) -> libc::c_int;
    fn __assert_fail(
        __assertion: *const libc::c_char,
        __file: *const libc::c_char,
        __line: libc::c_uint,
        __function: *const libc::c_char,
    ) -> !;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type __int64_t = i64;
pub type __uint64_t = u64;
pub type int64_t = __int64_t;
pub type uint8_t = __uint8_t;
pub type uint16_t = __uint16_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cp_pixel_t {
    pub r: uint8_t,
    pub g: uint8_t,
    pub b: uint8_t,
    pub a: uint8_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cp_image_t {
    pub w: libc::c_int,
    pub h: libc::c_int,
    pub pix: *mut cp_pixel_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cp_state_t {
    pub bits: uint64_t,
    pub count: libc::c_int,
    pub words: *mut uint32_t,
    pub word_count: libc::c_int,
    pub word_index: libc::c_int,
    pub bits_left: libc::c_int,
    pub final_word_available: libc::c_int,
    pub final_word: uint32_t,
    pub out: *mut libc::c_char,
    pub out_end: *mut libc::c_char,
    pub begin: *mut libc::c_char,
    pub lookup: [uint16_t; 512],
    pub lit: [uint32_t; 288],
    pub dst: [uint32_t; 32],
    pub len: [uint32_t; 19],
    pub nlit: uint32_t,
    pub ndst: uint32_t,
    pub nlen: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cp_raw_png_t {
    pub p: *const uint8_t,
    pub end: *const uint8_t,
}
 extern "C" fn cp_make_pixel_a(
    mut r: uint8_t,
    mut g: uint8_t,
    mut b: uint8_t,
    mut a: uint8_t,
) -> cp_pixel_t {
    let mut p: cp_pixel_t = cp_pixel_t {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    p.r = r;
    p.g = g;
    p.b = b;
    p.a = a;
    return p;
}
 extern "C" fn cp_make_pixel(mut r: uint8_t, mut g: uint8_t, mut b: uint8_t) -> cp_pixel_t {
    let mut p: cp_pixel_t = cp_pixel_t {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    p.r = r;
    p.g = g;
    p.b = b;
    p.a = 0xff as uint8_t;
    return p;
}
#[no_mangle]
pub static mut cp_error_reason: *const libc::c_char =
    std::ptr::null::<libc::c_char>();
#[no_mangle]
pub static mut cp_fixed_table: [uint8_t; 320] = [
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
];
#[no_mangle]
pub static mut cp_permutation_order: [uint8_t; 19] = [
    16 as libc::c_int as uint8_t,
    17 as libc::c_int as uint8_t,
    18 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    6 as libc::c_int as uint8_t,
    10 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    11 as libc::c_int as uint8_t,
    4 as libc::c_int as uint8_t,
    12 as libc::c_int as uint8_t,
    3 as libc::c_int as uint8_t,
    13 as libc::c_int as uint8_t,
    2 as libc::c_int as uint8_t,
    14 as libc::c_int as uint8_t,
    1 as libc::c_int as uint8_t,
    15 as libc::c_int as uint8_t,
];
#[no_mangle]
pub static mut cp_len_extra_bits: [uint8_t; 31] = [
    0 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
    1 as libc::c_int as uint8_t,
    1 as libc::c_int as uint8_t,
    1 as libc::c_int as uint8_t,
    1 as libc::c_int as uint8_t,
    2 as libc::c_int as uint8_t,
    2 as libc::c_int as uint8_t,
    2 as libc::c_int as uint8_t,
    2 as libc::c_int as uint8_t,
    3 as libc::c_int as uint8_t,
    3 as libc::c_int as uint8_t,
    3 as libc::c_int as uint8_t,
    3 as libc::c_int as uint8_t,
    4 as libc::c_int as uint8_t,
    4 as libc::c_int as uint8_t,
    4 as libc::c_int as uint8_t,
    4 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
];
#[no_mangle]
pub static mut cp_len_base: [uint32_t; 31] = [
    3 as libc::c_int as uint32_t,
    4 as libc::c_int as uint32_t,
    5 as libc::c_int as uint32_t,
    6 as libc::c_int as uint32_t,
    7 as libc::c_int as uint32_t,
    8 as libc::c_int as uint32_t,
    9 as libc::c_int as uint32_t,
    10 as libc::c_int as uint32_t,
    11 as libc::c_int as uint32_t,
    13 as libc::c_int as uint32_t,
    15 as libc::c_int as uint32_t,
    17 as libc::c_int as uint32_t,
    19 as libc::c_int as uint32_t,
    23 as libc::c_int as uint32_t,
    27 as libc::c_int as uint32_t,
    31 as libc::c_int as uint32_t,
    35 as libc::c_int as uint32_t,
    43 as libc::c_int as uint32_t,
    51 as libc::c_int as uint32_t,
    59 as libc::c_int as uint32_t,
    67 as libc::c_int as uint32_t,
    83 as libc::c_int as uint32_t,
    99 as libc::c_int as uint32_t,
    115 as libc::c_int as uint32_t,
    131 as libc::c_int as uint32_t,
    163 as libc::c_int as uint32_t,
    195 as libc::c_int as uint32_t,
    227 as libc::c_int as uint32_t,
    258 as libc::c_int as uint32_t,
    0 as libc::c_int as uint32_t,
    0 as libc::c_int as uint32_t,
];
#[no_mangle]
pub static mut cp_dist_extra_bits: [uint8_t; 32] = [
    0 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
    1 as libc::c_int as uint8_t,
    1 as libc::c_int as uint8_t,
    2 as libc::c_int as uint8_t,
    2 as libc::c_int as uint8_t,
    3 as libc::c_int as uint8_t,
    3 as libc::c_int as uint8_t,
    4 as libc::c_int as uint8_t,
    4 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    5 as libc::c_int as uint8_t,
    6 as libc::c_int as uint8_t,
    6 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    7 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    8 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    9 as libc::c_int as uint8_t,
    10 as libc::c_int as uint8_t,
    10 as libc::c_int as uint8_t,
    11 as libc::c_int as uint8_t,
    11 as libc::c_int as uint8_t,
    12 as libc::c_int as uint8_t,
    12 as libc::c_int as uint8_t,
    13 as libc::c_int as uint8_t,
    13 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
    0 as libc::c_int as uint8_t,
];
#[no_mangle]
pub static mut cp_dist_base: [uint32_t; 32] = [
    1 as libc::c_int as uint32_t,
    2 as libc::c_int as uint32_t,
    3 as libc::c_int as uint32_t,
    4 as libc::c_int as uint32_t,
    5 as libc::c_int as uint32_t,
    7 as libc::c_int as uint32_t,
    9 as libc::c_int as uint32_t,
    13 as libc::c_int as uint32_t,
    17 as libc::c_int as uint32_t,
    25 as libc::c_int as uint32_t,
    33 as libc::c_int as uint32_t,
    49 as libc::c_int as uint32_t,
    65 as libc::c_int as uint32_t,
    97 as libc::c_int as uint32_t,
    129 as libc::c_int as uint32_t,
    193 as libc::c_int as uint32_t,
    257 as libc::c_int as uint32_t,
    385 as libc::c_int as uint32_t,
    513 as libc::c_int as uint32_t,
    769 as libc::c_int as uint32_t,
    1025 as libc::c_int as uint32_t,
    1537 as libc::c_int as uint32_t,
    2049 as libc::c_int as uint32_t,
    3073 as libc::c_int as uint32_t,
    4097 as libc::c_int as uint32_t,
    6145 as libc::c_int as uint32_t,
    8193 as libc::c_int as uint32_t,
    12289 as libc::c_int as uint32_t,
    16385 as libc::c_int as uint32_t,
    24577 as libc::c_int as uint32_t,
    0 as libc::c_int as uint32_t,
    0 as libc::c_int as uint32_t,
];
unsafe extern "C" fn cp_would_overflow(
    mut s: *mut cp_state_t,
    mut num_bits: libc::c_int,
) -> libc::c_int {
    return ((*s).bits_left + (*s).count - num_bits < 0 as libc::c_int)
        as libc::c_int;
}
unsafe extern "C" fn cp_ptr(mut s: *mut cp_state_t) -> *mut libc::c_char {
    '_c2rust_label: {
        if (*s).bits_left & 7 as libc::c_int == 0 {
        } else {
            __assert_fail(
                b"!(s->bits_left & 7)\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-phmlEF/driver/c_src/src/lib.c\0" as *const u8
                    as *const libc::c_char,
                80 as libc::c_uint,
                b"char *cp_ptr(cp_state_t *)\0" as *const u8 as *const libc::c_char,
            );
        }
    };
    return ((*s).words.offset((*s).word_index as isize) as *mut libc::c_char)
        .offset(-(((*s).count / 8 as libc::c_int) as isize));
}
unsafe extern "C" fn cp_peak_bits(
    mut s: *mut cp_state_t,
    mut num_bits_to_read: libc::c_int,
) -> uint64_t {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let fresh21 = (*s).word_index;
            (*s).word_index = (*s).word_index + 1;
            let mut word: uint32_t = *(*s).words.offset(fresh21 as isize);
            (*s).bits = ((*s).bits as libc::c_ulong
                | ((word as uint64_t) << (*s).count) as libc::c_ulong)
                as uint64_t;
            (*s).count += 32 as libc::c_int;
            '_c2rust_label: {
                if (*s).word_index <= (*s).word_count {
                } else {
                    __assert_fail(
                        b"s->word_index <= s->word_count\0" as *const u8
                            as *const libc::c_char,
                        b"/tmp/harvest-translate-phmlEF/driver/c_src/src/lib.c\0"
                            as *const u8 as *const libc::c_char,
                        89 as libc::c_uint,
                        b"uint64_t cp_peak_bits(cp_state_t *, int)\0" as *const u8
                            as *const libc::c_char,
                    );
                }
            };
        } else if (*s).final_word_available != 0 {
            let mut word_0: uint32_t = (*s).final_word;
            (*s).bits = ((*s).bits as libc::c_ulong
                | ((word_0 as uint64_t) << (*s).count) as libc::c_ulong)
                as uint64_t;
            (*s).count += (*s).bits_left;
            (*s).final_word_available = 0 as libc::c_int;
        }
    }
    return (*s).bits;
}
unsafe extern "C" fn cp_consume_bits(
    mut s: *mut cp_state_t,
    mut num_bits_to_read: libc::c_int,
) -> uint32_t {
    '_c2rust_label: {
        if (*s).count >= num_bits_to_read {
        } else {
            __assert_fail(
                b"s->count >= num_bits_to_read\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-phmlEF/driver/c_src/src/lib.c\0" as *const u8
                    as *const libc::c_char,
                100 as libc::c_uint,
                b"uint32_t cp_consume_bits(cp_state_t *, int)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    let mut bits: uint32_t = ((*s).bits
        & ((1 as libc::c_int as uint64_t) << num_bits_to_read).wrapping_sub(1 as uint64_t))
        as uint32_t;
    (*s).bits >>= num_bits_to_read;
    (*s).count -= num_bits_to_read;
    (*s).bits_left -= num_bits_to_read;
    return bits;
}
unsafe extern "C" fn cp_read_bits(
    mut s: *mut cp_state_t,
    mut num_bits_to_read: libc::c_int,
) -> uint32_t {
    '_c2rust_label: {
        if num_bits_to_read <= 32 as libc::c_int {
        } else {
            __assert_fail(
                b"num_bits_to_read <= 32\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-phmlEF/driver/c_src/src/lib.c\0" as *const u8
                    as *const libc::c_char,
                108 as libc::c_uint,
                b"uint32_t cp_read_bits(cp_state_t *, int)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_0: {
        if num_bits_to_read >= 0 as libc::c_int {
        } else {
            __assert_fail(
                b"num_bits_to_read >= 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-phmlEF/driver/c_src/src/lib.c\0" as *const u8
                    as *const libc::c_char,
                109 as libc::c_uint,
                b"uint32_t cp_read_bits(cp_state_t *, int)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_1: {
        if (*s).bits_left > 0 as libc::c_int {
        } else {
            __assert_fail(
                b"s->bits_left > 0\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-phmlEF/driver/c_src/src/lib.c\0" as *const u8
                    as *const libc::c_char,
                110 as libc::c_uint,
                b"uint32_t cp_read_bits(cp_state_t *, int)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_2: {
        if (*s).count <= 64 as libc::c_int {
        } else {
            __assert_fail(
                b"s->count <= 64\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-phmlEF/driver/c_src/src/lib.c\0" as *const u8
                    as *const libc::c_char,
                111 as libc::c_uint,
                b"uint32_t cp_read_bits(cp_state_t *, int)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    '_c2rust_label_3: {
        if cp_would_overflow(s, num_bits_to_read) == 0 {
        } else {
            __assert_fail(
                b"!cp_would_overflow(s, num_bits_to_read)\0" as *const u8
                    as *const libc::c_char,
                b"/tmp/harvest-translate-phmlEF/driver/c_src/src/lib.c\0" as *const u8
                    as *const libc::c_char,
                112 as libc::c_uint,
                b"uint32_t cp_read_bits(cp_state_t *, int)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    cp_peak_bits(s, num_bits_to_read);
    let mut bits: uint32_t = cp_consume_bits(s, num_bits_to_read);
    return bits;
}
 extern "C" fn cp_rev16(mut a: uint32_t) -> uint32_t {
    a = (a & 0xaaaa as uint32_t) >> 1 as libc::c_int
        | (a & 0x5555 as uint32_t) << 1 as libc::c_int;
    a = (a & 0xcccc as uint32_t) >> 2 as libc::c_int
        | (a & 0x3333 as uint32_t) << 2 as libc::c_int;
    a = (a & 0xf0f0 as uint32_t) >> 4 as libc::c_int
        | (a & 0xf0f as uint32_t) << 4 as libc::c_int;
    a = (a & 0xff00 as uint32_t) >> 8 as libc::c_int
        | (a & 0xff as uint32_t) << 8 as libc::c_int;
    return a;
}
unsafe extern "C" fn cp_build(
    mut s: *mut cp_state_t,
    mut tree: *mut uint32_t,
    mut lens: *mut uint8_t,
    mut sym_count: libc::c_int,
) -> libc::c_int {
    let mut n: libc::c_int = 0;
    let mut codes: [libc::c_int; 16] = [0; 16];
    let mut first: [libc::c_int; 16] = [0; 16];
    let mut counts: [libc::c_int; 16] = [0 as libc::c_int; 16];
    n = 0 as libc::c_int;
    while n < sym_count {
        counts[*lens.offset(n as isize) as usize] += 1;
        n += 1;
    }
    first[0 as libc::c_int as usize] = 0 as libc::c_int;
    codes[0 as libc::c_int as usize] = first[0 as libc::c_int as usize];
    counts[0 as libc::c_int as usize] = codes[0 as libc::c_int as usize];
    n = 1 as libc::c_int;
    while n <= 15 as libc::c_int {
        codes[n as usize] = codes[(n - 1 as libc::c_int) as usize]
            + counts[(n - 1 as libc::c_int) as usize]
            << 1 as libc::c_int;
        first[n as usize] = first[(n - 1 as libc::c_int) as usize]
            + counts[(n - 1 as libc::c_int) as usize];
        n += 1;
    }
    if !s.is_null() {
        memset(
            &raw mut (*s).lookup as *mut uint16_t as *mut libc::c_void,
            0 as libc::c_int,
            std::mem::size_of::<[uint16_t; 512]>() as size_t,
        );
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < sym_count {
        let mut len: libc::c_int = *lens.offset(i as isize) as libc::c_int;
        if len != 0 as libc::c_int {
            '_c2rust_label: {
                if len < 16 as libc::c_int {
                } else {
                    __assert_fail(
                        b"len < 16\0" as *const u8 as *const libc::c_char,
                        b"/tmp/harvest-translate-phmlEF/driver/c_src/src/lib.c\0"
                            as *const u8 as *const libc::c_char,
                        139 as libc::c_uint,
                        b"int cp_build(cp_state_t *, uint32_t *, uint8_t *, int)\0" as *const u8
                            as *const libc::c_char,
                    );
                }
            };
            let fresh23 = codes[len as usize];
            codes[len as usize] = codes[len as usize] + 1;
            let mut code: uint32_t = fresh23 as uint32_t;
            let fresh24 = first[len as usize];
            first[len as usize] = first[len as usize] + 1;
            let mut slot: uint32_t = fresh24 as uint32_t;
            *tree.offset(slot as isize) = code << 32 as libc::c_int - len
                | (i << 4 as libc::c_int) as uint32_t
                | len as uint32_t;
            if !s.is_null() && len <= 9 as libc::c_int {
                let mut j: libc::c_int =
                    (cp_rev16(code) >> 16 as libc::c_int - len) as libc::c_int;
                while j < (1 as libc::c_int) << 9 as libc::c_int {
                    (*s).lookup[j as usize] = (len << 9 as libc::c_int | i) as uint16_t;
                    j += (1 as libc::c_int) << len;
                }
            }
        }
        i += 1;
    }
    let mut max_index: libc::c_int = first[15 as libc::c_int as usize];
    return max_index;
}
unsafe extern "C" fn cp_stored(mut s: *mut cp_state_t) -> libc::c_int {
    let mut p: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    cp_read_bits(s, (*s).count & 7 as libc::c_int);
    let mut LEN: uint16_t = cp_read_bits(s, 16 as libc::c_int) as uint16_t;
    let mut NLEN: uint16_t = cp_read_bits(s, 16 as libc::c_int) as uint16_t;
    if !(LEN as libc::c_int
        == !(NLEN as libc::c_int) as uint16_t as libc::c_int)
    {
        cp_error_reason =
            b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0"
                as *const u8 as *const libc::c_char;
    } else if !((*s).bits_left / 8 as libc::c_int <= LEN as libc::c_int) {
        cp_error_reason = b"Stored block extends beyond end of input stream.\0" as *const u8
            as *const libc::c_char;
    } else {
        p = cp_ptr(s);
        memcpy(
            (*s).out as *mut libc::c_void,
            p as *const libc::c_void,
            LEN as size_t,
        );
        (*s).out = (*s).out.offset(LEN as libc::c_int as isize);
        return 1 as libc::c_int;
    }
    return 0 as libc::c_int;
}
unsafe extern "C" fn cp_fixed(mut s: *mut cp_state_t) -> libc::c_int {
    (*s).nlit = cp_build(
        s,
        &raw mut (*s).lit as *mut uint32_t,
        &raw mut cp_fixed_table as *mut uint8_t,
        288 as libc::c_int,
    ) as uint32_t;
    (*s).ndst = cp_build(
        std::ptr::null_mut::<cp_state_t>(),
        &raw mut (*s).dst as *mut uint32_t,
        (&raw mut cp_fixed_table as *mut uint8_t).offset(288 as libc::c_int as isize),
        32 as libc::c_int,
    ) as uint32_t;
    return 1 as libc::c_int;
}
unsafe extern "C" fn cp_decode(
    mut s: *mut cp_state_t,
    mut tree: *mut uint32_t,
    mut hi: libc::c_int,
) -> libc::c_int {
    let mut bits: uint64_t = cp_peak_bits(s, 16 as libc::c_int);
    let mut search: uint32_t =
        cp_rev16(bits as uint32_t) << 16 as libc::c_int | 0xffff as uint32_t;
    let mut lo: libc::c_int = 0 as libc::c_int;
    while lo < hi {
        let mut guess: libc::c_int = lo + hi >> 1 as libc::c_int;
        if search < *tree.offset(guess as isize) {
            hi = guess;
        } else {
            lo = guess + 1 as libc::c_int;
        }
    }
    let mut key: uint32_t = *tree.offset((lo - 1 as libc::c_int) as isize);
    let mut len: uint32_t = (32 as uint32_t).wrapping_sub(key & 0xf as uint32_t);
    '_c2rust_label: {
        if search >> len == key >> len {
        } else {
            __assert_fail(
                b"(search >> len) == (key >> len)\0" as *const u8 as *const libc::c_char,
                b"/tmp/harvest-translate-phmlEF/driver/c_src/src/lib.c\0" as *const u8
                    as *const libc::c_char,
                202 as libc::c_uint,
                b"int cp_decode(cp_state_t *, uint32_t *, int)\0" as *const u8
                    as *const libc::c_char,
            );
        }
    };
    let mut code: libc::c_int =
        cp_consume_bits(s, (key & 0xf as uint32_t) as libc::c_int) as libc::c_int;
    return (key >> 4 as libc::c_int & 0xfff as uint32_t) as libc::c_int;
}
unsafe extern "C" fn cp_dynamic(mut s: *mut cp_state_t) -> libc::c_int {
    let mut lenlens: [uint8_t; 19] = [
        0 as libc::c_int as uint8_t,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    let mut nlit: libc::c_int = (257 as uint32_t)
        .wrapping_add(cp_read_bits(s, 5 as libc::c_int))
        as libc::c_int;
    let mut ndst: libc::c_int = (1 as uint32_t)
        .wrapping_add(cp_read_bits(s, 5 as libc::c_int))
        as libc::c_int;
    let mut nlen: libc::c_int = (4 as uint32_t)
        .wrapping_add(cp_read_bits(s, 4 as libc::c_int))
        as libc::c_int;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < nlen {
        lenlens[cp_permutation_order[i as usize] as usize] =
            cp_read_bits(s, 3 as libc::c_int) as uint8_t;
        i += 1;
    }
    (*s).nlen = cp_build(
        std::ptr::null_mut::<cp_state_t>(),
        &raw mut (*s).len as *mut uint32_t,
        &raw mut lenlens as *mut uint8_t,
        19 as libc::c_int,
    ) as uint32_t;
    let mut lens: [uint8_t; 320] = [0; 320];
    let mut n: libc::c_int = 0 as libc::c_int;
    while n < nlit + ndst {
        let mut sym: libc::c_int = cp_decode(
            s,
            &raw mut (*s).len as *mut uint32_t,
            (*s).nlen as libc::c_int,
        );
        match sym {
            16 => {
                let mut i_0: libc::c_int = (3 as uint32_t)
                    .wrapping_add(cp_read_bits(s, 2 as libc::c_int))
                    as libc::c_int;
                while i_0 != 0 {
                    lens[n as usize] = lens[(n - 1 as libc::c_int) as usize];
                    i_0 -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i_1: libc::c_int = (3 as uint32_t)
                    .wrapping_add(cp_read_bits(s, 3 as libc::c_int))
                    as libc::c_int;
                while i_1 != 0 {
                    lens[n as usize] = 0 as uint8_t;
                    i_1 -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i_2: libc::c_int = (11 as uint32_t)
                    .wrapping_add(cp_read_bits(s, 7 as libc::c_int))
                    as libc::c_int;
                while i_2 != 0 {
                    lens[n as usize] = 0 as uint8_t;
                    i_2 -= 1;
                    n += 1;
                }
            }
            _ => {
                let fresh22 = n;
                n = n + 1;
                lens[fresh22 as usize] = sym as uint8_t;
            }
        }
    }
    (*s).nlit = cp_build(
        s,
        &raw mut (*s).lit as *mut uint32_t,
        &raw mut lens as *mut uint8_t,
        nlit,
    ) as uint32_t;
    (*s).ndst = cp_build(
        std::ptr::null_mut::<cp_state_t>(),
        &raw mut (*s).dst as *mut uint32_t,
        (&raw mut lens as *mut uint8_t).offset(nlit as isize),
        ndst,
    ) as uint32_t;
    return 1 as libc::c_int;
}
unsafe extern "C" fn cp_block(mut s: *mut cp_state_t) -> libc::c_int {
    let mut current_block: u64;
    loop {
        let mut symbol: libc::c_int = cp_decode(
            s,
            &raw mut (*s).lit as *mut uint32_t,
            (*s).nlit as libc::c_int,
        );
        if symbol < 256 as libc::c_int {
            if !((*s).out.offset(1 as libc::c_int as isize) <= (*s).out_end) {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a symbol.\0"
                    as *const u8 as *const libc::c_char;
                current_block = 297282898163270830;
                break;
            } else {
                *(*s).out = symbol as libc::c_char;
                (*s).out = (*s).out.offset(1 as libc::c_int as isize);
            }
        } else {
            if !(symbol > 256 as libc::c_int) {
                current_block = 17788412896529399552;
                break;
            }
            symbol -= 257 as libc::c_int;
            let mut length: libc::c_int =
                cp_read_bits(s, cp_len_extra_bits[symbol as usize] as libc::c_int)
                    .wrapping_add(cp_len_base[symbol as usize])
                    as libc::c_int;
            let mut distance_symbol: libc::c_int = cp_decode(
                s,
                &raw mut (*s).dst as *mut uint32_t,
                (*s).ndst as libc::c_int,
            );
            let mut backwards_distance: libc::c_int = cp_read_bits(
                s,
                cp_dist_extra_bits[distance_symbol as usize] as libc::c_int,
            )
            .wrapping_add(cp_dist_base[distance_symbol as usize])
                as libc::c_int;
            if !((*s).out.offset(-(backwards_distance as isize)) >= (*s).begin) {
                cp_error_reason =
                    b"Attempted to write before out buffer (invalid backwards distance).\0"
                        as *const u8 as *const libc::c_char;
                current_block = 297282898163270830;
                break;
            } else if !((*s).out.offset(length as isize) <= (*s).out_end) {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a string.\0"
                    as *const u8 as *const libc::c_char;
                current_block = 297282898163270830;
                break;
            } else {
                let mut src: *mut libc::c_char =
                    (*s).out.offset(-(backwards_distance as isize));
                let mut dst: *mut libc::c_char = (*s).out;
                (*s).out = (*s).out.offset(length as isize);
                match backwards_distance {
                    1 => {
                        memset(
                            dst as *mut libc::c_void,
                            *src as libc::c_int,
                            length as size_t,
                        );
                    }
                    _ => loop {
                        let fresh18 = length;
                        length = length - 1;
                        if !(fresh18 != 0) {
                            break;
                        }
                        let fresh19 = src;
                        src = src.offset(1);
                        let fresh20 = dst;
                        dst = dst.offset(1);
                        *fresh20 = *fresh19;
                    },
                }
            }
        }
    }
    match current_block {
        17788412896529399552 => return 1 as libc::c_int,
        _ => return 0 as libc::c_int,
    };
}
#[no_mangle]
pub unsafe extern "C" fn cp_inflate(
    mut in_0: *mut libc::c_void,
    mut in_bytes: libc::c_int,
    mut out: *mut libc::c_void,
    mut out_bytes: libc::c_int,
) -> libc::c_int {
    let mut current_block: u64;
    let mut s: *mut cp_state_t =
        calloc(1 as size_t, std::mem::size_of::<cp_state_t>() as size_t) as *mut cp_state_t;
    (*s).bits = 0 as uint64_t;
    (*s).count = 0 as libc::c_int;
    (*s).word_index = 0 as libc::c_int;
    (*s).bits_left = in_bytes * 8 as libc::c_int;
    let mut first_bytes: libc::c_int =
        ((in_0 as size_t).wrapping_add(3 as size_t) & !(3 as libc::c_int) as size_t)
            .wrapping_sub(in_0 as size_t) as libc::c_int;
    (*s).words = (in_0 as *mut libc::c_char).offset(first_bytes as isize) as *mut uint32_t;
    (*s).word_count = (in_bytes - first_bytes) / 4 as libc::c_int;
    let mut last_bytes: libc::c_int = in_bytes - first_bytes & 3 as libc::c_int;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < first_bytes {
        (*s).bits = ((*s).bits as libc::c_ulong
            | ((*(in_0 as *mut uint8_t).offset(i as isize) as uint64_t)
                << i * 8 as libc::c_int) as libc::c_ulong)
            as uint64_t;
        i += 1;
    }
    (*s).final_word_available = if last_bytes != 0 {
        1 as libc::c_int
    } else {
        0 as libc::c_int
    };
    (*s).final_word = 0 as uint32_t;
    let mut i_0: libc::c_int = 0 as libc::c_int;
    while i_0 < last_bytes {
        (*s).final_word = ((*s).final_word as libc::c_uint
            | ((*(in_0 as *mut uint8_t).offset((in_bytes - last_bytes + i_0) as isize)
                as libc::c_int)
                << i_0 * 8 as libc::c_int) as libc::c_uint)
            as uint32_t;
        i_0 += 1;
    }
    (*s).count = first_bytes * 8 as libc::c_int;
    (*s).out = out as *mut libc::c_char;
    (*s).out_end = (*s).out.offset(out_bytes as isize);
    (*s).begin = out as *mut libc::c_char;
    let mut count: libc::c_int = 0 as libc::c_int;
    let mut bfinal: libc::c_int = 0;
    loop {
        bfinal = cp_read_bits(s, 1 as libc::c_int) as libc::c_int;
        let mut btype: libc::c_int =
            cp_read_bits(s, 2 as libc::c_int) as libc::c_int;
        match btype {
            0 => {
                if cp_stored(s) == 0 {
                    current_block = 3831819345633471047;
                    break;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    current_block = 3831819345633471047;
                    break;
                }
            }
            2 => {
                cp_dynamic(s);
                if cp_block(s) == 0 {
                    current_block = 3831819345633471047;
                    break;
                }
            }
            3 => {
                if 0 as libc::c_int == 0 {
                    cp_error_reason = b"Detected unknown block type within input stream.\0"
                        as *const u8
                        as *const libc::c_char;
                    current_block = 3831819345633471047;
                    break;
                }
            }
            _ => {}
        }
        count += 1;
        if !(bfinal == 0) {
            current_block = 17184638872671510253;
            break;
        }
    }
    match current_block {
        3831819345633471047 => {
            free(s as *mut libc::c_void);
            return 0 as libc::c_int;
        }
        _ => {
            free(s as *mut libc::c_void);
            return 1 as libc::c_int;
        }
    };
}
unsafe extern "C" fn cp_paeth(mut a: uint8_t, mut b: uint8_t, mut c: uint8_t) -> uint8_t {
    let mut p: libc::c_int =
        a as libc::c_int + b as libc::c_int - c as libc::c_int;
    let mut pa: libc::c_int = abs(p - a as libc::c_int);
    let mut pb: libc::c_int = abs(p - b as libc::c_int);
    let mut pc: libc::c_int = abs(p - c as libc::c_int);
    return (if pa <= pb && pa <= pc {
        a as libc::c_int
    } else if pb <= pc {
        b as libc::c_int
    } else {
        c as libc::c_int
    }) as uint8_t;
}
unsafe extern "C" fn cp_make32(mut s: *const uint8_t) -> uint32_t {
    return ((*s.offset(0 as libc::c_int as isize) as libc::c_int)
        << 24 as libc::c_int
        | (*s.offset(1 as libc::c_int as isize) as libc::c_int)
            << 16 as libc::c_int
        | (*s.offset(2 as libc::c_int as isize) as libc::c_int)
            << 8 as libc::c_int
        | *s.offset(3 as libc::c_int as isize) as libc::c_int)
        as uint32_t;
}
unsafe extern "C" fn cp_chunk(
    mut png: *mut cp_raw_png_t,
    mut chunk: *const libc::c_char,
    mut minlen: uint32_t,
) -> *const uint8_t {
    let mut len: uint32_t = cp_make32((*png).p);
    let mut start: *const uint8_t = (*png).p;
    if memcmp(
        start.offset(4 as libc::c_int as isize) as *const libc::c_void,
        chunk as *const libc::c_void,
        4 as size_t,
    ) == 0
        && len >= minlen
    {
        let mut offset: libc::c_int = len.wrapping_add(12 as uint32_t) as libc::c_int;
        if (*png).p.offset(offset as isize) <= (*png).end {
            (*png).p = (*png).p.offset(offset as isize);
            return start.offset(8 as libc::c_int as isize);
        }
    }
    return std::ptr::null::<uint8_t>();
}
unsafe extern "C" fn cp_find(
    mut png: *mut cp_raw_png_t,
    mut chunk: *const libc::c_char,
    mut minlen: uint32_t,
) -> *const uint8_t {
    let mut start: *const uint8_t = std::ptr::null::<uint8_t>();
    while (*png).p < (*png).end {
        let mut len: uint32_t = cp_make32((*png).p);
        start = (*png).p;
        (*png).p = (*png).p.offset(len.wrapping_add(12 as uint32_t) as isize);
        if memcmp(
            start.offset(4 as libc::c_int as isize) as *const libc::c_void,
            chunk as *const libc::c_void,
            4 as size_t,
        ) == 0
            && len >= minlen
            && (*png).p <= (*png).end
        {
            return start.offset(8 as libc::c_int as isize);
        }
    }
    return std::ptr::null::<uint8_t>();
}
unsafe extern "C" fn cp_unfilter(
    mut w: libc::c_int,
    mut h: libc::c_int,
    mut bpp: libc::c_int,
    mut raw: *mut uint8_t,
) -> libc::c_int {
    let mut len: libc::c_int = w * bpp;
    let mut prev: *mut uint8_t = std::ptr::null_mut::<uint8_t>();
    let mut x: libc::c_int = 0;
    if h > 0 as libc::c_int {
        let fresh5 = raw;
        raw = raw.offset(1);
        match *fresh5 as libc::c_int {
            1 => {
                x = bpp;
                while x < len {
                    let ref mut fresh6 = *raw.offset(x as isize);
                    *fresh6 = (*fresh6 as libc::c_int
                        + *raw.offset((x - bpp) as isize) as libc::c_int)
                        as uint8_t;
                    x += 1;
                }
            }
            0 | 2 => {}
            3 => {
                x = bpp;
                while x < len {
                    let ref mut fresh7 = *raw.offset(x as isize);
                    *fresh7 = (*fresh7 as libc::c_int
                        + *raw.offset((x - bpp) as isize) as libc::c_int
                            / 2 as libc::c_int) as uint8_t;
                    x += 1;
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    let ref mut fresh8 = *raw.offset(x as isize);
                    *fresh8 = (*fresh8 as libc::c_int
                        + cp_paeth(*raw.offset((x - bpp) as isize), 0 as uint8_t, 0 as uint8_t)
                            as libc::c_int) as uint8_t;
                    x += 1;
                }
            }
            _ => return 0 as libc::c_int,
        }
    }
    prev = raw;
    raw = raw.offset(len as isize);
    let mut y: libc::c_int = 1 as libc::c_int;
    while y < h {
        let fresh9 = raw;
        raw = raw.offset(1);
        match *fresh9 as libc::c_int {
            0 => {}
            1 => {
                x = 0 as libc::c_int;
                while x < bpp {
                    let ref mut fresh10 = *raw.offset(x as isize);
                    *fresh10 =
                        (*fresh10 as libc::c_int + 0 as libc::c_int) as uint8_t;
                    x += 1;
                }
                while x < len {
                    let ref mut fresh11 = *raw.offset(x as isize);
                    *fresh11 = (*fresh11 as libc::c_int
                        + *raw.offset((x - bpp) as isize) as libc::c_int)
                        as uint8_t;
                    x += 1;
                }
            }
            2 => {
                x = 0 as libc::c_int;
                while x < bpp {
                    let ref mut fresh12 = *raw.offset(x as isize);
                    *fresh12 = (*fresh12 as libc::c_int
                        + *prev.offset(x as isize) as libc::c_int)
                        as uint8_t;
                    x += 1;
                }
                while x < len {
                    let ref mut fresh13 = *raw.offset(x as isize);
                    *fresh13 = (*fresh13 as libc::c_int
                        + *prev.offset(x as isize) as libc::c_int)
                        as uint8_t;
                    x += 1;
                }
            }
            3 => {
                x = 0 as libc::c_int;
                while x < bpp {
                    let ref mut fresh14 = *raw.offset(x as isize);
                    *fresh14 = (*fresh14 as libc::c_int
                        + *prev.offset(x as isize) as libc::c_int / 2 as libc::c_int)
                        as uint8_t;
                    x += 1;
                }
                while x < len {
                    let ref mut fresh15 = *raw.offset(x as isize);
                    *fresh15 = (*fresh15 as libc::c_int
                        + (*raw.offset((x - bpp) as isize) as libc::c_int
                            + *prev.offset(x as isize) as libc::c_int)
                            / 2 as libc::c_int) as uint8_t;
                    x += 1;
                }
            }
            4 => {
                x = 0 as libc::c_int;
                while x < bpp {
                    let ref mut fresh16 = *raw.offset(x as isize);
                    *fresh16 = (*fresh16 as libc::c_int
                        + *prev.offset(x as isize) as libc::c_int)
                        as uint8_t;
                    x += 1;
                }
                while x < len {
                    let ref mut fresh17 = *raw.offset(x as isize);
                    *fresh17 = (*fresh17 as libc::c_int
                        + cp_paeth(
                            *raw.offset((x - bpp) as isize),
                            *prev.offset(x as isize),
                            *prev.offset((x - bpp) as isize),
                        ) as libc::c_int) as uint8_t;
                    x += 1;
                }
            }
            _ => return 0 as libc::c_int,
        }
        y += 1;
        prev = raw;
        raw = raw.offset(len as isize);
    }
    return 1 as libc::c_int;
}
unsafe extern "C" fn cp_convert(
    mut bpp: libc::c_int,
    mut w: libc::c_int,
    mut h: libc::c_int,
    mut src: *mut uint8_t,
    mut dst: *mut cp_pixel_t,
) {
    let mut y: libc::c_int = 0 as libc::c_int;
    while y < h {
        src = src.offset(1);
        let mut x: libc::c_int = 0 as libc::c_int;
        while x < w {
            match bpp {
                1 => {
                    let fresh0 = dst;
                    dst = dst.offset(1);
                    *fresh0 = cp_make_pixel(
                        *src.offset(0 as libc::c_int as isize),
                        *src.offset(0 as libc::c_int as isize),
                        *src.offset(0 as libc::c_int as isize),
                    );
                }
                2 => {
                    let fresh1 = dst;
                    dst = dst.offset(1);
                    *fresh1 = cp_make_pixel_a(
                        *src.offset(0 as libc::c_int as isize),
                        *src.offset(0 as libc::c_int as isize),
                        *src.offset(0 as libc::c_int as isize),
                        *src.offset(1 as libc::c_int as isize),
                    );
                }
                3 => {
                    let fresh2 = dst;
                    dst = dst.offset(1);
                    *fresh2 = cp_make_pixel(
                        *src.offset(0 as libc::c_int as isize),
                        *src.offset(1 as libc::c_int as isize),
                        *src.offset(2 as libc::c_int as isize),
                    );
                }
                4 => {
                    let fresh3 = dst;
                    dst = dst.offset(1);
                    *fresh3 = cp_make_pixel_a(
                        *src.offset(0 as libc::c_int as isize),
                        *src.offset(1 as libc::c_int as isize),
                        *src.offset(2 as libc::c_int as isize),
                        *src.offset(3 as libc::c_int as isize),
                    );
                }
                _ => {}
            }
            x += 1;
            src = src.offset(bpp as isize);
        }
        y += 1;
    }
}
unsafe extern "C" fn cp_get_alpha_for_indexed_image(
    mut index: libc::c_int,
    mut trns: *const uint8_t,
    mut trns_len: uint32_t,
) -> uint8_t {
    if trns.is_null() {
        return 255 as uint8_t;
    } else if index as uint32_t >= trns_len {
        return 255 as uint8_t;
    } else {
        return *trns.offset(index as isize);
    };
}
unsafe extern "C" fn cp_depalette(
    mut w: libc::c_int,
    mut h: libc::c_int,
    mut src: *mut uint8_t,
    mut dst: *mut cp_pixel_t,
    mut plte: *const uint8_t,
    mut trns: *const uint8_t,
    mut trns_len: uint32_t,
) {
    let mut y: libc::c_int = 0 as libc::c_int;
    while y < h {
        src = src.offset(1);
        let mut x: libc::c_int = 0 as libc::c_int;
        while x < w {
            let mut c: libc::c_int = *src as libc::c_int;
            let mut r: uint8_t = *plte.offset((c * 3 as libc::c_int) as isize);
            let mut g: uint8_t =
                *plte.offset((c * 3 as libc::c_int + 1 as libc::c_int) as isize);
            let mut b: uint8_t =
                *plte.offset((c * 3 as libc::c_int + 2 as libc::c_int) as isize);
            let mut a: uint8_t = cp_get_alpha_for_indexed_image(c, trns, trns_len);
            let fresh4 = dst;
            dst = dst.offset(1);
            *fresh4 = cp_make_pixel_a(r, g, b, a);
            x += 1;
            src = src.offset(1);
        }
        y += 1;
    }
}
unsafe extern "C" fn cp_get_chunk_byte_length(mut chunk: *const uint8_t) -> uint32_t {
    return cp_make32(chunk.offset(-(8 as libc::c_int as isize)));
}
unsafe extern "C" fn cp_out_size(
    mut img: *mut cp_image_t,
    mut bpp: libc::c_int,
) -> libc::c_int {
    return ((*img).w + 1 as libc::c_int) * (*img).h * bpp;
}
#[no_mangle]
pub unsafe extern "C" fn load_png_mem(
    mut png_data: *const uint8_t,
    mut png_length: libc::c_int,
) -> cp_image_t {
    let mut current_block: u64;
    let mut sig: *const libc::c_char =
        b"\x89PNG\r\n\x1A\n\0" as *const u8 as *const libc::c_char;
    let mut ihdr: *const uint8_t = std::ptr::null::<uint8_t>();
    let mut first: *const uint8_t = std::ptr::null::<uint8_t>();
    let mut plte: *const uint8_t = std::ptr::null::<uint8_t>();
    let mut trns: *const uint8_t = std::ptr::null::<uint8_t>();
    let mut bit_depth: libc::c_int = 0;
    let mut color_type: libc::c_int = 0;
    let mut bpp: libc::c_int = 0;
    let mut w: libc::c_int = 0;
    let mut h: libc::c_int = 0;
    let mut pix_bytes: libc::c_int = 0;
    let mut compression: libc::c_int = 0;
    let mut filter: libc::c_int = 0;
    let mut interlace: libc::c_int = 0;
    let mut datalen: libc::c_int = 0;
    let mut offset: libc::c_int = 0;
    let mut out: *mut uint8_t = std::ptr::null_mut::<uint8_t>();
    let mut img: cp_image_t = cp_image_t {
        w: 0 as libc::c_int,
        h: 0,
        pix: std::ptr::null_mut::<cp_pixel_t>(),
    };
    let mut data: *mut uint8_t = std::ptr::null_mut::<uint8_t>();
    let mut png: cp_raw_png_t = cp_raw_png_t {
        p: std::ptr::null::<uint8_t>(),
        end: std::ptr::null::<uint8_t>(),
    };
    png.p = png_data as *mut uint8_t;
    png.end = (png_data as *mut uint8_t).offset(png_length as isize);
    if memcmp(
        png.p as *const libc::c_void,
        sig as *const libc::c_void,
        8 as size_t,
    ) != 0
    {
        cp_error_reason = b"incorrect file signature (is this a png file?)\0" as *const u8
            as *const libc::c_char;
    } else {
        png.p = png.p.offset(8 as libc::c_int as isize);
        ihdr = cp_chunk(
            &raw mut png,
            b"IHDR\0" as *const u8 as *const libc::c_char,
            13 as uint32_t,
        );
        if ihdr.is_null() {
            cp_error_reason =
                b"unable to find IHDR chunk\0" as *const u8 as *const libc::c_char;
        } else {
            bit_depth = *ihdr.offset(8 as libc::c_int as isize) as libc::c_int;
            color_type = *ihdr.offset(9 as libc::c_int as isize) as libc::c_int;
            if !(bit_depth == 8 as libc::c_int) {
                cp_error_reason = b"only bit-depth of 8 is supported\0" as *const u8
                    as *const libc::c_char;
            } else {
                match color_type {
                    0 => {
                        bpp = 1 as libc::c_int;
                        current_block = 6450636197030046351;
                    }
                    2 => {
                        bpp = 3 as libc::c_int;
                        current_block = 6450636197030046351;
                    }
                    3 => {
                        bpp = 1 as libc::c_int;
                        current_block = 6450636197030046351;
                    }
                    4 => {
                        bpp = 2 as libc::c_int;
                        current_block = 6450636197030046351;
                    }
                    6 => {
                        bpp = 4 as libc::c_int;
                        current_block = 6450636197030046351;
                    }
                    _ => {
                        if 0 as libc::c_int == 0 {
                            cp_error_reason =
                                b"unknown color type\0" as *const u8 as *const libc::c_char;
                            current_block = 15461442727611312104;
                        } else {
                            current_block = 6450636197030046351;
                        }
                    }
                }
                match current_block {
                    15461442727611312104 => {}
                    _ => {
                        w = cp_make32(ihdr).wrapping_add(1 as uint32_t) as libc::c_int;
                        h = cp_make32(ihdr.offset(4 as libc::c_int as isize))
                            as libc::c_int;
                        if !(w >= 1 as libc::c_int) {
                            cp_error_reason =
                                b"invalid IHDR chunk found, image width was less than 1\0"
                                    as *const u8
                                    as *const libc::c_char;
                        } else if !(h >= 1 as libc::c_int) {
                            cp_error_reason =
                                b"invalid IHDR chunk found, image height was less than 1\0"
                                    as *const u8
                                    as *const libc::c_char;
                        } else if !(((w as int64_t * h as int64_t) as usize)
                            .wrapping_mul(std::mem::size_of::<cp_pixel_t>() as usize)
                            < INT_MAX as usize)
                        {
                            cp_error_reason =
                                b"image too large\0" as *const u8 as *const libc::c_char;
                        } else {
                            pix_bytes =
                                ((w * h) as usize)
                                    .wrapping_mul(std::mem::size_of::<cp_pixel_t>() as usize)
                                    as libc::c_int;
                            img.w = w - 1 as libc::c_int;
                            img.h = h;
                            img.pix = malloc(pix_bytes as size_t) as *mut cp_pixel_t;
                            if img.pix.is_null() {
                                cp_error_reason = b"unable to allocate raw image space\0"
                                    as *const u8
                                    as *const libc::c_char;
                            } else {
                                compression = *ihdr.offset(10 as libc::c_int as isize)
                                    as libc::c_int;
                                filter = *ihdr.offset(11 as libc::c_int as isize)
                                    as libc::c_int;
                                interlace = *ihdr.offset(12 as libc::c_int as isize)
                                    as libc::c_int;
                                if compression != 0 {
                                    cp_error_reason =
                                        b"only standard compression DEFLATE is supported\0"
                                            as *const u8
                                            as *const libc::c_char;
                                } else if filter != 0 {
                                    cp_error_reason =
                                        b"only standard adaptive filtering is supported\0"
                                            as *const u8
                                            as *const libc::c_char;
                                } else if interlace != 0 {
                                    cp_error_reason = b"interlacing is not supported\0" as *const u8
                                        as *const libc::c_char;
                                } else {
                                    first = png.p;
                                    plte = cp_find(
                                        &raw mut png,
                                        b"PLTE\0" as *const u8 as *const libc::c_char,
                                        0 as uint32_t,
                                    );
                                    if plte.is_null() {
                                        png.p = first;
                                    } else {
                                        first = png.p;
                                    }
                                    trns = cp_find(
                                        &raw mut png,
                                        b"tRNS\0" as *const u8 as *const libc::c_char,
                                        0 as uint32_t,
                                    );
                                    if trns.is_null() {
                                        png.p = first;
                                    } else {
                                        first = png.p;
                                    }
                                    datalen = 0 as libc::c_int;
                                    let mut idat: *const uint8_t = cp_find(
                                        &raw mut png,
                                        b"IDAT\0" as *const u8 as *const libc::c_char,
                                        0 as uint32_t,
                                    );
                                    while !idat.is_null() {
                                        let mut len: uint32_t = cp_get_chunk_byte_length(idat);
                                        datalen = (datalen as libc::c_uint)
                                            .wrapping_add(len as libc::c_uint)
                                            as libc::c_int
                                            as libc::c_int;
                                        idat = cp_chunk(
                                            &raw mut png,
                                            b"IDAT\0" as *const u8 as *const libc::c_char,
                                            0 as uint32_t,
                                        );
                                    }
                                    png.p = first;
                                    data = malloc(datalen as size_t) as *mut uint8_t;
                                    offset = 0 as libc::c_int;
                                    let mut idat_0: *const uint8_t = cp_find(
                                        &raw mut png,
                                        b"IDAT\0" as *const u8 as *const libc::c_char,
                                        0 as uint32_t,
                                    );
                                    while !idat_0.is_null() {
                                        let mut len_0: uint32_t = cp_get_chunk_byte_length(idat_0);
                                        memcpy(
                                            data.offset(offset as isize)
                                                as *mut libc::c_void,
                                            idat_0 as *const libc::c_void,
                                            len_0 as size_t,
                                        );
                                        offset = (offset as libc::c_uint)
                                            .wrapping_add(len_0 as libc::c_uint)
                                            as libc::c_int
                                            as libc::c_int;
                                        idat_0 = cp_chunk(
                                            &raw mut png,
                                            b"IDAT\0" as *const u8 as *const libc::c_char,
                                            0 as uint32_t,
                                        );
                                    }
                                    if !(!data.is_null() && datalen >= 6 as libc::c_int) {
                                        cp_error_reason =
                                            b"corrupt zlib structure in DEFLATE stream\0"
                                                as *const u8
                                                as *const libc::c_char;
                                    } else if !(*data.offset(0 as libc::c_int as isize)
                                        as libc::c_int
                                        & 0xf as libc::c_int
                                        == 0x8 as libc::c_int)
                                    {
                                        cp_error_reason = b"only zlib compression method (RFC 1950) is supported\0"
                                            as *const u8 as *const libc::c_char;
                                    } else if !(*data.offset(0 as libc::c_int as isize)
                                        as libc::c_int
                                        & 0xf0 as libc::c_int
                                        <= 0x70 as libc::c_int)
                                    {
                                        cp_error_reason = b"innapropriate window size detected\0"
                                            as *const u8
                                            as *const libc::c_char;
                                    } else if *data.offset(1 as libc::c_int as isize)
                                        as libc::c_int
                                        & 0x20 as libc::c_int
                                        != 0
                                    {
                                        cp_error_reason =
                                            b"preset dictionary is present and not supported\0"
                                                as *const u8
                                                as *const libc::c_char;
                                    } else if !(cp_out_size(&raw mut img, 4 as libc::c_int)
                                        >= 1 as libc::c_int)
                                    {
                                        cp_error_reason = b"invalid image size found\0" as *const u8
                                            as *const libc::c_char;
                                    } else if !(cp_out_size(&raw mut img, bpp)
                                        >= 1 as libc::c_int)
                                    {
                                        cp_error_reason = b"invalid image size found\0" as *const u8
                                            as *const libc::c_char;
                                    } else {
                                        out = (img.pix as *mut uint8_t)
                                            .offset(cp_out_size(
                                                &raw mut img,
                                                4 as libc::c_int,
                                            )
                                                as isize)
                                            .offset(-(cp_out_size(&raw mut img, bpp) as isize));
                                        if cp_inflate(
                                            data.offset(2 as libc::c_int as isize)
                                                as *mut libc::c_void,
                                            datalen - 6 as libc::c_int,
                                            out as *mut libc::c_void,
                                            pix_bytes,
                                        ) == 0
                                        {
                                            cp_error_reason = b"DEFLATE algorithm failed\0"
                                                as *const u8
                                                as *const libc::c_char;
                                        } else if cp_unfilter(img.w, img.h, bpp, out) == 0 {
                                            cp_error_reason = b"invalid filter byte found\0"
                                                as *const u8
                                                as *const libc::c_char;
                                        } else {
                                            if color_type == 3 as libc::c_int {
                                                if plte.is_null() {
                                                    cp_error_reason = b"color type of indexed requires a PLTE chunk\0"
                                                        as *const u8 as *const libc::c_char;
                                                    current_block = 15461442727611312104;
                                                } else {
                                                    let mut trns_len: uint32_t = if !trns.is_null()
                                                    {
                                                        cp_get_chunk_byte_length(trns)
                                                    } else {
                                                        0 as uint32_t
                                                    };
                                                    cp_depalette(
                                                        img.w, img.h, out, img.pix, plte, trns,
                                                        trns_len,
                                                    );
                                                    current_block = 10494165753505607199;
                                                }
                                            } else {
                                                cp_convert(bpp, img.w, img.h, out, img.pix);
                                                current_block = 10494165753505607199;
                                            }
                                            match current_block {
                                                15461442727611312104 => {}
                                                _ => {
                                                    free(data as *mut libc::c_void);
                                                    return img;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    free(data as *mut libc::c_void);
    free(img.pix as *mut libc::c_void);
    img.pix = std::ptr::null_mut::<cp_pixel_t>();
    return img;
}
pub const __INT_MAX__: libc::c_int = 2147483647 as libc::c_int;
pub const INT_MAX: libc::c_int = __INT_MAX__;
