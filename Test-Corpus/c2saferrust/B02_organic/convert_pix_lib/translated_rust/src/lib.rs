







use core::slice;

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
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
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
pub type __uint64_t = u64;
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
fn cp_make_pixel(r: uint8_t, g: uint8_t, b: uint8_t) -> cp_pixel_t {
    cp_pixel_t {
        r,
        g,
        b,
        a: 0xff as uint8_t,
    }
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
fn cp_ptr(s: &cp_state_t) -> *mut ::core::ffi::c_char {
    assert_eq!(s.bits_left & 7, 0);
    let byte_offset = (s.count / 8 as ::core::ffi::c_int) as isize;
    let word_ptr = s.words.wrapping_offset(s.word_index as isize) as *mut ::core::ffi::c_char;
    word_ptr.wrapping_offset(-byte_offset)
}

fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: i32) -> u64 {
    unsafe {
        if (*s).count < num_bits_to_read {
            if (*s).word_index < (*s).word_count {
                let index = (*s).word_index;
                (*s).word_index += 1;

                let word: u32 = *(*s).words.add(index as usize);
                (*s).bits |= (word as u64) << (*s).count;
                (*s).count += 32;

                assert!((*s).word_index <= (*s).word_count);
            } else if (*s).final_word_available != 0 {
                let word = (*s).final_word;
                (*s).bits |= (word as u64) << (*s).count;
                (*s).count += (*s).bits_left;
                (*s).final_word_available = 0;
            }
        }
        (*s).bits
    }
}

fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: ::core::ffi::c_int) -> uint32_t {
    let s = unsafe { &mut *s };

    assert!(s.count >= num_bits_to_read);

    let mask = ((1u64 << num_bits_to_read) - 1) as u64;
    let bits = (s.bits & mask) as uint32_t;

    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;

    bits
}

fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: ::core::ffi::c_int) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!(unsafe { (*s).bits_left > 0 });
    assert!(unsafe { (*s).count <= 64 });
    assert!(unsafe { cp_would_overflow(s, num_bits_to_read) == 0 });

    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

fn cp_rev16(mut a: u32) -> u32 {
    a &= 0xffff;
    a = ((a & 0xaaaa) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xcccc) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xf0f0) >> 4) | ((a & 0x0f0f) << 4);
    ((a & 0xff00) >> 8) | ((a & 0x00ff) << 8)
}

fn cp_build(
    s: Option<&mut cp_state_t>,
    tree: &mut [uint32_t],
    lens: &[uint8_t],
    sym_count: usize,
) -> i32 {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];

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

    if let Some(state) = s {
        state.lookup.fill(0);

        for (i, &len_u8) in lens.iter().take(sym_count).enumerate() {
            let len = len_u8 as i32;
            if len != 0 {
                assert!(len < 16, "len < 16");

                let code = codes[len as usize] as uint32_t;
                codes[len as usize] += 1;

                let slot = first[len as usize] as usize;
                first[len as usize] += 1;

                tree[slot] = (code << (32 - len)) | ((i as uint32_t) << 4) | (len as uint32_t);

                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1usize << 9) {
                        state.lookup[j] = ((len << 9) | i as i32) as uint16_t;
                        j += 1usize << len;
                    }
                }
            }
        }
    } else {
        for (i, &len_u8) in lens.iter().take(sym_count).enumerate() {
            let len = len_u8 as i32;
            if len != 0 {
                assert!(len < 16, "len < 16");

                let code = codes[len as usize] as uint32_t;
                codes[len as usize] += 1;

                let slot = first[len as usize] as usize;
                first[len as usize] += 1;

                tree[slot] = (code << (32 - len)) | ((i as uint32_t) << 4) | (len as uint32_t);
            }
        }
    }

    first[15]
}

