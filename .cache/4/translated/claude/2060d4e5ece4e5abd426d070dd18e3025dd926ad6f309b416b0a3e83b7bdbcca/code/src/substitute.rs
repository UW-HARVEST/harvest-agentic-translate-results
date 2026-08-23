/* Translated from pcre2_substitute.c
8-bit code units, SUPPORT_UNICODE, no JIT, no EBCDIC, LINK_SIZE 2. */

use crate::compile_h::*;
use crate::internal::*;
use crate::pcre2_pub::*;
use crate::tables::*;
use crate::ucd_data::*;
use crate::ucp::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

const PTR_STACK_SIZE: usize = 20;

const SUBSTITUTE_OPTIONS: u32 = PCRE2_SUBSTITUTE_EXTENDED
    | PCRE2_SUBSTITUTE_GLOBAL
    | PCRE2_SUBSTITUTE_LITERAL
    | PCRE2_SUBSTITUTE_MATCHED
    | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
    | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY
    | PCRE2_SUBSTITUTE_UNKNOWN_UNSET
    | PCRE2_SUBSTITUTE_UNSET_EMPTY;

/*************************************************
*           Find end of substitute text          *
*************************************************/

/* In extended mode, we recognize ${name:+set text:unset text} and similar
constructions. This requires the identification of unescaped : and }
characters. This function scans for such. It must deal with nested ${
constructions. The pointer to the text is updated, either to the required end
character, or to where an error was detected.

Arguments:
  code      points to the compiled expression (for options)
  ptrptr    points to the pointer to the start of the text (updated)
  ptrend    end of the whole string
  last      TRUE if the last expected string (only } recognized)

Returns:    0 on success
            negative error code on failure
*/

unsafe fn find_text_end(
    code: *const pcre2_real_code,
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    last: BOOL,
) -> c_int {
    let mut rc: c_int = 0;
    let mut nestlevel: u32 = 0;
    let mut literal: BOOL = FALSE;
    let mut ptr: PCRE2_SPTR = *ptrptr;

    'EXIT: {
        while ptr < ptrend {
            'CONTINUE: {
                if literal != FALSE {
                    if *ptr.add(0) as u32 == CHAR_BACKSLASH
                        && ptr < ptrend.wrapping_sub(1)
                        && *ptr.add(1) as u32 == CHAR_E
                    {
                        literal = FALSE;
                        ptr = ptr.add(1);
                    }
                } else if *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                    if nestlevel == 0 {
                        break 'EXIT;
                    }
                    nestlevel -= 1;
                } else if *ptr as u32 == CHAR_COLON && last == FALSE && nestlevel == 0 {
                    break 'EXIT;
                } else if *ptr as u32 == CHAR_DOLLAR_SIGN {
                    if ptr < ptrend.wrapping_sub(1) && *ptr.add(1) as u32 == CHAR_LEFT_CURLY_BRACKET
                    {
                        nestlevel += 1;
                        ptr = ptr.add(1);
                    }
                } else if *ptr as u32 == CHAR_BACKSLASH {
                    let erc: c_int;
                    let mut errorcode: c_int = 0;
                    let mut ch: u32 = 0;
                    let esc_end_ptr: PCRE2_SPTR;

                    if ptr < ptrend.wrapping_sub(1) {
                        let nc: u32 = *ptr.add(1) as u32;
                        if nc == CHAR_L || nc == CHAR_l || nc == CHAR_U || nc == CHAR_u {
                            ptr = ptr.add(1);
                            break 'CONTINUE;
                        }
                    }

                    ptr = ptr.add(1); /* Must point after \ */
                    erc = crate::compile_util::_pcre2_check_escape_8(
                        &mut ptr,
                        ptrend,
                        &mut ch,
                        &mut errorcode,
                        (*code).overall_options,
                        (*code).extra_options,
                        (*code).top_bracket as u32,
                        FALSE,
                        core::ptr::null_mut(),
                    );
                    if errorcode != 0 {
                        /* errorcode from check_escape is positive, so must not be returned by
                        pcre2_substitute(). */
                        rc = PCRE2_ERROR_BADREPESCAPE;
                        break 'EXIT;
                    }

                    esc_end_ptr = ptr;
                    ptr = ptr.wrapping_sub(1); /* Rewind by one, because the for-loop will increment it */

                    if erc == 0 || erc == ESC_b || erc == ESC_v || erc == ESC_E {
                        /* Data characters and isolated \E are ignored */
                    } else if erc == ESC_Q {
                        literal = TRUE;
                    } else if erc == ESC_g {
                        /* The \g<name> form (\g<number> already handled by check_escape) */
                    } else {
                        if erc < 0 {
                            /* capture group reference */
                        } else {
                            ptr = esc_end_ptr;
                            rc = PCRE2_ERROR_BADREPESCAPE;
                            break 'EXIT;
                        }
                    }
                }
            }
            ptr = ptr.add(1);
        }

        rc = PCRE2_ERROR_REPMISSINGBRACE; /* Terminator not found */
    }

    /* EXIT: */
    *ptrptr = ptr;
    rc
}

/*************************************************
*           Validate group name                  *
*************************************************/

/* This function scans for a capture group name, validating it
consists of legal characters, is not empty, and does not exceed
MAX_NAME_SIZE.

Arguments:
  ptrptr    points to the pointer to the start of the text (updated)
  ptrend    end of the whole string
  utf       true if the input is UTF-encoded
  ctypes    pointer to the character types table

Returns:    TRUE if a name was read
            FALSE otherwise
*/

unsafe fn read_name_subst(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    ctypes: *const u8,
) -> BOOL {
    let mut ptr: PCRE2_SPTR = *ptrptr;
    let nameptr: PCRE2_SPTR = ptr;

    'FAILED: {
        if ptr >= ptrend
        /* No characters in name */
        {
            break 'FAILED;
        }

        /* We do not need to check whether the name starts with a non-digit.
        We are simply referencing names here, not defining them. */

        if utf != FALSE {
            let mut c: u32;
            let mut type_: u32;

            while ptr < ptrend {
                /* GETCHAR(c, ptr) */
                c = *ptr as u32;
                if c >= 0xc0 {
                    c = getutf8(c, ptr);
                }
                type_ = UCD_CHARTYPE(c);
                if type_ != ucp_Nd
                    && _pcre2_ucp_gentype_8[type_ as usize] != ucp_L
                    && c != CHAR_UNDERSCORE
                {
                    break;
                }
                ptr = ptr.add(1);
                /* FORWARDCHARTEST(ptr, ptrend) */
                while ptr < ptrend && (*ptr & 0xc0) == 0x80 {
                    ptr = ptr.add(1);
                }
            }
        }
        /* Handle group names in non-UTF modes. */
        else {
            while ptr < ptrend
                && MAX_255(*ptr as u32)
                && (*ctypes.add(*ptr as usize) & ctype_word) != 0
            {
                ptr = ptr.add(1);
            }
        }

        /* Check name length */

        if ptr.offset_from(nameptr) > MAX_NAME_SIZE as isize {
            break 'FAILED;
        }

        /* Subpattern names must not be empty */
        if ptr == nameptr {
            break 'FAILED;
        }

        *ptrptr = ptr;
        return TRUE;
    }

    /* FAILED: */
    *ptrptr = ptr;
    FALSE
}

/*************************************************
*              Case transformations              *
*************************************************/

const PCRE2_SUBSTITUTE_CASE_NONE: c_int = 0;
/* 1, 2, 3 are PCRE2_SUBSTITUTE_CASE_LOWER, UPPER, TITLE_FIRST. */
const PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST: c_int = 4;

const PCRE2_SUBSTITUTE_CASE_LOWER: c_int = crate::pcre2_pub::PCRE2_SUBSTITUTE_CASE_LOWER as c_int;
const PCRE2_SUBSTITUTE_CASE_UPPER: c_int = crate::pcre2_pub::PCRE2_SUBSTITUTE_CASE_UPPER as c_int;
const PCRE2_SUBSTITUTE_CASE_TITLE_FIRST: c_int =
    crate::pcre2_pub::PCRE2_SUBSTITUTE_CASE_TITLE_FIRST as c_int;

#[derive(Copy, Clone)]
struct case_state {
    to_case: c_int, /* One of PCRE2_SUBSTITUTE_CASE_xyz */
    single_char: BOOL,
}

/* Helper to guess how much a string is likely to increase in size when
case-transformed. */

unsafe fn pessimistic_case_inflation(len: PCRE2_SIZE) -> PCRE2_SIZE {
    (len >> 3u32) + 10
}

/* Case transformation behaviour if no callout is passed. */

