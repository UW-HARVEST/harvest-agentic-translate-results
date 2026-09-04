use std::ffi::{c_char, c_void};
use std::ptr;

const REPLACEMENT_INC: usize = 4096;

unsafe extern "C" {
    fn abort() -> !;
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(pointer: *mut c_void, size: usize) -> *mut c_void;
    fn strdup(string: *const c_char) -> *mut c_char;
    fn strlen(string: *const c_char) -> usize;
}

#[inline]
unsafe fn valid_1(string: *const u8) -> bool {
    unsafe { *string & 0x80 == 0 }
}

#[inline]
unsafe fn valid_2(string: *const u8) -> bool {
    unsafe { *string & 0xe0 == 0xc0 && *string >= 0xc2 && *string.add(1) & 0xc0 == 0x80 }
}

#[inline]
unsafe fn valid_3(string: *const u8) -> bool {
    unsafe {
        *string & 0xf0 == 0xe0
            && *string.add(1) & 0xc0 == 0x80
            && *string.add(2) & 0xc0 == 0x80
            && (*string != 0xe0 || *string.add(1) >= 0xa0)
            && (*string != 0xed || *string.add(1) < 0xa0)
            && (*string != 0xef || *string.add(1) <= 0xbf)
    }
}

#[inline]
unsafe fn valid_4(string: *const u8) -> bool {
    unsafe {
        *string & 0xf8 == 0xf0
            && *string <= 0xf4
            && *string.add(1) & 0xc0 == 0x80
            && *string.add(2) & 0xc0 == 0x80
            && *string.add(3) & 0xc0 == 0x80
            && (*string != 0xf0 || *string.add(1) >= 0x90)
            && (*string != 0xf4 || *string.add(1) <= 0x8f)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_drop(string: *const c_char) -> *const c_char {
    if string.is_null() {
        unsafe { abort() };
    }

    let mut current = string.cast::<u8>();

    while unsafe { *current } != 0 {
        if unsafe { valid_1(current) } {
            current = unsafe { current.add(1) };
        } else if unsafe { valid_2(current) } {
            current = unsafe { current.add(2) };
        } else if unsafe { valid_3(current) } {
            current = unsafe { current.add(3) };
        } else if unsafe { valid_4(current) } {
            current = unsafe { current.add(4) };
        } else {
            return current.cast::<c_char>();
        }
    }

    current.cast::<c_char>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn w_utf8_filter(string: *const c_char, replacement: bool) -> *mut c_char {
    if string.is_null() {
        unsafe { abort() };
    }

    let mut valid = unsafe { w_utf8_drop(string) }.cast::<u8>();

    if unsafe { *valid } == 0 {
        return unsafe { strdup(string) };
    }

    let mut size = unsafe { strlen(string) } + 1;
    let mut i = unsafe { valid.offset_from(string.cast::<u8>()) } as usize;
    let mut repl = 0usize;
    let mut copy = unsafe { malloc(size) }.cast::<u8>();

    if copy.is_null() {
        return ptr::null_mut();
    }
    unsafe { ptr::copy_nonoverlapping(string.cast::<u8>(), copy, i) };

    while unsafe { *valid } != 0 {
        if unsafe { valid_1(valid) } {
            unsafe {
                *copy.add(i) = *valid;
                i += 1;
                valid = valid.add(1);
            }
        } else if unsafe { valid_2(valid) } {
            unsafe {
                ptr::copy_nonoverlapping(valid, copy.add(i), 2);
                i += 2;
                valid = valid.add(2);
            }
        } else if unsafe { valid_3(valid) } {
            unsafe {
                ptr::copy_nonoverlapping(valid, copy.add(i), 3);
                i += 3;
                valid = valid.add(3);
            }
        } else if unsafe { valid_4(valid) } {
            unsafe {
                ptr::copy_nonoverlapping(valid, copy.add(i), 4);
                i += 4;
                valid = valid.add(4);
            }
        } else {
            if replacement {
                if repl < 3 {
                    size += REPLACEMENT_INC;
                    copy = unsafe { realloc(copy.cast::<c_void>(), size) }.cast::<u8>();
                    if copy.is_null() {
                        return ptr::null_mut();
                    }
                    repl += REPLACEMENT_INC;
                }

                unsafe {
                    *copy.add(i) = 0xef;
                    *copy.add(i + 1) = 0xbf;
                    *copy.add(i + 2) = 0xbd;
                }
                i += 3;
                repl -= 3;
            }

            valid = unsafe { valid.add(1) };
        }
    }

    unsafe { *copy.add(i) = 0 };
    copy.cast::<c_char>()
}
