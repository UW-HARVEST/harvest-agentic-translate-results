use std::ffi::{c_char, c_void};

const REPLACEMENT_INC: usize = 4096;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn strdup(string: *const c_char) -> *mut c_char;
    fn strlen(string: *const c_char) -> usize;
    fn memcpy(dest: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
}

#[inline]
unsafe fn byte_at(string: *const c_char, offset: usize) -> u8 {
    unsafe { *string.add(offset) as u8 }
}

#[inline]
unsafe fn valid_1(string: *const c_char) -> bool {
    unsafe { byte_at(string, 0) & 0x80 == 0 }
}

#[inline]
unsafe fn valid_2(string: *const c_char) -> bool {
    unsafe {
        byte_at(string, 0) & 0xe0 == 0xc0
            && byte_at(string, 0) >= 0xc2
            && byte_at(string, 1) & 0xc0 == 0x80
    }
}

#[inline]
unsafe fn valid_3(string: *const c_char) -> bool {
    unsafe {
        byte_at(string, 0) & 0xf0 == 0xe0
            && byte_at(string, 1) & 0xc0 == 0x80
            && byte_at(string, 2) & 0xc0 == 0x80
            && (byte_at(string, 0) != 0xe0 || byte_at(string, 1) >= 0xa0)
            && (byte_at(string, 0) != 0xed || byte_at(string, 1) < 0xa0)
            && (byte_at(string, 0) != 0xef || byte_at(string, 1) <= 0xbf)
    }
}

#[inline]
unsafe fn valid_4(string: *const c_char) -> bool {
    unsafe {
        byte_at(string, 0) & 0xf8 == 0xf0
            && byte_at(string, 0) <= 0xf4
            && byte_at(string, 1) & 0xc0 == 0x80
            && byte_at(string, 2) & 0xc0 == 0x80
            && byte_at(string, 3) & 0xc0 == 0x80
            && (byte_at(string, 0) != 0xf0 || byte_at(string, 1) >= 0x90)
            && (byte_at(string, 0) != 0xf4 || byte_at(string, 1) <= 0x8f)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_drop(mut string: *const c_char) -> *const c_char {
    while unsafe { *string } != 0 {
        if unsafe { valid_1(string) } {
            string = unsafe { string.add(1) };
        } else if unsafe { valid_2(string) } {
            string = unsafe { string.add(2) };
        } else if unsafe { valid_3(string) } {
            string = unsafe { string.add(3) };
        } else if unsafe { valid_4(string) } {
            string = unsafe { string.add(4) };
        } else {
            return string;
        }
    }

    string
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_filter(string: *const c_char, replacement: bool) -> *mut c_char {
    let mut valid = unsafe { w_utf8_drop(string) };

    if unsafe { *valid } == 0 {
        return unsafe { strdup(string) };
    }

    let mut size = unsafe { strlen(string) } + 1;
    let mut i = unsafe { valid.offset_from(string) as usize };
    let mut repl = 0;

    let mut copy = unsafe { malloc(size) }.cast::<c_char>();
    if copy.is_null() {
        return copy;
    }
    unsafe {
        memcpy(copy.cast(), string.cast(), i);
    }

    while unsafe { *valid } != 0 {
        if unsafe { valid_1(valid) } {
            unsafe {
                *copy.add(i) = *valid;
                i += 1;
                valid = valid.add(1);
            }
        } else if unsafe { valid_2(valid) } {
            for _ in 0..2 {
                unsafe {
                    *copy.add(i) = *valid;
                    i += 1;
                    valid = valid.add(1);
                }
            }
        } else if unsafe { valid_3(valid) } {
            for _ in 0..3 {
                unsafe {
                    *copy.add(i) = *valid;
                    i += 1;
                    valid = valid.add(1);
                }
            }
        } else if unsafe { valid_4(valid) } {
            for _ in 0..4 {
                unsafe {
                    *copy.add(i) = *valid;
                    i += 1;
                    valid = valid.add(1);
                }
            }
        } else {
            if replacement {
                if repl < 3 {
                    size += REPLACEMENT_INC;
                    copy = unsafe { realloc(copy.cast(), size) }.cast();
                    if copy.is_null() {
                        return copy;
                    }
                    repl += REPLACEMENT_INC;
                }

                unsafe {
                    *copy.add(i) = 0xef_u8 as c_char;
                    *copy.add(i + 1) = 0xbf_u8 as c_char;
                    *copy.add(i + 2) = 0xbd_u8 as c_char;
                }
                i += 3;
                repl -= 3;
            }

            valid = unsafe { valid.add(1) };
        }
    }

    unsafe {
        *copy.add(i) = 0;
    }
    copy
}
