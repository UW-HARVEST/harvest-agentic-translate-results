//! Translated from pcre2_maketables.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn tolower(c: i32) -> i32;
    fn toupper(c: i32) -> i32;
    fn isspace(c: i32) -> i32;
    fn isupper(c: i32) -> i32;
    fn islower(c: i32) -> i32;
    fn isalpha(c: i32) -> i32;
    fn isalnum(c: i32) -> i32;
    fn isdigit(c: i32) -> i32;
    fn isxdigit(c: i32) -> i32;
    fn isgraph(c: i32) -> i32;
    fn isprint(c: i32) -> i32;
    fn ispunct(c: i32) -> i32;
    fn iscntrl(c: i32) -> i32;
}

/* charfn_to(c) and charfn_from(c) are identity macros when compiling the
library (i.e. when PCRE2_DFTABLES is not defined). */

/*************************************************
*           Create PCRE2 character tables        *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_maketables_8(gcontext: *mut pcre2_real_general_context) -> *const u8 {
    let yield_: *mut u8 = (if !gcontext.is_null() {
        ((*gcontext).memctl.malloc.unwrap())(TABLES_LENGTH, (*gcontext).memctl.memory_data)
    } else {
        malloc(TABLES_LENGTH)
    }) as *mut u8;

    let mut i: i32;
    let mut p: *mut u8;

    if yield_.is_null() {
        return core::ptr::null();
    }
    p = yield_;

    /* First comes the lower casing table */

    i = 0;
    while i < 256 {
        let c: i32 = tolower(i);
        *p = (if c < 256 { c } else { i }) as u8;
        p = p.add(1);
        i += 1;
    }

    /* Next the case-flipping table */

    i = 0;
    while i < 256 {
        let c: i32 = if islower(i) != 0 { toupper(i) } else { tolower(i) };
        *p = (if c < 256 { c } else { i }) as u8;
        p = p.add(1);
        i += 1;
    }

    /* Then the character class tables. Don't try to be clever and save effort on
    exclusive ones - in some locales things may be different. */

    core::ptr::write_bytes(p, 0u8, cbit_length);
    i = 0;
    while i < 256 {
        if isdigit(i) != 0 {
            *p.add(cbit_digit + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if isupper(i) != 0 {
            *p.add(cbit_upper + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if islower(i) != 0 {
            *p.add(cbit_lower + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if isalnum(i) != 0 {
            *p.add(cbit_word + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if i == b'_' as i32
        /* CHAR_UNDERSCORE */
        {
            *p.add(cbit_word + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if isspace(i) != 0 {
            *p.add(cbit_space + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if isxdigit(i) != 0 {
            *p.add(cbit_xdigit + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if isgraph(i) != 0 {
            *p.add(cbit_graph + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if isprint(i) != 0 {
            *p.add(cbit_print + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if ispunct(i) != 0 {
            *p.add(cbit_punct + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if iscntrl(i) != 0 {
            *p.add(cbit_cntrl + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        i += 1;
    }
    p = p.add(cbit_length);

    /* Finally, the character type table. */

    i = 0;
    while i < 256 {
        let mut x: i32 = 0;
        if isspace(i) != 0 {
            x += ctype_space as i32;
        }
        if isalpha(i) != 0 {
            x += ctype_letter as i32;
        }
        if islower(i) != 0 {
            x += ctype_lcletter as i32;
        }
        if isdigit(i) != 0 {
            x += ctype_digit as i32;
        }
        if isalnum(i) != 0 || i == b'_' as i32
        /* CHAR_UNDERSCORE */
        {
            x += ctype_word as i32;
        }
        *p = x as u8;
        p = p.add(1);
        i += 1;
    }

    yield_ as *const u8
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_maketables_free_8(gcontext: *mut pcre2_real_general_context, tables: *const u8) {
    if !gcontext.is_null() {
        ((*gcontext).memctl.free.unwrap())(tables as *mut c_void, (*gcontext).memctl.memory_data);
    } else {
        free(tables as *mut c_void);
    }
}
