use std::ffi::{c_char, c_int};
use std::mem::MaybeUninit;
use std::ptr;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

static STRING_LINE_FORMAT: &[u8] = b"%s\n\0";
static INT_LINE_FORMAT: &[u8] = b"%d\n\0";

#[allow(dead_code)]
#[repr(align(16))]
struct AlignedBytes([u8; 10]);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(STRING_LINE_FORMAT.as_ptr().cast(), line);
    }
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn printIntLine(intNumber: c_int) {
    printf(INT_LINE_FORMAT.as_ptr().cast(), intNumber);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data: *mut c_int;
    let mut storage = MaybeUninit::<AlignedBytes>::uninit();
    data = storage.as_mut_ptr().cast::<c_int>();

    let source: [c_int; 10] = [0; 10];
    let mut i: usize = 0;
    while i < 10 {
        ptr::write(data.add(i), source[i]);
        i += 1;
    }
    printIntLine(ptr::read(data));
}

#[unsafe(no_mangle)]
#[allow(unused_assignments)]
pub unsafe extern "C" fn good() {
    let mut data: *mut c_int;
    data = ptr::null_mut();

    let mut storage = [0 as c_int; 10];
    data = storage.as_mut_ptr();

    let source: [c_int; 10] = [0; 10];
    let mut i: usize = 0;
    while i < 10 {
        ptr::write(data.add(i), source[i]);
        i += 1;
    }
    printIntLine(ptr::read(data));
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        good();
    } else {
        bad();
    }
}
