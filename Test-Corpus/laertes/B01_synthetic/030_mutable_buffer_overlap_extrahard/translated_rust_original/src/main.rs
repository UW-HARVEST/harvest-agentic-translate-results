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
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn scanf(__format: *const libc::c_char, ...) -> libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn fma_array(
    mut out: *mut libc::c_int,
    mut mul1: *const libc::c_int,
    mut mul2: *const libc::c_int,
    mut add: *const libc::c_int,
    mut len: libc::c_int,
) {
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < len {
        *out.offset(i as isize) =
            *mul1.offset(i as isize) * *mul2.offset(i as isize) + *add.offset(i as isize);
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut out: *mut libc::c_int, mut len: libc::c_int) {
    fma_array(out, out, out, out, len);
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < len {
        printf(
            b"%d\n\0" as *const u8 as *const libc::c_char,
            *out.offset(i as isize),
        );
        i += 1;
    }
}
unsafe fn main_0() -> libc::c_int {
    let mut data: [libc::c_int; 100] = [0; 100];
    let mut i: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < 100 as libc::c_int {
        if scanf(
            b"%d\0" as *const u8 as *const libc::c_char,
            (&raw mut data as *mut libc::c_int).offset(i as isize)
                as *mut libc::c_int,
        ) != 1 as libc::c_int
        {
            break;
        }
        i += 1;
    }
    driver(&raw mut data as *mut libc::c_int, i);
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
