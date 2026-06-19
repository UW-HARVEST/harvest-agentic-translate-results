use std::ffi::{c_char, c_void, CStr};
use std::ptr;

const REPLACEMENT_INC: usize = 4096;

#[inline]
fn valid_1(bytes: &[u8], idx: usize) -> bool {
    (bytes[idx] & 0x80) == 0
}

#[inline]
fn valid_2(bytes: &[u8], idx: usize) -> bool {
    ((bytes[idx] & 0xE0) == 0xC0)
        && ((bytes[idx] as c_char) >= (0xC2_u8 as c_char))
        && ((bytes.get(idx + 1).copied().unwrap_or(0) & 0xC0) == 0x80)
}

#[inline]
fn valid_3(bytes: &[u8], idx: usize) -> bool {
    ((bytes[idx] & 0xF0) == 0xE0)
        && ((bytes.get(idx + 1).copied().unwrap_or(0) & 0xC0) == 0x80)
        && ((bytes.get(idx + 2).copied().unwrap_or(0) & 0xC0) == 0x80)
        && ((bytes[idx] != 0xE0) || bytes.get(idx + 1).copied().unwrap_or(0) >= 0xA0)
        && ((bytes[idx] != 0xED) || bytes.get(idx + 1).copied().unwrap_or(0) < 0xA0)
        && ((bytes[idx] != 0xEF) || bytes.get(idx + 1).copied().unwrap_or(0) <= 0xBF)
}

#[inline]
fn valid_4(bytes: &[u8], idx: usize) -> bool {
    ((bytes[idx] & 0xF8) == 0xF0)
        && (bytes[idx] <= 0xF4)
        && ((bytes.get(idx + 1).copied().unwrap_or(0) & 0xC0) == 0x80)
        && ((bytes.get(idx + 2).copied().unwrap_or(0) & 0xC0) == 0x80)
        && ((bytes.get(idx + 3).copied().unwrap_or(0) & 0xC0) == 0x80)
        && ((bytes[idx] != 0xF0) || bytes.get(idx + 1).copied().unwrap_or(0) >= 0x90)
        && ((bytes[idx] != 0xF4) || bytes.get(idx + 1).copied().unwrap_or(0) <= 0x8F)
}

#[inline]
unsafe fn assert_not_null<T>(ptr: *const T) {
    if ptr.is_null() {
        libc::abort();
    }
}

#[inline]
unsafe fn bytes_with_nul<'a>(string: *const c_char) -> &'a [u8] {
    CStr::from_ptr(string).to_bytes_with_nul()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_drop(string: *const c_char) -> *const c_char {
    assert_not_null(string);

    let bytes = bytes_with_nul(string);
    let mut idx = 0usize;

    while bytes[idx] != 0 {
        if valid_1(bytes, idx) {
            idx += 1;
        } else if valid_2(bytes, idx) {
            idx += 2;
        } else if valid_3(bytes, idx) {
            idx += 3;
        } else if valid_4(bytes, idx) {
            idx += 4;
        } else {
            return string.add(idx);
        }
    }

    string.add(idx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_filter(string: *const c_char, replacement: bool) -> *mut c_char {
    assert_not_null(string);

    let valid = w_utf8_drop(string);

    if *valid == 0 {
        return libc::strdup(string);
    }

    let bytes = bytes_with_nul(string);
    let mut size = libc::strlen(string) + 1;
    let mut i = valid.offset_from(string) as usize;
    let mut repl = 0usize;
    let mut valid_idx = i;

    let mut copy = libc::malloc(size).cast::<c_char>();
    if copy.is_null() {
        return ptr::null_mut();
    }

    libc::memcpy(copy.cast::<c_void>(), string.cast::<c_void>(), i);

    while bytes[valid_idx] != 0 {
        if valid_1(bytes, valid_idx) {
            *copy.add(i) = *string.add(valid_idx);
            i += 1;
            valid_idx += 1;
        } else if valid_2(bytes, valid_idx) {
            *copy.add(i) = *string.add(valid_idx);
            i += 1;
            valid_idx += 1;
            *copy.add(i) = *string.add(valid_idx);
            i += 1;
            valid_idx += 1;
        } else if valid_3(bytes, valid_idx) {
            *copy.add(i) = *string.add(valid_idx);
            i += 1;
            valid_idx += 1;
            *copy.add(i) = *string.add(valid_idx);
            i += 1;
            valid_idx += 1;
            *copy.add(i) = *string.add(valid_idx);
            i += 1;
            valid_idx += 1;
        } else if valid_4(bytes, valid_idx) {
            *copy.add(i) = *string.add(valid_idx);
            i += 1;
            valid_idx += 1;
            *copy.add(i) = *string.add(valid_idx);
            i += 1;
            valid_idx += 1;
            *copy.add(i) = *string.add(valid_idx);
            i += 1;
            valid_idx += 1;
            *copy.add(i) = *string.add(valid_idx);
            i += 1;
            valid_idx += 1;
        } else {
            if replacement {
                if repl < 3 {
                    size += REPLACEMENT_INC;
                    copy = libc::realloc(copy.cast::<c_void>(), size).cast::<c_char>();
                    if copy.is_null() {
                        return ptr::null_mut();
                    }
                    repl += REPLACEMENT_INC;
                }

                *copy.add(i) = 0xEF_u8 as c_char;
                i += 1;
                *copy.add(i) = 0xBF_u8 as c_char;
                i += 1;
                *copy.add(i) = 0xBD_u8 as c_char;
                i += 1;
                repl -= 3;
            }

            valid_idx += 1;
        }
    }

    *copy.add(i) = 0;
    copy
}