unsafe fn default_substitute_case_callout(
    input: PCRE2_SPTR,
    input_len: PCRE2_SIZE,
    output: *mut PCRE2_UCHAR,
    output_cap: PCRE2_SIZE,
    state: *mut case_state,
    code: *const pcre2_real_code,
) -> PCRE2_SIZE {
    let mut input: PCRE2_SPTR = input;
    let mut output: *mut PCRE2_UCHAR = output;
    let mut output_cap: PCRE2_SIZE = output_cap;
    let input_end: PCRE2_SPTR = input.add(input_len);
    let utf: BOOL;
    let ucp: BOOL;
    let mut temp: [PCRE2_UCHAR; 6] = [0; 6];
    let mut next_to_upper: BOOL;
    let rest_to_upper: BOOL;
    let single_char: BOOL;
    let mut overflow: BOOL = FALSE;
    let mut written: PCRE2_SIZE = 0;

    utf = (((*code).overall_options & PCRE2_UTF) != 0) as BOOL;
    ucp = (((*code).overall_options & PCRE2_UCP) != 0) as BOOL;

    if input_len == 0 {
        return 0;
    }

    if (*state).to_case == PCRE2_SUBSTITUTE_CASE_LOWER
        || (*state).to_case == PCRE2_SUBSTITUTE_CASE_UPPER
    {
        rest_to_upper = ((*state).to_case == PCRE2_SUBSTITUTE_CASE_UPPER) as BOOL;
        next_to_upper = rest_to_upper;
    } else if (*state).to_case == PCRE2_SUBSTITUTE_CASE_TITLE_FIRST {
        next_to_upper = TRUE;
        rest_to_upper = FALSE;
        (*state).to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
    } else if (*state).to_case == PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST {
        next_to_upper = FALSE;
        rest_to_upper = TRUE;
        (*state).to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
    } else {
        return 0;
    }

    single_char = (*state).single_char;
    if single_char != FALSE {
        (*state).to_case = PCRE2_SUBSTITUTE_CASE_NONE;
    }

    while input < input_end {
        let mut ch: u32;
        let chlen: c_uint;

        /* GETCHARINCTEST(ch, input) */
        ch = *input as u32;
        input = input.add(1);
        if utf != FALSE && ch >= 0xc0 {
            let r = getutf8inc(ch, input);
            ch = r.0;
            input = r.1;
        }

        if (utf != FALSE || ucp != FALSE) && ch >= 128 {
            let type_: u32 = UCD_CHARTYPE(ch);
            if _pcre2_ucp_gentype_8[type_ as usize] == ucp_L
                && type_ != (if next_to_upper != FALSE { ucp_Lu } else { ucp_Ll })
            {
                ch = UCD_OTHERCASE(ch);
            }
        } else if MAX_255(ch) {
            let bits: *const u8 = (*code).tables.add(
                cbits_offset
                    + (if next_to_upper != FALSE {
                        cbit_upper
                    } else {
                        cbit_lower
                    }),
            );
            if (*bits.add((ch / 8) as usize) & (1u8 << (ch % 8))) == 0 {
                ch = *(*code).tables.add(fcc_offset + ch as usize) as u32;
            }
        }

        if utf != FALSE {
            chlen = crate::ord2utf::_pcre2_ord2utf_8(ch, temp.as_mut_ptr());
        } else {
            temp[0] = ch as PCRE2_UCHAR;
            chlen = 1;
        }

        if overflow == FALSE && (chlen as PCRE2_SIZE) <= output_cap {
            memcpy(
                output as *mut c_void,
                temp.as_ptr() as *const c_void,
                CU2BYTES(chlen as PCRE2_SIZE),
            );
            output = output.add(chlen as usize);
            output_cap -= chlen as PCRE2_SIZE;
        } else {
            overflow = TRUE;
        }

        if (chlen as PCRE2_SIZE) > !(0 as PCRE2_SIZE) - written
        /* Integer overflow */
        {
            return !(0 as PCRE2_SIZE);
        }
        written += chlen as PCRE2_SIZE;

        next_to_upper = rest_to_upper;

        /* memcpy the remainder, if only transforming a single character. */

        if single_char != FALSE {
            let rest_len: PCRE2_SIZE = input_end.offset_from(input) as PCRE2_SIZE;

            if overflow == FALSE && rest_len <= output_cap {
                memcpy(
                    output as *mut c_void,
                    input as *const c_void,
                    CU2BYTES(rest_len),
                );
            }

            if rest_len > !(0 as PCRE2_SIZE) - written
            /* Integer overflow */
            {
                return !(0 as PCRE2_SIZE);
            }
            written += rest_len;

            return written;
        }
    }

    written
}

/* Helper to perform the call to the substitute_case_callout. */

unsafe fn do_case_copy(
    input_output: *mut PCRE2_UCHAR,
    input_len: PCRE2_SIZE,
    output_cap: PCRE2_SIZE,
    state: *mut case_state,
    utf: BOOL,
    substitute_case_callout: SubstCaseCalloutFn,
    substitute_case_callout_data: *mut c_void,
) -> PCRE2_SIZE {
    let input: PCRE2_SPTR = input_output as PCRE2_SPTR;
    let output: *mut PCRE2_UCHAR = input_output;
    let mut rc: PCRE2_SIZE = 0;
    let rc2: PCRE2_SIZE;
    let ch1_to_case: c_int;
    let rest_to_case: c_int;
    let mut ch1: [PCRE2_UCHAR; 6] = [0; 6];
    let ch1_len: PCRE2_SIZE;
    let mut rest: PCRE2_SPTR;
    let rest_len: PCRE2_SIZE;
    let mut ch1_overflow: BOOL = FALSE;
    let mut rest_overflow: BOOL = FALSE;

    if (*state).to_case == PCRE2_SUBSTITUTE_CASE_LOWER
        || (*state).to_case == PCRE2_SUBSTITUTE_CASE_UPPER
        || (*state).to_case == PCRE2_SUBSTITUTE_CASE_TITLE_FIRST
    {
        /* The easy case, where our internal casing operations align with those of
        the callout. */

        if (*state).single_char == FALSE {
            rc = (substitute_case_callout.unwrap())(
                input,
                input_len,
                output,
                output_cap,
                (*state).to_case,
                substitute_case_callout_data,
            );

            if (*state).to_case == PCRE2_SUBSTITUTE_CASE_TITLE_FIRST {
                (*state).to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
            }

            return rc;
        }

        ch1_to_case = (*state).to_case;
        rest_to_case = PCRE2_SUBSTITUTE_CASE_NONE;
    } else if (*state).to_case == PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST {
        ch1_to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
        rest_to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
    } else {
        return 0;
    }

    /* Identify the leading character. Take copy, because its storage overlaps with
    `output`, and hence may be scrambled by the callout. */

    {
        let mut ch_end: PCRE2_SPTR = input;
        let mut ch: u32;

        /* GETCHARINCTEST(ch, ch_end) */
        ch = *ch_end as u32;
        ch_end = ch_end.add(1);
        if utf != FALSE && ch >= 0xc0 {
            let r = getutf8inc(ch, ch_end);
            ch = r.0;
            ch_end = r.1;
        }
        let _ = ch;
        ch1_len = ch_end.offset_from(input) as PCRE2_SIZE;
        memcpy(
            ch1.as_mut_ptr() as *mut c_void,
            input as *const c_void,
            CU2BYTES(ch1_len),
        );
    }

    rest = input.add(ch1_len);
    rest_len = input_len - ch1_len;

    /* Transform just ch1. */

    {
        let mut ch1_cap: PCRE2_SIZE;
        let max_ch1_cap: PCRE2_SIZE;

        ch1_cap = ch1_len; /* First attempt uses the space vacated by ch1. */
        max_ch1_cap = output_cap - rest_len;

        loop {
            rc = (substitute_case_callout.unwrap())(
                ch1.as_ptr(),
                ch1_len,
                output,
                ch1_cap,
                ch1_to_case,
                substitute_case_callout_data,
            );
            if rc == !(0 as PCRE2_SIZE) {
                return rc;
            }

            if rc <= ch1_cap {
                break;
            }

            if rc > max_ch1_cap {
                ch1_overflow = TRUE;
                break;
            }

            /* Move the rest to the right, to make room for expanding ch1. */

            memmove(
                input_output.add(rc) as *mut c_void,
                rest as *const c_void,
                CU2BYTES(rest_len),
            );
            rest = input.add(rc);

            ch1_cap = rc;
        }
    }

    if rest_to_case == PCRE2_SUBSTITUTE_CASE_NONE {
        if ch1_overflow == FALSE {
            memmove(
                output.add(rc) as *mut c_void,
                rest as *const c_void,
                CU2BYTES(rest_len),
            );
        }
        rc2 = rest_len;

        (*state).to_case = PCRE2_SUBSTITUTE_CASE_NONE;
    } else {
        let mut dummy: [PCRE2_UCHAR; 1] = [0; 1];
        let mut rc2v: PCRE2_SIZE;

        rc2v = (substitute_case_callout.unwrap())(
            rest,
            rest_len,
            if ch1_overflow != FALSE {
                dummy.as_mut_ptr()
            } else {
                output.add(rc)
            },
            if ch1_overflow != FALSE {
                0u32 as PCRE2_SIZE
            } else {
                output_cap - rc
            },
            rest_to_case,
            substitute_case_callout_data,
        );
        if rc2v == !(0 as PCRE2_SIZE) {
            return rc2v;
        }

        if ch1_overflow == FALSE && rc2v > output_cap - rc {
            rest_overflow = TRUE;
        }

        if ch1_overflow != FALSE && rc2v < rest_len {
            rc2v = rest_len;
        }

        (*state).to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
        rc2 = rc2v;
    }

    if rc2 > !(0 as PCRE2_SIZE) - rc
    /* Integer overflow */
    {
        return !(0 as PCRE2_SIZE);
    }

    let _ = rest_overflow;

    rc + rc2
}

