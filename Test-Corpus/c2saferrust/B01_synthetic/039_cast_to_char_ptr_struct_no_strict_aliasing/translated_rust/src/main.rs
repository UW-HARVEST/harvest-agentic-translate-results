#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]
#[allow(unused_imports)]
use ::driver;
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn scanf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct house_t {
    pub floors: ::core::ffi::c_int,
    pub bedrooms: ::core::ffi::c_int,
    pub bathrooms: ::core::ffi::c_double,
}
unsafe extern "C" fn print_hex(mut p: *mut ::core::ffi::c_uchar, mut len: ::core::ffi::c_int) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < len {
        printf(
            b"%02x\0" as *const u8 as *const ::core::ffi::c_char,
            *p.offset(i as isize) as ::core::ffi::c_int,
        );
        i += 1;
    }
    printf(b"\n\0" as *const u8 as *const ::core::ffi::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut floors: ::core::ffi::c_int) {
    let mut house: house_t = house_t {
        floors: 0 as ::core::ffi::c_int,
        bedrooms: 0,
        bathrooms: 0.,
    };
    house.floors = floors;
    house.bedrooms = 3 as ::core::ffi::c_int;
    house.bathrooms = 2.0f64;
    let mut raw: [::core::ffi::c_char; 16] = [0; 16];
    memcpy(
        &raw mut raw as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
        &raw mut house as *const ::core::ffi::c_void,
        ::core::mem::size_of::<house_t>() as size_t,
    );
    print_hex(
        &raw mut raw as *mut ::core::ffi::c_uchar,
        ::core::mem::size_of::<[::core::ffi::c_char; 16]>() as ::core::ffi::c_int,
    );
}
unsafe fn main_0() -> ::core::ffi::c_int {
    let mut x: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut x,
    );
    driver(x);
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
