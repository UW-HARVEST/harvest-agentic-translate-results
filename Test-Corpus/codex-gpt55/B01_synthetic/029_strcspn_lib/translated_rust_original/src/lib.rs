use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

unsafe fn strcspn(mut s1: *const c_char, s2: *const c_char) -> usize {
    let mut count = 0usize;

    while unsafe { *s1 } != 0 {
        let ch = unsafe { *s1 as u8 };
        let mut reject = s2;

        while unsafe { *reject } != 0 {
            if ch == unsafe { *reject as u8 } {
                return count;
            }
            reject = unsafe { reject.add(1) };
        }

        count += 1;
        s1 = unsafe { s1.add(1) };
    }

    count
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let count = unsafe { strcspn(s1, s2) };
    unsafe {
        printf(c"%zu\n".as_ptr(), count);
    }
}
