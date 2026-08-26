// Translated from pcre2_maketables.c
use crate::internal::*;
use core::ffi::{c_int, c_void};

extern "C" {
    fn tolower(c: c_int) -> c_int;
    fn toupper(c: c_int) -> c_int;
    fn isdigit(c: c_int) -> c_int;
    fn isupper(c: c_int) -> c_int;
    fn islower(c: c_int) -> c_int;
    fn isalnum(c: c_int) -> c_int;
    fn isalpha(c: c_int) -> c_int;
    fn isspace(c: c_int) -> c_int;
    fn isxdigit(c: c_int) -> c_int;
    fn isgraph(c: c_int) -> c_int;
    fn isprint(c: c_int) -> c_int;
    fn ispunct(c: c_int) -> c_int;
    fn iscntrl(c: c_int) -> c_int;
}

/*************************************************
*           Create PCRE2 character tables        *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_maketables_8(
    gcontext: *mut pcre2_real_general_context,
) -> *const u8 {
    let yield_: *mut u8 = (if !gcontext.is_null() {
        ((*gcontext).memctl.malloc.unwrap())(TABLES_LENGTH, (*gcontext).memctl.memory_data)
    } else {
        malloc(TABLES_LENGTH)
    }) as *mut u8;

    let mut i: c_int;
    let mut p: *mut u8;

    if yield_.is_null() {
        return core::ptr::null();
    }
    p = yield_;

    /* First comes the lower casing table */

    i = 0;
    while i < 256 {
        let c: c_int = tolower(i);
        *p = (if c < 256 { c } else { i }) as u8;
        p = p.add(1);
        i += 1;
    }

    /* Next the case-flipping table */

    i = 0;
    while i < 256 {
        let c: c_int = if islower(i) != 0 { toupper(i) } else { tolower(i) };
        *p = (if c < 256 { c } else { i }) as u8;
        p = p.add(1);
        i += 1;
    }

    /* Then the character class tables. */

    memset(p as *mut c_void, 0, cbit_length);
    i = 0;
    while i < 256 {
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
        i += 1;
    }
    p = p.add(cbit_length);

    /* Finally, the character type table. */

    i = 0;
    while i < 256 {
        let mut x: c_int = 0;
        if isspace(i) != 0 {
            x += ctype_space as c_int;
        }
        if isalpha(i) != 0 {
            x += ctype_letter as c_int;
        }
        if islower(i) != 0 {
            x += ctype_lcletter as c_int;
        }
        if isdigit(i) != 0 {
            x += ctype_digit as c_int;
        }
        if isalnum(i) != 0 || i as u32 == CHAR_UNDERSCORE {
            x += ctype_word as c_int;
        }
        *p = x as u8;
        p = p.add(1);
        i += 1;
    }

    yield_ as *const u8
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_maketables_free_8(
    gcontext: *mut pcre2_real_general_context,
    tables: *const u8,
) {
    if !gcontext.is_null() {
        ((*gcontext).memctl.free.unwrap())(tables as *mut c_void, (*gcontext).memctl.memory_data);
    } else {
        free(tables as *mut c_void);
    }
}
