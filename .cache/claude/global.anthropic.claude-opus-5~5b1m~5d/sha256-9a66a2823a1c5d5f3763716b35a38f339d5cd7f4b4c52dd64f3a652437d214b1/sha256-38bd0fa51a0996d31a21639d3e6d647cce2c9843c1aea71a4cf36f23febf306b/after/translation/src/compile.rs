//! Translated from pcre2_compile.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use crate::compile_tables::*;
use core::ffi::{c_char, c_void};

/* #define MAX_GROUP_NUMBER 65535u */
pub const MAX_GROUP_NUMBER: u32 = 65535u32;

/* #define XDIGIT(c) xdigitab[c] in 8-bit mode */
macro_rules! XDIGIT {
    ($c:expr) => {
        xdigitab[($c) as usize] as u32
    };
}

/* #define UPPER_CASE(c) ((c)-32) */
macro_rules! UPPER_CASE {
    ($c:expr) => {
        ($c) - 32
    };
}

/* #define IS_DIGIT(x) ((x) >= CHAR_0 && (x) <= CHAR_9) */
macro_rules! IS_DIGIT {
    ($x:expr) => {
        ($x) >= b'0' && ($x) <= b'9'
    };
}



/*************************************************
*               Copy compiled code               *
*************************************************/

/* Compiled JIT code cannot be copied, so the new compiled block has no
associated JIT data. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_copy_8(code: *const pcre2_real_code) -> *mut pcre2_real_code {
    let ref_count: *mut PCRE2_SIZE;
    let newcode: *mut pcre2_real_code;

    if code.is_null() {
        return core::ptr::null_mut();
    }
    newcode = ((*code).memctl.malloc.unwrap())((*code).blocksize, (*code).memctl.memory_data)
        as *mut pcre2_real_code;
    if newcode.is_null() {
        return core::ptr::null_mut();
    }
    core::ptr::copy_nonoverlapping(code as *const u8, newcode as *mut u8, (*code).blocksize);
    (*newcode).executable_jit = core::ptr::null_mut();

    /* If the code is one that has been deserialized, increment the reference count
    in the decoded tables. */

    if ((*code).flags & PCRE2_DEREF_TABLES) != 0 {
        ref_count = (*code).tables.add(TABLES_LENGTH) as *mut PCRE2_SIZE;
        *ref_count = (*ref_count) + 1;
    }

    newcode
}



/*************************************************
*     Copy compiled code and character tables    *
*************************************************/

/* Compiled JIT code cannot be copied, so the new compiled block has no
associated JIT data. This version of code_copy also makes a separate copy of
the character tables. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_copy_with_tables_8(
    code: *const pcre2_real_code,
) -> *mut pcre2_real_code {
    let ref_count: *mut PCRE2_SIZE;
    let newcode: *mut pcre2_real_code;
    let newtables: *mut u8;

    if code.is_null() {
        return core::ptr::null_mut();
    }
    newcode = ((*code).memctl.malloc.unwrap())((*code).blocksize, (*code).memctl.memory_data)
        as *mut pcre2_real_code;
    if newcode.is_null() {
        return core::ptr::null_mut();
    }
    core::ptr::copy_nonoverlapping(code as *const u8, newcode as *mut u8, (*code).blocksize);
    (*newcode).executable_jit = core::ptr::null_mut();

    newtables = ((*code).memctl.malloc.unwrap())(
        TABLES_LENGTH + core::mem::size_of::<PCRE2_SIZE>(),
        (*code).memctl.memory_data,
    ) as *mut u8;
    if newtables.is_null() {
        ((*code).memctl.free.unwrap())(newcode as *mut c_void, (*code).memctl.memory_data);
        return core::ptr::null_mut();
    }
    core::ptr::copy_nonoverlapping((*code).tables, newtables, TABLES_LENGTH);
    ref_count = newtables.add(TABLES_LENGTH) as *mut PCRE2_SIZE;
    *ref_count = 1;

    (*newcode).tables = newtables as *const u8;
    (*newcode).flags |= PCRE2_DEREF_TABLES;
    newcode
}



/*************************************************
*               Free compiled code               *
*************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_free_8(code: *mut pcre2_real_code) {
    let ref_count: *mut PCRE2_SIZE;

    if !code.is_null() {
        if ((*code).flags & PCRE2_DEREF_TABLES) != 0 {
            /* Decoded tables belong to the codes after deserialization, and they must
            be freed when there are no more references to them. The *ref_count should
            always be > 0. */

            ref_count = (*code).tables.add(TABLES_LENGTH) as *mut PCRE2_SIZE;
            if *ref_count > 0 {
                *ref_count = (*ref_count) - 1;
                if *ref_count == 0 {
                    ((*code).memctl.free.unwrap())(
                        (*code).tables as *mut c_void,
                        (*code).memctl.memory_data,
                    );
                }
            }
        }

        ((*code).memctl.free.unwrap())(code as *mut c_void, (*code).memctl.memory_data);
    }
}



/*************************************************
*         Read a number, possibly signed         *
*************************************************/

/* This function is used to read numbers in the pattern. The initial pointer
must be at the sign or first digit of the number. When relative values
(introduced by + or -) are allowed, they are relative group numbers, and the
result must be greater than zero.

Returns:      TRUE  - a number was read
              FALSE - errorcode == 0 => no number was found
                      errorcode != 0 => an error occurred
*/

pub(crate) unsafe fn read_number(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    allow_sign: i32,
    mut max_value: u32,
    max_error: u32,
    intptr: *mut i32,
    errorcodeptr: *mut i32,
) -> BOOL {
    let mut sign: i32 = 0;
    let mut n: u32 = 0;
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let mut yield_: BOOL = FALSE;

    /* PCRE2_ASSERT(max_value <= INT_MAX/10 - 1); */

    *errorcodeptr = 0;

    if allow_sign >= 0 && ptr < ptrend {
        if *ptr == b'+' {
            sign = 1;
            max_value = max_value.wrapping_sub(allow_sign as u32);
            ptr = ptr.add(1);
        } else if *ptr == b'-' {
            sign = -1;
            ptr = ptr.add(1);
        }
    }

    'exit: {
        if ptr >= ptrend || !IS_DIGIT!(*ptr) {
            return FALSE;
        }
        while ptr < ptrend && IS_DIGIT!(*ptr) {
            n = n * 10
                + ({
                    let t = *ptr;
                    ptr = ptr.add(1);
                    t
                } as u32
                    - b'0' as u32);
            if n > max_value {
                *errorcodeptr = max_error as i32;
                while ptr < ptrend && IS_DIGIT!(*ptr) {
                    ptr = ptr.add(1);
                }
                break 'exit; /* goto EXIT */
            }
        }

        if allow_sign >= 0 && sign != 0 {
            if n == 0 {
                *errorcodeptr = ERR26; /* +0 and -0 are not allowed */
                break 'exit; /* goto EXIT */
            }

            if sign > 0 {
                n = n.wrapping_add(allow_sign as u32);
            } else if n > allow_sign as u32 {
                *errorcodeptr = ERR15; /* Non-existent subpattern */
                break 'exit; /* goto EXIT */
            } else {
                n = (allow_sign as u32).wrapping_add(1).wrapping_sub(n);
            }
        }

        yield_ = TRUE;
    }

    /* EXIT: */
    *intptr = n as i32;
    *ptrptr = ptr;
    yield_
}



