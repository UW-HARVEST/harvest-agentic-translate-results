use crate::pcre2_internal::*;
use core::ffi::{c_int, c_void};
use core::ptr;

extern "C" {
    fn tolower(c: c_int) -> c_int;
    fn toupper(c: c_int) -> c_int;
    fn islower(c: c_int) -> c_int;
    fn isupper(c: c_int) -> c_int;
    fn isdigit(c: c_int) -> c_int;
    fn isalnum(c: c_int) -> c_int;
    fn isalpha(c: c_int) -> c_int;
    fn isspace(c: c_int) -> c_int;
    fn isxdigit(c: c_int) -> c_int;
    fn isgraph(c: c_int) -> c_int;
    fn isprint(c: c_int) -> c_int;
    fn ispunct(c: c_int) -> c_int;
    fn iscntrl(c: c_int) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_maketables_8(gcontext: *mut pcre2_general_context) -> *const u8 {
    let yield_: *mut u8 = if !gcontext.is_null() {
        ((*gcontext).memctl.malloc.unwrap())(TABLES_LENGTH, (*gcontext).memctl.memory_data) as *mut u8
    } else {
        malloc(TABLES_LENGTH) as *mut u8
    };
    if yield_.is_null() {
        return ptr::null_mut();
    }
    let mut p = yield_;

    // Lower casing table
    for i in 0..256i32 {
        let c = tolower(i);
        *p = if c < 256 { c as u8 } else { i as u8 };
        p = p.add(1);
    }

    // Case-flipping table
    for i in 0..256i32 {
        let c = if islower(i) != 0 { toupper(i) } else { tolower(i) };
        *p = if c < 256 { c as u8 } else { i as u8 };
        p = p.add(1);
    }

    // Character class tables
    memset(p as *mut c_void, 0, cbit_length);
    for i in 0..256i32 {
        let iu = i as usize;
        if isdigit(i) != 0 {
            *p.add(cbit_digit + iu / 8) |= 1u8 << (i & 7);
        }
        if isupper(i) != 0 {
            *p.add(cbit_upper + iu / 8) |= 1u8 << (i & 7);
        }
        if islower(i) != 0 {
            *p.add(cbit_lower + iu / 8) |= 1u8 << (i & 7);
        }
        if isalnum(i) != 0 {
            *p.add(cbit_word + iu / 8) |= 1u8 << (i & 7);
        }
        if i as u32 == CHAR_UNDERSCORE {
            *p.add(cbit_word + iu / 8) |= 1u8 << (i & 7);
        }
        if isspace(i) != 0 {
            *p.add(cbit_space + iu / 8) |= 1u8 << (i & 7);
        }
        if isxdigit(i) != 0 {
            *p.add(cbit_xdigit + iu / 8) |= 1u8 << (i & 7);
        }
        if isgraph(i) != 0 {
            *p.add(cbit_graph + iu / 8) |= 1u8 << (i & 7);
        }
        if isprint(i) != 0 {
            *p.add(cbit_print + iu / 8) |= 1u8 << (i & 7);
        }
        if ispunct(i) != 0 {
            *p.add(cbit_punct + iu / 8) |= 1u8 << (i & 7);
        }
        if iscntrl(i) != 0 {
            *p.add(cbit_cntrl + iu / 8) |= 1u8 << (i & 7);
        }
    }
    p = p.add(cbit_length);

    // Character type table
    for i in 0..256i32 {
        let mut x: u8 = 0;
        if isspace(i) != 0 {
            x += ctype_space;
        }
        if isalpha(i) != 0 {
            x += ctype_letter;
        }
        if islower(i) != 0 {
            x += ctype_lcletter;
        }
        if isdigit(i) != 0 {
            x += ctype_digit;
        }
        if isalnum(i) != 0 || i as u32 == CHAR_UNDERSCORE {
            x += ctype_word;
        }
        *p = x;
        p = p.add(1);
    }

    yield_ as *const u8
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_maketables_free_8(
    gcontext: *mut pcre2_general_context,
    tables: *const u8,
) {
    if !gcontext.is_null() {
        ((*gcontext).memctl.free.unwrap())(tables as *mut c_void, (*gcontext).memctl.memory_data);
    } else {
        free(tables as *mut c_void);
    }
}
