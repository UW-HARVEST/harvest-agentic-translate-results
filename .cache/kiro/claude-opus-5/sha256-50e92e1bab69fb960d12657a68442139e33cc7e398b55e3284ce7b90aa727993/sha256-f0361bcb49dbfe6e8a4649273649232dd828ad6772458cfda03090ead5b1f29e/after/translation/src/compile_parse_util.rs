//! Translation of PART 1 of `pcre2_compile.c` (C lines 1131–3111): the public
//! `pcre2_code_copy` / `pcre2_code_copy_with_tables` / `pcre2_code_free`
//! functions, the escape reader `PRIV(check_escape)`, and the parsing helpers
//! `read_number`, `read_repeat_counts`, `get_ucp`, `check_posix_syntax`,
//! `check_posix_name`, `read_name`, `parse_capture_list`, `manage_callouts`
//! and `handle_escdsw`.

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::compile_h::*;
use crate::compile_local::{MAX_GROUP_NUMBER, MAX_REPEAT_COUNT, REPEAT_UNLIMITED, UPPER_CASE};
use crate::compile_tables::{
    ESCAPES, ESCAPES_FIRST, ESCAPES_LAST, POSIX_NAMES, POSIX_NAME_LENGTHS, XDIGITAB,
};
// `internal` re-exports `consts::*`, so ESC_*, PT_*, PCRE2_* and the `BOOL`
// `TRUE`/`FALSE` values all come from here (avoiding a double-glob ambiguity).
use crate::internal::*;
use crate::string_utils::{_pcre2_strcmp_c8_8, _pcre2_strncmp_c8_8};

// ---------------------------------------------------------------------------
// Local character constants (mirrors the CHAR_* macros for the ASCII build).
// ---------------------------------------------------------------------------

const CHAR_HT: u32 = 0x09;
const CHAR_LF: u32 = 0x0a;
const CHAR_CR: u32 = 0x0d;
const CHAR_SPACE: u32 = 0x20;
const CHAR_0: u32 = b'0' as u32;
const CHAR_1: u32 = b'1' as u32;
const CHAR_7: u32 = b'7' as u32;
const CHAR_8: u32 = b'8' as u32;
const CHAR_9: u32 = b'9' as u32;
const CHAR_A: u32 = b'A' as u32;
const CHAR_F: u32 = b'F' as u32;
const CHAR_L: u32 = b'L' as u32;
const CHAR_U: u32 = b'U' as u32;
const CHAR_Z: u32 = b'Z' as u32;
const CHAR_a: u32 = b'a' as u32;
const CHAR_c: u32 = b'c' as u32;
const CHAR_g: u32 = b'g' as u32;
const CHAR_l: u32 = b'l' as u32;
const CHAR_o: u32 = b'o' as u32;
const CHAR_u: u32 = b'u' as u32;
const CHAR_x: u32 = b'x' as u32;
const CHAR_z: u32 = b'z' as u32;
const CHAR_PLUS: u32 = b'+' as u32;
const CHAR_MINUS: u32 = b'-' as u32;
const CHAR_COMMA: u32 = b',' as u32;
const CHAR_COLON: u32 = b':' as u32;
const CHAR_EQUALS_SIGN: u32 = b'=' as u32;
const CHAR_UNDERSCORE: u32 = b'_' as u32;
const CHAR_AMPERSAND: u32 = b'&' as u32;
const CHAR_ASTERISK: u32 = b'*' as u32;
const CHAR_CIRCUMFLEX_ACCENT: u32 = b'^' as u32;
const CHAR_LESS_THAN_SIGN: u32 = b'<' as u32;
const CHAR_GREATER_THAN_SIGN: u32 = b'>' as u32;
const CHAR_APOSTROPHE: u32 = b'\'' as u32;
const CHAR_LEFT_PARENTHESIS: u32 = b'(' as u32;
const CHAR_RIGHT_PARENTHESIS: u32 = b')' as u32;
const CHAR_LEFT_CURLY_BRACKET: u32 = b'{' as u32;
const CHAR_RIGHT_CURLY_BRACKET: u32 = b'}' as u32;
const CHAR_LEFT_SQUARE_BRACKET: u32 = b'[' as u32;
const CHAR_RIGHT_SQUARE_BRACKET: u32 = b']' as u32;
const CHAR_BACKSLASH: u32 = b'\\' as u32;

// ---------------------------------------------------------------------------
// Local helpers matching the file-scope macros of pcre2_compile.c.
// ---------------------------------------------------------------------------

/// `IS_DIGIT(x)`.
#[inline(always)]
fn IS_DIGIT(x: u32) -> bool {
    x >= CHAR_0 && x <= CHAR_9
}

/// `XDIGIT(c)` — `xdigitab[c]`, always valid in 8-bit mode.
#[inline(always)]
fn XDIGIT(c: u32) -> u32 {
    XDIGITAB[(c & 0xff) as usize] as u32
}

// Convenience integer aliases for the const-table values (stored as i64/i16).
const ESC_P_I: c_int = ESC_P as c_int;
const ESC_p_I: c_int = ESC_p as c_int;
const ESC_X_I: c_int = ESC_X as c_int;
const ESC_N_I: c_int = ESC_N as c_int;
const ESC_k_I: c_int = ESC_k as c_int;
const ESC_g_I: c_int = ESC_g as c_int;
const ESC_ub_I: c_int = ESC_ub as c_int;

// ---------------------------------------------------------------------------
//                       Copy compiled code
// ---------------------------------------------------------------------------

/// `pcre2_code_copy` — copy compiled code, but not the character tables.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_copy_8(code: *const pcre2_code) -> *mut pcre2_code {
    unsafe {
        if code.is_null() {
            return ptr::null_mut();
        }
        let malloc = (*code).memctl.malloc.unwrap();
        let newcode =
            malloc((*code).blocksize, (*code).memctl.memory_data) as *mut pcre2_code;
        if newcode.is_null() {
            return ptr::null_mut();
        }
        c_memcpy(
            newcode as *mut c_void,
            code as *const c_void,
            (*code).blocksize,
        );
        (*newcode).executable_jit = ptr::null_mut();

        // If the code was deserialized, increment the tables reference count.
        if ((*code).flags & PCRE2_DEREF_TABLES as u32) != 0 {
            let ref_count = (*code).tables.add(TABLES_LENGTH as usize) as *mut PCRE2_SIZE;
            *ref_count += 1;
        }

        newcode
    }
}

