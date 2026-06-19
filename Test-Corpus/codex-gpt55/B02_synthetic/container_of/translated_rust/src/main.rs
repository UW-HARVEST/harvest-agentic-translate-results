#![no_main]

use std::ffi::{c_char, c_int};
use std::mem::offset_of;

#[repr(C)]
struct Test {
    a: c_int,
    b: c_int,
}

extern "C" {
    fn atoi(nptr: *const c_char) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

unsafe fn find_container_of_a(i: *mut c_int) -> *mut Test {
    i.cast::<u8>().sub(offset_of!(Test, a)).cast::<Test>()
}

unsafe fn find_container_of_b(i: *mut c_int) -> *mut Test {
    i.cast::<u8>().sub(offset_of!(Test, b)).cast::<Test>()
}

#[no_mangle]
pub unsafe extern "C" fn main(_argc: c_int, argv: *mut *mut c_char) -> c_int {
    let a = atoi(*argv.add(1));
    let b = atoi(*argv.add(2));

    let mut t = Test { a: 0, b: 0 };
    t.a = a;
    t.b = b;

    printf(
        b"%d\n\0".as_ptr().cast::<c_char>(),
        (*find_container_of_a(&mut t.a)).a.wrapping_add((*find_container_of_b(&mut t.b)).b),
    );

    0
}
