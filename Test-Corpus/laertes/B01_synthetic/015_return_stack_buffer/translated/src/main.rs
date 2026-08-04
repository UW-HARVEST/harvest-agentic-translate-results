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
unsafe extern "C" fn helperBad() -> *mut libc::c_char {
    let mut charString: [libc::c_char; 17] =
        std::mem::transmute::<[u8; 17], [libc::c_char; 17]>(*b"helperBad string\0");
    return &raw mut charString as *mut libc::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn bad() {
    printLine(helperBad());
}
unsafe extern "C" fn helperGood1() -> *mut libc::c_char {
    static mut charString: [libc::c_char; 19] = unsafe {
        std::mem::transmute::<[u8; 19], [libc::c_char; 19]>(*b"helperGood1 string\0")
    };
    return &raw mut charString as *mut libc::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    printLine(helperGood1());
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