/*************************************************
*         Read repeat counts                     *
*************************************************/

/* Read an item of the form {n,m} and return the values when non-NULL pointers
are supplied.

Returns:         FALSE if not a repeat quantifier, errorcode set zero
                 FALSE on error, with errorcode set non-zero
                 TRUE on success, with pointer updated to point after '}'
*/

pub(crate) unsafe fn read_repeat_counts(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    minp: *mut u32,
    maxp: *mut u32,
    errorcodeptr: *mut i32,
) -> BOOL {
    let mut p: PCRE2_SPTR = *ptrptr;
    let mut pp: PCRE2_SPTR;
    let mut yield_: BOOL = FALSE;
    let mut had_minimum: BOOL = FALSE;
    let mut min: i32 = 0;
    let mut max: i32 = REPEAT_UNLIMITED as i32; /* This value is larger than MAX_REPEAT_COUNT */

    *errorcodeptr = 0;
    while p < ptrend && (*p == b' ' || *p == 0x09) {
        p = p.add(1);
    }

    /* Check the syntax before interpreting. Otherwise, a non-quantifier sequence
    such as "X{123456ABC" would incorrectly give a "number too big in quantifier"
    error. */

    pp = p;
    if pp < ptrend && IS_DIGIT!(*pp) {
        had_minimum = TRUE;
        loop {
            pp = pp.add(1);
            if !(pp < ptrend && IS_DIGIT!(*pp)) {
                break;
            }
        }
    }

    while pp < ptrend && (*pp == b' ' || *pp == 0x09) {
        pp = pp.add(1);
    }
    if pp >= ptrend {
        return FALSE;
    }

    if *pp == b'}' {
        if had_minimum == FALSE {
            return FALSE;
        }
    } else {
        if {
            let t = *pp;
            pp = pp.add(1);
            t
        } != b','
        {
            return FALSE;
        }
        while pp < ptrend && (*pp == b' ' || *pp == 0x09) {
            pp = pp.add(1);
        }
        if pp >= ptrend {
            return FALSE;
        }
        if IS_DIGIT!(*pp) {
            loop {
                pp = pp.add(1);
                if !(pp < ptrend && IS_DIGIT!(*pp)) {
                    break;
                }
            }
        } else if had_minimum == FALSE {
            return FALSE;
        }
        while pp < ptrend && (*pp == b' ' || *pp == 0x09) {
            pp = pp.add(1);
        }
        if pp >= ptrend || *pp != b'}' {
            return FALSE;
        }
    }

    /* Now process the quantifier for real. We know it must be {n} or {n,} or {,m}
    or {n,m}. The only error that read_number() can return is for a number that is
    too big. If *errorcodeptr is returned as zero it means no number was found. */

    'exit: {
        /* Deal with {,m} or n too big. If we successfully read m there is no need to
        check m >= n because n defaults to zero. */

        if read_number(
            &mut p,
            ptrend,
            -1,
            MAX_REPEAT_COUNT,
            ERR5 as u32,
            &mut min,
            errorcodeptr,
        ) == FALSE
        {
            if *errorcodeptr != 0 {
                break 'exit; /* goto EXIT */
            }
            p = p.add(1); /* Skip comma and subsequent spaces */
            while p < ptrend && (*p == b' ' || *p == 0x09) {
                p = p.add(1);
            }
            if read_number(
                &mut p,
                ptrend,
                -1,
                MAX_REPEAT_COUNT,
                ERR5 as u32,
                &mut max,
                errorcodeptr,
            ) == FALSE
            {
                if *errorcodeptr != 0 {
                    break 'exit; /* goto EXIT */
                }
            }
        }
        /* Have read one number. Deal with {n} or {n,} or {n,m} */
        else {
            while p < ptrend && (*p == b' ' || *p == 0x09) {
                p = p.add(1);
            }
            if *p == b'}' {
                max = min;
            } else
            /* Handle {n,} or {n,m} */
            {
                p = p.add(1); /* Skip comma and subsequent spaces */
                while p < ptrend && (*p == b' ' || *p == 0x09) {
                    p = p.add(1);
                }
                if read_number(
                    &mut p,
                    ptrend,
                    -1,
                    MAX_REPEAT_COUNT,
                    ERR5 as u32,
                    &mut max,
                    errorcodeptr,
                ) == FALSE
                {
                    if *errorcodeptr != 0 {
                        break 'exit; /* goto EXIT */
                    }
                }

                if max < min {
                    *errorcodeptr = ERR4;
                    break 'exit; /* goto EXIT */
                }
            }
        }

        /* Valid quantifier exists */

        while p < ptrend && (*p == b' ' || *p == 0x09) {
            p = p.add(1);
        }
        p = p.add(1);
        yield_ = TRUE;
        if !minp.is_null() {
            *minp = min as u32;
        }
        if !maxp.is_null() {
            *maxp = max as u32;
        }
    }

    /* Update the pattern pointer */

    /* EXIT: */
    *ptrptr = p;
    yield_
}



/*************************************************
*            Handle escapes                      *
*************************************************/

/* This function is called when a \ has been encountered. It either returns a
positive value for a simple escape such as \d, or 0 for a data character, which
is placed in chptr. A backreference to group n is returned as -(n+1).

Returns:         zero => a data character
                 positive => a special escape sequence
                 negative => a numerical back reference
                 on error, errorcodeptr is set non-zero
*/

