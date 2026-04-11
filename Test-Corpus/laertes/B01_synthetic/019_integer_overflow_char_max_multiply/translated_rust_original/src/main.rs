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
pub unsafe extern "C" fn printHexCharLine(mut charHex: libc::c_char) {
    printf(
        b"%02x\n\0" as *const u8 as *const libc::c_char,
        charHex as libc::c_int,
    );
}
#[no_mangle]
pub unsafe extern "C" fn bad() {
    let mut data: libc::c_char = 0;
    data = CHAR_MAX as libc::c_char;
    if data as libc::c_int > 0 as libc::c_int {
        let mut result: libc::c_char =
            (data as libc::c_int * 2 as libc::c_int) as libc::c_char;
        printHexCharLine(result);
    }
}
unsafe extern "C" fn goodG2B() {
    let mut data: libc::c_char = 0;
    data = 2 as libc::c_char;
    if data as libc::c_int > 0 as libc::c_int {
        let mut result: libc::c_char =
            (data as libc::c_int * 2 as libc::c_int) as libc::c_char;
        printHexCharLine(result);
    }
}
unsafe extern "C" fn goodB2G() {
    let mut data: libc::c_char = 0;
    data = ' ' as i32 as libc::c_char;
    data = CHAR_MAX as libc::c_char;
    if data as libc::c_int > 0 as libc::c_int {
        if (data as libc::c_int) < CHAR_MAX / 2 as libc::c_int {
            let mut result: libc::c_char =
                (data as libc::c_int * 2 as libc::c_int) as libc::c_char;
            printHexCharLine(result);
        } else {
            printLine(
                b"data value is too large to perform arithmetic safely.\0" as *const u8
                    as *const libc::c_char,
            );
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    goodG2B();
    goodB2G();
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
pub const __SCHAR_MAX__: libc::c_int = 127 as libc::c_int;
pub const CHAR_MAX: libc::c_int = __SCHAR_MAX__;
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
