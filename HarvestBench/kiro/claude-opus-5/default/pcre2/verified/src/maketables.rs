//! Translation of `pcre2_maketables.c`.
//!
//! This module contains the external function `pcre2_maketables()`, which
//! builds character tables for PCRE2 in the current locale.

use crate::internal::*;
use core::ffi::{c_int, c_void};

unsafe extern "C" {
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

// When compiling the library, charfn_to / charfn_from are identity.
#[inline(always)]
fn charfn_to(c: c_int) -> c_int {
    c
}
#[inline(always)]
fn charfn_from(c: c_int) -> c_int {
    c
}

/// `pcre2_maketables()` — build a set of character tables for use by PCRE2 and
/// return a pointer to them. Their contents depend on the current locale.
///
/// Arguments:   a PCRE2 general context or NULL
/// Returns:     pointer to the contiguous block of data;
///                else NULL if memory allocation failed
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_maketables_8(
    gcontext: *mut pcre2_real_general_context,
) -> *const u8 {
    unsafe {
        let yield_: *mut u8 = if !gcontext.is_null() {
            let memctl = &(*gcontext).memctl;
            (memctl.malloc.unwrap())(TABLES_LENGTH as usize, memctl.memory_data) as *mut u8
        } else {
            malloc(TABLES_LENGTH as usize) as *mut u8
        };

        if yield_.is_null() {
            return core::ptr::null();
        }
        let mut p = yield_;

        // First comes the lower casing table.
        for i in 0..256i32 {
            let c = charfn_from(tolower(charfn_to(i)));
            *p = if c < 256 { c as u8 } else { i as u8 };
            p = p.add(1);
        }

        // Next the case-flipping table.
        for i in 0..256i32 {
            let c = charfn_from(if islower(charfn_to(i)) != 0 {
                toupper(charfn_to(i))
            } else {
                tolower(charfn_to(i))
            });
            *p = if c < 256 { c as u8 } else { i as u8 };
            p = p.add(1);
        }

        // Then the character class tables.
        core::ptr::write_bytes(p, 0, cbit_length as usize);
        for i in 0..256i32 {
            if isdigit(charfn_to(i)) != 0 {
                *p.add(cbit_digit as usize + (i as usize) / 8) |= 1u8 << (i & 7);
            }
            if isupper(charfn_to(i)) != 0 {
                *p.add(cbit_upper as usize + (i as usize) / 8) |= 1u8 << (i & 7);
            }
            if islower(charfn_to(i)) != 0 {
                *p.add(cbit_lower as usize + (i as usize) / 8) |= 1u8 << (i & 7);
            }
            if isalnum(charfn_to(i)) != 0 {
                *p.add(cbit_word as usize + (i as usize) / 8) |= 1u8 << (i & 7);
            }
            if i == CHAR_UNDERSCORE {
                *p.add(cbit_word as usize + (i as usize) / 8) |= 1u8 << (i & 7);
            }
            if isspace(charfn_to(i)) != 0 {
                *p.add(cbit_space as usize + (i as usize) / 8) |= 1u8 << (i & 7);
            }
            if isxdigit(charfn_to(i)) != 0 {
                *p.add(cbit_xdigit as usize + (i as usize) / 8) |= 1u8 << (i & 7);
            }
            if isgraph(charfn_to(i)) != 0 {
                *p.add(cbit_graph as usize + (i as usize) / 8) |= 1u8 << (i & 7);
            }
            if isprint(charfn_to(i)) != 0 {
                *p.add(cbit_print as usize + (i as usize) / 8) |= 1u8 << (i & 7);
            }
            if ispunct(charfn_to(i)) != 0 {
                *p.add(cbit_punct as usize + (i as usize) / 8) |= 1u8 << (i & 7);
            }
            if iscntrl(charfn_to(i)) != 0 {
                *p.add(cbit_cntrl as usize + (i as usize) / 8) |= 1u8 << (i & 7);
            }
        }
        p = p.add(cbit_length as usize);

        // Finally, the character type table.
        for i in 0..256i32 {
            let mut x: i32 = 0;
            if isspace(charfn_to(i)) != 0 {
                x += ctype_space as i32;
            }
            if isalpha(charfn_to(i)) != 0 {
                x += ctype_letter as i32;
            }
            if islower(charfn_to(i)) != 0 {
                x += ctype_lcletter as i32;
            }
            if isdigit(charfn_to(i)) != 0 {
                x += ctype_digit as i32;
            }
            if isalnum(charfn_to(i)) != 0 || i == CHAR_UNDERSCORE {
                x += ctype_word as i32;
            }
            *p = x as u8;
            p = p.add(1);
        }

        yield_ as *const u8
    }
}

/// `pcre2_maketables_free()` — free tables created by `pcre2_maketables()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_maketables_free_8(
    gcontext: *mut pcre2_real_general_context,
    tables: *const u8,
) {
    unsafe {
        if !gcontext.is_null() {
            let memctl = &(*gcontext).memctl;
            (memctl.free.unwrap())(tables as *mut c_void, memctl.memory_data);
        } else {
            free(tables as *mut c_void);
        }
    }
}

// CHAR_UNDERSCORE is '_' (0x5F) in the ASCII/non-EBCDIC configuration.
const CHAR_UNDERSCORE: c_int = b'_' as c_int;
