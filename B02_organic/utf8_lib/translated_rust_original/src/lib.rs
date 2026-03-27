use std::ffi::c_char;

const REPLACEMENT_INC: usize = 4096;

// Helper: read byte at offset from a *const c_char pointer
#[inline(always)]
unsafe fn b(p: *const c_char, off: isize) -> u8 {
    *p.offset(off) as u8
}

#[inline(always)]
unsafe fn valid_1(x: *const c_char) -> bool {
    (b(x, 0) & 0x80) == 0
}

#[inline(always)]
unsafe fn valid_2(x: *const c_char) -> bool {
    (b(x, 0) & 0xE0) == 0xC0
        && b(x, 0) >= 0xC2
        && (b(x, 1) & 0xC0) == 0x80
}

#[inline(always)]
unsafe fn valid_3(x: *const c_char) -> bool {
    (b(x, 0) & 0xF0) == 0xE0
        && (b(x, 1) & 0xC0) == 0x80
        && (b(x, 2) & 0xC0) == 0x80
        && (b(x, 0) != 0xE0 || b(x, 1) >= 0xA0)
        && (b(x, 0) != 0xED || b(x, 1) < 0xA0)
        && (b(x, 0) != 0xEF || b(x, 1) <= 0xBF)
}

#[inline(always)]
unsafe fn valid_4(x: *const c_char) -> bool {
    (b(x, 0) & 0xF8) == 0xF0
        && b(x, 0) <= 0xF4
        && (b(x, 1) & 0xC0) == 0x80
        && (b(x, 2) & 0xC0) == 0x80
        && (b(x, 3) & 0xC0) == 0x80
        && (b(x, 0) != 0xF0 || b(x, 1) >= 0x90)
        && (b(x, 0) != 0xF4 || b(x, 1) <= 0x8F)
}

/// Return pointer to the first character that does not match UTF-8, or the last byte (0)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_drop(string: *const c_char) -> *const c_char {
    assert!(!string.is_null());

    let mut s = string;

    while *s != 0 {
        if valid_1(s) {
            s = s.offset(1);
        } else if valid_2(s) {
            s = s.offset(2);
        } else if valid_3(s) {
            s = s.offset(3);
        } else if valid_4(s) {
            s = s.offset(4);
        } else {
            return s;
        }
    }

    s
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_filter(
    string: *const c_char,
    replacement: bool,
) -> *mut c_char {
    assert!(!string.is_null());

    let valid = w_utf8_drop(string);

    if *valid == 0 {
        return libc::strdup(string);
    }

    let size = libc::strlen(string) + 1;
    let mut size = size;
    let mut i = (valid as usize) - (string as usize);

    let copy = libc::malloc(size) as *mut c_char;
    if copy.is_null() {
        return std::ptr::null_mut();
    }
    libc::memcpy(copy as *mut _, string as *const _, i);

    let mut valid = valid;
    let mut copy = copy;
    let mut repl: usize = 0;

    while *valid != 0 {
        if valid_1(valid) {
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.offset(1);
        } else if valid_2(valid) {
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.offset(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.offset(1);
        } else if valid_3(valid) {
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.offset(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.offset(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.offset(1);
        } else if valid_4(valid) {
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.offset(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.offset(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.offset(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.offset(1);
        } else {
            if replacement {
                if repl < 3 {
                    size += REPLACEMENT_INC;
                    let new_copy = libc::realloc(copy as *mut _, size) as *mut c_char;
                    if new_copy.is_null() {
                        return std::ptr::null_mut();
                    }
                    copy = new_copy;
                    repl += REPLACEMENT_INC;
                }

                *copy.add(i) = 0xEFu8 as c_char;
                i += 1;
                *copy.add(i) = 0xBFu8 as c_char;
                i += 1;
                *copy.add(i) = 0xBDu8 as c_char;
                i += 1;
                repl -= 3;
            }

            valid = valid.offset(1);
        }
    }

    *copy.add(i) = 0;
    copy
}
