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
pub unsafe extern "C" fn printIntPtrLine(mut intNumber: *const libc::c_int) {
    printf(
        b"%d\n\0" as *const u8 as *const libc::c_char,
        *intNumber,
    );
}
#[no_mangle]
pub unsafe extern "C" fn bad() {
    let mut data: *mut libc::c_int = std::ptr::null_mut::<libc::c_int>();
    printIntPtrLine(data);
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    let mut data: libc::c_int = 0;
    data = 5 as libc::c_int;
    let mut data_addr: *mut libc::c_int = std::ptr::null_mut::<libc::c_int>();
    data_addr = &raw mut data;
    printIntPtrLine(data_addr);
}
unsafe fn main_0() -> libc::c_int {
    let mut x: libc::c_int = 0 as libc::c_int;
    scanf(
        b"%d\0" as *const u8 as *const libc::c_char,
        &raw mut x,
    );
    if x != 0 {
        good();
    } else {
        bad();
    }
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