/// `pcre2_code_copy_with_tables` — copy compiled code and character tables.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_copy_with_tables_8(
    code: *const pcre2_code,
) -> *mut pcre2_code {
    unsafe {
        if code.is_null() {
            return ptr::null_mut();
        }
        let malloc = (*code).memctl.malloc.unwrap();
        let newcode =
            malloc((*code).blocksize, (*code).memctl.memory_data) as *mut pcre2_code;
        if newcode.is_null() {
            return ptr::null_mut();
        }
        c_memcpy(
            newcode as *mut c_void,
            code as *const c_void,
            (*code).blocksize,
        );
        (*newcode).executable_jit = ptr::null_mut();

        let newtables = malloc(
            TABLES_LENGTH as usize + core::mem::size_of::<PCRE2_SIZE>(),
            (*code).memctl.memory_data,
        ) as *mut u8;
        if newtables.is_null() {
            let free = (*code).memctl.free.unwrap();
            free(newcode as *mut c_void, (*code).memctl.memory_data);
            return ptr::null_mut();
        }
        c_memcpy(
            newtables as *mut c_void,
            (*code).tables as *const c_void,
            TABLES_LENGTH as usize,
        );
        let ref_count = newtables.add(TABLES_LENGTH as usize) as *mut PCRE2_SIZE;
        *ref_count = 1;

        (*newcode).tables = newtables;
        (*newcode).flags |= PCRE2_DEREF_TABLES as u32;
        newcode
    }
}

/// `pcre2_code_free` — free compiled code (and its tables if deserialized).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_code_free_8(code: *mut pcre2_code) {
    unsafe {
        if !code.is_null() {
            // SUPPORT_JIT is not defined, so no executable_jit freeing.

            if ((*code).flags & PCRE2_DEREF_TABLES as u32) != 0 {
                // Decoded tables belong to the codes after deserialization, and
                // must be freed when there are no more references to them.
                let ref_count =
                    (*code).tables.add(TABLES_LENGTH as usize) as *mut PCRE2_SIZE;
                if *ref_count > 0 {
                    *ref_count -= 1;
                    if *ref_count == 0 {
                        let free = (*code).memctl.free.unwrap();
                        free(
                            (*code).tables as *mut c_void,
                            (*code).memctl.memory_data,
                        );
                    }
                }
            }

            let free = (*code).memctl.free.unwrap();
            free(code as *mut c_void, (*code).memctl.memory_data);
        }
    }
}

// ---------------------------------------------------------------------------
//                    Read a number, possibly signed
// ---------------------------------------------------------------------------

/// `read_number` — read a number in the pattern.
pub(crate) unsafe fn read_number(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    allow_sign: i32,
    max_value: u32,
    max_error: u32,
    intptr: *mut c_int,
    errorcodeptr: *mut c_int,
) -> BOOL {
    unsafe {
        let mut sign: c_int = 0;
        let mut n: u32 = 0;
        let mut ptr = *ptrptr;
        let mut yield_: BOOL = FALSE;
        let mut max_value = max_value;

        // PCRE2_ASSERT(max_value <= INT_MAX/10 - 1);

        *errorcodeptr = 0;

        if allow_sign >= 0 && ptr < ptrend {
            if *ptr as u32 == CHAR_PLUS {
                sign = 1;
                max_value -= allow_sign as u32;
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
            n = n * 10 + (*ptr as u32 - CHAR_0);
            ptr = ptr.add(1);
            if n > max_value {
                *errorcodeptr = max_error as c_int;
                while ptr < ptrend && IS_DIGIT(*ptr as u32) {
                    ptr = ptr.add(1);
                }
                // goto EXIT
                *intptr = n as c_int;
                *ptrptr = ptr;
                return yield_;
            }
        }

        if allow_sign >= 0 && sign != 0 {
            if n == 0 {
                *errorcodeptr = ERR26; // +0 and -0 are not allowed
                *intptr = n as c_int;
                *ptrptr = ptr;
                return yield_;
            }

            if sign > 0 {
                n += allow_sign as u32;
            } else if n > allow_sign as u32 {
                *errorcodeptr = ERR15; // Non-existent subpattern
                *intptr = n as c_int;
                *ptrptr = ptr;
                return yield_;
            } else {
                n = allow_sign as u32 + 1 - n;
            }
        }

        yield_ = TRUE;

        // EXIT:
        *intptr = n as c_int;
        *ptrptr = ptr;
        yield_
    }
}

// ---------------------------------------------------------------------------
//                        Read repeat counts
// ---------------------------------------------------------------------------

/// `read_repeat_counts` — read `{n,m}` style quantifiers.
pub(crate) unsafe fn read_repeat_counts(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    minp: *mut u32,
    maxp: *mut u32,
    errorcodeptr: *mut c_int,
) -> BOOL {
    unsafe {
        let mut p = *ptrptr;
        let mut pp: PCRE2_SPTR;
        let mut yield_: BOOL = FALSE;
        let mut had_minimum: BOOL = FALSE;
        let mut min: c_int = 0;
        let mut max: c_int = REPEAT_UNLIMITED as c_int; // larger than MAX_REPEAT_COUNT

        *errorcodeptr = 0;
        while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
            p = p.add(1);
        }

        // Check the syntax before interpreting.
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
            if *pp as u32 != CHAR_COMMA {
                return FALSE;
            }
            pp = pp.add(1);
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

        // Now process the quantifier for real.

        if read_number(&mut p, ptrend, -1, MAX_REPEAT_COUNT, ERR5 as u32, &mut min, errorcodeptr)
            == FALSE
        {
            if *errorcodeptr != 0 {
                *ptrptr = p;
                return yield_;
            } // n too big
            p = p.add(1); // Skip comma and subsequent spaces
            while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                p = p.add(1);
            }
            if read_number(&mut p, ptrend, -1, MAX_REPEAT_COUNT, ERR5 as u32, &mut max, errorcodeptr)
                == FALSE
            {
                if *errorcodeptr != 0 {
                    *ptrptr = p;
                    return yield_;
                } // m too big
            }
        }
        // Have read one number. Deal with {n} or {n,} or {n,m}
        else {
            while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                p = p.add(1);
            }
            if *p as u32 == CHAR_RIGHT_CURLY_BRACKET {
                max = min;
            } else {
                // Handle {n,} or {n,m}
                p = p.add(1); // Skip comma and subsequent spaces
                while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
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
                        *ptrptr = p;
                        return yield_;
                    } // m too big
                }

                if max < min {
                    *errorcodeptr = ERR4;
                    *ptrptr = p;
                    return yield_;
                }
            }
        }

        // Valid quantifier exists
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

        // EXIT:
        *ptrptr = p;
        yield_
    }
}

// ---------------------------------------------------------------------------
//                          Handle escapes
// ---------------------------------------------------------------------------