const CE_MAIN: u32 = 0;
const CE_COME_FROM_NU: u32 = 1;
const CE_ESCAPE_FAILED_FORWARD: u32 = 2;
const CE_EXIT: u32 = 3;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_check_escape_8(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    chptr: *mut u32,
    errorcodeptr: *mut i32,
    options: u32,
    xoptions: u32,
    bracount: u32,
    isclass: BOOL,
    cb: *mut compile_block,
) -> i32 {
    let utf: BOOL = ((options & PCRE2_UTF) != 0) as BOOL;
    let mut alt_bsux: BOOL =
        (((options & PCRE2_ALT_BSUX) | (xoptions & PCRE2_EXTRA_ALT_BSUX)) != 0) as BOOL;
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let mut c: u32 = 0;
    let mut cc: u32 = 0;
    let mut escape: i32 = 0;
    let mut i: i32 = 0;

    /* Variables that are declared inside the "further processing" block in C but
    which have to be shared with the COME_FROM_NU label. */
    let mut s: i32 = 0;
    let mut oldptr: PCRE2_SPTR = core::ptr::null();
    let mut overflow: BOOL = FALSE;

    /* If backslash is at the end of the string, it's an error. */

    if ptr >= ptrend {
        *errorcodeptr = ERR1;
        return 0;
    }

    GETCHARINCTEST!(c, ptr, utf); /* Get character value, increment pointer */
    *errorcodeptr = 0; /* Be optimistic */

    let mut state: u32 = CE_MAIN;
    'sm: loop {
        match state {
            CE_MAIN => {
                /* Non-alphanumerics are literals, so we just leave the value in c. An
                initial value test saves a memory lookup for code points outside the
                alphanumeric range. */

                if c < ESCAPES_FIRST || c > ESCAPES_LAST { /* Definitely literal */ }
                /* Otherwise, do a table lookup. */
                else if {
                    i = escapes[(c - ESCAPES_FIRST) as usize] as i32;
                    i != 0
                } {
                    if i > 0 {
                        c = i as u32;
                        if c == 0x0d /* CHAR_CR */
                            && (xoptions & PCRE2_EXTRA_ESCAPED_CR_IS_LF) != 0
                        {
                            c = 0x0a; /* CHAR_LF */
                        }
                    } else
                    /* Negative table entry */
                    {
                        escape = -i; /* Else return a special escape */
                        if !cb.is_null()
                            && (escape == ESC_P as i32
                                || escape == ESC_p as i32
                                || escape == ESC_X as i32)
                        {
                            (*cb).external_flags |= PCRE2_HASBKPORX; /* Note \P, \p, or \X */
                        }

                        /* Perl supports \N{name} for character names and \N{U+dddd} for
                        numerical Unicode code points, as well as plain \N for "not
                        newline". PCRE does not support \N{name}. However, it does support
                        quantification such as \N{2,3}, so if \N{ is not followed by U+dddd
                        we check for a quantifier. */

                        if escape == ESC_N as i32 && ptr < ptrend && *ptr == b'{' {
                            let mut p: PCRE2_SPTR = ptr.add(1);

                            /* Perl ignores spaces and tabs after { */

                            while p < ptrend && (*p == b' ' || *p == 0x09) {
                                p = p.add(1);
                            }

                            /* \N{U+ can be handled by the \x{ code. */

                            if ptrend.offset_from(p) > 1 && *p == b'U' && *p.add(1) == b'+' {
                                if utf != 0 {
                                    ptr = p.add(2);
                                    escape = 0; /* Not a fancy escape after all */
                                    state = CE_COME_FROM_NU; /* goto COME_FROM_NU */
                                    continue 'sm;
                                }

                                /* Improve error offset. */
                                ptr = p.add(2);
                                while ptr < ptrend && XDIGIT!(*ptr) != 0xff {
                                    ptr = ptr.add(1);
                                }
                                while ptr < ptrend && (*ptr == b' ' || *ptr == 0x09) {
                                    ptr = ptr.add(1);
                                }
                                if ptr < ptrend && *ptr == b'}' {
                                    ptr = ptr.add(1);
                                }

                                *errorcodeptr = ERR93;
                            }
                            /* Give an error in contexts where quantifiers are not allowed
                            (character classes; substitution strings). */
                            else if isclass != 0 || cb.is_null() {
                                ptr = ptr.add(1); /* Skip over the opening brace */
                                *errorcodeptr = ERR37;
                            }
                            /* Give an error if what follows is not a quantifier, but don't
                            override an error set by the quantifier reader. */
                            else {
                                if read_repeat_counts(
                                    &mut p,
                                    ptrend,
                                    core::ptr::null_mut(),
                                    core::ptr::null_mut(),
                                    errorcodeptr,
                                ) == FALSE
                                    && *errorcodeptr == 0
                                {
                                    ptr = ptr.add(1); /* Skip over the opening brace */
                                    *errorcodeptr = ERR37;
                                }
                            }
                        }
                    }
                }
                /* Escapes that need further processing, including those that are
                unknown, have a zero entry in the lookup table. When called from
                pcre2_substitute(), only \c, \o, and \x are recognized. */
                else {
                    /* Filter calls from pcre2_substitute(). */

                    if cb.is_null() {
                        if !(c >= b'0' as u32 && c <= b'9' as u32)
                            && c != b'c' as u32
                            && c != b'o' as u32
                            && c != b'x' as u32
                            && c != b'g' as u32
                        {
                            *errorcodeptr = ERR3;
                            state = CE_EXIT; /* goto EXIT */
                            continue 'sm;
                        }
                        alt_bsux = FALSE; /* Do not modify \x handling */
                    }

                    'sw: {
                        match c {
                            /* A number of Perl escapes are not handled by PCRE. We give an
                            explicit error. */
                            0x46 /* CHAR_F */ | 0x6c /* CHAR_l */ | 0x4c /* CHAR_L */ => {
                                *errorcodeptr = ERR37;
                            }

                            /* \u is unrecognized when neither PCRE2_ALT_BSUX nor
                            PCRE2_EXTRA_ALT_BSUX is set. */
                            0x75 /* CHAR_u */ => {
                                if alt_bsux == FALSE {
                                    *errorcodeptr = ERR37;
                                } else {
                                    let mut xc: u32;

                                    if ptr >= ptrend {
                                        break 'sw;
                                    }
                                    if *ptr == b'{' && (xoptions & PCRE2_EXTRA_ALT_BSUX) != 0 {
                                        let mut hptr: PCRE2_SPTR = ptr.add(1);

                                        cc = 0;
                                        while hptr < ptrend && {
                                            xc = XDIGIT!(*hptr);
                                            xc != 0xff
                                        } {
                                            if (cc & 0xf0000000) != 0 {
                                                /* Test for 32-bit overflow */
                                                *errorcodeptr = ERR77;
                                                ptr = hptr; /* Show where */
                                                break; /* *hptr != } will cause another break below */
                                            }
                                            cc = (cc << 4) | xc;
                                            hptr = hptr.add(1);
                                        }

                                        if hptr == ptr.add(1) ||  /* No hex digits */
                                           hptr >= ptrend ||      /* Hit end of input */
                                           *hptr != b'}'
                                        /* No } terminator */
                                        {
                                            if isclass != 0 {
                                                break 'sw; /* In a class, just treat as '\u' literal */
                                            }
                                            escape = ESC_ub as i32; /* Special return */
                                            ptr = ptr.add(1); /* Skip { */
                                            break 'sw; /* Hex escape not recognized */
                                        }

                                        c = cc; /* Accept the code point */
                                        ptr = hptr.add(1);
                                    } else
                                    /* Must be exactly 4 hex digits */
                                    {
                                        if ptrend.offset_from(ptr) < 4 {
                                            break 'sw; /* Less than 4 chars */
                                        }
                                        cc = XDIGIT!(*ptr.add(0));
                                        if cc == 0xff {
                                            break 'sw; /* Not a hex digit */
                                        }
                                        xc = XDIGIT!(*ptr.add(1));
                                        if xc == 0xff {
                                            break 'sw; /* Not a hex digit */
                                        }
                                        cc = (cc << 4) | xc;
                                        xc = XDIGIT!(*ptr.add(2));
                                        if xc == 0xff {
                                            break 'sw; /* Not a hex digit */
                                        }
                                        cc = (cc << 4) | xc;
                                        xc = XDIGIT!(*ptr.add(3));
                                        if xc == 0xff {
                                            break 'sw; /* Not a hex digit */
                                        }
                                        c = (cc << 4) | xc;
                                        ptr = ptr.add(4);
                                    }

                                    if utf != 0 {
                                        if c > 0x10ffffu32 {
                                            *errorcodeptr = ERR77;
                                        } else if c >= 0xd800
                                            && c <= 0xdfff
                                            && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0
                                        {
                                            *errorcodeptr = ERR73;
                                        }
                                    } else if c > MAX_NON_UTF_CHAR {
                                        *errorcodeptr = ERR77;
                                    }
                                }
                            }

                            /* \U is unrecognized unless PCRE2_ALT_BSUX or
                            PCRE2_EXTRA_ALT_BSUX is set, in which case it is an upper case
                            letter. */
                            0x55 /* CHAR_U */ => {
                                if alt_bsux == FALSE {
                                    *errorcodeptr = ERR37;
                                }
                            }

                            /* In a character class, \g is just a literal "g". */
                            0x67 /* CHAR_g */ => {
                                if isclass != 0 {
                                    break 'sw;
                                }

                                if ptr >= ptrend {
                                    *errorcodeptr = ERR57;
                                    break 'sw;
                                }

                                if cb.is_null() {
                                    let mut p: PCRE2_SPTR;
                                    /* Substitution strings */
                                    if *ptr != b'<' {
                                        *errorcodeptr = ERR57;
                                        break 'sw;
                                    }

                                    p = ptr.add(1);

                                    if read_number(
                                        &mut p,
                                        ptrend,
                                        -1,
                                        MAX_GROUP_NUMBER,
                                        ERR61 as u32,
                                        &mut s,
                                        errorcodeptr,
                                    ) == FALSE
                                    {
                                        if *errorcodeptr == 0 {
                                            escape = ESC_g as i32; /* No number found */
                                        }
                                        break 'sw;
                                    }

                                    if p >= ptrend || *p != b'>' {
                                        ptr = p;
                                        *errorcodeptr = ERR119; /* Missing terminator for number */
                                        break 'sw;
                                    }

                                    /* This is the reason that back references are returned as
                                    -(s+1) rather than just -s. */
                                    ptr = p.add(1);
                                    escape = -(s + 1);
                                    break 'sw;
                                }

                                if *ptr == b'<' || *ptr == b'\'' {
                                    escape = ESC_g as i32;
                                    break 'sw;
                                }

                                /* If there is a brace delimiter, try to read a numerical
                                reference. If there isn't one, assume we have a name and treat
                                it as \k. */

                                if *ptr == b'{' {
                                    let mut p: PCRE2_SPTR = ptr.add(1);

                                    while p < ptrend && (*p == b' ' || *p == 0x09) {
                                        p = p.add(1);
                                    }
                                    if read_number(
                                        &mut p,
                                        ptrend,
                                        bracount as i32,
                                        MAX_GROUP_NUMBER,
                                        ERR61 as u32,
                                        &mut s,
                                        errorcodeptr,
                                    ) == FALSE
                                    {
                                        if *errorcodeptr == 0 {
                                            escape = ESC_k as i32; /* No number found */
                                        }
                                        break 'sw;
                                    }
                                    while p < ptrend && (*p == b' ' || *p == 0x09) {
                                        p = p.add(1);
                                    }

                                    if p >= ptrend || *p != b'}' {
                                        ptr = p;
                                        *errorcodeptr = ERR119; /* Missing terminator for number */
                                        break 'sw;
                                    }
                                    ptr = p.add(1);
                                }
                                /* Read an undelimited number */
                                else {
                                    if read_number(
                                        &mut ptr,
                                        ptrend,
                                        bracount as i32,
                                        MAX_GROUP_NUMBER,
                                        ERR61 as u32,
                                        &mut s,
                                        errorcodeptr,
                                    ) == FALSE
                                    {
                                        if *errorcodeptr == 0 {
                                            *errorcodeptr = ERR57; /* No number found */
                                        }
                                        break 'sw;
                                    }
                                }

                                if s <= 0 {
                                    *errorcodeptr = ERR15;
                                    break 'sw;
                                }

                                escape = -(s + 1);
                            }

                            /* Digits. CHAR_1 .. CHAR_9 fall through into CHAR_0. */
                            0x30..=0x39 => {
                                if c != 0x30
                                /* case CHAR_1 ... CHAR_9 */
                                {
                                    if isclass != 0 {
                                        /* Fall through to octal handling; never a
                                        backreference inside a class. */
                                    } else if (xoptions & PCRE2_EXTRA_PYTHON_OCTAL) != 0 {
                                        /* Python-style disambiguation. */
                                        if *ptr.offset(-1) <= b'7'
                                            && ptr.add(1) < ptrend
                                            && *ptr.add(0) >= b'0'
                                            && *ptr.add(0) <= b'7'
                                            && *ptr.add(1) >= b'0'
                                            && *ptr.add(1) <= b'7'
                                        {
                                            /* We peeked a three-digit octal, so fall through */
                                        } else {
                                            /* We are at a digit, so the only possible error from
                                            read_number() is a number that is too large. */
                                            ptr = ptr.wrapping_sub(1); /* Back to the digit */

                                            if read_number(
                                                &mut ptr,
                                                ptrend,
                                                -1,
                                                MAX_GROUP_NUMBER,
                                                0,
                                                &mut s,
                                                errorcodeptr,
                                            ) == FALSE
                                            {
                                                *errorcodeptr = ERR61;
                                                break 'sw;
                                            }

                                            escape = -(s + 1);
                                            break 'sw;
                                        }
                                    } else {
                                        /* Perl-style disambiguation. */
                                        oldptr = ptr;
                                        ptr = ptr.wrapping_sub(1); /* Back to the digit */

                                        if read_number(
                                            &mut ptr,
                                            ptrend,
                                            -1,
                                            MAX_GROUP_NUMBER,
                                            0,
                                            &mut s,
                                            errorcodeptr,
                                        ) == FALSE
                                        {
                                            s = i32::MAX;
                                        }

                                        /* \1 to \9 are always back references. \8x and \9x are
                                        too; \1x to \7x are octal escapes if there are not that
                                        many previous captures. */

                                        if s < 10 || c >= b'8' as u32 || (s as u32) <= bracount {
                                            if (s as u32) > MAX_GROUP_NUMBER {
                                                /* PCRE2_ASSERT(s == INT_MAX); */
                                                *errorcodeptr = ERR61;
                                            } else {
                                                escape = -(s + 1); /* Indicates a back reference */
                                            }
                                            break 'sw;
                                        }

                                        ptr = oldptr; /* Put the pointer back and fall through */
                                    }

                                    /* Handle a digit following \ when the number is not a back
                                    reference, or we are within a character class. */

                                    if c >= b'8' as u32 {
                                        break 'sw;
                                    }

                                    /* PCRE2_FALLTHROUGH */
                                }

                                /* case CHAR_0: */
                                c -= b'0' as u32;
                                loop {
                                    let cond = {
                                        let t = i;
                                        i = i + 1;
                                        t
                                    } < 2
                                        && ptr < ptrend
                                        && *ptr >= b'0'
                                        && *ptr <= b'7';
                                    if !cond {
                                        break;
                                    }
                                    c = c * 8
                                        + ({
                                            let t = *ptr;
                                            ptr = ptr.add(1);
                                            t
                                        } as u32)
                                        - b'0' as u32;
                                }
                                if c > 0xff {
                                    if (xoptions & PCRE2_EXTRA_PYTHON_OCTAL) != 0 {
                                        *errorcodeptr = ERR102;
                                    } else if utf == 0 {
                                        *errorcodeptr = ERR51;
                                    }
                                }

                                /* PCRE2_EXTRA_NO_BS0 disables the NUL escape '\0' but doesn't
                                affect two- or three-character octal escapes \00 and \000, nor
                                \x00. */

                                if (xoptions & PCRE2_EXTRA_NO_BS0) != 0 && c == 0 && i == 1 {
                                    *errorcodeptr = ERR98;
                                }
                            }

                            /* \o is a relatively new Perl feature. The only supported form is
                            \o{ddd}. */
                            0x6f /* CHAR_o */ => {
                                if ptr >= ptrend || *ptr != b'{' {
                                    *errorcodeptr = ERR55;
                                    break 'sw;
                                }
                                ptr = ptr.add(1);

                                while ptr < ptrend && (*ptr == b' ' || *ptr == 0x09) {
                                    ptr = ptr.add(1);
                                }
                                if ptr >= ptrend || *ptr == b'}' {
                                    *errorcodeptr = ERR78;
                                    break 'sw;
                                }

                                c = 0;
                                overflow = FALSE;
                                while ptr < ptrend && *ptr >= b'0' && *ptr <= b'7' {
                                    cc = {
                                        let t = *ptr;
                                        ptr = ptr.add(1);
                                        t
                                    } as u32;
                                    if c == 0 && cc == b'0' as u32 {
                                        continue; /* Leading zeroes */
                                    }
                                    c = (c << 3) + (cc - b'0' as u32);
                                    if c > (if utf != 0 { 0x10ffffu32 } else { 0xffu32 }) {
                                        overflow = TRUE;
                                        break;
                                    }
                                }

                                while ptr < ptrend && (*ptr == b' ' || *ptr == 0x09) {
                                    ptr = ptr.add(1);
                                }

                                if overflow != FALSE {
                                    while ptr < ptrend && *ptr >= b'0' && *ptr <= b'7' {
                                        ptr = ptr.add(1);
                                    }
                                    *errorcodeptr = ERR34;
                                } else if utf != 0
                                    && c >= 0xd800
                                    && c <= 0xdfff
                                    && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0
                                {
                                    *errorcodeptr = ERR73;
                                } else if ptr < ptrend && *ptr == b'}' {
                                    ptr = ptr.add(1);
                                } else {
                                    *errorcodeptr = ERR64;
                                    state = CE_ESCAPE_FAILED_FORWARD; /* goto ESCAPE_FAILED_FORWARD */
                                    continue 'sm;
                                }
                            }

                            /* When PCRE2_ALT_BSUX or PCRE2_EXTRA_ALT_BSUX is set, \x must be
                            followed by two hexadecimal digits. */
                            0x78 /* CHAR_x */ => {
                                if alt_bsux != FALSE {
                                    let mut xc: u32;
                                    if ptrend.offset_from(ptr) < 2 {
                                        break 'sw; /* Less than 2 characters */
                                    }
                                    cc = XDIGIT!(*ptr.add(0));
                                    if cc == 0xff {
                                        break 'sw; /* Not a hex digit */
                                    }
                                    xc = XDIGIT!(*ptr.add(1));
                                    if xc == 0xff {
                                        break 'sw; /* Not a hex digit */
                                    }
                                    c = (cc << 4) | xc;
                                    ptr = ptr.add(2);
                                }
                                /* Handle \x in Perl's style. */
                                else {
                                    if ptr < ptrend && *ptr == b'{' {
                                        ptr = ptr.add(1);
                                        while ptr < ptrend && (*ptr == b' ' || *ptr == 0x09) {
                                            ptr = ptr.add(1);
                                        }

                                        state = CE_COME_FROM_NU; /* fall into COME_FROM_NU */
                                        continue 'sm;
                                    }
                                    /* Read a up to two hex digits after \x */
                                    else {
                                        /* Perl has the surprising/broken behaviour that \x
                                        without following hex digits is treated as an escape for
                                        NUL. Because we don't have warnings, we simply forbid it. */
                                        if ptr >= ptrend || {
                                            cc = XDIGIT!(*ptr);
                                            cc == 0xff
                                        } {
                                            /* Not a hex digit */
                                            *errorcodeptr = ERR78;
                                            break 'sw;
                                        }
                                        ptr = ptr.add(1);
                                        c = cc;

                                        if ptr >= ptrend || {
                                            cc = XDIGIT!(*ptr);
                                            cc == 0xff
                                        } {
                                            break 'sw; /* Not a hex digit */
                                        }
                                        ptr = ptr.add(1);
                                        c = (c << 4) | cc;
                                    } /* End of \xdd handling */
                                } /* End of Perl-style \x handling */
                            }

                            /* The handling of \c is different in ASCII and EBCDIC
                            environments. */
                            0x63 /* CHAR_c */ => {
                                if ptr >= ptrend {
                                    *errorcodeptr = ERR2;
                                    break 'sw;
                                }
                                c = *ptr as u32;
                                if c >= b'a' as u32 && c <= b'z' as u32 {
                                    c = UPPER_CASE!(c);
                                }

                                /* Handle \c in an ASCII/Unicode environment. */

                                if c < 32 || c > 126
                                /* Excludes all non-printable ASCII */
                                {
                                    *errorcodeptr = ERR68;
                                    state = CE_ESCAPE_FAILED_FORWARD; /* goto ESCAPE_FAILED_FORWARD */
                                    continue 'sm;
                                }
                                c ^= 0x40;

                                ptr = ptr.add(1);
                            }

                            /* Any other alphanumeric following \ is an error. */
                            _ => {
                                *errorcodeptr = ERR3;
                            }
                        }
                    }
                }

                /* Set the pointer to the next character before returning. */
                state = CE_EXIT;
                continue 'sm;
            }

            CE_COME_FROM_NU => {
                'sw2: {
                    if ptr >= ptrend || *ptr == b'}' {
                        *errorcodeptr = ERR78;
                        break 'sw2;
                    }
                    c = 0;
                    overflow = FALSE;

                    while ptr < ptrend && {
                        cc = XDIGIT!(*ptr);
                        cc != 0xff
                    } {
                        ptr = ptr.add(1);
                        if c == 0 && cc == 0 {
                            continue; /* Leading zeroes */
                        }
                        c = (c << 4) | cc;
                        if (utf != 0 && c > 0x10ffffu32) || (utf == 0 && c > MAX_NON_UTF_CHAR) {
                            overflow = TRUE;
                            break;
                        }
                    }

                    /* Perl ignores spaces and tabs before } */

                    while ptr < ptrend && (*ptr == b' ' || *ptr == 0x09) {
                        ptr = ptr.add(1);
                    }

                    /* On overflow, skip remaining hex digits */

                    if overflow != FALSE {
                        while ptr < ptrend && XDIGIT!(*ptr) != 0xff {
                            ptr = ptr.add(1);
                        }
                        *errorcodeptr = ERR34;
                    } else if utf != 0
                        && c >= 0xd800
                        && c <= 0xdfff
                        && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0
                    {
                        *errorcodeptr = ERR73;
                    } else if ptr < ptrend && *ptr == b'}' {
                        ptr = ptr.add(1);
                    }
                    /* If the sequence of hex digits (followed by optional space) does not
                    end with '}', give an error. */
                    else {
                        *errorcodeptr = ERR67;
                        state = CE_ESCAPE_FAILED_FORWARD; /* goto ESCAPE_FAILED_FORWARD */
                        continue 'sm;
                    }
                } /* End of \x{} processing; falls out of the switch */

                state = CE_EXIT;
                continue 'sm;
            }

            /* Some errors need to indicate the next character. */
            CE_ESCAPE_FAILED_FORWARD => {
                ptr = ptr.add(1);
                if utf != 0 {
                    FORWARDCHARTEST!(ptr, ptrend);
                }
                state = CE_EXIT; /* goto EXIT */
                continue 'sm;
            }

            /* EXIT: */
            _ => {
                *ptrptr = ptr;
                *chptr = c;
                return escape;
            }
        }
    }
}



