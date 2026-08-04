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
static mut the_house: house_t = house_t {
    floors: 2 as libc::c_int,
    bedrooms: 5 as libc::c_int,
    bathrooms: 2.5f64,
};
unsafe extern "C" fn add_floor(mut house: *mut house_t) {
    (*house).floors += 1;
}
unsafe extern "C" fn add_bedrooms(mut house: *mut house_t, mut extra_bedrooms: libc::c_int) {
    (*house).bedrooms += extra_bedrooms;
}
unsafe extern "C" fn add_floor_to_the_house() {
    add_floor(&raw mut the_house);
}
unsafe extern "C" fn print_the_house() {
    printf(
        b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0" as *const u8
            as *const libc::c_char,
        the_house.floors,
        the_house.bedrooms,
        the_house.bathrooms,
    );
}
#[no_mangle]
pub unsafe extern "C" fn run(mut extra_bedrooms: libc::c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    the_house.bathrooms += 1.0f64;
    print_the_house();
    add_bedrooms(&raw mut the_house, extra_bedrooms);
    print_the_house();
}
unsafe fn main_0() -> libc::c_int {
    let mut x: libc::c_int = 0 as libc::c_int;
    scanf(
        b"%d\0" as *const u8 as *const libc::c_char,
        &raw mut x,
    );
    run(x);
    run(x);
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
