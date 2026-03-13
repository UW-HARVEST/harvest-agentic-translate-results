use std::os::raw::c_char;

const REPLACEMENT_INC: usize = 4096;

#[inline]
fn valid_1(x: *const u8) -> bool {
    unsafe { (*x & 0x80) == 0 }
}

#[inline]
fn valid_2(x: *const u8) -> bool {
    unsafe {
        (*x & 0xE0) == 0xC0
            && *x >= 0xC2
            && (*x.add(1) & 0xC0) == 0x80
    }
}

#[inline]
fn valid_3(x: *const u8) -> bool {
    unsafe {
        (*x & 0xF0) == 0xE0
            && (*x.add(1) & 0xC0) == 0x80
            && (*x.add(2) & 0xC0) == 0x80
            && (*x != 0xE0 || *x.add(1) >= 0xA0)
            && (*x != 0xED || *x.add(1) < 0xA0)
            && (*x != 0xEF || *x.add(1) <= 0xBF)
    }
}

#[inline]
fn valid_4(x: *const u8) -> bool {
    unsafe {
        (*x & 0xF8) == 0xF0
            && *x <= 0xF4
            && (*x.add(1) & 0xC0) == 0x80
            && (*x.add(2) & 0xC0) == 0x80
            && (*x.add(3) & 0xC0) == 0x80
            && (*x != 0xF0 || *x.add(1) >= 0x90)
            && (*x != 0xF4 || *x.add(1) <= 0x8F)
    }
}

unsafe fn w_utf8_drop(mut string: *const u8) -> *const u8 {
    while *string != 0 {
        if valid_1(string) {
            string = string.add(1);
        } else if valid_2(string) {
            string = string.add(2);
        } else if valid_3(string) {
            string = string.add(3);
        } else if valid_4(string) {
            string = string.add(4);
        } else {
            return string;
        }
    }
    string
}

#[unsafe(no_mangle)]
pub extern "C" fn w_utf8_filter(string: *const c_char, replacement: bool) -> *mut c_char {
    assert!(!string.is_null());

    unsafe {
        let s = string as *const u8;
        let valid = w_utf8_drop(s);

        if *valid == 0 {
            return libc::strdup(string);
        }

        let size = libc::strlen(string) + 1;
        let mut size = size;
        let mut i = valid.offset_from(s) as usize;

        let mut copy = libc::malloc(size) as *mut u8;
        if copy.is_null() {
            return std::ptr::null_mut();
        }
        std::ptr::copy_nonoverlapping(s, copy, i);

        let mut valid = valid;
        let mut repl: usize = 0;

        while *valid != 0 {
            if valid_1(valid) {
                *copy.add(i) = *valid;
                i += 1;
                valid = valid.add(1);
            } else if valid_2(valid) {
                *copy.add(i) = *valid;
                *copy.add(i + 1) = *valid.add(1);
                i += 2;
                valid = valid.add(2);
            } else if valid_3(valid) {
                *copy.add(i) = *valid;
                *copy.add(i + 1) = *valid.add(1);
                *copy.add(i + 2) = *valid.add(2);
                i += 3;
                valid = valid.add(3);
            } else if valid_4(valid) {
                *copy.add(i) = *valid;
                *copy.add(i + 1) = *valid.add(1);
                *copy.add(i + 2) = *valid.add(2);
                *copy.add(i + 3) = *valid.add(3);
                i += 4;
                valid = valid.add(4);
            } else {
                if replacement {
                    if repl < 3 {
                        size += REPLACEMENT_INC;
                        copy = libc::realloc(copy as *mut libc::c_void, size) as *mut u8;
                        if copy.is_null() {
                            return std::ptr::null_mut();
                        }
                        repl += REPLACEMENT_INC;
                    }

                    *copy.add(i) = 0xEF;
                    *copy.add(i + 1) = 0xBF;
                    *copy.add(i + 2) = 0xBD;
                    i += 3;
                    repl -= 3;
                }

                valid = valid.add(1);
            }
        }

        *copy.add(i) = 0;
        copy as *mut c_char
    }
}
