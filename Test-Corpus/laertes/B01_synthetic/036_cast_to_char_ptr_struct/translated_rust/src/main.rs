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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct house_t {
    pub floors: libc::c_int,
    pub bedrooms: libc::c_int,
    pub bathrooms: libc::c_double,
}
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
pub unsafe extern "C" fn driver(mut floors: libc::c_int) {
    let mut house: house_t = house_t {
        floors: 0 as libc::c_int,
        bedrooms: 0,
        bathrooms: 0.,
    };
    house.floors = floors;
    house.bedrooms = 3 as libc::c_int;
    house.bathrooms = 2.0f64;
    print_hex(
        &raw mut house as *mut libc::c_uchar,
        std::mem::size_of::<house_t>() as libc::c_int,
    );
}
unsafe fn main_0() -> libc::c_int {
    let mut x: libc::c_int = 0 as libc::c_int;
    scanf(
        b"%d\0" as *const u8 as *const libc::c_char,
        &raw mut x,
    );
    driver(x);
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
