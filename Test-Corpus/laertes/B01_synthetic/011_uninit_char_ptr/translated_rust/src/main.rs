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
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn printLine(mut line: *const libc::c_char) {
    if !line.is_null() {
        printf(b"%s\n\0" as *const u8 as *const libc::c_char, line);
    }
}
#[no_mangle]
pub unsafe extern "C" fn bad() {
    let mut data: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    printLine(data);
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    let mut data: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    data = b"string\0" as *const u8 as *const libc::c_char as *mut libc::c_char;
    printLine(data);
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