/*************************************************
*               Handle \P and \p                 *
*************************************************/

/* This function is called after \P or \p has been encountered, provided that
PCRE2 is compiled with support for UTF and Unicode properties.

Returns:         TRUE if the type value was found, or FALSE for an invalid type
*/

pub(crate) unsafe fn get_ucp(
    ptrptr: *mut PCRE2_SPTR,
    utf: BOOL,
    negptr: *mut BOOL,
    ptypeptr: *mut u16,
    pdataptr: *mut u16,
    errorcodeptr: *mut i32,
    cb: *mut compile_block,
) -> BOOL {
    let mut c: u32 = 0;
    let mut i: isize = 0;
    let mut bot: PCRE2_SIZE;
    let mut top: PCRE2_SIZE;
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let mut name: [PCRE2_UCHAR; 50] = [0; 50];
    let name_p: *mut PCRE2_UCHAR = name.as_mut_ptr();
    let mut vptr: *mut PCRE2_UCHAR = core::ptr::null_mut();
    let mut ptscript: u16 = PT_NOTSCRIPT as u16;

    'error_return: {
        if ptr >= (*cb).end_pattern {
            break 'error_return; /* goto ERROR_RETURN */
        }
        GETCHARINCTEST!(c, ptr, utf);
        *negptr = FALSE;

        /* \P or \p can be followed by a name in {}, optionally preceded by ^ for
        negation. */

        if c == b'{' as u32 {
            if ptr >= (*cb).end_pattern {
                break 'error_return; /* goto ERROR_RETURN */
            }

            i = 0;
            'nameloop: while i < (50 as isize) - 1 {
                'redo: loop {
                    /* REDO: */

                    if ptr >= (*cb).end_pattern {
                        break 'error_return; /* goto ERROR_RETURN */
                    }
                    GETCHARINCTEST!(c, ptr, utf);

                    /* Skip ignorable Unicode characters. */

                    if c == b'_' as u32
                        || c == b'-' as u32
                        || c == b' ' as u32
                        || (c >= 0x09 && c <= 0x0d)
                    {
                        continue 'redo; /* goto REDO */
                    }

                    /* The first significant character being circumflex negates the
                    meaning of the item. */

                    if i == 0 && *negptr == FALSE && c == b'^' as u32 {
                        *negptr = TRUE;
                        continue 'redo; /* goto REDO */
                    }

                    if c == b'}' as u32 {
                        break 'nameloop;
                    }

                    /* Names consist of ASCII letters and digits, but equals and colon may
                    also occur as a name/value separator. */

                    if c < b'&' as u32 || c > b'z' as u32 {
                        break 'error_return; /* goto ERROR_RETURN */
                    }

                    /* Lower case a capital letter or remember where the name/value
                    separator is. */

                    if c >= b'A' as u32 && c <= b'Z' as u32 {
                        c |= 0x20;
                    } else if (c == b':' as u32 || c == b'=' as u32) && vptr.is_null() {
                        vptr = name_p.offset(i);
                    }

                    name[i as usize] = c as PCRE2_UCHAR;
                    break;
                }
                i += 1;
            }

            /* Error if the loop didn't end with '}' - either we hit the end of the
            pattern or the name was longer than any legal property name. */

            if c != b'}' as u32 {
                break 'error_return; /* goto ERROR_RETURN */
            }
            name[i as usize] = 0;
        }
        /* If { doesn't follow \p or \P there is just one following character, which
        must be an ASCII letter. */
        else if c >= b'A' as u32 && c <= b'Z' as u32 {
            name[0] = (c | 0x20) as PCRE2_UCHAR; /* Lower case */
            name[1] = 0;
        } else if c >= b'a' as u32 && c <= b'z' as u32 {
            name[0] = c as PCRE2_UCHAR;
            name[1] = 0;
        } else {
            break 'error_return; /* goto ERROR_RETURN */
        }

        *ptrptr = ptr; /* Update pattern pointer */

        /* If the property contains ':' or '=' we have class name and value
        separately specified. */

        if !vptr.is_null() {
            let mut offset: i32 = 0;
            let mut sname: [PCRE2_UCHAR; 8] = [0; 8];

            *vptr = 0; /* Terminate property name */
            if crate::string_utils::_pcre2_strcmp_c8_8(
                name_p as PCRE2_SPTR,
                b"bidiclass\0".as_ptr() as *const c_char,
            ) == 0
                || crate::string_utils::_pcre2_strcmp_c8_8(
                    name_p as PCRE2_SPTR,
                    b"bc\0".as_ptr() as *const c_char,
                ) == 0
            {
                offset = 4;
                sname[0] = b'b';
                sname[1] = b'i'; /* There is no strcpy_c8 function */
                sname[2] = b'd';
                sname[3] = b'i';
            } else if crate::string_utils::_pcre2_strcmp_c8_8(
                name_p as PCRE2_SPTR,
                b"script\0".as_ptr() as *const c_char,
            ) == 0
                || crate::string_utils::_pcre2_strcmp_c8_8(
                    name_p as PCRE2_SPTR,
                    b"sc\0".as_ptr() as *const c_char,
                ) == 0
            {
                ptscript = PT_SC as u16;
            } else if crate::string_utils::_pcre2_strcmp_c8_8(
                name_p as PCRE2_SPTR,
                b"scriptextensions\0".as_ptr() as *const c_char,
            ) == 0
                || crate::string_utils::_pcre2_strcmp_c8_8(
                    name_p as PCRE2_SPTR,
                    b"scx\0".as_ptr() as *const c_char,
                ) == 0
            {
                ptscript = PT_SCX as u16;
            } else {
                *errorcodeptr = ERR47;
                return FALSE;
            }

            /* Adjust the string in name[] as needed */

            core::ptr::copy(
                vptr.add(1) as *const PCRE2_UCHAR,
                name_p.offset(offset as isize),
                name_p.offset(i).offset_from(vptr) as usize,
            );
            if offset != 0 {
                core::ptr::copy(sname.as_ptr(), name_p, offset as usize);
            }
        }

        /* Search for a recognized property using binary chop. */

        bot = 0;
        top = crate::tables::_pcre2_utt_size_8;

        while bot < top {
            let r: i32;
            i = ((bot + top) >> 1) as isize;
            r = crate::string_utils::_pcre2_strcmp_c8_8(
                name_p as PCRE2_SPTR,
                crate::tables::_pcre2_utt_names_8
                    .as_ptr()
                    .add(crate::tables::_pcre2_utt_8[i as usize].name_offset as usize)
                    as *const c_char,
            );

            /* When a matching property is found, some extra checking is needed when
            the \p{xx:yy} syntax is used and xx is either sc or scx. */

            if r == 0 {
                *pdataptr = crate::tables::_pcre2_utt_8[i as usize].value;
                if vptr.is_null() || ptscript == PT_NOTSCRIPT as u16 {
                    *ptypeptr = crate::tables::_pcre2_utt_8[i as usize].type_;
                    return TRUE;
                }

                match crate::tables::_pcre2_utt_8[i as usize].type_ as u32 {
                    PT_SC => {
                        *ptypeptr = PT_SC as u16;
                        return TRUE;
                    }
                    PT_SCX => {
                        *ptypeptr = ptscript;
                        return TRUE;
                    }
                    _ => {}
                }

                break; /* Non-script found */
            }

            if r > 0 {
                bot = (i + 1) as PCRE2_SIZE;
            } else {
                top = i as PCRE2_SIZE;
            }
        }

        *errorcodeptr = ERR47; /* Unrecognized property */
        return FALSE;
    }

    /* ERROR_RETURN: Malformed \P or \p */
    *errorcodeptr = ERR46;
    *ptrptr = ptr;
    FALSE
}



