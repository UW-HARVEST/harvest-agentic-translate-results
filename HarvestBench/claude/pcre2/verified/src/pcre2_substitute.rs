#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
use core::ffi::{c_char, c_int, c_void};
use core::ptr;
use crate::pcre2_internal::*;

use crate::pcre2_match::pcre2_match_8;
use crate::pcre2_match_data::{
    pcre2_get_mark_8, pcre2_get_ovector_count_8, pcre2_get_ovector_pointer_8,
    pcre2_match_data_create_8, pcre2_match_data_create_from_pattern_8, pcre2_match_data_free_8,
};
use crate::pcre2_substring::{
    pcre2_substring_length_bynumber_8, pcre2_substring_nametable_scan_8,
};
use crate::pcre2_compile::_pcre2_check_escape_8;
use crate::pcre2_ord2utf::_pcre2_ord2utf_8;
use crate::pcre2_string_utils::{_pcre2_strlen_8, _pcre2_strcmp_c8_8};
use crate::pcre2_valid_utf::_pcre2_valid_utf_8;

extern "C" {
    fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
}

type pcre2_code = pcre2_real_code;

const PTR_STACK_SIZE: usize = 20;

const PCRE2_SUBSTITUTE_CASE_NONE: u32 = 0;
const PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST: u32 = 4;

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
    code: *const pcre2_code,
    ptrptr: *mut *const u8,
    ptrend: *const u8,
    last: c_int,
) -> c_int {
    let mut rc: c_int = 0;
    let mut nestlevel: u32 = 0;
    let mut literal: c_int = 0; /* FALSE */
    let mut ptr: *const u8 = *ptrptr;
    let mut natural_end: c_int = 1; /* set to 0 on goto EXIT */

    while ptr < ptrend {
        if literal != 0 {
            if *ptr.offset(0) == CHAR_BACKSLASH as u8
                && ptr < ptrend.offset(-1)
                && *ptr.offset(1) == CHAR_E as u8
            {
                literal = 0; /* FALSE */
                ptr = ptr.offset(1);
            }
        } else if *ptr == CHAR_RIGHT_CURLY_BRACKET as u8 {
            if nestlevel == 0 {
                natural_end = 0;
                break;
            }
            nestlevel -= 1;
        } else if *ptr == CHAR_COLON as u8 && last == 0 && nestlevel == 0 {
            natural_end = 0;
            break;
        } else if *ptr == CHAR_DOLLAR_SIGN as u8 {
            if ptr < ptrend.offset(-1) && *ptr.offset(1) == CHAR_LEFT_CURLY_BRACKET as u8 {
                nestlevel += 1;
                ptr = ptr.offset(1);
            }
        } else if *ptr == CHAR_BACKSLASH as u8 {
            let erc: c_int;
            let mut errorcode: c_int = 0;
            let mut ch: u32 = 0;
            let esc_end_ptr: *const u8;

            if ptr < ptrend.offset(-1) {
                match *ptr.offset(1) as u32 {
                    CHAR_L | CHAR_l | CHAR_U | CHAR_u => {
                        ptr = ptr.offset(1);
                        ptr = ptr.offset(1); /* for-loop increment */
                        continue;
                    }
                    _ => {}
                }
            }

            ptr = ptr.offset(1); /* Must point after \ */
            erc = _pcre2_check_escape_8(
                &mut ptr,
                ptrend,
                &mut ch,
                &mut errorcode,
                (*code).overall_options,
                (*code).extra_options,
                (*code).top_bracket as u32,
                0, /* FALSE */
                core::ptr::null_mut(),
            );
            if errorcode != 0 {
                /* errorcode from check_escape is positive, so must not be returned by
                pcre2_substitute(). */
                rc = PCRE2_ERROR_BADREPESCAPE;
                natural_end = 0;
                break;
            }

            esc_end_ptr = ptr;
            ptr = ptr.offset(-1); /* Rewind by one, because the for-loop will increment it */

            match erc {
                0 | ESC_b | ESC_v | ESC_E => {}

                ESC_Q => {
                    literal = 1; /* TRUE */
                }

                ESC_g => {}

                _ => {
                    if erc < 0 {
                        /* capture group reference */
                    } else {
                        ptr = esc_end_ptr;
                        rc = PCRE2_ERROR_BADREPESCAPE;
                        natural_end = 0;
                        break;
                    }
                }
            }
        }
        ptr = ptr.offset(1);
    }

    if natural_end != 0 {
        rc = PCRE2_ERROR_REPMISSINGBRACE; /* Terminator not found */
    }

    /* EXIT: */
    *ptrptr = ptr;
    return rc;
}


/*************************************************
*        Advance the match (pcre2_next_match)    *
*************************************************/

/* Advance the offset by one code unit, and return the new value.
It is only called when the offset is not at the end of the subject. */

unsafe fn do_bumpalong(match_data: *mut pcre2_match_data, offset: PCRE2_SIZE) -> PCRE2_SIZE {
    let subject: PCRE2_SPTR = (*match_data).subject;
    let subject_length: PCRE2_SIZE = (*match_data).subject_length;
    let utf: BOOL = (((*(*match_data).code).overall_options & PCRE2_UTF) != 0) as BOOL;

    /* Skip over CRLF as an atomic sequence, if CRLF is configured as a newline
    sequence. */
    if *subject.add(offset) as u32 == CHAR_CR
        && offset + 1 < subject_length
        && *subject.add(offset + 1) as u32 == CHAR_LF
    {
        match (*(*match_data).code).newline_convention as u32 {
            PCRE2_NEWLINE_CRLF | PCRE2_NEWLINE_ANY | PCRE2_NEWLINE_ANYCRLF => {
                return offset + 2;
            }
            _ => {}
        }
    }

    /* Advance by one full character if in UTF mode. */
    if utf != FALSE {
        let mut next: PCRE2_SPTR = subject.add(offset + 1);
        let subject_end: PCRE2_SPTR = subject.add(subject_length);
        while next < subject_end && NOT_FIRSTCU(*next as u32) {
            next = next.offset(1);
        }
        return next.offset_from(subject) as PCRE2_SIZE;
    }

    offset + 1
}

/* Advance the match. Returns TRUE if further iteration is possible. */

unsafe fn pcre2_next_match(
    match_data: *mut pcre2_match_data,
    pstart_offset: *mut PCRE2_SIZE,
    poptions: *mut u32,
) -> BOOL {
    let rc: c_int = (*match_data).rc;
    let start_offset: PCRE2_SIZE = (*match_data).start_offset;
    let ovector: *mut PCRE2_SIZE = (*match_data).ovector.as_mut_ptr();

    /* Match error, or no match: no further iteration possible. */
    if rc < 0 {
        return FALSE;
    }

    /* Special handling for \K in lookarounds. */
    if *ovector.add(0) != start_offset && *ovector.add(1) == start_offset {
        if start_offset >= (*match_data).subject_length {
            return FALSE;
        }

        *pstart_offset = do_bumpalong(match_data, *ovector.add(1));
        *poptions = 0;
        return TRUE;
    }

    /* If the previous match was for an empty string. */
    if *ovector.add(0) == *ovector.add(1) {
        if *ovector.add(0) >= (*match_data).subject_length {
            return FALSE;
        }

        *pstart_offset = *ovector.add(1);
        *poptions = PCRE2_NOTEMPTY_ATSTART;
        return TRUE;
    }

    /* Non-empty match where the end is further on than start_offset. */
    *pstart_offset = *ovector.add(1);
    *poptions = 0;
    TRUE
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
    ptrptr: *mut *const u8,
    ptrend: *const u8,
    utf: BOOL,
    ctypes: *const u8,
) -> BOOL {
    let mut ptr: *const u8 = *ptrptr;
    let nameptr: *const u8 = ptr;
    let mut failed: BOOL = FALSE;

    if ptr >= ptrend {
        /* No characters in name */
        failed = TRUE;
    } else {
        /* We do not need to check whether the name starts with a non-digit.
        We are simply referencing names here, not defining them. */

        if utf != 0 {
            let mut c: u32;
            let mut typ: u32;

            while ptr < ptrend {
                c = GETCHAR(ptr);
                typ = UCD_CHARTYPE(c);
                if typ != ucp_Nd
                    && _pcre2_ucp_gentype_8[typ as usize] != ucp_L
                    && c != CHAR_UNDERSCORE
                {
                    break;
                }
                ptr = ptr.offset(1);
                /* FORWARDCHARTEST(ptr, ptrend) */
                if utf != 0 {
                    while ptr < ptrend && NOT_FIRSTCU(*ptr as u32) {
                        ptr = ptr.offset(1);
                    }
                }
            }
        } else {
            /* Handle group names in non-UTF modes. */
            while ptr < ptrend && (*(ctypes.add(*ptr as usize)) & ctype_word) != 0 {
                ptr = ptr.offset(1);
            }
        }

        /* Check name length */
        if (ptr as isize - nameptr as isize) as u32 > MAX_NAME_SIZE {
            failed = TRUE;
        } else if ptr == nameptr {
            /* Subpattern names must not be empty */
            failed = TRUE;
        }
    }

    *ptrptr = ptr;
    if failed != 0 {
        return FALSE;
    }
    return TRUE;
}


