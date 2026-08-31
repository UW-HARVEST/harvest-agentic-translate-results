//! Translation of `c_src/src/pcre2_substitute.c`.

#![allow(non_snake_case, non_upper_case_globals, unused_parens, unused_assignments)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::chars::*;
use crate::internal::*;
use crate::match_data::{
    pcre2_get_mark_8, pcre2_get_ovector_count_8, pcre2_get_ovector_pointer_8,
    pcre2_match_data_create_8, pcre2_match_data_create_from_pattern_8, pcre2_match_data_free_8,
};
use crate::match_next::pcre2_next_match_8;
use crate::opcodes::*;
use crate::ord2utf::ord2utf;
use crate::string_utils::{strcmp_c8, strlen};
use crate::substring::{pcre2_substring_length_bynumber_8, pcre2_substring_nametable_scan_8};
use crate::ucp::{ucp_L, ucp_Ll, ucp_Lu, ucp_Nd};

/* MAX_NAME_SIZE, from config.h. */
const MAX_NAME_SIZE: usize = 128;

const PTR_STACK_SIZE: usize = 20;

const SUBSTITUTE_OPTIONS: u32 = PCRE2_SUBSTITUTE_EXTENDED
    | PCRE2_SUBSTITUTE_GLOBAL
    | PCRE2_SUBSTITUTE_LITERAL
    | PCRE2_SUBSTITUTE_MATCHED
    | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
    | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY
    | PCRE2_SUBSTITUTE_UNKNOWN_UNSET
    | PCRE2_SUBSTITUTE_UNSET_EMPTY;

const INT_MAX: c_int = c_int::MAX;

/*************************************************
*           Find end of substitute text          *
*************************************************/

/* In extended mode, we recognize ${name:+set text:unset text} and similar
constructions. This requires the identification of unescaped : and }
characters. This function scans for such. It must deal with nested ${
constructions. The pointer to the text is updated, either to the required end
character, or to where an error was detected. */

unsafe fn find_text_end(
    code: *const pcre2_real_code,
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    last: BOOL,
) -> c_int {
    unsafe {
        let mut rc: c_int = 0;
        let mut nestlevel: u32 = 0;
        let mut literal: BOOL = FALSE;
        let mut ptr: PCRE2_SPTR = *ptrptr;

        while ptr < ptrend {
            if literal != FALSE {
                if *ptr.add(0) as u32 == CHAR_BACKSLASH
                    && ptr < ptrend.sub(1)
                    && *ptr.add(1) as u32 == CHAR_E
                {
                    literal = FALSE;
                    ptr = ptr.add(1);
                }
            } else if *ptr as u32 == CHAR_RIGHT_CURLY_BRACKET {
                if nestlevel == 0 {
                    break;
                }
                nestlevel -= 1;
            } else if *ptr as u32 == CHAR_COLON && last == FALSE && nestlevel == 0 {
                break;
            } else if *ptr as u32 == CHAR_DOLLAR_SIGN {
                if ptr < ptrend.sub(1) && *ptr.add(1) as u32 == CHAR_LEFT_CURLY_BRACKET {
                    nestlevel += 1;
                    ptr = ptr.add(1);
                }
            } else if *ptr as u32 == CHAR_BACKSLASH {
                let erc: c_int;
                let mut errorcode: c_int = 0;
                let mut ch: u32 = 0;
                let esc_end_ptr: PCRE2_SPTR;
                let mut skip = false;

                if ptr < ptrend.sub(1) {
                    match *ptr.add(1) as u32 {
                        CHAR_L | CHAR_l | CHAR_U | CHAR_u => {
                            ptr = ptr.add(1);
                            skip = true;
                        }
                        _ => {}
                    }
                }

                if !skip {
                    ptr = ptr.add(1); /* Must point after \ */
                    erc = crate::compile_tables::check_escape(
                        &mut ptr,
                        ptrend,
                        &mut ch,
                        &mut errorcode,
                        (*code).overall_options,
                        (*code).extra_options,
                        (*code).top_bracket as u32,
                        FALSE,
                        ptr::null_mut(),
                    );
                    if errorcode != 0 {
                        /* errorcode from check_escape is positive, so must not be
                        returned by pcre2_substitute(). */
                        rc = PCRE2_ERROR_BADREPESCAPE;
                        break;
                    }

                    esc_end_ptr = ptr;
                    ptr = ptr.sub(1); /* Rewind by one; the for-loop will increment it */

                    match erc {
                        0 | ESC_b | ESC_v | ESC_E => {
                            /* Data character / isolated \E ignored */
                        }
                        ESC_Q => {
                            literal = TRUE;
                        }
                        ESC_g => {
                            /* The \g<name> form. Super lenient here. */
                        }
                        _ => {
                            if erc < 0 {
                                /* capture group reference */
                            } else {
                                ptr = esc_end_ptr;
                                rc = PCRE2_ERROR_BADREPESCAPE;
                                break;
                            }
                        }
                    }
                }
            }

            ptr = ptr.add(1);
        }

        if ptr >= ptrend && rc == 0 {
            rc = PCRE2_ERROR_REPMISSINGBRACE; /* Terminator not found */
        }

        *ptrptr = ptr;
        rc
    }
}

/*************************************************
*           Validate group name                  *
*************************************************/

/* This function scans for a capture group name, validating it consists of
legal characters, is not empty, and does not exceed MAX_NAME_SIZE. */