/*************************************************
*           Check for POSIX class syntax         *
*************************************************/

/* This function is called when the sequence "[:" or "[." or "[=" is
encountered in a character class.

Returns:   TRUE or FALSE
*/

pub(crate) unsafe fn check_posix_syntax(
    mut ptr: PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    endptr: *mut PCRE2_SPTR,
) -> BOOL {
    let terminator: PCRE2_UCHAR; /* Don't combine these lines; the Solaris cc */
    terminator = {
        let t = *ptr;
        ptr = ptr.add(1);
        t
    }; /* compiler warns about "non-constant" initializer. */

    while ptrend.offset_from(ptr) >= 2 {
        if *ptr == b'\\' && (*ptr.add(1) == b']' || *ptr.add(1) == b'\\') {
            ptr = ptr.add(1);
        } else if (*ptr == b'[' && *ptr.add(1) == terminator) || *ptr == b']' {
            return FALSE;
        } else if *ptr == terminator && *ptr.add(1) == b']' {
            *endptr = ptr;
            return TRUE;
        }

        ptr = ptr.add(1);
    }

    FALSE
}



/*************************************************
*          Check POSIX class name                *
*************************************************/

/* This function is called to check the name given in a POSIX-style class entry
such as [:alnum:].

Returns:     a value representing the name, or -1 if unknown
*/

