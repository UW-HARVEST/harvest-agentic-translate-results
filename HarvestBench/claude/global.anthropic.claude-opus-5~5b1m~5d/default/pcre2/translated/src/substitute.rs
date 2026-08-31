//! Translated from pcre2_substitute.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

use crate::compile::_pcre2_check_escape_8;
use crate::context::{default_free, default_malloc, _pcre2_default_match_context_8};
use crate::match_data::{
    pcre2_get_mark_8, pcre2_get_ovector_count_8, pcre2_get_ovector_pointer_8,
    pcre2_match_data_create_8, pcre2_match_data_create_from_pattern_8, pcre2_match_data_free_8,
};
use crate::match_next::pcre2_next_match_8;
use crate::matcher::pcre2_match_8;
use crate::ord2utf::_pcre2_ord2utf_8;
use crate::string_utils::{_pcre2_strcmp_c8_8, _pcre2_strlen_8};
use crate::substring::{pcre2_substring_length_bynumber_8, pcre2_substring_nametable_scan_8};
use crate::valid_utf::_pcre2_valid_utf_8;

const PTR_STACK_SIZE: usize = 20;

const SUBSTITUTE_OPTIONS: u32 = PCRE2_SUBSTITUTE_EXTENDED
    | PCRE2_SUBSTITUTE_GLOBAL
    | PCRE2_SUBSTITUTE_LITERAL
    | PCRE2_SUBSTITUTE_MATCHED
    | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
    | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY
    | PCRE2_SUBSTITUTE_UNKNOWN_UNSET
    | PCRE2_SUBSTITUTE_UNSET_EMPTY;

/* A local memcmp(), because the C code calls the library function. Returns 0 if
the two byte strings of the given length are equal. */

pub(crate) unsafe fn substitute_memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i: usize = 0;
    while i < n {
        let ca = *a.add(i);
        let cb = *b.add(i);
        if ca != cb {
            return if (ca as i32) < (cb as i32) { -1 } else { 1 };
        }
        i += 1;
    }
    0
}

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