/// `PRIV(check_escape)` — process a `\` escape sequence.
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
    unsafe {
        let utf = (options & PCRE2_UTF as u32) != 0;
        let mut alt_bsux =
            ((options & PCRE2_ALT_BSUX as u32) | (xoptions & PCRE2_EXTRA_ALT_BSUX as u32)) != 0;
        let mut ptr = *ptrptr;
        let mut c: u32;
        let mut cc: u32 = 0;
        let mut escape: c_int = 0;
        let mut i: c_int;

        // If backslash is at the end of the string, it's an error.
        if ptr >= ptrend {
            *errorcodeptr = ERR1;
            *chptr = 0; // c is uninitialized in C but returned; keep consistent
            *ptrptr = ptr;
            return 0;
        }

        c = GETCHARINCTEST(&mut ptr, utf); // Get character value, increment pointer
        *errorcodeptr = 0; // Be optimistic

        // Non-alphanumerics are literals.
        if c < ESCAPES_FIRST || c > ESCAPES_LAST {
            // Definitely literal
        } else {
            i = ESCAPES[(c - ESCAPES_FIRST) as usize] as c_int;
            if i != 0 {
                if i > 0 {
                    c = i as u32;
                    if c == CHAR_CR && (xoptions & PCRE2_EXTRA_ESCAPED_CR_IS_LF as u32) != 0 {
                        c = CHAR_LF;
                    }
                } else {
                    // Negative table entry
                    escape = -i; // Else return a special escape
                    if !cb.is_null()
                        && (escape == ESC_P_I || escape == ESC_p_I || escape == ESC_X_I)
                    {
                        (*cb).external_flags |= PCRE2_HASBKPORX as u32; // Note \P, \p, or \X
                    }

                    // \N handling.
                    if escape == ESC_N_I
                        && ptr < ptrend
                        && *ptr as u32 == CHAR_LEFT_CURLY_BRACKET
                    {
                        let mut p = ptr.add(1);

                        // Perl ignores spaces and tabs after {
                        while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                            p = p.add(1);
                        }

                        // \N{U+ can be handled by the \x{ code (UTF only; not EBCDIC).
                        if ptrend.offset_from(p) > 1
                            && *p as u32 == CHAR_U
                            && *p.add(1) as u32 == CHAR_PLUS
                        {
                            // #ifndef EBCDIC
                            if utf {
                                ptr = p.add(2);
                                escape = 0; // Not a fancy escape after all
                                return check_escape_come_from_nu(
                                    ptr,
                                    ptrend,
                                    chptr,
                                    errorcodeptr,
                                    xoptions,
                                    utf,
                                    ptrptr,
                                    &mut c,
                                    escape,
                                );
                            }
                            // #endif

                            // Improve error offset.
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

                            *errorcodeptr = ERR93;
                        }
                        // Give an error where quantifiers are not allowed.
                        else if isclass != FALSE || cb.is_null() {
                            ptr = ptr.add(1); // Skip over the opening brace
                            *errorcodeptr = ERR37;
                        }
                        // Give an error if what follows is not a quantifier.
                        else {
                            if read_repeat_counts(
                                &mut p,
                                ptrend,
                                ptr::null_mut(),
                                ptr::null_mut(),
                                errorcodeptr,
                            ) == FALSE
                                && *errorcodeptr == 0
                            {
                                ptr = ptr.add(1); // Skip over the opening brace
                                *errorcodeptr = ERR37;
                            }
                        }
                    }
                }
            } else {
                // Escapes with a zero table entry, and unknown escapes.
                return check_escape_zero(
                    ptr, ptrend, c, cc, escape, alt_bsux, errorcodeptr, options, xoptions,
                    bracount, isclass, cb, utf, chptr, ptrptr,
                );
            }
        }

        // EXIT:
        *ptrptr = ptr;
        *chptr = c;
        escape
    }
}

