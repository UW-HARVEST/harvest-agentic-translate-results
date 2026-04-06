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
unsafe extern "C" fn foo(mut x: ::core::ffi::c_int, mut y: ::core::ffi::c_int) {
    let mut current_block_6: u64;
    while x > 0 as ::core::ffi::c_int || y > 0 as ::core::ffi::c_int {
        printf(b"loop\n\0" as *const u8 as *const ::core::ffi::c_char);
        if x == 1 as ::core::ffi::c_int && y == 4 as ::core::ffi::c_int {
            current_block_6 = 6231185082549427558;
        } else {
            current_block_6 = 15966560278257819154;
        }
        loop {
            match current_block_6 {
                15966560278257819154 => {
                    if x > 0 as ::core::ffi::c_int {
                        printf(b"x\n\0" as *const u8 as *const ::core::ffi::c_char);
                        x -= 1;
                    }
                    current_block_6 = 6231185082549427558;
                }
                _ => {
                    if y == 0 as ::core::ffi::c_int {
                        break;
                    }
                    printf(b"y\n\0" as *const u8 as *const ::core::ffi::c_char);
                    y -= 1;
                    if x < 3 as ::core::ffi::c_int {
                        current_block_6 = 15966560278257819154;
                    } else {
                        break;
                    }
                }
            }
        }
    }
}
unsafe fn main_0() -> ::core::ffi::c_int {
    let mut x: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut y: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    scanf(
        b"%d %d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut x,
        &raw mut y,
    );
    foo(x, y);
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