unsafe fn read_name_subst(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    ctypes: *const u8,
) -> BOOL {
    unsafe {
        let mut ptr: PCRE2_SPTR = *ptrptr;
        let nameptr: PCRE2_SPTR = ptr;

        if ptr >= ptrend {
            /* No characters in name */
            *ptrptr = ptr;
            return FALSE;
        }

        if utf != FALSE {
            while ptr < ptrend {
                let c = getchar_(ptr);
                let type_ = ucd_chartype(c);
                if type_ != ucp_Nd
                    && UCP_GENTYPE[type_ as usize] != ucp_L
                    && c != CHAR_UNDERSCORE
                {
                    break;
                }
                ptr = ptr.add(1);
                forwardchartest(&mut ptr, ptrend);
            }
        } else {
            /* Handle group names in non-UTF modes. */
            while ptr < ptrend
                && max_255(*ptr as u32)
                && (*ctypes.add(*ptr as usize) & ctype_word) != 0
            {
                ptr = ptr.add(1);
            }
        }

        /* Check name length */
        if ptr.offset_from(nameptr) as usize > MAX_NAME_SIZE {
            *ptrptr = ptr;
            return FALSE;
        }

        /* Subpattern names must not be empty */
        if ptr == nameptr {
            *ptrptr = ptr;
            return FALSE;
        }

        *ptrptr = ptr;
        TRUE
    }
}

/*************************************************
*              Case transformations              *
*************************************************/

const PCRE2_SUBSTITUTE_CASE_NONE: c_int = 0;
/* 1, 2, 3 are PCRE2_SUBSTITUTE_CASE_LOWER, UPPER, TITLE_FIRST. */
const PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST: c_int = 4;

#[derive(Clone, Copy)]
struct case_state {
    to_case: c_int, /* One of PCRE2_SUBSTITUTE_CASE_xyz */
    single_char: BOOL,
}

/* Helper to guess how much a string is likely to increase in size when
case-transformed. Estimate +10%, plus another few characters. */

fn pessimistic_case_inflation(len: PCRE2_SIZE) -> PCRE2_SIZE {
    (len >> 3u32) + 10
}

/* Case transformation behaviour if no callout is passed. */

unsafe fn default_substitute_case_callout(
    mut input: PCRE2_SPTR,
    input_len: PCRE2_SIZE,
    mut output: *mut PCRE2_UCHAR,
    mut output_cap: PCRE2_SIZE,
    state: *mut case_state,
    code: *const pcre2_real_code,
) -> PCRE2_SIZE {
    unsafe {
        let input_end: PCRE2_SPTR = input.add(input_len);
        let utf: BOOL = ((*code).overall_options & PCRE2_UTF != 0) as BOOL;
        let ucp: BOOL = ((*code).overall_options & PCRE2_UCP != 0) as BOOL;
        let mut temp: [PCRE2_UCHAR; 6] = [0; 6];
        let mut next_to_upper: BOOL;
        let rest_to_upper: BOOL;
        let single_char: BOOL;
        let mut overflow: BOOL = FALSE;
        let mut written: PCRE2_SIZE = 0;

        if input_len == 0 {
            return 0;
        }

        match (*state).to_case {
            PCRE2_SUBSTITUTE_CASE_LOWER | PCRE2_SUBSTITUTE_CASE_UPPER => {
                next_to_upper = ((*state).to_case == PCRE2_SUBSTITUTE_CASE_UPPER) as BOOL;
                rest_to_upper = next_to_upper;
            }
            PCRE2_SUBSTITUTE_CASE_TITLE_FIRST => {
                next_to_upper = TRUE;
                rest_to_upper = FALSE;
                (*state).to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
            }
            PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST => {
                next_to_upper = FALSE;
                rest_to_upper = TRUE;
                (*state).to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
            }
            _ => {
                return 0;
            }
        }

        single_char = (*state).single_char;
        if single_char != FALSE {
            (*state).to_case = PCRE2_SUBSTITUTE_CASE_NONE;
        }

        while input < input_end {
            let mut ch: u32;
            let chlen: u32;

            ch = getcharinctest(&mut input, utf != FALSE);

            if (utf != FALSE || ucp != FALSE) && ch >= 128 {
                let type_ = ucd_chartype(ch);
                if UCP_GENTYPE[type_ as usize] == ucp_L
                    && type_ != (if next_to_upper != FALSE { ucp_Lu } else { ucp_Ll })
                {
                    ch = ucd_othercase(ch);
                }
            } else if max_255(ch) {
                if (*(*code).tables.add(
                    cbits_offset
                        + (if next_to_upper != FALSE { cbit_upper } else { cbit_lower })
                        + (ch / 8) as usize,
                ) & (1u32 << (ch % 8)) as u8)
                    == 0
                {
                    ch = *(*code).tables.add(fcc_offset + ch as usize) as u32;
                }
            }

            if utf != FALSE {
                chlen = ord2utf(ch, temp.as_mut_ptr());
            } else {
                temp[0] = ch as u8;
                chlen = 1;
            }

            if overflow == FALSE && chlen as PCRE2_SIZE <= output_cap {
                memcpy(output, temp.as_ptr(), cu2bytes(chlen as usize));
                output = output.add(chlen as usize);
                output_cap -= chlen as PCRE2_SIZE;
            } else {
                overflow = TRUE;
            }

            if chlen as PCRE2_SIZE > !(0 as PCRE2_SIZE) - written {
                return !(0 as PCRE2_SIZE);
            }
            written += chlen as PCRE2_SIZE;

            next_to_upper = rest_to_upper;

            /* memcpy the remainder, if only transforming a single character. */
            if single_char != FALSE {
                let rest_len: PCRE2_SIZE = input_end.offset_from(input) as PCRE2_SIZE;

                if overflow == FALSE && rest_len <= output_cap {
                    memcpy(output, input, cu2bytes(rest_len));
                }

                if rest_len > !(0 as PCRE2_SIZE) - written {
                    return !(0 as PCRE2_SIZE);
                }
                written += rest_len;

                return written;
            }
        }

        written
    }
}

/* Helper to perform the call to the substitute_case_callout. */