pub(crate) unsafe fn find_text_end(
    code: *const pcre2_real_code,
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    last: BOOL,
) -> i32 {
    let mut rc: i32 = 0;
    let mut nestlevel: u32 = 0;
    let mut literal: BOOL = FALSE;
    let mut ptr: PCRE2_SPTR = *ptrptr;

    'EXIT: {
        'forloop: while ptr < ptrend {
            if literal != 0 {
                if *ptr.add(0) == b'\\' && ptr < ptrend.wrapping_sub(1) && *ptr.add(1) == b'E' {
                    literal = FALSE;
                    ptr = ptr.wrapping_add(1);
                }
            } else if *ptr == b'}' {
                if nestlevel == 0 {
                    break 'EXIT; /* goto EXIT */
                }
                nestlevel -= 1;
            } else if *ptr == b':' && last == 0 && nestlevel == 0 {
                break 'EXIT; /* goto EXIT */
            } else if *ptr == b'$' {
                if ptr < ptrend.wrapping_sub(1) && *ptr.add(1) == b'{' {
                    nestlevel += 1;
                    ptr = ptr.wrapping_add(1);
                }
            } else if *ptr == b'\\' {
                let erc: i32;
                let mut errorcode: i32 = 0;
                let mut ch: u32 = 0;
                let esc_end_ptr: PCRE2_SPTR;

                if ptr < ptrend.wrapping_sub(1) {
                    match *ptr.add(1) {
                        b'L' | b'l' | b'U' | b'u' => {
                            ptr = ptr.wrapping_add(1);
                            /* continue: the for-loop increment still happens */
                            ptr = ptr.wrapping_add(1);
                            continue 'forloop;
                        }
                        _ => {}
                    }
                }

                ptr = ptr.wrapping_add(1); /* Must point after \ */
                erc = _pcre2_check_escape_8(
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
                    break 'EXIT; /* goto EXIT */
                }

                esc_end_ptr = ptr;
                ptr = ptr.wrapping_sub(1); /* Rewind by one, because the for-loop will increment it */

                match erc as u32 {
                    /* case 0: Data character */
                    /* case ESC_b: Data character */
                    /* case ESC_v: Data character */
                    /* case ESC_E: Isolated \E is ignored */
                    0 | ESC_b | ESC_v | ESC_E => {}

                    ESC_Q => {
                        literal = TRUE;
                    }

                    ESC_g => {
                        /* The \g<name> form (\g<number> already handled by check_escape)

                        Don't worry about finding the matching ">". We are super, super lenient
                        about validating ${} replacements inside find_text_end(), so we certainly
                        don't need to worry about other syntax. Importantly, a \g<..> or $<...>
                        sequence can't contain a '}' character. */
                    }

                    _ => {
                        if erc < 0 {
                            /* capture group reference */
                        } else {
                            ptr = esc_end_ptr;
                            rc = PCRE2_ERROR_BADREPESCAPE;
                            break 'EXIT; /* goto EXIT */
                        }
                    }
                }
            }

            ptr = ptr.wrapping_add(1);
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

pub(crate) unsafe fn read_name_subst(
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
            break 'FAILED; /* goto FAILED */
        }

        /* We do not need to check whether the name starts with a non-digit.
        We are simply referencing names here, not defining them. */

        /* See read_name in the pcre2_compile.c for the corresponding logic
        restricting group names inside the pattern itself. */

        if utf != 0 {
            let mut c: u32;
            let mut type_: u32;

            while ptr < ptrend {
                GETCHAR!(c, ptr);
                type_ = UCD_CHARTYPE!(c);
                if type_ != ucp_Nd
                    && crate::tables::_pcre2_ucp_gentype_8[type_ as usize] != ucp_L
                    && c != b'_' as u32
                {
                    break;
                }
                ptr = ptr.add(1);
                FORWARDCHARTEST!(ptr, ptrend);
            }
        } else
        /* Handle group names in non-UTF modes. */
        {
            while ptr < ptrend && MAX_255!(*ptr) != 0 && (*ctypes.add(*ptr as usize) & ctype_word) != 0
            {
                ptr = ptr.add(1);
            }
        }

        /* Check name length */

        if ptr.offset_from(nameptr) > MAX_NAME_SIZE as isize {
            break 'FAILED; /* goto FAILED */
        }

        /* Subpattern names must not be empty */
        if ptr == nameptr {
            break 'FAILED; /* goto FAILED */
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

/* These shadow the (uint32_t) definitions in crate::consts with the int-typed
values that the C file uses locally. */

pub(crate) const PCRE2_SUBSTITUTE_CASE_NONE: i32 = 0;
// 1, 2, 3 are PCRE2_SUBSTITUTE_CASE_LOWER, UPPER, TITLE_FIRST.
pub(crate) const PCRE2_SUBSTITUTE_CASE_LOWER: i32 = 1;
pub(crate) const PCRE2_SUBSTITUTE_CASE_UPPER: i32 = 2;
pub(crate) const PCRE2_SUBSTITUTE_CASE_TITLE_FIRST: i32 = 3;
pub(crate) const PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST: i32 = 4;

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct case_state {
    pub to_case: i32, /* One of PCRE2_SUBSTITUTE_CASE_xyz */
    pub single_char: BOOL,
}

/* Helper to guess how much a string is likely to increase in size when
case-transformed. Usually, strings don't change size at all, but some rare
characters do grow. Estimate +10%, plus another few characters.

Performing this estimation is unfortunate, but inevitable, since we can't call
the callout if we ran out of buffer space to prepare its input.

Because this estimate is inexact (and in pathological cases, underestimates the
required buffer size) we must document that when you have a
substitute_case_callout, and you are using PCRE2_SUBSTITUTE_OVERFLOW_LENGTH, you
may need more than two calls to determine the final buffer size. */

pub(crate) unsafe fn pessimistic_case_inflation(len: PCRE2_SIZE) -> PCRE2_SIZE {
    (len >> 3u32) + 10
}

/* Case transformation behaviour if no callout is passed. */

pub(crate) unsafe fn default_substitute_case_callout(
    mut input: PCRE2_SPTR,
    input_len: PCRE2_SIZE,
    mut output: *mut PCRE2_UCHAR,
    mut output_cap: PCRE2_SIZE,
    state: *mut case_state,
    code: *const pcre2_real_code,
) -> PCRE2_SIZE {
    let input_end: PCRE2_SPTR = input.wrapping_add(input_len);
    let utf: BOOL;
    let ucp: BOOL;
    let mut temp: [PCRE2_UCHAR; 6] = [0; 6];
    let mut next_to_upper: BOOL;
    let mut rest_to_upper: BOOL;
    let single_char: BOOL;
    let mut overflow: BOOL = FALSE;
    let mut written: PCRE2_SIZE = 0;

    /* Helpful simplifying invariant: input and output are disjoint buffers.
    (PCRE2_ASSERT omitted.) */

    utf = (((*code).overall_options & PCRE2_UTF) != 0) as BOOL;
    ucp = (((*code).overall_options & PCRE2_UCP) != 0) as BOOL;

    if input_len == 0 {
        return 0;
    }

    match (*state).to_case {
        PCRE2_SUBSTITUTE_CASE_LOWER /* Can be single_char TRUE or FALSE */
        | PCRE2_SUBSTITUTE_CASE_UPPER /* Can only be single_char FALSE */ => {
            rest_to_upper = ((*state).to_case == PCRE2_SUBSTITUTE_CASE_UPPER) as BOOL;
            next_to_upper = rest_to_upper;
        }

        PCRE2_SUBSTITUTE_CASE_TITLE_FIRST /* Can be single_char TRUE or FALSE */ => {
            next_to_upper = TRUE;
            rest_to_upper = FALSE;
            (*state).to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
        }

        PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST /* Can only be single_char FALSE */ => {
            next_to_upper = FALSE;
            rest_to_upper = TRUE;
            (*state).to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
        }

        /* LCOV_EXCL_START */
        _ => {
            return 0;
        }
        /* LCOV_EXCL_STOP */
    }

    single_char = (*state).single_char;
    if single_char != 0 {
        (*state).to_case = PCRE2_SUBSTITUTE_CASE_NONE;
    }

    while input < input_end {
        let mut ch: u32;
        let mut chlen: u32;

        GETCHARINCTEST!(ch, input, utf);

        if (utf != 0 || ucp != 0) && ch >= 128 {
            let type_: u32 = UCD_CHARTYPE!(ch);
            if crate::tables::_pcre2_ucp_gentype_8[type_ as usize] == ucp_L
                && type_ != (if next_to_upper != 0 { ucp_Lu } else { ucp_Ll })
            {
                ch = UCD_OTHERCASE!(ch);
            }

            /* TODO This is far from correct... it doesn't support the SpecialCasing.txt
            mappings, but worse, it's not even correct for all the ordinary case
            mappings. */
        } else if MAX_255!(ch) != 0 {
            if (*(*code)
                .tables
                .add(cbits_offset)
                .add(if next_to_upper != 0 { cbit_upper } else { cbit_lower })
                .add((ch / 8) as usize)
                & (1u32 << (ch % 8)) as u8)
                == 0
            {
                ch = *(*code).tables.add(fcc_offset).add(ch as usize) as u32;
            }
        }

        if utf != 0 {
            chlen = _pcre2_ord2utf_8(ch, temp.as_mut_ptr());
        } else {
            temp[0] = ch as PCRE2_UCHAR;
            chlen = 1;
        }

        if overflow == 0 && (chlen as PCRE2_SIZE) <= output_cap {
            core::ptr::copy_nonoverlapping(
                temp.as_ptr(),
                output,
                CU2BYTES!(chlen as PCRE2_SIZE),
            );
            output = output.wrapping_add(chlen as usize);
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

        if single_char != 0 {
            let rest_len: PCRE2_SIZE = input_end.offset_from(input) as PCRE2_SIZE;

            if overflow == 0 && rest_len <= output_cap {
                core::ptr::copy_nonoverlapping(input, output, CU2BYTES!(rest_len));
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

/* Helper to perform the call to the substitute_case_callout. We wrap the
user-provided callout because our internal arguments are slightly extended. We
don't want the user callout to handle the case of "\l" (first character only to
lowercase) or "\l\U" (first character to lowercase, rest to uppercase) because
those are not operations defined by Unicode. Instead the user callout simply
needs to provide the three Unicode primitives: lower, upper, titlecase. */

pub(crate) unsafe fn do_case_copy(
    input_output: *mut PCRE2_UCHAR,
    input_len: PCRE2_SIZE,
    output_cap: PCRE2_SIZE,
    state: *mut case_state,
    utf: BOOL,
    substitute_case_callout: SubstituteCaseCalloutFn,
    substitute_case_callout_data: *mut c_void,
) -> PCRE2_SIZE {
    let input: PCRE2_SPTR = input_output;
    let output: *mut PCRE2_UCHAR = input_output;
    let mut rc: PCRE2_SIZE = 0;
    let mut rc2: PCRE2_SIZE;
    let ch1_to_case: i32;
    let rest_to_case: i32;
    let mut ch1: [PCRE2_UCHAR; 6] = [0; 6];
    let ch1_len: PCRE2_SIZE;
    let mut rest: PCRE2_SPTR;
    let rest_len: PCRE2_SIZE;
    let mut ch1_overflow: BOOL = FALSE;
    let mut rest_overflow: BOOL = FALSE;

    /* PCRE2_ASSERT(input_len != 0); */

    match (*state).to_case {
        PCRE2_SUBSTITUTE_CASE_LOWER /* Can be single_char TRUE or FALSE */
        | PCRE2_SUBSTITUTE_CASE_UPPER /* Can only be single_char FALSE */
        | PCRE2_SUBSTITUTE_CASE_TITLE_FIRST /* Can be single_char TRUE or FALSE */ => {
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
        }

        PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST /* Can only be single_char FALSE */ => {
            ch1_to_case = PCRE2_SUBSTITUTE_CASE_LOWER;
            rest_to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
        }

        /* LCOV_EXCL_START */
        _ => {
            return 0;
        }
        /* LCOV_EXCL_STOP */
    }

    /* Identify the leading character. Take copy, because its storage overlaps with
    `output`, and hence may be scrambled by the callout. */

    {
        let mut ch_end: PCRE2_SPTR = input;
        let mut ch: u32;

        GETCHARINCTEST!(ch, ch_end, utf);
        let _ = ch;
        ch1_len = ch_end.offset_from(input) as PCRE2_SIZE;
        core::ptr::copy_nonoverlapping(input, ch1.as_mut_ptr(), CU2BYTES!(ch1_len));
    }

    rest = input.wrapping_add(ch1_len);
    rest_len = input_len - ch1_len;

    /* Transform just ch1. The buffers are always in-place (input == output). With a
    custom callout, we need a loop to discover its required buffer size. */

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

            core::ptr::copy(rest, input_output.wrapping_add(rc), CU2BYTES!(rest_len));
            rest = input.wrapping_add(rc);

            ch1_cap = rc;

            /* Proof of loop termination: `ch1_cap` is growing on each iteration, but
            the loop ends if `rc` reaches the (unchanging) upper bound of output_cap. */
        }
    }

    if rest_to_case == PCRE2_SUBSTITUTE_CASE_NONE {
        if ch1_overflow == 0 {
            core::ptr::copy(rest, output.wrapping_add(rc), CU2BYTES!(rest_len));
        }
        rc2 = rest_len;

        (*state).to_case = PCRE2_SUBSTITUTE_CASE_NONE;
    } else {
        let mut dummy: [PCRE2_UCHAR; 1] = [0; 1];

        rc2 = (substitute_case_callout.unwrap())(
            rest,
            rest_len,
            if ch1_overflow != 0 {
                dummy.as_mut_ptr()
            } else {
                output.wrapping_add(rc)
            },
            if ch1_overflow != 0 { 0u32 as PCRE2_SIZE } else { output_cap - rc },
            rest_to_case,
            substitute_case_callout_data,
        );
        if rc2 == !(0 as PCRE2_SIZE) {
            return rc2;
        }

        if ch1_overflow == 0 && rc2 > output_cap - rc {
            rest_overflow = TRUE;
        }

        /* If ch1 grows so that `xform(ch1)+rest` can't fit in the buffer, but then
        `rest` shrinks, it's actually possible for the total calculated length of
        `xform(ch1)+xform(rest)` to come out at less than output_cap. But we can't
        report that, because it would make it seem that the operation succeeded. */
        if ch1_overflow != 0 && rc2 < rest_len {
            rc2 = rest_len;
        }

        (*state).to_case = PCRE2_SUBSTITUTE_CASE_UPPER;
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
*              Match and substitute              *
*************************************************/

/* Labels used by the goto-emulation in pcre2_substitute(). */

const L_EXIT: u32 = 0;
const L_NOROOM: u32 = 1;
const L_CASEERROR: u32 = 2;
const L_TOOLARGEREPLACE: u32 = 3;
const L_BAD: u32 = 4;
const L_BADESCAPE: u32 = 5;
const L_PTREXIT: u32 = 6;

/* States used by the goto-emulation inside the replacement-scanning loop. */

const S_TOP: u32 = 0;
const S_LOADLITERAL: u32 = 1;
const S_GROUP_SUBSTITUTE: u32 = 2;
const S_LITERAL_SUBSTITUTE: u32 = 3;
const S_SUBPTR_SUBSTITUTE: u32 = 4;
const S_SETFORCECASE: u32 = 5;

/* This macro checks for space in the buffer before copying into it. On
overflow, either give an error immediately, or keep on, accumulating the
length.

Because Rust's macro_rules! are hygienic, all the local variables of
pcre2_substitute() that the C macro refers to have to be passed in explicitly,
along with the label of the enclosing block that stands in for "goto". */

macro_rules! CHECKMEMCPY {
    ($body:lifetime, $gotolbl:ident, $buffer:ident, $buff_offset:ident, $lengthleft:ident,
     $overflowed:ident, $extra_needed:ident, $suboptions:ident, $from:expr, $length_:expr) => {{
        let chkmc_length: PCRE2_SIZE = $length_;
        if $overflowed != 0 {
            if chkmc_length > !(0 as PCRE2_SIZE) - $extra_needed
            /* Integer overflow */
            {
                $gotolbl = L_TOOLARGEREPLACE; /* goto TOOLARGEREPLACE */
                break $body;
            }
            $extra_needed += chkmc_length;
        } else if $lengthleft < chkmc_length {
            if ($suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH) == 0 {
                $gotolbl = L_NOROOM; /* goto NOROOM */
                break $body;
            }
            $overflowed = TRUE;
            $extra_needed = chkmc_length - $lengthleft;
        } else {
            core::ptr::copy_nonoverlapping(
                $from as *const u8,
                $buffer.wrapping_add($buff_offset) as *mut u8,
                CU2BYTES!(chkmc_length),
            );
            $buff_offset += chkmc_length;
            $lengthleft -= chkmc_length;
        }
    }};
}

/* This macro checks for space and copies characters with casing modifications.
On overflow, it behaves as for CHECKMEMCPY().

When substitute_case_callout is NULL, the source and destination buffers must
not overlap, because our default handler does not support this.

CHECKCASECPY_BASE() has been inlined into each of its two users. */

macro_rules! CHECKCASECPY_DEFAULT {
    ($body:lifetime, $gotolbl:ident, $buffer:ident, $buff_offset:ident, $lengthleft:ident,
     $overflowed:ident, $extra_needed:ident, $suboptions:ident, $forcecase:ident, $code:ident,
     $from:expr, $length_:expr) => {{
        let chkcc_length: PCRE2_SIZE = ($length_) as PCRE2_SIZE;
        let mut chkcc_rc: PCRE2_SIZE;
        'chkcc: {
            /* do_call */
            chkcc_rc = default_substitute_case_callout(
                $from,
                chkcc_length,
                $buffer.wrapping_add($buff_offset),
                if $overflowed != 0 { 0 } else { $lengthleft },
                &mut $forcecase,
                $code,
            );
            if $overflowed != 0 {
                if chkcc_rc > !(0 as PCRE2_SIZE) - $extra_needed
                /* Integer overflow */
                {
                    $gotolbl = L_TOOLARGEREPLACE; /* goto TOOLARGEREPLACE */
                    break $body;
                }
                $extra_needed += chkcc_rc;
                break 'chkcc;
            }
            /* end do_call */

            if $lengthleft < chkcc_rc {
                if ($suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH) == 0 {
                    $gotolbl = L_NOROOM; /* goto NOROOM */
                    break $body;
                }
                $overflowed = TRUE;
                $extra_needed = chkcc_rc - $lengthleft;
            } else {
                $buff_offset += chkcc_rc;
                $lengthleft -= chkcc_rc;
            }
        }
    }};
}

macro_rules! CHECKCASECPY_CALLOUT {
    ($body:lifetime, $gotolbl:ident, $buffer:ident, $buff_offset:ident, $lengthleft:ident,
     $overflowed:ident, $extra_needed:ident, $suboptions:ident, $forcecase:ident, $utf:ident,
     $scc:ident, $sccd:ident, $length_:expr) => {{
        let chkcc_length: PCRE2_SIZE = ($length_) as PCRE2_SIZE;
        let mut chkcc_rc: PCRE2_SIZE;
        /* do_call */
        chkcc_rc = do_case_copy(
            $buffer.wrapping_add($buff_offset),
            chkcc_length,
            $lengthleft,
            &mut $forcecase,
            $utf,
            $scc,
            $sccd,
        );
        if chkcc_rc == !(0 as PCRE2_SIZE) {
            $gotolbl = L_CASEERROR; /* goto CASEERROR */
            break $body;
        }
        /* end do_call */

        if $lengthleft < chkcc_rc {
            if ($suboptions & PCRE2_SUBSTITUTE_OVERFLOW_LENGTH) == 0 {
                $gotolbl = L_NOROOM; /* goto NOROOM */
                break $body;
            }
            $overflowed = TRUE;
            $extra_needed = chkcc_rc - $lengthleft;
        } else {
            $buff_offset += chkcc_rc;
            $lengthleft -= chkcc_rc;
        }
    }};
}

/* This macro does a delayed case transformation, for the situation when we have
a case-forcing callout. */

macro_rules! DELAYEDFORCECASE {
    ($body:lifetime, $gotolbl:ident, $buffer:ident, $buff_offset:ident, $lengthleft:ident,
     $overflowed:ident, $extra_needed:ident, $suboptions:ident, $forcecase:ident, $utf:ident,
     $scc:ident, $sccd:ident, $casestart_offset:ident, $casestart_extra_needed:ident) => {{
        let chars_outstanding: PCRE2_SIZE = ($buff_offset - $casestart_offset)
            + ($extra_needed - $casestart_extra_needed);
        if chars_outstanding > 0 {
            if $overflowed != 0 {
                let guess: PCRE2_SIZE = pessimistic_case_inflation(chars_outstanding);
                if guess > !(0 as PCRE2_SIZE) - $extra_needed
                /* Integer overflow */
                {
                    $gotolbl = L_TOOLARGEREPLACE; /* goto TOOLARGEREPLACE */
                    break $body;
                }
                $extra_needed += guess;
            } else {
                /* Rewind the buffer */
                $lengthleft += $buff_offset - $casestart_offset;
                $buff_offset = $casestart_offset;
                /* Care! In-place case transformation */
                CHECKCASECPY_CALLOUT!(
                    $body, $gotolbl, $buffer, $buff_offset, $lengthleft, $overflowed,
                    $extra_needed, $suboptions, $forcecase, $utf, $scc, $sccd,
                    chars_outstanding
                );
            }
        }
    }};
}

/* Here's the function */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substitute_8(
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
) -> i32 {
    let mut rc: i32 = 0;
    let mut subs: i32;
    let mut ovector_count: u32;
    let mut goptions: u32 = 0;
    let mut suboptions: u32;
    let mut internal_match_data: *mut pcre2_real_match_data = core::ptr::null_mut();
    let mut escaped_literal: BOOL = FALSE;
    let mut overflowed: BOOL = FALSE;
    let mut use_existing_match: BOOL;
    let replacement_only: BOOL;
    let utf: BOOL = (((*code).overall_options & PCRE2_UTF) != 0) as BOOL;
    let partial: BOOL = ((options & (PCRE2_PARTIAL_HARD | PCRE2_PARTIAL_SOFT)) != 0) as BOOL;
    let mut temp: [PCRE2_UCHAR; 6] = [0; 6];
    let null_str: [PCRE2_UCHAR; 1] = [0xcd];
    let original_subject: PCRE2_SPTR = subject;
    let mut ptr: PCRE2_SPTR = core::ptr::null();
    let mut repend: PCRE2_SPTR = core::ptr::null();
    let mut extra_needed: PCRE2_SIZE = 0;
    let mut buff_offset: PCRE2_SIZE;
    let buff_length: PCRE2_SIZE;
    let mut lengthleft: PCRE2_SIZE;
    let mut fraglength: PCRE2_SIZE;
    let mut ovector: *mut PCRE2_SIZE;
    let mut ovecsave: [PCRE2_SIZE; 2] = [0, 0];
    let mut scb: pcre2_substitute_callout_block = pcre2_substitute_callout_block {
        version: 0,
        input: core::ptr::null(),
        output: core::ptr::null(),
        output_offsets: [0, 0],
        ovector: core::ptr::null_mut(),
        oveccount: 0,
        subscount: 0,
    };
    let mut sub_start_extra_needed: PCRE2_SIZE = 0;
    let mut substitute_case_callout: SubstituteCaseCalloutFn = None;
    let mut substitute_case_callout_data: *mut c_void = core::ptr::null_mut();

    let mut gotolbl: u32 = L_EXIT;

    'body: {
        /* General initialization */

        buff_offset = 0;
        buff_length = *blength;
        lengthleft = buff_length;
        *blength = PCRE2_UNSET;

        if !mcontext.is_null() {
            substitute_case_callout = (*mcontext).substitute_case_callout;
            substitute_case_callout_data = (*mcontext).substitute_case_callout_data;
        }

        /* Partial matching is supported, with limitations. We allow matching in partial
        mode, however, if a partial match is found, the substitution will fail with a
        PCRE2_ERROR_PARTIAL error. Additionally, outputting the after-match text is not
        allowed (PCRE2_ERROR_BADOPTION), and certain replacement items such as $' and $_
        are not supported (PCRE2_ERROR_PARTIALSUBS).

        This must come after setting *blength to PCRE2_UNSET, so as not to imply an
        offset in the replacement. */

        if partial != 0 && (options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY) == 0 {
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
        repend = replacement.wrapping_add(rlength);

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

        /* Check for using a match that has already happened. Note that the subject
        pointer in the match data may be NULL after a no-match. */

        use_existing_match = ((options & PCRE2_SUBSTITUTE_MATCHED) != 0) as BOOL;
        replacement_only = ((options & PCRE2_SUBSTITUTE_REPLACEMENT_ONLY) != 0) as BOOL;

        if use_existing_match != 0 && match_data.is_null() {
            return PCRE2_ERROR_NULL;
        }

        /* If an existing match is being passed in, we should check that it matches
        the passed-in subject pointer, length, and match options. */

        if use_existing_match != 0 {
            /* Return early, as the rest of the match_data may not have been
            initialised. This duplicates and must be in sync with the check below that
            aborts substitution on any result other than success or no-match. */
            if (*match_data).rc < 0 && (*match_data).rc != PCRE2_ERROR_NOMATCH {
                return (*match_data).rc;
            }

            /* Not supported if the passed-in match was from the DFA interpreter. */
            if (*match_data).matchedby == PCRE2_MATCHEDBY_DFA_INTERPRETER {
                return PCRE2_ERROR_DFA_UFUNC;
            }

            if code != (*match_data).code {
                return PCRE2_ERROR_DIFFSUBSPATTERN;
            }

            /* We want the passed-in subject strings to match. */
            if length != (*match_data).subject_length
                || !(original_subject == (*match_data).subject
                    || (((*match_data).flags & PCRE2_MD_COPIED_SUBJECT) != 0
                        && (length == 0
                            || substitute_memcmp(
                                subject,
                                (*match_data).subject,
                                CU2BYTES!(length),
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

        /* If starting from an existing match, there must be an externally provided
        match data block. We create an internal match_data block in two cases: (a) an
        external one is not supplied (and we are not starting from an existing match);
        (b) an existing match is to be used for the first substitution. */

        /* WARNING: In both cases below a general context is constructed "by hand"
        because calling pcre2_general_context_create() involves a memory allocation. */

        if match_data.is_null() {
            let mut gcontext: pcre2_real_general_context = pcre2_real_general_context {
                memctl: if mcontext.is_null() {
                    (*(code as *mut pcre2_real_code)).memctl
                } else {
                    (*(mcontext as *mut pcre2_real_match_context)).memctl
                },
            };
            internal_match_data = pcre2_match_data_create_from_pattern_8(code, &mut gcontext);
            match_data = internal_match_data;
            if internal_match_data.is_null() {
                return PCRE2_ERROR_NOMEMORY;
            }
        } else if use_existing_match != 0 {
            let pairs: i32;
            let mut gcontext: pcre2_real_general_context = pcre2_real_general_context {
                memctl: if mcontext.is_null() {
                    (*(code as *mut pcre2_real_code)).memctl
                } else {
                    (*(mcontext as *mut pcre2_real_match_context)).memctl
                },
            };
            pairs = if ((*code).top_bracket as i32 + 1) < (*match_data).oveccount as i32 {
                (*code).top_bracket as i32 + 1
            } else {
                (*match_data).oveccount as i32
            };
            internal_match_data =
                pcre2_match_data_create_8((*match_data).oveccount as u32, &mut gcontext);
            if internal_match_data.is_null() {
                return PCRE2_ERROR_NOMEMORY;
            }
            core::ptr::copy_nonoverlapping(
                match_data as *const u8,
                internal_match_data as *mut u8,
                OVECTOR_OFFSET_IN_MATCH_DATA
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

        ovector = pcre2_get_ovector_pointer_8(match_data);
        ovector_count = pcre2_get_ovector_count_8(match_data);

        /* Fixed things in the callout block */

        scb.version = 0;
        scb.input = subject;
        scb.output = buffer as PCRE2_SPTR;
        scb.ovector = ovector;

        /* Check UTF replacement string if necessary. */

        if utf != 0 && (options & PCRE2_NO_UTF_CHECK) == 0 {
            rc = _pcre2_valid_utf_8(replacement, rlength, &mut (*match_data).startchar);
            if rc != 0 {
                (*match_data).leftchar = 0;
                gotolbl = L_EXIT; /* goto EXIT */
                break 'body;
            }
        }

        /* Save the substitute options and remove them from the match options. */

        suboptions = options & SUBSTITUTE_OPTIONS;
        options &= !SUBSTITUTE_OPTIONS;

        /* Error if the start match offset is greater than the length of the subject. */

        if start_offset > length {
            (*match_data).leftchar = 0;
            rc = PCRE2_ERROR_BADOFFSET;
            gotolbl = L_EXIT; /* goto EXIT */
            break 'body;
        }

        /* Copy up to the start offset, unless only the replacement is required. */

        if replacement_only == 0 {
            CHECKMEMCPY!(
                'body, gotolbl, buffer, buff_offset, lengthleft, overflowed, extra_needed,
                suboptions, subject, start_offset
            );
        }

        /* Loop for global substituting. If PCRE2_SUBSTITUTE_MATCHED is set, the first
        match is taken from the match_data that was passed in. */

        subs = 0;
        'globalloop: loop {
            let mut ptrstack: [PCRE2_SPTR; PTR_STACK_SIZE] = [core::ptr::null(); PTR_STACK_SIZE];
            let mut ptrstackptr: u32 = 0;
            let mut forcecase: case_state = case_state {
                to_case: PCRE2_SUBSTITUTE_CASE_NONE,
                single_char: FALSE,
            };
            let mut casestart_offset: PCRE2_SIZE = 0;
            let mut casestart_extra_needed: PCRE2_SIZE = 0;

            if use_existing_match != 0 {
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

            if utf != 0 {
                options |= PCRE2_NO_UTF_CHECK; /* Only need to check once */
            }

            /* Any error other than no match returns the error code. No match breaks the
            global loop. */

            if rc == PCRE2_ERROR_NOMATCH {
                break 'globalloop;
            }

            if rc < 0 {
                gotolbl = L_EXIT; /* goto EXIT */
                break 'body;
            }

            /* Handle a successful match. Matches that use \K to end before they start
            or start before the current point in the subject are not supported. */

            if *ovector.add(1) < *ovector.add(0) || *ovector.add(0) < start_offset {
                rc = PCRE2_ERROR_BADSUBSPATTERN;
                gotolbl = L_EXIT; /* goto EXIT */
                break 'body;
            }

            /* Assert that our replacement loop is making progress, checked even in
            release builds. */

            /* LCOV_EXCL_START */
            if subs > 0
                && !(*ovector.add(1) > ovecsave[1]
                    || (*ovector.add(1) == *ovector.add(0)
                        && ovecsave[1] > ovecsave[0]
                        && *ovector.add(1) == ovecsave[1]))
            {
                rc = PCRE2_ERROR_INTERNAL_DUPMATCH;
                gotolbl = L_EXIT; /* goto EXIT */
                break 'body;
            }
            /* LCOV_EXCL_STOP */

            ovecsave[0] = *ovector.add(0);
            ovecsave[1] = *ovector.add(1);

            /* Count substitutions with a paranoid check for integer overflow; surely no
            real call to this function would ever hit this! */

            if subs == i32::MAX {
                rc = PCRE2_ERROR_TOOMANYREPLACE;
                gotolbl = L_EXIT; /* goto EXIT */
                break 'body;
            }
            subs += 1;

            /* Copy the text leading up to the match (unless not required); remember
            where the insert begins and how many ovector pairs are set; and remember how
            much space we have requested in extra_needed. */

            if rc == 0 {
                rc = ovector_count as i32;
            }
            fraglength = *ovector.add(0) - start_offset;
            if replacement_only == 0 {
                CHECKMEMCPY!(
                    'body, gotolbl, buffer, buff_offset, lengthleft, overflowed, extra_needed,
                    suboptions, subject.wrapping_add(start_offset), fraglength
                );
            }
            scb.output_offsets[0] = buff_offset;
            scb.oveccount = rc as u32;
            sub_start_extra_needed = extra_needed;

            /* Process the replacement string. If the entire replacement is literal, just
            copy it with length check. */

            ptr = replacement;
            if (suboptions & PCRE2_SUBSTITUTE_LITERAL) != 0 {
                CHECKMEMCPY!(
                    'body, gotolbl, buffer, buff_offset, lengthleft, overflowed, extra_needed,
                    suboptions, ptr, rlength
                );
            }
            /* Within a non-literal replacement, which must be scanned character by
            character, local literal mode can be set by \Q, but only in extended mode
            when backslashes are being interpreted. In extended mode we must handle
            nested substrings that are to be reprocessed. */
            else {
                'replloop: loop {
                    let mut ch: u32 = 0;
                    let mut chlen: u32 = 0;
                    let mut group: i32 = 0;
                    let mut special: u32 = 0;
                    let mut text1_start: PCRE2_SPTR = core::ptr::null();
                    let mut text1_end: PCRE2_SPTR = core::ptr::null();
                    let mut text2_start: PCRE2_SPTR = core::ptr::null();
                    let mut text2_end: PCRE2_SPTR = core::ptr::null();
                    let mut name: [PCRE2_UCHAR; MAX_NAME_SIZE as usize + 1] =
                        [0; MAX_NAME_SIZE as usize + 1];

                    /* Declared inside the '$' branch in C, but shared with the
                    GROUP_SUBSTITUTE / SUBPTR_SUBSTITUTE labels. */
                    let mut sublength: PCRE2_SIZE = 0;
                    let mut subptr: PCRE2_SPTR = core::ptr::null();
                    let mut subptrend: PCRE2_SPTR = core::ptr::null();

                    /* Declared inside the backslash branch in C, but shared with the
                    SETFORCECASE label. */
                    let mut new_forcecase: case_state = case_state {
                        to_case: PCRE2_SUBSTITUTE_CASE_NONE,
                        single_char: FALSE,
                    };

                    /* Declared inside the literal branch in C, but shared with the
                    LOADLITERAL label. */
                    let mut ch_start: PCRE2_SPTR;

                    let mut state: u32 = S_TOP;
                    'sm: loop {
                        match state {
                            S_TOP => {
                                /* If at the end of a nested substring, pop the stack. */

                                if ptr >= repend {
                                    if ptrstackptr == 0 {
                                        break 'replloop; /* End of replacement string */
                                    }
                                    ptrstackptr -= 1;
                                    repend = *ptrstack.as_ptr().add(ptrstackptr as usize);
                                    ptrstackptr -= 1;
                                    ptr = *ptrstack.as_ptr().add(ptrstackptr as usize);
                                    continue 'replloop;
                                }

                                /* Handle the next character */

                                if escaped_literal != 0 {
                                    if *ptr.add(0) == b'\\'
                                        && ptr < repend.wrapping_sub(1)
                                        && *ptr.add(1) == b'E'
                                    {
                                        escaped_literal = FALSE;
                                        ptr = ptr.wrapping_add(2);
                                        continue 'replloop;
                                    }
                                    state = S_LOADLITERAL; /* goto LOADLITERAL */
                                    continue 'sm;
                                }

                                /* Not in literal mode. */

                                if *ptr == b'$' {
                                    let mut inparens: BOOL;
                                    let mut inangle: BOOL;
                                    let mut star: BOOL;
                                    let mut next: PCRE2_UCHAR;

                                    ptr = ptr.wrapping_add(1);
                                    if ptr >= repend {
                                        gotolbl = L_BAD; /* goto BAD */
                                        break 'body;
                                    }
                                    next = *ptr;
                                    if next == b'$' {
                                        state = S_LOADLITERAL; /* goto LOADLITERAL */
                                        continue 'sm;
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
                                    if next == b'&' {
                                        ptr = ptr.wrapping_add(1);
                                        group = 0;
                                        state = S_GROUP_SUBSTITUTE; /* goto GROUP_SUBSTITUTE */
                                        continue 'sm;
                                    }
                                    if next == b'`' || next == b'\'' {
                                        ptr = ptr.wrapping_add(1);

                                        /* (Sanity-check ovector before reading from it.) */
                                        rc = pcre2_substring_length_bynumber_8(
                                            match_data,
                                            0,
                                            &mut sublength,
                                        );
                                        /* LCOV_EXCL_START */
                                        if rc < 0 {
                                            gotolbl = L_PTREXIT; /* goto PTREXIT */
                                            break 'body;
                                        }
                                        /* LCOV_EXCL_STOP */

                                        if next == b'`' {
                                            subptr = subject;
                                            subptrend = subject.wrapping_add(*ovector.add(0));
                                        } else {
                                            if partial != 0 {
                                                rc = PCRE2_ERROR_PARTIALSUBS;
                                                gotolbl = L_PTREXIT; /* goto PTREXIT */
                                                break 'body;
                                            }

                                            subptr = subject.wrapping_add(*ovector.add(1));
                                            subptrend = subject.wrapping_add(length);
                                        }

                                        state = S_SUBPTR_SUBSTITUTE; /* goto SUBPTR_SUBSTITUTE */
                                        continue 'sm;
                                    }
                                    if next == b'_' {
                                        /* Java, .NET support $_ for "entire input string". */
                                        ptr = ptr.wrapping_add(1);

                                        if partial != 0 {
                                            rc = PCRE2_ERROR_PARTIALSUBS;
                                            gotolbl = L_PTREXIT; /* goto PTREXIT */
                                            break 'body;
                                        }

                                        subptr = subject;
                                        subptrend = subject.wrapping_add(length);
                                        state = S_SUBPTR_SUBSTITUTE; /* goto SUBPTR_SUBSTITUTE */
                                        continue 'sm;
                                    }
                                    if next == b'+'
                                        && !(ptr.wrapping_add(1) < repend && *ptr.add(1) == b'{')
                                    {
                                        /* Perl supports $+ for "highest captured group". We also
                                        don't accept "$+{..." since that's Perl syntax for our
                                        ${name}. */
                                        ptr = ptr.wrapping_add(1);
                                        if (*code).top_bracket == 0 {
                                            /* Treat either as "no such group" or "all groups
                                            unset" based on the PCRE2_SUBSTITUTE_UNKNOWN_UNSET
                                            option. */
                                            if (suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET) == 0 {
                                                rc = PCRE2_ERROR_NOSUBSTRING;
                                                gotolbl = L_PTREXIT; /* goto PTREXIT */
                                                break 'body;
                                            }
                                            group = 0;
                                        } else {
                                            /* If we have any capture groups, then the ovector
                                            needs to be large enough for all of them, or the
                                            result won't be accurate. */
                                            if ((*match_data).oveccount as i32)
                                                < (*code).top_bracket as i32 + 1
                                            {
                                                rc = PCRE2_ERROR_UNAVAILABLE;
                                                gotolbl = L_PTREXIT; /* goto PTREXIT */
                                                break 'body;
                                            }
                                            group = (*code).top_bracket as i32;
                                            while group > 0 {
                                                if *ovector.add((2 * group) as usize) != PCRE2_UNSET
                                                {
                                                    break;
                                                }
                                                group -= 1;
                                            }
                                        }
                                        if group == 0 {
                                            if (suboptions & PCRE2_SUBSTITUTE_UNSET_EMPTY) != 0 {
                                                continue 'replloop;
                                            }
                                            rc = PCRE2_ERROR_UNSET;
                                            gotolbl = L_PTREXIT; /* goto PTREXIT */
                                            break 'body;
                                        }
                                        state = S_GROUP_SUBSTITUTE; /* goto GROUP_SUBSTITUTE */
                                        continue 'sm;
                                    }

                                    if next == b'{' {
                                        ptr = ptr.wrapping_add(1);
                                        if ptr >= repend {
                                            gotolbl = L_BAD; /* goto BAD */
                                            break 'body;
                                        }
                                        next = *ptr;
                                        inparens = TRUE;
                                    } else if next == b'<' {
                                        /* JavaScript compatibility syntax, $<name>. Processes only
                                        named groups (not numbered) and does not support extensions
                                        such as star. */
                                        ptr = ptr.wrapping_add(1);
                                        if ptr >= repend {
                                            gotolbl = L_BAD; /* goto BAD */
                                            break 'body;
                                        }
                                        next = *ptr;
                                        inangle = TRUE;
                                    }

                                    if inangle == 0 && next == b'*' {
                                        ptr = ptr.wrapping_add(1);
                                        if ptr >= repend {
                                            gotolbl = L_BAD; /* goto BAD */
                                            break 'body;
                                        }
                                        next = *ptr;
                                        star = TRUE;
                                    }

                                    if star == 0 && inangle == 0 && next >= b'0' && next <= b'9' {
                                        group = (next - b'0') as i32;
                                        loop {
                                            ptr = ptr.wrapping_add(1);
                                            if !(ptr < repend) {
                                                break;
                                            }
                                            next = *ptr;
                                            if next < b'0' || next > b'9' {
                                                break;
                                            }
                                            group = group * 10 + (next - b'0') as i32;

                                            /* A check for a number greater than the hightest
                                            captured group is sufficient here; no need for a
                                            separate overflow check. */

                                            if group > (*code).top_bracket as i32 {
                                                if (suboptions & PCRE2_SUBSTITUTE_UNKNOWN_UNSET)
                                                    != 0
                                                {
                                                    loop {
                                                        ptr = ptr.wrapping_add(1);
                                                        if !(ptr < repend
                                                            && *ptr >= b'0'
                                                            && *ptr <= b'9')
                                                        {
                                                            break;
                                                        }
                                                    }
                                                    break;
                                                } else {
                                                    rc = PCRE2_ERROR_NOSUBSTRING;
                                                    gotolbl = L_PTREXIT; /* goto PTREXIT */
                                                    break 'body;
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
                                        ) == 0
                                        {
                                            gotolbl = L_BAD; /* goto BAD */
                                            break 'body;
                                        }
                                        name_len = ptr.offset_from(name_start) as PCRE2_SIZE;
                                        core::ptr::copy_nonoverlapping(
                                            name_start,
                                            name.as_mut_ptr(),
                                            CU2BYTES!(name_len),
                                        );
                                        *name.as_mut_ptr().add(name_len) = 0;
                                    }

                                    next = 0; /* not used or updated after this point */
                                    let _ = next;

                                    /* In extended mode we recognize ${name:+set text:unset text}
                                    and ${name:-default text}. */

                                    if inparens != 0 {
                                        if (suboptions & PCRE2_SUBSTITUTE_EXTENDED) != 0
                                            && star == 0
                                            && ptr < repend.wrapping_sub(2)
                                            && *ptr == b':'
                                        {
                                            ptr = ptr.wrapping_add(1);
                                            special = *ptr as u32;
                                            if special != b'+' as u32 && special != b'-' as u32 {
                                                rc = PCRE2_ERROR_BADSUBSTITUTION;
                                                gotolbl = L_PTREXIT; /* goto PTREXIT */
                                                break 'body;
                                            }

                                            ptr = ptr.wrapping_add(1);
                                            text1_start = ptr;
                                            rc = find_text_end(
                                                code,
                                                &mut ptr,
                                                repend,
                                                (special == b'-' as u32) as BOOL,
                                            );
                                            if rc != 0 {
                                                gotolbl = L_PTREXIT; /* goto PTREXIT */
                                                break 'body;
                                            }
                                            text1_end = ptr;

                                            if special == b'+' as u32 && *ptr == b':' {
                                                ptr = ptr.wrapping_add(1);
                                                text2_start = ptr;
                                                rc = find_text_end(code, &mut ptr, repend, TRUE);
                                                if rc != 0 {
                                                    gotolbl = L_PTREXIT; /* goto PTREXIT */
                                                    break 'body;
                                                }
                                                text2_end = ptr;
                                            }
                                        } else {
                                            if ptr >= repend || *ptr != b'}' {
                                                rc = PCRE2_ERROR_REPMISSINGBRACE;
                                                gotolbl = L_PTREXIT; /* goto PTREXIT */
                                                break 'body;
                                            }
                                        }

                                        ptr = ptr.wrapping_add(1);
                                    }

                                    if inangle != 0 {
                                        if ptr >= repend || *ptr != b'>' {
                                            gotolbl = L_BAD; /* goto BAD */
                                            break 'body;
                                        }
                                        ptr = ptr.wrapping_add(1);
                                    }

                                    /* Have found a syntactically correct group number or name, or
                                    *name. Only *MARK is currently recognized. */

                                    if star != 0 {
                                        if _pcre2_strcmp_c8_8(
                                            name.as_ptr(),
                                            b"MARK\0".as_ptr() as *const c_char,
                                        ) == 0
                                        {
                                            let mark: PCRE2_SPTR = pcre2_get_mark_8(match_data);
                                            if !mark.is_null() {
                                                /* Peek backwards one code unit to obtain the
                                                length of the mark. It can (theoretically) contain
                                                an embedded NUL. */
                                                fraglength =
                                                    *mark.offset(-1) as PCRE2_SIZE;
                                                if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                                    && substitute_case_callout.is_none()
                                                {
                                                    CHECKCASECPY_DEFAULT!(
                                                        'body, gotolbl, buffer, buff_offset,
                                                        lengthleft, overflowed, extra_needed,
                                                        suboptions, forcecase, code,
                                                        mark, fraglength
                                                    );
                                                } else {
                                                    CHECKMEMCPY!(
                                                        'body, gotolbl, buffer, buff_offset,
                                                        lengthleft, overflowed, extra_needed,
                                                        suboptions, mark, fraglength
                                                    );
                                                }
                                            }
                                        } else {
                                            gotolbl = L_BAD; /* goto BAD */
                                            break 'body;
                                        }

                                        /* End of the '$' processing: next iteration. */
                                        continue 'replloop;
                                    }
                                    /* Substitute the contents of a group. We don't use
                                    substring_copy functions any more, in order to support case
                                    forcing. */
                                    else {
                                        state = S_GROUP_SUBSTITUTE; /* fall into GROUP_SUBSTITUTE */
                                        continue 'sm;
                                    }
                                }
                                /* Handle an escape sequence in extended mode. We can use
                                check_escape() to process \Q, \E, \c, \o, \x and \ followed by
                                non-alphanumerics, but the case-forcing escapes are not supported
                                in pcre2_compile() so must be recognized here. */
                                else if (suboptions & PCRE2_SUBSTITUTE_EXTENDED) != 0
                                    && *ptr == b'\\'
                                {
                                    let mut errorcode: i32 = 0;

                                    if ptr < repend.wrapping_sub(1) {
                                        match *ptr.add(1) {
                                            b'L' => {
                                                new_forcecase.to_case =
                                                    PCRE2_SUBSTITUTE_CASE_LOWER;
                                                new_forcecase.single_char = FALSE;
                                                ptr = ptr.wrapping_add(2);
                                            }

                                            b'l' => {
                                                new_forcecase.to_case =
                                                    PCRE2_SUBSTITUTE_CASE_LOWER;
                                                new_forcecase.single_char = TRUE;
                                                ptr = ptr.wrapping_add(2);
                                                if ptr.wrapping_add(2) < repend
                                                    && *ptr.add(0) == b'\\'
                                                    && *ptr.add(1) == b'U'
                                                {
                                                    /* Perl reverse-title-casing feature for \l\U */
                                                    new_forcecase.to_case =
                                                        PCRE2_SUBSTITUTE_CASE_REVERSE_TITLE_FIRST;
                                                    new_forcecase.single_char = FALSE;
                                                    ptr = ptr.wrapping_add(2);
                                                }
                                            }

                                            b'U' => {
                                                new_forcecase.to_case =
                                                    PCRE2_SUBSTITUTE_CASE_UPPER;
                                                new_forcecase.single_char = FALSE;
                                                ptr = ptr.wrapping_add(2);
                                            }

                                            b'u' => {
                                                new_forcecase.to_case =
                                                    PCRE2_SUBSTITUTE_CASE_TITLE_FIRST;
                                                new_forcecase.single_char = TRUE;
                                                ptr = ptr.wrapping_add(2);
                                                if ptr.wrapping_add(2) < repend
                                                    && *ptr.add(0) == b'\\'
                                                    && *ptr.add(1) == b'L'
                                                {
                                                    /* Perl title-casing feature for \u\L */
                                                    new_forcecase.to_case =
                                                        PCRE2_SUBSTITUTE_CASE_TITLE_FIRST;
                                                    new_forcecase.single_char = FALSE;
                                                    ptr = ptr.wrapping_add(2);
                                                }
                                            }

                                            _ => {}
                                        }
                                    }

                                    if new_forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE {
                                        state = S_SETFORCECASE; /* fall into SETFORCECASE */
                                        continue 'sm;
                                    }

                                    ptr = ptr.wrapping_add(1); /* Point after \ */
                                    rc = _pcre2_check_escape_8(
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
                                        gotolbl = L_BADESCAPE; /* goto BADESCAPE */
                                        break 'body;
                                    }

                                    match rc as u32 {
                                        ESC_E => {
                                            state = S_SETFORCECASE; /* goto SETFORCECASE */
                                            continue 'sm;
                                        }

                                        ESC_Q => {
                                            escaped_literal = TRUE;
                                            continue 'replloop;
                                        }

                                        /* case 0: Data character */
                                        /* case ESC_b: \b is backspace in a substitution */
                                        /* case ESC_v: \v is vertical tab in a substitution */
                                        0 | ESC_b | ESC_v => {
                                            if rc as u32 == ESC_b {
                                                ch = 0x08; /* CHAR_BS */
                                            }
                                            if rc as u32 == ESC_v {
                                                ch = 0x0b; /* CHAR_VT */
                                            }

                                            if utf != 0 {
                                                chlen = _pcre2_ord2utf_8(ch, temp.as_mut_ptr());
                                            } else {
                                                temp[0] = ch as PCRE2_UCHAR;
                                                chlen = 1;
                                            }

                                            if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                                && substitute_case_callout.is_none()
                                            {
                                                CHECKCASECPY_DEFAULT!(
                                                    'body, gotolbl, buffer, buff_offset, lengthleft,
                                                    overflowed, extra_needed, suboptions, forcecase,
                                                    code, temp.as_ptr(), chlen
                                                );
                                            } else {
                                                CHECKMEMCPY!(
                                                    'body, gotolbl, buffer, buff_offset, lengthleft,
                                                    overflowed, extra_needed, suboptions,
                                                    temp.as_ptr(), chlen as PCRE2_SIZE
                                                );
                                            }
                                            continue 'replloop;
                                        }

                                        ESC_g => {
                                            let name_len: PCRE2_SIZE;
                                            let name_start: PCRE2_SPTR;

                                            /* Parse the \g<name> form (\g<number> already handled
                                            by check_escape) */
                                            if ptr >= repend || *ptr != b'<' {
                                                gotolbl = L_BADESCAPE; /* goto BADESCAPE */
                                                break 'body;
                                            }
                                            ptr = ptr.wrapping_add(1);

                                            name_start = ptr;
                                            if read_name_subst(
                                                &mut ptr,
                                                repend,
                                                utf,
                                                (*code).tables.add(ctypes_offset),
                                            ) == 0
                                            {
                                                gotolbl = L_BADESCAPE; /* goto BADESCAPE */
                                                break 'body;
                                            }
                                            name_len = ptr.offset_from(name_start) as PCRE2_SIZE;

                                            if ptr >= repend || *ptr != b'>' {
                                                gotolbl = L_BADESCAPE; /* goto BADESCAPE */
                                                break 'body;
                                            }
                                            ptr = ptr.wrapping_add(1);

                                            special = 0;
                                            group = -1;
                                            core::ptr::copy_nonoverlapping(
                                                name_start,
                                                name.as_mut_ptr(),
                                                CU2BYTES!(name_len),
                                            );
                                            *name.as_mut_ptr().add(name_len) = 0;
                                            state = S_GROUP_SUBSTITUTE; /* goto GROUP_SUBSTITUTE */
                                            continue 'sm;
                                        }

                                        _ => {
                                            if rc < 0 {
                                                special = 0;
                                                group = -rc - 1;
                                                state = S_GROUP_SUBSTITUTE; /* goto GROUP_SUBSTITUTE */
                                                continue 'sm;
                                            }
                                            gotolbl = L_BADESCAPE; /* goto BADESCAPE */
                                            break 'body;
                                        }
                                    }
                                }
                                /* Handle a literal code unit */
                                else {
                                    state = S_LOADLITERAL; /* fall into LOADLITERAL */
                                    continue 'sm;
                                }
                            }

                            S_SETFORCECASE => {
                                /* If the substitute_case_callout is unset, our case-forcing is
                                done immediately. If there is a callout however, then its action is
                                delayed until all the characters have been collected.

                                Apply the callout now, before we set the new casing mode. */

                                if substitute_case_callout.is_some()
                                    && forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                {
                                    DELAYEDFORCECASE!(
                                        'body, gotolbl, buffer, buff_offset, lengthleft, overflowed,
                                        extra_needed, suboptions, forcecase, utf,
                                        substitute_case_callout, substitute_case_callout_data,
                                        casestart_offset, casestart_extra_needed
                                    );
                                }

                                forcecase = new_forcecase;
                                casestart_offset = buff_offset;
                                casestart_extra_needed = extra_needed;
                                continue 'replloop;
                            }

                            S_GROUP_SUBSTITUTE => {
                                /* Find a number for a named group. In case there are duplicate
                                names, search for the first one that is set. If the name is not
                                found when PCRE2_SUBSTITUTE_UNKNOWN_EMPTY is set, set the group
                                number to a non-existent group. */

                                if group < 0 {
                                    let mut first: PCRE2_SPTR = core::ptr::null();
                                    let mut last: PCRE2_SPTR = core::ptr::null();
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
                                        group = (*code).top_bracket as i32 + 1;
                                    } else {
                                        if rc < 0 {
                                            gotolbl = L_PTREXIT; /* goto PTREXIT */
                                            break 'body;
                                        }
                                        entry = first;
                                        while entry <= last {
                                            let ng: u32 = GET2!(entry, 0);
                                            if ng < ovector_count {
                                                if group < 0 {
                                                    group = ng as i32; /* First in ovector */
                                                }
                                                if *ovector.add((ng * 2) as usize) != PCRE2_UNSET {
                                                    group = ng as i32; /* First that is set */
                                                    break;
                                                }
                                            }
                                            entry = entry.wrapping_offset(rc as isize);
                                        }

                                        /* If group is still negative, it means we did not find a
                                        group that is in the ovector. Just set the first group. */

                                        if group < 0 {
                                            group = GET2!(first, 0) as i32;
                                        }
                                    }
                                }

                                /* We now have a group that is identified by number. Find the
                                length of the captured string. If a group in a non-special
                                substitution is unset when PCRE2_SUBSTITUTE_UNSET_EMPTY is set,
                                substitute nothing. */

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
                                        gotolbl = L_PTREXIT; /* goto PTREXIT: non-unset errors */
                                        break 'body;
                                    }
                                    if special == 0
                                    /* Plain substitution */
                                    {
                                        if (suboptions & PCRE2_SUBSTITUTE_UNSET_EMPTY) != 0 {
                                            continue 'replloop;
                                        }
                                        gotolbl = L_PTREXIT; /* goto PTREXIT: else error */
                                        break 'body;
                                    }
                                }

                                /* If special is '+' we have a 'set' and possibly an 'unset' text,
                                both of which are reprocessed when used. If special is '-' we have
                                a default text for when the group is unset; it must be
                                reprocessed. */

                                if special != 0 {
                                    if special == b'-' as u32 {
                                        if rc == 0 {
                                            state = S_LITERAL_SUBSTITUTE; /* goto LITERAL_SUBSTITUTE */
                                            continue 'sm;
                                        }
                                        text2_start = text1_start;
                                        text2_end = text1_end;
                                    }

                                    if ptrstackptr as usize >= PTR_STACK_SIZE {
                                        gotolbl = L_BAD; /* goto BAD */
                                        break 'body;
                                    }
                                    *ptrstack.as_mut_ptr().add(ptrstackptr as usize) = ptr;
                                    ptrstackptr += 1;
                                    *ptrstack.as_mut_ptr().add(ptrstackptr as usize) = repend;
                                    ptrstackptr += 1;

                                    if rc == 0 {
                                        ptr = text1_start;
                                        repend = text1_end;
                                    } else {
                                        ptr = text2_start;
                                        repend = text2_end;
                                    }
                                    continue 'replloop;
                                }

                                /* Otherwise we have a literal substitution of a group's
                                contents. */

                                state = S_LITERAL_SUBSTITUTE; /* fall into LITERAL_SUBSTITUTE */
                                continue 'sm;
                            }

                            S_LITERAL_SUBSTITUTE => {
                                subptr = subject.wrapping_add(*ovector.add((group * 2) as usize));
                                subptrend =
                                    subject.wrapping_add(*ovector.add((group * 2 + 1) as usize));

                                state = S_SUBPTR_SUBSTITUTE; /* fall into SUBPTR_SUBSTITUTE */
                                continue 'sm;
                            }

                            S_SUBPTR_SUBSTITUTE => {
                                /* Substitute a literal string, possibly forcing alphabetic
                                case. */

                                if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                    && substitute_case_callout.is_none()
                                {
                                    CHECKCASECPY_DEFAULT!(
                                        'body, gotolbl, buffer, buff_offset, lengthleft, overflowed,
                                        extra_needed, suboptions, forcecase, code,
                                        subptr, subptrend.offset_from(subptr) as PCRE2_SIZE
                                    );
                                } else {
                                    CHECKMEMCPY!(
                                        'body, gotolbl, buffer, buff_offset, lengthleft, overflowed,
                                        extra_needed, suboptions, subptr,
                                        subptrend.offset_from(subptr) as PCRE2_SIZE
                                    );
                                }

                                /* End of the '$' processing: next iteration. */
                                continue 'replloop;
                            }

                            S_LOADLITERAL => {
                                ch_start = ptr;
                                GETCHARINCTEST!(ch, ptr, utf); /* Get character value, increment pointer */
                                let _ = ch;

                                if forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
                                    && substitute_case_callout.is_none()
                                {
                                    CHECKCASECPY_DEFAULT!(
                                        'body, gotolbl, buffer, buff_offset, lengthleft, overflowed,
                                        extra_needed, suboptions, forcecase, code,
                                        ch_start, ptr.offset_from(ch_start) as PCRE2_SIZE
                                    );
                                } else {
                                    CHECKMEMCPY!(
                                        'body, gotolbl, buffer, buff_offset, lengthleft, overflowed,
                                        extra_needed, suboptions, ch_start,
                                        ptr.offset_from(ch_start) as PCRE2_SIZE
                                    );
                                }

                                /* End handling a literal code unit */
                                continue 'replloop;
                            }

                            _ => {}
                        }

                        break 'sm;
                    }
                } /* End of loop for scanning the replacement. */
            }

            /* If the substitute_case_callout is unset, our case-forcing is done
            immediately. If there is a callout however, then its action is delayed
            until all the characters have been collected.

            We now clean up any trailing section of the replacement for which we deferred
            the case-forcing. */

            if substitute_case_callout.is_some()
                && forcecase.to_case != PCRE2_SUBSTITUTE_CASE_NONE
            {
                DELAYEDFORCECASE!(
                    'body, gotolbl, buffer, buff_offset, lengthleft, overflowed, extra_needed,
                    suboptions, forcecase, utf, substitute_case_callout,
                    substitute_case_callout_data, casestart_offset, casestart_extra_needed
                );
            }

            /* The replacement has been copied to the output, or its size has been
            remembered. Handle the callout if there is one. */

            if !mcontext.is_null() && (*mcontext).substitute_callout.is_some() {
                /* If we an actual (non-simulated) replacement, do the callout. */

                if overflowed == 0 {
                    scb.subscount = subs as u32;
                    scb.output_offsets[1] = buff_offset;
                    rc = ((*mcontext).substitute_callout.unwrap())(
                        &mut scb,
                        (*mcontext).substitute_callout_data,
                    );

                    /* A non-zero return means cancel this substitution. Instead, copy the
                    matched string fragment. */

                    if rc != 0 {
                        let newlength: PCRE2_SIZE =
                            scb.output_offsets[1] - scb.output_offsets[0];
                        let oldlength: PCRE2_SIZE = *ovector.add(1) - *ovector.add(0);

                        buff_offset -= newlength;
                        lengthleft += newlength;
                        if replacement_only == 0 {
                            CHECKMEMCPY!(
                                'body, gotolbl, buffer, buff_offset, lengthleft, overflowed,
                                extra_needed, suboptions,
                                subject.wrapping_add(*ovector.add(0)), oldlength
                            );
                        }

                        /* A negative return means do not do any more. */

                        if rc < 0 {
                            suboptions &= !PCRE2_SUBSTITUTE_GLOBAL;
                        }
                    }
                }
                /* In this interesting case, we cannot do the callout, so it's hard to
                estimate the required buffer size. What callers want is to be able to make
                two calls to pcre2_substitute(), once with PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
                to discover the buffer size, and then a second and final call. */
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
                            gotolbl = L_TOOLARGEREPLACE; /* goto TOOLARGEREPLACE */
                            break 'body;
                        }
                        extra_needed += additional;
                    }

                    /* Proceed as if the callout did not return a negative. */
                }
            }

            /* Exit the global loop if we are not in global mode, or if pcre2_next_match()
            indicates we have reached the end of the subject. */

            if (suboptions & PCRE2_SUBSTITUTE_GLOBAL) == 0
                || pcre2_next_match_8(match_data, &mut start_offset, &mut goptions) == 0
            {
                start_offset = *ovector.add(1);
                break 'globalloop;
            }

            /* Verify that pcre2_next_match() has not done a bumpalong (because we have
            already returned PCRE2_ERROR_BADSUBSPATTERN for \K in lookarounds).
            (PCRE2_ASSERT omitted.) */
        } /* End of global loop */

        /* Copy the rest of the subject unless not required, and terminate the output
        with a binary zero. */

        if replacement_only == 0 {
            fraglength = length - start_offset;
            CHECKMEMCPY!(
                'body, gotolbl, buffer, buff_offset, lengthleft, overflowed, extra_needed,
                suboptions, subject.wrapping_add(start_offset), fraglength
            );
        }

        temp[0] = 0;
        CHECKMEMCPY!(
            'body, gotolbl, buffer, buff_offset, lengthleft, overflowed, extra_needed,
            suboptions, temp.as_ptr(), 1 as PCRE2_SIZE
        );

        /* If overflowed is set it means the PCRE2_SUBSTITUTE_OVERFLOW_LENGTH is set,
        and matching has carried on after a full buffer, in order to compute the length
        needed. Otherwise, an overflow generates an immediate error return. */

        if overflowed != 0 {
            rc = PCRE2_ERROR_NOMEMORY;

            if extra_needed > !(0 as PCRE2_SIZE) - buff_length
            /* Integer overflow */
            {
                gotolbl = L_TOOLARGEREPLACE; /* goto TOOLARGEREPLACE */
                break 'body;
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

    /* The trailer labels of the C function, in source order. Each one falls into
    the next unless it jumps. */

    if gotolbl == L_NOROOM {
        /* NOROOM: */
        rc = PCRE2_ERROR_NOMEMORY;
        gotolbl = L_EXIT; /* goto EXIT */
    }

    if gotolbl == L_CASEERROR {
        /* CASEERROR: */
        rc = PCRE2_ERROR_REPLACECASE;
        gotolbl = L_EXIT; /* goto EXIT */
    }

    if gotolbl == L_TOOLARGEREPLACE {
        /* TOOLARGEREPLACE: */
        rc = PCRE2_ERROR_TOOLARGEREPLACE;
        gotolbl = L_EXIT; /* goto EXIT */
    }

    if gotolbl == L_BAD {
        /* BAD: */
        rc = PCRE2_ERROR_BADREPLACEMENT;
        gotolbl = L_PTREXIT; /* goto PTREXIT */
    }

    if gotolbl == L_BADESCAPE {
        /* BADESCAPE: */
        rc = PCRE2_ERROR_BADREPESCAPE;
        gotolbl = L_PTREXIT; /* falls into PTREXIT */
    }

    if gotolbl == L_PTREXIT {
        /* PTREXIT: */
        *blength = ptr.offset_from(replacement) as PCRE2_SIZE;
        gotolbl = L_EXIT; /* goto EXIT */
    }

    /* EXIT: */
    if !internal_match_data.is_null() {
        pcre2_match_data_free_8(internal_match_data);
    } else {
        (*match_data).rc = rc;
    }
    rc
}

/* End of pcre2_substitute.c */
