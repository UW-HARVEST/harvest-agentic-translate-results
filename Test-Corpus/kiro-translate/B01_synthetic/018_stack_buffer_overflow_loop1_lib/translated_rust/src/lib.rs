use std::ffi::{c_char, c_int, c_void};
use std::ptr;

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn alloca(size: usize) -> *mut c_void;
}

unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe { printf(b"%s\n\0".as_ptr() as *const c_char, line) };
    }
}

unsafe fn print_int_line(int_number: c_int) {
    unsafe { printf(b"%d\n\0".as_ptr() as *const c_char, int_number) };
}

unsafe fn bad() {
    let data: *mut c_int = unsafe { alloca(10) } as *mut c_int;
    let source: [c_int; 10] = [0; 10];
    for i in 0..10 {
        unsafe { ptr::write(data.add(i), source[i]) };
    }
    unsafe { print_int_line(*data) };
}

unsafe fn good() {
    let data: *mut c_int =
        unsafe { alloca(10 * std::mem::size_of::<c_int>()) } as *mut c_int;
    let source: [c_int; 10] = [0; 10];
    for i in 0..10 {
        unsafe { ptr::write(data.add(i), source[i]) };
    }
    unsafe { print_int_line(*data) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        unsafe { good() };
    } else {
        unsafe { bad() };
    }
}