/*************************************************
*   Helpers replacing the buffer-copying macros   *
*************************************************/

/* These all return 0 on success, or the error code with which
pcre2_substitute() must immediately exit (the C code's NOROOM, CASEERROR and
TOOLARGEREPLACE labels, all of which jump to EXIT). */

unsafe fn CHECKMEMCPY(
    from: PCRE2_SPTR,
    length_: PCRE2_SIZE,
    overflowed: *mut BOOL,
    extra_needed: *mut PCRE2_SIZE,
    lengthleft: *mut PCRE2_SIZE,
    buff_offset: *mut PCRE2_SIZE,
    buffer: *mut PCRE2_UCHAR,
    suboptions: u32,
) -> c_int {
    let chkmc_length: PCRE2_SIZE = length_;
    if *overflowed != FALSE {
        if chkmc_length > !(0 as PCRE2_SIZE) - *extra_needed
        /* Integer overflow */
        {
            return PCRE2_ERROR_TOOLARGEREPLACE;
        }
        *extra_needed += chkmc_length;
    } else if *lengthleft < chkmc_length {
        if (suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH) == 0 {
            return PCRE2_ERROR_NOMEMORY;
        }
        *overflowed = TRUE;
        *extra_needed = chkmc_length - *lengthleft;
    } else {
        memcpy(
            buffer.wrapping_add(*buff_offset) as *mut c_void,
            from as *const c_void,
            CU2BYTES(chkmc_length),
        );
        *buff_offset += chkmc_length;
        *lengthleft -= chkmc_length;
    }
    0
}

unsafe fn CHECKCASECPY_DEFAULT(
    from: PCRE2_SPTR,
    length_: PCRE2_SIZE,
    overflowed: *mut BOOL,
    extra_needed: *mut PCRE2_SIZE,
    lengthleft: *mut PCRE2_SIZE,
    buff_offset: *mut PCRE2_SIZE,
    buffer: *mut PCRE2_UCHAR,
    suboptions: u32,
    forcecase: *mut case_state,
    code: *const pcre2_real_code,
) -> c_int {
    let chkcc_length: PCRE2_SIZE = length_;
    let chkcc_rc: PCRE2_SIZE;

    chkcc_rc = default_substitute_case_callout(
        from,
        chkcc_length,
        buffer.wrapping_add(*buff_offset),
        if *overflowed != FALSE { 0 } else { *lengthleft },
        forcecase,
        code,
    );
    if *overflowed != FALSE {
        if chkcc_rc > !(0 as PCRE2_SIZE) - *extra_needed
        /* Integer overflow */
        {
            return PCRE2_ERROR_TOOLARGEREPLACE;
        }
        *extra_needed += chkcc_rc;
        return 0;
    }

    if *lengthleft < chkcc_rc {
        if (suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH) == 0 {
            return PCRE2_ERROR_NOMEMORY;
        }
        *overflowed = TRUE;
        *extra_needed = chkcc_rc - *lengthleft;
    } else {
        *buff_offset += chkcc_rc;
        *lengthleft -= chkcc_rc;
    }
    0
}

unsafe fn CHECKCASECPY_CALLOUT(
    length_: PCRE2_SIZE,
    overflowed: *mut BOOL,
    extra_needed: *mut PCRE2_SIZE,
    lengthleft: *mut PCRE2_SIZE,
    buff_offset: *mut PCRE2_SIZE,
    buffer: *mut PCRE2_UCHAR,
    suboptions: u32,
    forcecase: *mut case_state,
    utf: BOOL,
    substitute_case_callout: SubstCaseCalloutFn,
    substitute_case_callout_data: *mut c_void,
) -> c_int {
    let chkcc_length: PCRE2_SIZE = length_;
    let chkcc_rc: PCRE2_SIZE;

    chkcc_rc = do_case_copy(
        buffer.wrapping_add(*buff_offset),
        chkcc_length,
        *lengthleft,
        forcecase,
        utf,
        substitute_case_callout,
        substitute_case_callout_data,
    );
    if chkcc_rc == !(0 as PCRE2_SIZE) {
        return PCRE2_ERROR_REPLACECASE;
    }

    if *lengthleft < chkcc_rc {
        if (suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH) == 0 {
            return PCRE2_ERROR_NOMEMORY;
        }
        *overflowed = TRUE;
        *extra_needed = chkcc_rc - *lengthleft;
    } else {
        *buff_offset += chkcc_rc;
        *lengthleft -= chkcc_rc;
    }
    0
}

/* This does a delayed case transformation, for the situation when we have a
case-forcing callout. */

unsafe fn DELAYEDFORCECASE(
    buff_offset: *mut PCRE2_SIZE,
    casestart_offset: PCRE2_SIZE,
    extra_needed: *mut PCRE2_SIZE,
    casestart_extra_needed: PCRE2_SIZE,
    overflowed: *mut BOOL,
    lengthleft: *mut PCRE2_SIZE,
    buffer: *mut PCRE2_UCHAR,
    suboptions: u32,
    forcecase: *mut case_state,
    utf: BOOL,
    substitute_case_callout: SubstCaseCalloutFn,
    substitute_case_callout_data: *mut c_void,
) -> c_int {
    let chars_outstanding: PCRE2_SIZE =
        (*buff_offset - casestart_offset) + (*extra_needed - casestart_extra_needed);
    if chars_outstanding > 0 {
        if *overflowed != FALSE {
            let guess: PCRE2_SIZE = pessimistic_case_inflation(chars_outstanding);
            if guess > !(0 as PCRE2_SIZE) - *extra_needed
            /* Integer overflow */
            {
                return PCRE2_ERROR_TOOLARGEREPLACE;
            }
            *extra_needed += guess;
        } else {
            /* Rewind the buffer */
            *lengthleft += *buff_offset - casestart_offset;
            *buff_offset = casestart_offset;
            /* Care! In-place case transformation */
            let r = CHECKCASECPY_CALLOUT(
                chars_outstanding,
                overflowed,
                extra_needed,
                lengthleft,
                buff_offset,
                buffer,
                suboptions,
                forcecase,
                utf,
                substitute_case_callout,
                substitute_case_callout_data,
            );
            if r != 0 {
                return r;
            }
        }
    }
    0
}

/*************************************************
*              Match and substitute              *
*************************************************/

/* This function applies a compiled re to a subject string and creates a new
string with substitutions. The first 7 arguments are the same as for
pcre2_match(). Either string length may be PCRE2_ZERO_TERMINATED.

Arguments:
  code            points to the compiled expression
  subject         points to the subject string
  length          length of subject string (may contain binary zeros)
  start_offset    where to start in the subject string
  options         option bits
  match_data      points to a match_data block, or is NULL
  context         points a PCRE2 context
  replacement     points to the replacement string
  rlength         length of replacement string
  buffer          where to put the substituted string
  blength         points to length of buffer; updated to length of string

Returns:          >= 0 number of substitutions made
                  < 0 an error code
                  PCRE2_ERROR_BADREPLACEMENT means invalid use of $
*/

/* The C code's labels inside the replacement-scanning loop (LOADLITERAL,
GROUP_SUBSTITUTE, LITERAL_SUBSTITUTE and SUBPTR_SUBSTITUTE) are entered from
several places, including from code that lexically follows them. The bodies of
those labels are therefore hoisted to the end of the loop body and selected by
this variable, which corresponds exactly to the C control flow. */

