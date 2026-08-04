#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]
#[macro_use]
extern crate c2rust_bitfields;
#[allow(unused_imports)]
use ::driver;
extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn scanf(__format: *const libc::c_char, ...) -> libc::c_int;
}
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C)]
pub struct foo_t {
    #[bitfield(name = "x", ty = "libc::c_uint", bits = "0..=1")]
    #[bitfield(name = "y", ty = "libc::c_uint", bits = "2..=4")]
    #[bitfield(name = "b", ty = "bool", bits = "5..=5")]
    pub x_y_b: [u8; 1],
    #[bitfield(padding)]
    pub c2rust_padding: [u8; 3],
    pub z: libc::c_int,
}
#[no_mangle]
pub unsafe extern "C" fn print_foo(mut foo: *const foo_t) {
    printf(
        b"%u %u %d %d\n\0" as *const u8 as *const libc::c_char,
        (*foo).x() as libc::c_int,
        (*foo).y() as libc::c_int,
        (*foo).b() as libc::c_int,
        (*foo).z,
    );
}
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut x: libc::c_uint,
    mut y: libc::c_uint,
    mut b: bool,
    mut z: libc::c_int,
) {
    let mut foo: foo_t = {
        let mut init = foo_t {
            x_y_b: [0; 1],
            c2rust_padding: [0; 3],
            z: z,
        };
        init.set_x(x);
        init.set_y(y);
        init.set_b(b);
        init
    };
    print_foo(&raw mut foo);
}
unsafe fn main_0() -> libc::c_int {
    let mut x: libc::c_uint = 0 as libc::c_uint;
    let mut y: libc::c_uint = 0 as libc::c_uint;
    let mut b: libc::c_int = 0 as libc::c_int;
    let mut z: libc::c_int = 0 as libc::c_int;
    scanf(
        b"%u\0" as *const u8 as *const libc::c_char,
        &raw mut x,
    );
    scanf(
        b"%u\0" as *const u8 as *const libc::c_char,
        &raw mut y,
    );
    scanf(
        b"%d\0" as *const u8 as *const libc::c_char,
        &raw mut b,
    );
    scanf(
        b"%d\0" as *const u8 as *const libc::c_char,
        &raw mut z,
    );
    driver(x, y, b != 0, z);
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