/*************************************************
*              Case transformations              *
*************************************************/

#[repr(C)]
#[derive(Clone, Copy)]
struct case_state {
    to_case: c_int, /* One of PCRE2_SUBSTITUTE_CASE_xyz */
    single_char: BOOL,
}

/* Helper to guess how much a string is likely to increase in size when
case-transformed. */

fn pessimistic_case_inflation(len: PCRE2_SIZE) -> PCRE2_SIZE {
    return (len >> 3u32) + 10;
}

/* Case transformation behaviour if no callout is passed. */

unsafe fn default_substitute_case_callout(
    mut input: PCRE2_SPTR,
    input_len: PCRE2_SIZE,
    mut output: *mut PCRE2_UCHAR,
    mut output_cap: PCRE2_SIZE,
    state: *mut case_state,
    code: *const pcre2_code,
) -> PCRE2_SIZE {
    let input_end: PCRE2_SPTR = input.add(input_len);
    let utf: BOOL;
    let ucp: BOOL;
    let mut temp: [PCRE2_UCHAR; 6] = [0; 6];
    let mut next_to_upper: BOOL;
    let rest_to_upper: BOOL;
    let single_char: BOOL;
    let mut overflow: BOOL = FALSE;
    let mut written: PCRE2_SIZE = 0;

    utf = if ((*code).overall_options & PCRE2_UTF) != 0 { 1 } else { 0 };
    ucp = if ((*code).overall_options & PCRE2_UCP) != 0 { 1 } else { 0 };

    if input_len == 0 {
        return 0;
    }

    match (*state).to_case {
        x if x == PCRE2_SUBSTITUTE_CASE_LOWER || x == PCRE2_SUBSTITUTE_CASE_UPPER => {
            next_to_upper = if (*state).to_case == PCRE2_SUBSTITUTE_CASE_UPPER { TRUE } else { FALSE };
            rest_to_upper = next_to_upper;
        }
        x if x == PCRE2_SUBSTITUTE_CASE_TITLE_FIRST => {
            next_to_upper = TRUE;
            rest_to_upper = FALSE;
            (*state).to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
        }
        x if x == PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST as c_int => {
            next_to_upper = FALSE;
            rest_to_upper = TRUE;
            (*state).to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
        }
        _ => {
            return 0;
        }
    }

    single_char = (*state).single_char;
    if single_char != 0 {
        (*state).to_case = PCRE2_SUBSTITUTE_CASE_NONE as c_int;
    }

    while input < input_end {
        let mut ch: u32;
        let chlen: u32;

        /* GETCHARINCTEST(ch, input); */
        if utf != 0 {
            let (c, adv) = GETCHARINC(input);
            ch = c;
            input = input.add(adv);
        } else {
            ch = *input as u32;
            input = input.add(1);
        }

        if (utf != 0 || ucp != 0) && ch >= 128 {
            let typ: u32 = UCD_CHARTYPE(ch);
            if _pcre2_ucp_gentype_8[typ as usize] == ucp_L
                && typ != (if next_to_upper != 0 { ucp_Lu } else { ucp_Ll })
            {
                ch = UCD_OTHERCASE(ch);
            }
        } else if ch <= 255 {
            /* MAX_255(ch) always true in 8-bit */
            let table = (*code).tables;
            let bitoff = cbits_offset
                + (if next_to_upper != 0 { cbit_upper } else { cbit_lower });
            if (*table.add(bitoff + (ch / 8) as usize) & (1u32 << (ch % 8)) as u8) == 0 {
                ch = *table.add(fcc_offset + ch as usize) as u32;
            }
        }

        if utf != 0 {
            chlen = _pcre2_ord2utf_8(ch, temp.as_mut_ptr());
        } else {
            temp[0] = ch as u8;
            chlen = 1;
        }

        if overflow == 0 && (chlen as PCRE2_SIZE) <= output_cap {
            memcpy(
                output as *mut c_void,
                temp.as_ptr() as *const c_void,
                CU2BYTES(chlen as usize),
            );
            output = output.add(chlen as usize);
            output_cap -= chlen as PCRE2_SIZE;
        } else {
            overflow = TRUE;
        }

        if (chlen as PCRE2_SIZE) > PCRE2_SIZE_MAX - written {
            /* Integer overflow */
            return PCRE2_SIZE_MAX;
        }
        written += chlen as PCRE2_SIZE;

        next_to_upper = rest_to_upper;

        /* memcpy the remainder, if only transforming a single character. */
        if single_char != 0 {
            let rest_len: PCRE2_SIZE = (input_end as usize) - (input as usize);

            if overflow == 0 && rest_len <= output_cap {
                memcpy(
                    output as *mut c_void,
                    input as *const c_void,
                    CU2BYTES(rest_len),
                );
            }

            if rest_len > PCRE2_SIZE_MAX - written {
                /* Integer overflow */
                return PCRE2_SIZE_MAX;
            }
            written += rest_len;

            return written;
        }
    }

    return written;
}


/* Helper to perform the call to the substitute_case_callout. We wrap the
user-provided callout because our internal arguments are slightly extended. We
don't want the user callout to handle the case of "\l" (first character only to
lowercase) or "\l\U" (first character to lowercase, rest to uppercase) because
those are not operations defined by Unicode. Instead the user callout simply
needs to provide the three Unicode primitives: lower, upper, titlecase. */

type CaseCalloutFnRaw = unsafe extern "C" fn(
    PCRE2_SPTR,
    PCRE2_SIZE,
    *mut PCRE2_UCHAR,
    PCRE2_SIZE,
    c_int,
    *mut c_void,
) -> PCRE2_SIZE;

