use std::mem::offset_of;
use std::os::raw::{c_char, c_int};
use std::ptr;

#[repr(C)]
pub struct Test {
    pub a: c_int,
    pub b: c_int,
}

extern "C" {
    fn atoi(value: *const c_char) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[no_mangle]
pub unsafe extern "C" fn find_container_of_a(i: *mut c_int) -> *mut Test {
    i.cast::<u8>()
        .wrapping_sub(offset_of!(Test, a))
        .cast::<Test>()
}

#[no_mangle]
pub unsafe extern "C" fn find_container_of_b(i: *mut c_int) -> *mut Test {
    i.cast::<u8>()
        .wrapping_sub(offset_of!(Test, b))
        .cast::<Test>()
}

#[no_mangle]
pub unsafe extern "C" fn main(_argc: c_int, argv: *mut *mut c_char) -> c_int {
    let a_ptr: *mut c_char = ptr::read(argv.wrapping_add(1));
    let b_ptr: *mut c_char = ptr::read(argv.wrapping_add(2));
    let a = atoi(a_ptr);
    let b = atoi(b_ptr);

    let mut value = Test { a: 0, b: 0 };
    value.a = a;
    value.b = b;

    let sum = (*find_container_of_a(&mut value.a))
        .a
        .wrapping_add((*find_container_of_b(&mut value.b)).b);
    printf(b"%d\n\0".as_ptr().cast(), sum);
    0
}
