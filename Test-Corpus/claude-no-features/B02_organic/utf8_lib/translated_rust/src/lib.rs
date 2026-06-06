use std::ffi::c_char;
use std::os::raw::c_void;
use std::ptr;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

const REPLACEMENT_INC: usize = 4096;

/* Single byte: 0xxxxxxx */
#[inline]
unsafe fn valid_1(x: *const u8) -> bool {
    (*x & 0x80) == 0
}

/* Two bytes: 110xxxxx 10xxxxxx */
/* Starting bytes 0xC0 and 0xC1 are forbidden (overlong) */
#[inline]
unsafe fn valid_2(x: *const u8) -> bool {
    (*x & 0xE0) == 0xC0 && *x >= 0xC2 && (*x.add(1) & 0xC0) == 0x80
}

/* Three bytes: 1110xxxx 10xxxxxx 10xxxxxx */
#[inline]
unsafe fn valid_3(x: *const u8) -> bool {
    (*x & 0xF0) == 0xE0
        && (*x.add(1) & 0xC0) == 0x80
        && (*x.add(2) & 0xC0) == 0x80
        && (*x != 0xE0 || *x.add(1) >= 0xA0)
        && (*x != 0xED || *x.add(1) < 0xA0)
        && (*x != 0xEF || *x.add(1) <= 0xBF)
}

/* Four bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx */
#[inline]
unsafe fn valid_4(x: *const u8) -> bool {
    (*x & 0xF8) == 0xF0
        && *x <= 0xF4
        && (*x.add(1) & 0xC0) == 0x80
        && (*x.add(2) & 0xC0) == 0x80
        && (*x.add(3) & 0xC0) == 0x80
        && (*x != 0xF0 || *x.add(1) >= 0x90)
        && (*x != 0xF4 || *x.add(1) <= 0x8F)
}

/* Return pointer to the first character that does not match UTF-8, or the last byte (0) */
unsafe fn w_utf8_drop(mut string: *const u8) -> *const u8 {
    assert!(!string.is_null());

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
pub unsafe extern "C" fn w_utf8_filter(string: *const c_char, replacement: bool) -> *mut c_char {
    assert!(!string.is_null());

    let string_u8 = string as *const u8;
    let valid = w_utf8_drop(string_u8);

    if *valid == 0 {
        let copy = strdup(string);
        return copy;
    }

    let mut size = strlen(string) + 1;
    let mut i = valid.offset_from(string_u8) as usize;
    let mut repl: usize = 0;

    let mut copy = malloc(size) as *mut u8;
    if copy.is_null() {
        return ptr::null_mut();
    }
    memcpy(copy as *mut c_void, string as *const c_void, i);

    let mut valid_ptr = valid;
    while *valid_ptr != 0 {
        if valid_1(valid_ptr) {
            *copy.add(i) = *valid_ptr;
            i += 1;
            valid_ptr = valid_ptr.add(1);
        } else if valid_2(valid_ptr) {
            *copy.add(i) = *valid_ptr;
            i += 1;
            valid_ptr = valid_ptr.add(1);
            *copy.add(i) = *valid_ptr;
            i += 1;
            valid_ptr = valid_ptr.add(1);
        } else if valid_3(valid_ptr) {
            *copy.add(i) = *valid_ptr;
            i += 1;
            valid_ptr = valid_ptr.add(1);
            *copy.add(i) = *valid_ptr;
            i += 1;
            valid_ptr = valid_ptr.add(1);
            *copy.add(i) = *valid_ptr;
            i += 1;
            valid_ptr = valid_ptr.add(1);
        } else if valid_4(valid_ptr) {
            *copy.add(i) = *valid_ptr;
            i += 1;
            valid_ptr = valid_ptr.add(1);
            *copy.add(i) = *valid_ptr;
            i += 1;
            valid_ptr = valid_ptr.add(1);
            *copy.add(i) = *valid_ptr;
            i += 1;
            valid_ptr = valid_ptr.add(1);
            *copy.add(i) = *valid_ptr;
            i += 1;
            valid_ptr = valid_ptr.add(1);
        } else {
            if replacement {
                if repl < 3 {
                    size += REPLACEMENT_INC;
                    copy = realloc(copy as *mut c_void, size) as *mut u8;
                    if copy.is_null() {
                        return ptr::null_mut();
                    }
                    repl += REPLACEMENT_INC;
                }

                *copy.add(i) = 0xEF;
                i += 1;
                *copy.add(i) = 0xBF;
                i += 1;
                *copy.add(i) = 0xBD;
                i += 1;
                repl -= 3;
            }

            valid_ptr = valid_ptr.add(1);
        }
    }

    *copy.add(i) = 0;
    copy as *mut c_char
}