/// Handles the `else` branch of `check_escape` (zero table entry / unknown
/// escape), including the shared `COME_FROM_NU` and `ESCAPE_FAILED_FORWARD`
/// labels. Kept as a helper to model C's `goto` structure.
#[inline(always)]
unsafe fn check_escape_zero(
    mut ptr: PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    mut c: u32,
    mut cc: u32,
    mut escape: c_int,
    mut alt_bsux: bool,
    errorcodeptr: *mut c_int,
    options: u32,
    xoptions: u32,
    bracount: u32,
    isclass: BOOL,
    cb: *mut compile_block,
    utf: bool,
    chptr: *mut u32,
    ptrptr: *mut PCRE2_SPTR,
) -> c_int {
    unsafe {
        let mut s: c_int = 0;
        let mut oldptr: PCRE2_SPTR;
        let mut overflow: bool;

        // Filter calls from pcre2_substitute().
        if cb.is_null() {
            if !(c >= CHAR_0 && c <= CHAR_9)
                && c != CHAR_c
                && c != CHAR_o
                && c != CHAR_x
                && c != CHAR_g
            {
                *errorcodeptr = ERR3;
                // goto EXIT
                *ptrptr = ptr;
                *chptr = c;
                return escape;
            }
            alt_bsux = false; // Do not modify \x handling
        }

        // A macro to model `goto ESCAPE_FAILED_FORWARD`.
        macro_rules! escape_failed_forward {
            () => {{
                ptr = ptr.add(1);
                if utf {
                    let mut pp = ptr;
                    FORWARDCHARTEST(&mut pp, ptrend);
                    ptr = pp;
                }
                *ptrptr = ptr;
                *chptr = c;
                return escape;
            }};
        }

        if c == CHAR_F || c == CHAR_l || c == CHAR_L {
            *errorcodeptr = ERR37;
        } else if c == CHAR_u {
            if !alt_bsux {
                *errorcodeptr = ERR37;
            } else {
                let mut xc: u32;
                if ptr >= ptrend {
                    // break
                    *ptrptr = ptr;
                    *chptr = c;
                    return escape;
                }
                if *ptr as u32 == CHAR_LEFT_CURLY_BRACKET
                    && (xoptions & PCRE2_EXTRA_ALT_BSUX as u32) != 0
                {
                    let mut hptr = ptr.add(1);

                    cc = 0;
                    while hptr < ptrend && {
                        xc = XDIGIT(*hptr as u32);
                        xc != 0xff
                    } {
                        if (cc & 0xf0000000) != 0 {
                            // 32-bit overflow
                            *errorcodeptr = ERR77;
                            ptr = hptr; // Show where
                            break;
                        }
                        cc = (cc << 4) | xc;
                        hptr = hptr.add(1);
                    }

                    if hptr == ptr.add(1)
                        || hptr >= ptrend
                        || *hptr as u32 != CHAR_RIGHT_CURLY_BRACKET
                    {
                        if isclass != FALSE {
                            // In a class, just treat as '\u' literal
                            *ptrptr = ptr;
                            *chptr = c;
                            return escape;
                        }
                        escape = ESC_ub_I; // Special return
                        ptr = ptr.add(1); // Skip {
                        *ptrptr = ptr;
                        *chptr = c;
                        return escape;
                    }

                    c = cc; // Accept the code point
                    ptr = hptr.add(1);
                } else {
                    // Must be exactly 4 hex digits
                    if ptrend.offset_from(ptr) < 4 {
                        *ptrptr = ptr;
                        *chptr = c;
                        return escape;
                    }
                    cc = XDIGIT(*ptr.add(0) as u32);
                    if cc == 0xff {
                        *ptrptr = ptr;
                        *chptr = c;
                        return escape;
                    }
                    xc = XDIGIT(*ptr.add(1) as u32);
                    if xc == 0xff {
                        *ptrptr = ptr;
                        *chptr = c;
                        return escape;
                    }
                    cc = (cc << 4) | xc;
                    xc = XDIGIT(*ptr.add(2) as u32);
                    if xc == 0xff {
                        *ptrptr = ptr;
                        *chptr = c;
                        return escape;
                    }
                    cc = (cc << 4) | xc;
                    xc = XDIGIT(*ptr.add(3) as u32);
                    if xc == 0xff {
                        *ptrptr = ptr;
                        *chptr = c;
                        return escape;
                    }
                    c = (cc << 4) | xc;
                    ptr = ptr.add(4);
                }

                if utf {
                    if c > 0x10ffff {
                        *errorcodeptr = ERR77;
                    } else if c >= 0xd800
                        && c <= 0xdfff
                        && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES as u32) == 0
                    {
                        *errorcodeptr = ERR73;
                    }
                } else if c > MAX_NON_UTF_CHAR as u32 {
                    *errorcodeptr = ERR77;
                }
            }
        } else if c == CHAR_U {
            if !alt_bsux {
                *errorcodeptr = ERR37;
            }
        } else if c == CHAR_g {
            if isclass != FALSE {
                *ptrptr = ptr;
                *chptr = c;
                return escape;
            }

            if ptr >= ptrend {
                *errorcodeptr = ERR57;
                *ptrptr = ptr;
                *chptr = c;
                return escape;
            }

            if cb.is_null() {
                // Substitution strings
                if *ptr as u32 != CHAR_LESS_THAN_SIGN {
                    *errorcodeptr = ERR57;
                    *ptrptr = ptr;
                    *chptr = c;
                    return escape;
                }

                let mut p = ptr.add(1);

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
                        escape = ESC_g_I; // No number found
                    }
                    *ptrptr = ptr;
                    *chptr = c;
                    return escape;
                }

                if p >= ptrend || *p as u32 != CHAR_GREATER_THAN_SIGN {
                    ptr = p;
                    *errorcodeptr = ERR119; // Missing terminator for number
                    *ptrptr = ptr;
                    *chptr = c;
                    return escape;
                }

                ptr = p.add(1);
                escape = -(s + 1);
                *ptrptr = ptr;
                *chptr = c;
                return escape;
            }

            if *ptr as u32 == CHAR_LESS_THAN_SIGN || *ptr as u32 == CHAR_APOSTROPHE {
                escape = ESC_g_I;
                *ptrptr = ptr;
                *chptr = c;
                return escape;
            }

            // If there is a brace delimiter, try to read a numerical reference.
            if *ptr as u32 == CHAR_LEFT_CURLY_BRACKET {
                let mut p = ptr.add(1);

                while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
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
                        escape = ESC_k_I; // No number found
                    }
                    *ptrptr = ptr;
                    *chptr = c;
                    return escape;
                }
                while p < ptrend && (*p as u32 == CHAR_SPACE || *p as u32 == CHAR_HT) {
                    p = p.add(1);
                }

                if p >= ptrend || *p as u32 != CHAR_RIGHT_CURLY_BRACKET {
                    ptr = p;
                    *errorcodeptr = ERR119; // Missing terminator for number
                    *ptrptr = ptr;
                    *chptr = c;
                    return escape;
                }
                ptr = p.add(1);
            } else {
                // Read an undelimited number
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
                        *errorcodeptr = ERR57; // No number found
                    }
                    *ptrptr = ptr;
                    *chptr = c;
                    return escape;
                }
            }

            if s <= 0 {
                *errorcodeptr = ERR15;
                *ptrptr = ptr;
                *chptr = c;
                return escape;
            }

            escape = -(s + 1);
        } else if (c >= CHAR_1 && c <= CHAR_9) || c == CHAR_0 {
            // Digit handling: CHAR_1..CHAR_9 first (with fall-through to CHAR_0).
            let mut fall_to_octal = false;

            if c != CHAR_0 {
                // case CHAR_1..CHAR_9
                if isclass != FALSE {
                    // Fall through to octal handling.
                    fall_to_octal = true;
                } else if (xoptions & PCRE2_EXTRA_PYTHON_OCTAL as u32) != 0 {
                    // Python-style disambiguation.
                    if *ptr.sub(1) as u32 <= CHAR_7
                        && ptr.add(1) < ptrend
                        && *ptr.add(0) as u32 >= CHAR_0
                        && *ptr.add(0) as u32 <= CHAR_7
                        && *ptr.add(1) as u32 >= CHAR_0
                        && *ptr.add(1) as u32 <= CHAR_7
                    {
                        // Peeked a three-digit octal, fall through.
                        fall_to_octal = true;
                    } else {
                        ptr = ptr.sub(1); // Back to the digit
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
                            *ptrptr = ptr;
                            *chptr = c;
                            return escape;
                        }
                        escape = -(s + 1);
                        *ptrptr = ptr;
                        *chptr = c;
                        return escape;
                    }
                } else {
                    // Perl-style disambiguation.
                    oldptr = ptr;
                    ptr = ptr.sub(1); // Back to the digit

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

                    // \1 to \9 are always back references. \8x and \9x too; \1x
                    // to \7x are octal escapes if not that many captures.
                    if s < 10 || c >= CHAR_8 || (s as u32) <= bracount {
                        if (s as u32) > MAX_GROUP_NUMBER {
                            // PCRE2_ASSERT(s == INT_MAX);
                            *errorcodeptr = ERR61;
                        } else {
                            escape = -(s + 1); // Indicates a back reference
                        }
                        *ptrptr = ptr;
                        *chptr = c;
                        return escape;
                    }

                    ptr = oldptr; // Put the pointer back and fall through
                    fall_to_octal = true;
                }

                // Handle a digit following \ when not a back reference.
                if !fall_to_octal {
                    // (unreachable: all non-fall paths returned above)
                }

                if c >= CHAR_8 {
                    // break
                    *ptrptr = ptr;
                    *chptr = c;
                    return escape;
                }
                // Fall through to CHAR_0 octal handling.
            }

            // case CHAR_0 (and fall-through from digit handling)
            // Local reproduction of `i` from the parent scope: after
            // GETCHARINCTEST + escapes lookup, `i` is set by the earlier code.
            // For the octal loop, C uses the parent `i` which at this point
            // equals 0 (escapes table returned 0 to get here). We track it
            // locally starting from 0.
            let mut oi: c_int = 0;
            c -= CHAR_0;
            while {
                let cond = oi < 2 && ptr < ptrend && *ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_7;
                oi += 1;
                cond
            } {
                c = c * 8 + *ptr as u32 - CHAR_0;
                ptr = ptr.add(1);
            }
            if c > 0xff {
                if (xoptions & PCRE2_EXTRA_PYTHON_OCTAL as u32) != 0 {
                    *errorcodeptr = ERR102;
                }
                // PCRE2_CODE_UNIT_WIDTH == 8
                else if !utf {
                    *errorcodeptr = ERR51;
                }
            }

            if (xoptions & PCRE2_EXTRA_NO_BS0 as u32) != 0 && c == 0 && oi == 1 {
                *errorcodeptr = ERR98;
            }
        } else if c == CHAR_o {
            if ptr >= ptrend || *ptr as u32 != CHAR_LEFT_CURLY_BRACKET {
                *errorcodeptr = ERR55;
                *ptrptr = ptr;
                *chptr = c;
                return escape;
            }
            ptr = ptr.add(1);

            while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                ptr = ptr.add(1);
            }
            if ptr >= ptrend || *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                *errorcodeptr = ERR78;
                *ptrptr = ptr;
                *chptr = c;
                return escape;
            }

            c = 0;
            overflow = false;
            while ptr < ptrend && *ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_7 {
                cc = *ptr as u32;
                ptr = ptr.add(1);
                if c == 0 && cc == CHAR_0 {
                    continue; // Leading zeroes
                }
                c = (c << 3) + (cc - CHAR_0);
                // PCRE2_CODE_UNIT_WIDTH == 8
                if c > (if utf { 0x10ffff } else { 0xff }) {
                    overflow = true;
                    break;
                }
            }

            while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                ptr = ptr.add(1);
            }

            if overflow {
                while ptr < ptrend && *ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_7 {
                    ptr = ptr.add(1);
                }
                *errorcodeptr = ERR34;
            } else if utf
                && c >= 0xd800
                && c <= 0xdfff
                && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES as u32) == 0
            {
                *errorcodeptr = ERR73;
            } else if ptr < ptrend && *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                ptr = ptr.add(1);
            } else {
                *errorcodeptr = ERR64;
                escape_failed_forward!();
            }
        } else if c == CHAR_x {
            if alt_bsux {
                let mut xc: u32;
                if ptrend.offset_from(ptr) < 2 {
                    *ptrptr = ptr;
                    *chptr = c;
                    return escape;
                }
                cc = XDIGIT(*ptr.add(0) as u32);
                if cc == 0xff {
                    *ptrptr = ptr;
                    *chptr = c;
                    return escape;
                }
                xc = XDIGIT(*ptr.add(1) as u32);
                if xc == 0xff {
                    *ptrptr = ptr;
                    *chptr = c;
                    return escape;
                }
                c = (cc << 4) | xc;
                ptr = ptr.add(2);
            } else {
                if ptr < ptrend && *ptr as u32 == CHAR_LEFT_CURLY_BRACKET {
                    ptr = ptr.add(1);
                    while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                        ptr = ptr.add(1);
                    }

                    // COME_FROM_NU label handled by check_escape_come_from_nu;
                    // here we execute the same logic inline.
                    if ptr >= ptrend || *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                        *errorcodeptr = ERR78;
                        *ptrptr = ptr;
                        *chptr = c;
                        return escape;
                    }
                    c = 0;
                    overflow = false;

                    while ptr < ptrend && {
                        cc = XDIGIT(*ptr as u32);
                        cc != 0xff
                    } {
                        ptr = ptr.add(1);
                        if c == 0 && cc == 0 {
                            continue; // Leading zeroes
                        }
                        c = (c << 4) | cc;
                        if (utf && c > 0x10ffff) || (!utf && c > MAX_NON_UTF_CHAR as u32) {
                            overflow = true;
                            break;
                        }
                    }

                    while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                        ptr = ptr.add(1);
                    }

                    if overflow {
                        while ptr < ptrend && XDIGIT(*ptr as u32) != 0xff {
                            ptr = ptr.add(1);
                        }
                        *errorcodeptr = ERR34;
                    } else if utf
                        && c >= 0xd800
                        && c <= 0xdfff
                        && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES as u32) == 0
                    {
                        *errorcodeptr = ERR73;
                    } else if ptr < ptrend && *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                        ptr = ptr.add(1);
                    } else {
                        *errorcodeptr = ERR67;
                        escape_failed_forward!();
                    }
                } else {
                    // Read up to two hex digits after \x
                    if ptr >= ptrend || {
                        cc = XDIGIT(*ptr as u32);
                        cc == 0xff
                    } {
                        *errorcodeptr = ERR78;
                        *ptrptr = ptr;
                        *chptr = c;
                        return escape;
                    }
                    ptr = ptr.add(1);
                    c = cc;

                    if ptr >= ptrend || {
                        cc = XDIGIT(*ptr as u32);
                        cc == 0xff
                    } {
                        *ptrptr = ptr;
                        *chptr = c;
                        return escape;
                    }
                    ptr = ptr.add(1);
                    c = (c << 4) | cc;
                }
            }
        } else if c == CHAR_c {
            if ptr >= ptrend {
                *errorcodeptr = ERR2;
                *ptrptr = ptr;
                *chptr = c;
                return escape;
            }
            c = *ptr as u32;
            if c >= CHAR_a && c <= CHAR_z {
                c = UPPER_CASE(c);
            }

            // ASCII/UTF-8 coding
            if c < 32 || c > 126 {
                *errorcodeptr = ERR68;
                escape_failed_forward!();
            }
            c ^= 0x40;

            ptr = ptr.add(1);
        } else {
            *errorcodeptr = ERR3;
        }

        // EXIT:
        *ptrptr = ptr;
        *chptr = c;
        escape
    }
}