pub(crate) unsafe fn check_posix_name(ptr: PCRE2_SPTR, len: i32) -> i32 {
    let mut pn: *const c_char = posix_names.as_ptr() as *const c_char;
    let mut yield_: i32 = 0;
    while posix_name_lengths[yield_ as usize] != 0 {
        if len == posix_name_lengths[yield_ as usize] as i32
            && crate::string_utils::_pcre2_strncmp_c8_8(ptr, pn, len as u32 as usize) == 0
        {
            return yield_;
        }
        pn = pn.add(posix_name_lengths[yield_ as usize] as usize + 1);
        yield_ += 1;
    }
    -1
}



/*************************************************
*       Read a subpattern or VERB name           *
*************************************************/

/* This function is called from parse_regex() below whenever it needs to read
the name of a subpattern or a (*VERB) or an (*alpha_assertion).

Returns:    TRUE if a name was read
            FALSE otherwise, with error code set
*/

pub(crate) unsafe fn read_name(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    terminator: u32,
    offsetptr: *mut PCRE2_SIZE,
    nameptr: *mut PCRE2_SPTR,
    namelenptr: *mut u32,
    errorcodeptr: *mut i32,
    cb: *mut compile_block,
) -> BOOL {
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let is_group: BOOL = ({
        let t = *ptr;
        ptr = ptr.add(1);
        t
    } != b'*') as BOOL;
    let is_braced: BOOL = (terminator == b'}' as u32) as BOOL;

    'failed: {
        if is_braced != 0 {
            while ptr < ptrend && (*ptr == b' ' || *ptr == 0x09) {
                ptr = ptr.add(1);
            }
        }

        if ptr >= ptrend
        /* No characters in name */
        {
            *errorcodeptr = if is_group != 0 {
                ERR62 /* Subpattern name expected */
            } else {
                ERR60 /* Verb not recognized or malformed */
            };
            break 'failed; /* goto FAILED */
        }

        *nameptr = ptr;
        *offsetptr = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;

        /* In UTF mode, a group name may contain letters and decimal digits as
        defined by Unicode properties, and underscores, but must not start with a
        digit. */

        if utf != 0 && is_group != 0 {
            let mut c: u32 = 0;
            let mut type_: u32;
            let mut p: PCRE2_SPTR = ptr;

            GETCHARINC!(c, p); /* Peek at next character */
            type_ = UCD_CHARTYPE!(c);

            if type_ == ucp_Nd {
                ptr = p;
                *errorcodeptr = ERR44;
                break 'failed; /* goto FAILED */
            }

            loop {
                if type_ != ucp_Nd
                    && crate::tables::_pcre2_ucp_gentype_8[type_ as usize] != ucp_L
                    && c != b'_' as u32
                {
                    break;
                }
                ptr = p; /* Accept character and peek again */
                if p >= ptrend {
                    break;
                }
                GETCHARINC!(c, p);
                type_ = UCD_CHARTYPE!(c);
            }
        }
        /* Handle non-group names and group names in non-UTF modes. */
        else {
            if is_group != 0 && IS_DIGIT!(*ptr) {
                ptr = ptr.add(1);
                *errorcodeptr = ERR44;
                break 'failed; /* goto FAILED */
            }

            while ptr < ptrend
                && MAX_255!(*ptr) != 0
                && (*(*cb).ctypes.add(*ptr as usize) & ctype_word) != 0
            {
                ptr = ptr.add(1);
            }
        }

        /* Check name length */

        if ptr.offset_from(*nameptr) > MAX_NAME_SIZE as isize {
            *errorcodeptr = ERR48;
            break 'failed; /* goto FAILED */
        }
        *namelenptr = ptr.offset_from(*nameptr) as u32;

        /* Subpattern names must not be empty, and their terminator is checked
        here. */

        if is_group != 0 {
            if ptr == *nameptr {
                *errorcodeptr = ERR62; /* Subpattern name expected */
                break 'failed; /* goto FAILED */
            }
            if is_braced != 0 {
                while ptr < ptrend && (*ptr == b' ' || *ptr == 0x09) {
                    ptr = ptr.add(1);
                }
            }
            if terminator != 0 {
                if ptr >= ptrend || *ptr != terminator as PCRE2_UCHAR {
                    *errorcodeptr = ERR42;
                    break 'failed; /* goto FAILED */
                }
                ptr = ptr.add(1);
            }
        }

        *ptrptr = ptr;
        return TRUE;
    }

    /* FAILED: */
    *ptrptr = ptr;
    FALSE
}



