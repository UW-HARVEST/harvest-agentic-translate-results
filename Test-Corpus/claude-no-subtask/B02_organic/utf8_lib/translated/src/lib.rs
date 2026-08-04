use std::ffi::c_char;
use std::os::raw::c_int;

const REPLACEMENT_INC: usize = 4096;

// libc bindings we use
extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn realloc(ptr: *mut u8, size: usize) -> *mut u8;
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
    fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8;
    fn __assert_fail(
        assertion: *const c_char,
        file: *const c_char,
        line: c_int,
        function: *const c_char,
    ) -> !;
}

/// Single byte: 0xxxxxxx
#[inline]
unsafe fn valid_1(x: *const c_char) -> bool {
    let b0 = *x as u8;
    (b0 & 0x80) == 0
}

/// Two bytes: 110xxxxx 10xxxxxx
/// Starting bytes 0xC0 and 0xC1 are forbidden (overlong)
#[inline]
unsafe fn valid_2(x: *const c_char) -> bool {
    let b0 = *x as u8;
    let b1 = *x.add(1) as u8;
    (b0 & 0xE0) == 0xC0 && b0 >= 0xC2 && (b1 & 0xC0) == 0x80
}

/// Three bytes: 1110xxxx 10xxxxxx 10xxxxxx
#[inline]
unsafe fn valid_3(x: *const c_char) -> bool {
    let b0 = *x as u8;
    let b1 = *x.add(1) as u8;
    let b2 = *x.add(2) as u8;
    (b0 & 0xF0) == 0xE0
        && (b1 & 0xC0) == 0x80
        && (b2 & 0xC0) == 0x80
        && (b0 != 0xE0 || b1 >= 0xA0)
        && (b0 != 0xED || b1 < 0xA0)
        && (b0 != 0xEF || b1 <= 0xBF)
}

/// Four bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
#[inline]
unsafe fn valid_4(x: *const c_char) -> bool {
    let b0 = *x as u8;
    let b1 = *x.add(1) as u8;
    let b2 = *x.add(2) as u8;
    let b3 = *x.add(3) as u8;
    (b0 & 0xF8) == 0xF0
        && b0 <= 0xF4
        && (b1 & 0xC0) == 0x80
        && (b2 & 0xC0) == 0x80
        && (b3 & 0xC0) == 0x80
        && (b0 != 0xF0 || b1 >= 0x90)
        && (b0 != 0xF4 || b1 <= 0x8F)
}

/// Return pointer to the first character that does not match UTF-8, or the last byte (0)
unsafe fn w_utf8_drop(mut string: *const c_char) -> *const c_char {
    if string.is_null() {
        // mimic assert(string != NULL)
        let assertion = b"string != NULL\0".as_ptr() as *const c_char;
        let file = b"src/lib.c\0".as_ptr() as *const c_char;
        let func = b"w_utf8_drop\0".as_ptr() as *const c_char;
        __assert_fail(assertion, file, 39, func);
    }

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
    if string.is_null() {
        let assertion = b"string != NULL\0".as_ptr() as *const c_char;
        let file = b"src/lib.c\0".as_ptr() as *const c_char;
        let func = b"w_utf8_filter\0".as_ptr() as *const c_char;
        __assert_fail(assertion, file, 60, func);
    }

    let mut valid = w_utf8_drop(string);

    if *valid == 0 {
        let copy = strdup(string);
        return copy;
    }

    let mut size = strlen(string) + 1;
    let mut i: usize = (valid as usize) - (string as usize);
    let mut repl: usize = 0;

    let mut copy = malloc(size) as *mut c_char;
    if copy.is_null() {
        return std::ptr::null_mut();
    }
    memcpy(copy as *mut u8, string as *const u8, i);

    while *valid != 0 {
        if valid_1(valid) {
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
        } else if valid_2(valid) {
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
        } else if valid_3(valid) {
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
        } else if valid_4(valid) {
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid;
            i += 1;
            valid = valid.add(1);
        } else {
            if replacement {
                if repl < 3 {
                    size += REPLACEMENT_INC;
                    copy = realloc(copy as *mut u8, size) as *mut c_char;
                    if copy.is_null() {
                        return std::ptr::null_mut();
                    }
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

            valid = valid.add(1);
        }
    }

    *copy.add(i) = 0;
    copy
}
