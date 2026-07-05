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
#[no_mangle]
fn fma_array(out: &mut [i32], mul1: &[i32], mul2: &[i32], add: &[i32]) {
    for (((o, a), b), c) in out.iter_mut().zip(mul1.iter()).zip(mul2.iter()).zip(add.iter()) {
        *o = *a * *b + *c;
    }
}

#[no_mangle]
pub unsafe extern "C" fn driver(mut out: *mut ::core::ffi::c_int, mut len: ::core::ffi::c_int) {
    let len_usize = len as usize;
let out_slice = unsafe { std::slice::from_raw_parts_mut(out, len_usize) };
let in_slice = unsafe { std::slice::from_raw_parts(out as *const i32, len_usize) };
fma_array(out_slice, in_slice, in_slice, in_slice);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < len {
        printf(
            b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
            *out.offset(i as isize),
        );
        i += 1;
    }
}
unsafe fn main_0() -> ::core::ffi::c_int {
    let mut data: [::core::ffi::c_int; 100] = [0; 100];
    let mut i: ::core::ffi::c_int = 0;
    i = 0 as ::core::ffi::c_int;
    while i < 100 as ::core::ffi::c_int {
        if scanf(
            b"%d\0" as *const u8 as *const ::core::ffi::c_char,
            (&raw mut data as *mut ::core::ffi::c_int).offset(i as isize)
                as *mut ::core::ffi::c_int,
        ) != 1 as ::core::ffi::c_int
        {
            break;
        }
        i += 1;
    }
    driver(&raw mut data as *mut ::core::ffi::c_int, i);
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