unsafe extern "C" fn cp_stored(mut s: *mut cp_state_t) -> ::core::ffi::c_int {
    let mut p: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    cp_read_bits(s, (*s).count & 7 as ::core::ffi::c_int);
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
        p = cp_ptr(&*s);
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
    &cp_fixed_table[..],
    288,
) as uint32_t;
    (*s).ndst = cp_build(
    None,
    &mut (*s).dst[..],
    &cp_fixed_table[288..],
    32,
) as uint32_t;
    return 1 as ::core::ffi::c_int;
}
fn cp_decode(s: *mut cp_state_t, tree: *mut u32, mut hi: i32) -> i32 {
    let bits: u64 = cp_peak_bits(s, 16);
    let search: u32 = (cp_rev16(bits as u32) << 16) | 0xffff;
    let tree: &[u32] = unsafe { slice::from_raw_parts(tree, hi as usize) };
    let mut lo: i32 = 0;

    while lo < hi {
        let guess: i32 = (lo + hi) >> 1;
        if search < tree[guess as usize] {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }

    let key: u32 = tree[(lo - 1) as usize];
    let len: u32 = 32u32.wrapping_sub(key & 0xf);

    assert_eq!(search >> len, key >> len);

    let _ = cp_consume_bits(s, (key & 0xf) as i32);
    ((key >> 4) & 0xfff) as i32
}

fn cp_dynamic(s: &mut cp_state_t) -> ::core::ffi::c_int {
    let mut lenlens: [uint8_t; 19] = [0; 19];
    let nlit = (257u32.wrapping_add(cp_read_bits(s, 5))) as usize;
    let ndst = (1u32.wrapping_add(cp_read_bits(s, 5))) as usize;
    let nlen = (4u32.wrapping_add(cp_read_bits(s, 4))) as usize;

    let mut i = 0usize;
    while i < nlen {
        lenlens[cp_permutation_order[i] as usize] = cp_read_bits(s, 3) as uint8_t;
        i += 1;
    }

    s.nlen = cp_build(None, &mut s.len, &lenlens, 19) as uint32_t;

    let mut lens: [uint8_t; 320] = [0; 320];
    let mut n = 0usize;
    while n < nlit + ndst {
        let sym = cp_decode(s as *mut cp_state_t, s.len.as_mut_ptr(), s.nlen as ::core::ffi::c_int);
        match sym {
            16 => {
                let mut count = (3u32.wrapping_add(cp_read_bits(s, 2))) as usize;
                while count != 0 {
                    lens[n] = lens[n - 1];
                    count -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut count = (3u32.wrapping_add(cp_read_bits(s, 3))) as usize;
                while count != 0 {
                    lens[n] = 0;
                    count -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut count = (11u32.wrapping_add(cp_read_bits(s, 7))) as usize;
                while count != 0 {
                    lens[n] = 0;
                    count -= 1;
                    n += 1;
                }
            }
            _ => {
                lens[n] = sym as uint8_t;
                n += 1;
            }
        }
    }

    s.nlit = cp_build(None, &mut s.lit, &lens[..nlit], nlit) as uint32_t;
    s.ndst = cp_build(None, &mut s.dst, &lens[nlit..nlit + ndst], ndst) as uint32_t;

    1
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
                current_block = 14638442084202155990;
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
    cp_read_bits(
        s,
        cp_len_extra_bits[symbol as usize] as ::core::ffi::c_int,
    )
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
                current_block = 14638442084202155990;
                break;
            } else if !((*s).out.offset(length as isize) <= (*s).out_end) {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a string.\0"
                    as *const u8 as *const ::core::ffi::c_char;
                current_block = 14638442084202155990;
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
                        let fresh8 = length;
                        length = length - 1;
                        if !(fresh8 != 0) {
                            break;
                        }
                        let fresh9 = src;
                        src = src.offset(1);
                        let fresh10 = dst;
                        dst = dst.offset(1);
                        *fresh10 = *fresh9;
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
                    current_block = 10680648185232105664;
                    break;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    current_block = 10680648185232105664;
                    break;
                }
            }
            2 => {
                cp_dynamic(unsafe { &mut *s });
                if cp_block(s) == 0 {
                    current_block = 10680648185232105664;
                    break;
                }
            }
            3 => {
                if 0 as ::core::ffi::c_int == 0 {
                    cp_error_reason = b"Detected unknown block type within input stream.\0"
                        as *const u8
                        as *const ::core::ffi::c_char;
                    current_block = 10680648185232105664;
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
        10680648185232105664 => {
            free(s as *mut ::core::ffi::c_void);
            return 0 as ::core::ffi::c_int;
        }
        _ => {
            free(s as *mut ::core::ffi::c_void);
            return 1 as ::core::ffi::c_int;
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn convert_pix(
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
