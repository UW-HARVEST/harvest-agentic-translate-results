





extern "C" {
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
    fn memcmp(
        __s1: *const ::core::ffi::c_void,
        __s2: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn abs(__x: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn __assert_fail(
        __assertion: *const ::core::ffi::c_char,
        __file: *const ::core::ffi::c_char,
        __line: ::core::ffi::c_uint,
        __function: *const ::core::ffi::c_char,
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
    pub w: ::core::ffi::c_int,
    pub h: ::core::ffi::c_int,
    pub pix: *mut cp_pixel_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cp_state_t {
    pub bits: uint64_t,
    pub count: ::core::ffi::c_int,
    pub words: *mut uint32_t,
    pub word_count: ::core::ffi::c_int,
    pub word_index: ::core::ffi::c_int,
    pub bits_left: ::core::ffi::c_int,
    pub final_word_available: ::core::ffi::c_int,
    pub final_word: uint32_t,
    pub out: *mut ::core::ffi::c_char,
    pub out_end: *mut ::core::ffi::c_char,
    pub begin: *mut ::core::ffi::c_char,
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
unsafe extern "C" fn cp_make_pixel_a(
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
unsafe extern "C" fn cp_make_pixel(mut r: uint8_t, mut g: uint8_t, mut b: uint8_t) -> cp_pixel_t {
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
pub static mut cp_error_reason: *const ::core::ffi::c_char =
    ::core::ptr::null::<::core::ffi::c_char>();
#[no_mangle]
pub static mut cp_fixed_table: [uint8_t; 320] = [
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
];
#[no_mangle]
pub static mut cp_permutation_order: [uint8_t; 19] = [
    16 as ::core::ffi::c_int as uint8_t,
    17 as ::core::ffi::c_int as uint8_t,
    18 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    6 as ::core::ffi::c_int as uint8_t,
    10 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    11 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    12 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    13 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    14 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    15 as ::core::ffi::c_int as uint8_t,
];
#[no_mangle]
pub static mut cp_len_extra_bits: [uint8_t; 31] = [
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
];
#[no_mangle]
pub static mut cp_len_base: [uint32_t; 31] = [
    3 as ::core::ffi::c_int as uint32_t,
    4 as ::core::ffi::c_int as uint32_t,
    5 as ::core::ffi::c_int as uint32_t,
    6 as ::core::ffi::c_int as uint32_t,
    7 as ::core::ffi::c_int as uint32_t,
    8 as ::core::ffi::c_int as uint32_t,
    9 as ::core::ffi::c_int as uint32_t,
    10 as ::core::ffi::c_int as uint32_t,
    11 as ::core::ffi::c_int as uint32_t,
    13 as ::core::ffi::c_int as uint32_t,
    15 as ::core::ffi::c_int as uint32_t,
    17 as ::core::ffi::c_int as uint32_t,
    19 as ::core::ffi::c_int as uint32_t,
    23 as ::core::ffi::c_int as uint32_t,
    27 as ::core::ffi::c_int as uint32_t,
    31 as ::core::ffi::c_int as uint32_t,
    35 as ::core::ffi::c_int as uint32_t,
    43 as ::core::ffi::c_int as uint32_t,
    51 as ::core::ffi::c_int as uint32_t,
    59 as ::core::ffi::c_int as uint32_t,
    67 as ::core::ffi::c_int as uint32_t,
    83 as ::core::ffi::c_int as uint32_t,
    99 as ::core::ffi::c_int as uint32_t,
    115 as ::core::ffi::c_int as uint32_t,
    131 as ::core::ffi::c_int as uint32_t,
    163 as ::core::ffi::c_int as uint32_t,
    195 as ::core::ffi::c_int as uint32_t,
    227 as ::core::ffi::c_int as uint32_t,
    258 as ::core::ffi::c_int as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
];
#[no_mangle]
pub static mut cp_dist_extra_bits: [uint8_t; 32] = [
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    6 as ::core::ffi::c_int as uint8_t,
    6 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    10 as ::core::ffi::c_int as uint8_t,
    10 as ::core::ffi::c_int as uint8_t,
    11 as ::core::ffi::c_int as uint8_t,
    11 as ::core::ffi::c_int as uint8_t,
    12 as ::core::ffi::c_int as uint8_t,
    12 as ::core::ffi::c_int as uint8_t,
    13 as ::core::ffi::c_int as uint8_t,
    13 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
];
#[no_mangle]
pub static mut cp_dist_base: [uint32_t; 32] = [
    1 as ::core::ffi::c_int as uint32_t,
    2 as ::core::ffi::c_int as uint32_t,
    3 as ::core::ffi::c_int as uint32_t,
    4 as ::core::ffi::c_int as uint32_t,
    5 as ::core::ffi::c_int as uint32_t,
    7 as ::core::ffi::c_int as uint32_t,
    9 as ::core::ffi::c_int as uint32_t,
    13 as ::core::ffi::c_int as uint32_t,
    17 as ::core::ffi::c_int as uint32_t,
    25 as ::core::ffi::c_int as uint32_t,
    33 as ::core::ffi::c_int as uint32_t,
    49 as ::core::ffi::c_int as uint32_t,
    65 as ::core::ffi::c_int as uint32_t,
    97 as ::core::ffi::c_int as uint32_t,
    129 as ::core::ffi::c_int as uint32_t,
    193 as ::core::ffi::c_int as uint32_t,
    257 as ::core::ffi::c_int as uint32_t,
    385 as ::core::ffi::c_int as uint32_t,
    513 as ::core::ffi::c_int as uint32_t,
    769 as ::core::ffi::c_int as uint32_t,
    1025 as ::core::ffi::c_int as uint32_t,
    1537 as ::core::ffi::c_int as uint32_t,
    2049 as ::core::ffi::c_int as uint32_t,
    3073 as ::core::ffi::c_int as uint32_t,
    4097 as ::core::ffi::c_int as uint32_t,
    6145 as ::core::ffi::c_int as uint32_t,
    8193 as ::core::ffi::c_int as uint32_t,
    12289 as ::core::ffi::c_int as uint32_t,
    16385 as ::core::ffi::c_int as uint32_t,
    24577 as ::core::ffi::c_int as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
    0 as ::core::ffi::c_int as uint32_t,
];
unsafe extern "C" fn cp_would_overflow(
    mut s: *mut cp_state_t,
    mut num_bits: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return ((*s).bits_left + (*s).count - num_bits < 0 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn cp_ptr(mut s: *mut cp_state_t) -> *mut ::core::ffi::c_char {
    '_c2rust_label: {
        if (*s).bits_left & 7 as ::core::ffi::c_int == 0 {
        } else {
            __assert_fail(
                b"!(s->bits_left & 7)\0" as *const u8 as *const ::core::ffi::c_char,
                b"/tmp/harvest-translate-phmlEF/driver/c_src/src/lib.c\0" as *const u8
                    as *const ::core::ffi::c_char,
                80 as ::core::ffi::c_uint,
                b"char *cp_ptr(cp_state_t *)\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
    };
    return ((*s).words.offset((*s).word_index as isize) as *mut ::core::ffi::c_char)
        .offset(-(((*s).count / 8 as ::core::ffi::c_int) as isize));
}
fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: i32) -> u64 {
    let s = unsafe { &mut *s };

    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let fresh21 = s.word_index;
            s.word_index += 1;
            let word = unsafe { *s.words.add(fresh21 as usize) };
            s.bits |= (word as u64) << s.count;
            s.count += 32;
            assert!(s.word_index <= s.word_count);
        } else if s.final_word_available != 0 {
            let word = s.final_word;
            s.bits |= (word as u64) << s.count;
            s.count += s.bits_left;
            s.final_word_available = 0;
        }
    }

    s.bits
}

fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: i32) -> u32 {
    let s = unsafe {
        assert!(!s.is_null());
        &mut *s
    };

    assert!(s.count >= num_bits_to_read);

    let mask = if num_bits_to_read == 0 {
        0
    } else {
        (1u64 << num_bits_to_read) - 1
    };

    let bits = (s.bits & mask) as u32;
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: i32) -> u32 {
    assert!(num_bits_to_read <= 32, "num_bits_to_read <= 32");
    assert!(num_bits_to_read >= 0, "num_bits_to_read >= 0");

    unsafe {
        assert!((*s).bits_left > 0, "s->bits_left > 0");
        assert!((*s).count <= 64, "s->count <= 64");
        assert!(
            cp_would_overflow(s, num_bits_to_read) == 0,
            "!cp_would_overflow(s, num_bits_to_read)"
        );

        cp_peak_bits(s, num_bits_to_read);
        cp_consume_bits(s, num_bits_to_read)
    }
}

fn cp_rev16(mut a: u32) -> u32 {
    a &= 0xffff;
    a = ((a & 0xaaaa) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xcccc) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xf0f0) >> 4) | ((a & 0x0f0f) << 4);
    ((a & 0xff00) >> 8) | ((a & 0x00ff) << 8)
}

fn cp_build(
    mut s: Option<&mut cp_state_t>,
    tree: &mut [uint32_t],
    lens: &[uint8_t],
    sym_count: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let sym_count = sym_count as usize;
    let mut codes: [::core::ffi::c_int; 16] = [0; 16];
    let mut first: [::core::ffi::c_int; 16] = [0; 16];
    let mut counts: [::core::ffi::c_int; 16] = [0; 16];

    for &len in lens.iter().take(sym_count) {
        counts[len as usize] += 1;
    }

    first[0] = 0;
    codes[0] = first[0];
    counts[0] = codes[0];

    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }

    if let Some(state) = s.as_deref_mut() {
        state.lookup.fill(0);
    }

    for (i, &len_u8) in lens.iter().take(sym_count).enumerate() {
        let len = len_u8 as ::core::ffi::c_int;
        if len != 0 {
            assert!(len < 16, "len < 16");

            let code = codes[len as usize] as uint32_t;
            codes[len as usize] += 1;

            let slot = first[len as usize] as usize;
            first[len as usize] += 1;

            tree[slot] = (code << (32 - len))
                | ((i as uint32_t) << 4)
                | (len as uint32_t);

            if let Some(state) = s.as_deref_mut() {
                if len <= 9 {
                    let mut j =
                        (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1usize << 9) {
                        state.lookup[j] =
                            (((len << 9) | (i as ::core::ffi::c_int)) as uint16_t);
                        j += 1usize << len;
                    }
                }
            }
        }
    }

    first[15]
}

unsafe extern "C" fn cp_stored(mut s: *mut cp_state_t) -> ::core::ffi::c_int {
    let mut p: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    cp_read_bits(s, unsafe { (*s).count & 7 });
    let mut LEN: uint16_t = cp_read_bits(s, 16) as uint16_t;
    let mut NLEN: uint16_t = cp_read_bits(s, 16) as uint16_t;
    if !(LEN as ::core::ffi::c_int
        == !(NLEN as ::core::ffi::c_int) as uint16_t as ::core::ffi::c_int)
    {
        cp_error_reason =
            b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0"
                as *const u8 as *const ::core::ffi::c_char;
    } else if !((*s).bits_left / 8 as ::core::ffi::c_int <= LEN as ::core::ffi::c_int) {
        cp_error_reason = b"Stored block extends beyond end of input stream.\0" as *const u8
            as *const ::core::ffi::c_char;
    } else {
        p = cp_ptr(s);
        memcpy(
            (*s).out as *mut ::core::ffi::c_void,
            p as *const ::core::ffi::c_void,
            LEN as size_t,
        );
        (*s).out = (*s).out.offset(LEN as ::core::ffi::c_int as isize);
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn cp_fixed(mut s: *mut cp_state_t) -> ::core::ffi::c_int {
    (*s).nlit = cp_build(
    Some(&mut *s),
    &mut (*s).lit[..],
    &cp_fixed_table[..288],
    288 as ::core::ffi::c_int,
) as uint32_t;
    (*s).ndst = cp_build(
    None,
    &mut (*s).dst[..],
    &cp_fixed_table[288..(288 + 32)],
    32 as ::core::ffi::c_int,
) as uint32_t;
    return 1 as ::core::ffi::c_int;
}
fn cp_decode(s: *mut cp_state_t, tree: *mut u32, mut hi: i32) -> i32 {
    let bits: u64 = cp_peak_bits(s, 16);
    let search: u32 = (cp_rev16(bits as u32) << 16) | 0xffff;
    let mut lo: i32 = 0;

    while lo < hi {
        let guess: i32 = (lo + hi) >> 1;
        let value = unsafe { *tree.add(guess as usize) };
        if search < value {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }

    let key: u32 = unsafe { *tree.add((lo - 1) as usize) };
    let len: u32 = 32u32.wrapping_sub(key & 0xf);

    assert_eq!(search >> len, key >> len);

    let _code: i32 = cp_consume_bits(s, (key & 0xf) as i32) as i32;
    ((key >> 4) & 0xfff) as i32
}

unsafe extern "C" fn cp_dynamic(mut s: *mut cp_state_t) -> ::core::ffi::c_int {
    let mut lenlens: [uint8_t; 19] = [
        0 as ::core::ffi::c_int as uint8_t,
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
    let mut nlit: ::core::ffi::c_int =
    (257u32).wrapping_add(cp_read_bits(s, 5)) as ::core::ffi::c_int;
    let mut ndst: ::core::ffi::c_int =
    (1u32).wrapping_add(cp_read_bits(s, 5)) as ::core::ffi::c_int;
    let mut nlen: ::core::ffi::c_int =
    (4u32).wrapping_add(cp_read_bits(s, 4)) as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < nlen {
        lenlens[cp_permutation_order[i as usize] as usize] =
            cp_read_bits(s, 3 as ::core::ffi::c_int) as uint8_t;
        i += 1;
    }
    (*s).nlen = cp_build(
    None,
    &mut (*s).len[..],
    &lenlens[..19],
    19 as ::core::ffi::c_int,
) as uint32_t;
    let mut lens: [uint8_t; 320] = [0; 320];
    let mut n: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while n < nlit + ndst {
        let mut sym: ::core::ffi::c_int = cp_decode(
            s,
            &raw mut (*s).len as *mut uint32_t,
            (*s).nlen as ::core::ffi::c_int,
        );
        match sym {
            16 => {
                let mut i_0: ::core::ffi::c_int = (3 as uint32_t)
                    .wrapping_add(cp_read_bits(s, 2 as ::core::ffi::c_int))
                    as ::core::ffi::c_int;
                while i_0 != 0 {
                    lens[n as usize] = lens[(n - 1 as ::core::ffi::c_int) as usize];
                    i_0 -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i_1: ::core::ffi::c_int = (3 as uint32_t)
                    .wrapping_add(cp_read_bits(s, 3 as ::core::ffi::c_int))
                    as ::core::ffi::c_int;
                while i_1 != 0 {
                    lens[n as usize] = 0 as uint8_t;
                    i_1 -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i_2: ::core::ffi::c_int = (11 as uint32_t)
                    .wrapping_add(cp_read_bits(s, 7 as ::core::ffi::c_int))
                    as ::core::ffi::c_int;
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
    Some(&mut *s),
    &mut (*s).lit[..],
    &lens[..nlit as usize],
    nlit,
) as uint32_t;
    (*s).ndst = cp_build(
    None,
    &mut (*s).dst[..],
    &lens[nlit as usize..],
    ndst,
) as uint32_t;
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn cp_block(mut s: *mut cp_state_t) -> ::core::ffi::c_int {
    let mut current_block: u64;
    loop {
        let mut symbol: ::core::ffi::c_int = cp_decode(
    s,
    &raw mut (*s).lit as *mut u32,
    (*s).nlit as ::core::ffi::c_int,
);
        if symbol < 256 as ::core::ffi::c_int {
            if !((*s).out.offset(1 as ::core::ffi::c_int as isize) <= (*s).out_end) {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a symbol.\0"
                    as *const u8 as *const ::core::ffi::c_char;
                current_block = 297282898163270830;
                break;
            } else {
                *(*s).out = symbol as ::core::ffi::c_char;
                (*s).out = (*s).out.offset(1 as ::core::ffi::c_int as isize);
            }
        } else {
            if !(symbol > 256 as ::core::ffi::c_int) {
                current_block = 17788412896529399552;
                break;
            }
            symbol -= 257 as ::core::ffi::c_int;
            let mut length: ::core::ffi::c_int =
    cp_read_bits(s, cp_len_extra_bits[symbol as usize] as ::core::ffi::c_int)
        .wrapping_add(cp_len_base[symbol as usize]) as ::core::ffi::c_int;
            let mut distance_symbol: ::core::ffi::c_int = cp_decode(
    s,
    &raw mut (*s).dst as *mut u32,
    (*s).ndst as ::core::ffi::c_int,
);
            let mut backwards_distance: ::core::ffi::c_int =
    cp_read_bits(
        s,
        cp_dist_extra_bits[distance_symbol as usize] as ::core::ffi::c_int,
    )
    .wrapping_add(cp_dist_base[distance_symbol as usize]) as ::core::ffi::c_int;
            if !((*s).out.offset(-(backwards_distance as isize)) >= (*s).begin) {
                cp_error_reason =
                    b"Attempted to write before out buffer (invalid backwards distance).\0"
                        as *const u8 as *const ::core::ffi::c_char;
                current_block = 297282898163270830;
                break;
            } else if !((*s).out.offset(length as isize) <= (*s).out_end) {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a string.\0"
                    as *const u8 as *const ::core::ffi::c_char;
                current_block = 297282898163270830;
                break;
            } else {
                let mut src: *mut ::core::ffi::c_char =
                    (*s).out.offset(-(backwards_distance as isize));
                let mut dst: *mut ::core::ffi::c_char = (*s).out;
                (*s).out = (*s).out.offset(length as isize);
                match backwards_distance {
                    1 => {
                        memset(
                            dst as *mut ::core::ffi::c_void,
                            *src as ::core::ffi::c_int,
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
        17788412896529399552 => return 1 as ::core::ffi::c_int,
        _ => return 0 as ::core::ffi::c_int,
    };
}
#[no_mangle]
pub unsafe extern "C" fn cp_inflate(
    mut in_0: *mut ::core::ffi::c_void,
    mut in_bytes: ::core::ffi::c_int,
    mut out: *mut ::core::ffi::c_void,
    mut out_bytes: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut current_block: u64;
    let mut s: *mut cp_state_t =
        calloc(1 as size_t, ::core::mem::size_of::<cp_state_t>() as size_t) as *mut cp_state_t;
    (*s).bits = 0 as uint64_t;
    (*s).count = 0 as ::core::ffi::c_int;
    (*s).word_index = 0 as ::core::ffi::c_int;
    (*s).bits_left = in_bytes * 8 as ::core::ffi::c_int;
    let mut first_bytes: ::core::ffi::c_int =
        ((in_0 as size_t).wrapping_add(3 as size_t) & !(3 as ::core::ffi::c_int) as size_t)
            .wrapping_sub(in_0 as size_t) as ::core::ffi::c_int;
    (*s).words = (in_0 as *mut ::core::ffi::c_char).offset(first_bytes as isize) as *mut uint32_t;
    (*s).word_count = (in_bytes - first_bytes) / 4 as ::core::ffi::c_int;
    let mut last_bytes: ::core::ffi::c_int = in_bytes - first_bytes & 3 as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < first_bytes {
        (*s).bits = ((*s).bits as ::core::ffi::c_ulong
            | ((*(in_0 as *mut uint8_t).offset(i as isize) as uint64_t)
                << i * 8 as ::core::ffi::c_int) as ::core::ffi::c_ulong)
            as uint64_t;
        i += 1;
    }
    (*s).final_word_available = if last_bytes != 0 {
        1 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    };
    (*s).final_word = 0 as uint32_t;
    let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_0 < last_bytes {
        (*s).final_word = ((*s).final_word as ::core::ffi::c_uint
            | ((*(in_0 as *mut uint8_t).offset((in_bytes - last_bytes + i_0) as isize)
                as ::core::ffi::c_int)
                << i_0 * 8 as ::core::ffi::c_int) as ::core::ffi::c_uint)
            as uint32_t;
        i_0 += 1;
    }
    (*s).count = first_bytes * 8 as ::core::ffi::c_int;
    (*s).out = out as *mut ::core::ffi::c_char;
    (*s).out_end = (*s).out.offset(out_bytes as isize);
    (*s).begin = out as *mut ::core::ffi::c_char;
    let mut count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut bfinal: ::core::ffi::c_int = 0;
    loop {
        bfinal = cp_read_bits(s, 1) as ::core::ffi::c_int;
        let mut btype: ::core::ffi::c_int = cp_read_bits(s, 2) as ::core::ffi::c_int;
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
                if 0 as ::core::ffi::c_int == 0 {
                    cp_error_reason = b"Detected unknown block type within input stream.\0"
                        as *const u8
                        as *const ::core::ffi::c_char;
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
            free(s as *mut ::core::ffi::c_void);
            return 0 as ::core::ffi::c_int;
        }
        _ => {
            free(s as *mut ::core::ffi::c_void);
            return 1 as ::core::ffi::c_int;
        }
    };
}
unsafe extern "C" fn cp_paeth(mut a: uint8_t, mut b: uint8_t, mut c: uint8_t) -> uint8_t {
    let mut p: ::core::ffi::c_int =
        a as ::core::ffi::c_int + b as ::core::ffi::c_int - c as ::core::ffi::c_int;
    let mut pa: ::core::ffi::c_int = abs(p - a as ::core::ffi::c_int);
    let mut pb: ::core::ffi::c_int = abs(p - b as ::core::ffi::c_int);
    let mut pc: ::core::ffi::c_int = abs(p - c as ::core::ffi::c_int);
    return (if pa <= pb && pa <= pc {
        a as ::core::ffi::c_int
    } else if pb <= pc {
        b as ::core::ffi::c_int
    } else {
        c as ::core::ffi::c_int
    }) as uint8_t;
}
unsafe extern "C" fn cp_make32(mut s: *const uint8_t) -> uint32_t {
    return ((*s.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
        << 24 as ::core::ffi::c_int
        | (*s.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            << 16 as ::core::ffi::c_int
        | (*s.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
        | *s.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
        as uint32_t;
}
unsafe extern "C" fn cp_chunk(
    mut png: *mut cp_raw_png_t,
    mut chunk: *const ::core::ffi::c_char,
    mut minlen: uint32_t,
) -> *const uint8_t {
    let mut len: uint32_t = cp_make32((*png).p);
    let mut start: *const uint8_t = (*png).p;
    if memcmp(
        start.offset(4 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
        chunk as *const ::core::ffi::c_void,
        4 as size_t,
    ) == 0
        && len >= minlen
    {
        let mut offset: ::core::ffi::c_int = len.wrapping_add(12 as uint32_t) as ::core::ffi::c_int;
        if (*png).p.offset(offset as isize) <= (*png).end {
            (*png).p = (*png).p.offset(offset as isize);
            return start.offset(8 as ::core::ffi::c_int as isize);
        }
    }
    return ::core::ptr::null::<uint8_t>();
}
unsafe extern "C" fn cp_find(
    mut png: *mut cp_raw_png_t,
    mut chunk: *const ::core::ffi::c_char,
    mut minlen: uint32_t,
) -> *const uint8_t {
    let mut start: *const uint8_t = ::core::ptr::null::<uint8_t>();
    while (*png).p < (*png).end {
        let mut len: uint32_t = cp_make32((*png).p);
        start = (*png).p;
        (*png).p = (*png).p.offset(len.wrapping_add(12 as uint32_t) as isize);
        if memcmp(
            start.offset(4 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
            chunk as *const ::core::ffi::c_void,
            4 as size_t,
        ) == 0
            && len >= minlen
            && (*png).p <= (*png).end
        {
            return start.offset(8 as ::core::ffi::c_int as isize);
        }
    }
    return ::core::ptr::null::<uint8_t>();
}
unsafe extern "C" fn cp_unfilter(
    mut w: ::core::ffi::c_int,
    mut h: ::core::ffi::c_int,
    mut bpp: ::core::ffi::c_int,
    mut raw: *mut uint8_t,
) -> ::core::ffi::c_int {
    let mut len: ::core::ffi::c_int = w * bpp;
    let mut prev: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    let mut x: ::core::ffi::c_int = 0;
    if h > 0 as ::core::ffi::c_int {
        let fresh5 = raw;
        raw = raw.offset(1);
        match *fresh5 as ::core::ffi::c_int {
            1 => {
                x = bpp;
                while x < len {
                    let ref mut fresh6 = *raw.offset(x as isize);
                    *fresh6 = (*fresh6 as ::core::ffi::c_int
                        + *raw.offset((x - bpp) as isize) as ::core::ffi::c_int)
                        as uint8_t;
                    x += 1;
                }
            }
            0 | 2 => {}
            3 => {
                x = bpp;
                while x < len {
                    let ref mut fresh7 = *raw.offset(x as isize);
                    *fresh7 = (*fresh7 as ::core::ffi::c_int
                        + *raw.offset((x - bpp) as isize) as ::core::ffi::c_int
                            / 2 as ::core::ffi::c_int) as uint8_t;
                    x += 1;
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    let ref mut fresh8 = *raw.offset(x as isize);
                    *fresh8 = (*fresh8 as ::core::ffi::c_int
                        + cp_paeth(*raw.offset((x - bpp) as isize), 0 as uint8_t, 0 as uint8_t)
                            as ::core::ffi::c_int) as uint8_t;
                    x += 1;
                }
            }
            _ => return 0 as ::core::ffi::c_int,
        }
    }
    prev = raw;
    raw = raw.offset(len as isize);
    let mut y: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    while y < h {
        let fresh9 = raw;
        raw = raw.offset(1);
        match *fresh9 as ::core::ffi::c_int {
            0 => {}
            1 => {
                x = 0 as ::core::ffi::c_int;
                while x < bpp {
                    let ref mut fresh10 = *raw.offset(x as isize);
                    *fresh10 =
                        (*fresh10 as ::core::ffi::c_int + 0 as ::core::ffi::c_int) as uint8_t;
                    x += 1;
                }
                while x < len {
                    let ref mut fresh11 = *raw.offset(x as isize);
                    *fresh11 = (*fresh11 as ::core::ffi::c_int
                        + *raw.offset((x - bpp) as isize) as ::core::ffi::c_int)
                        as uint8_t;
                    x += 1;
                }
            }
            2 => {
                x = 0 as ::core::ffi::c_int;
                while x < bpp {
                    let ref mut fresh12 = *raw.offset(x as isize);
                    *fresh12 = (*fresh12 as ::core::ffi::c_int
                        + *prev.offset(x as isize) as ::core::ffi::c_int)
                        as uint8_t;
                    x += 1;
                }
                while x < len {
                    let ref mut fresh13 = *raw.offset(x as isize);
                    *fresh13 = (*fresh13 as ::core::ffi::c_int
                        + *prev.offset(x as isize) as ::core::ffi::c_int)
                        as uint8_t;
                    x += 1;
                }
            }
            3 => {
                x = 0 as ::core::ffi::c_int;
                while x < bpp {
                    let ref mut fresh14 = *raw.offset(x as isize);
                    *fresh14 = (*fresh14 as ::core::ffi::c_int
                        + *prev.offset(x as isize) as ::core::ffi::c_int / 2 as ::core::ffi::c_int)
                        as uint8_t;
                    x += 1;
                }
                while x < len {
                    let ref mut fresh15 = *raw.offset(x as isize);
                    *fresh15 = (*fresh15 as ::core::ffi::c_int
                        + (*raw.offset((x - bpp) as isize) as ::core::ffi::c_int
                            + *prev.offset(x as isize) as ::core::ffi::c_int)
                            / 2 as ::core::ffi::c_int) as uint8_t;
                    x += 1;
                }
            }
            4 => {
                x = 0 as ::core::ffi::c_int;
                while x < bpp {
                    let ref mut fresh16 = *raw.offset(x as isize);
                    *fresh16 = (*fresh16 as ::core::ffi::c_int
                        + *prev.offset(x as isize) as ::core::ffi::c_int)
                        as uint8_t;
                    x += 1;
                }
                while x < len {
                    let ref mut fresh17 = *raw.offset(x as isize);
                    *fresh17 = (*fresh17 as ::core::ffi::c_int
                        + cp_paeth(
                            *raw.offset((x - bpp) as isize),
                            *prev.offset(x as isize),
                            *prev.offset((x - bpp) as isize),
                        ) as ::core::ffi::c_int) as uint8_t;
                    x += 1;
                }
            }
            _ => return 0 as ::core::ffi::c_int,
        }
        y += 1;
        prev = raw;
        raw = raw.offset(len as isize);
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn cp_convert(
    mut bpp: ::core::ffi::c_int,
    mut w: ::core::ffi::c_int,
    mut h: ::core::ffi::c_int,
    mut src: *mut uint8_t,
    mut dst: *mut cp_pixel_t,
) {
    let mut y: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while y < h {
        src = src.offset(1);
        let mut x: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while x < w {
            match bpp {
                1 => {
                    let fresh0 = dst;
                    dst = dst.offset(1);
                    *fresh0 = cp_make_pixel(
                        *src.offset(0 as ::core::ffi::c_int as isize),
                        *src.offset(0 as ::core::ffi::c_int as isize),
                        *src.offset(0 as ::core::ffi::c_int as isize),
                    );
                }
                2 => {
                    let fresh1 = dst;
                    dst = dst.offset(1);
                    *fresh1 = cp_make_pixel_a(
                        *src.offset(0 as ::core::ffi::c_int as isize),
                        *src.offset(0 as ::core::ffi::c_int as isize),
                        *src.offset(0 as ::core::ffi::c_int as isize),
                        *src.offset(1 as ::core::ffi::c_int as isize),
                    );
                }
                3 => {
                    let fresh2 = dst;
                    dst = dst.offset(1);
                    *fresh2 = cp_make_pixel(
                        *src.offset(0 as ::core::ffi::c_int as isize),
                        *src.offset(1 as ::core::ffi::c_int as isize),
                        *src.offset(2 as ::core::ffi::c_int as isize),
                    );
                }
                4 => {
                    let fresh3 = dst;
                    dst = dst.offset(1);
                    *fresh3 = cp_make_pixel_a(
                        *src.offset(0 as ::core::ffi::c_int as isize),
                        *src.offset(1 as ::core::ffi::c_int as isize),
                        *src.offset(2 as ::core::ffi::c_int as isize),
                        *src.offset(3 as ::core::ffi::c_int as isize),
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
    mut index: ::core::ffi::c_int,
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
    mut w: ::core::ffi::c_int,
    mut h: ::core::ffi::c_int,
    mut src: *mut uint8_t,
    mut dst: *mut cp_pixel_t,
    mut plte: *const uint8_t,
    mut trns: *const uint8_t,
    mut trns_len: uint32_t,
) {
    let mut y: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while y < h {
        src = src.offset(1);
        let mut x: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while x < w {
            let mut c: ::core::ffi::c_int = *src as ::core::ffi::c_int;
            let mut r: uint8_t = *plte.offset((c * 3 as ::core::ffi::c_int) as isize);
            let mut g: uint8_t =
                *plte.offset((c * 3 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize);
            let mut b: uint8_t =
                *plte.offset((c * 3 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as isize);
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
    return cp_make32(chunk.offset(-(8 as ::core::ffi::c_int as isize)));
}
unsafe extern "C" fn cp_out_size(
    mut img: *mut cp_image_t,
    mut bpp: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return ((*img).w + 1 as ::core::ffi::c_int) * (*img).h * bpp;
}
#[no_mangle]
pub unsafe extern "C" fn load_png_mem(
    mut png_data: *const uint8_t,
    mut png_length: ::core::ffi::c_int,
) -> cp_image_t {
    let mut current_block: u64;
    let mut sig: *const ::core::ffi::c_char =
        b"\x89PNG\r\n\x1A\n\0" as *const u8 as *const ::core::ffi::c_char;
    let mut ihdr: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut first: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut plte: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut trns: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut bit_depth: ::core::ffi::c_int = 0;
    let mut color_type: ::core::ffi::c_int = 0;
    let mut bpp: ::core::ffi::c_int = 0;
    let mut w: ::core::ffi::c_int = 0;
    let mut h: ::core::ffi::c_int = 0;
    let mut pix_bytes: ::core::ffi::c_int = 0;
    let mut compression: ::core::ffi::c_int = 0;
    let mut filter: ::core::ffi::c_int = 0;
    let mut interlace: ::core::ffi::c_int = 0;
    let mut datalen: ::core::ffi::c_int = 0;
    let mut offset: ::core::ffi::c_int = 0;
    let mut out: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    let mut img: cp_image_t = cp_image_t {
        w: 0 as ::core::ffi::c_int,
        h: 0,
        pix: ::core::ptr::null_mut::<cp_pixel_t>(),
    };
    let mut data: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    let mut png: cp_raw_png_t = cp_raw_png_t {
        p: ::core::ptr::null::<uint8_t>(),
        end: ::core::ptr::null::<uint8_t>(),
    };
    png.p = png_data as *mut uint8_t;
    png.end = (png_data as *mut uint8_t).offset(png_length as isize);
    if memcmp(
        png.p as *const ::core::ffi::c_void,
        sig as *const ::core::ffi::c_void,
        8 as size_t,
    ) != 0
    {
        cp_error_reason = b"incorrect file signature (is this a png file?)\0" as *const u8
            as *const ::core::ffi::c_char;
    } else {
        png.p = png.p.offset(8 as ::core::ffi::c_int as isize);
        ihdr = cp_chunk(
            &raw mut png,
            b"IHDR\0" as *const u8 as *const ::core::ffi::c_char,
            13 as uint32_t,
        );
        if ihdr.is_null() {
            cp_error_reason =
                b"unable to find IHDR chunk\0" as *const u8 as *const ::core::ffi::c_char;
        } else {
            bit_depth = *ihdr.offset(8 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
            color_type = *ihdr.offset(9 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
            if !(bit_depth == 8 as ::core::ffi::c_int) {
                cp_error_reason = b"only bit-depth of 8 is supported\0" as *const u8
                    as *const ::core::ffi::c_char;
            } else {
                match color_type {
                    0 => {
                        bpp = 1 as ::core::ffi::c_int;
                        current_block = 6450636197030046351;
                    }
                    2 => {
                        bpp = 3 as ::core::ffi::c_int;
                        current_block = 6450636197030046351;
                    }
                    3 => {
                        bpp = 1 as ::core::ffi::c_int;
                        current_block = 6450636197030046351;
                    }
                    4 => {
                        bpp = 2 as ::core::ffi::c_int;
                        current_block = 6450636197030046351;
                    }
                    6 => {
                        bpp = 4 as ::core::ffi::c_int;
                        current_block = 6450636197030046351;
                    }
                    _ => {
                        if 0 as ::core::ffi::c_int == 0 {
                            cp_error_reason =
                                b"unknown color type\0" as *const u8 as *const ::core::ffi::c_char;
                            current_block = 15461442727611312104;
                        } else {
                            current_block = 6450636197030046351;
                        }
                    }
                }
                match current_block {
                    15461442727611312104 => {}
                    _ => {
                        w = cp_make32(ihdr).wrapping_add(1 as uint32_t) as ::core::ffi::c_int;
                        h = cp_make32(ihdr.offset(4 as ::core::ffi::c_int as isize))
                            as ::core::ffi::c_int;
                        if !(w >= 1 as ::core::ffi::c_int) {
                            cp_error_reason =
                                b"invalid IHDR chunk found, image width was less than 1\0"
                                    as *const u8
                                    as *const ::core::ffi::c_char;
                        } else if !(h >= 1 as ::core::ffi::c_int) {
                            cp_error_reason =
                                b"invalid IHDR chunk found, image height was less than 1\0"
                                    as *const u8
                                    as *const ::core::ffi::c_char;
                        } else if !(((w as int64_t * h as int64_t) as usize)
                            .wrapping_mul(::core::mem::size_of::<cp_pixel_t>() as usize)
                            < INT_MAX as usize)
                        {
                            cp_error_reason =
                                b"image too large\0" as *const u8 as *const ::core::ffi::c_char;
                        } else {
                            pix_bytes =
                                ((w * h) as usize)
                                    .wrapping_mul(::core::mem::size_of::<cp_pixel_t>() as usize)
                                    as ::core::ffi::c_int;
                            img.w = w - 1 as ::core::ffi::c_int;
                            img.h = h;
                            img.pix = malloc(pix_bytes as size_t) as *mut cp_pixel_t;
                            if img.pix.is_null() {
                                cp_error_reason = b"unable to allocate raw image space\0"
                                    as *const u8
                                    as *const ::core::ffi::c_char;
                            } else {
                                compression = *ihdr.offset(10 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int;
                                filter = *ihdr.offset(11 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int;
                                interlace = *ihdr.offset(12 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int;
                                if compression != 0 {
                                    cp_error_reason =
                                        b"only standard compression DEFLATE is supported\0"
                                            as *const u8
                                            as *const ::core::ffi::c_char;
                                } else if filter != 0 {
                                    cp_error_reason =
                                        b"only standard adaptive filtering is supported\0"
                                            as *const u8
                                            as *const ::core::ffi::c_char;
                                } else if interlace != 0 {
                                    cp_error_reason = b"interlacing is not supported\0" as *const u8
                                        as *const ::core::ffi::c_char;
                                } else {
                                    first = png.p;
                                    plte = cp_find(
                                        &raw mut png,
                                        b"PLTE\0" as *const u8 as *const ::core::ffi::c_char,
                                        0 as uint32_t,
                                    );
                                    if plte.is_null() {
                                        png.p = first;
                                    } else {
                                        first = png.p;
                                    }
                                    trns = cp_find(
                                        &raw mut png,
                                        b"tRNS\0" as *const u8 as *const ::core::ffi::c_char,
                                        0 as uint32_t,
                                    );
                                    if trns.is_null() {
                                        png.p = first;
                                    } else {
                                        first = png.p;
                                    }
                                    datalen = 0 as ::core::ffi::c_int;
                                    let mut idat: *const uint8_t = cp_find(
                                        &raw mut png,
                                        b"IDAT\0" as *const u8 as *const ::core::ffi::c_char,
                                        0 as uint32_t,
                                    );
                                    while !idat.is_null() {
                                        let mut len: uint32_t = cp_get_chunk_byte_length(idat);
                                        datalen = (datalen as ::core::ffi::c_uint)
                                            .wrapping_add(len as ::core::ffi::c_uint)
                                            as ::core::ffi::c_int
                                            as ::core::ffi::c_int;
                                        idat = cp_chunk(
                                            &raw mut png,
                                            b"IDAT\0" as *const u8 as *const ::core::ffi::c_char,
                                            0 as uint32_t,
                                        );
                                    }
                                    png.p = first;
                                    data = malloc(datalen as size_t) as *mut uint8_t;
                                    offset = 0 as ::core::ffi::c_int;
                                    let mut idat_0: *const uint8_t = cp_find(
                                        &raw mut png,
                                        b"IDAT\0" as *const u8 as *const ::core::ffi::c_char,
                                        0 as uint32_t,
                                    );
                                    while !idat_0.is_null() {
                                        let mut len_0: uint32_t = cp_get_chunk_byte_length(idat_0);
                                        memcpy(
                                            data.offset(offset as isize)
                                                as *mut ::core::ffi::c_void,
                                            idat_0 as *const ::core::ffi::c_void,
                                            len_0 as size_t,
                                        );
                                        offset = (offset as ::core::ffi::c_uint)
                                            .wrapping_add(len_0 as ::core::ffi::c_uint)
                                            as ::core::ffi::c_int
                                            as ::core::ffi::c_int;
                                        idat_0 = cp_chunk(
                                            &raw mut png,
                                            b"IDAT\0" as *const u8 as *const ::core::ffi::c_char,
                                            0 as uint32_t,
                                        );
                                    }
                                    if !(!data.is_null() && datalen >= 6 as ::core::ffi::c_int) {
                                        cp_error_reason =
                                            b"corrupt zlib structure in DEFLATE stream\0"
                                                as *const u8
                                                as *const ::core::ffi::c_char;
                                    } else if !(*data.offset(0 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        & 0xf as ::core::ffi::c_int
                                        == 0x8 as ::core::ffi::c_int)
                                    {
                                        cp_error_reason = b"only zlib compression method (RFC 1950) is supported\0"
                                            as *const u8 as *const ::core::ffi::c_char;
                                    } else if !(*data.offset(0 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        & 0xf0 as ::core::ffi::c_int
                                        <= 0x70 as ::core::ffi::c_int)
                                    {
                                        cp_error_reason = b"innapropriate window size detected\0"
                                            as *const u8
                                            as *const ::core::ffi::c_char;
                                    } else if *data.offset(1 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        & 0x20 as ::core::ffi::c_int
                                        != 0
                                    {
                                        cp_error_reason =
                                            b"preset dictionary is present and not supported\0"
                                                as *const u8
                                                as *const ::core::ffi::c_char;
                                    } else if !(cp_out_size(&raw mut img, 4 as ::core::ffi::c_int)
                                        >= 1 as ::core::ffi::c_int)
                                    {
                                        cp_error_reason = b"invalid image size found\0" as *const u8
                                            as *const ::core::ffi::c_char;
                                    } else if !(cp_out_size(&raw mut img, bpp)
                                        >= 1 as ::core::ffi::c_int)
                                    {
                                        cp_error_reason = b"invalid image size found\0" as *const u8
                                            as *const ::core::ffi::c_char;
                                    } else {
                                        out = (img.pix as *mut uint8_t)
                                            .offset(cp_out_size(
                                                &raw mut img,
                                                4 as ::core::ffi::c_int,
                                            )
                                                as isize)
                                            .offset(-(cp_out_size(&raw mut img, bpp) as isize));
                                        if cp_inflate(
                                            data.offset(2 as ::core::ffi::c_int as isize)
                                                as *mut ::core::ffi::c_void,
                                            datalen - 6 as ::core::ffi::c_int,
                                            out as *mut ::core::ffi::c_void,
                                            pix_bytes,
                                        ) == 0
                                        {
                                            cp_error_reason = b"DEFLATE algorithm failed\0"
                                                as *const u8
                                                as *const ::core::ffi::c_char;
                                        } else if cp_unfilter(img.w, img.h, bpp, out) == 0 {
                                            cp_error_reason = b"invalid filter byte found\0"
                                                as *const u8
                                                as *const ::core::ffi::c_char;
                                        } else {
                                            if color_type == 3 as ::core::ffi::c_int {
                                                if plte.is_null() {
                                                    cp_error_reason = b"color type of indexed requires a PLTE chunk\0"
                                                        as *const u8 as *const ::core::ffi::c_char;
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
                                                    free(data as *mut ::core::ffi::c_void);
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
    free(data as *mut ::core::ffi::c_void);
    free(img.pix as *mut ::core::ffi::c_void);
    img.pix = ::core::ptr::null_mut::<cp_pixel_t>();
    return img;
}
pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
pub const INT_MAX: ::core::ffi::c_int = __INT_MAX__;
