// Translated from pcre2_compile.c lines 1133-3113
use crate::compile_h::*;
use crate::compile_tables::*;
use crate::internal::*;
use crate::pcre2_pub::*;
use crate::tables::*;
use crate::ucd_data::*;
use crate::ucp::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

/*************************************************
*           Copy compiled code                   *
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
    memcpy(
        newcode as *mut c_void,
        code as *const c_void,
        (*code).blocksize,
    );
    (*newcode).executable_jit = core::ptr::null_mut();

    /* If the code is one that has been deserialized, increment the reference count
    in the decoded tables. */

    if ((*code).flags & PCRE2_DEREF_TABLES) != 0 {
        ref_count = (*code).tables.add(TABLES_LENGTH) as *mut PCRE2_SIZE;
        *ref_count = (*ref_count).wrapping_add(1);
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
    memcpy(
        newcode as *mut c_void,
        code as *const c_void,
        (*code).blocksize,
    );
    (*newcode).executable_jit = core::ptr::null_mut();

    newtables = ((*code).memctl.malloc.unwrap())(
        TABLES_LENGTH + core::mem::size_of::<PCRE2_SIZE>(),
        (*code).memctl.memory_data,
    ) as *mut u8;
    if newtables.is_null() {
        ((*code).memctl.free.unwrap())(newcode as *mut c_void, (*code).memctl.memory_data);
        return core::ptr::null_mut();
    }
    memcpy(
        newtables as *mut c_void,
        (*code).tables as *const c_void,
        TABLES_LENGTH,
    );
    ref_count = newtables.add(TABLES_LENGTH) as *mut PCRE2_SIZE;
    *ref_count = 1;

    (*newcode).tables = newtables;
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
                *ref_count = (*ref_count).wrapping_sub(1);
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
    max_value: u32,
    max_error: u32,
    intptr: *mut c_int,
    errorcodeptr: *mut c_int,
) -> BOOL {
    let mut sign: c_int = 0;
    let mut n: u32 = 0;
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let mut yield_: BOOL = FALSE;
    let mut max_value = max_value;

    /* PCRE2_ASSERT(max_value <= INT_MAX/10 - 1); */

    *errorcodeptr = 0;

    'exit: {
        if allow_sign >= 0 && ptr < ptrend {
            if *ptr as u32 == CHAR_PLUS {
                sign = 1;
                max_value = max_value.wrapping_sub(allow_sign as u32);
                ptr = ptr.add(1);
            } else if *ptr as u32 == CHAR_MINUS {
                sign = -1;
                ptr = ptr.add(1);
            }
        }

        if ptr >= ptrend || !IS_DIGIT(*ptr as u32) {
            return FALSE;
        }
        while ptr < ptrend && IS_DIGIT(*ptr as u32) {
            let d = *ptr as u32;
            ptr = ptr.add(1);
            n = n.wrapping_mul(10).wrapping_add(d.wrapping_sub(CHAR_0));
            if n > max_value {
                *errorcodeptr = max_error as c_int;
                while ptr < ptrend && IS_DIGIT(*ptr as u32) {
                    ptr = ptr.add(1);
                }
                break 'exit;
            }
        }

        if allow_sign >= 0 && sign != 0 {
            if n == 0 {
                *errorcodeptr = ERR(26); /* +0 and -0 are not allowed */
                break 'exit;
            }

            if sign > 0 {
                n = n.wrapping_add(allow_sign as u32);
            } else if n > allow_sign as u32 {
                *errorcodeptr = ERR(15); /* Non-existent subpattern */
                break 'exit;
            } else {
                n = (allow_sign as u32).wrapping_add(1).wrapping_sub(n);
            }
        }

        yield_ = TRUE;
    }

    /* EXIT: */
    *intptr = n as c_int;
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
    errorcodeptr: *mut c_int,
) -> BOOL {
    let mut p: PCRE2_SPTR = *ptrptr;
    let mut pp: PCRE2_SPTR;
    let mut yield_: BOOL = FALSE;
    let mut had_minimum: BOOL = FALSE;
    let mut min: c_int = 0;
    let mut max: c_int = REPEAT_UNLIMITED as c_int; /* Larger than MAX_REPEAT_COUNT */

    *errorcodeptr = 0;
    while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
        p = p.add(1);
    }

    /* Check the syntax before interpreting. Otherwise, a non-quantifier sequence
    such as "X{123456ABC" would incorrectly give a "number too big in quantifier"
    error. */

    pp = p;
    if pp < ptrend && IS_DIGIT(*pp as u32) {
        had_minimum = TRUE;
        loop {
            pp = pp.add(1);
            if !(pp < ptrend && IS_DIGIT(*pp as u32)) {
                break;
            }
        }
    }

    while pp < ptrend && (*pp as u32 == CHAR_SPACE || *pp as u32 == CHAR_HT) {
        pp = pp.add(1);
    }
    if pp >= ptrend {
        return FALSE;
    }

    if *pp as u32 == CHAR_RIGHT_CURLY_BRACKET {
        if had_minimum == FALSE {
            return FALSE;
        }
    } else {
        let v = *pp;
        pp = pp.add(1);
        if v as u32 != CHAR_COMMA {
            return FALSE;
        }
        while pp < ptrend && (*pp as u32 == CHAR_SPACE || *pp as u32 == CHAR_HT) {
            pp = pp.add(1);
        }
        if pp >= ptrend {
            return FALSE;
        }
        if IS_DIGIT(*pp as u32) {
            loop {
                pp = pp.add(1);
                if !(pp < ptrend && IS_DIGIT(*pp as u32)) {
                    break;
                }
            }
        } else if had_minimum == FALSE {
            return FALSE;
        }
        while pp < ptrend && (*pp as u32 == CHAR_SPACE || *pp as u32 == CHAR_HT) {
            pp = pp.add(1);
        }
        if pp >= ptrend || *pp as u32 != CHAR_RIGHT_CURLY_BRACKET {
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
            ERR(5) as u32,
            &mut min,
            errorcodeptr,
        ) == FALSE
        {
            if *errorcodeptr != 0 {
                break 'exit;
            } /* n too big */
            p = p.add(1); /* Skip comma and subsequent spaces */
            while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                p = p.add(1);
            }
            if read_number(
                &mut p,
                ptrend,
                -1,
                MAX_REPEAT_COUNT,
                ERR(5) as u32,
                &mut max,
                errorcodeptr,
            ) == FALSE
            {
                if *errorcodeptr != 0 {
                    break 'exit;
                } /* m too big */
            }
        }
        /* Have read one number. Deal with {n} or {n,} or {n,m} */
        else {
            while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                p = p.add(1);
            }
            if *p as u32 == CHAR_RIGHT_CURLY_BRACKET {
                max = min;
            } else
            /* Handle {n,} or {n,m} */
            {
                p = p.add(1); /* Skip comma and subsequent spaces */
                while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                    p = p.add(1);
                }
                if read_number(
                    &mut p,
                    ptrend,
                    -1,
                    MAX_REPEAT_COUNT,
                    ERR(5) as u32,
                    &mut max,
                    errorcodeptr,
                ) == FALSE
                {
                    if *errorcodeptr != 0 {
                        break 'exit;
                    } /* m too big */
                }

                if max < min {
                    *errorcodeptr = ERR(4);
                    break 'exit;
                }
            }
        }

        /* Valid quantifier exists */

        while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_check_escape_8(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    chptr: *mut u32,
    errorcodeptr: *mut c_int,
    options: u32,
    xoptions: u32,
    bracount: u32,
    isclass: BOOL,
    cb: *mut compile_block,
) -> c_int {
    let utf: BOOL = ((options & PCRE2_UTF) != 0) as BOOL;
    let mut alt_bsux: BOOL =
        (((options & PCRE2_ALT_BSUX) | (xoptions & PCRE2_EXTRA_ALT_BSUX)) != 0) as BOOL;
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let mut c: u32;
    let mut cc: u32 = 0;
    let mut escape: c_int = 0;
    let mut i: c_int = 0;

    /* Variables hoisted out of the inner block so that the COME_FROM_NU code can
    be shared (see below). */
    let mut s: c_int = 0;
    let mut oldptr: PCRE2_SPTR = core::ptr::null();
    let mut overflow: BOOL = FALSE;

    /* If backslash is at the end of the string, it's an error. */

    if ptr >= ptrend {
        *errorcodeptr = ERR(1);
        return 0;
    }

    /* GETCHARINCTEST(c, ptr) */
    c = *ptr as u32;
    ptr = ptr.add(1);
    if utf != 0 && c >= 0xc0 {
        let r = getutf8inc(c, ptr);
        c = r.0;
        ptr = r.1;
    }
    *errorcodeptr = 0; /* Be optimistic */

    'exit_blk: {
        'esc_failed_fwd: {
            'come_from_nu: {
                /* Non-alphanumerics are literals, so we just leave the value in c. An
                initial value test saves a memory lookup for code points outside the
                alphanumeric range. */

                if c < ESCAPES_FIRST || c > ESCAPES_LAST { /* Definitely literal */
                } else {
                    i = escapes[(c - ESCAPES_FIRST) as usize] as c_int;
                    if i != 0 {
                        if i > 0 {
                            c = i as u32;
                            if c == CHAR_CR && (xoptions & PCRE2_EXTRA_ESCAPED_CR_IS_LF) != 0 {
                                c = CHAR_LF;
                            }
                        } else
                        /* Negative table entry */
                        {
                            escape = -i; /* Else return a special escape */
                            if !cb.is_null()
                                && (escape == ESC_P || escape == ESC_p || escape == ESC_X)
                            {
                                (*cb).external_flags |= PCRE2_HASBKPORX; /* Note \P, \p, or \X */
                            }

                            /* Perl supports \N{name} for character names and \N{U+dddd} for
                            numerical Unicode code points, as well as plain \N for "not
                            newline". PCRE does not support \N{name}. However, it does support
                            quantification such as \N{2,3}, so if \N{ is not followed by U+dddd
                            we check for a quantifier. */

                            if escape == ESC_N
                                && ptr < ptrend
                                && *ptr as u32 == CHAR_LEFT_CURLY_BRACKET
                            {
                                let mut p: PCRE2_SPTR = ptr.add(1);

                                /* Perl ignores spaces and tabs after { */

                                while p < ptrend
                                    && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT)
                                {
                                    p = p.add(1);
                                }

                                /* \N{U+ can be handled by the \x{ code. Also, in Perl, \N{U+
                                forces Unicode casing semantics for the entire pattern, so allow
                                it only in UTF (i.e. Unicode) mode. */

                                if ptrend.offset_from(p) > 1
                                    && *p as u32 == CHAR_U
                                    && *p.add(1) as u32 == CHAR_PLUS
                                {
                                    if utf != 0 {
                                        ptr = p.add(2);
                                        escape = 0; /* Not a fancy escape after all */
                                        break 'come_from_nu;
                                    }

                                    /* Improve error offset. */
                                    ptr = p.add(2);
                                    while ptr < ptrend && XDIGIT(*ptr as u32) != 0xff {
                                        ptr = ptr.add(1);
                                    }
                                    while ptr < ptrend
                                        && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT)
                                    {
                                        ptr = ptr.add(1);
                                    }
                                    if ptr < ptrend && *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                                        ptr = ptr.add(1);
                                    }

                                    *errorcodeptr = ERR(93);
                                }
                                /* Give an error in contexts where quantifiers are not allowed
                                (character classes; substitution strings). */
                                else if isclass != 0 || cb.is_null() {
                                    ptr = ptr.add(1); /* Skip over the opening brace */
                                    *errorcodeptr = ERR(37);
                                }
                                /* Give an error if what follows is not a quantifier, but don't
                                override an error set by the quantifier reader (e.g. number
                                overflow). */
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
                                        *errorcodeptr = ERR(37);
                                    }
                                }
                            }
                        }
                    }
                    /* Escapes that need further processing, including those that are
                    unknown, have a zero entry in the lookup table. When called from
                    pcre2_substitute(), only \c, \o, and \x are recognized (\u and \U can
                    never appear as they are used for case forcing). */
                    else {
                        /* Filter calls from pcre2_substitute(). */

                        if cb.is_null() {
                            if !(c >= CHAR_0 && c <= CHAR_9)
                                && c != CHAR_c
                                && c != CHAR_o
                                && c != CHAR_x
                                && c != CHAR_g
                            {
                                *errorcodeptr = ERR(3);
                                break 'exit_blk;
                            }
                            alt_bsux = FALSE; /* Do not modify \x handling */
                        }

                        'sw: {
                            'octal: {
                                match c {
                                    /* A number of Perl escapes are not handled by PCRE. We give
                                    an explicit error. */
                                    CHAR_F | CHAR_l | CHAR_L => {
                                        *errorcodeptr = ERR(37);
                                        break 'sw;
                                    }

                                    /* \u is unrecognized when neither PCRE2_ALT_BSUX nor
                                    PCRE2_EXTRA_ALT_BSUX is set. Otherwise, \u must be followed
                                    by exactly four hex digits or, if PCRE2_EXTRA_ALT_BSUX is
                                    set, by any number of hex digits in braces. */
                                    CHAR_u => {
                                        if alt_bsux == FALSE {
                                            *errorcodeptr = ERR(37);
                                        } else {
                                            let mut xc: u32;

                                            if ptr >= ptrend {
                                                break 'sw;
                                            }
                                            if *ptr as u32 == CHAR_LEFT_CURLY_BRACKET
                                                && (xoptions & PCRE2_EXTRA_ALT_BSUX) != 0
                                            {
                                                let mut hptr: PCRE2_SPTR = ptr.add(1);

                                                cc = 0;
                                                loop {
                                                    if !(hptr < ptrend) {
                                                        break;
                                                    }
                                                    xc = XDIGIT(*hptr as u32);
                                                    if xc == 0xff {
                                                        break;
                                                    }
                                                    if (cc & 0xf0000000) != 0 {
                                                        /* Test for 32-bit overflow */
                                                        *errorcodeptr = ERR(77);
                                                        ptr = hptr; /* Show where */
                                                        break; /* *hptr != } will cause another break below */
                                                    }
                                                    cc = (cc << 4) | xc;
                                                    hptr = hptr.add(1);
                                                }

                                                if hptr == ptr.add(1)
                                                    || /* No hex digits */
                                                   hptr >= ptrend
                                                    || /* Hit end of input */
                                                   *hptr as u32 != CHAR_RIGHT_CURLY_BRACKET
                                                /* No } terminator */
                                                {
                                                    if isclass != 0 {
                                                        break 'sw;
                                                    } /* In a class, just treat as '\u' literal */
                                                    escape = ESC_ub; /* Special return */
                                                    ptr = ptr.add(1); /* Skip { */
                                                    break 'sw; /* Hex escape not recognized */
                                                }

                                                c = cc; /* Accept the code point */
                                                ptr = hptr.add(1);
                                            } else
                                            /* Must be exactly 4 hex digits */
                                            {
                                                if ptrend.offset_from(ptr) < 4 {
                                                    break 'sw;
                                                } /* Less than 4 chars */
                                                cc = XDIGIT(*ptr.add(0) as u32);
                                                if cc == 0xff {
                                                    break 'sw;
                                                } /* Not a hex digit */
                                                xc = XDIGIT(*ptr.add(1) as u32);
                                                if xc == 0xff {
                                                    break 'sw;
                                                } /* Not a hex digit */
                                                cc = (cc << 4) | xc;
                                                xc = XDIGIT(*ptr.add(2) as u32);
                                                if xc == 0xff {
                                                    break 'sw;
                                                } /* Not a hex digit */
                                                cc = (cc << 4) | xc;
                                                xc = XDIGIT(*ptr.add(3) as u32);
                                                if xc == 0xff {
                                                    break 'sw;
                                                } /* Not a hex digit */
                                                c = (cc << 4) | xc;
                                                ptr = ptr.add(4);
                                            }

                                            if utf != 0 {
                                                if c > 0x10ffff {
                                                    *errorcodeptr = ERR(77);
                                                } else if c >= 0xd800
                                                    && c <= 0xdfff
                                                    && (xoptions
                                                        & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES)
                                                        == 0
                                                {
                                                    *errorcodeptr = ERR(73);
                                                }
                                            } else if c > MAX_NON_UTF_CHAR {
                                                *errorcodeptr = ERR(77);
                                            }
                                        }
                                        break 'sw;
                                    }

                                    /* \U is unrecognized unless PCRE2_ALT_BSUX or
                                    PCRE2_EXTRA_ALT_BSUX is set, in which case it is an upper
                                    case letter. */
                                    CHAR_U => {
                                        if alt_bsux == FALSE {
                                            *errorcodeptr = ERR(37);
                                        }
                                        break 'sw;
                                    }

                                    /* In a character class, \g is just a literal "g". Outside a
                                    character class, \g must be followed by one of a number of
                                    specific things. */
                                    CHAR_g => {
                                        if isclass != 0 {
                                            break 'sw;
                                        }

                                        if ptr >= ptrend {
                                            *errorcodeptr = ERR(57);
                                            break 'sw;
                                        }

                                        if cb.is_null() {
                                            let mut p: PCRE2_SPTR;
                                            /* Substitution strings */
                                            if *ptr as u32 != CHAR_LESS_THAN_SIGN {
                                                *errorcodeptr = ERR(57);
                                                break 'sw;
                                            }

                                            p = ptr.add(1);

                                            if read_number(
                                                &mut p,
                                                ptrend,
                                                -1,
                                                MAX_GROUP_NUMBER,
                                                ERR(61) as u32,
                                                &mut s,
                                                errorcodeptr,
                                            ) == FALSE
                                            {
                                                if *errorcodeptr == 0 {
                                                    escape = ESC_g;
                                                } /* No number found */
                                                break 'sw;
                                            }

                                            if p >= ptrend || *p as u32 != CHAR_GREATER_THAN_SIGN {
                                                ptr = p;
                                                *errorcodeptr = ERR(119); /* Missing terminator for number */
                                                break 'sw;
                                            }

                                            /* This is the reason that back references are
                                            returned as -(s+1) rather than just -s. */
                                            ptr = p.add(1);
                                            escape = -(s + 1);
                                            break 'sw;
                                        }

                                        if *ptr as u32 == CHAR_LESS_THAN_SIGN
                                            || *ptr as u32 == CHAR_APOSTROPHE
                                        {
                                            escape = ESC_g;
                                            break 'sw;
                                        }

                                        /* If there is a brace delimiter, try to read a numerical
                                        reference. If there isn't one, assume we have a name and
                                        treat it as \k. */

                                        if *ptr as u32 == CHAR_LEFT_CURLY_BRACKET {
                                            let mut p: PCRE2_SPTR = ptr.add(1);

                                            while p < ptrend
                                                && (*p as u32 == CHAR_SPACE
                                                    || *p as u32 == CHAR_HT)
                                            {
                                                p = p.add(1);
                                            }
                                            if read_number(
                                                &mut p,
                                                ptrend,
                                                bracount as i32,
                                                MAX_GROUP_NUMBER,
                                                ERR(61) as u32,
                                                &mut s,
                                                errorcodeptr,
                                            ) == FALSE
                                            {
                                                if *errorcodeptr == 0 {
                                                    escape = ESC_k;
                                                } /* No number found */
                                                break 'sw;
                                            }
                                            while p < ptrend
                                                && (*p as u32 == CHAR_SPACE
                                                    || *p as u32 == CHAR_HT)
                                            {
                                                p = p.add(1);
                                            }

                                            if p >= ptrend || *p as u32 != CHAR_RIGHT_CURLY_BRACKET
                                            {
                                                ptr = p;
                                                *errorcodeptr = ERR(119); /* Missing terminator for number */
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
                                                ERR(61) as u32,
                                                &mut s,
                                                errorcodeptr,
                                            ) == FALSE
                                            {
                                                if *errorcodeptr == 0 {
                                                    *errorcodeptr = ERR(57);
                                                } /* No number found */
                                                break 'sw;
                                            }
                                        }

                                        if s <= 0 {
                                            *errorcodeptr = ERR(15);
                                            break 'sw;
                                        }

                                        escape = -(s + 1);
                                        break 'sw;
                                    }

                                    /* The handling of escape sequences consisting of a string of
                                    digits starting with one that is not zero is not
                                    straightforward. */
                                    CHAR_1 | CHAR_2 | CHAR_3 | CHAR_4 | CHAR_5 | CHAR_6
                                    | CHAR_7 | CHAR_8 | CHAR_9 => {
                                        if isclass != 0 {
                                            /* Fall through to octal handling; never a
                                            backreference inside a class. */
                                        } else if (xoptions & PCRE2_EXTRA_PYTHON_OCTAL) != 0 {
                                            /* Python-style disambiguation. */
                                            if (*ptr.offset(-1) as u32) <= CHAR_7
                                                && ptr.add(1) < ptrend
                                                && (*ptr.add(0) as u32) >= CHAR_0
                                                && (*ptr.add(0) as u32) <= CHAR_7
                                                && (*ptr.add(1) as u32) >= CHAR_0
                                                && (*ptr.add(1) as u32) <= CHAR_7
                                            {
                                                /* We peeked a three-digit octal, so fall through */
                                            } else {
                                                /* We are at a digit, so the only possible error
                                                from read_number() is a number that is too
                                                large. */
                                                ptr = ptr.offset(-1); /* Back to the digit */

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
                                                    *errorcodeptr = ERR(61);
                                                    break 'sw;
                                                }

                                                escape = -(s + 1);
                                                break 'sw;
                                            }
                                        } else {
                                            /* Perl-style disambiguation. */
                                            oldptr = ptr;
                                            ptr = ptr.offset(-1); /* Back to the digit */

                                            /* As we know we are at a digit, the only possible
                                            error from read_number() is a number that is too large
                                            to be a group number. */

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
                                                s = c_int::MAX;
                                            }

                                            /* \1 to \9 are always back references. \8x and \9x
                                            are too; \1x to \7x are octal escapes if there are not
                                            that many previous captures. */

                                            if s < 10 || c >= CHAR_8 || (s as c_uint) <= bracount {
                                                /* s > MAX_GROUP_NUMBER should not be possible
                                                because of read_number(), but we keep it just to be
                                                safe. */

                                                if (s as c_uint) > MAX_GROUP_NUMBER {
                                                    *errorcodeptr = ERR(61);
                                                } else {
                                                    escape = -(s + 1); /* Indicates a back reference */
                                                }
                                                break 'sw;
                                            }

                                            ptr = oldptr; /* Put the pointer back and fall through */
                                        }

                                        /* Handle a digit following \ when the number is not a
                                        back reference, or we are within a character class. */

                                        if c >= CHAR_8 {
                                            break 'sw;
                                        }

                                        break 'octal; /* Fall through */
                                    }

                                    /* \0 always starts an octal number, but we may drop through to
                                    here with a larger first octal digit. */
                                    CHAR_0 => {
                                        break 'octal;
                                    }

                                    /* \o is a relatively new Perl feature, supporting a more
                                    general way of specifying character codes in octal. The only
                                    supported form is \o{ddd}. */
                                    CHAR_o => {
                                        if ptr >= ptrend
                                            || *ptr as u32 != CHAR_LEFT_CURLY_BRACKET
                                        {
                                            *errorcodeptr = ERR(55);
                                            break 'sw;
                                        }
                                        ptr = ptr.add(1);

                                        while ptr < ptrend
                                            && (*ptr as u32 == CHAR_SPACE
                                                || *ptr as u32 == CHAR_HT)
                                        {
                                            ptr = ptr.add(1);
                                        }
                                        if ptr >= ptrend || *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET
                                        {
                                            *errorcodeptr = ERR(78);
                                            break 'sw;
                                        }

                                        c = 0;
                                        overflow = FALSE;
                                        while ptr < ptrend
                                            && (*ptr as u32) >= CHAR_0
                                            && (*ptr as u32) <= CHAR_7
                                        {
                                            cc = *ptr as u32;
                                            ptr = ptr.add(1);
                                            if c == 0 && cc == CHAR_0 {
                                                continue;
                                            } /* Leading zeroes */
                                            c = (c << 3) + (cc - CHAR_0);
                                            if c > (if utf != 0 { 0x10ffff } else { 0xff }) {
                                                overflow = TRUE;
                                                break;
                                            }
                                        }

                                        while ptr < ptrend
                                            && (*ptr as u32 == CHAR_SPACE
                                                || *ptr as u32 == CHAR_HT)
                                        {
                                            ptr = ptr.add(1);
                                        }

                                        if overflow != FALSE {
                                            while ptr < ptrend
                                                && (*ptr as u32) >= CHAR_0
                                                && (*ptr as u32) <= CHAR_7
                                            {
                                                ptr = ptr.add(1);
                                            }
                                            *errorcodeptr = ERR(34);
                                        } else if utf != 0
                                            && c >= 0xd800
                                            && c <= 0xdfff
                                            && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0
                                        {
                                            *errorcodeptr = ERR(73);
                                        } else if ptr < ptrend
                                            && *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET
                                        {
                                            ptr = ptr.add(1);
                                        } else {
                                            *errorcodeptr = ERR(64);
                                            break 'esc_failed_fwd;
                                        }
                                        break 'sw;
                                    }

                                    /* When PCRE2_ALT_BSUX or PCRE2_EXTRA_ALT_BSUX is set, \x must
                                    be followed by two hexadecimal digits. Otherwise it is a
                                    lowercase x letter. */
                                    CHAR_x => {
                                        if alt_bsux != FALSE {
                                            let xc: u32;
                                            if ptrend.offset_from(ptr) < 2 {
                                                break 'sw;
                                            } /* Less than 2 characters */
                                            cc = XDIGIT(*ptr.add(0) as u32);
                                            if cc == 0xff {
                                                break 'sw;
                                            } /* Not a hex digit */
                                            xc = XDIGIT(*ptr.add(1) as u32);
                                            if xc == 0xff {
                                                break 'sw;
                                            } /* Not a hex digit */
                                            c = (cc << 4) | xc;
                                            ptr = ptr.add(2);
                                        }
                                        /* Handle \x in Perl's style. */
                                        else {
                                            if ptr < ptrend
                                                && *ptr as u32 == CHAR_LEFT_CURLY_BRACKET
                                            {
                                                ptr = ptr.add(1);
                                                while ptr < ptrend
                                                    && (*ptr as u32 == CHAR_SPACE
                                                        || *ptr as u32 == CHAR_HT)
                                                {
                                                    ptr = ptr.add(1);
                                                }

                                                break 'come_from_nu;
                                            }
                                            /* Read a up to two hex digits after \x */
                                            else {
                                                /* Perl has the surprising/broken behaviour that \x
                                                without following hex digits is treated as an
                                                escape for NUL. Because we don't have warnings, we
                                                simply forbid it. */
                                                if ptr >= ptrend {
                                                    *errorcodeptr = ERR(78);
                                                    break 'sw;
                                                }
                                                cc = XDIGIT(*ptr as u32);
                                                if cc == 0xff {
                                                    /* Not a hex digit */
                                                    *errorcodeptr = ERR(78);
                                                    break 'sw;
                                                }
                                                ptr = ptr.add(1);
                                                c = cc;

                                                if ptr >= ptrend {
                                                    break 'sw;
                                                }
                                                cc = XDIGIT(*ptr as u32);
                                                if cc == 0xff {
                                                    break 'sw;
                                                } /* Not a hex digit */
                                                ptr = ptr.add(1);
                                                c = (c << 4) | cc;
                                            } /* End of \xdd handling */
                                        } /* End of Perl-style \x handling */
                                        break 'sw;
                                    }

                                    /* The handling of \c is different in ASCII and EBCDIC
                                    environments. In an ASCII (or Unicode) environment, an error
                                    is given if the character following \c is not a printable
                                    ASCII character. */
                                    CHAR_c => {
                                        if ptr >= ptrend {
                                            *errorcodeptr = ERR(2);
                                            break 'sw;
                                        }
                                        c = *ptr as u32;
                                        if c >= CHAR_a && c <= CHAR_z {
                                            c = UPPER_CASE(c);
                                        }

                                        /* Handle \c in an ASCII/Unicode environment. */

                                        if c < 32 || c > 126
                                        /* Excludes all non-printable ASCII */
                                        {
                                            *errorcodeptr = ERR(68);
                                            break 'esc_failed_fwd;
                                        }
                                        c ^= 0x40;

                                        ptr = ptr.add(1);
                                        break 'sw;
                                    }

                                    /* Any other alphanumeric following \ is an error. */
                                    _ => {
                                        *errorcodeptr = ERR(3);
                                        break 'sw;
                                    }
                                }
                            }

                            /* case CHAR_0: (also reached by fall-through from the digits) */
                            c -= CHAR_0;
                            while {
                                let t = i < 2;
                                i += 1;
                                t
                            } && ptr < ptrend
                                && (*ptr as u32) >= CHAR_0
                                && (*ptr as u32) <= CHAR_7
                            {
                                let d = *ptr as u32;
                                ptr = ptr.add(1);
                                c = c * 8 + d - CHAR_0;
                            }
                            if c > 0xff {
                                if (xoptions & PCRE2_EXTRA_PYTHON_OCTAL) != 0 {
                                    *errorcodeptr = ERR(102);
                                } else if utf == 0 {
                                    *errorcodeptr = ERR(51);
                                }
                            }

                            /* PCRE2_EXTRA_NO_BS0 disables the NUL escape '\0' but doesn't affect
                            two- or three-character octal escapes \00 and \000, nor \x00. */

                            if (xoptions & PCRE2_EXTRA_NO_BS0) != 0 && c == 0 && i == 1 {
                                *errorcodeptr = ERR(98);
                            }
                            break 'sw;
                        }
                    }
                }

                break 'exit_blk;
            }

            /* COME_FROM_NU: shared \x{...} hex processing */

            if ptr >= ptrend || *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                *errorcodeptr = ERR(78);
                break 'exit_blk;
            }
            c = 0;
            overflow = FALSE;

            loop {
                if !(ptr < ptrend) {
                    break;
                }
                cc = XDIGIT(*ptr as u32);
                if cc == 0xff {
                    break;
                }
                ptr = ptr.add(1);
                if c == 0 && cc == 0 {
                    continue;
                } /* Leading zeroes */
                c = (c << 4) | cc;
                if (utf != 0 && c > 0x10ffff) || (utf == 0 && c > MAX_NON_UTF_CHAR) {
                    overflow = TRUE;
                    break;
                }
            }

            /* Perl ignores spaces and tabs before } */

            while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                ptr = ptr.add(1);
            }

            /* On overflow, skip remaining hex digits */

            if overflow != FALSE {
                while ptr < ptrend && XDIGIT(*ptr as u32) != 0xff {
                    ptr = ptr.add(1);
                }
                *errorcodeptr = ERR(34);
            } else if utf != 0
                && c >= 0xd800
                && c <= 0xdfff
                && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES) == 0
            {
                *errorcodeptr = ERR(73);
            } else if ptr < ptrend && *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                ptr = ptr.add(1);
            }
            /* If the sequence of hex digits (followed by optional space) does not end
            with '}', give an error. */
            else {
                *errorcodeptr = ERR(67);
                break 'esc_failed_fwd;
            }

            break 'exit_blk;
        }

        /* ESCAPE_FAILED_FORWARD: Some errors need to indicate the next character. */

        ptr = ptr.add(1);
        if utf != 0 {
            /* FORWARDCHARTEST(ptr, ptrend) */
            while ptr < ptrend && NOT_FIRSTCU(*ptr as u32) {
                ptr = ptr.add(1);
            }
        }
    }

    /* EXIT: Set the pointer to the next character before returning. */

    *ptrptr = ptr;
    *chptr = c;
    escape
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
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    let mut c: u32;
    let mut i: isize;
    let mut bot: PCRE2_SIZE;
    let mut top: PCRE2_SIZE;
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let mut name: [PCRE2_UCHAR; 50] = [0; 50];
    let mut vptr: *mut PCRE2_UCHAR = core::ptr::null_mut();
    let mut ptscript: u16 = PT_NOTSCRIPT as u16;

    'error_return: {
        if ptr >= (*cb).end_pattern {
            break 'error_return;
        }
        /* GETCHARINCTEST(c, ptr) */
        c = *ptr as u32;
        ptr = ptr.add(1);
        if utf != 0 && c >= 0xc0 {
            let r = getutf8inc(c, ptr);
            c = r.0;
            ptr = r.1;
        }
        *negptr = FALSE;

        /* \P or \p can be followed by a name in {}, optionally preceded by ^ for
        negation. In accordance with Unicode's "loose matching" rules, ASCII white
        space, hyphens, and underscores are ignored. */

        if c == CHAR_LEFT_CURLY_BRACKET {
            if ptr >= (*cb).end_pattern {
                break 'error_return;
            }

            i = 0;
            while i < (50 - 1) as isize {
                /* REDO: */
                'redo: loop {
                    if ptr >= (*cb).end_pattern {
                        break 'error_return;
                    }
                    /* GETCHARINCTEST(c, ptr) */
                    c = *ptr as u32;
                    ptr = ptr.add(1);
                    if utf != 0 && c >= 0xc0 {
                        let r = getutf8inc(c, ptr);
                        c = r.0;
                        ptr = r.1;
                    }

                    /* Skip ignorable Unicode characters. */

                    if c == CHAR_UNDERSCORE
                        || c == CHAR_MINUS
                        || c == CHAR_SPACE
                        || (c >= CHAR_HT && c <= CHAR_CR)
                    {
                        continue 'redo;
                    }

                    /* The first significant character being circumflex negates the
                    meaning of the item. */

                    if i == 0 && *negptr == FALSE && c == CHAR_CIRCUMFLEX_ACCENT {
                        *negptr = TRUE;
                        continue 'redo;
                    }

                    break;
                }

                if c == CHAR_RIGHT_CURLY_BRACKET {
                    break;
                }

                /* Names consist of ASCII letters and digits, but equals and colon may
                also occur as a name/value separator. We must also allow for \p{L&}. */

                if c < CHAR_AMPERSAND || c > CHAR_z {
                    break 'error_return;
                }

                /* Lower case a capital letter or remember where the name/value
                separator is. */

                if c >= CHAR_A && c <= CHAR_Z {
                    c |= 0x20;
                } else if (c == CHAR_COLON || c == CHAR_EQUALS_SIGN) && vptr.is_null() {
                    vptr = name.as_mut_ptr().offset(i);
                }

                name[i as usize] = c as PCRE2_UCHAR;

                i += 1;
            }

            /* Error if the loop didn't end with '}' - either we hit the end of the
            pattern or the name was longer than any legal property name. */

            if c != CHAR_RIGHT_CURLY_BRACKET {
                break 'error_return;
            }
            name[i as usize] = 0;
        }
        /* If { doesn't follow \p or \P there is just one following character, which
        must be an ASCII letter. */
        else if c >= CHAR_A && c <= CHAR_Z {
            name[0] = (c | 0x20) as PCRE2_UCHAR; /* Lower case */
            name[1] = 0;
            i = 0;
        } else if c >= CHAR_a && c <= CHAR_z {
            name[0] = c as PCRE2_UCHAR;
            name[1] = 0;
            i = 0;
        } else {
            break 'error_return;
        }

        *ptrptr = ptr; /* Update pattern pointer */

        /* If the property contains ':' or '=' we have class name and value separately
        specified. */

        if !vptr.is_null() {
            let mut offset: c_int = 0;
            let mut sname: [PCRE2_UCHAR; 8] = [0; 8];

            *vptr = 0; /* Terminate property name */
            if crate::string_utils::_pcre2_strcmp_c8_8(
                name.as_ptr(),
                b"bidiclass\0".as_ptr() as *const c_char,
            ) == 0
                || crate::string_utils::_pcre2_strcmp_c8_8(
                    name.as_ptr(),
                    b"bc\0".as_ptr() as *const c_char,
                ) == 0
            {
                offset = 4;
                sname[0] = CHAR_b as PCRE2_UCHAR;
                sname[1] = CHAR_i as PCRE2_UCHAR; /* There is no strcpy_c8 function */
                sname[2] = CHAR_d as PCRE2_UCHAR;
                sname[3] = CHAR_i as PCRE2_UCHAR;
            } else if crate::string_utils::_pcre2_strcmp_c8_8(
                name.as_ptr(),
                b"script\0".as_ptr() as *const c_char,
            ) == 0
                || crate::string_utils::_pcre2_strcmp_c8_8(
                    name.as_ptr(),
                    b"sc\0".as_ptr() as *const c_char,
                ) == 0
            {
                ptscript = PT_SC as u16;
            } else if crate::string_utils::_pcre2_strcmp_c8_8(
                name.as_ptr(),
                b"scriptextensions\0".as_ptr() as *const c_char,
            ) == 0
                || crate::string_utils::_pcre2_strcmp_c8_8(
                    name.as_ptr(),
                    b"scx\0".as_ptr() as *const c_char,
                ) == 0
            {
                ptscript = PT_SCX as u16;
            } else {
                *errorcodeptr = ERR(47);
                return FALSE;
            }

            /* Adjust the string in name[] as needed */

            memmove(
                name.as_mut_ptr().offset(offset as isize) as *mut c_void,
                vptr.add(1) as *const c_void,
                (name.as_mut_ptr().offset(i).offset_from(vptr)) as usize,
            );
            if offset != 0 {
                memmove(
                    name.as_mut_ptr() as *mut c_void,
                    sname.as_ptr() as *const c_void,
                    offset as usize,
                );
            }
        }

        /* Search for a recognized property using binary chop. */

        bot = 0;
        top = _pcre2_utt_size_8;

        while bot < top {
            let r: c_int;
            i = ((bot + top) >> 1) as isize;
            r = crate::string_utils::_pcre2_strcmp_c8_8(
                name.as_ptr(),
                _pcre2_utt_names_8
                    .as_ptr()
                    .add(_pcre2_utt_8[i as usize].name_offset as usize) as *const c_char,
            );

            /* When a matching property is found, some extra checking is needed when
            the \p{xx:yy} syntax is used and xx is either sc or scx. */

            if r == 0 {
                *pdataptr = _pcre2_utt_8[i as usize].value;
                if vptr.is_null() || ptscript == PT_NOTSCRIPT as u16 {
                    *ptypeptr = _pcre2_utt_8[i as usize].type_;
                    return TRUE;
                }

                match _pcre2_utt_8[i as usize].type_ as u32 {
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

        *errorcodeptr = ERR(47); /* Unrecognized property */
        return FALSE;
    }

    /* ERROR_RETURN: Malformed \P or \p */
    *errorcodeptr = ERR(46);
    *ptrptr = ptr;
    FALSE
}

/*************************************************
*           Check for POSIX class syntax         *
*************************************************/

/* This function is called when the sequence "[:" or "[." or "[=" is
encountered in a character class. It checks whether this is followed by a
sequence of characters terminated by a matching ":]" or ".]" or "=]".

Returns:   TRUE or FALSE
*/

pub(crate) unsafe fn check_posix_syntax(
    ptr: PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    endptr: *mut PCRE2_SPTR,
) -> BOOL {
    let mut ptr = ptr;
    let terminator: PCRE2_UCHAR; /* Don't combine these lines; the Solaris cc */
    terminator = *ptr; /* compiler warns about "non-constant" initializer. */
    ptr = ptr.add(1);

    while ptrend.offset_from(ptr) >= 2 {
        if *ptr as u32 == CHAR_BACKSLASH
            && (*ptr.add(1) as u32 == CHAR_RIGHT_SQUARE_BRACKET
                || *ptr.add(1) as u32 == CHAR_BACKSLASH)
        {
            ptr = ptr.add(1);
        } else if (*ptr as u32 == CHAR_LEFT_SQUARE_BRACKET && *ptr.add(1) == terminator)
            || *ptr as u32 == CHAR_RIGHT_SQUARE_BRACKET
        {
            return FALSE;
        } else if *ptr == terminator && *ptr.add(1) as u32 == CHAR_RIGHT_SQUARE_BRACKET {
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

pub(crate) unsafe fn check_posix_name(ptr: PCRE2_SPTR, len: c_int) -> c_int {
    let mut pn: *const c_char = posix_names.as_ptr() as *const c_char;
    let mut yield_: c_int = 0;
    while posix_name_lengths[yield_ as usize] != 0 {
        if len == posix_name_lengths[yield_ as usize] as c_int
            && crate::string_utils::_pcre2_strncmp_c8_8(ptr, pn, len as c_uint as PCRE2_SIZE) == 0
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
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let is_group: BOOL;
    let is_braced: BOOL;
    {
        let v = *ptr;
        ptr = ptr.add(1);
        is_group = ((v as u32) != CHAR_ASTERISK) as BOOL;
    }
    is_braced = (terminator == CHAR_RIGHT_CURLY_BRACKET) as BOOL;

    'failed: {
        if is_braced != 0 {
            while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                ptr = ptr.add(1);
            }
        }

        if ptr >= ptrend
        /* No characters in name */
        {
            *errorcodeptr = if is_group != 0 {
                ERR(62) /* Subpattern name expected */
            } else {
                ERR(60) /* Verb not recognized or malformed */
            };
            break 'failed;
        }

        *nameptr = ptr;
        *offsetptr = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;

        /* In UTF mode, a group name may contain letters and decimal digits as
        defined by Unicode properties, and underscores, but must not start with a
        digit. */

        if utf != 0 && is_group != 0 {
            let mut c: u32;
            let mut type_: u32;
            let mut p: PCRE2_SPTR = ptr;

            /* GETCHARINC(c, p) -- Peek at next character */
            c = *p as u32;
            p = p.add(1);
            if c >= 0xc0 {
                let r = getutf8inc(c, p);
                c = r.0;
                p = r.1;
            }
            type_ = UCD_CHARTYPE(c);

            if type_ == ucp_Nd {
                ptr = p;
                *errorcodeptr = ERR(44);
                break 'failed;
            }

            loop {
                if type_ != ucp_Nd
                    && _pcre2_ucp_gentype_8[type_ as usize] != ucp_L
                    && c != CHAR_UNDERSCORE
                {
                    break;
                }
                ptr = p; /* Accept character and peek again */
                if p >= ptrend {
                    break;
                }
                /* GETCHARINC(c, p) */
                c = *p as u32;
                p = p.add(1);
                if c >= 0xc0 {
                    let r = getutf8inc(c, p);
                    c = r.0;
                    p = r.1;
                }
                type_ = UCD_CHARTYPE(c);
            }
        }
        /* Handle non-group names and group names in non-UTF modes. A group name must
        not start with a digit. If either of the others start with a digit it just
        won't be recognized. */
        else {
            if is_group != 0 && IS_DIGIT(*ptr as u32) {
                ptr = ptr.add(1);
                *errorcodeptr = ERR(44);
                break 'failed;
            }

            while ptr < ptrend
                && MAX_255(*ptr as u32)
                && (*(*cb).ctypes.add(*ptr as usize) & ctype_word) != 0
            {
                ptr = ptr.add(1);
            }
        }

        /* Check name length */

        if ptr.offset_from(*nameptr) > MAX_NAME_SIZE as isize {
            *errorcodeptr = ERR(48);
            break 'failed;
        }
        *namelenptr = ptr.offset_from(*nameptr) as u32;

        /* Subpattern names must not be empty, and their terminator is checked here. */

        if is_group != 0 {
            if ptr == *nameptr {
                *errorcodeptr = ERR(62); /* Subpattern name expected */
                break 'failed;
            }
            if is_braced != 0 {
                while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                    ptr = ptr.add(1);
                }
            }
            if terminator != 0 {
                if ptr >= ptrend || *ptr != terminator as PCRE2_UCHAR {
                    *errorcodeptr = ERR(42);
                    break 'failed;
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
    parsed_pattern: *mut u32,
    offset: PCRE2_SIZE,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> *mut u32 {
    let mut next_offset: PCRE2_SIZE;
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let mut name: PCRE2_SPTR = core::ptr::null();
    let mut terminator: PCRE2_UCHAR = 0;
    let mut meta: u32 = 0;
    let mut namelen: u32 = 0;
    let mut i: c_int = 0;
    let mut parsed_pattern = parsed_pattern;
    let mut offset = offset;

    'failed: {
        'unclosed_parenthesis: {
            if ptr >= ptrend || *ptr as u32 != CHAR_LEFT_PARENTHESIS {
                *errorcodeptr = ERR(118);
                break 'failed;
            }

            loop {
                ptr = ptr.add(1);
                next_offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;

                if ptr >= ptrend {
                    *errorcodeptr = ERR(117);
                    break 'failed;
                }

                /* Handle [+-]number cases */
                if read_number(
                    &mut ptr,
                    ptrend,
                    (*cb).bracount as i32,
                    MAX_GROUP_NUMBER,
                    ERR(61) as u32,
                    &mut i,
                    errorcodeptr,
                ) != FALSE
                {
                    /* PCRE2_ASSERT(i >= 0); */
                    if i <= 0 {
                        *errorcodeptr = ERR(15);
                        break 'failed;
                    }
                    meta = META_CAPTURE_NUMBER;
                    namelen = i as u32;
                } else if *errorcodeptr != 0 {
                    break 'failed;
                }
                /* Number too big */
                else {
                    /* Handle 'name' or <name> cases. */
                    if *ptr as u32 == CHAR_LESS_THAN_SIGN {
                        terminator = CHAR_GREATER_THAN_SIGN as PCRE2_UCHAR;
                    } else if *ptr as u32 == CHAR_APOSTROPHE {
                        terminator = CHAR_APOSTROPHE as PCRE2_UCHAR;
                    } else {
                        *errorcodeptr = ERR(117);
                        break 'failed;
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
                        break 'failed;
                    }

                    meta = META_CAPTURE_NAME;
                }

                /* PCRE2_ASSERT(next_offset > 0); */
                if offset == 0 || (next_offset.wrapping_sub(offset)) >= 0x10000 {
                    *parsed_pattern = META_OFFSET;
                    parsed_pattern = parsed_pattern.add(1);
                    /* PUTOFFSET(next_offset, parsed_pattern) */
                    *parsed_pattern = (next_offset >> 32) as u32;
                    parsed_pattern = parsed_pattern.add(1);
                    *parsed_pattern = (next_offset & 0xffffffff) as u32;
                    parsed_pattern = parsed_pattern.add(1);
                    offset = next_offset;
                }

                /* The offset is encoded as a relative offset, because for some inputs
                such as ",2" in (1,2,3), we only have space for two uint32_t values, and
                an opcode and absolute offset may require three uint32_t values. */
                *parsed_pattern = meta | (next_offset.wrapping_sub(offset)) as u32;
                parsed_pattern = parsed_pattern.add(1);
                *parsed_pattern = namelen;
                parsed_pattern = parsed_pattern.add(1);
                offset = next_offset;

                if ptr >= ptrend {
                    break 'unclosed_parenthesis;
                }

                if *ptr as u32 == CHAR_RIGHT_PARENTHESIS {
                    break;
                }

                if *ptr as u32 != CHAR_COMMA {
                    *errorcodeptr = ERR(24);
                    break 'failed;
                }
            }

            *ptrptr = ptr.add(1);
            return parsed_pattern;
        }

        /* UNCLOSED_PARENTHESIS: */
        *errorcodeptr = ERR(14);
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
    parsed_pattern: *mut u32,
    cb: *mut compile_block,
) -> *mut u32 {
    let mut previous_callout: *mut u32 = *pcalloutptr;
    let mut parsed_pattern = parsed_pattern;

    if !previous_callout.is_null() {
        *previous_callout.add(2) = (ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE)
            .wrapping_sub(*previous_callout.add(1) as PCRE2_SIZE) as u32;
    }

    if auto_callout == FALSE {
        previous_callout = core::ptr::null_mut();
    } else {
        if previous_callout.is_null()
            || previous_callout != parsed_pattern.offset(-4)
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
    escape: c_int,
    parsed_pattern: *mut u32,
    options: u32,
    xoptions: u32,
) -> *mut u32 {
    let mut ascii_option: u32 = 0;
    let mut prop: u32 = ESC_p as u32;
    let mut parsed_pattern = parsed_pattern;

    match escape {
        ESC_D => {
            prop = ESC_P as u32;
            /* Fall through */
            ascii_option = PCRE2_EXTRA_ASCII_BSD;
        }
        ESC_d => {
            ascii_option = PCRE2_EXTRA_ASCII_BSD;
        }

        ESC_S => {
            prop = ESC_P as u32;
            /* Fall through */
            ascii_option = PCRE2_EXTRA_ASCII_BSS;
        }
        ESC_s => {
            ascii_option = PCRE2_EXTRA_ASCII_BSS;
        }

        ESC_W => {
            prop = ESC_P as u32;
            /* Fall through */
            ascii_option = PCRE2_EXTRA_ASCII_BSW;
        }
        ESC_w => {
            ascii_option = PCRE2_EXTRA_ASCII_BSW;
        }
        _ => {}
    }

    if (options & PCRE2_UCP) == 0 || (xoptions & ascii_option) != 0 {
        *parsed_pattern = META_ESCAPE.wrapping_add(escape as u32);
        parsed_pattern = parsed_pattern.add(1);
    } else {
        *parsed_pattern = META_ESCAPE.wrapping_add(prop);
        parsed_pattern = parsed_pattern.add(1);
        match escape {
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
of memory to allocate for parsed_pattern. It is also called to check whether
the amount of data written respects the amount of memory allocated.

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

    /* When PCRE2_AUTO_CALLOUT is not set, in all but one case the number of
    unsigned 32-bit ints written out to the parsed pattern is bounded by the length
    of the pattern. The exceptional case is when running in 32-bit, non-UTF mode,
    which does not apply here. */

    let _ = utf; /* Avoid compiler warning */

    parsed_size_needed = ptrend.offset_from(ptr) + big32count as isize;

    /* When PCRE2_AUTO_CALLOUT is set we have to assume a numerical callout (4
    elements) for each character. This is overkill, but memory is plentiful these
    days. */

    if (options & PCRE2_AUTO_CALLOUT) != 0 {
        parsed_size_needed += ptrend.offset_from(ptr) * 4;
    }

    parsed_size_needed
}
