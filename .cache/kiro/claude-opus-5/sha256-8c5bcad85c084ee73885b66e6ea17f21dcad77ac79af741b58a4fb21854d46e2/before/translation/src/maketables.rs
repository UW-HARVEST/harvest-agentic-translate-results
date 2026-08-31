//! Translation of `c_src/src/pcre2_maketables.c`.
//!
//! Contains the external function `pcre2_maketables()`, which builds character
//! tables for PCRE2 in the current locale, and `pcre2_maketables_free()`.

#![allow(non_snake_case)]

use core::ffi::{c_int, c_void};

use crate::chars::*;
use crate::internal::*;

/* Locale-dependent ctype functions from the C library. maketables() builds its
tables using these, so their contents depend on the current locale setting. */
unsafe extern "C" {
    fn tolower(c: c_int) -> c_int;
    fn toupper(c: c_int) -> c_int;
    fn islower(c: c_int) -> c_int;
    fn isdigit(c: c_int) -> c_int;
    fn isupper(c: c_int) -> c_int;
    fn isalnum(c: c_int) -> c_int;
    fn isspace(c: c_int) -> c_int;
    fn isxdigit(c: c_int) -> c_int;
    fn isgraph(c: c_int) -> c_int;
    fn isprint(c: c_int) -> c_int;
    fn ispunct(c: c_int) -> c_int;
    fn iscntrl(c: c_int) -> c_int;
    fn isalpha(c: c_int) -> c_int;
}

/* Create PCRE2 character tables.

This function builds a set of character tables for use by PCRE2 and returns a
pointer to them. They are built using the ctype functions, and consequently
their contents will depend upon the current locale setting. The store is
obtained via a general context malloc, if supplied.

Arguments: a PCRE2 general context or NULL
Returns:   pointer to the contiguous block of data;
             else NULL if memory allocation failed */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_maketables_8(
    gcontext: *mut pcre2_real_general_context,
) -> *const u8 {
    unsafe {
        let yield_ = (if !gcontext.is_null() {
            ((*gcontext).memctl.malloc.unwrap())(TABLES_LENGTH, (*gcontext).memctl.memory_data)
        } else {
            malloc(TABLES_LENGTH)
        }) as *mut u8;

        if yield_.is_null() {
            return core::ptr::null();
        }
        let mut p = yield_;

        /* First comes the lower casing table */

        for i in 0..256i32 {
            let c = tolower(i);
            *p = if c < 256 { c as u8 } else { i as u8 };
            p = p.add(1);
        }

        /* Next the case-flipping table */

        for i in 0..256i32 {
            let c = if islower(i) != 0 { toupper(i) } else { tolower(i) };
            *p = if c < 256 { c as u8 } else { i as u8 };
            p = p.add(1);
        }

        /* Then the character class tables. Don't try to be clever and save
        effort on exclusive ones - in some locales things may be different.

        Note that the table for "space" includes everything "isspace" gives,
        including VT in the default locale. This makes it work for the POSIX
        class [:space:]. From PCRE1 release 8.34 and for all PCRE2 releases it is
        also correct for Perl space, because Perl added VT at release 5.18.

        Note also that it is possible for a character to be alnum or alpha
        without being lower or upper, such as "male and female ordinals" (\xAA
        and \xBA) in the fr_FR locale (at least under Debian Linux's locales as
        of 12/2005). So we must test for alnum specially. */

        memset(p, 0, cbit_length);
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

        /* Finally, the character type table. In this, we used to exclude VT from
        the white space chars, because Perl didn't recognize it as such for \s
        and for comments within regexes. However, Perl changed at release 5.18,
        so PCRE1 changed at release 8.34 and it's always been this way for
        PCRE2. */

        for i in 0..256i32 {
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
            if isalnum(i) != 0 || i as u32 == CHAR_UNDERSCORE {
                x += ctype_word as i32;
            }
            *p = x as u8;
            p = p.add(1);
        }

        yield_ as *const u8
    }
}

/* Free the memory that was allocated by pcre2_maketables(). */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_maketables_free_8(
    gcontext: *mut pcre2_real_general_context,
    tables: *const u8,
) {
    unsafe {
        if !gcontext.is_null() {
            ((*gcontext).memctl.free.unwrap())(
                tables as *mut c_void,
                (*gcontext).memctl.memory_data,
            );
        } else {
            free(tables as *mut c_void);
        }
    }
}
