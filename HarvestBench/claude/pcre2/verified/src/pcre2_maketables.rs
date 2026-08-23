// Translated from c_src/src/pcre2_maketables.c
use crate::internal::*;

/* This module contains the external function pcre2_maketables(), which builds
character tables for PCRE2 in the current locale. */

/*************************************************
*           Create PCRE2 character tables        *
*************************************************/

/* This function builds a set of character tables for use by PCRE2 and returns
a pointer to them. They are build using the ctype functions, and consequently
their contents will depend upon the current locale setting. When compiled as
part of the library, the store is obtained via a general context malloc, if
supplied, but when PCRE2_DFTABLES is defined (when compiling the pcre2_dftables
freestanding auxiliary program) malloc() is used, and the function has a
different name so as not to clash with the prototype in pcre2.h.

Arguments:   a PCRE2 general context or NULL
Returns:     pointer to the contiguous block of data;
               else NULL if memory allocation failed
*/

#[inline]
unsafe fn charfn_to(c: c_int) -> c_int {
    c
}

#[inline]
unsafe fn charfn_from(c: c_int) -> c_int {
    c
}

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
        return std::ptr::null();
    }
    p = yield_;

    /* First comes the lower casing table */

    i = 0;
    while i < 256 {
        let c: c_int = charfn_from(tolower(charfn_to(i)));
        *p = (if c < 256 { c } else { i }) as u8;
        p = p.add(1);
        i += 1;
    }

    /* Next the case-flipping table */

    i = 0;
    while i < 256 {
        let c: c_int = charfn_from(if islower(charfn_to(i)) != 0 {
            toupper(charfn_to(i))
        } else {
            tolower(charfn_to(i))
        });
        *p = (if c < 256 { c } else { i }) as u8;
        p = p.add(1);
        i += 1;
    }

    /* Then the character class tables. Don't try to be clever and save effort on
    exclusive ones - in some locales things may be different.

    Note that the table for "space" includes everything "isspace" gives, including
    VT in the default locale. This makes it work for the POSIX class [:space:].
    From PCRE1 release 8.34 and for all PCRE2 releases it is also correct for Perl
    space, because Perl added VT at release 5.18.

    Note also that it is possible for a character to be alnum or alpha without
    being lower or upper, such as "male and female ordinals" (\xAA and \xBA) in the
    fr_FR locale (at least under Debian Linux's locales as of 12/2005). So we must
    test for alnum specially. */

    memset(p as *mut c_void, 0, cbit_length);
    i = 0;
    while i < 256 {
        if isdigit(charfn_to(i)) != 0 {
            *p.add(cbit_digit + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if isupper(charfn_to(i)) != 0 {
            *p.add(cbit_upper + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if islower(charfn_to(i)) != 0 {
            *p.add(cbit_lower + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if isalnum(charfn_to(i)) != 0 {
            *p.add(cbit_word + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if i == CHAR_UNDERSCORE as c_int {
            *p.add(cbit_word + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if isspace(charfn_to(i)) != 0 {
            *p.add(cbit_space + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if isxdigit(charfn_to(i)) != 0 {
            *p.add(cbit_xdigit + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if isgraph(charfn_to(i)) != 0 {
            *p.add(cbit_graph + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if isprint(charfn_to(i)) != 0 {
            *p.add(cbit_print + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if ispunct(charfn_to(i)) != 0 {
            *p.add(cbit_punct + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        if iscntrl(charfn_to(i)) != 0 {
            *p.add(cbit_cntrl + (i / 8) as usize) |= (1u32 << (i & 7)) as u8;
        }
        i += 1;
    }
    p = p.add(cbit_length);

    /* Finally, the character type table. In this, we used to exclude VT from the
    white space chars, because Perl didn't recognize it as such for \s and for
    comments within regexes. However, Perl changed at release 5.18, so PCRE1
    changed at release 8.34 and it's always been this way for PCRE2. */

    i = 0;
    while i < 256 {
        let mut x: c_int = 0;
        if isspace(charfn_to(i)) != 0 {
            x += ctype_space as c_int;
        }
        if isalpha(charfn_to(i)) != 0 {
            x += ctype_letter as c_int;
        }
        if islower(charfn_to(i)) != 0 {
            x += ctype_lcletter as c_int;
        }
        if isdigit(charfn_to(i)) != 0 {
            x += ctype_digit as c_int;
        }
        if isalnum(charfn_to(i)) != 0 || i == CHAR_UNDERSCORE as c_int {
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

/* End of pcre2_maketables.c */