/**************************************************
*        Parse capturing bracket argument list    *
**************************************************/

/* Reads a list of capture references. The references can be numbers or names.

Returns: updated parsed_pattern pointer on success
         NULL otherwise
*/

pub(crate) unsafe fn parse_capture_list(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    mut parsed_pattern: *mut u32,
    mut offset: PCRE2_SIZE,
    errorcodeptr: *mut i32,
    cb: *mut compile_block,
) -> *mut u32 {
    let mut next_offset: PCRE2_SIZE;
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let mut name: PCRE2_SPTR = core::ptr::null();
    let mut terminator: PCRE2_UCHAR;
    let mut meta: u32;
    let mut namelen: u32 = 0;
    let mut i: i32 = 0;

    'failed: {
        if ptr >= ptrend || *ptr != b'(' {
            *errorcodeptr = ERR118;
            break 'failed; /* goto FAILED */
        }

        loop {
            ptr = ptr.add(1);
            next_offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;

            if ptr >= ptrend {
                *errorcodeptr = ERR117;
                break 'failed; /* goto FAILED */
            }

            /* Handle [+-]number cases */
            if read_number(
                &mut ptr,
                ptrend,
                (*cb).bracount as i32,
                MAX_GROUP_NUMBER,
                ERR61 as u32,
                &mut i,
                errorcodeptr,
            ) != FALSE
            {
                /* PCRE2_ASSERT(i >= 0); */
                if i <= 0 {
                    *errorcodeptr = ERR15;
                    break 'failed; /* goto FAILED */
                }
                meta = META_CAPTURE_NUMBER;
                namelen = i as u32;
            } else if *errorcodeptr != 0 {
                break 'failed; /* goto FAILED - Number too big */
            } else {
                /* Handle 'name' or <name> cases. */
                if *ptr == b'<' {
                    terminator = b'>';
                } else if *ptr == b'\'' {
                    terminator = b'\'';
                } else {
                    *errorcodeptr = ERR117;
                    break 'failed; /* goto FAILED */
                }

                if read_name(
                    &mut ptr,
                    ptrend,
                    utf,
                    terminator as u32,
                    &mut next_offset,
                    &mut name,
                    &mut namelen,
                    errorcodeptr,
                    cb,
                ) == FALSE
                {
                    break 'failed; /* goto FAILED */
                }

                meta = META_CAPTURE_NAME;
            }

            /* PCRE2_ASSERT(next_offset > 0); */
            if offset == 0 || (next_offset.wrapping_sub(offset)) >= 0x10000 {
                *parsed_pattern = META_OFFSET;
                parsed_pattern = parsed_pattern.add(1);
                PUTOFFSET!(next_offset, parsed_pattern);
                offset = next_offset;
            }

            /* The offset is encoded as a relative offset. */
            *parsed_pattern = meta | (next_offset.wrapping_sub(offset) as u32);
            parsed_pattern = parsed_pattern.add(1);
            *parsed_pattern = namelen;
            parsed_pattern = parsed_pattern.add(1);
            offset = next_offset;

            if ptr >= ptrend {
                /* goto UNCLOSED_PARENTHESIS */
                *errorcodeptr = ERR14;
                break 'failed;
            }

            if *ptr == b')' {
                break;
            }

            if *ptr != b',' {
                *errorcodeptr = ERR24;
                break 'failed; /* goto FAILED */
            }
        }

        *ptrptr = ptr.add(1);
        return parsed_pattern;
    }

    /* FAILED: */
    *ptrptr = ptr;
    core::ptr::null_mut()
}