/// Executes the `COME_FROM_NU` branch reached from `\N{U+...}` in UTF mode.
/// This is the `\x{...}` hex-scanning logic starting at the `COME_FROM_NU`
/// label with `ptr` already positioned after `U+`.
#[inline(always)]
unsafe fn check_escape_come_from_nu(
    mut ptr: PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    chptr: *mut u32,
    errorcodeptr: *mut c_int,
    xoptions: u32,
    utf: bool,
    ptrptr: *mut PCRE2_SPTR,
    c_out: *mut u32,
    mut escape: c_int,
) -> c_int {
    unsafe {
        let mut c: u32;
        let mut cc: u32;
        let mut overflow: bool;

        macro_rules! escape_failed_forward {
            () => {{
                ptr = ptr.add(1);
                if utf {
                    let mut pp = ptr;
                    FORWARDCHARTEST(&mut pp, ptrend);
                    ptr = pp;
                }
                *ptrptr = ptr;
                *chptr = c;
                return escape;
            }};
        }

        if ptr >= ptrend || *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
            c = *c_out;
            *errorcodeptr = ERR78;
            *ptrptr = ptr;
            *chptr = c;
            return escape;
        }
        c = 0;
        overflow = false;

        while ptr < ptrend && {
            cc = XDIGIT(*ptr as u32);
            cc != 0xff
        } {
            ptr = ptr.add(1);
            if c == 0 && cc == 0 {
                continue;
            }
            c = (c << 4) | cc;
            if (utf && c > 0x10ffff) || (!utf && c > MAX_NON_UTF_CHAR as u32) {
                overflow = true;
                break;
            }
        }

        while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
            ptr = ptr.add(1);
        }

        if overflow {
            while ptr < ptrend && XDIGIT(*ptr as u32) != 0xff {
                ptr = ptr.add(1);
            }
            *errorcodeptr = ERR34;
        } else if utf
            && c >= 0xd800
            && c <= 0xdfff
            && (xoptions & PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES as u32) == 0
        {
            *errorcodeptr = ERR73;
        } else if ptr < ptrend && *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
            ptr = ptr.add(1);
        } else {
            *errorcodeptr = ERR67;
            escape_failed_forward!();
        }

        // EXIT:
        *ptrptr = ptr;
        *chptr = c;
        escape
    }
}

