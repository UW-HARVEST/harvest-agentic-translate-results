use std::ffi::c_char;

const REPLACEMENT_INC: usize = 4096;

extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn realloc(ptr: *mut u8, size: usize) -> *mut u8;
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> usize;
}

#[inline]
unsafe fn byte_at(p: *const c_char, off: usize) -> u8 {
    *p.add(off) as u8
}

/* Single byte: 0xxxxxxx */
#[inline]
unsafe fn valid_1(x: *const c_char) -> bool {
    (byte_at(x, 0) & 0x80) == 0
}

/* Two bytes: 110xxxxx 10xxxxxx */
/* Starting bytes 0xC0 and 0xC1 are forbidden (overlong) */
#[inline]
unsafe fn valid_2(x: *const c_char) -> bool {
    (byte_at(x, 0) & 0xE0) == 0xC0
        && byte_at(x, 0) >= 0xC2
        && (byte_at(x, 1) & 0xC0) == 0x80
}

/* Three bytes: 1110xxxx 10xxxxxx 10xxxxxx */
/* 0xE0 could start overlong encodings */
/* 0xED (range U+D800-U+DFFF) is reserved for UTF-16 surrogate halves */
#[inline]
unsafe fn valid_3(x: *const c_char) -> bool {
    (byte_at(x, 0) & 0xF0) == 0xE0
        && (byte_at(x, 1) & 0xC0) == 0x80
        && (byte_at(x, 2) & 0xC0) == 0x80
        && (byte_at(x, 0) != 0xE0 || byte_at(x, 1) >= 0xA0)
        && (byte_at(x, 0) != 0xED || byte_at(x, 1) < 0xA0)
        && (byte_at(x, 0) != 0xEF || byte_at(x, 1) <= 0xBF)
}

/* Four bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx */
/* 0xF0 could start overlong encodings */
/* Start bytes 0xF5 and above are invalid for UTF-8 */
#[inline]
unsafe fn valid_4(x: *const c_char) -> bool {
    (byte_at(x, 0) & 0xF8) == 0xF0
        && byte_at(x, 0) <= 0xF4
        && (byte_at(x, 1) & 0xC0) == 0x80
        && (byte_at(x, 2) & 0xC0) == 0x80
        && (byte_at(x, 3) & 0xC0) == 0x80
        && (byte_at(x, 0) != 0xF0 || byte_at(x, 1) >= 0x90)
        && (byte_at(x, 0) != 0xF4 || byte_at(x, 1) <= 0x8F)
}

/* Return pointer to the first character that does not match UTF-8, or the last byte (0) */
unsafe fn w_utf8_drop(mut string: *const c_char) -> *const c_char {
    // assert(string != NULL); -- assert is a no-op in C release builds (NDEBUG)

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
    // assert(string != NULL); -- assert is a no-op in C release builds (NDEBUG)

    let mut valid: *const c_char = w_utf8_drop(string);

    if *valid == 0 {
        return strdup(string);
    }

    let mut size: usize = strlen(string) + 1;
    let mut i: usize = valid.offset_from(string) as usize;
    let mut repl: usize = 0;

    let mut copy: *mut u8 = malloc(size);
    if copy.is_null() {
        return std::ptr::null_mut();
    }
    std::ptr::copy_nonoverlapping(string as *const u8, copy, i);

    while *valid != 0 {
        if valid_1(valid) {
            *copy.add(i) = *valid as u8;
            i += 1;
            valid = valid.add(1);
        } else if valid_2(valid) {
            *copy.add(i) = *valid as u8;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid as u8;
            i += 1;
            valid = valid.add(1);
        } else if valid_3(valid) {
            *copy.add(i) = *valid as u8;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid as u8;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid as u8;
            i += 1;
            valid = valid.add(1);
        } else if valid_4(valid) {
            *copy.add(i) = *valid as u8;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid as u8;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid as u8;
            i += 1;
            valid = valid.add(1);
            *copy.add(i) = *valid as u8;
            i += 1;
            valid = valid.add(1);
        } else {
            if replacement {
                if repl < 3 {
                    size += REPLACEMENT_INC;
                    copy = realloc(copy, size);
                    if copy.is_null() {
                        return std::ptr::null_mut();
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

            valid = valid.add(1);
        }
    }

    *copy.add(i) = 0;
    copy as *mut c_char
}