unsafe fn do_case_copy(
    input_output: *mut PCRE2_UCHAR,
    input_len: PCRE2_SIZE,
    output_cap: PCRE2_SIZE,
    state: *mut case_state,
    utf: BOOL,
    substitute_case_callout: CaseCalloutFnRaw,
    substitute_case_callout_data: *mut c_void,
) -> PCRE2_SIZE {
    let input: PCRE2_SPTR = input_output;
    let output: *mut PCRE2_UCHAR = input_output;
    let mut rc: PCRE2_SIZE;
    let mut rc2: PCRE2_SIZE;
    let ch1_to_case: c_int;
    let rest_to_case: c_int;
    let mut ch1: [PCRE2_UCHAR; 6] = [0; 6];
    let ch1_len: PCRE2_SIZE;
    let mut rest: PCRE2_SPTR;
    let rest_len: PCRE2_SIZE;
    let mut ch1_overflow: BOOL = FALSE;
    let mut rest_overflow: BOOL = FALSE;

    let _ = utf;

    match (*state).to_case {
        x if x == PCRE2_SUBSTITUTE_CASE_LOWER
            || x == PCRE2_SUBSTITUTE_CASE_UPPER
            || x == PCRE2_SUBSTITUTE_CASE_TITLE_FIRST =>
        {
            /* The easy case, where our internal casing operations align with those of
            the callout. */
            if (*state).single_char == FALSE {
                rc = substitute_case_callout(
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
            rest_to_case = PCRE2_SUBSTITUTE_CASE_NONE as c_int;
        }
        x if x == PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST as c_int => {
            ch1_to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
            rest_to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
        }
        _ => {
            return 0;
        }
    }

    /* Identify the leading character. Take copy, because its storage overlaps with
    `output`, and hence may be scrambled by the callout. */
    {
        let mut ch_end: PCRE2_SPTR = input;
        /* GETCHARINCTEST(ch, ch_end) */
        if utf != FALSE {
            let (_c, adv) = GETCHARINC(ch_end);
            ch_end = ch_end.add(adv);
        } else {
            ch_end = ch_end.add(1);
        }
        ch1_len = (ch_end as usize - input as usize) as PCRE2_SIZE;
        memcpy(
            ch1.as_mut_ptr() as *mut c_void,
            input as *const c_void,
            CU2BYTES(ch1_len),
        );
    }

    rest = input.add(ch1_len);
    rest_len = input_len - ch1_len;

    /* Transform just ch1. The buffers are always in-place (input == output). */
    {
        let mut ch1_cap: PCRE2_SIZE;
        let max_ch1_cap: PCRE2_SIZE;

        ch1_cap = ch1_len; /* First attempt uses the space vacated by ch1. */
        max_ch1_cap = output_cap - rest_len;

        loop {
            rc = substitute_case_callout(
                ch1.as_ptr(),
                ch1_len,
                output,
                ch1_cap,
                ch1_to_case,
                substitute_case_callout_data,
            );
            if rc == PCRE2_SIZE_MAX {
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

    if rest_to_case == PCRE2_SUBSTITUTE_CASE_NONE as c_int {
        if ch1_overflow == FALSE {
            memmove(
                output.add(rc) as *mut c_void,
                rest as *const c_void,
                CU2BYTES(rest_len),
            );
        }
        rc2 = rest_len;

        (*state).to_case = PCRE2_SUBSTITUTE_CASE_NONE as c_int;
    } else {
        let mut dummy: [PCRE2_UCHAR; 1] = [0; 1];

        rc2 = substitute_case_callout(
            rest,
            rest_len,
            if ch1_overflow != FALSE {
                dummy.as_mut_ptr()
            } else {
                output.add(rc)
            },
            if ch1_overflow != FALSE {
                0
            } else {
                output_cap - rc
            },
            rest_to_case,
            substitute_case_callout_data,
        );
        if rc2 == PCRE2_SIZE_MAX {
            return rc2;
        }

        if ch1_overflow == FALSE && rc2 > output_cap - rc {
            rest_overflow = TRUE;
        }

        /* If ch1 grows so that `xform(ch1)+rest` can't fit in the buffer, but then
        `rest` shrinks, it's actually possible for the total calculated length of
        `xform(ch1)+xform(rest)` to come out at less than output_cap. */
        let rc2 = if ch1_overflow != FALSE && rc2 < rest_len {
            rest_len
        } else {
            rc2
        };

        (*state).to_case = PCRE2_SUBSTITUTE_CASE_UPPER;

        let _ = rest_overflow;
        if rc2 > PCRE2_SIZE_MAX - rc {
            /* Integer overflow */
            return PCRE2_SIZE_MAX;
        }
        return rc + rc2;
    }

    let _ = rest_overflow;
    if rc2 > PCRE2_SIZE_MAX - rc {
        /* Integer overflow */
        return PCRE2_SIZE_MAX;
    }

    rc + rc2
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

const INT_MAX: c_int = 2147483647;

/* Labels used to emulate the C goto-based cleanup flow. */
#[derive(PartialEq, Clone, Copy)]
enum SubLabel {
    Exit,
    NoRoom,
    CaseError,
    TooLargeReplace,
    PtrExit,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substitute_8(
    code: *const pcre2_code,
    mut subject: PCRE2_SPTR,
    mut length: PCRE2_SIZE,
    mut start_offset: PCRE2_SIZE,
    mut options: u32,
    mut match_data: *mut pcre2_match_data,
    mcontext: *mut pcre2_match_context,
    mut replacement: PCRE2_SPTR,
    mut rlength: PCRE2_SIZE,
    buffer: *mut PCRE2_UCHAR,
    blength: *mut PCRE2_SIZE,
) -> c_int {
    let mut rc: c_int;
    let mut subs: c_int;
    let ovector_count: u32;
    let mut goptions: u32 = 0;
    let mut suboptions: u32 = 0;
    let mut internal_match_data: *mut pcre2_match_data = ptr::null_mut();
    let mut escaped_literal: BOOL = FALSE;
    let mut overflowed: BOOL = FALSE;
    let mut use_existing_match: BOOL;
    let replacement_only: BOOL;
    let utf: BOOL = (((*code).overall_options & PCRE2_UTF) != 0) as BOOL;
    let partial: BOOL =
        ((options & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) != 0) as BOOL;
    let mut temp: [PCRE2_UCHAR; 6] = [0; 6];
    let mut null_str: [PCRE2_UCHAR; 1] = [0xcd];
    let original_subject: PCRE2_SPTR = subject;
    let mut ptr: PCRE2_SPTR;
    let mut repend: PCRE2_SPTR;
    let mut extra_needed: PCRE2_SIZE = 0;
    let mut buff_offset: PCRE2_SIZE;
    let buff_length: PCRE2_SIZE;
    let mut lengthleft: PCRE2_SIZE;
    let mut fraglength: PCRE2_SIZE;
    let ovector: *mut PCRE2_SIZE;
    let mut ovecsave: [PCRE2_SIZE; 2] = [0, 0];
    let mut scb: pcre2_substitute_callout_block = core::mem::zeroed();
    let mut sub_start_extra_needed: PCRE2_SIZE = 0;
    let mut substitute_case_callout: Option<CaseCalloutFnRaw> = None;
    let mut substitute_case_callout_data: *mut c_void = ptr::null_mut();

    /* General initialization */
    buff_offset = 0;
    buff_length = *blength;
    lengthleft = buff_length;
    *blength = PCRE2_UNSET;

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
        rlength = _pcre2_strlen_8(replacement);
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
        length = _pcre2_strlen_8(subject);
    }

    /* Check for using a match that has already happened. */
    use_existing_match = ((options & PCRE2_SUBSTITUTE_MATCHED) != 0) as BOOL;
    replacement_only = ((options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY) != 0) as BOOL;

    if use_existing_match != FALSE && match_data.is_null() {
        return PCRE2_ERROR_NULL;
    }

    /* Validate an existing match against the passed-in parameters. */
    if use_existing_match != FALSE {
        if (*match_data).rc < 0 && (*match_data).rc != PCRE2_ERROR_NOMATCH {
            return (*match_data).rc;
        }

        if (*match_data).matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER {
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

    /* Create an internal match_data block if needed. */
    if match_data.is_null() {
        let mut gcontext: pcre2_general_context = core::mem::zeroed();
        gcontext.memctl = if mcontext.is_null() {
            (*code).memctl
        } else {
            (*mcontext).memctl
        };
        internal_match_data =
            pcre2_match_data_create_from_pattern_8(code, &mut gcontext);
        match_data = internal_match_data;
        if internal_match_data.is_null() {
            return PCRE2_ERROR_NOMEMORY;
        }
    } else if use_existing_match != FALSE {
        let pairs: c_int;
        let mut gcontext: pcre2_general_context = core::mem::zeroed();
        gcontext.memctl = if mcontext.is_null() {
            (*code).memctl
        } else {
            (*mcontext).memctl
        };
        pairs = if ((*code).top_bracket as c_int + 1) < (*match_data).oveccount as c_int {
            (*code).top_bracket as c_int + 1
        } else {
            (*match_data).oveccount as c_int
        };
        internal_match_data =
            pcre2_match_data_create_8((*match_data).oveccount as u32, &mut gcontext);
        if internal_match_data.is_null() {
            return PCRE2_ERROR_NOMEMORY;
        }
        memcpy(
            internal_match_data as *mut c_void,
            match_data as *const c_void,
            match_data_ovector_offset()
                + 2 * (pairs as usize) * core::mem::size_of::<PCRE2_SIZE>(),
        );
        (*internal_match_data).heapframes = ptr::null_mut();
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
    ovector = pcre2_get_ovector_pointer_8(match_data);
    ovector_count = pcre2_get_ovector_count_8(match_data);

    /* Fixed things in the callout block */
    scb.version = 0;
    scb.input = subject;
    scb.output = buffer as PCRE2_SPTR;
    scb.ovector = ovector;

    rc = 0;
    subs = 0;
    ptr = replacement;
    repend = replacement.add(rlength);

    /* The overall body runs inside a labelled block. Its value indicates which
    C label the flow jumped to (EXIT / NOROOM / CASEERROR / TOOLARGEREPLACE /
    PTREXIT). Falling off the end represents the normal (no goto) completion. */
    let terminal: Option<SubLabel> = 'body: {
        /* Check UTF replacement string if necessary. */
        if utf != FALSE && (options & PCRE2_NO_UTF_CHECK) == 0 {
            rc = _pcre2_valid_utf_8(replacement, rlength, &mut (*match_data).startchar);
            if rc != 0 {
                (*match_data).leftchar = 0;
                break 'body Some(SubLabel::Exit);
            }
        }

        /* Save the substitute options and remove them from the match options. */
        suboptions = options & SUBSTITUTE_OPTIONS;
        options &= !SUBSTITUTE_OPTIONS;

        /* Error if the start match offset is greater than the length of the subject. */
        if start_offset > length {
            (*match_data).leftchar = 0;
            rc = PCRE2_ERROR_BADOFFSET;
            break 'body Some(SubLabel::Exit);
        }

        /* CHECKMEMCPY: checks for space in the buffer before copying. On overflow,
        either give an error immediately, or keep on, accumulating the length.
        Evaluates to Some(label) to indicate a goto, None to continue. */
        macro_rules! checkmemcpy {
            ($from:expr, $length_:expr) => {{
                let chkmc_length: PCRE2_SIZE = $length_;
                let mut jump: Option<SubLabel> = None;
                if overflowed != FALSE {
                    if chkmc_length > PCRE2_SIZE_MAX - extra_needed {
                        jump = Some(SubLabel::TooLargeReplace);
                    } else {
                        extra_needed += chkmc_length;
                    }
                } else if lengthleft < chkmc_length {
                    if (suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH) == 0 {
                        jump = Some(SubLabel::NoRoom);
                    } else {
                        overflowed = TRUE;
                        extra_needed = chkmc_length - lengthleft;
                    }
                } else {
                    memcpy(
                        buffer.add(buff_offset) as *mut c_void,
                        $from as *const c_void,
                        CU2BYTES(chkmc_length),
                    );
                    buff_offset += chkmc_length;
                    lengthleft -= chkmc_length;
                }
                jump
            }};
        }

        /* Copy up to the start offset, unless only the replacement is required. */
        if replacement_only == FALSE {
            if let Some(l) = checkmemcpy!(subject, start_offset) {
                break 'body Some(l);
            }
        }

        /* Loop for global substituting. */
        subs = 0;
        'global: loop {
            let mut ptrstack: [PCRE2_SPTR; PTR_STACK_SIZE] = [ptr::null(); PTR_STACK_SIZE];
            let mut ptrstackptr: u32 = 0;
            let mut forcecase: case_state = case_state {
                to_case: PCRE2_SUBSTITUTE_CASE_NONE as c_int,
                single_char: FALSE,
            };
            let mut casestart_offset: PCRE2_SIZE = 0;
            let mut casestart_extra_needed: PCRE2_SIZE = 0;

            if use_existing_match != FALSE {
                rc = (*match_data).rc;
                use_existing_match = FALSE;
            } else {
                rc = pcre2_match_8(
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

            /* Any error other than no match returns the error code. No match breaks
            the global loop. */
            if rc == PCRE2_ERROR_NOMATCH {
                break 'global;
            }

            if rc < 0 {
                break 'body Some(SubLabel::Exit);
            }

            /* Handle a successful match. */
            if *ovector.add(1) < *ovector.add(0) || *ovector.add(0) < start_offset {
                rc = PCRE2_ERROR_BADSUBSPATTERN;
                break 'body Some(SubLabel::Exit);
            }

            /* Assert that our replacement loop is making progress. */
            if subs > 0
                && !(*ovector.add(1) > ovecsave[1]
                    || (*ovector.add(1) == *ovector.add(0)
                        && ovecsave[1] > ovecsave[0]
                        && *ovector.add(1) == ovecsave[1]))
            {
                rc = PCRE2_ERROR_INTERNAL_DUPMATCH;
                break 'body Some(SubLabel::Exit);
            }

            ovecsave[0] = *ovector.add(0);
            ovecsave[1] = *ovector.add(1);

            /* Count substitutions with a paranoid check for integer overflow. */
            if subs == INT_MAX {
                rc = PCRE2_ERROR_TOOMANYREPLACE;
                break 'body Some(SubLabel::Exit);
            }
            subs += 1;

            /* Copy the text leading up to the match (unless not required). */
            if rc == 0 {
                rc = ovector_count as c_int;
            }
            fraglength = *ovector.add(0) - start_offset;
            if replacement_only == FALSE {
                if let Some(l) = checkmemcpy!(subject.add(start_offset), fraglength) {
                    break 'body Some(l);
                }
            }
            scb.output_offsets[0] = buff_offset;
            scb.oveccount = rc as u32;
            sub_start_extra_needed = extra_needed;

            /* Process the replacement string. */
            ptr = replacement;
            if (suboptions & PCRE2_SUBSTITUTE_LITERAL) != 0 {
                if let Some(l) = checkmemcpy!(ptr, rlength) {
                    break 'body Some(l);
                }
            } else {
                /* Scan the replacement character by character. We emulate the C
                gotos (LOADLITERAL, GROUP_SUBSTITUTE, LITERAL_SUBSTITUTE,
                SUBPTR_SUBSTITUTE, SETFORCECASE, BAD, BADESCAPE, PTREXIT) using a
                per-iteration section dispatcher. */
                #[derive(PartialEq, Clone, Copy)]
                enum Sec {
                    Classify,
                    LoadLiteral,
                    GroupSubstitute,
                    LiteralSubstitute,
                    SubptrSubstitute,
                    SetForceCase,
                }

                'replace: loop {
                    let mut ch: u32 = 0;
                    let mut chlen: u32;
                    let mut group: c_int = -1;
                    let mut special: u32 = 0;
                    let mut text1_start: PCRE2_SPTR = ptr::null();
                    let mut text1_end: PCRE2_SPTR = ptr::null();
                    let mut text2_start: PCRE2_SPTR = ptr::null();
                    let mut text2_end: PCRE2_SPTR = ptr::null();
                    let mut name: [PCRE2_UCHAR; (MAX_NAME_SIZE + 1) as usize] =
                        [0; (MAX_NAME_SIZE + 1) as usize];
                    let mut subptr: PCRE2_SPTR = ptr::null();
                    let mut subptrend: PCRE2_SPTR = ptr::null();
                    let mut sublength: PCRE2_SIZE = 0;
                    let mut new_forcecase: case_state = case_state {
                        to_case: PCRE2_SUBSTITUTE_CASE_NONE as c_int,
                        single_char: FALSE,
                    };
                    let _ = &mut sublength;

                    /* If at the end of a nested substring, pop the stack. */
                    if ptr >= repend {
                        if ptrstackptr == 0 {
                            break 'replace; /* End of replacement string */
                        }
                        ptrstackptr -= 1;
                        repend = ptrstack[ptrstackptr as usize];
                        ptrstackptr -= 1;
                        ptr = ptrstack[ptrstackptr as usize];
                        continue 'replace;
                    }

                    let mut sec = Sec::Classify;

                    'dispatch: loop {
                        match sec {
                            Sec::Classify => {
                                /* Handle the next character. */
                                if escaped_literal != FALSE {
                                    if *ptr.add(0) as u32 == CHAR_BACKSLASH
                                        && ptr < repend.offset(-1)
                                        && *ptr.add(1) as u32 == CHAR_E
                                    {
                                        escaped_literal = FALSE;
                                        ptr = ptr.add(2);
                                        continue 'replace;
                                    }
                                    sec = Sec::LoadLiteral;
                                    continue 'dispatch;
                                }

                                /* Not in literal mode. */
                                if *ptr as u32 == CHAR_DOLLAR_SIGN {
                                    let mut inparens: BOOL = FALSE;
                                    let mut inangle: BOOL = FALSE;
                                    let mut star: BOOL = FALSE;
                                    let mut next: PCRE2_UCHAR;

                                    ptr = ptr.add(1);
                                    if ptr >= repend {
                                        rc = PCRE2_ERROR_BADREPLACEMENT;
                                        break 'body Some(SubLabel::PtrExit);
                                    }
                                    next = *ptr;
                                    if next as u32 == CHAR_DOLLAR_SIGN {
                                        sec = Sec::LoadLiteral;
                                        continue 'dispatch;
                                    }

                                    special = 0;
                                    text1_start = ptr::null();
                                    text1_end = ptr::null();
                                    text2_start = ptr::null();
                                    text2_end = ptr::null();
                                    group = -1;
                                    inparens = FALSE;
                                    inangle = FALSE;
                                    star = FALSE;
                                    subptr = ptr::null();
                                    subptrend = ptr::null();

                                    /* Special $ sequences. */
                                    if next as u32 == CHAR_AMPERSAND {
                                        ptr = ptr.add(1);
                                        group = 0;
                                        sec = Sec::GroupSubstitute;
                                        continue 'dispatch;
                                    }
                                    if next as u32 == CHAR_GRAVE_ACCENT
                                        || next as u32 == CHAR_APOSTROPHE
                                    {
                                        ptr = ptr.add(1);

                                        rc = pcre2_substring_length_bynumber_8(
                                            match_data,
                                            0,
                                            &mut sublength,
                                        );
                                        if rc < 0 {
                                            break 'body Some(SubLabel::PtrExit);
                                        }

                                        if next as u32 == CHAR_GRAVE_ACCENT {
                                            subptr = subject;
                                            subptrend = subject.add(*ovector.add(0));
                                        } else {
                                            if partial != FALSE {
                                                rc = PCRE2_ERROR_PARTIALSUBS;
                                                break 'body Some(SubLabel::PtrExit);
                                            }
                                            subptr = subject.add(*ovector.add(1));
                                            subptrend = subject.add(length);
                                        }

                                        sec = Sec::SubptrSubstitute;
                                        continue 'dispatch;
                                    }
                                    if next as u32 == CHAR_UNDERSCORE {
                                        ptr = ptr.add(1);

                                        if partial != FALSE {
                                            rc = PCRE2_ERROR_PARTIALSUBS;
                                            break 'body Some(SubLabel::PtrExit);
                                        }

                                        subptr = subject;
                                        subptrend = subject.add(length);
                                        sec = Sec::SubptrSubstitute;
                                        continue 'dispatch;
                                    }
                                    if next as u32 == CHAR_PLUS
                                        && !(ptr.add(1) < repend
                                            && *ptr.add(1) as u32 == CHAR_LEFT_CURLY_BRACKET)
                                    {
                                        ptr = ptr.add(1);
                                        if (*code).top_bracket == 0 {
                                            if (suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET) == 0 {
                                                rc = PCRE2_ERROR_NOSUBSTRING;
                                                break 'body Some(SubLabel::PtrExit);
                                            }
                                            group = 0;
                                        } else {
                                            if (*match_data).oveccount < (*code).top_bracket + 1 {
                                                rc = PCRE2_ERROR_UNAVAILABLE;
                                                break 'body Some(SubLabel::PtrExit);
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
                                                continue 'replace;
                                            }
                                            rc = PCRE2_ERROR_UNSET;
                                            break 'body Some(SubLabel::PtrExit);
                                        }
                                        sec = Sec::GroupSubstitute;
                                        continue 'dispatch;
                                    }

                                    if next as u32 == CHAR_LEFT_CURLY_BRACKET {
                                        ptr = ptr.add(1);
                                        if ptr >= repend {
                                            rc = PCRE2_ERROR_BADREPLACEMENT;
                                            break 'body Some(SubLabel::PtrExit);
                                        }
                                        next = *ptr;
                                        inparens = TRUE;
                                    } else if next as u32 == CHAR_LESS_THAN_SIGN {
                                        ptr = ptr.add(1);
                                        if ptr >= repend {
                                            rc = PCRE2_ERROR_BADREPLACEMENT;
                                            break 'body Some(SubLabel::PtrExit);
                                        }
                                        next = *ptr;
                                        inangle = TRUE;
                                    }

                                    if inangle == FALSE && next as u32 == CHAR_ASTERISK {
                                        ptr = ptr.add(1);
                                        if ptr >= repend {
                                            rc = PCRE2_ERROR_BADREPLACEMENT;
                                            break 'body Some(SubLabel::PtrExit);
                                        }
                                        next = *ptr;
                                        star = TRUE;
                                    }

                                    if star == FALSE
                                        && inangle == FALSE
                                        && next as u32 >= CHAR_0
                                        && next as u32 <= CHAR_9
                                    {
                                        group = (next as u32 - CHAR_0) as c_int;
                                        loop {
                                            ptr = ptr.add(1);
                                            if ptr >= repend {
                                                break;
                                            }
                                            next = *ptr;
                                            if (next as u32) < CHAR_0 || next as u32 > CHAR_9 {
                                                break;
                                            }
                                            group = group * 10 + (next as u32 - CHAR_0) as c_int;

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
                                                    break 'body Some(SubLabel::PtrExit);
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
                                            break 'body Some(SubLabel::PtrExit);
                                        }
                                        name_len =
                                            (ptr as usize - name_start as usize) as PCRE2_SIZE;
                                        memcpy(
                                            name.as_mut_ptr() as *mut c_void,
                                            name_start as *const c_void,
                                            CU2BYTES(name_len),
                                        );
                                        name[name_len as usize] = 0;
                                    }

                                    next = 0;
                                    let _ = next;

                                    /* Extended: ${name:+set:unset}, ${name:-default}. */
                                    if inparens != FALSE {
                                        if (suboptions & PCRE2_SUBSTITUTE_EXTENDED) != 0
                                            && star == FALSE
                                            && ptr < repend.offset(-2)
                                            && *ptr as u32 == CHAR_COLON
                                        {
                                            ptr = ptr.add(1);
                                            special = *ptr as u32;
                                            if special != CHAR_PLUS && special != CHAR_MINUS {
                                                rc = PCRE2_ERROR_BADSUBSTITUTION;
                                                break 'body Some(SubLabel::PtrExit);
                                            }

                                            ptr = ptr.add(1);
                                            text1_start = ptr;
                                            rc = find_text_end(
                                                code,
                                                &mut ptr,
                                                repend,
                                                (special == CHAR_MINUS) as c_int,
                                            );
                                            if rc != 0 {
                                                break 'body Some(SubLabel::PtrExit);
                                            }
                                            text1_end = ptr;

                                            if special == CHAR_PLUS && *ptr as u32 == CHAR_COLON {
                                                ptr = ptr.add(1);
                                                text2_start = ptr;
                                                rc = find_text_end(code, &mut ptr, repend, TRUE);
                                                if rc != 0 {
                                                    break 'body Some(SubLabel::PtrExit);
                                                }
                                                text2_end = ptr;
                                            }
                                        } else {
                                            if ptr >= repend
                                                || *ptr as u32 != CHAR_RIGHT_CURLY_BRACKET
                                            {
                                                rc = PCRE2_ERROR_REPMISSINGBRACE;
                                                break 'body Some(SubLabel::PtrExit);
                                            }
                                        }

                                        ptr = ptr.add(1);
                                    }

                                    if inangle != FALSE {
                                        if ptr >= repend
                                            || *ptr as u32 != CHAR_GREATER_THAN_SIGN
                                        {
                                            rc = PCRE2_ERROR_BADREPLACEMENT;
                                            break 'body Some(SubLabel::PtrExit);
                                        }
                                        ptr = ptr.add(1);
                                    }

                                    /* Only *MARK is currently recognized. */
                                    if star != FALSE {
                                        if _pcre2_strcmp_c8_8(
                                            name.as_ptr(),
                                            b"MARK\0".as_ptr() as *const c_char,
                                        ) == 0
                                        {
                                            let mark: PCRE2_SPTR = pcre2_get_mark_8(match_data);
                                            if !mark.is_null() {
                                                fraglength = *mark.offset(-1) as PCRE2_SIZE;
                                                if forcecase.to_case
                                                    != PCRE2_SUBSTITUTE_CASE_NONE as c_int
                                                    && substitute_case_callout.is_none()
                                                {
                                                    if let Some(l) = checkcasecpy_default_fn(
                                                        mark,
                                                        fraglength,
                                                        buffer,
                                                        &mut buff_offset,
                                                        &mut lengthleft,
                                                        &mut extra_needed,
                                                        &mut overflowed,
                                                        &mut forcecase,
                                                        code,
                                                        suboptions,
                                                    ) {
                                                        break 'body Some(l);
                                                    }
                                                } else if let Some(l) =
                                                    checkmemcpy!(mark, fraglength)
                                                {
                                                    break 'body Some(l);
                                                }
                                            }
                                            /* End of $ processing */
                                            continue 'replace;
                                        } else {
                                            rc = PCRE2_ERROR_BADREPLACEMENT;
                                            break 'body Some(SubLabel::PtrExit);
                                        }
                                    } else {
                                        sec = Sec::GroupSubstitute;
                                        continue 'dispatch;
                                    }
                                }
                                /* Handle an escape sequence in extended mode. */
                                else if (suboptions & PCRE2_SUBSTITUTE_EXTENDED) != 0
                                    && *ptr as u32 == CHAR_BACKSLASH
                                {
                                    let mut errorcode: c_int = 0;
                                    new_forcecase = case_state {
                                        to_case: PCRE2_SUBSTITUTE_CASE_NONE as c_int,
                                        single_char: FALSE,
                                    };

                                    if ptr < repend.offset(-1) {
                                        match *ptr.add(1) as u32 {
                                            CHAR_L => {
                                                new_forcecase.to_case =
                                                    PCRE2_SUBSTITUTE_CASE_LOWER;
                                                new_forcecase.single_char = FALSE;
                                                ptr = ptr.add(2);
                                            }
                                            CHAR_l => {
                                                new_forcecase.to_case =
                                                    PCRE2_SUBSTITUTE_CASE_LOWER;
                                                new_forcecase.single_char = TRUE;
                                                ptr = ptr.add(2);
                                                if ptr.add(2) < repend
                                                    && *ptr.add(0) as u32 == CHAR_BACKSLASH
                                                    && *ptr.add(1) as u32 == CHAR_U
                                                {
                                                    new_forcecase.to_case =
                                                        PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST
                                                            as c_int;
                                                    new_forcecase.single_char = FALSE;
                                                    ptr = ptr.add(2);
                                                }
                                            }
                                            CHAR_U => {
                                                new_forcecase.to_case =
                                                    PCRE2_SUBSTITUTE_CASE_UPPER;
                                                new_forcecase.single_char = FALSE;
                                                ptr = ptr.add(2);
                                            }
                                            CHAR_u => {
                                                new_forcecase.to_case =
                                                    PCRE2_SUBSTITUTE_CASE_TITLE_FIRST;
                                                new_forcecase.single_char = TRUE;
                                                ptr = ptr.add(2);
                                                if ptr.add(2) < repend
                                                    && *ptr.add(0) as u32 == CHAR_BACKSLASH
                                                    && *ptr.add(1) as u32 == CHAR_L
                                                {
                                                    new_forcecase.to_case =
                                                        PCRE2_SUBSTITUTE_CASE_TITLE_FIRST;
                                                    new_forcecase.single_char = FALSE;
                                                    ptr = ptr.add(2);
                                                }
                                            }
                                            _ => {}
                                        }
                                    }

                                    if new_forcecase.to_case
                                        != PCRE2_SUBSTITUTE_CASE_NONE as c_int
                                    {
                                        sec = Sec::SetForceCase;
                                        continue 'dispatch;
                                    }

                                    ptr = ptr.add(1); /* Point after \ */
                                    rc = _pcre2_check_escape_8(
                                        &mut ptr,
                                        repend,
                                        &mut ch,
                                        &mut errorcode,
                                        (*code).overall_options,
                                        (*code).extra_options,
                                        (*code).top_bracket as u32,
                                        FALSE,
                                        ptr::null_mut(),
                                    );
                                    if errorcode != 0 {
                                        rc = PCRE2_ERROR_BADREPESCAPE;
                                        break 'body Some(SubLabel::PtrExit);
                                    }

                                    if rc == ESC_E {
                                        sec = Sec::SetForceCase;
                                        continue 'dispatch;
                                    } else if rc == ESC_Q {
                                        escaped_literal = TRUE;
                                        continue 'replace;
                                    } else if rc == 0 || rc == ESC_b || rc == ESC_v {
                                        if rc == ESC_b {
                                            ch = CHAR_BS;
                                        }
                                        if rc == ESC_v {
                                            ch = CHAR_VT;
                                        }

                                        if utf != FALSE {
                                            chlen = _pcre2_ord2utf_8(ch, temp.as_mut_ptr());
                                        } else {
                                            temp[0] = ch as PCRE2_UCHAR;
                                            chlen = 1;
                                        }

                                        if forcecase.to_case
                                            != PCRE2_SUBSTITUTE_CASE_NONE as c_int
                                            && substitute_case_callout.is_none()
                                        {
                                            if let Some(l) = checkcasecpy_default_fn(
                                                temp.as_ptr(),
                                                chlen as PCRE2_SIZE,
                                                buffer,
                                                &mut buff_offset,
                                                &mut lengthleft,
                                                &mut extra_needed,
                                                &mut overflowed,
                                                &mut forcecase,
                                                code,
                                                suboptions,
                                            ) {
                                                break 'body Some(l);
                                            }
                                        } else if let Some(l) =
                                            checkmemcpy!(temp.as_ptr(), chlen as PCRE2_SIZE)
                                        {
                                            break 'body Some(l);
                                        }
                                        continue 'replace;
                                    } else if rc == ESC_g {
                                        let name_len: PCRE2_SIZE;
                                        let name_start: PCRE2_SPTR;

                                        if ptr >= repend || *ptr as u32 != CHAR_LESS_THAN_SIGN {
                                            rc = PCRE2_ERROR_BADREPESCAPE;
                                            break 'body Some(SubLabel::PtrExit);
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
                                            break 'body Some(SubLabel::PtrExit);
                                        }
                                        name_len =
                                            (ptr as usize - name_start as usize) as PCRE2_SIZE;

                                        if ptr >= repend
                                            || *ptr as u32 != CHAR_GREATER_THAN_SIGN
                                        {
                                            rc = PCRE2_ERROR_BADREPESCAPE;
                                            break 'body Some(SubLabel::PtrExit);
                                        }
                                        ptr = ptr.add(1);

                                        special = 0;
                                        group = -1;
                                        memcpy(
                                            name.as_mut_ptr() as *mut c_void,
                                            name_start as *const c_void,
                                            CU2BYTES(name_len),
                                        );
                                        name[name_len as usize] = 0;
                                        sec = Sec::GroupSubstitute;
                                        continue 'dispatch;
                                    } else {
                                        if rc < 0 {
                                            special = 0;
                                            group = -rc - 1;
                                            sec = Sec::GroupSubstitute;
                                            continue 'dispatch;
                                        }
                                        rc = PCRE2_ERROR_BADREPESCAPE;
                                        break 'body Some(SubLabel::PtrExit);
                                    }
                                }
                                /* Handle a literal code unit. */
                                else {
                                    sec = Sec::LoadLiteral;
                                    continue 'dispatch;
                                }
                            }

                            Sec::GroupSubstitute => {
                                /* Find a number for a named group. */
                                if group < 0 {
                                    let mut first: PCRE2_SPTR = ptr::null();
                                    let mut last: PCRE2_SPTR = ptr::null();
                                    rc = pcre2_substring_nametable_scan_8(
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
                                            break 'body Some(SubLabel::PtrExit);
                                        }
                                        let mut entry: PCRE2_SPTR = first;
                                        while entry <= last {
                                            let ng: u32 = GET2(entry, 0);
                                            if ng < ovector_count {
                                                if group < 0 {
                                                    group = ng as c_int;
                                                }
                                                if *ovector.add(ng as usize * 2) != PCRE2_UNSET {
                                                    group = ng as c_int;
                                                    break;
                                                }
                                            }
                                            entry = entry.add(rc as usize);
                                        }

                                        if group < 0 {
                                            group = GET2(first, 0) as c_int;
                                        }
                                    }
                                }

                                /* We now have a group identified by number. */
                                rc = pcre2_substring_length_bynumber_8(
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
                                        break 'body Some(SubLabel::PtrExit);
                                    }
                                    if special == 0 {
                                        if (suboptions & PCRE2_SUBSTITUTE_UNSET_EMPTY) != 0 {
                                            continue 'replace;
                                        }
                                        break 'body Some(SubLabel::PtrExit);
                                    }
                                }

                                /* Handle set/unset/default reprocessed text. */
                                if special != 0 {
                                    let mut do_literal_substitute = false;
                                    if special == CHAR_MINUS {
                                        if rc == 0 {
                                            do_literal_substitute = true;
                                        } else {
                                            text2_start = text1_start;
                                            text2_end = text1_end;
                                        }
                                    }

                                    if !do_literal_substitute {
                                        if ptrstackptr >= PTR_STACK_SIZE as u32 {
                                            rc = PCRE2_ERROR_BADREPLACEMENT;
                                            break 'body Some(SubLabel::PtrExit);
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
                                        continue 'replace;
                                    }
                                }

                                sec = Sec::LiteralSubstitute;
                                continue 'dispatch;
                            }

                            Sec::LiteralSubstitute => {
                                subptr = subject.add(*ovector.add(group as usize * 2));
                                subptrend = subject.add(*ovector.add(group as usize * 2 + 1));
                                sec = Sec::SubptrSubstitute;
                                continue 'dispatch;
                            }

                            Sec::SubptrSubstitute => {
                                if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE as c_int
                                    && substitute_case_callout.is_none()
                                {
                                    if let Some(l) = checkcasecpy_default_fn(
                                        subptr,
                                        (subptrend as usize - subptr as usize) as PCRE2_SIZE,
                                        buffer,
                                        &mut buff_offset,
                                        &mut lengthleft,
                                        &mut extra_needed,
                                        &mut overflowed,
                                        &mut forcecase,
                                        code,
                                        suboptions,
                                    ) {
                                        break 'body Some(l);
                                    }
                                } else if let Some(l) = checkmemcpy!(
                                    subptr,
                                    (subptrend as usize - subptr as usize) as PCRE2_SIZE
                                ) {
                                    break 'body Some(l);
                                }
                                continue 'replace;
                            }

                            Sec::SetForceCase => {
                                if substitute_case_callout.is_some()
                                    && forcecase.to_case
                                        != PCRE2_SUBSTITUTE_CASE_NONE as c_int
                                {
                                    if let Some(l) = delayedforcecase_outer(
                                        &mut forcecase,
                                        &mut buff_offset,
                                        casestart_offset,
                                        &mut extra_needed,
                                        casestart_extra_needed,
                                        &mut overflowed,
                                        &mut lengthleft,
                                        suboptions,
                                        buffer,
                                        utf,
                                        substitute_case_callout.unwrap(),
                                        substitute_case_callout_data,
                                    ) {
                                        break 'body Some(l);
                                    }
                                }

                                forcecase = new_forcecase;
                                casestart_offset = buff_offset;
                                casestart_extra_needed = extra_needed;
                                continue 'replace;
                            }

                            Sec::LoadLiteral => {
                                let ch_start: PCRE2_SPTR = ptr;
                                /* GETCHARINCTEST(ch, ptr) */
                                if utf != FALSE {
                                    let (_c, adv) = GETCHARINC(ptr);
                                    ptr = ptr.add(adv);
                                } else {
                                    ptr = ptr.add(1);
                                }

                                if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE as c_int
                                    && substitute_case_callout.is_none()
                                {
                                    if let Some(l) = checkcasecpy_default_fn(
                                        ch_start,
                                        (ptr as usize - ch_start as usize) as PCRE2_SIZE,
                                        buffer,
                                        &mut buff_offset,
                                        &mut lengthleft,
                                        &mut extra_needed,
                                        &mut overflowed,
                                        &mut forcecase,
                                        code,
                                        suboptions,
                                    ) {
                                        break 'body Some(l);
                                    }
                                } else if let Some(l) = checkmemcpy!(
                                    ch_start,
                                    (ptr as usize - ch_start as usize) as PCRE2_SIZE
                                ) {
                                    break 'body Some(l);
                                }
                                continue 'replace;
                            }
                        }
                    } /* End of 'dispatch loop */
                } /* End of 'replace loop */
            }

            /* Clean up any trailing deferred case-forcing. */
            if substitute_case_callout.is_some()
                && forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE as c_int
            {
                if let Some(l) = delayedforcecase_outer(
                    &mut forcecase,
                    &mut buff_offset,
                    casestart_offset,
                    &mut extra_needed,
                    casestart_extra_needed,
                    &mut overflowed,
                    &mut lengthleft,
                    suboptions,
                    buffer,
                    utf,
                    substitute_case_callout.unwrap(),
                    substitute_case_callout_data,
                ) {
                    break 'body Some(l);
                }
            }

            /* Handle the substitute callout if there is one. */
            if !mcontext.is_null() && (*mcontext).substitute_callout.is_some() {
                if overflowed == FALSE {
                    scb.subscount = subs as u32;
                    scb.output_offsets[1] = buff_offset;
                    rc = (*mcontext).substitute_callout.unwrap()(
                        &mut scb,
                        (*mcontext).substitute_callout_data,
                    );

                    /* A non-zero return means cancel this substitution. */
                    if rc != 0 {
                        let newlength: PCRE2_SIZE =
                            scb.output_offsets[1] - scb.output_offsets[0];
                        let oldlength: PCRE2_SIZE = *ovector.add(1) - *ovector.add(0);

                        buff_offset -= newlength;
                        lengthleft += newlength;
                        if replacement_only == FALSE {
                            if let Some(l) =
                                checkmemcpy!(subject.add(*ovector.add(0)), oldlength)
                            {
                                break 'body Some(l);
                            }
                        }

                        if rc < 0 {
                            suboptions &= !PCRE2_SUBSTITUTE_GLOBAL;
                        }
                    }
                } else {
                    let newlength_buf: PCRE2_SIZE = buff_offset - scb.output_offsets[0];
                    let newlength_extra: PCRE2_SIZE = extra_needed - sub_start_extra_needed;
                    let newlength: PCRE2_SIZE =
                        if newlength_extra > PCRE2_SIZE_MAX - newlength_buf {
                            PCRE2_SIZE_MAX
                        } else {
                            newlength_buf + newlength_extra
                        };
                    let oldlength: PCRE2_SIZE = *ovector.add(1) - *ovector.add(0);

                    if oldlength > newlength {
                        let additional: PCRE2_SIZE = oldlength - newlength;
                        if additional > PCRE2_SIZE_MAX - extra_needed {
                            break 'body Some(SubLabel::TooLargeReplace);
                        }
                        extra_needed += additional;
                    }
                }
            }

            /* Exit the global loop if not global or reached the end. */
            if (suboptions & PCRE2_SUBSTITUTE_GLOBAL) == 0
                || pcre2_next_match(match_data, &mut start_offset, &mut goptions) == FALSE
            {
                start_offset = *ovector.add(1);
                break 'global;
            }
        } /* End of global loop */

        /* Copy the rest of the subject unless not required, then terminate. */
        if replacement_only == FALSE {
            fraglength = length - start_offset;
            if let Some(l) = checkmemcpy!(subject.add(start_offset), fraglength) {
                break 'body Some(l);
            }
        }

        temp[0] = 0;
        if let Some(l) = checkmemcpy!(temp.as_ptr(), 1) {
            break 'body Some(l);
        }

        if overflowed != FALSE {
            rc = PCRE2_ERROR_NOMEMORY;

            if extra_needed > PCRE2_SIZE_MAX - buff_length {
                break 'body Some(SubLabel::TooLargeReplace);
            }
            *blength = buff_length + extra_needed;
        } else {
            rc = subs;
            *blength = buff_offset - 1;
        }

        None
    };

    /* Dispatch the terminal labels. */
    match terminal {
        Some(SubLabel::NoRoom) => {
            rc = PCRE2_ERROR_NOMEMORY;
        }
        Some(SubLabel::CaseError) => {
            rc = PCRE2_ERROR_REPLACECASE;
        }
        Some(SubLabel::TooLargeReplace) => {
            rc = PCRE2_ERROR_TOOLARGEREPLACE;
        }
        Some(SubLabel::PtrExit) => {
            *blength = (ptr as usize - replacement as usize) as PCRE2_SIZE;
        }
        Some(SubLabel::Exit) | None => {}
    }

    /* EXIT: */
    if !internal_match_data.is_null() {
        pcre2_match_data_free_8(internal_match_data);
    } else {
        (*match_data).rc = rc;
    }
    rc
}

/* Helper wrapping DELAYEDFORCECASE for use where the loop-local macro is not in
scope. Mirrors the same logic. */
unsafe fn delayedforcecase_outer(
    forcecase: *mut case_state,
    buff_offset: *mut PCRE2_SIZE,
    casestart_offset: PCRE2_SIZE,
    extra_needed: *mut PCRE2_SIZE,
    casestart_extra_needed: PCRE2_SIZE,
    overflowed: *mut BOOL,
    lengthleft: *mut PCRE2_SIZE,
    suboptions: u32,
    buffer: *mut PCRE2_UCHAR,
    utf: BOOL,
    substitute_case_callout: CaseCalloutFnRaw,
    substitute_case_callout_data: *mut c_void,
) -> Option<SubLabel> {
    let chars_outstanding: PCRE2_SIZE =
        (*buff_offset - casestart_offset) + (*extra_needed - casestart_extra_needed);
    if chars_outstanding > 0 {
        if *overflowed != FALSE {
            let guess = pessimistic_case_inflation(chars_outstanding);
            if guess > PCRE2_SIZE_MAX - *extra_needed {
                return Some(SubLabel::TooLargeReplace);
            }
            *extra_needed += guess;
        } else {
            *lengthleft += *buff_offset - casestart_offset;
            *buff_offset = casestart_offset;
            let chkcc_length: PCRE2_SIZE = chars_outstanding;
            let chkcc_rc = do_case_copy(
                buffer.add(*buff_offset),
                chkcc_length,
                *lengthleft,
                forcecase,
                utf,
                substitute_case_callout,
                substitute_case_callout_data,
            );
            if chkcc_rc == PCRE2_SIZE_MAX {
                return Some(SubLabel::CaseError);
            }
            if *lengthleft < chkcc_rc {
                if (suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH) == 0 {
                    return Some(SubLabel::NoRoom);
                }
                *overflowed = TRUE;
                *extra_needed = chkcc_rc - *lengthleft;
            } else {
                *buff_offset += chkcc_rc;
                *lengthleft -= chkcc_rc;
            }
        }
    }
    None
}

/* CHECKCASECPY_DEFAULT: space check + casing via the default (built-in) case
handler. Returns Some(label) to indicate a goto, or None to continue. */
unsafe fn checkcasecpy_default_fn(
    from: PCRE2_SPTR,
    length_: PCRE2_SIZE,
    buffer: *mut PCRE2_UCHAR,
    buff_offset: *mut PCRE2_SIZE,
    lengthleft: *mut PCRE2_SIZE,
    extra_needed: *mut PCRE2_SIZE,
    overflowed: *mut BOOL,
    forcecase: *mut case_state,
    code: *const pcre2_code,
    suboptions: u32,
) -> Option<SubLabel> {
    let chkcc_length: PCRE2_SIZE = length_;
    let chkcc_rc = default_substitute_case_callout(
        from,
        chkcc_length,
        buffer.add(*buff_offset),
        if *overflowed != FALSE { 0 } else { *lengthleft },
        forcecase,
        code,
    );
    if *overflowed != FALSE {
        if chkcc_rc > PCRE2_SIZE_MAX - *extra_needed {
            return Some(SubLabel::TooLargeReplace);
        }
        *extra_needed += chkcc_rc;
    } else if *lengthleft < chkcc_rc {
        if (suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH) == 0 {
            return Some(SubLabel::NoRoom);
        }
        *overflowed = TRUE;
        *extra_needed = chkcc_rc - *lengthleft;
    } else {
        *buff_offset += chkcc_rc;
        *lengthleft -= chkcc_rc;
    }
    None
}

/* End of pcre2_substitute.rs */
