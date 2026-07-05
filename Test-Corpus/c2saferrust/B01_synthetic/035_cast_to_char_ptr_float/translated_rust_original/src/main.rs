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
pub unsafe extern "C" fn driver(mut x: ::core::ffi::c_float) {
    print_hex(
        &raw mut x as *mut ::core::ffi::c_uchar,
        ::core::mem::size_of::<::core::ffi::c_float>() as ::core::ffi::c_int,
    );
}
unsafe fn main_0() -> ::core::ffi::c_int {
    let mut x: ::core::ffi::c_float = 0.0f32;
    scanf(
        b"%f\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut x,
    );
    driver(x);
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