const ACT_NONE: u32 = 0;
const ACT_LOADLITERAL: u32 = 1;
const ACT_GROUP: u32 = 2;
const ACT_SUBPTR: u32 = 3;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substitute_8(
    code: *const pcre2_real_code,
    subject: PCRE2_SPTR,
    length: PCRE2_SIZE,
    start_offset: PCRE2_SIZE,
    options: u32,
    match_data: *mut pcre2_real_match_data,
    mcontext: *mut pcre2_real_match_context,
    replacement: PCRE2_SPTR,
    rlength: PCRE2_SIZE,
    buffer: *mut PCRE2_UCHAR,
    blength: *mut PCRE2_SIZE,
) -> c_int {
    let mut subject: PCRE2_SPTR = subject;
    let mut length: PCRE2_SIZE = length;
    let mut start_offset: PCRE2_SIZE = start_offset;
    let mut options: u32 = options;
    let mut match_data: *mut pcre2_real_match_data = match_data;
    let mut replacement: PCRE2_SPTR = replacement;
    let mut rlength: PCRE2_SIZE = rlength;

    let mut rc: c_int = 0;
    let mut subs: c_int;
    let mut ovector_count: u32 = 0;
    let mut goptions: u32 = 0;
    let mut suboptions: u32;
    let mut internal_match_data: *mut pcre2_real_match_data = core::ptr::null_mut();
    let mut escaped_literal: BOOL = FALSE;
    let mut overflowed: BOOL = FALSE;
    let mut use_existing_match: BOOL;
    let mut replacement_only: BOOL;
    let utf: BOOL = (((*code).overall_options & PCRE2_UTF) != 0) as BOOL;
    let partial: BOOL =
        ((options & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) != 0) as BOOL;
    let mut temp: [PCRE2_UCHAR; 6] = [0; 6];
    let mut null_str: [PCRE2_UCHAR; 1] = [0xcd];
    let original_subject: PCRE2_SPTR = subject;
    let mut ptr: PCRE2_SPTR = core::ptr::null();
    let mut repend: PCRE2_SPTR = core::ptr::null();
    let mut extra_needed: PCRE2_SIZE = 0;
    let mut buff_offset: PCRE2_SIZE;
    let buff_length: PCRE2_SIZE;
    let mut lengthleft: PCRE2_SIZE;
    let mut fraglength: PCRE2_SIZE;
    let mut ovector: *mut PCRE2_SIZE = core::ptr::null_mut();
    let mut ovecsave: [PCRE2_SIZE; 2] = [0, 0];
    let mut scb: pcre2_substitute_callout_block = core::mem::zeroed();
    let mut sub_start_extra_needed: PCRE2_SIZE = 0;
    let mut substitute_case_callout: SubstCaseCalloutFn = None;
    let mut substitute_case_callout_data: *mut c_void = core::ptr::null_mut();

    /* General initialization */

    buff_offset = 0;
    buff_length = *blength;
    lengthleft = buff_length;
    *blength = PCRE2_UNSET;

    'EXIT: {
        if !mcontext.is_null() {
            substitute_case_callout = (*mcontext).substitute_case_callout;
            substitute_case_callout_data = (*mcontext).substitute_case_callout_data;
        }

        /* Partial matching is supported, with limitations. */

        if partial != FALSE && (options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY) == 0 {
            return PCRE2_ERROR_BADOPTION;
        }

        /* Validate length and find the end of the replacement. A NULL replacement of
        zero length is interpreted as an empty string. */

        if replacement.is_null() {
            if rlength != 0 {
                return PCRE2_ERROR_NULL;
            }
            replacement = null_str.as_ptr();
        }

        if rlength == PCRE2_ZERO_TERMINATED {
            rlength = crate::string_utils::_pcre2_strlen_8(replacement);
        }
        repend = replacement.add(rlength);

        /* A NULL subject of zero length is treated as an empty string. */

        if subject.is_null() {
            if length != 0 {
                return PCRE2_ERROR_NULL;
            }
            subject = null_str.as_ptr();
        }

        if length == PCRE2_ZERO_TERMINATED {
            length = crate::string_utils::_pcre2_strlen_8(subject);
        }

        /* Check for using a match that has already happened. */

        use_existing_match = ((options & PCRE2_SUBSTITUTE_MATCHED) != 0) as BOOL;
        replacement_only = ((options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY) != 0) as BOOL;

        if use_existing_match != FALSE && match_data.is_null() {
            return PCRE2_ERROR_NULL;
        }

        if use_existing_match != FALSE {
            /* Return early, as the rest of the match_data may not have been
            initialised. */
            if (*match_data).rc < 0 && (*match_data).rc != PCRE2_ERROR_NOMATCH {
                return (*match_data).rc;
            }

            /* Not supported if the passed-in match was from the DFA interpreter. */
            if (*match_data).matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER as u8 {
                return PCRE2_ERROR_DFA_UFUNC;
            }

            if code != (*match_data).code {
                return PCRE2_ERROR_DIFFSUBSPATTERN;
            }

            if length != (*match_data).subject_length
                || !(original_subject == (*match_data).subject
                    || (((*match_data).flags & PCRE2_MD_COPIED_SUBJECT) != 0
                        && (length == 0
                            || memcmp(
                                subject as *const c_void,
                                (*match_data).subject as *const c_void,
                                CU2BYTES(length),
                            ) == 0)))
            {
                return PCRE2_ERROR_DIFFSUBSSUBJECT;
            }

            if start_offset != (*match_data).start_offset {
                return PCRE2_ERROR_DIFFSUBSOFFSET;
            }

            if (options & !(SUBSTITUTE_OPTIONS | PCRE2_NO_UTF_CHECK))
                != ((*match_data).options & !PCRE2_NO_UTF_CHECK)
            {
                return PCRE2_ERROR_DIFFSUBSOPTIONS;
            }
        }

        /* WARNING: In both cases below a general context is constructed "by hand"
        because calling pcre2_general_context_create() involves a memory allocation. */

        if match_data.is_null() {
            let mut gcontext: pcre2_real_general_context = core::mem::zeroed();
            gcontext.memctl = if mcontext.is_null() {
                (*(code as *mut pcre2_real_code)).memctl
            } else {
                (*(mcontext as *mut pcre2_real_match_context)).memctl
            };
            internal_match_data = crate::match_data::pcre2_match_data_create_from_pattern_8(
                code,
                &mut gcontext as *mut pcre2_real_general_context,
            );
            match_data = internal_match_data;
            if internal_match_data.is_null() {
                return PCRE2_ERROR_NOMEMORY;
            }
        } else if use_existing_match != FALSE {
            let pairs: c_int;
            let mut gcontext: pcre2_real_general_context = core::mem::zeroed();
            gcontext.memctl = if mcontext.is_null() {
                (*(code as *mut pcre2_real_code)).memctl
            } else {
                (*(mcontext as *mut pcre2_real_match_context)).memctl
            };
            pairs = if (*code).top_bracket as c_int + 1 < (*match_data).oveccount as c_int {
                (*code).top_bracket as c_int + 1
            } else {
                (*match_data).oveccount as c_int
            };
            internal_match_data = crate::match_data::pcre2_match_data_create_8(
                (*match_data).oveccount as u32,
                &mut gcontext as *mut pcre2_real_general_context,
            );
            if internal_match_data.is_null() {
                return PCRE2_ERROR_NOMEMORY;
            }
            memcpy(
                internal_match_data as *mut c_void,
                match_data as *const c_void,
                core::mem::offset_of!(pcre2_real_match_data, ovector)
                    + 2 * (pairs as usize) * core::mem::size_of::<PCRE2_SIZE>(),
            );
            (*internal_match_data).heapframes = core::ptr::null_mut();
            (*internal_match_data).heapframes_size = 0;
            /* Ensure that the subject is not freed when internal_match_data is */
            (*internal_match_data).flags &= !PCRE2_MD_COPIED_SUBJECT;
            match_data = internal_match_data;
        }

        /* If using an internal match data, there's no need to copy the subject. */

        if !internal_match_data.is_null() {
            options &= !PCRE2_COPY_MATCHED_SUBJECT;
        }

        /* Remember ovector details */

        ovector = crate::match_data::pcre2_get_ovector_pointer_8(match_data);
        ovector_count = crate::match_data::pcre2_get_ovector_count_8(match_data);

        /* Fixed things in the callout block */

        scb.version = 0;
        scb.input = subject;
        scb.output = buffer as PCRE2_SPTR;
        scb.ovector = ovector;

        /* Check UTF replacement string if necessary. */

        if utf != FALSE && (options & PCRE2_NO_UTF_CHECK) == 0 {
            rc = crate::valid_utf::_pcre2_valid_utf_8(
                replacement,
                rlength,
                core::ptr::addr_of_mut!((*match_data).startchar),
            );
            if rc != 0 {
                (*match_data).leftchar = 0;
                break 'EXIT;
            }
        }

        /* Save the substitute options and remove them from the match options. */

        suboptions = options & SUBSTITUTE_OPTIONS;
        options &= !SUBSTITUTE_OPTIONS;

        /* Error if the start match offset is greater than the length of the subject. */

        if start_offset > length {
            (*match_data).leftchar = 0;
            rc = PCRE2_ERROR_BADOFFSET;
            break 'EXIT;
        }

        /* Copy up to the start offset, unless only the replacement is required. */

        if replacement_only == FALSE {
            let r = CHECKMEMCPY(
                subject,
                start_offset,
                &mut overflowed,
                &mut extra_needed,
                &mut lengthleft,
                &mut buff_offset,
                buffer,
                suboptions,
            );
            if r != 0 {
                rc = r;
                break 'EXIT;
            }
        }

        /* Loop for global substituting. */

        subs = 0;
        'GLOBAL: loop {
            let mut ptrstack: [PCRE2_SPTR; PTR_STACK_SIZE] = [core::ptr::null(); PTR_STACK_SIZE];
            let mut ptrstackptr: u32 = 0;
            let mut forcecase: case_state = case_state {
                to_case: PCRE2_SUBSTITUTE_CASE_NONE,
                single_char: FALSE,
            };
            let mut casestart_offset: PCRE2_SIZE = 0;
            let mut casestart_extra_needed: PCRE2_SIZE = 0;

            if use_existing_match != FALSE {
                rc = (*match_data).rc;
                use_existing_match = FALSE;
            } else {
                rc = crate::matcher::pcre2_match_8(
                    code,
                    subject,
                    length,
                    start_offset,
                    options | goptions,
                    match_data,
                    mcontext,
                );
            }

            if utf != FALSE {
                options |= PCRE2_NO_UTF_CHECK; /* Only need to check once */
            }

            /* Any error other than no match returns the error code. No match breaks the
            global loop. */

            if rc == PCRE2_ERROR_NOMATCH {
                break 'GLOBAL;
            }

            if rc < 0 {
                break 'EXIT;
            }

            /* Handle a successful match. */

            if *ovector.add(1) < *ovector.add(0) || *ovector.add(0) < start_offset {
                rc = PCRE2_ERROR_BADSUBSPATTERN;
                break 'EXIT;
            }

            /* Assert that our replacement loop is making progress. */

            if subs > 0
                && !(*ovector.add(1) > ovecsave[1]
                    || (*ovector.add(1) == *ovector.add(0)
                        && ovecsave[1] > ovecsave[0]
                        && *ovector.add(1) == ovecsave[1]))
            {
                rc = PCRE2_ERROR_INTERNAL_DUPMATCH;
                break 'EXIT;
            }

            ovecsave[0] = *ovector.add(0);
            ovecsave[1] = *ovector.add(1);

            /* Count substitutions with a paranoid check for integer overflow. */

            if subs == c_int::MAX {
                rc = PCRE2_ERROR_TOOMANYREPLACE;
                break 'EXIT;
            }
            subs += 1;

            /* Copy the text leading up to the match (unless not required). */

            if rc == 0 {
                rc = ovector_count as c_int;
            }
            fraglength = *ovector.add(0) - start_offset;
            if replacement_only == FALSE {
                let r = CHECKMEMCPY(
                    subject.add(start_offset),
                    fraglength,
                    &mut overflowed,
                    &mut extra_needed,
                    &mut lengthleft,
                    &mut buff_offset,
                    buffer,
                    suboptions,
                );
                if r != 0 {
                    rc = r;
                    break 'EXIT;
                }
            }
            scb.output_offsets[0] = buff_offset;
            scb.oveccount = rc as u32;
            sub_start_extra_needed = extra_needed;

            /* Process the replacement string. If the entire replacement is literal,
            just copy it with length check. */

            ptr = replacement;
            if (suboptions & PCRE2_SUBSTITUTE_LITERAL) != 0 {
                let r = CHECKMEMCPY(
                    ptr,
                    rlength,
                    &mut overflowed,
                    &mut extra_needed,
                    &mut lengthleft,
                    &mut buff_offset,
                    buffer,
                    suboptions,
                );
                if r != 0 {
                    rc = r;
                    break 'EXIT;
                }
            }
            /* Within a non-literal replacement, which must be scanned character by
            character, local literal mode can be set by \Q, but only in extended mode
            when backslashes are being interpreted. */
            else {
                'REPL: loop {
                    let mut ch: u32 = 0;
                    let mut chlen: c_uint = 0;
                    let mut group: c_int = 0;
                    let mut special: u32 = 0;
                    let mut text1_start: PCRE2_SPTR = core::ptr::null();
                    let mut text1_end: PCRE2_SPTR = core::ptr::null();
                    let mut text2_start: PCRE2_SPTR = core::ptr::null();
                    let mut text2_end: PCRE2_SPTR = core::ptr::null();
                    let mut name: [PCRE2_UCHAR; MAX_NAME_SIZE as usize + 1] =
                        [0; MAX_NAME_SIZE as usize + 1];
                    let mut subptr: PCRE2_SPTR = core::ptr::null();
                    let mut subptrend: PCRE2_SPTR = core::ptr::null();
                    let mut sublength: PCRE2_SIZE = 0;
                    let mut action: u32 = ACT_NONE;

                    /* If at the end of a nested substring, pop the stack. */

                    if ptr >= repend {
                        if ptrstackptr == 0 {
                            break 'REPL; /* End of replacement string */
                        }
                        ptrstackptr -= 1;
                        repend = ptrstack[ptrstackptr as usize];
                        ptrstackptr -= 1;
                        ptr = ptrstack[ptrstackptr as usize];
                        continue 'REPL;
                    }

                    /* Handle the next character */

                    if escaped_literal != FALSE {
                        if *ptr.add(0) as u32 == CHAR_BACKSLASH
                            && ptr < repend.wrapping_sub(1)
                            && *ptr.add(1) as u32 == CHAR_E
                        {
                            escaped_literal = FALSE;
                            ptr = ptr.add(2);
                            continue 'REPL;
                        }
                        action = ACT_LOADLITERAL;
                    }
                    /* Not in literal mode. */
                    else if *ptr as u32 == CHAR_DOLLAR_SIGN {
                        let mut inparens: BOOL;
                        let mut inangle: BOOL;
                        let mut star: BOOL;
                        let mut next: PCRE2_UCHAR;

                        'DOLLAR: {
                            ptr = ptr.add(1);
                            if ptr >= repend {
                                /* BAD */
                                rc = PCRE2_ERROR_BADREPLACEMENT;
                                *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                break 'EXIT;
                            }
                            next = *ptr;
                            if next as u32 == CHAR_DOLLAR_SIGN {
                                action = ACT_LOADLITERAL;
                                break 'DOLLAR;
                            }

                            special = 0;
                            text1_start = core::ptr::null();
                            text1_end = core::ptr::null();
                            text2_start = core::ptr::null();
                            text2_end = core::ptr::null();
                            group = -1;
                            inparens = FALSE;
                            inangle = FALSE;
                            star = FALSE;
                            subptr = core::ptr::null();
                            subptrend = core::ptr::null();

                            /* Special $ sequences, as supported by Perl, JavaScript,
                            .NET and others. */
                            if next as u32 == CHAR_AMPERSAND {
                                ptr = ptr.add(1);
                                group = 0;
                                action = ACT_GROUP;
                                break 'DOLLAR;
                            }
                            if next as u32 == CHAR_GRAVE_ACCENT
                                || next as u32 == CHAR_APOSTROPHE
                            {
                                ptr = ptr.add(1);

                                /* (Sanity-check ovector before reading from it.) */
                                rc = crate::substring::pcre2_substring_length_bynumber_8(
                                    match_data,
                                    0,
                                    &mut sublength,
                                );
                                if rc < 0 {
                                    /* PTREXIT */
                                    *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                    break 'EXIT;
                                }

                                if next as u32 == CHAR_GRAVE_ACCENT {
                                    subptr = subject;
                                    subptrend = subject.add(*ovector.add(0));
                                } else {
                                    if partial != FALSE {
                                        rc = PCRE2_ERROR_PARTIALSUBS;
                                        *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                        break 'EXIT;
                                    }

                                    subptr = subject.add(*ovector.add(1));
                                    subptrend = subject.add(length);
                                }

                                action = ACT_SUBPTR;
                                break 'DOLLAR;
                            }
                            if next as u32 == CHAR_UNDERSCORE {
                                /* Java, .NET support $_ for "entire input string". */
                                ptr = ptr.add(1);

                                if partial != FALSE {
                                    rc = PCRE2_ERROR_PARTIALSUBS;
                                    *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                    break 'EXIT;
                                }

                                subptr = subject;
                                subptrend = subject.add(length);
                                action = ACT_SUBPTR;
                                break 'DOLLAR;
                            }
                            if next as u32 == CHAR_PLUS
                                && !(ptr.add(1) < repend
                                    && *ptr.add(1) as u32 == CHAR_LEFT_CURLY_BRACKET)
                            {
                                /* Perl supports $+ for "highest captured group". */
                                ptr = ptr.add(1);
                                if (*code).top_bracket == 0 {
                                    /* Treat either as "no such group" or "all groups
                                    unset" based on PCRE2_SUBSTITUTE_UNKNOWN_UNSET. */
                                    if (suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET) == 0 {
                                        rc = PCRE2_ERROR_NOSUBSTRING;
                                        *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                        break 'EXIT;
                                    }
                                    group = 0;
                                } else {
                                    /* If we have any capture groups, then the ovector
                                    needs to be large enough for all of them. */
                                    if ((*match_data).oveccount as c_int)
                                        < (*code).top_bracket as c_int + 1
                                    {
                                        rc = PCRE2_ERROR_UNAVAILABLE;
                                        *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                        break 'EXIT;
                                    }
                                    group = (*code).top_bracket as c_int;
                                    while group > 0 {
                                        if *ovector.add(2 * group as usize) != PCRE2_UNSET {
                                            break;
                                        }
                                        group -= 1;
                                    }
                                }
                                if group == 0 {
                                    if (suboptions & PCRE2_SUBSTITUTE_UNSET_EMPTY) != 0 {
                                        continue 'REPL;
                                    }
                                    rc = PCRE2_ERROR_UNSET;
                                    *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                    break 'EXIT;
                                }
                                action = ACT_GROUP;
                                break 'DOLLAR;
                            }

                            if next as u32 == CHAR_LEFT_CURLY_BRACKET {
                                ptr = ptr.add(1);
                                if ptr >= repend {
                                    rc = PCRE2_ERROR_BADREPLACEMENT;
                                    *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                    break 'EXIT;
                                }
                                next = *ptr;
                                inparens = TRUE;
                            } else if next as u32 == CHAR_LESS_THAN_SIGN {
                                /* JavaScript compatibility syntax, $<name>. */
                                ptr = ptr.add(1);
                                if ptr >= repend {
                                    rc = PCRE2_ERROR_BADREPLACEMENT;
                                    *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                    break 'EXIT;
                                }
                                next = *ptr;
                                inangle = TRUE;
                            }

                            if inangle == FALSE && next as u32 == CHAR_ASTERISK {
                                ptr = ptr.add(1);
                                if ptr >= repend {
                                    rc = PCRE2_ERROR_BADREPLACEMENT;
                                    *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                    break 'EXIT;
                                }
                                next = *ptr;
                                star = TRUE;
                            }

                            if star == FALSE
                                && inangle == FALSE
                                && next as u32 >= CHAR_0
                                && next as u32 <= CHAR_9
                            {
                                group = next as c_int - CHAR_0 as c_int;
                                loop {
                                    ptr = ptr.add(1);
                                    if !(ptr < repend) {
                                        break;
                                    }
                                    next = *ptr;
                                    if (next as u32) < CHAR_0 || next as u32 > CHAR_9 {
                                        break;
                                    }
                                    group = group * 10 + (next as c_int - CHAR_0 as c_int);

                                    /* A check for a number greater than the highest
                                    captured group is sufficient here. */

                                    if group > (*code).top_bracket as c_int {
                                        if (suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET) != 0 {
                                            loop {
                                                ptr = ptr.add(1);
                                                if !(ptr < repend
                                                    && *ptr as u32 >= CHAR_0
                                                    && *ptr as u32 <= CHAR_9)
                                                {
                                                    break;
                                                }
                                            }
                                            break;
                                        } else {
                                            rc = PCRE2_ERROR_NOSUBSTRING;
                                            *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                            break 'EXIT;
                                        }
                                    }
                                }
                            } else {
                                let name_len: PCRE2_SIZE;
                                let name_start: PCRE2_SPTR = ptr;
                                if read_name_subst(
                                    &mut ptr,
                                    repend,
                                    utf,
                                    (*code).tables.add(ctypes_offset),
                                ) == FALSE
                                {
                                    rc = PCRE2_ERROR_BADREPLACEMENT;
                                    *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                    break 'EXIT;
                                }
                                name_len = ptr.offset_from(name_start) as PCRE2_SIZE;
                                memcpy(
                                    name.as_mut_ptr() as *mut c_void,
                                    name_start as *const c_void,
                                    CU2BYTES(name_len),
                                );
                                name[name_len] = 0;
                            }

                            next = 0; /* not used or updated after this point */
                            let _ = next;

                            /* In extended mode we recognize ${name:+set text:unset text}
                            and ${name:-default text}. */

                            if inparens != FALSE {
                                if (suboptions & PCRE2_SUBSTITUTE_EXTENDED) != 0
                                    && star == FALSE
                                    && ptr < repend.wrapping_sub(2)
                                    && *ptr as u32 == CHAR_COLON
                                {
                                    ptr = ptr.add(1);
                                    special = *ptr as u32;
                                    if special != CHAR_PLUS && special != CHAR_MINUS {
                                        rc = PCRE2_ERROR_BADSUBSTITUTION;
                                        *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                        break 'EXIT;
                                    }

                                    ptr = ptr.add(1);
                                    text1_start = ptr;
                                    rc = find_text_end(
                                        code,
                                        &mut ptr,
                                        repend,
                                        (special == CHAR_MINUS) as BOOL,
                                    );
                                    if rc != 0 {
                                        *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                        break 'EXIT;
                                    }
                                    text1_end = ptr;

                                    if special == CHAR_PLUS && *ptr as u32 == CHAR_COLON {
                                        ptr = ptr.add(1);
                                        text2_start = ptr;
                                        rc = find_text_end(code, &mut ptr, repend, TRUE);
                                        if rc != 0 {
                                            *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                            break 'EXIT;
                                        }
                                        text2_end = ptr;
                                    }
                                } else {
                                    if ptr >= repend
                                        || *ptr as u32 != CHAR_RIGHT_CURLY_BRACKET
                                    {
                                        rc = PCRE2_ERROR_REPMISSINGBRACE;
                                        *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                        break 'EXIT;
                                    }
                                }

                                ptr = ptr.add(1);
                            }

                            if inangle != FALSE {
                                if ptr >= repend || *ptr as u32 != CHAR_GREATER_THAN_SIGN {
                                    rc = PCRE2_ERROR_BADREPLACEMENT;
                                    *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                    break 'EXIT;
                                }
                                ptr = ptr.add(1);
                            }

                            /* Have found a syntactically correct group number or name, or
                            *name. Only *MARK is currently recognized. */

                            if star != FALSE {
                                if crate::string_utils::_pcre2_strcmp_c8_8(
                                    name.as_ptr(),
                                    b"MARK\0".as_ptr() as *const c_char,
                                ) == 0
                                {
                                    let mark: PCRE2_SPTR =
                                        crate::match_data::pcre2_get_mark_8(match_data);
                                    if !mark.is_null() {
                                        /* Peek backwards one code unit to obtain the
                                        length of the mark. */
                                        fraglength = *mark.sub(1) as PCRE2_SIZE;
                                        if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                            && substitute_case_callout.is_none()
                                        {
                                            let r = CHECKCASECPY_DEFAULT(
                                                mark,
                                                fraglength,
                                                &mut overflowed,
                                                &mut extra_needed,
                                                &mut lengthleft,
                                                &mut buff_offset,
                                                buffer,
                                                suboptions,
                                                &mut forcecase,
                                                code,
                                            );
                                            if r != 0 {
                                                rc = r;
                                                break 'EXIT;
                                            }
                                        } else {
                                            let r = CHECKMEMCPY(
                                                mark,
                                                fraglength,
                                                &mut overflowed,
                                                &mut extra_needed,
                                                &mut lengthleft,
                                                &mut buff_offset,
                                                buffer,
                                                suboptions,
                                            );
                                            if r != 0 {
                                                rc = r;
                                                break 'EXIT;
                                            }
                                        }
                                    }
                                } else {
                                    rc = PCRE2_ERROR_BADREPLACEMENT;
                                    *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                    break 'EXIT;
                                }
                            }
                            /* Substitute the contents of a group. */
                            else {
                                action = ACT_GROUP;
                            }
                        } /* End of $ processing */
                    }
                    /* Handle an escape sequence in extended mode. */
                    else if (suboptions & PCRE2_SUBSTITUTE_EXTENDED) != 0
                        && *ptr as u32 == CHAR_BACKSLASH
                    {
                        let mut errorcode: c_int = 0;
                        let mut new_forcecase: case_state = case_state {
                            to_case: PCRE2_SUBSTITUTE_CASE_NONE,
                            single_char: FALSE,
                        };
                        let mut do_setforcecase: BOOL = FALSE;

                        'BSLASH: {
                            if ptr < repend.wrapping_sub(1) {
                                let nc: u32 = *ptr.add(1) as u32;
                                if nc == CHAR_L {
                                    new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
                                    new_forcecase.single_char = FALSE;
                                    ptr = ptr.add(2);
                                } else if nc == CHAR_l {
                                    new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
                                    new_forcecase.single_char = TRUE;
                                    ptr = ptr.add(2);
                                    if ptr.add(2) < repend
                                        && *ptr.add(0) as u32 == CHAR_BACKSLASH
                                        && *ptr.add(1) as u32 == CHAR_U
                                    {
                                        /* Perl reverse-title-casing feature for \l\U */
                                        new_forcecase.to_case =
                                            PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST;
                                        new_forcecase.single_char = FALSE;
                                        ptr = ptr.add(2);
                                    }
                                } else if nc == CHAR_U {
                                    new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
                                    new_forcecase.single_char = FALSE;
                                    ptr = ptr.add(2);
                                } else if nc == CHAR_u {
                                    new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_TITLE_FIRST;
                                    new_forcecase.single_char = TRUE;
                                    ptr = ptr.add(2);
                                    if ptr.add(2) < repend
                                        && *ptr.add(0) as u32 == CHAR_BACKSLASH
                                        && *ptr.add(1) as u32 == CHAR_L
                                    {
                                        /* Perl title-casing feature for \u\L */
                                        new_forcecase.to_case =
                                            PCRE2_SUBSTITUTE_CASE_TITLE_FIRST;
                                        new_forcecase.single_char = FALSE;
                                        ptr = ptr.add(2);
                                    }
                                }
                            }

                            if new_forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE {
                                do_setforcecase = TRUE;
                                break 'BSLASH; /* goto SETFORCECASE */
                            }

                            ptr = ptr.add(1); /* Point after \ */
                            rc = crate::compile_util::_pcre2_check_escape_8(
                                &mut ptr,
                                repend,
                                &mut ch,
                                &mut errorcode,
                                (*code).overall_options,
                                (*code).extra_options,
                                (*code).top_bracket as u32,
                                FALSE,
                                core::ptr::null_mut(),
                            );
                            if errorcode != 0 {
                                /* BADESCAPE */
                                rc = PCRE2_ERROR_BADREPESCAPE;
                                *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                break 'EXIT;
                            }

                            if rc == ESC_E {
                                do_setforcecase = TRUE;
                                break 'BSLASH; /* goto SETFORCECASE */
                            }

                            if rc == ESC_Q {
                                escaped_literal = TRUE;
                                continue 'REPL;
                            }

                            if rc == 0 || rc == ESC_b || rc == ESC_v {
                                if rc == ESC_b {
                                    ch = CHAR_BS;
                                }
                                if rc == ESC_v {
                                    ch = CHAR_VT;
                                }

                                if utf != FALSE {
                                    chlen = crate::ord2utf::_pcre2_ord2utf_8(
                                        ch,
                                        temp.as_mut_ptr(),
                                    );
                                } else {
                                    temp[0] = ch as PCRE2_UCHAR;
                                    chlen = 1;
                                }

                                if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                    && substitute_case_callout.is_none()
                                {
                                    let r = CHECKCASECPY_DEFAULT(
                                        temp.as_ptr(),
                                        chlen as PCRE2_SIZE,
                                        &mut overflowed,
                                        &mut extra_needed,
                                        &mut lengthleft,
                                        &mut buff_offset,
                                        buffer,
                                        suboptions,
                                        &mut forcecase,
                                        code,
                                    );
                                    if r != 0 {
                                        rc = r;
                                        break 'EXIT;
                                    }
                                } else {
                                    let r = CHECKMEMCPY(
                                        temp.as_ptr(),
                                        chlen as PCRE2_SIZE,
                                        &mut overflowed,
                                        &mut extra_needed,
                                        &mut lengthleft,
                                        &mut buff_offset,
                                        buffer,
                                        suboptions,
                                    );
                                    if r != 0 {
                                        rc = r;
                                        break 'EXIT;
                                    }
                                }
                                continue 'REPL;
                            }

                            if rc == ESC_g {
                                let name_len: PCRE2_SIZE;
                                let name_start: PCRE2_SPTR;

                                /* Parse the \g<name> form (\g<number> already handled
                                by check_escape) */
                                if ptr >= repend || *ptr as u32 != CHAR_LESS_THAN_SIGN {
                                    rc = PCRE2_ERROR_BADREPESCAPE;
                                    *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                    break 'EXIT;
                                }
                                ptr = ptr.add(1);

                                name_start = ptr;
                                if read_name_subst(
                                    &mut ptr,
                                    repend,
                                    utf,
                                    (*code).tables.add(ctypes_offset),
                                ) == FALSE
                                {
                                    rc = PCRE2_ERROR_BADREPESCAPE;
                                    *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                    break 'EXIT;
                                }
                                name_len = ptr.offset_from(name_start) as PCRE2_SIZE;

                                if ptr >= repend || *ptr as u32 != CHAR_GREATER_THAN_SIGN {
                                    rc = PCRE2_ERROR_BADREPESCAPE;
                                    *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                    break 'EXIT;
                                }
                                ptr = ptr.add(1);

                                special = 0;
                                group = -1;
                                memcpy(
                                    name.as_mut_ptr() as *mut c_void,
                                    name_start as *const c_void,
                                    CU2BYTES(name_len),
                                );
                                name[name_len] = 0;
                                action = ACT_GROUP; /* goto GROUP_SUBSTITUTE */
                                break 'BSLASH;
                            }

                            /* default */
                            if rc < 0 {
                                special = 0;
                                group = -rc - 1;
                                action = ACT_GROUP; /* goto GROUP_SUBSTITUTE */
                                break 'BSLASH;
                            }

                            /* BADESCAPE */
                            rc = PCRE2_ERROR_BADREPESCAPE;
                            *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                            break 'EXIT;
                        } /* End of backslash processing */

                        if do_setforcecase != FALSE {
                            /* SETFORCECASE:

                            If the substitute_case_callout is unset, our case-forcing is
                            done immediately. If there is a callout however, then its
                            action is delayed until all the characters have been
                            collected. Apply the callout now, before we set the new
                            casing mode. */

                            if substitute_case_callout.is_some()
                                && forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                            {
                                let r = DELAYEDFORCECASE(
                                    &mut buff_offset,
                                    casestart_offset,
                                    &mut extra_needed,
                                    casestart_extra_needed,
                                    &mut overflowed,
                                    &mut lengthleft,
                                    buffer,
                                    suboptions,
                                    &mut forcecase,
                                    utf,
                                    substitute_case_callout,
                                    substitute_case_callout_data,
                                );
                                if r != 0 {
                                    rc = r;
                                    break 'EXIT;
                                }
                            }

                            forcecase = new_forcecase;
                            casestart_offset = buff_offset;
                            casestart_extra_needed = extra_needed;
                            continue 'REPL;
                        }
                    }
                    /* Handle a literal code unit */
                    else {
                        action = ACT_LOADLITERAL;
                    }

                    /* LOADLITERAL: */

                    if action == ACT_LOADLITERAL {
                        let ch_start: PCRE2_SPTR = ptr;
                        /* GETCHARINCTEST(ch, ptr): get character value, increment
                        pointer */
                        ch = *ptr as u32;
                        ptr = ptr.add(1);
                        if utf != FALSE && ch >= 0xc0 {
                            let r = getutf8inc(ch, ptr);
                            ch = r.0;
                            ptr = r.1;
                        }
                        let _ = ch;

                        if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                            && substitute_case_callout.is_none()
                        {
                            let r = CHECKCASECPY_DEFAULT(
                                ch_start,
                                ptr.offset_from(ch_start) as PCRE2_SIZE,
                                &mut overflowed,
                                &mut extra_needed,
                                &mut lengthleft,
                                &mut buff_offset,
                                buffer,
                                suboptions,
                                &mut forcecase,
                                code,
                            );
                            if r != 0 {
                                rc = r;
                                break 'EXIT;
                            }
                        } else {
                            let r = CHECKMEMCPY(
                                ch_start,
                                ptr.offset_from(ch_start) as PCRE2_SIZE,
                                &mut overflowed,
                                &mut extra_needed,
                                &mut lengthleft,
                                &mut buff_offset,
                                buffer,
                                suboptions,
                            );
                            if r != 0 {
                                rc = r;
                                break 'EXIT;
                            }
                        }
                    } else if action == ACT_GROUP || action == ACT_SUBPTR {
                        if action == ACT_GROUP {
                            'GROUP: {
                                /* GROUP_SUBSTITUTE:
                                Find a number for a named group. In case there are
                                duplicate names, search for the first one that is set. */

                                if group < 0 {
                                    let mut first: PCRE2_SPTR = core::ptr::null();
                                    let mut last: PCRE2_SPTR = core::ptr::null();
                                    let mut entry: PCRE2_SPTR;
                                    rc = crate::substring::pcre2_substring_nametable_scan_8(
                                        code,
                                        name.as_ptr(),
                                        &mut first,
                                        &mut last,
                                    );
                                    if rc == PCRE2_ERROR_NOSUBSTRING
                                        && (suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET) != 0
                                    {
                                        group = (*code).top_bracket as c_int + 1;
                                    } else {
                                        if rc < 0 {
                                            *blength =
                                                ptr.offset_from(replacement) as PCRE2_SIZE;
                                            break 'EXIT;
                                        }
                                        entry = first;
                                        while entry <= last {
                                            let ng: u32 = GET2(entry, 0);
                                            if ng < ovector_count {
                                                if group < 0 {
                                                    group = ng as c_int; /* First in ovector */
                                                }
                                                if *ovector.add((ng * 2) as usize)
                                                    != PCRE2_UNSET
                                                {
                                                    group = ng as c_int; /* First that is set */
                                                    break;
                                                }
                                            }
                                            entry = entry.offset(rc as isize);
                                        }

                                        /* If group is still negative, it means we did not
                                        find a group that is in the ovector. Just set the
                                        first group. */

                                        if group < 0 {
                                            group = GET2(first, 0) as c_int;
                                        }
                                    }
                                }

                                /* We now have a group that is identified by number. */

                                rc = crate::substring::pcre2_substring_length_bynumber_8(
                                    match_data,
                                    group as u32,
                                    &mut sublength,
                                );
                                if rc < 0 {
                                    if rc == PCRE2_ERROR_NOSUBSTRING
                                        && (suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET) != 0
                                    {
                                        rc = PCRE2_ERROR_UNSET;
                                    }
                                    if rc != PCRE2_ERROR_UNSET {
                                        /* Non-unset errors */
                                        *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                        break 'EXIT;
                                    }
                                    if special == 0
                                    /* Plain substitution */
                                    {
                                        if (suboptions & PCRE2_SUBSTITUTE_UNSET_EMPTY) != 0 {
                                            continue 'REPL;
                                        }
                                        /* Else error */
                                        *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                        break 'EXIT;
                                    }
                                }

                                /* If special is '+' we have a 'set' and possibly an
                                'unset' text, both of which are reprocessed when used. If
                                special is '-' we have a default text for when the group
                                is unset; it must be reprocessed. */

                                if special != 0 {
                                    if special == CHAR_MINUS {
                                        if rc == 0 {
                                            break 'GROUP; /* goto LITERAL_SUBSTITUTE */
                                        }
                                        text2_start = text1_start;
                                        text2_end = text1_end;
                                    }

                                    if ptrstackptr as usize >= PTR_STACK_SIZE {
                                        rc = PCRE2_ERROR_BADREPLACEMENT;
                                        *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
                                        break 'EXIT;
                                    }
                                    ptrstack[ptrstackptr as usize] = ptr;
                                    ptrstackptr += 1;
                                    ptrstack[ptrstackptr as usize] = repend;
                                    ptrstackptr += 1;

                                    if rc == 0 {
                                        ptr = text1_start;
                                        repend = text1_end;
                                    } else {
                                        ptr = text2_start;
                                        repend = text2_end;
                                    }
                                    continue 'REPL;
                                }
                            }

                            /* LITERAL_SUBSTITUTE:
                            Otherwise we have a literal substitution of a group's
                            contents. */

                            subptr = subject.add(*ovector.add((group * 2) as usize));
                            subptrend = subject.add(*ovector.add((group * 2 + 1) as usize));
                        }

                        /* SUBPTR_SUBSTITUTE:
                        Substitute a literal string, possibly forcing alphabetic case. */

                        if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                            && substitute_case_callout.is_none()
                        {
                            let r = CHECKCASECPY_DEFAULT(
                                subptr,
                                subptrend.offset_from(subptr) as PCRE2_SIZE,
                                &mut overflowed,
                                &mut extra_needed,
                                &mut lengthleft,
                                &mut buff_offset,
                                buffer,
                                suboptions,
                                &mut forcecase,
                                code,
                            );
                            if r != 0 {
                                rc = r;
                                break 'EXIT;
                            }
                        } else {
                            let r = CHECKMEMCPY(
                                subptr,
                                subptrend.offset_from(subptr) as PCRE2_SIZE,
                                &mut overflowed,
                                &mut extra_needed,
                                &mut lengthleft,
                                &mut buff_offset,
                                buffer,
                                suboptions,
                            );
                            if r != 0 {
                                rc = r;
                                break 'EXIT;
                            }
                        }
                    }
                } /* End of loop for scanning the replacement. */
            }

            /* We now clean up any trailing section of the replacement for which we
            deferred the case-forcing. */

            if substitute_case_callout.is_some()
                && forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
            {
                let r = DELAYEDFORCECASE(
                    &mut buff_offset,
                    casestart_offset,
                    &mut extra_needed,
                    casestart_extra_needed,
                    &mut overflowed,
                    &mut lengthleft,
                    buffer,
                    suboptions,
                    &mut forcecase,
                    utf,
                    substitute_case_callout,
                    substitute_case_callout_data,
                );
                if r != 0 {
                    rc = r;
                    break 'EXIT;
                }
            }

            /* The replacement has been copied to the output, or its size has been
            remembered. Handle the callout if there is one. */

            if !mcontext.is_null() && (*mcontext).substitute_callout.is_some() {
                /* If we an actual (non-simulated) replacement, do the callout. */

                if overflowed == FALSE {
                    scb.subscount = subs as u32;
                    scb.output_offsets[1] = buff_offset;
                    rc = ((*mcontext).substitute_callout.unwrap())(
                        &mut scb as *mut pcre2_substitute_callout_block,
                        (*mcontext).substitute_callout_data,
                    );

                    /* A non-zero return means cancel this substitution. Instead, copy
                    the matched string fragment. */

                    if rc != 0 {
                        let newlength: PCRE2_SIZE =
                            scb.output_offsets[1] - scb.output_offsets[0];
                        let oldlength: PCRE2_SIZE = *ovector.add(1) - *ovector.add(0);

                        buff_offset -= newlength;
                        lengthleft += newlength;
                        if replacement_only == FALSE {
                            let r = CHECKMEMCPY(
                                subject.add(*ovector.add(0)),
                                oldlength,
                                &mut overflowed,
                                &mut extra_needed,
                                &mut lengthleft,
                                &mut buff_offset,
                                buffer,
                                suboptions,
                            );
                            if r != 0 {
                                rc = r;
                                break 'EXIT;
                            }
                        }

                        /* A negative return means do not do any more. */

                        if rc < 0 {
                            suboptions &= !PCRE2_SUBSTITUTE_GLOBAL;
                        }
                    }
                }
                /* In this interesting case, we cannot do the callout, so it's hard to
                estimate the required buffer size. */
                else {
                    let newlength_buf: PCRE2_SIZE = buff_offset - scb.output_offsets[0];
                    let newlength_extra: PCRE2_SIZE = extra_needed - sub_start_extra_needed;
                    let newlength: PCRE2_SIZE =
                        if newlength_extra > !(0 as PCRE2_SIZE) - newlength_buf {
                            /* Integer overflow */
                            !(0 as PCRE2_SIZE) /* Cap the addition */
                        } else {
                            newlength_buf + newlength_extra
                        };
                    let oldlength: PCRE2_SIZE = *ovector.add(1) - *ovector.add(0);

                    /* Be pessimistic: request whichever buffer size is larger out of
                    accepting or rejecting the substitution. */

                    if oldlength > newlength {
                        let additional: PCRE2_SIZE = oldlength - newlength;
                        if additional > !(0 as PCRE2_SIZE) - extra_needed
                        /* Integer overflow */
                        {
                            rc = PCRE2_ERROR_TOOLARGEREPLACE;
                            break 'EXIT;
                        }
                        extra_needed += additional;
                    }
                }
            }

            /* Exit the global loop if we are not in global mode, or if
            pcre2_next_match() indicates we have reached the end of the subject. */

            if (suboptions & PCRE2_SUBSTITUTE_GLOBAL) == 0
                || crate::match_next::pcre2_next_match_8(
                    match_data,
                    &mut start_offset,
                    &mut goptions,
                ) == 0
            {
                start_offset = *ovector.add(1);
                break 'GLOBAL;
            }
        } /* End of global loop */

        /* Copy the rest of the subject unless not required, and terminate the output
        with a binary zero. */

        if replacement_only == FALSE {
            fraglength = length - start_offset;
            let r = CHECKMEMCPY(
                subject.add(start_offset),
                fraglength,
                &mut overflowed,
                &mut extra_needed,
                &mut lengthleft,
                &mut buff_offset,
                buffer,
                suboptions,
            );
            if r != 0 {
                rc = r;
                break 'EXIT;
            }
        }

        temp[0] = 0;
        {
            let r = CHECKMEMCPY(
                temp.as_ptr(),
                1,
                &mut overflowed,
                &mut extra_needed,
                &mut lengthleft,
                &mut buff_offset,
                buffer,
                suboptions,
            );
            if r != 0 {
                rc = r;
                break 'EXIT;
            }
        }

        /* If overflowed is set it means the PCRE2_SUBSTITUTE_OVERFLOW_LENGTH is set,
        and matching has carried on after a full buffer, in order to compute the length
        needed. Otherwise, an overflow generates an immediate error return. */

        if overflowed != FALSE {
            rc = PCRE2_ERROR_NOMEMORY;

            if extra_needed > !(0 as PCRE2_SIZE) - buff_length
            /* Integer overflow */
            {
                rc = PCRE2_ERROR_TOOLARGEREPLACE;
                break 'EXIT;
            }
            *blength = buff_length + extra_needed;
        }
        /* After a successful execution, return the number of substitutions and set the
        length of buffer used, excluding the trailing zero. */
        else {
            rc = subs;
            *blength = buff_offset - 1;
        }
    }

    /* EXIT: */
    if !internal_match_data.is_null() {
        crate::match_data::pcre2_match_data_free_8(internal_match_data);
    } else {
        (*match_data).rc = rc;
    }
    rc
}

/* End of pcre2_substitute.c */
