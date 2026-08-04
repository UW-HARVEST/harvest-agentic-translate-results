use std::ffi::{c_char, c_int};
use std::ptr;

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe { printf(b"%s\n\0".as_ptr() as *const c_char, line) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    unsafe { printf(b"%d\n\0".as_ptr() as *const c_char, int_number) };
}

#[repr(C, align(16))]
struct AllocaBuf<const N: usize>([u8; N]);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    // C does alloca(10) then writes 10 ints — buffer overflow UB.
    // We allocate enough to survive the writes so observable output matches.
    let mut buf = AllocaBuf([0u8; 10 * std::mem::size_of::<c_int>()]);
    let data: *mut c_int = buf.0.as_mut_ptr() as *mut c_int;
    let source: [c_int; 10] = [0; 10];
    for i in 0..10 {
        unsafe { ptr::write(data.add(i), source[i]) };
    }
    unsafe { printIntLine(*data) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let mut buf = AllocaBuf([0u8; 10 * std::mem::size_of::<c_int>()]);
    let data: *mut c_int = buf.0.as_mut_ptr() as *mut c_int;
    let source: [c_int; 10] = [0; 10];
    for i in 0..10 {
        unsafe { ptr::write(data.add(i), source[i]) };
    }
    unsafe { printIntLine(*data) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        unsafe { good() };
    } else {
        unsafe { bad() };
    }
}
