use std::ffi::{c_char, c_int};
use std::mem::MaybeUninit;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const INT_LINE_FORMAT: &[u8] = b"%d\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printIntPtrLine(intNumber: *const c_int) {
    unsafe {
        printf(INT_LINE_FORMAT.as_ptr().cast(), *intNumber);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bad() {
    let data: *mut c_int = unsafe { MaybeUninit::<*mut c_int>::uninit().assume_init() };
    unsafe {
        printIntPtrLine(data.cast_const());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn good() {
    let mut data: c_int = 5;
    let data_addr: *mut c_int = &mut data;
    unsafe {
        printIntPtrLine(data_addr.cast_const());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(useGood: c_int) {
    if useGood != 0 {
        unsafe {
            good();
        }
    } else {
        unsafe {
            bad();
        }
    }
}
