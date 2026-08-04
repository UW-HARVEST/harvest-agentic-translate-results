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
static mut y: libc::c_int = 123 as libc::c_int;
unsafe extern "C" fn multi_stage(
    mut x: libc::c_int,
    mut z: libc::c_int,
) -> libc::c_int {
    let mut result: libc::c_int = 0 as libc::c_int;
    if x != 1 as libc::c_int {
        printf(b"Error: x != 1\n\0" as *const u8 as *const libc::c_char);
        result = 1 as libc::c_int;
    } else if y != 2 as libc::c_int {
        printf(b"Error: x == 1 but y != 2\n\0" as *const u8 as *const libc::c_char);
        result = 2 as libc::c_int;
    } else if z != 3 as libc::c_int {
        printf(
            b"Error: x == 1 and y == 2, but z != 3\n\0" as *const u8 as *const libc::c_char,
        );
        result = 3 as libc::c_int;
    } else {
        printf(b"Ok!\n\0" as *const u8 as *const libc::c_char);
        return result;
    }
    printf(b"Operation failed\n\0" as *const u8 as *const libc::c_char);
    return result;
}
unsafe fn main_0() -> libc::c_int {
    let mut x: libc::c_int = 0 as libc::c_int;
    let mut z: libc::c_int = 0 as libc::c_int;
    scanf(
        b"%d %d %d\0" as *const u8 as *const libc::c_char,
        &raw mut x,
        &raw mut y,
        &raw mut z,
    );
    let mut result: libc::c_int = multi_stage(x, z);
    printf(
        b"Result: %d\n\0" as *const u8 as *const libc::c_char,
        result,
    );
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
