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
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
}
pub type size_t = usize;
unsafe extern "C" fn print_hex(mut p: *mut libc::c_uchar, mut len: libc::c_int) {
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < len {
        printf(
            b"%02x\0" as *const u8 as *const libc::c_char,
            *p.offset(i as isize) as libc::c_int,
        );
        i += 1;
    }
    printf(b"\n\0" as *const u8 as *const libc::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut x: libc::c_float) {
    let mut raw: [libc::c_char; 4] = [0; 4];
    memcpy(
        &raw mut raw as *mut libc::c_char as *mut libc::c_void,
        &raw mut x as *const libc::c_void,
        std::mem::size_of::<libc::c_float>() as size_t,
    );
    print_hex(
        &raw mut raw as *mut libc::c_char as *mut libc::c_uchar,
        std::mem::size_of::<[libc::c_char; 4]>() as libc::c_int,
    );
}
unsafe fn main_0() -> libc::c_int {
    let mut x: libc::c_float = 0.0f32;
    scanf(
        b"%f\0" as *const u8 as *const libc::c_char,
        &raw mut x,
    );
    driver(x);
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