// ---------------------------------------------------------------------------
//                        Handle \P and \p (get_ucp)
// ---------------------------------------------------------------------------

/// `get_ucp` — parse a `\p{...}` / `\P{...}` property name.
pub(crate) unsafe fn get_ucp(
    ptrptr: *mut PCRE2_SPTR,
    utf: BOOL,
    negptr: *mut BOOL,
    ptypeptr: *mut u16,
    pdataptr: *mut u16,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> BOOL {
    unsafe {
        let _ = utf; // MAYBE_UTF_MULTI is defined in UTF 8-bit; parameter retained.
        let mut c: u32;
        let mut i: isize;
        let mut bot: PCRE2_SIZE;
        let mut top: PCRE2_SIZE;
        let mut ptr = *ptrptr;
        let mut name: [PCRE2_UCHAR; 50] = [0; 50];
        let mut vptr: *mut PCRE2_UCHAR = ptr::null_mut();
        let mut ptscript: u16 = PT_NOTSCRIPT as u16;

        if ptr >= (*cb).end_pattern {
            *errorcodeptr = ERR46;
            *ptrptr = ptr;
            return FALSE;
        }
        c = GETCHARINCTEST(&mut ptr, utf != FALSE);
        *negptr = FALSE;

        i = 0;
        if c == CHAR_LEFT_CURLY_BRACKET {
            if ptr >= (*cb).end_pattern {
                *errorcodeptr = ERR46;
                *ptrptr = ptr;
                return FALSE;
            }

            i = 0;
            'outer: while i < (name.len() as isize) - 1 {
                // REDO:
                loop {
                    if ptr >= (*cb).end_pattern {
                        *errorcodeptr = ERR46;
                        *ptrptr = ptr;
                        return FALSE;
                    }
                    c = GETCHARINCTEST(&mut ptr, utf != FALSE);

                    // Skip ignorable Unicode characters.
                    if c == CHAR_UNDERSCORE
                        || c == CHAR_MINUS
                        || c == CHAR_SPACE
                        || (c >= CHAR_HT && c <= CHAR_CR)
                    {
                        continue; // goto REDO
                    }

                    // Leading circumflex negates.
                    if i == 0 && *negptr == FALSE && c == CHAR_CIRCUMFLEX_ACCENT {
                        *negptr = TRUE;
                        continue; // goto REDO
                    }
                    break;
                }

                if c == CHAR_RIGHT_CURLY_BRACKET {
                    break 'outer;
                }

                if c < CHAR_AMPERSAND || c > CHAR_z {
                    *errorcodeptr = ERR46;
                    *ptrptr = ptr;
                    return FALSE;
                }

                if c >= CHAR_A && c <= CHAR_Z {
                    c |= 0x20;
                } else if (c == CHAR_COLON || c == CHAR_EQUALS_SIGN) && vptr.is_null() {
                    vptr = name.as_mut_ptr().add(i as usize);
                }

                name[i as usize] = c as PCRE2_UCHAR;
                i += 1;
            }

            // Error if the loop didn't end with '}'.
            if c != CHAR_RIGHT_CURLY_BRACKET {
                *errorcodeptr = ERR46;
                *ptrptr = ptr;
                return FALSE;
            }
            name[i as usize] = 0;
        }
        // If { doesn't follow, one following ASCII letter.
        else if c >= CHAR_A && c <= CHAR_Z {
            name[0] = (c | 0x20) as PCRE2_UCHAR; // Lower case
            name[1] = 0;
        } else if c >= CHAR_a && c <= CHAR_z {
            name[0] = c as PCRE2_UCHAR;
            name[1] = 0;
        } else {
            *errorcodeptr = ERR46;
            *ptrptr = ptr;
            return FALSE;
        }

        *ptrptr = ptr; // Update pattern pointer

        // Class name and value separately specified.
        if !vptr.is_null() {
            let mut offset: usize = 0;
            let mut sname: [PCRE2_UCHAR; 8] = [0; 8];

            *vptr = 0; // Terminate property name
            if _pcre2_strcmp_c8_8(name.as_ptr(), b"bidiclass\0".as_ptr() as *const c_char) == 0
                || _pcre2_strcmp_c8_8(name.as_ptr(), b"bc\0".as_ptr() as *const c_char) == 0
            {
                offset = 4;
                sname[0] = b'b';
                sname[1] = b'i';
                sname[2] = b'd';
                sname[3] = b'i';
            } else if _pcre2_strcmp_c8_8(name.as_ptr(), b"script\0".as_ptr() as *const c_char) == 0
                || _pcre2_strcmp_c8_8(name.as_ptr(), b"sc\0".as_ptr() as *const c_char) == 0
            {
                ptscript = PT_SC as u16;
            } else if _pcre2_strcmp_c8_8(
                name.as_ptr(),
                b"scriptextensions\0".as_ptr() as *const c_char,
            ) == 0
                || _pcre2_strcmp_c8_8(name.as_ptr(), b"scx\0".as_ptr() as *const c_char) == 0
            {
                ptscript = PT_SCX as u16;
            } else {
                *errorcodeptr = ERR47;
                return FALSE;
            }

            // Adjust the string in name[] as needed.
            let move_len = (name.as_ptr().add(i as usize) as usize - vptr as usize)
                / core::mem::size_of::<PCRE2_UCHAR>();
            c_memmove(
                name.as_mut_ptr().add(offset) as *mut c_void,
                vptr.add(1) as *const c_void,
                move_len * core::mem::size_of::<PCRE2_UCHAR>(),
            );
            if offset != 0 {
                c_memmove(
                    name.as_mut_ptr() as *mut c_void,
                    sname.as_ptr() as *const c_void,
                    offset * core::mem::size_of::<PCRE2_UCHAR>(),
                );
            }
        }

        // Search for a recognized property using binary chop.
        bot = 0;
        top = crate::tables::_pcre2_utt_size;

        while bot < top {
            let r: c_int;
            let mid = (bot + top) >> 1;
            r = _pcre2_strcmp_c8_8(
                name.as_ptr(),
                crate::tables::_pcre2_utt_names
                    .as_ptr()
                    .add(crate::tables::_pcre2_utt[mid].name_offset as usize),
            );

            if r == 0 {
                *pdataptr = crate::tables::_pcre2_utt[mid].value;
                if vptr.is_null() || ptscript == PT_NOTSCRIPT as u16 {
                    *ptypeptr = crate::tables::_pcre2_utt[mid].type_;
                    return TRUE;
                }

                match crate::tables::_pcre2_utt[mid].type_ as u32 {
                    x if x == PT_SC as u32 => {
                        *ptypeptr = PT_SC as u16;
                        return TRUE;
                    }
                    x if x == PT_SCX as u32 => {
                        *ptypeptr = ptscript;
                        return TRUE;
                    }
                    _ => {}
                }

                break; // Non-script found
            }

            if r > 0 {
                bot = mid + 1;
            } else {
                top = mid;
            }
        }

        *errorcodeptr = ERR47; // Unrecognized property
        FALSE
    }
}