unsafe fn do_case_copy(
    input_output: *mut PCRE2_UCHAR,
    input_len: PCRE2_SIZE,
    output_cap: PCRE2_SIZE,
    state: *mut case_state,
    _utf: BOOL,
    substitute_case_callout: unsafe extern "C" fn(
        PCRE2_SPTR,
        PCRE2_SIZE,
        *mut PCRE2_UCHAR,
        PCRE2_SIZE,
        c_int,
        *mut c_void,
    ) -> PCRE2_SIZE,
    substitute_case_callout_data: *mut c_void,
) -> PCRE2_SIZE {
    unsafe {
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

        match (*state).to_case {
            PCRE2_SUBSTITUTE_CASE_LOWER
            | PCRE2_SUBSTITUTE_CASE_UPPER
            | PCRE2_SUBSTITUTE_CASE_TITLE_FIRST => {
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
                rest_to_case = PCRE2_SUBSTITUTE_CASE_NONE;
            }
            PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST => {
                ch1_to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
                rest_to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
            }
            _ => {
                return 0;
            }
        }

        /* Identify the leading character. Take a copy. */
        {
            let mut ch_end: PCRE2_SPTR = input;
            let _ch: u32 = getcharinctest(&mut ch_end, _utf != FALSE);
            ch1_len = ch_end.offset_from(input) as PCRE2_SIZE;
            memcpy(ch1.as_mut_ptr(), input, cu2bytes(ch1_len));
        }

        rest = input.add(ch1_len);
        rest_len = input_len - ch1_len;

        /* Transform just ch1. Buffers are in-place (input == output). */
        {
            let mut ch1_cap: PCRE2_SIZE;
            let max_ch1_cap: PCRE2_SIZE;

            ch1_cap = ch1_len; /* First attempt uses space vacated by ch1. */
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

                memmove(input_output.add(rc), rest, cu2bytes(rest_len));
                rest = input.add(rc);

                ch1_cap = rc;
            }
        }

        if rest_to_case == PCRE2_SUBSTITUTE_CASE_NONE {
            if ch1_overflow == FALSE {
                memmove(output.add(rc), rest, cu2bytes(rest_len));
            }
            rc2 = rest_len;

            (*state).to_case = PCRE2_SUBSTITUTE_CASE_NONE;
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
                if ch1_overflow != FALSE { 0 } else { output_cap - rc },
                rest_to_case,
                substitute_case_callout_data,
            );
            if rc2 == !(0 as PCRE2_SIZE) {
                return rc2;
            }

            if ch1_overflow == FALSE && rc2 > output_cap - rc {
                rest_overflow = TRUE;
            }

            if ch1_overflow != FALSE && rc2 < rest_len {
                rc2 = rest_len;
            }

            (*state).to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
        }

        if rc2 > !(0 as PCRE2_SIZE) - rc {
            return !(0 as PCRE2_SIZE);
        }

        let _ = rest_overflow;

        rc + rc2
    }
}

/*************************************************
*              Match and substitute              *
*************************************************/

/* Goto-target labels of the C function, modelled as an enum. */
#[derive(Clone, Copy, PartialEq)]
enum Jump {
    None,
    Exit,
    Noroom,
    Caseerror,
    Toolargereplace,
    Bad,
    Badescape,
    Ptrexit,
}

/* This function applies a compiled re to a subject string and creates a new
string with substitutions. See the C source for the argument documentation. */

