use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

unsafe fn span_without_bytes(string: *const c_char, rejected: *const c_char) -> usize {
    let mut length = 0;

    loop {
        let byte = unsafe { *string.add(length).cast::<u8>() };
        if byte == 0 {
            return length;
        }

        let mut rejected_index = 0;
        loop {
            let rejected_byte = unsafe { *rejected.add(rejected_index).cast::<u8>() };
            if rejected_byte == 0 {
                break;
            }
            if byte == rejected_byte {
                return length;
            }
            rejected_index += 1;
        }

        length += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(s1: *const c_char, s2: *const c_char) {
    let length = unsafe { span_without_bytes(s1, s2) };
    unsafe {
        printf(c"%zu\n".as_ptr(), length);
    }
}