// ---------------------------------------------------------------------------
//                    Check for POSIX class syntax
// ---------------------------------------------------------------------------

/// `check_posix_syntax` — check `[:name:]` style POSIX syntax.
pub(crate) unsafe fn check_posix_syntax(
    ptr: PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    endptr: *mut PCRE2_SPTR,
) -> BOOL {
    unsafe {
        let mut ptr = ptr;
        let terminator: u32;
        terminator = *ptr as u32;
        ptr = ptr.add(1);

        while ptrend.offset_from(ptr) >= 2 {
            if *ptr as u32 == CHAR_BACKSLASH
                && (*ptr.add(1) as u32 == CHAR_RIGHT_SQUARE_BRACKET
                    || *ptr.add(1) as u32 == CHAR_BACKSLASH)
            {
                ptr = ptr.add(1);
            } else if (*ptr as u32 == CHAR_LEFT_SQUARE_BRACKET && *ptr.add(1) as u32 == terminator)
                || *ptr as u32 == CHAR_RIGHT_SQUARE_BRACKET
            {
                return FALSE;
            } else if *ptr as u32 == terminator
                && *ptr.add(1) as u32 == CHAR_RIGHT_SQUARE_BRACKET
            {
                *endptr = ptr;
                return TRUE;
            }

            ptr = ptr.add(1);
        }

        FALSE
    }
}

// ---------------------------------------------------------------------------
//                       Check POSIX class name
// ---------------------------------------------------------------------------

/// `check_posix_name` — look up a POSIX class name.
pub(crate) unsafe fn check_posix_name(ptr: PCRE2_SPTR, len: c_int) -> c_int {
    unsafe {
        let mut pn: usize = 0; // offset into POSIX_NAMES
        let mut yield_: usize = 0;
        while POSIX_NAME_LENGTHS[yield_] != 0 {
            if len == POSIX_NAME_LENGTHS[yield_] as c_int
                && _pcre2_strncmp_c8_8(
                    ptr,
                    POSIX_NAMES.as_ptr().add(pn) as *const c_char,
                    len as usize,
                ) == 0
            {
                return yield_ as c_int;
            }
            pn += POSIX_NAME_LENGTHS[yield_] as usize + 1;
            yield_ += 1;
        }
        -1
    }
}

// ---------------------------------------------------------------------------
//                   Read a subpattern or VERB name
// ---------------------------------------------------------------------------

/// `read_name` — read a subpattern or `(*VERB)` / `(*alpha_assertion)` name.
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
    unsafe {
        let mut ptr = *ptrptr;
        let is_group: BOOL = (*ptr as u32 != CHAR_ASTERISK) as BOOL;
        ptr = ptr.add(1);
        let is_braced: BOOL = (terminator == CHAR_RIGHT_CURLY_BRACKET) as BOOL;

        if is_braced != FALSE {
            while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                ptr = ptr.add(1);
            }
        }

        if ptr >= ptrend {
            // No characters in name
            *errorcodeptr = if is_group != FALSE { ERR62 } else { ERR60 };
            *ptrptr = ptr;
            return FALSE;
        }

        *nameptr = ptr;
        *offsetptr = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;

        let mut handled_unicode = false;

        // In UTF mode a group name may contain Unicode letters/digits.
        if utf != FALSE && is_group != FALSE {
            let mut c: u32;
            let mut ty: u32;
            let mut p = ptr;

            c = GETCHARINC(&mut p); // Peek at next character
            ty = UCD_CHARTYPE(c);

            if ty == ucp_Nd {
                ptr = p;
                *errorcodeptr = ERR44;
                *ptrptr = ptr;
                return FALSE;
            }

            loop {
                if ty != ucp_Nd
                    && crate::tables::_pcre2_ucp_gentype[ty as usize] != ucp_L
                    && c != CHAR_UNDERSCORE
                {
                    break;
                }
                ptr = p; // Accept character and peek again
                if p >= ptrend {
                    break;
                }
                c = GETCHARINC(&mut p);
                ty = UCD_CHARTYPE(c);
            }
            handled_unicode = true;
        }

        // Handle non-group names and group names in non-UTF modes.
        if !handled_unicode {
            if is_group != FALSE && IS_DIGIT(*ptr as u32) {
                ptr = ptr.add(1);
                *errorcodeptr = ERR44;
                *ptrptr = ptr;
                return FALSE;
            }

            while ptr < ptrend
                && MAX_255(*ptr as u32)
                && (*(*cb).ctypes.add(*ptr as usize) as i64 & ctype_word) != 0
            {
                ptr = ptr.add(1);
            }
        }

        // Check name length.
        if ptr.offset_from(*nameptr) > MAX_NAME_SIZE as isize {
            *errorcodeptr = ERR48;
            *ptrptr = ptr;
            return FALSE;
        }
        *namelenptr = ptr.offset_from(*nameptr) as u32;

        // Subpattern names must not be empty; check terminator.
        if is_group != FALSE {
            if ptr == *nameptr {
                *errorcodeptr = ERR62; // Subpattern name expected
                *ptrptr = ptr;
                return FALSE;
            }
            if is_braced != FALSE {
                while ptr < ptrend && (*ptr as u32 == CHAR_SPACE || *ptr as u32 == CHAR_HT) {
                    ptr = ptr.add(1);
                }
            }
            if terminator != 0 {
                if ptr >= ptrend || *ptr as u32 != terminator {
                    *errorcodeptr = ERR42;
                    *ptrptr = ptr;
                    return FALSE;
                }
                ptr = ptr.add(1);
            }
        }

        *ptrptr = ptr;
        TRUE
    }
}

// ---------------------------------------------------------------------------
//                 Parse capturing bracket argument list
// ---------------------------------------------------------------------------