pub unsafe fn pcre2_substitute(
    code: *const pcre2_real_code,
    mut subject: PCRE2_SPTR,
    mut length: PCRE2_SIZE,
    mut start_offset: PCRE2_SIZE,
    mut options: u32,
    mut match_data: *mut pcre2_real_match_data,
    mcontext: *mut pcre2_real_match_context,
    mut replacement: PCRE2_SPTR,
    mut rlength: PCRE2_SIZE,
    buffer: *mut PCRE2_UCHAR,
    blength: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut rc: c_int = 0;
        let mut subs: c_int;
        let ovector_count: u32;
        let mut goptions: u32 = 0;
        let mut suboptions: u32 = 0;
        let mut internal_match_data: *mut pcre2_real_match_data = ptr::null_mut();
        let mut escaped_literal: BOOL = FALSE;
        let mut overflowed: BOOL = FALSE;
        let mut use_existing_match: BOOL;
        let replacement_only: BOOL;
        let utf: BOOL = ((*code).overall_options & PCRE2_UTF != 0) as BOOL;
        let partial: BOOL =
            ((options & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) != 0) as BOOL;
        let mut temp: [PCRE2_UCHAR; 6] = [0; 6];
        let null_str: [PCRE2_UCHAR; 1] = [0xcd];
        let original_subject: PCRE2_SPTR = subject;
        let mut ptr: PCRE2_SPTR = replacement;
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
        let mut substitute_case_callout: SubstituteCaseCalloutFn = None;
        let mut substitute_case_callout_data: *mut c_void = ptr::null_mut();

        let mut jump: Jump = Jump::None;

        /* -------- Local macros mirroring the C preprocessor macros -------- */

        macro_rules! checkmemcpy {
            ($from:expr, $length_:expr) => {{
                let chkmc_length: PCRE2_SIZE = $length_;
                if overflowed != FALSE {
                    if chkmc_length > !(0 as PCRE2_SIZE) - extra_needed {
                        jump = Jump::Toolargereplace;
                    } else {
                        extra_needed += chkmc_length;
                    }
                } else if lengthleft < chkmc_length {
                    if (suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH) == 0 {
                        jump = Jump::Noroom;
                    } else {
                        overflowed = TRUE;
                        extra_needed = chkmc_length - lengthleft;
                    }
                } else {
                    memcpy(buffer.add(buff_offset), $from, cu2bytes(chkmc_length));
                    buff_offset += chkmc_length;
                    lengthleft -= chkmc_length;
                }
            }};
        }

        macro_rules! checkcasecpy_default {
            ($from:expr, $length_:expr, $forcecase:expr) => {{
                let chkcc_length: PCRE2_SIZE = ($length_) as PCRE2_SIZE;
                let chkcc_rc: PCRE2_SIZE = default_substitute_case_callout(
                    $from,
                    chkcc_length,
                    buffer.add(buff_offset),
                    if overflowed != FALSE { 0 } else { lengthleft },
                    &mut $forcecase,
                    code,
                );
                let mut __handled = false;
                if overflowed != FALSE {
                    if chkcc_rc > !(0 as PCRE2_SIZE) - extra_needed {
                        jump = Jump::Toolargereplace;
                    } else {
                        extra_needed += chkcc_rc;
                    }
                    __handled = true;
                }
                if !__handled {
                    if lengthleft < chkcc_rc {
                        if (suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH) == 0 {
                            jump = Jump::Noroom;
                        } else {
                            overflowed = TRUE;
                            extra_needed = chkcc_rc - lengthleft;
                        }
                    } else {
                        buff_offset += chkcc_rc;
                        lengthleft -= chkcc_rc;
                    }
                }
            }};
        }

        macro_rules! checkcasecpy_callout {
            ($length_:expr, $forcecase:expr) => {{
                let chkcc_length: PCRE2_SIZE = ($length_) as PCRE2_SIZE;
                let chkcc_rc: PCRE2_SIZE = do_case_copy(
                    buffer.add(buff_offset),
                    chkcc_length,
                    lengthleft,
                    &mut $forcecase,
                    utf,
                    substitute_case_callout.unwrap(),
                    substitute_case_callout_data,
                );
                if chkcc_rc == !(0 as PCRE2_SIZE) {
                    jump = Jump::Caseerror;
                } else if lengthleft < chkcc_rc {
                    if (suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH) == 0 {
                        jump = Jump::Noroom;
                    } else {
                        overflowed = TRUE;
                        extra_needed = chkcc_rc - lengthleft;
                    }
                } else {
                    buff_offset += chkcc_rc;
                    lengthleft -= chkcc_rc;
                }
            }};
        }

        macro_rules! delayedforcecase {
            ($forcecase:expr, $casestart_offset:expr, $casestart_extra_needed:expr) => {{
                let chars_outstanding: PCRE2_SIZE = (buff_offset - $casestart_offset)
                    + (extra_needed - $casestart_extra_needed);
                if chars_outstanding > 0 {
                    if overflowed != FALSE {
                        let guess: PCRE2_SIZE = pessimistic_case_inflation(chars_outstanding);
                        if guess > !(0 as PCRE2_SIZE) - extra_needed {
                            jump = Jump::Toolargereplace;
                        } else {
                            extra_needed += guess;
                        }
                    } else {
                        /* Rewind the buffer */
                        lengthleft += buff_offset - $casestart_offset;
                        buff_offset = $casestart_offset;
                        /* Care! In-place case transformation */
                        checkcasecpy_callout!(chars_outstanding, $forcecase);
                    }
                }
            }};
        }

        /* -------------------------- General init -------------------------- */

        buff_offset = 0;
        buff_length = *blength;
        lengthleft = buff_length;
        *blength = PCRE2_UNSET;

        if !mcontext.is_null() {
            substitute_case_callout = (*mcontext).substitute_case_callout;
            substitute_case_callout_data = (*mcontext).substitute_case_callout_data;
        }

        if partial != FALSE && (options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY) == 0 {
            return PCRE2_ERROR_BADOPTION;
        }

        if replacement.is_null() {
            if rlength != 0 {
                return PCRE2_ERROR_NULL;
            }
            replacement = null_str.as_ptr();
        }

        if rlength == PCRE2_ZERO_TERMINATED {
            rlength = strlen(replacement);
        }
        repend = replacement.add(rlength);

        if subject.is_null() {
            if length != 0 {
                return PCRE2_ERROR_NULL;
            }
            subject = null_str.as_ptr();
        }

        if length == PCRE2_ZERO_TERMINATED {
            length = strlen(subject);
        }

        use_existing_match = ((options & PCRE2_SUBSTITUTE_MATCHED) != 0) as BOOL;
        replacement_only = ((options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY) != 0) as BOOL;

        if use_existing_match != FALSE && match_data.is_null() {
            return PCRE2_ERROR_NULL;
        }

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
                                cu2bytes(length),
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

        /* WARNING: general contexts are constructed "by hand". */

        if match_data.is_null() {
            let mut gcontext: pcre2_real_general_context = core::mem::zeroed();
            gcontext.memctl = if mcontext.is_null() {
                (*(code as *mut pcre2_real_code)).memctl
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
            let mut gcontext: pcre2_real_general_context = core::mem::zeroed();
            gcontext.memctl = if mcontext.is_null() {
                (*(code as *mut pcre2_real_code)).memctl
            } else {
                (*mcontext).memctl
            };
            pairs = if ((*code).top_bracket as u32 + 1) < (*match_data).oveccount as u32 {
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
                internal_match_data as *mut u8,
                match_data as *const u8,
                MATCH_DATA_OVECTOR_OFFSET
                    + 2 * pairs as usize * core::mem::size_of::<PCRE2_SIZE>(),
            );
            (*internal_match_data).heapframes = ptr::null_mut();
            (*internal_match_data).heapframes_size = 0;
            (*internal_match_data).flags &= !PCRE2_MD_COPIED_SUBJECT;
            match_data = internal_match_data;
        }

        if !internal_match_data.is_null() {
            options &= !PCRE2_COPY_MATCHED_SUBJECT;
        }

        ovector = pcre2_get_ovector_pointer_8(match_data);
        ovector_count = pcre2_get_ovector_count_8(match_data);

        scb.version = 0;
        scb.input = subject;
        scb.output = buffer as PCRE2_SPTR;
        scb.ovector = ovector;

        if utf != FALSE && (options & PCRE2_NO_UTF_CHECK) == 0 {
            rc = crate::valid_utf::valid_utf(replacement, rlength, &mut (*match_data).startchar);
            if rc != 0 {
                (*match_data).leftchar = 0;
                jump = Jump::Exit;
            }
        }

        /* The remainder is executed within this labeled block so that the C
        `goto` targets can be modelled. */

        'exit: loop {
            if jump == Jump::Exit {
                break 'exit;
            }

            suboptions = options & SUBSTITUTE_OPTIONS;
            options &= !SUBSTITUTE_OPTIONS;

            if start_offset > length {
                (*match_data).leftchar = 0;
                rc = PCRE2_ERROR_BADOFFSET;
                jump = Jump::Exit;
                break 'exit;
            }

            if replacement_only == FALSE {
                checkmemcpy!(subject, start_offset);
                if jump != Jump::None {
                    break 'exit;
                }
            }

            subs = 0;
            'global: loop {
                let mut ptrstack: [PCRE2_SPTR; PTR_STACK_SIZE] = [ptr::null(); PTR_STACK_SIZE];
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
                    rc = crate::match_::pcre2_match_8(
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
                    options |= PCRE2_NO_UTF_CHECK;
                }

                if rc == PCRE2_ERROR_NOMATCH {
                    break 'global;
                }
                if rc < 0 {
                    jump = Jump::Exit;
                    break 'exit;
                }

                if *ovector.add(1) < *ovector.add(0) || *ovector.add(0) < start_offset {
                    rc = PCRE2_ERROR_BADSUBSPATTERN;
                    jump = Jump::Exit;
                    break 'exit;
                }

                if subs > 0
                    && !(*ovector.add(1) > ovecsave[1]
                        || (*ovector.add(1) == *ovector.add(0)
                            && ovecsave[1] > ovecsave[0]
                            && *ovector.add(1) == ovecsave[1]))
                {
                    rc = PCRE2_ERROR_INTERNAL_DUPMATCH;
                    jump = Jump::Exit;
                    break 'exit;
                }

                ovecsave[0] = *ovector.add(0);
                ovecsave[1] = *ovector.add(1);

                if subs == INT_MAX {
                    rc = PCRE2_ERROR_TOOMANYREPLACE;
                    jump = Jump::Exit;
                    break 'exit;
                }
                subs += 1;

                if rc == 0 {
                    rc = ovector_count as c_int;
                }
                fraglength = *ovector.add(0) - start_offset;
                if replacement_only == FALSE {
                    checkmemcpy!(subject.add(start_offset), fraglength);
                    if jump != Jump::None {
                        break 'exit;
                    }
                }
                scb.output_offsets[0] = buff_offset;
                scb.oveccount = rc as u32;
                sub_start_extra_needed = extra_needed;

                ptr = replacement;
                if (suboptions & PCRE2_SUBSTITUTE_LITERAL) != 0 {
                    checkmemcpy!(ptr, rlength);
                    if jump != Jump::None {
                        break 'exit;
                    }
                } else {
                    'scan: loop {
                        let mut ch: u32 = 0;
                        let mut chlen: u32;
                        let mut group: c_int = -1;
                        let mut special: u32 = 0;
                        let mut text1_start: PCRE2_SPTR = ptr::null();
                        let mut text1_end: PCRE2_SPTR = ptr::null();
                        let mut text2_start: PCRE2_SPTR = ptr::null();
                        let mut text2_end: PCRE2_SPTR = ptr::null();
                        let mut name: [PCRE2_UCHAR; MAX_NAME_SIZE + 1] =
                            [0; MAX_NAME_SIZE + 1];

                        if ptr >= repend {
                            if ptrstackptr == 0 {
                                break 'scan;
                            }
                            ptrstackptr -= 1;
                            repend = ptrstack[ptrstackptr as usize];
                            ptrstackptr -= 1;
                            ptr = ptrstack[ptrstackptr as usize];
                            continue 'scan;
                        }

                        /* Control-flow flags modelling the goto targets within one
                        scan iteration. */
                        let mut do_loadliteral = false;
                        let mut do_group_substitute = false;
                        let mut do_literal_substitute = false;
                        let mut do_subptr_substitute = false;
                        let mut subptr: PCRE2_SPTR = ptr::null();
                        let mut subptrend: PCRE2_SPTR = ptr::null();
                        let mut sublength: PCRE2_SIZE = 0;

                        if escaped_literal != FALSE {
                            if *ptr.add(0) as u32 == CHAR_BACKSLASH
                                && ptr < repend.sub(1)
                                && *ptr.add(1) as u32 == CHAR_E
                            {
                                escaped_literal = FALSE;
                                ptr = ptr.add(2);
                                continue 'scan;
                            }
                            do_loadliteral = true;
                        }

                        /* ---- $ processing ---- */
                        if !do_loadliteral && *ptr as u32 == CHAR_DOLLAR_SIGN {
                            let mut inparens: BOOL = FALSE;
                            let mut inangle: BOOL = FALSE;
                            let mut star: BOOL = FALSE;
                            let mut next: PCRE2_UCHAR;
                            let mut fell_through = false; /* reached the group/name section */

                            ptr = ptr.add(1);
                            if ptr >= repend {
                                jump = Jump::Bad;
                                break 'exit;
                            }
                            next = *ptr;
                            if next as u32 == CHAR_DOLLAR_SIGN {
                                do_loadliteral = true;
                            }

                            if !do_loadliteral {
                                special = 0;
                                group = -1;
                                inparens = FALSE;
                                inangle = FALSE;
                                star = FALSE;

                                if next as u32 == CHAR_AMPERSAND {
                                    ptr = ptr.add(1);
                                    group = 0;
                                    do_group_substitute = true;
                                } else if next as u32 == CHAR_GRAVE_ACCENT
                                    || next as u32 == CHAR_APOSTROPHE
                                {
                                    ptr = ptr.add(1);
                                    rc = pcre2_substring_length_bynumber_8(
                                        match_data,
                                        0,
                                        &mut sublength,
                                    );
                                    if rc < 0 {
                                        jump = Jump::Ptrexit;
                                        break 'exit;
                                    }
                                    if next as u32 == CHAR_GRAVE_ACCENT {
                                        subptr = subject;
                                        subptrend = subject.add(*ovector.add(0));
                                    } else {
                                        if partial != FALSE {
                                            rc = PCRE2_ERROR_PARTIALSUBS;
                                            jump = Jump::Ptrexit;
                                            break 'exit;
                                        }
                                        subptr = subject.add(*ovector.add(1));
                                        subptrend = subject.add(length);
                                    }
                                    do_subptr_substitute = true;
                                } else if next as u32 == CHAR_UNDERSCORE {
                                    ptr = ptr.add(1);
                                    if partial != FALSE {
                                        rc = PCRE2_ERROR_PARTIALSUBS;
                                        jump = Jump::Ptrexit;
                                        break 'exit;
                                    }
                                    subptr = subject;
                                    subptrend = subject.add(length);
                                    do_subptr_substitute = true;
                                } else if next as u32 == CHAR_PLUS
                                    && !(ptr.add(1) < repend
                                        && *ptr.add(1) as u32 == CHAR_LEFT_CURLY_BRACKET)
                                {
                                    ptr = ptr.add(1);
                                    if (*code).top_bracket == 0 {
                                        if (suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET) == 0 {
                                            rc = PCRE2_ERROR_NOSUBSTRING;
                                            jump = Jump::Ptrexit;
                                            break 'exit;
                                        }
                                        group = 0;
                                    } else {
                                        if (*match_data).oveccount < (*code).top_bracket + 1 {
                                            rc = PCRE2_ERROR_UNAVAILABLE;
                                            jump = Jump::Ptrexit;
                                            break 'exit;
                                        }
                                        group = (*code).top_bracket as c_int;
                                        while group > 0 {
                                            if *ovector.add((2 * group) as usize) != PCRE2_UNSET {
                                                break;
                                            }
                                            group -= 1;
                                        }
                                    }
                                    if group == 0 {
                                        if (suboptions & PCRE2_SUBSTITUTE_UNSET_EMPTY) != 0 {
                                            continue 'scan;
                                        }
                                        rc = PCRE2_ERROR_UNSET;
                                        jump = Jump::Ptrexit;
                                        break 'exit;
                                    }
                                    do_group_substitute = true;
                                } else {
                                    fell_through = true;
                                }

                                if fell_through {
                                    if next as u32 == CHAR_LEFT_CURLY_BRACKET {
                                        ptr = ptr.add(1);
                                        if ptr >= repend {
                                            jump = Jump::Bad;
                                            break 'exit;
                                        }
                                        next = *ptr;
                                        inparens = TRUE;
                                    } else if next as u32 == CHAR_LESS_THAN_SIGN {
                                        ptr = ptr.add(1);
                                        if ptr >= repend {
                                            jump = Jump::Bad;
                                            break 'exit;
                                        }
                                        next = *ptr;
                                        inangle = TRUE;
                                    }

                                    if inangle == FALSE && next as u32 == CHAR_ASTERISK {
                                        ptr = ptr.add(1);
                                        if ptr >= repend {
                                            jump = Jump::Bad;
                                            break 'exit;
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
                                                if (suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET)
                                                    != 0
                                                {
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
                                                    jump = Jump::Ptrexit;
                                                    break 'exit;
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
                                            jump = Jump::Bad;
                                            break 'exit;
                                        }
                                        name_len = ptr.offset_from(name_start) as PCRE2_SIZE;
                                        memcpy(name.as_mut_ptr(), name_start, cu2bytes(name_len));
                                        name[name_len] = 0;
                                    }

                                    next = 0;
                                    let _ = next;

                                    if inparens != FALSE {
                                        if (suboptions & PCRE2_SUBSTITUTE_EXTENDED) != 0
                                            && star == FALSE
                                            && ptr < repend.sub(2)
                                            && *ptr as u32 == CHAR_COLON
                                        {
                                            ptr = ptr.add(1);
                                            special = *ptr as u32;
                                            if special != CHAR_PLUS && special != CHAR_MINUS {
                                                rc = PCRE2_ERROR_BADSUBSTITUTION;
                                                jump = Jump::Ptrexit;
                                                break 'exit;
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
                                                jump = Jump::Ptrexit;
                                                break 'exit;
                                            }
                                            text1_end = ptr;

                                            if special == CHAR_PLUS && *ptr as u32 == CHAR_COLON {
                                                ptr = ptr.add(1);
                                                text2_start = ptr;
                                                rc = find_text_end(code, &mut ptr, repend, TRUE);
                                                if rc != 0 {
                                                    jump = Jump::Ptrexit;
                                                    break 'exit;
                                                }
                                                text2_end = ptr;
                                            }
                                        } else {
                                            if ptr >= repend
                                                || *ptr as u32 != CHAR_RIGHT_CURLY_BRACKET
                                            {
                                                rc = PCRE2_ERROR_REPMISSINGBRACE;
                                                jump = Jump::Ptrexit;
                                                break 'exit;
                                            }
                                        }
                                        ptr = ptr.add(1);
                                    }

                                    if inangle != FALSE {
                                        if ptr >= repend
                                            || *ptr as u32 != CHAR_GREATER_THAN_SIGN
                                        {
                                            jump = Jump::Bad;
                                            break 'exit;
                                        }
                                        ptr = ptr.add(1);
                                    }

                                    if star != FALSE {
                                        if strcmp_c8(
                                            name.as_ptr(),
                                            STRING_MARK.as_ptr() as *const c_char,
                                        ) == 0
                                        {
                                            let mark: PCRE2_SPTR = pcre2_get_mark_8(match_data);
                                            if !mark.is_null() {
                                                fraglength = *mark.sub(1) as PCRE2_SIZE;
                                                if forcecase.to_case
                                                    != PCRE2_SUBSTITUTE_CASE_NONE
                                                    && substitute_case_callout.is_none()
                                                {
                                                    checkcasecpy_default!(
                                                        mark, fraglength, forcecase
                                                    );
                                                    if jump != Jump::None {
                                                        break 'exit;
                                                    }
                                                } else {
                                                    checkmemcpy!(mark, fraglength);
                                                    if jump != Jump::None {
                                                        break 'exit;
                                                    }
                                                }
                                            }
                                        } else {
                                            jump = Jump::Bad;
                                            break 'exit;
                                        }
                                    } else {
                                        do_group_substitute = true;
                                    }
                                }
                            }
                        }
                        /* ---- \ escape processing in extended mode ---- */
                        else if !do_loadliteral
                            && (suboptions & PCRE2_SUBSTITUTE_EXTENDED) != 0
                            && *ptr as u32 == CHAR_BACKSLASH
                        {
                            let mut errorcode: c_int = 0;
                            let mut new_forcecase: case_state = case_state {
                                to_case: PCRE2_SUBSTITUTE_CASE_NONE,
                                single_char: FALSE,
                            };
                            let mut do_setforcecase = false;

                            if ptr < repend.sub(1) {
                                match *ptr.add(1) as u32 {
                                    CHAR_L => {
                                        new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
                                        new_forcecase.single_char = FALSE;
                                        ptr = ptr.add(2);
                                    }
                                    CHAR_l => {
                                        new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
                                        new_forcecase.single_char = TRUE;
                                        ptr = ptr.add(2);
                                        if ptr.add(2) < repend
                                            && *ptr.add(0) as u32 == CHAR_BACKSLASH
                                            && *ptr.add(1) as u32 == CHAR_U
                                        {
                                            new_forcecase.to_case =
                                                PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST;
                                            new_forcecase.single_char = FALSE;
                                            ptr = ptr.add(2);
                                        }
                                    }
                                    CHAR_U => {
                                        new_forcecase.to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
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

                            if new_forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE {
                                do_setforcecase = true;
                            }

                            if !do_setforcecase {
                                ptr = ptr.add(1); /* Point after \ */
                                rc = crate::compile_tables::check_escape(
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
                                    jump = Jump::Badescape;
                                    break 'exit;
                                }

                                if rc == ESC_E {
                                    do_setforcecase = true;
                                } else if rc == ESC_Q {
                                    escaped_literal = TRUE;
                                    continue 'scan;
                                } else if rc == 0 || rc == ESC_b || rc == ESC_v {
                                    if rc == ESC_b {
                                        ch = CHAR_BS;
                                    }
                                    if rc == ESC_v {
                                        ch = CHAR_VT;
                                    }

                                    if utf != FALSE {
                                        chlen = ord2utf(ch, temp.as_mut_ptr());
                                    } else {
                                        temp[0] = ch as u8;
                                        chlen = 1;
                                    }

                                    if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                        && substitute_case_callout.is_none()
                                    {
                                        checkcasecpy_default!(
                                            temp.as_ptr(), chlen as PCRE2_SIZE, forcecase
                                        );
                                        if jump != Jump::None {
                                            break 'exit;
                                        }
                                    } else {
                                        checkmemcpy!(temp.as_ptr(), chlen as PCRE2_SIZE);
                                        if jump != Jump::None {
                                            break 'exit;
                                        }
                                    }
                                    continue 'scan;
                                } else if rc == ESC_g {
                                    let name_len: PCRE2_SIZE;
                                    let name_start: PCRE2_SPTR;

                                    if ptr >= repend || *ptr as u32 != CHAR_LESS_THAN_SIGN {
                                        jump = Jump::Badescape;
                                        break 'exit;
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
                                        jump = Jump::Badescape;
                                        break 'exit;
                                    }
                                    name_len = ptr.offset_from(name_start) as PCRE2_SIZE;

                                    if ptr >= repend || *ptr as u32 != CHAR_GREATER_THAN_SIGN {
                                        jump = Jump::Badescape;
                                        break 'exit;
                                    }
                                    ptr = ptr.add(1);

                                    special = 0;
                                    group = -1;
                                    memcpy(name.as_mut_ptr(), name_start, cu2bytes(name_len));
                                    name[name_len] = 0;
                                    do_group_substitute = true;
                                } else if rc < 0 {
                                    special = 0;
                                    group = -rc - 1;
                                    do_group_substitute = true;
                                } else {
                                    jump = Jump::Badescape;
                                    break 'exit;
                                }
                            }

                            if do_setforcecase {
                                /* SETFORCECASE */
                                if substitute_case_callout.is_some()
                                    && forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                {
                                    delayedforcecase!(
                                        forcecase, casestart_offset, casestart_extra_needed
                                    );
                                    if jump != Jump::None {
                                        break 'exit;
                                    }
                                }
                                forcecase = new_forcecase;
                                casestart_offset = buff_offset;
                                casestart_extra_needed = extra_needed;
                                continue 'scan;
                            }
                        }
                        /* ---- literal code unit ---- */
                        else {
                            do_loadliteral = true;
                        }

                        /* ---- GROUP_SUBSTITUTE ---- */
                        if do_group_substitute {
                            if group < 0 {
                                let mut first: PCRE2_SPTR = ptr::null();
                                let mut last: PCRE2_SPTR = ptr::null();
                                let mut entry: PCRE2_SPTR;
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
                                        jump = Jump::Ptrexit;
                                        break 'exit;
                                    }
                                    entry = first;
                                    while entry <= last {
                                        let ng = get2(entry, 0);
                                        if ng < ovector_count {
                                            if group < 0 {
                                                group = ng as c_int;
                                            }
                                            if *ovector.add((ng * 2) as usize) != PCRE2_UNSET {
                                                group = ng as c_int;
                                                break;
                                            }
                                        }
                                        entry = entry.add(rc as usize);
                                    }
                                    if group < 0 {
                                        group = get2(first, 0) as c_int;
                                    }
                                }
                            }

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
                                    jump = Jump::Ptrexit;
                                    break 'exit;
                                }
                                if special == 0 {
                                    if (suboptions & PCRE2_SUBSTITUTE_UNSET_EMPTY) != 0 {
                                        continue 'scan;
                                    }
                                    jump = Jump::Ptrexit;
                                    break 'exit;
                                }
                            }

                            if special != 0 {
                                if special == CHAR_MINUS {
                                    if rc == 0 {
                                        do_literal_substitute = true;
                                    } else {
                                        text2_start = text1_start;
                                        text2_end = text1_end;
                                    }
                                }

                                if !do_literal_substitute {
                                    if ptrstackptr as usize >= PTR_STACK_SIZE {
                                        jump = Jump::Bad;
                                        break 'exit;
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
                                    continue 'scan;
                                }
                            }

                            /* Fall into LITERAL_SUBSTITUTE. */
                            do_literal_substitute = true;
                        }

                        /* ---- LITERAL_SUBSTITUTE ---- */
                        if do_literal_substitute {
                            subptr = subject.add(*ovector.add((group * 2) as usize));
                            subptrend = subject.add(*ovector.add((group * 2 + 1) as usize));
                            do_subptr_substitute = true;
                        }

                        /* ---- SUBPTR_SUBSTITUTE ---- */
                        if do_subptr_substitute {
                            if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                && substitute_case_callout.is_none()
                            {
                                checkcasecpy_default!(
                                    subptr,
                                    subptrend.offset_from(subptr) as PCRE2_SIZE,
                                    forcecase
                                );
                                if jump != Jump::None {
                                    break 'exit;
                                }
                            } else {
                                checkmemcpy!(
                                    subptr,
                                    subptrend.offset_from(subptr) as PCRE2_SIZE
                                );
                                if jump != Jump::None {
                                    break 'exit;
                                }
                            }
                            continue 'scan;
                        }

                        /* ---- LOADLITERAL ---- */
                        if do_loadliteral {
                            let ch_start: PCRE2_SPTR = ptr;
                            ch = getcharinctest(&mut ptr, utf != FALSE);
                            let _ = ch;

                            if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                && substitute_case_callout.is_none()
                            {
                                checkcasecpy_default!(
                                    ch_start,
                                    ptr.offset_from(ch_start) as PCRE2_SIZE,
                                    forcecase
                                );
                                if jump != Jump::None {
                                    break 'exit;
                                }
                            } else {
                                checkmemcpy!(
                                    ch_start,
                                    ptr.offset_from(ch_start) as PCRE2_SIZE
                                );
                                if jump != Jump::None {
                                    break 'exit;
                                }
                            }
                        }
                    } /* End 'scan loop */
                }

                /* Clean up any trailing deferred case-forcing. */
                if substitute_case_callout.is_some()
                    && forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                {
                    delayedforcecase!(forcecase, casestart_offset, casestart_extra_needed);
                    if jump != Jump::None {
                        break 'exit;
                    }
                }

                /* Handle the callout if there is one. */
                if !mcontext.is_null() && (*mcontext).substitute_callout.is_some() {
                    if overflowed == FALSE {
                        scb.subscount = subs as u32;
                        scb.output_offsets[1] = buff_offset;
                        rc = (*mcontext).substitute_callout.unwrap()(
                            &mut scb,
                            (*mcontext).substitute_callout_data,
                        );

                        if rc != 0 {
                            let newlength: PCRE2_SIZE =
                                scb.output_offsets[1] - scb.output_offsets[0];
                            let oldlength: PCRE2_SIZE = *ovector.add(1) - *ovector.add(0);

                            buff_offset -= newlength;
                            lengthleft += newlength;
                            if replacement_only == FALSE {
                                checkmemcpy!(subject.add(*ovector.add(0)), oldlength);
                                if jump != Jump::None {
                                    break 'exit;
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
                            if newlength_extra > !(0 as PCRE2_SIZE) - newlength_buf {
                                !(0 as PCRE2_SIZE)
                            } else {
                                newlength_buf + newlength_extra
                            };
                        let oldlength: PCRE2_SIZE = *ovector.add(1) - *ovector.add(0);

                        if oldlength > newlength {
                            let additional: PCRE2_SIZE = oldlength - newlength;
                            if additional > !(0 as PCRE2_SIZE) - extra_needed {
                                jump = Jump::Toolargereplace;
                                break 'exit;
                            }
                            extra_needed += additional;
                        }
                    }
                }

                if (suboptions & PCRE2_SUBSTITUTE_GLOBAL) == 0
                    || pcre2_next_match_8(match_data, &mut start_offset, &mut goptions) == FALSE
                {
                    start_offset = *ovector.add(1);
                    break 'global;
                }

                /* Verify pcre2_next_match has not done a bumpalong. */
            } /* End 'global loop */

            if replacement_only == FALSE {
                fraglength = length - start_offset;
                checkmemcpy!(subject.add(start_offset), fraglength);
                if jump != Jump::None {
                    break 'exit;
                }
            }

            temp[0] = 0;
            checkmemcpy!(temp.as_ptr(), 1);
            if jump != Jump::None {
                break 'exit;
            }

            if overflowed != FALSE {
                rc = PCRE2_ERROR_NOMEMORY;
                if extra_needed > !(0 as PCRE2_SIZE) - buff_length {
                    jump = Jump::Toolargereplace;
                    break 'exit;
                }
                *blength = buff_length + extra_needed;
            } else {
                rc = subs;
                *blength = buff_offset - 1;
            }

            jump = Jump::Exit;
            break 'exit;
        } /* End 'exit loop */

        /* --- Goto-target handlers --- */
        match jump {
            Jump::Noroom => {
                rc = PCRE2_ERROR_NOMEMORY;
            }
            Jump::Caseerror => {
                rc = PCRE2_ERROR_REPLACECASE;
            }
            Jump::Toolargereplace => {
                rc = PCRE2_ERROR_TOOLARGEREPLACE;
            }
            Jump::Bad => {
                rc = PCRE2_ERROR_BADREPLACEMENT;
                *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
            }
            Jump::Badescape => {
                rc = PCRE2_ERROR_BADREPESCAPE;
                *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
            }
            Jump::Ptrexit => {
                *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
            }
            _ => {}
        }

        /* EXIT */
        if !internal_match_data.is_null() {
            pcre2_match_data_free_8(internal_match_data);
        } else {
            (*match_data).rc = rc;
        }
        rc
    }
}

/* Exported as `pcre2_substitute_8`. */
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
    unsafe {
        pcre2_substitute(
            code,
            subject,
            length,
            start_offset,
            options,
            match_data,
            mcontext,
            replacement,
            rlength,
            buffer,
            blength,
        )
    }
}
