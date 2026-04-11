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
pub unsafe extern "C" fn call_fma(
    mut data: *const libc::c_int,
    mut len: libc::c_int,
) -> libc::c_int {
    if len == 0 as libc::c_int {
        return 0 as libc::c_int;
    }
    let vla = len as usize;
    let mut out: Vec<libc::c_int> = ::std::vec::from_elem(0, vla);
    let vla_0 = len as usize;
    let mut ones: Vec<libc::c_int> = ::std::vec::from_elem(0, vla_0);
    let vla_1 = len as usize;
    let mut zeros: Vec<libc::c_int> = ::std::vec::from_elem(0, vla_1);
    *out.as_mut_ptr().offset(0 as libc::c_int as isize) = 0 as libc::c_int;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < len {
        *ones.as_mut_ptr().offset(i as isize) = 1 as libc::c_int;
        *zeros.as_mut_ptr().offset(i as isize) = 0 as libc::c_int;
        i += 1;
    }
    fma_array(
        out.as_mut_ptr(),
        ones.as_mut_ptr(),
        data,
        zeros.as_mut_ptr(),
        len,
    );
    return *out
        .as_mut_ptr()
        .offset((len - 1 as libc::c_int) as isize);
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
    let mut result: libc::c_int = call_fma(&raw mut data as *mut libc::c_int, i);
    printf(b"%d\n\0" as *const u8 as *const libc::c_char, result);
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