/// `parse_capture_list` — read a list of capture references.
pub(crate) unsafe fn parse_capture_list(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    mut parsed_pattern: *mut u32,
    mut offset: PCRE2_SIZE,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> *mut u32 {
    unsafe {
        let mut next_offset: PCRE2_SIZE;
        let mut ptr = *ptrptr;
        let mut name: PCRE2_SPTR = ptr::null();
        let mut terminator: u32;
        let mut meta: u32;
        let mut namelen: u32 = 0;
        let mut i: c_int = 0;

        if ptr >= ptrend || *ptr as u32 != CHAR_LEFT_PARENTHESIS {
            *errorcodeptr = ERR118;
            *ptrptr = ptr;
            return ptr::null_mut();
        }

        loop {
            ptr = ptr.add(1);
            next_offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;

            if ptr >= ptrend {
                *errorcodeptr = ERR117;
                *ptrptr = ptr;
                return ptr::null_mut();
            }

            // Handle [+-]number cases
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
                // PCRE2_ASSERT(i >= 0);
                if i <= 0 {
                    *errorcodeptr = ERR15;
                    *ptrptr = ptr;
                    return ptr::null_mut();
                }
                meta = META_CAPTURE_NUMBER as u32;
                namelen = i as u32;
            } else if *errorcodeptr != 0 {
                *ptrptr = ptr;
                return ptr::null_mut(); // Number too big
            } else {
                // Handle 'name' or <name> cases.
                if *ptr as u32 == CHAR_LESS_THAN_SIGN {
                    terminator = CHAR_GREATER_THAN_SIGN;
                } else if *ptr as u32 == CHAR_APOSTROPHE {
                    terminator = CHAR_APOSTROPHE;
                } else {
                    *errorcodeptr = ERR117;
                    *ptrptr = ptr;
                    return ptr::null_mut();
                }

                if read_name(
                    &mut ptr,
                    ptrend,
                    utf,
                    terminator,
                    &mut next_offset,
                    &mut name,
                    &mut namelen,
                    errorcodeptr,
                    cb,
                ) == FALSE
                {
                    *ptrptr = ptr;
                    return ptr::null_mut();
                }

                meta = META_CAPTURE_NAME as u32;
            }

            // PCRE2_ASSERT(next_offset > 0);
            if offset == 0 || (next_offset - offset) >= 0x10000 {
                *parsed_pattern = META_OFFSET as u32;
                parsed_pattern = parsed_pattern.add(1);
                PUTOFFSET(next_offset, &mut parsed_pattern);
                offset = next_offset;
            }

            *parsed_pattern = meta | (next_offset - offset) as u32;
            parsed_pattern = parsed_pattern.add(1);
            *parsed_pattern = namelen;
            parsed_pattern = parsed_pattern.add(1);
            offset = next_offset;

            if ptr >= ptrend {
                // UNCLOSED_PARENTHESIS
                *errorcodeptr = ERR14;
                *ptrptr = ptr;
                return ptr::null_mut();
            }

            if *ptr as u32 == CHAR_RIGHT_PARENTHESIS {
                break;
            }

            if *ptr as u32 != CHAR_COMMA {
                *errorcodeptr = ERR24;
                *ptrptr = ptr;
                return ptr::null_mut();
            }
        }

        *ptrptr = ptr.add(1);
        parsed_pattern
    }
}

// ---------------------------------------------------------------------------
//                 Manage callouts at start of cycle
// ---------------------------------------------------------------------------

/// `manage_callouts` — record prior item details and set up auto callouts.
pub(crate) unsafe fn manage_callouts(
    ptr: PCRE2_SPTR,
    pcalloutptr: *mut *mut u32,
    auto_callout: BOOL,
    mut parsed_pattern: *mut u32,
    cb: *mut compile_block,
) -> *mut u32 {
    unsafe {
        let mut previous_callout = *pcalloutptr;

        if !previous_callout.is_null() {
            *previous_callout.add(2) = (ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE
                - *previous_callout.add(1) as PCRE2_SIZE)
                as u32;
        }

        if auto_callout == FALSE {
            previous_callout = ptr::null_mut();
        } else {
            if previous_callout.is_null()
                || previous_callout != parsed_pattern.sub(4)
                || *previous_callout.add(3) != 255
            {
                previous_callout = parsed_pattern; // Set up new automatic callout
                parsed_pattern = parsed_pattern.add(4);
                *previous_callout.add(0) = META_CALLOUT_NUMBER as u32;
                *previous_callout.add(2) = 0;
                *previous_callout.add(3) = 255;
            }
            *previous_callout.add(1) = ptr.offset_from((*cb).start_pattern) as u32;
        }

        *pcalloutptr = previous_callout;
        parsed_pattern
    }
}

// ---------------------------------------------------------------------------
//                 Handle \d, \D, \s, \S, \w, \W
// ---------------------------------------------------------------------------

/// `handle_escdsw` — emit the parsed-pattern code for `\d` etc.
pub(crate) unsafe fn handle_escdsw(
    escape: c_int,
    mut parsed_pattern: *mut u32,
    options: u32,
    xoptions: u32,
) -> *mut u32 {
    unsafe {
        let mut ascii_option: u32 = 0;
        let mut prop: c_int = ESC_p as c_int;

        match escape as u32 {
            x if x == ESC_D => {
                prop = ESC_P as c_int;
                ascii_option = PCRE2_EXTRA_ASCII_BSD as u32;
            }
            x if x == ESC_d => {
                ascii_option = PCRE2_EXTRA_ASCII_BSD as u32;
            }
            x if x == ESC_S => {
                prop = ESC_P as c_int;
                ascii_option = PCRE2_EXTRA_ASCII_BSS as u32;
            }
            x if x == ESC_s => {
                ascii_option = PCRE2_EXTRA_ASCII_BSS as u32;
            }
            x if x == ESC_W => {
                prop = ESC_P as c_int;
                ascii_option = PCRE2_EXTRA_ASCII_BSW as u32;
            }
            x if x == ESC_w => {
                ascii_option = PCRE2_EXTRA_ASCII_BSW as u32;
            }
            _ => {}
        }

        if (options & PCRE2_UCP as u32) == 0 || (xoptions & ascii_option) != 0 {
            *parsed_pattern = META_ESCAPE as u32 + escape as u32;
            parsed_pattern = parsed_pattern.add(1);
        } else {
            *parsed_pattern = META_ESCAPE as u32 + prop as u32;
            parsed_pattern = parsed_pattern.add(1);
            match escape as u32 {
                x if x == ESC_d || x == ESC_D => {
                    *parsed_pattern = ((PT_PC as u32) << 16) | ucp_Nd;
                    parsed_pattern = parsed_pattern.add(1);
                }
                x if x == ESC_s || x == ESC_S => {
                    *parsed_pattern = (PT_SPACE as u32) << 16;
                    parsed_pattern = parsed_pattern.add(1);
                }
                x if x == ESC_w || x == ESC_W => {
                    *parsed_pattern = (PT_WORD as u32) << 16;
                    parsed_pattern = parsed_pattern.add(1);
                }
                _ => {}
            }
        }

        parsed_pattern
    }
}