/*************************************************
*          Manage callouts at start of cycle     *
*************************************************/

/* At the start of a new item in parse_regex() we are able to record the
details of the previous item in a prior callout, and also to set up an
automatic callout if enabled.

Returns: possibly updated parsed_pattern pointer.
*/

pub(crate) unsafe fn manage_callouts(
    ptr: PCRE2_SPTR,
    pcalloutptr: *mut *mut u32,
    auto_callout: BOOL,
    mut parsed_pattern: *mut u32,
    cb: *mut compile_block,
) -> *mut u32 {
    let mut previous_callout: *mut u32 = *pcalloutptr;

    if !previous_callout.is_null() {
        *previous_callout.add(2) = ((ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE)
            .wrapping_sub(*previous_callout.add(1) as PCRE2_SIZE)) as u32;
    }

    if auto_callout == FALSE {
        previous_callout = core::ptr::null_mut();
    } else {
        if previous_callout.is_null()
            || previous_callout != parsed_pattern.wrapping_sub(4)
            || *previous_callout.add(3) != 255
        {
            previous_callout = parsed_pattern; /* Set up new automatic callout */
            parsed_pattern = parsed_pattern.add(4);
            *previous_callout.add(0) = META_CALLOUT_NUMBER;
            *previous_callout.add(2) = 0;
            *previous_callout.add(3) = 255;
        }
        *previous_callout.add(1) = ptr.offset_from((*cb).start_pattern) as u32;
    }

    *pcalloutptr = previous_callout;
    parsed_pattern
}



/*************************************************
*          Handle \d, \D, \s, \S, \w, \W         *
*************************************************/

/* This function is called from parse_regex() below, both for freestanding
escapes, and those within classes, to handle those escapes that may change when
Unicode property support is requested.

Returns:          updated value of parsed_pattern
*/

pub(crate) unsafe fn handle_escdsw(
    escape: i32,
    mut parsed_pattern: *mut u32,
    options: u32,
    xoptions: u32,
) -> *mut u32 {
    let mut ascii_option: u32 = 0;
    let mut prop: u32 = ESC_p;

    match escape as u32 {
        ESC_D => {
            prop = ESC_P;
            /* PCRE2_FALLTHROUGH */
            ascii_option = PCRE2_EXTRA_ASCII_BSD;
        }
        ESC_d => {
            ascii_option = PCRE2_EXTRA_ASCII_BSD;
        }

        ESC_S => {
            prop = ESC_P;
            /* PCRE2_FALLTHROUGH */
            ascii_option = PCRE2_EXTRA_ASCII_BSS;
        }
        ESC_s => {
            ascii_option = PCRE2_EXTRA_ASCII_BSS;
        }

        ESC_W => {
            prop = ESC_P;
            /* PCRE2_FALLTHROUGH */
            ascii_option = PCRE2_EXTRA_ASCII_BSW;
        }
        ESC_w => {
            ascii_option = PCRE2_EXTRA_ASCII_BSW;
        }
        _ => {}
    }

    if (options & PCRE2_UCP) == 0 || (xoptions & ascii_option) != 0 {
        *parsed_pattern = META_ESCAPE + escape as u32;
        parsed_pattern = parsed_pattern.add(1);
    } else {
        *parsed_pattern = META_ESCAPE + prop;
        parsed_pattern = parsed_pattern.add(1);
        match escape as u32 {
            ESC_d | ESC_D => {
                *parsed_pattern = (PT_PC << 16) | ucp_Nd;
                parsed_pattern = parsed_pattern.add(1);
            }

            ESC_s | ESC_S => {
                *parsed_pattern = PT_SPACE << 16;
                parsed_pattern = parsed_pattern.add(1);
            }

            ESC_w | ESC_W => {
                *parsed_pattern = PT_WORD << 16;
                parsed_pattern = parsed_pattern.add(1);
            }
            _ => {}
        }
    }

    parsed_pattern
}



/*************************************************
* Maximum size of parsed_pattern for given input *
*************************************************/

/* This function is called from parse_regex() below, to determine the amount
of memory to allocate for parsed_pattern.

Returns:          the number of uint32_t units for parsed_pattern
*/

pub(crate) unsafe fn max_parsed_pattern(
    ptr: PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    options: u32,
) -> isize {
    let big32count: PCRE2_SIZE = 0;
    let mut parsed_size_needed: isize;

    parsed_size_needed = ptrend.offset_from(ptr) + big32count as isize;

    /* When PCRE2_AUTO_CALLOUT is set we have to assume a numerical callout (4
    elements) for each character. */

    if (options & PCRE2_AUTO_CALLOUT) != 0 {
        parsed_size_needed += ptrend.offset_from(ptr) * 4;
    }

    parsed_size_needed
}
