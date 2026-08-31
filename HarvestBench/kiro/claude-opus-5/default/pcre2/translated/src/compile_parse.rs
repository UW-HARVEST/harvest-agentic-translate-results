//! Translation of `parse_regex` from `c_src/src/pcre2_compile.c`
//! (roughly lines 3040..5960), including the local `#define`s and enums that
//! precede it.
//!
//! Built for the 8-bit library with `SUPPORT_UNICODE` (hence
//! `SUPPORT_WIDE_CHARS`), `LINK_SIZE == 2`, no JIT, no EBCDIC, no `PCRE2_DEBUG`.

#![allow(non_snake_case, non_upper_case_globals, unused_parens, dead_code, unused_assignments)]

use core::ffi::{c_char, c_int};

use crate::chars::*;
use crate::compile_internal::*;
use crate::compile_tables::*;
use crate::internal::*;
use crate::opcodes::*;
use crate::ucp::*;

/* `MAX_NAME_COUNT` from config.h. */
const MAX_NAME_COUNT: u16 = 10000;

/* The nest_save structure and its flags (local to pcre2_compile.c). */

#[repr(C)]
#[derive(Clone, Copy)]
struct nest_save {
    nest_depth: u16,
    reset_group: u16,
    max_group: u16,
    flags: u16,
    options: u32,
    xoptions: u32,
}

const NSF_RESET: u16 = 0x0001;
const NSF_CONDASSERT: u16 = 0x0002;
const NSF_ATOMICSR: u16 = 0x0004;

/* Options that are changeable within the pattern must be tracked during
parsing. */

const PARSE_TRACKED_OPTIONS: u32 = PCRE2_CASELESS
    | PCRE2_DOTALL
    | PCRE2_DUPNAMES
    | PCRE2_EXTENDED
    | PCRE2_EXTENDED_MORE
    | PCRE2_MULTILINE
    | PCRE2_NO_AUTO_CAPTURE
    | PCRE2_UNGREEDY;

const PARSE_TRACKED_EXTRA_OPTIONS: u32 = PCRE2_EXTRA_CASELESS_RESTRICT
    | PCRE2_EXTRA_ASCII_BSD
    | PCRE2_EXTRA_ASCII_BSS
    | PCRE2_EXTRA_ASCII_BSW
    | PCRE2_EXTRA_ASCII_DIGIT
    | PCRE2_EXTRA_ASCII_POSIX;

/* States used for analyzing ranges in character classes. */

const RANGE_NO: u32 = 0;
const RANGE_STARTED: u32 = 1;
const RANGE_FORBID_NO: u32 = 2;
const RANGE_FORBID_STARTED: u32 = 3;
const RANGE_OK_ESCAPED: u32 = 4;
const RANGE_OK_LITERAL: u32 = 5;

/* States used for analyzing operators and operands in extended classes. */

const CLASS_OP_EMPTY: u32 = 0;
const CLASS_OP_OPERAND: u32 = 1;
const CLASS_OP_OPERATOR: u32 = 2;

/* States used for determining the parse mode in character classes. */

const CLASS_MODE_NORMAL: u32 = 0;
const CLASS_MODE_ALT_EXT: u32 = 1;
const CLASS_MODE_PERL_EXT: u32 = 2;
const CLASS_MODE_PERL_EXT_LEAF: u32 = 3;

/* Emulation of the C `IS_NEWLINE(p)` macro inside parse_regex, where NLBLOCK is
`cb` and PSEND is `end_pattern`. */
#[inline]
unsafe fn is_newline_at(cb: *mut compile_block, p: PCRE2_SPTR, utf: BOOL) -> bool {
    unsafe {
        if (*cb).nltype != NLTYPE_FIXED {
            p < (*cb).end_pattern
                && crate::newline::is_newline(
                    p,
                    (*cb).nltype,
                    (*cb).end_pattern,
                    &mut (*cb).nllen,
                    utf,
                ) != FALSE
        } else {
            p <= (*cb).end_pattern.sub((*cb).nllen as usize)
                && *p == (*cb).nl[0]
                && ((*cb).nllen == 1 || *p.add(1) == (*cb).nl[1])
        }
    }
}

/*************************************************
*      Parse regex and identify meta items       *
*************************************************/

pub(crate) unsafe fn parse_regex(
    ptr_in: PCRE2_SPTR,
    mut options: u32,
    mut xoptions: u32,
    has_lookbehind: *mut BOOL,
    cb: *mut compile_block,
) -> c_int {
    unsafe {
        let mut ptr: PCRE2_SPTR = ptr_in;
        let mut c: u32;
        let mut namelen: u32 = 0;
        let mut class_range_state: u32 = 0;
        let mut class_op_state: u32 = 0;
        let mut class_mode_state: u32 = 0;
        let mut class_start: *mut u32 = core::ptr::null_mut();
        let mut verblengthptr: *mut u32 = core::ptr::null_mut();
        let mut verbstartptr: *mut u32 = core::ptr::null_mut();
        let mut previous_callout: *mut u32 = core::ptr::null_mut();
        let mut parsed_pattern: *mut u32 = (*cb).parsed_pattern;
        let parsed_pattern_end: *mut u32 = (*cb).parsed_pattern_end;
        let mut this_parsed_item: *mut u32 = core::ptr::null_mut();
        let mut prev_parsed_item: *mut u32 = core::ptr::null_mut();
        let mut meta_quantifier: u32 = 0;
        let mut add_after_mark: u32 = 0;
        let mut nest_depth: u16 = 0;
        let mut class_depth_m1: i16 = -1; /* The m1 means minus 1. */
        let mut class_maxdepth_m1: i16 = -1;
        let mut hash: u16 = 0;
        let mut after_manual_callout: c_int = 0;
        let mut expect_cond_assert: c_int = 0;
        let mut errorcode: c_int = 0;
        let mut escape: c_int = 0;
        let mut i: c_int = 0;
        let mut inescq: BOOL = FALSE;
        let mut inverbname: BOOL = FALSE;
        let utf: BOOL = ((options & PCRE2_UTF) != 0) as BOOL;
        let auto_callout: BOOL = ((options & PCRE2_AUTO_CALLOUT) != 0) as BOOL;
        let mut is_dupname: BOOL = FALSE;
        let mut negate_class: BOOL = FALSE;
        let mut okquantifier: BOOL = FALSE;
        let mut thisptr: PCRE2_SPTR;
        let mut name: PCRE2_SPTR = core::ptr::null();
        let ptrend: PCRE2_SPTR = (*cb).end_pattern;
        let mut verbnamestart: PCRE2_SPTR = core::ptr::null();
        let mut class_range_forbid_ptr: PCRE2_SPTR = core::ptr::null();
        let mut ng: *mut named_group = core::ptr::null_mut();
        let mut top_nest: *mut nest_save = core::ptr::null_mut();
        let mut end_nests: *mut nest_save = core::ptr::null_mut();

        /* `PARSED_LITERAL(c, parsed_pattern)`. */
        macro_rules! PARSED_LITERAL {
            ($cc:expr) => {{
                *parsed_pattern = $cc;
                parsed_pattern = parsed_pattern.add(1);
                okquantifier = TRUE;
            }};
        }

        /* Insert leading items for word and line matching. */
        if (xoptions & PCRE2_EXTRA_MATCH_LINE) != 0 {
            *parsed_pattern = META_CIRCUMFLEX;
            parsed_pattern = parsed_pattern.add(1);
            *parsed_pattern = META_NOCAPTURE;
            parsed_pattern = parsed_pattern.add(1);
        } else if (xoptions & PCRE2_EXTRA_MATCH_WORD) != 0 {
            *parsed_pattern = META_ESCAPE + ESC_b as u32;
            parsed_pattern = parsed_pattern.add(1);
            *parsed_pattern = META_NOCAPTURE;
            parsed_pattern = parsed_pattern.add(1);
        }

        /* The whole function body is a labelled block that yields the return
        value. `break 'exit <val>` corresponds to `return <val>`. All the goto
        FAILED / FAILED_BACK / FAILED_FORWARD / UNCLOSED_PARENTHESIS paths set
        `errorcode`, adjust `ptr`, then jump to the FAILED handling that lives
        just after the scan loop, implemented with `break 'scan`. */

        'exit: {
            /* If the pattern is actually a literal string, process it
            separately to avoid cluttering up the main loop. */
            if (options & PCRE2_LITERAL) != 0 {
                'litscan: {
                    while ptr < ptrend {
                        if parsed_pattern >= parsed_pattern_end {
                            errorcode = ERR63;
                            break 'litscan;
                        }
                        thisptr = ptr;
                        c = getcharinctest(&mut ptr, utf != 0);
                        if auto_callout != 0 {
                            parsed_pattern = manage_callouts(
                                thisptr,
                                &mut previous_callout,
                                auto_callout,
                                parsed_pattern,
                                cb,
                            );
                        }
                        PARSED_LITERAL!(c);
                    }

                    /* PARSED_END for the literal path. */
                    parsed_pattern = manage_callouts(
                        ptr,
                        &mut previous_callout,
                        auto_callout,
                        parsed_pattern,
                        cb,
                    );
                    if (xoptions & PCRE2_EXTRA_MATCH_LINE) != 0 {
                        *parsed_pattern = META_KET;
                        parsed_pattern = parsed_pattern.add(1);
                        *parsed_pattern = META_DOLLAR;
                        parsed_pattern = parsed_pattern.add(1);
                    } else if (xoptions & PCRE2_EXTRA_MATCH_WORD) != 0 {
                        *parsed_pattern = META_KET;
                        parsed_pattern = parsed_pattern.add(1);
                        *parsed_pattern = META_ESCAPE + ESC_b as u32;
                        parsed_pattern = parsed_pattern.add(1);
                    }
                    if parsed_pattern >= parsed_pattern_end {
                        errorcode = ERR63;
                        break 'litscan;
                    }
                    *parsed_pattern = META_END;
                    if nest_depth == 0 {
                        break 'exit 0;
                    }
                    errorcode = ERR14; /* UNCLOSED_PARENTHESIS */
                }
                /* FAILED for literal path. */
                (*cb).erroroffset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                break 'exit errorcode;
            }

            /* Process a real regex which may contain meta-characters. */

            top_nest = core::ptr::null_mut();
            end_nests =
                ((*cb).start_workspace as *mut u8).add((*cb).workspace_size) as *mut nest_save;

            /* Round down end_nests so as not to span the end of the workspace. */
            end_nests = ((end_nests as *mut c_char).sub(
                ((*cb).workspace_size * core::mem::size_of::<PCRE2_UCHAR>())
                    % core::mem::size_of::<nest_save>(),
            )) as *mut nest_save;

            /* PCRE2_EXTENDED_MORE implies PCRE2_EXTENDED */
            if (options & PCRE2_EXTENDED_MORE) != 0 {
                options |= PCRE2_EXTENDED;
            }

            /* The scan loop is wrapped in a labelled block so error paths can
            `break 'scan`, landing on the FAILED handler. */
            'scan: {

            'mainloop: while ptr < ptrend {
                let prev_expect_cond_assert: c_int;
                let mut min_repeat: u32 = 0;
                let mut max_repeat: u32 = 0;
                let mut terminator: u32;
                let prev_meta_quantifier: u32;
                let prev_okquantifier: BOOL;
                let mut tempptr: PCRE2_SPTR = core::ptr::null();
                let mut offset: PCRE2_SIZE = 0;

                if nest_depth > (*(*cb).cx).parens_nest_limit as u16 {
                    errorcode = ERR19;
                    break 'scan; /* Parentheses too deeply nested */
                }

                if parsed_pattern >= parsed_pattern_end {
                    errorcode = ERR63; /* parsed pattern overflow */
                    break 'scan;
                }

                /* Remember where the previous item started. */
                if this_parsed_item != parsed_pattern {
                    prev_parsed_item = this_parsed_item;
                    this_parsed_item = parsed_pattern;
                }

                /* Get next input character, save its position. */
                thisptr = ptr;
                c = getcharinctest(&mut ptr, utf != 0);

                /* Copy quoted literals until \E. */
                if inescq != 0 {
                    if c == CHAR_BACKSLASH && ptr < ptrend && *ptr as u32 == CHAR_E {
                        inescq = FALSE;
                        ptr = ptr.add(1); /* Skip E */
                    } else {
                        if inverbname != 0 {
                            *parsed_pattern = c;
                            parsed_pattern = parsed_pattern.add(1);
                        } else {
                            let ao = after_manual_callout;
                            after_manual_callout -= 1;
                            if ao <= 0 {
                                parsed_pattern = manage_callouts(
                                    thisptr,
                                    &mut previous_callout,
                                    auto_callout,
                                    parsed_pattern,
                                    cb,
                                );
                            }
                            PARSED_LITERAL!(c);
                        }
                        meta_quantifier = 0;
                    }
                    continue 'mainloop; /* Next character */
                }

                /* Verb name processing. */
                if inverbname != 0
                    && (((options & (PCRE2_EXTENDED | PCRE2_ALT_VERBNAMES))
                        != (PCRE2_EXTENDED | PCRE2_ALT_VERBNAMES))
                        || (c > 255 && (c | 1) != 0x200f && (c | 1) != 0x2029)
                        || (c < 256
                            && c != CHAR_NUMBER_SIGN
                            && ((*(*cb).ctypes.add(c as usize)) & ctype_space) == 0
                            && c != CHAR_NEL))
                {
                    match c {
                        CHAR_RIGHT_PARENTHESIS => {
                            inverbname = FALSE;
                            let verbnamelength: PCRE2_SIZE =
                                parsed_pattern.offset_from(verblengthptr) as PCRE2_SIZE - 1;
                            if ptr.offset_from(verbnamestart) - 1 > MAX_MARK as isize {
                                ptr = ptr.sub(1);
                                errorcode = ERR76;
                                break 'scan;
                            }
                            *verblengthptr = verbnamelength as u32;

                            if add_after_mark != 0 {
                                *parsed_pattern = add_after_mark;
                                parsed_pattern = parsed_pattern.add(1);
                                add_after_mark = 0;
                            }
                        }

                        CHAR_BACKSLASH => {
                            if (options & PCRE2_ALT_VERBNAMES) != 0 {
                                escape = check_escape(
                                    &mut ptr,
                                    ptrend,
                                    &mut c,
                                    &mut errorcode,
                                    options,
                                    xoptions,
                                    (*cb).bracount,
                                    FALSE,
                                    cb,
                                );
                                if errorcode != 0 {
                                    break 'scan;
                                }
                            } else {
                                escape = 0; /* Treat all as literal */
                            }

                            if escape == 0 {
                                *parsed_pattern = c;
                                parsed_pattern = parsed_pattern.add(1);
                            } else if escape == ESC_ub {
                                *parsed_pattern = CHAR_u;
                                parsed_pattern = parsed_pattern.add(1);
                                PARSED_LITERAL!(CHAR_LEFT_CURLY_BRACKET);
                            } else if escape == ESC_Q {
                                inescq = TRUE;
                            } else if escape == ESC_E {
                                /* Ignore */
                            } else {
                                errorcode = ERR40; /* Invalid in verb name */
                                break 'scan;
                            }
                        }

                        _ => {
                            *parsed_pattern = c;
                            parsed_pattern = parsed_pattern.add(1);
                        }
                    }
                    continue 'mainloop; /* Next character in pattern */
                }

                /* Not a verb name character. Handle \Q and \E here. */
                if c == CHAR_BACKSLASH && ptr < ptrend {
                    if *ptr as u32 == CHAR_Q || *ptr as u32 == CHAR_E {
                        if expect_cond_assert > 0
                            && *ptr as u32 == CHAR_Q
                            && !(ptrend.offset_from(ptr) >= 3
                                && *ptr.add(1) as u32 == CHAR_BACKSLASH
                                && *ptr.add(2) as u32 == CHAR_E)
                        {
                            ptr = ptr.sub(1);
                            errorcode = ERR28;
                            break 'scan;
                        }
                        inescq = (*ptr as u32 == CHAR_Q) as BOOL;
                        ptr = ptr.add(1);
                        continue 'mainloop;
                    }
                }

                /* Skip over whitespace and # comments in extended mode. */
                if (options & PCRE2_EXTENDED) != 0 {
                    if c < 256 && ((*(*cb).ctypes.add(c as usize)) & ctype_space) != 0 {
                        continue 'mainloop;
                    }
                    if c == CHAR_NEL || (c | 1) == 0x200f || (c | 1) == 0x2029 {
                        continue 'mainloop;
                    }
                    if c == CHAR_NUMBER_SIGN {
                        while ptr < ptrend {
                            if is_newline_at(cb, ptr, utf) {
                                ptr = ptr.add((*cb).nllen as usize);
                                break;
                            }
                            ptr = ptr.add(1);
                            if utf != 0 {
                                forwardchartest(&mut ptr, ptrend);
                            }
                        }
                        continue 'mainloop;
                    }
                }

                /* Skip over bracketed comments */
                if c == CHAR_LEFT_PARENTHESIS
                    && ptrend.offset_from(ptr) >= 2
                    && *ptr.add(0) as u32 == CHAR_QUESTION_MARK
                    && *ptr.add(1) as u32 == CHAR_NUMBER_SIGN
                {
                    ptr = ptr.add(1);
                    while ptr < ptrend && *ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                        ptr = ptr.add(1);
                    }
                    if ptr >= ptrend {
                        errorcode = ERR18;
                        break 'scan;
                    }
                    ptr = ptr.add(1);
                    continue 'mainloop;
                }

                /* If the next item is not a quantifier, fill in length of any
                previous callout and create an auto callout if required. */
                {
                    let mut not_quant = c != CHAR_ASTERISK
                        && c != CHAR_PLUS
                        && c != CHAR_QUESTION_MARK;
                    if not_quant && c == CHAR_LEFT_CURLY_BRACKET {
                        let mut tp = ptr;
                        if read_repeat_counts(
                            &mut tp,
                            ptrend,
                            core::ptr::null_mut(),
                            core::ptr::null_mut(),
                            &mut errorcode,
                        ) != FALSE
                        {
                            not_quant = false;
                        }
                    }
                    if not_quant {
                        let ao = after_manual_callout;
                        after_manual_callout -= 1;
                        if ao <= 0 {
                            parsed_pattern = manage_callouts(
                                thisptr,
                                &mut previous_callout,
                                auto_callout,
                                parsed_pattern,
                                cb,
                            );
                            this_parsed_item = parsed_pattern;
                        }
                    }
                }

                /* Conditional assertion expectation checks. */
                if expect_cond_assert > 0 {
                    let mut ok = c == CHAR_LEFT_PARENTHESIS
                        && ptrend.offset_from(ptr) >= 3
                        && (*ptr.add(0) as u32 == CHAR_QUESTION_MARK
                            || *ptr.add(0) as u32 == CHAR_ASTERISK);
                    if ok {
                        if *ptr.add(0) as u32 == CHAR_ASTERISK {
                            ok = chmax_255(*ptr.add(1) as u32)
                                && ((*(*cb).ctypes.add(*ptr.add(1) as usize)) & ctype_lcletter)
                                    != 0;
                        } else {
                            match *ptr.add(1) as u32 {
                                CHAR_C => ok = expect_cond_assert == 2,
                                CHAR_EQUALS_SIGN | CHAR_EXCLAMATION_MARK => {}
                                CHAR_LESS_THAN_SIGN => {
                                    ok = *ptr.add(2) as u32 == CHAR_EQUALS_SIGN
                                        || *ptr.add(2) as u32 == CHAR_EXCLAMATION_MARK;
                                }
                                _ => ok = false,
                            }
                        }
                    }

                    if !ok {
                        errorcode = ERR28;
                        if expect_cond_assert == 2 {
                            break 'scan;
                        }
                        /* goto FAILED_BACK */
                        ptr = ptr.sub(1);
                        if utf != 0 {
                            backchar(&mut ptr);
                        }
                        break 'scan;
                    }
                }

                prev_expect_cond_assert = expect_cond_assert;
                expect_cond_assert = 0;

                prev_okquantifier = okquantifier;
                prev_meta_quantifier = meta_quantifier;
                okquantifier = FALSE;
                meta_quantifier = 0;

                /* Following modifier for a previous quantifier. */
                if prev_meta_quantifier != 0
                    && (c == CHAR_QUESTION_MARK || c == CHAR_PLUS)
                {
                    let idx: isize = if prev_meta_quantifier == META_MINMAX { -3 } else { -1 };
                    *parsed_pattern.offset(idx) = prev_meta_quantifier
                        + (if c == CHAR_QUESTION_MARK {
                            0x00020000u32
                        } else {
                            0x00010000u32
                        });
                    continue 'mainloop;
                }

                /* Process the next item. The C `switch(c)` with its internal
                gotos is modelled with a labelled block `'itemsw`; a plain
                `break 'itemsw` corresponds to the C `break` out of the switch.
                Shared goto targets used from multiple cases are implemented as
                labelled sub-blocks or by setting `goto_target` and re-entering. */

                'itemsw: {
                    /* Shared meta-quantifier value for the CHECK_QUANTIFIER
                    path; set before jumping there. */
                    let mut check_quant_meta: u32;

                    /* Dispatch for the (? / alpha-assertion shared goto labels.
                    A helper closure isn't usable due to borrows, so these are
                    handled by structuring the code below. */

                    match c {
                        /* ---- Escape sequence ---- */
                        CHAR_BACKSLASH => {
                            tempptr = ptr;
                            escape = check_escape(
                                &mut ptr,
                                ptrend,
                                &mut c,
                                &mut errorcode,
                                options,
                                xoptions,
                                (*cb).bracount,
                                FALSE,
                                cb,
                            );

                            /* The 'escape_failed block emulates the ESCAPE_FAILED
                            label: on entry we treat the escape as literal if
                            PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL is set, else fail. */
                            let mut escape_failed = errorcode != 0;

                            'esc: {
                                if escape_failed {
                                    if (xoptions & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL) == 0 {
                                        break 'scan;
                                    }
                                    ptr = tempptr;
                                    if ptr >= ptrend {
                                        c = CHAR_BACKSLASH;
                                    } else {
                                        c = getcharinctest(&mut ptr, utf != 0);
                                    }
                                    escape = 0;
                                    escape_failed = false;
                                }

                                if escape == 0 {
                                    PARSED_LITERAL!(c);
                                    break 'esc;
                                } else if escape < 0 {
                                    offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                                    escape = -escape - 1;
                                    *parsed_pattern = META_BACKREF | (escape as u32);
                                    parsed_pattern = parsed_pattern.add(1);
                                    if escape < 10 {
                                        if (*cb).small_ref_offset[escape as usize] == PCRE2_UNSET {
                                            (*cb).small_ref_offset[escape as usize] = offset;
                                        }
                                    } else {
                                        putoffset(offset, &mut parsed_pattern);
                                    }
                                    okquantifier = TRUE;
                                    break 'esc;
                                }

                                /* escape > 0: special escape indicator. */
                                'esc_switch: {
                                    if escape == ESC_C {
                                        if (options & PCRE2_NEVER_BACKSLASH_C) != 0 {
                                            errorcode = ERR83;
                                            /* goto ESCAPE_FAILED */
                                            if (xoptions & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL) == 0 {
                                                break 'scan;
                                            }
                                            ptr = tempptr;
                                            if ptr >= ptrend {
                                                c = CHAR_BACKSLASH;
                                            } else {
                                                c = getcharinctest(&mut ptr, utf != 0);
                                            }
                                            PARSED_LITERAL!(c);
                                            break 'esc;
                                        }
                                        okquantifier = TRUE;
                                        *parsed_pattern = META_ESCAPE + escape as u32;
                                        parsed_pattern = parsed_pattern.add(1);
                                        break 'esc_switch;
                                    }

                                    if escape == ESC_ub {
                                        *parsed_pattern = CHAR_u;
                                        parsed_pattern = parsed_pattern.add(1);
                                        PARSED_LITERAL!(CHAR_LEFT_CURLY_BRACKET);
                                        break 'esc_switch;
                                    }

                                    if escape == ESC_X
                                        || escape == ESC_H
                                        || escape == ESC_h
                                        || escape == ESC_N
                                        || escape == ESC_R
                                        || escape == ESC_V
                                        || escape == ESC_v
                                    {
                                        okquantifier = TRUE;
                                        *parsed_pattern = META_ESCAPE + escape as u32;
                                        parsed_pattern = parsed_pattern.add(1);
                                        break 'esc_switch;
                                    }

                                    if escape == ESC_d
                                        || escape == ESC_D
                                        || escape == ESC_s
                                        || escape == ESC_S
                                        || escape == ESC_w
                                        || escape == ESC_W
                                    {
                                        okquantifier = TRUE;
                                        parsed_pattern =
                                            handle_escdsw(escape, parsed_pattern, options, xoptions);
                                        break 'esc_switch;
                                    }

                                    if escape == ESC_P || escape == ESC_p {
                                        let mut negated: BOOL = FALSE;
                                        let mut ptype: u16 = 0;
                                        let mut pdata: u16 = 0;
                                        if get_ucp(
                                            &mut ptr,
                                            utf,
                                            &mut negated,
                                            &mut ptype,
                                            &mut pdata,
                                            &mut errorcode,
                                            cb,
                                        ) == FALSE
                                        {
                                            /* goto ESCAPE_FAILED */
                                            if (xoptions & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL) == 0 {
                                                break 'scan;
                                            }
                                            ptr = tempptr;
                                            if ptr >= ptrend {
                                                c = CHAR_BACKSLASH;
                                            } else {
                                                c = getcharinctest(&mut ptr, utf != 0);
                                            }
                                            PARSED_LITERAL!(c);
                                            break 'esc;
                                        }
                                        if negated != 0 {
                                            escape = if escape == ESC_P { ESC_p } else { ESC_P };
                                        }
                                        *parsed_pattern = META_ESCAPE + escape as u32;
                                        parsed_pattern = parsed_pattern.add(1);
                                        *parsed_pattern = ((ptype as u32) << 16) | pdata as u32;
                                        parsed_pattern = parsed_pattern.add(1);
                                        okquantifier = TRUE;
                                        break 'esc_switch;
                                    }

                                    if escape == ESC_g || escape == ESC_k {
                                        if ptr >= ptrend
                                            || (*ptr as u32 != CHAR_LEFT_CURLY_BRACKET
                                                && *ptr as u32 != CHAR_LESS_THAN_SIGN
                                                && *ptr as u32 != CHAR_APOSTROPHE)
                                        {
                                            errorcode =
                                                if escape == ESC_g { ERR57 } else { ERR69 };
                                            /* goto ESCAPE_FAILED */
                                            if (xoptions & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL) == 0 {
                                                break 'scan;
                                            }
                                            ptr = tempptr;
                                            if ptr >= ptrend {
                                                c = CHAR_BACKSLASH;
                                            } else {
                                                c = getcharinctest(&mut ptr, utf != 0);
                                            }
                                            PARSED_LITERAL!(c);
                                            break 'esc;
                                        }
                                        let term = if *ptr as u32 == CHAR_LESS_THAN_SIGN {
                                            CHAR_GREATER_THAN_SIGN
                                        } else if *ptr as u32 == CHAR_APOSTROPHE {
                                            CHAR_APOSTROPHE
                                        } else {
                                            CHAR_RIGHT_CURLY_BRACKET
                                        };
                                        terminator = term;

                                        /* For a non-braced \g, check for
                                        numerical recursion. */
                                        if escape == ESC_g
                                            && terminator != CHAR_RIGHT_CURLY_BRACKET
                                        {
                                            let mut p = ptr.add(1);
                                            if read_number(
                                                &mut p,
                                                ptrend,
                                                (*cb).bracount as i32,
                                                MAX_GROUP_NUMBER,
                                                ERR61 as u32,
                                                &mut i,
                                                &mut errorcode,
                                            ) != FALSE
                                            {
                                                if p >= ptrend || *p as u32 != terminator {
                                                    ptr = p;
                                                    errorcode = ERR119;
                                                    if (xoptions
                                                        & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL)
                                                        == 0
                                                    {
                                                        break 'scan;
                                                    }
                                                    ptr = tempptr;
                                                    if ptr >= ptrend {
                                                        c = CHAR_BACKSLASH;
                                                    } else {
                                                        c = getcharinctest(&mut ptr, utf != 0);
                                                    }
                                                    PARSED_LITERAL!(c);
                                                    break 'esc;
                                                }
                                                ptr = p.add(1);
                                                /* goto SET_RECURSION */
                                                *parsed_pattern = META_RECURSE | (i as u32);
                                                parsed_pattern = parsed_pattern.add(1);
                                                offset = ptr.offset_from((*cb).start_pattern)
                                                    as PCRE2_SIZE;
                                                /* READ_RECURSION_ARGUMENTS */
                                                putoffset(offset, &mut parsed_pattern);
                                                okquantifier = TRUE;
                                                /* terminator != NUL so args not
                                                supported: break. */
                                                break 'esc_switch;
                                            }
                                            if errorcode != 0 {
                                                /* goto ESCAPE_FAILED */
                                                if (xoptions & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL)
                                                    == 0
                                                {
                                                    break 'scan;
                                                }
                                                ptr = tempptr;
                                                if ptr >= ptrend {
                                                    c = CHAR_BACKSLASH;
                                                } else {
                                                    c = getcharinctest(&mut ptr, utf != 0);
                                                }
                                                PARSED_LITERAL!(c);
                                                break 'esc;
                                            }
                                        }

                                        /* Not a numerical recursion. */
                                        if read_name(
                                            &mut ptr,
                                            ptrend,
                                            utf,
                                            terminator,
                                            &mut offset,
                                            &mut name,
                                            &mut namelen,
                                            &mut errorcode,
                                            cb,
                                        ) == FALSE
                                        {
                                            /* goto ESCAPE_FAILED */
                                            if (xoptions & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL) == 0 {
                                                break 'scan;
                                            }
                                            ptr = tempptr;
                                            if ptr >= ptrend {
                                                c = CHAR_BACKSLASH;
                                            } else {
                                                c = getcharinctest(&mut ptr, utf != 0);
                                            }
                                            PARSED_LITERAL!(c);
                                            break 'esc;
                                        }

                                        *parsed_pattern = if escape == ESC_k
                                            || terminator == CHAR_RIGHT_CURLY_BRACKET
                                        {
                                            META_BACKREF_BYNAME
                                        } else {
                                            META_RECURSE_BYNAME
                                        };
                                        parsed_pattern = parsed_pattern.add(1);
                                        *parsed_pattern = namelen;
                                        parsed_pattern = parsed_pattern.add(1);
                                        putoffset(offset, &mut parsed_pattern);
                                        okquantifier = TRUE;
                                        break 'esc_switch;
                                    }

                                    /* Default: \A, \B, \b, \G, \K, \Z, \z. */
                                    *parsed_pattern = META_ESCAPE + escape as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                } /* 'esc_switch */
                            } /* 'esc */
                            break 'itemsw;
                        }

                        /* ---- Single-character special items ---- */
                        CHAR_CIRCUMFLEX_ACCENT => {
                            *parsed_pattern = META_CIRCUMFLEX;
                            parsed_pattern = parsed_pattern.add(1);
                            break 'itemsw;
                        }

                        CHAR_DOLLAR_SIGN => {
                            *parsed_pattern = META_DOLLAR;
                            parsed_pattern = parsed_pattern.add(1);
                            break 'itemsw;
                        }

                        CHAR_DOT => {
                            *parsed_pattern = META_DOT;
                            parsed_pattern = parsed_pattern.add(1);
                            okquantifier = TRUE;
                            break 'itemsw;
                        }

                        /* ---- Single-character quantifiers ---- */
                        CHAR_ASTERISK => {
                            check_quant_meta = META_ASTERISK;
                            /* goto CHECK_QUANTIFIER */
                            match do_check_quantifier(
                                c,
                                check_quant_meta,
                                prev_okquantifier,
                                prev_parsed_item,
                                verbstartptr,
                                &mut parsed_pattern,
                                &mut meta_quantifier,
                                min_repeat,
                                max_repeat,
                            ) {
                                Ok(()) => break 'itemsw,
                                Err(e) => {
                                    errorcode = e;
                                    break 'scan;
                                }
                            }
                        }

                        CHAR_PLUS => {
                            check_quant_meta = META_PLUS;
                            match do_check_quantifier(
                                c,
                                check_quant_meta,
                                prev_okquantifier,
                                prev_parsed_item,
                                verbstartptr,
                                &mut parsed_pattern,
                                &mut meta_quantifier,
                                min_repeat,
                                max_repeat,
                            ) {
                                Ok(()) => break 'itemsw,
                                Err(e) => {
                                    errorcode = e;
                                    break 'scan;
                                }
                            }
                        }

                        CHAR_QUESTION_MARK => {
                            check_quant_meta = META_QUERY;
                            match do_check_quantifier(
                                c,
                                check_quant_meta,
                                prev_okquantifier,
                                prev_parsed_item,
                                verbstartptr,
                                &mut parsed_pattern,
                                &mut meta_quantifier,
                                min_repeat,
                                max_repeat,
                            ) {
                                Ok(()) => break 'itemsw,
                                Err(e) => {
                                    errorcode = e;
                                    break 'scan;
                                }
                            }
                        }

                        /* ---- Potential {n,m} quantifier ---- */
                        CHAR_LEFT_CURLY_BRACKET => {
                            if read_repeat_counts(
                                &mut ptr,
                                ptrend,
                                &mut min_repeat,
                                &mut max_repeat,
                                &mut errorcode,
                            ) == FALSE
                            {
                                if errorcode != 0 {
                                    break 'scan;
                                }
                                PARSED_LITERAL!(c);
                                break 'itemsw;
                            }
                            check_quant_meta = META_MINMAX;
                            match do_check_quantifier(
                                c,
                                check_quant_meta,
                                prev_okquantifier,
                                prev_parsed_item,
                                verbstartptr,
                                &mut parsed_pattern,
                                &mut meta_quantifier,
                                min_repeat,
                                max_repeat,
                            ) {
                                Ok(()) => break 'itemsw,
                                Err(e) => {
                                    errorcode = e;
                                    break 'scan;
                                }
                            }
                        }

                        /* ---- Character class ---- */
                        CHAR_LEFT_SQUARE_BRACKET => {
                            /* [[:<:]] and [[:>:]] special word boundaries. */
                            if ptrend.offset_from(ptr) >= 6
                                && (crate::string_utils::strncmp_c8(
                                    ptr,
                                    STRING_WEIRD_STARTWORD.as_ptr() as *const c_char,
                                    6,
                                ) == 0
                                    || crate::string_utils::strncmp_c8(
                                        ptr,
                                        STRING_WEIRD_ENDWORD.as_ptr() as *const c_char,
                                        6,
                                    ) == 0)
                            {
                                *parsed_pattern = META_ESCAPE + ESC_b as u32;
                                parsed_pattern = parsed_pattern.add(1);

                                if *ptr.add(2) as u32 == CHAR_LESS_THAN_SIGN {
                                    *parsed_pattern = META_LOOKAHEAD;
                                    parsed_pattern = parsed_pattern.add(1);
                                } else {
                                    *parsed_pattern = META_LOOKBEHIND;
                                    parsed_pattern = parsed_pattern.add(1);
                                    *has_lookbehind = TRUE;
                                    putoffset(0 as PCRE2_SIZE, &mut parsed_pattern);
                                }

                                if (options & PCRE2_UCP) == 0 {
                                    *parsed_pattern = META_ESCAPE + ESC_w as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                } else {
                                    *parsed_pattern = META_ESCAPE + ESC_p as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                    *parsed_pattern = PT_WORD << 16;
                                    parsed_pattern = parsed_pattern.add(1);
                                }
                                *parsed_pattern = META_KET;
                                parsed_pattern = parsed_pattern.add(1);
                                ptr = ptr.add(6);
                                okquantifier = TRUE;
                                break 'itemsw;
                            }

                            /* POSIX class stuff at the top level is an error. */
                            if ptr < ptrend
                                && (*ptr as u32 == CHAR_COLON
                                    || *ptr as u32 == CHAR_DOT
                                    || *ptr as u32 == CHAR_EQUALS_SIGN)
                                && check_posix_syntax(ptr, ptrend, &mut tempptr) != FALSE
                            {
                                errorcode = if *ptr as u32 == CHAR_COLON { ERR12 } else { ERR13 };
                                /* The `*ptr--` in the C decrements ptr, but ptr is
                                immediately overwritten, so the decrement is dead. */
                                ptr = tempptr.add(2);
                                break 'scan;
                            }

                            class_mode_state = if (options & PCRE2_ALT_EXTENDED_CLASS) != 0 {
                                CLASS_MODE_ALT_EXT
                            } else {
                                CLASS_MODE_NORMAL
                            };

                            /* FROM_PERL_EXTENDED_CLASS entry point. */
                            match parse_class_body(
                                &mut c,
                                &mut ptr,
                                ptrend,
                                options,
                                xoptions,
                                &mut parsed_pattern,
                                &mut class_range_state,
                                &mut class_op_state,
                                &mut class_mode_state,
                                &mut class_start,
                                &mut class_depth_m1,
                                &mut class_maxdepth_m1,
                                &mut class_range_forbid_ptr,
                                &mut inescq,
                                &mut negate_class,
                                &mut okquantifier,
                                utf,
                                cb,
                            ) {
                                Ok(()) => break 'itemsw,
                                Err(e) => {
                                    errorcode = e;
                                    break 'scan;
                                }
                            }
                        }

                        /* ---- Opening parenthesis ---- */
                        CHAR_LEFT_PARENTHESIS => {
                            match parse_paren(
                                &mut c,
                                &mut ptr,
                                ptrend,
                                &mut options,
                                &mut xoptions,
                                &mut parsed_pattern,
                                &mut this_parsed_item,
                                &mut prev_parsed_item,
                                &mut previous_callout,
                                &mut verbstartptr,
                                &mut verblengthptr,
                                &mut verbnamestart,
                                &mut add_after_mark,
                                &mut nest_depth,
                                &mut class_mode_state,
                                &mut after_manual_callout,
                                &mut expect_cond_assert,
                                prev_expect_cond_assert,
                                &mut inescq,
                                &mut inverbname,
                                utf,
                                auto_callout,
                                &mut okquantifier,
                                &mut is_dupname,
                                &mut hash,
                                &mut ng,
                                &mut name,
                                &mut namelen,
                                &mut top_nest,
                                end_nests,
                                has_lookbehind,
                                cb,
                            ) {
                                ParenResult::Break => break 'itemsw,
                                ParenResult::FromPerlClass => {
                                    /* (?[...]) => parse extended class. */
                                    match parse_class_body(
                                        &mut c,
                                        &mut ptr,
                                        ptrend,
                                        options,
                                        xoptions,
                                        &mut parsed_pattern,
                                        &mut class_range_state,
                                        &mut class_op_state,
                                        &mut class_mode_state,
                                        &mut class_start,
                                        &mut class_depth_m1,
                                        &mut class_maxdepth_m1,
                                        &mut class_range_forbid_ptr,
                                        &mut inescq,
                                        &mut negate_class,
                                        &mut okquantifier,
                                        utf,
                                        cb,
                                    ) {
                                        Ok(()) => break 'itemsw,
                                        Err(e) => {
                                            errorcode = e;
                                            break 'scan;
                                        }
                                    }
                                }
                                ParenResult::Failed(e) => {
                                    errorcode = e;
                                    break 'scan;
                                }
                            }
                        }

                        /* ---- Branch terminators ---- */
                        CHAR_VERTICAL_LINE => {
                            if !top_nest.is_null()
                                && (*top_nest).nest_depth == nest_depth
                                && ((*top_nest).flags & NSF_RESET) != 0
                            {
                                if (*cb).bracount > (*top_nest).max_group as u32 {
                                    (*top_nest).max_group = (*cb).bracount as u16;
                                }
                                (*cb).bracount = (*top_nest).reset_group as u32;
                            }
                            *parsed_pattern = META_ALT;
                            parsed_pattern = parsed_pattern.add(1);
                            break 'itemsw;
                        }

                        CHAR_RIGHT_PARENTHESIS => {
                            okquantifier = TRUE;
                            if !top_nest.is_null() && (*top_nest).nest_depth == nest_depth {
                                options = (options & !PARSE_TRACKED_OPTIONS)
                                    | (*top_nest).options;
                                xoptions = (xoptions & !PARSE_TRACKED_EXTRA_OPTIONS)
                                    | (*top_nest).xoptions;
                                if ((*top_nest).flags & NSF_RESET) != 0
                                    && (*top_nest).max_group as u32 > (*cb).bracount
                                {
                                    (*cb).bracount = (*top_nest).max_group as u32;
                                }
                                if ((*top_nest).flags & NSF_CONDASSERT) != 0 {
                                    okquantifier = FALSE;
                                }

                                if ((*top_nest).flags & NSF_ATOMICSR) != 0 {
                                    *parsed_pattern = META_KET;
                                    parsed_pattern = parsed_pattern.add(1);
                                }

                                if top_nest == (*cb).start_workspace as *mut nest_save {
                                    top_nest = core::ptr::null_mut();
                                } else {
                                    top_nest = top_nest.sub(1);
                                }
                            }
                            if nest_depth == 0 {
                                errorcode = ERR22; /* Unmatched closing parenthesis */
                                break 'scan;
                            }
                            nest_depth -= 1;
                            *parsed_pattern = META_KET;
                            parsed_pattern = parsed_pattern.add(1);
                            break 'itemsw;
                        }

                        /* default: Non-special character */
                        _ => {
                            PARSED_LITERAL!(c);
                            break 'itemsw;
                        }
                    } /* match c */
                } /* 'itemsw */
            } /* End of main character scan loop 'mainloop */

            /* End of pattern reached. Check for missing ) at end of verb name. */
            if inverbname != 0 && ptr >= ptrend {
                errorcode = ERR60;
                break 'scan;
            }

            /* PARSED_END */
            parsed_pattern = manage_callouts(
                ptr,
                &mut previous_callout,
                auto_callout,
                parsed_pattern,
                cb,
            );

            if (xoptions & PCRE2_EXTRA_MATCH_LINE) != 0 {
                *parsed_pattern = META_KET;
                parsed_pattern = parsed_pattern.add(1);
                *parsed_pattern = META_DOLLAR;
                parsed_pattern = parsed_pattern.add(1);
            } else if (xoptions & PCRE2_EXTRA_MATCH_WORD) != 0 {
                *parsed_pattern = META_KET;
                parsed_pattern = parsed_pattern.add(1);
                *parsed_pattern = META_ESCAPE + ESC_b as u32;
                parsed_pattern = parsed_pattern.add(1);
            }

            if parsed_pattern >= parsed_pattern_end {
                errorcode = ERR63;
                break 'scan;
            }

            *parsed_pattern = META_END;
            if nest_depth == 0 {
                break 'exit 0;
            }

            errorcode = ERR14; /* UNCLOSED_PARENTHESIS */
        } /* 'scan */

        /* FAILED */
        (*cb).erroroffset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
        errorcode
        } /* 'exit */
    }
}

/* The CHECK_QUANTIFIER label. `meta_q` is the quantifier meta value that the C
code stored into `meta_quantifier` just before the goto; we set the caller's
`meta_quantifier` to it. Returns Ok(()) for the C `break`, Err(code) for a
`goto FAILED`. */
#[inline]
unsafe fn do_check_quantifier(
    c: u32,
    meta_q: u32,
    prev_okquantifier: BOOL,
    prev_parsed_item: *mut u32,
    verbstartptr: *mut u32,
    parsed_pattern: &mut *mut u32,
    meta_quantifier: &mut u32,
    min_repeat: u32,
    max_repeat: u32,
) -> Result<(), c_int> {
    unsafe {
        *meta_quantifier = meta_q;

        if prev_okquantifier == FALSE {
            return Err(ERR9);
        }

        if *prev_parsed_item == META_ACCEPT {
            let mut p = (*parsed_pattern).sub(1);
            while p >= verbstartptr {
                *p.add(1) = *p.add(0);
                p = p.sub(1);
            }
            *verbstartptr = META_NOCAPTURE;
            *(*parsed_pattern).add(1) = META_KET;
            *parsed_pattern = (*parsed_pattern).add(2);
        }

        **parsed_pattern = *meta_quantifier;
        *parsed_pattern = (*parsed_pattern).add(1);
        if c == CHAR_LEFT_CURLY_BRACKET {
            **parsed_pattern = min_repeat;
            *parsed_pattern = (*parsed_pattern).add(1);
            **parsed_pattern = max_repeat;
            *parsed_pattern = (*parsed_pattern).add(1);
        }
        Ok(())
    }
}

/* Outcome of processing one character within the class loop, before the
CLASS_CONTINUE tail. */
enum ClassStep {
    /* Fall through to CLASS_CONTINUE (read next char). */
    Next,
    /* C `continue` -- loop again with c already loaded. */
    Again,
    /* C `break` out of the class loop -- class finished. */
    EndClass,
    /* Run the CLASS_LITERAL code, then CLASS_CONTINUE. */
    Literal,
}

/* The character class parser, entered at the C `FROM_PERL_EXTENDED_CLASS`
label. On entry `*c` holds '[' (or the leaf opener) and `*ptr` points just
after it. Returns Ok(()) for the C `break` that ends the class case, or
Err(code) for `goto FAILED`. */
unsafe fn parse_class_body(
    c: &mut u32,
    ptr: &mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    options: u32,
    xoptions: u32,
    parsed_pattern: &mut *mut u32,
    class_range_state: &mut u32,
    class_op_state: &mut u32,
    class_mode_state: &mut u32,
    class_start: &mut *mut u32,
    class_depth_m1: &mut i16,
    class_maxdepth_m1: &mut i16,
    class_range_forbid_ptr: &mut PCRE2_SPTR,
    inescq: &mut BOOL,
    negate_class: &mut BOOL,
    okquantifier: &mut BOOL,
    utf: BOOL,
    cb: *mut compile_block,
) -> Result<(), c_int> {
    unsafe {
        macro_rules! PARSED_LITERAL {
            ($cc:expr) => {{
                **parsed_pattern = $cc;
                *parsed_pattern = (*parsed_pattern).add(1);
                *okquantifier = TRUE;
            }};
        }

        *okquantifier = TRUE;

        *class_depth_m1 = -1;
        *class_maxdepth_m1 = -1;
        *class_range_state = RANGE_NO;
        *class_op_state = CLASS_OP_EMPTY;
        *class_start = core::ptr::null_mut();

        loop {
            let mut char_is_literal: BOOL = TRUE;
            let mut tempptr: PCRE2_SPTR = core::ptr::null();

            /* The 'chain block computes a ClassStep. */
            let step: ClassStep = 'chain: {
                /* Inside \Q...\E everything is literal except \E */
                if *inescq != 0 {
                    if *c == CHAR_BACKSLASH && *ptr < ptrend && **ptr as u32 == CHAR_E {
                        *inescq = FALSE;
                        *ptr = ptr.add(1);
                        break 'chain ClassStep::Next; /* goto CLASS_CONTINUE */
                    }
                    if *class_mode_state == CLASS_MODE_PERL_EXT {
                        return Err(ERR116);
                    }
                    break 'chain ClassStep::Literal; /* goto CLASS_LITERAL */
                }

                /* Skip space and tab in extended-more / Perl-extended. */
                if (*c == CHAR_SPACE || *c == CHAR_HT)
                    && ((options & PCRE2_EXTENDED_MORE) != 0
                        || *class_mode_state >= CLASS_MODE_PERL_EXT)
                {
                    break 'chain ClassStep::Next; /* goto CLASS_CONTINUE */
                }

                /* Handle POSIX class names. */
                if *class_depth_m1 >= 0
                    && *c == CHAR_LEFT_SQUARE_BRACKET
                    && ptrend.offset_from(*ptr) >= 3
                    && (**ptr as u32 == CHAR_COLON
                        || **ptr as u32 == CHAR_DOT
                        || **ptr as u32 == CHAR_EQUALS_SIGN)
                    && check_posix_syntax(*ptr, ptrend, &mut tempptr) != FALSE
                {
                    let mut posix_negate: BOOL = FALSE;
                    let posix_class: c_int;

                    if *class_range_state == RANGE_STARTED {
                        *ptr = tempptr.add(2);
                        return Err(ERR50);
                    }

                    if *class_range_state == RANGE_FORBID_STARTED {
                        *ptr = *class_range_forbid_ptr;
                        return Err(ERR50);
                    }

                    if *class_op_state == CLASS_OP_OPERAND
                        && *class_mode_state == CLASS_MODE_PERL_EXT
                    {
                        *ptr = tempptr.add(2);
                        return Err(ERR113);
                    }

                    if **ptr as u32 != CHAR_COLON {
                        *ptr = tempptr.add(2);
                        return Err(ERR13);
                    }

                    *ptr = ptr.add(1);
                    if **ptr as u32 == CHAR_CIRCUMFLEX_ACCENT {
                        posix_negate = TRUE;
                        *ptr = ptr.add(1);
                    }

                    posix_class = check_posix_name(*ptr, tempptr.offset_from(*ptr) as c_int);
                    *ptr = tempptr.add(2);
                    if posix_class < 0 {
                        return Err(ERR30);
                    }

                    *class_range_state = RANGE_FORBID_NO;
                    *class_op_state = CLASS_OP_OPERAND;

                    /* PCRE2_UCP conversions. */
                    if (options & PCRE2_UCP) != 0
                        && (xoptions & PCRE2_EXTRA_ASCII_POSIX) == 0
                        && !((xoptions & PCRE2_EXTRA_ASCII_DIGIT) != 0
                            && (posix_class == PC_DIGIT as c_int
                                || posix_class == PC_XDIGIT as c_int))
                    {
                        let ptype = posix_substitutes[(2 * posix_class) as usize];
                        let pvalue = posix_substitutes[(2 * posix_class + 1) as usize];

                        if ptype >= 0 {
                            **parsed_pattern = META_ESCAPE
                                + (if posix_negate != 0 { ESC_P } else { ESC_p }) as u32;
                            *parsed_pattern = (*parsed_pattern).add(1);
                            **parsed_pattern = ((ptype as u32) << 16) | pvalue as u32;
                            *parsed_pattern = (*parsed_pattern).add(1);
                            break 'chain ClassStep::Next; /* goto CLASS_CONTINUE */
                        }

                        if pvalue != 0 {
                            **parsed_pattern = META_ESCAPE
                                + (if posix_negate != 0 { ESC_H } else { ESC_h }) as u32;
                            *parsed_pattern = (*parsed_pattern).add(1);
                            break 'chain ClassStep::Next; /* goto CLASS_CONTINUE */
                        }
                        /* Fall through */
                    }

                    /* Non-UCP POSIX class */
                    **parsed_pattern = if posix_negate != 0 { META_POSIX_NEG } else { META_POSIX };
                    *parsed_pattern = (*parsed_pattern).add(1);
                    **parsed_pattern = posix_class as u32;
                    *parsed_pattern = (*parsed_pattern).add(1);
                    break 'chain ClassStep::Next;
                }

                /* Start of outermost class, or start of a nested class. */
                if (*c == CHAR_LEFT_SQUARE_BRACKET
                    && (*class_depth_m1 < 0
                        || *class_mode_state == CLASS_MODE_ALT_EXT
                        || *class_mode_state == CLASS_MODE_PERL_EXT))
                    || (*c == CHAR_LEFT_PARENTHESIS
                        && *class_mode_state == CLASS_MODE_PERL_EXT)
                {
                    let start_c = *c;
                    let new_class_mode_state: u32;

                    if start_c == CHAR_LEFT_SQUARE_BRACKET
                        && *class_mode_state == CLASS_MODE_PERL_EXT
                        && *class_depth_m1 >= 0
                    {
                        new_class_mode_state = CLASS_MODE_PERL_EXT_LEAF;
                    } else {
                        new_class_mode_state = *class_mode_state;
                    }

                    /* -[ beginning a nested class is a literal '-' */
                    if *class_range_state == RANGE_STARTED {
                        *(*parsed_pattern).offset(-1) = CHAR_MINUS;
                    }

                    if *class_op_state == CLASS_OP_OPERAND
                        && *class_mode_state == CLASS_MODE_PERL_EXT
                    {
                        return Err(ERR113);
                    }

                    if *class_depth_m1 >= (ECLASS_NEST_LIMIT as i16) - 1 {
                        *ptr = ptr.sub(1);
                        return Err(ERR107);
                    }

                    /* Process the character class start. */
                    *negate_class = FALSE;
                    loop {
                        if *ptr >= ptrend {
                            return Err(if start_c == CHAR_LEFT_PARENTHESIS {
                                ERR14
                            } else {
                                ERR6
                            });
                        }

                        *c = getcharinctest(ptr, utf != 0);
                        if new_class_mode_state == CLASS_MODE_PERL_EXT {
                            break;
                        } else if *c == CHAR_BACKSLASH {
                            if *ptr < ptrend && **ptr as u32 == CHAR_E {
                                *ptr = ptr.add(1);
                            } else if ptrend.offset_from(*ptr) >= 3
                                && crate::string_utils::strncmp_c8(
                                    *ptr,
                                    class_qbe_str().as_ptr() as *const c_char,
                                    3,
                                ) == 0
                            {
                                *ptr = ptr.add(3);
                            } else {
                                break;
                            }
                        } else if (*c == CHAR_SPACE || *c == CHAR_HT)
                            && ((options & PCRE2_EXTENDED_MORE) != 0
                                || new_class_mode_state >= CLASS_MODE_PERL_EXT)
                        {
                            continue;
                        } else if *negate_class == FALSE && *c == CHAR_CIRCUMFLEX_ACCENT {
                            *negate_class = TRUE;
                        } else {
                            break;
                        }
                    }

                    /* Empty class handling. */
                    if *c == CHAR_RIGHT_SQUARE_BRACKET
                        && ((*cb).external_options & PCRE2_ALLOW_EMPTY_CLASS) != 0
                        && new_class_mode_state < CLASS_MODE_PERL_EXT
                    {
                        if !(*class_start).is_null() {
                            **class_start |= CLASS_IS_ECLASS;
                            *class_start = core::ptr::null_mut();
                        }

                        **parsed_pattern = if *negate_class != 0 {
                            META_CLASS_EMPTY_NOT
                        } else {
                            META_CLASS_EMPTY
                        };
                        *parsed_pattern = (*parsed_pattern).add(1);

                        if *class_depth_m1 < 0 {
                            break 'chain ClassStep::EndClass;
                        }

                        *class_range_state = RANGE_NO;
                        *class_op_state = CLASS_OP_OPERAND;
                        break 'chain ClassStep::Next; /* goto CLASS_CONTINUE */
                    }

                    /* Enter a non-empty class. */
                    if !(*class_start).is_null() {
                        **class_start |= CLASS_IS_ECLASS;
                        *class_start = core::ptr::null_mut();
                    }

                    *class_start = *parsed_pattern;
                    **parsed_pattern = if *negate_class != 0 { META_CLASS_NOT } else { META_CLASS };
                    *parsed_pattern = (*parsed_pattern).add(1);
                    *class_range_state = RANGE_NO;
                    *class_op_state = CLASS_OP_EMPTY;
                    *class_mode_state = new_class_mode_state;
                    *class_depth_m1 += 1;
                    if *class_maxdepth_m1 < *class_depth_m1 {
                        *class_maxdepth_m1 = *class_depth_m1;
                    }
                    (*cb).class_op_used[*class_depth_m1 as usize] = 0;

                    /* Special start-of-class literal ']'. */
                    if *c == CHAR_RIGHT_SQUARE_BRACKET
                        && new_class_mode_state != CLASS_MODE_PERL_EXT
                    {
                        *class_range_state = RANGE_OK_LITERAL;
                        *class_op_state = CLASS_OP_OPERAND;
                        PARSED_LITERAL!(*c);
                        break 'chain ClassStep::Next; /* goto CLASS_CONTINUE */
                    }

                    break 'chain ClassStep::Again; /* continue (c already loaded) */
                }

                /* Check for end of the class. */
                if *c == CHAR_RIGHT_SQUARE_BRACKET
                    || (*c == CHAR_RIGHT_PARENTHESIS
                        && *class_mode_state == CLASS_MODE_PERL_EXT)
                {
                    if *class_mode_state == CLASS_MODE_PERL_EXT {
                        if *c == CHAR_RIGHT_SQUARE_BRACKET && *class_depth_m1 != 0 {
                            *ptr = ptr.sub(1);
                            return Err(ERR14);
                        }
                        if *c == CHAR_RIGHT_PARENTHESIS && *class_depth_m1 < 1 {
                            return Err(ERR22);
                        }
                    }

                    if *class_op_state == CLASS_OP_OPERATOR {
                        return Err(ERR110);
                    }

                    if *class_mode_state == CLASS_MODE_PERL_EXT
                        && *class_op_state == CLASS_OP_EMPTY
                    {
                        return Err(ERR114);
                    }

                    /* -] at the end of a class is a literal '-' */
                    if *class_range_state == RANGE_STARTED {
                        *(*parsed_pattern).offset(-1) = CHAR_MINUS;
                    }

                    **parsed_pattern = META_CLASS_END;
                    *parsed_pattern = (*parsed_pattern).add(1);

                    *class_depth_m1 -= 1;
                    if *class_depth_m1 < 0 {
                        if *class_mode_state == CLASS_MODE_PERL_EXT {
                            if *ptr >= ptrend || **ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                                return Err(ERR115);
                            }
                            *ptr = ptr.add(1);
                        }
                        break 'chain ClassStep::EndClass;
                    }

                    *class_range_state = RANGE_NO;
                    *class_op_state = CLASS_OP_OPERAND;
                    if *class_mode_state == CLASS_MODE_PERL_EXT_LEAF {
                        *class_mode_state = CLASS_MODE_PERL_EXT;
                    }
                    *class_start = core::ptr::null_mut();
                    break 'chain ClassStep::Next;
                }

                /* Perl set binary operator. */
                if *class_mode_state == CLASS_MODE_PERL_EXT
                    && (*c == CHAR_PLUS
                        || *c == CHAR_VERTICAL_LINE
                        || *c == CHAR_MINUS
                        || *c == CHAR_AMPERSAND
                        || *c == CHAR_CIRCUMFLEX_ACCENT)
                {
                    if *class_op_state != CLASS_OP_OPERAND {
                        return Err(ERR109);
                    }

                    if !(*class_start).is_null() {
                        **class_start |= CLASS_IS_ECLASS;
                        *class_start = core::ptr::null_mut();
                    }

                    **parsed_pattern = if *c == CHAR_PLUS {
                        META_ECLASS_OR
                    } else if *c == CHAR_VERTICAL_LINE {
                        META_ECLASS_OR
                    } else if *c == CHAR_MINUS {
                        META_ECLASS_SUB
                    } else if *c == CHAR_AMPERSAND {
                        META_ECLASS_AND
                    } else {
                        META_ECLASS_XOR
                    };
                    *parsed_pattern = (*parsed_pattern).add(1);
                    *class_range_state = RANGE_NO;
                    *class_op_state = CLASS_OP_OPERATOR;
                    break 'chain ClassStep::Next;
                }

                /* Perl set unary operator. */
                if *class_mode_state == CLASS_MODE_PERL_EXT && *c == CHAR_EXCLAMATION_MARK {
                    if *class_op_state == CLASS_OP_OPERAND {
                        return Err(ERR113);
                    }

                    if !(*class_start).is_null() {
                        **class_start |= CLASS_IS_ECLASS;
                        *class_start = core::ptr::null_mut();
                    }

                    **parsed_pattern = META_ECLASS_NOT;
                    *parsed_pattern = (*parsed_pattern).add(1);
                    *class_range_state = RANGE_NO;
                    *class_op_state = CLASS_OP_OPERATOR;
                    break 'chain ClassStep::Next;
                }

                /* UTS#18 set operator. */
                if *class_mode_state == CLASS_MODE_ALT_EXT
                    && (*c == CHAR_VERTICAL_LINE
                        || *c == CHAR_MINUS
                        || *c == CHAR_AMPERSAND
                        || *c == CHAR_TILDE)
                    && *ptr < ptrend
                    && **ptr as u32 == *c
                {
                    *ptr = ptr.add(1);

                    if *ptr < ptrend && **ptr as u32 == *c {
                        while *ptr < ptrend && **ptr as u32 == *c {
                            *ptr = ptr.add(1);
                        }
                        return Err(ERR108);
                    }

                    if *class_op_state != CLASS_OP_OPERAND {
                        return Err(ERR109);
                    }

                    if (*cb).class_op_used[*class_depth_m1 as usize] != 0
                        && (*cb).class_op_used[*class_depth_m1 as usize] != (*c as u8)
                    {
                        return Err(ERR111);
                    }

                    if !(*class_start).is_null() {
                        **class_start |= CLASS_IS_ECLASS;
                        *class_start = core::ptr::null_mut();
                    }

                    if *class_range_state == RANGE_STARTED {
                        *(*parsed_pattern).offset(-1) = CHAR_MINUS;
                    }

                    **parsed_pattern = if *c == CHAR_VERTICAL_LINE {
                        META_ECLASS_OR
                    } else if *c == CHAR_MINUS {
                        META_ECLASS_SUB
                    } else if *c == CHAR_AMPERSAND {
                        META_ECLASS_AND
                    } else {
                        META_ECLASS_XOR
                    };
                    *parsed_pattern = (*parsed_pattern).add(1);
                    *class_range_state = RANGE_NO;
                    *class_op_state = CLASS_OP_OPERATOR;
                    (*cb).class_op_used[*class_depth_m1 as usize] = *c as u8;
                    break 'chain ClassStep::Next;
                }

                /* Handle escapes in a class. */
                if *c == CHAR_BACKSLASH {
                    tempptr = *ptr;
                    let mut ec: c_int = 0;
                    let mut escape = check_escape(
                        ptr,
                        ptrend,
                        c,
                        &mut ec,
                        options,
                        xoptions,
                        (*cb).bracount,
                        TRUE,
                        cb,
                    );

                    if ec != 0 {
                        if (xoptions & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL) == 0
                            || *class_mode_state >= CLASS_MODE_PERL_EXT
                        {
                            return Err(ec);
                        }
                        *ptr = tempptr;
                        if *ptr >= ptrend {
                            *c = CHAR_BACKSLASH;
                        } else {
                            *c = getcharinctest(ptr, utf != 0);
                        }
                        escape = 0;
                    }

                    /* Switch on escape. */
                    if escape == 0 {
                        char_is_literal = FALSE;
                        break 'chain ClassStep::Literal;
                    } else if escape == ESC_b {
                        *c = CHAR_BS;
                        char_is_literal = FALSE;
                        break 'chain ClassStep::Literal;
                    } else if escape == ESC_k {
                        *c = CHAR_k;
                        char_is_literal = FALSE;
                        break 'chain ClassStep::Literal;
                    } else if escape == ESC_Q {
                        *inescq = TRUE;
                        break 'chain ClassStep::Next; /* goto CLASS_CONTINUE */
                    } else if escape == ESC_E {
                        break 'chain ClassStep::Next; /* goto CLASS_CONTINUE */
                    } else if escape == ESC_B || escape == ESC_R || escape == ESC_X {
                        return Err(ERR7);
                    } else if escape == ESC_N {
                        return Err(ERR71);
                    } else if escape == ESC_H
                        || escape == ESC_h
                        || escape == ESC_V
                        || escape == ESC_v
                    {
                        **parsed_pattern = META_ESCAPE + escape as u32;
                        *parsed_pattern = (*parsed_pattern).add(1);
                    } else if escape == ESC_d
                        || escape == ESC_D
                        || escape == ESC_s
                        || escape == ESC_S
                        || escape == ESC_w
                        || escape == ESC_W
                    {
                        *parsed_pattern = handle_escdsw(escape, *parsed_pattern, options, xoptions);
                    } else if escape == ESC_P || escape == ESC_p {
                        let mut negated: BOOL = FALSE;
                        let mut ptype: u16 = 0;
                        let mut pdata: u16 = 0;
                        let mut ec2: c_int = 0;
                        if get_ucp(ptr, utf, &mut negated, &mut ptype, &mut pdata, &mut ec2, cb)
                            == FALSE
                        {
                            return Err(ec2);
                        }

                        if (options & PCRE2_CASELESS) != 0
                            && ptype as u32 == PT_PC
                            && (pdata as u32 == ucp_Lu
                                || pdata as u32 == ucp_Ll
                                || pdata as u32 == ucp_Lt)
                        {
                            ptype = PT_LAMP as u16;
                            pdata = 0;
                        }

                        if negated != 0 {
                            escape = if escape == ESC_P { ESC_p } else { ESC_P };
                        }
                        **parsed_pattern = META_ESCAPE + escape as u32;
                        *parsed_pattern = (*parsed_pattern).add(1);
                        **parsed_pattern = ((ptype as u32) << 16) | pdata as u32;
                        *parsed_pattern = (*parsed_pattern).add(1);
                    } else {
                        /* ESC_A, ESC_Z, ESC_z, ESC_G, ESC_K, ESC_C and the
                        unreachable default. */
                        return Err(ERR7);
                    }

                    /* The break-cases above describe a set of characters; none
                    may start a range. */
                    if *class_range_state == RANGE_STARTED {
                        return Err(ERR50);
                    }

                    if *class_range_state == RANGE_FORBID_STARTED {
                        *ptr = *class_range_forbid_ptr;
                        return Err(ERR50);
                    }

                    if *class_op_state == CLASS_OP_OPERAND
                        && *class_mode_state == CLASS_MODE_PERL_EXT
                    {
                        return Err(ERR113);
                    }

                    *class_range_state = RANGE_FORBID_NO;
                    *class_op_state = CLASS_OP_OPERAND;
                    break 'chain ClassStep::Next;
                }

                /* Forbid unescaped literals and '-' in Perl extended class. */
                if *class_mode_state == CLASS_MODE_PERL_EXT {
                    return Err(ERR116);
                }

                /* Handle potential start of range. */
                if *c == CHAR_MINUS && *class_range_state >= RANGE_OK_ESCAPED {
                    **parsed_pattern = if *class_range_state == RANGE_OK_LITERAL {
                        META_RANGE_LITERAL
                    } else {
                        META_RANGE_ESCAPED
                    };
                    *parsed_pattern = (*parsed_pattern).add(1);
                    *class_range_state = RANGE_STARTED;
                    break 'chain ClassStep::Next;
                }

                /* Handle forbidden start of range. */
                if *c == CHAR_MINUS && *class_range_state == RANGE_FORBID_NO {
                    **parsed_pattern = CHAR_MINUS;
                    *parsed_pattern = (*parsed_pattern).add(1);
                    *class_range_state = RANGE_FORBID_STARTED;
                    *class_range_forbid_ptr = *ptr;
                    break 'chain ClassStep::Next;
                }

                /* Handle a literal character (falls through to CLASS_LITERAL). */
                break 'chain ClassStep::Literal;
            }; /* 'chain */

            /* Handle the ClassStep result. */
            match step {
                ClassStep::EndClass => return Ok(()),
                ClassStep::Again => continue,
                ClassStep::Literal => {
                    /* CLASS_LITERAL */
                    if *class_op_state == CLASS_OP_OPERAND
                        && *class_mode_state == CLASS_MODE_PERL_EXT
                    {
                        return Err(ERR113);
                    }

                    if *class_range_state == RANGE_STARTED {
                        if *c == *(*parsed_pattern).offset(-2) {
                            *parsed_pattern = (*parsed_pattern).sub(1);
                        } else if *(*parsed_pattern).offset(-2) > *c {
                            return Err(ERR8);
                        } else {
                            if char_is_literal == FALSE
                                && *(*parsed_pattern).offset(-1) == META_RANGE_LITERAL
                            {
                                *(*parsed_pattern).offset(-1) = META_RANGE_ESCAPED;
                            }
                            PARSED_LITERAL!(*c);
                        }
                        *class_range_state = RANGE_NO;
                        *class_op_state = CLASS_OP_OPERAND;
                    } else if *class_range_state == RANGE_FORBID_STARTED {
                        *ptr = *class_range_forbid_ptr;
                        return Err(ERR50);
                    } else {
                        *class_range_state = if char_is_literal != FALSE {
                            RANGE_OK_LITERAL
                        } else {
                            RANGE_OK_ESCAPED
                        };
                        *class_op_state = CLASS_OP_OPERAND;
                        PARSED_LITERAL!(*c);
                    }
                    /* falls to CLASS_CONTINUE */
                }
                ClassStep::Next => { /* falls to CLASS_CONTINUE */ }
            }

            /* CLASS_CONTINUE */
            if *ptr >= ptrend {
                if *class_mode_state == CLASS_MODE_PERL_EXT && *class_depth_m1 > 0 {
                    return Err(ERR14);
                }
                if *class_mode_state == CLASS_MODE_ALT_EXT
                    && *class_depth_m1 == 0
                    && *class_maxdepth_m1 == 1
                {
                    return Err(ERR112);
                } else {
                    return Err(ERR6);
                }
            }
            *c = getcharinctest(ptr, utf != 0);
        } /* End of class-processing loop */
    }
}

/* The C literal `STR_Q STR_BACKSLASH STR_E` -- "Q\\E". */
#[inline]
fn class_qbe_str() -> [u8; 3] {
    [CHAR_Q as u8, CHAR_BACKSLASH as u8, CHAR_E as u8]
}

/* Result of parsing a `(` item. */
enum ParenResult {
    /* C `break` -- done with this parenthesis. */
    Break,
    /* `(?[...]` -- caller should run the extended-class parser. */
    FromPerlClass,
    /* `goto FAILED` with the given error code (ptr already adjusted). */
    Failed(c_int),
}

/* Internal goto targets shared across the `(` handling. */
enum PGoto {
    AtomicGroup,
    PositiveLookAhead,
    PositiveNonatomicLookAhead,
    NegativeLookAhead,
    PostAssertion,
    PostLookbehind,
    DefineName,
    RecurseByName,
    RecursionByNumber,
    SetRecursion,
    ReadRecursionArguments,
    Unclosed,
}

/* Handle a `(` item. Corresponds to the CHAR_LEFT_PARENTHESIS case. */
unsafe fn parse_paren(
    c: &mut u32,
    ptr: &mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    options: &mut u32,
    xoptions: &mut u32,
    parsed_pattern: &mut *mut u32,
    _this_parsed_item: &mut *mut u32,
    _prev_parsed_item: &mut *mut u32,
    previous_callout: &mut *mut u32,
    verbstartptr: &mut *mut u32,
    verblengthptr: &mut *mut u32,
    verbnamestart: &mut PCRE2_SPTR,
    add_after_mark: &mut u32,
    nest_depth: &mut u16,
    class_mode_state: &mut u32,
    after_manual_callout: &mut c_int,
    expect_cond_assert: &mut c_int,
    prev_expect_cond_assert: c_int,
    _inescq: &mut BOOL,
    inverbname: &mut BOOL,
    utf: BOOL,
    _auto_callout: BOOL,
    okquantifier: &mut BOOL,
    is_dupname: &mut BOOL,
    hash: &mut u16,
    ng: &mut *mut named_group,
    name: &mut PCRE2_SPTR,
    namelen: &mut u32,
    top_nest: &mut *mut nest_save,
    end_nests: *mut nest_save,
    has_lookbehind: *mut BOOL,
    cb: *mut compile_block,
) -> ParenResult {
    unsafe {
        let mut errorcode: c_int = 0;
        let mut i: c_int = 0;
        let mut offset: PCRE2_SIZE = 0;
        let mut terminator: u32 = 0;

        macro_rules! fail {
            ($e:expr) => {{
                return ParenResult::Failed($e);
            }};
        }
        /* FAILED_FORWARD: advance ptr (and over a UTF char) then fail. */
        macro_rules! fail_forward {
            ($e:expr) => {{
                *ptr = ptr.add(1);
                if utf != 0 {
                    forwardchartest(ptr, ptrend);
                }
                return ParenResult::Failed($e);
            }};
        }

        if *ptr >= ptrend {
            /* goto UNCLOSED_PARENTHESIS */
            return ParenResult::Failed(ERR14);
        }

        /* The `pgoto` variable, when Some, means "jump to this label". We run a
        dispatch loop: first the entry code (which may set pgoto), then the loop
        services the label. */
        let mut pgoto: Option<PGoto> = None;

        /* ---- Entry: not followed by '?' ---- */
        if **ptr as u32 != CHAR_QUESTION_MARK {
            let mut vn: *const u8;

            if **ptr as u32 != CHAR_ASTERISK {
                *nest_depth += 1;
                if (*options & PCRE2_NO_AUTO_CAPTURE) == 0 {
                    if (*cb).bracount >= MAX_GROUP_NUMBER {
                        fail!(ERR97);
                    }
                    (*cb).bracount += 1;
                    **parsed_pattern = META_CAPTURE | (*cb).bracount;
                    *parsed_pattern = (*parsed_pattern).add(1);
                } else {
                    **parsed_pattern = META_NOCAPTURE;
                    *parsed_pattern = (*parsed_pattern).add(1);
                }
                /* Fall to the very end (break). */
                return ParenResult::Break;
            } else if ptrend.offset_from(*ptr) <= 1 || {
                *c = *ptr.add(1) as u32;
                *c == CHAR_RIGHT_PARENTHESIS
            } {
                /* (* at end or (*) -- do nothing, gives bad-quantifier later. */
                return ParenResult::Break;
            } else if chmax_255(*c) && ((*(*cb).ctypes.add(*c as usize)) & ctype_lcletter) != 0 {
                /* ---- Alpha assertions ---- */
                let meta: u32;

                vn = alasnames.as_ptr();
                if read_name(
                    ptr, ptrend, utf, 0, &mut offset, name, namelen, &mut errorcode, cb,
                ) == FALSE
                {
                    fail!(errorcode);
                }
                if *ptr >= ptrend {
                    return ParenResult::Failed(ERR14); /* UNCLOSED_PARENTHESIS */
                }
                if **ptr as u32 != CHAR_COLON {
                    fail_forward!(ERR95);
                }

                /* Scan the table of alpha assertion names */
                i = 0;
                while i < alascount {
                    if *namelen == alasmeta[i as usize].len
                        && crate::string_utils::strncmp_c8(
                            *name,
                            vn as *const c_char,
                            *namelen as usize,
                        ) == 0
                    {
                        break;
                    }
                    vn = vn.add(alasmeta[i as usize].len as usize + 1);
                    i += 1;
                }

                if i >= alascount {
                    fail!(ERR95);
                }

                meta = alasmeta[i as usize].meta;
                if prev_expect_cond_assert > 0
                    && (meta < META_LOOKAHEAD || meta > META_LOOKBEHINDNOT)
                {
                    fail!(ERR28);
                }

                /* Jump to the traditional symbolic handlers. */
                match meta {
                    META_ATOMIC => pgoto = Some(PGoto::AtomicGroup),
                    META_LOOKAHEAD => pgoto = Some(PGoto::PositiveLookAhead),
                    META_LOOKAHEAD_NA => pgoto = Some(PGoto::PositiveNonatomicLookAhead),
                    META_LOOKAHEADNOT => pgoto = Some(PGoto::NegativeLookAhead),
                    META_SCS => {
                        *ptr = ptr.add(1);
                        **parsed_pattern = META_SCS;
                        *parsed_pattern = (*parsed_pattern).add(1);

                        *parsed_pattern = parse_capture_list(
                            ptr,
                            ptrend,
                            utf,
                            *parsed_pattern,
                            0,
                            &mut errorcode,
                            cb,
                        );
                        if (*parsed_pattern).is_null() {
                            fail!(errorcode);
                        }
                        pgoto = Some(PGoto::PostAssertion);
                    }
                    META_LOOKBEHIND | META_LOOKBEHINDNOT | META_LOOKBEHIND_NA => {
                        **parsed_pattern = meta;
                        *parsed_pattern = (*parsed_pattern).add(1);
                        *ptr = ptr.sub(1);
                        pgoto = Some(PGoto::PostLookbehind);
                    }
                    META_SCRIPT_RUN | META_ATOMIC_SCRIPT_RUN => {
                        **parsed_pattern = META_SCRIPT_RUN;
                        *parsed_pattern = (*parsed_pattern).add(1);
                        *nest_depth += 1;
                        *ptr = ptr.add(1);
                        if meta == META_ATOMIC_SCRIPT_RUN {
                            **parsed_pattern = META_ATOMIC;
                            *parsed_pattern = (*parsed_pattern).add(1);
                            if (*top_nest).is_null() {
                                *top_nest = (*cb).start_workspace as *mut nest_save;
                            } else {
                                *top_nest = (*top_nest).add(1);
                                if *top_nest >= end_nests {
                                    fail!(ERR84);
                                }
                            }
                            (**top_nest).nest_depth = *nest_depth;
                            (**top_nest).flags = NSF_ATOMICSR;
                            (**top_nest).options = *options & PARSE_TRACKED_OPTIONS;
                            (**top_nest).xoptions = *xoptions & PARSE_TRACKED_EXTRA_OPTIONS;
                        }
                        return ParenResult::Break;
                    }
                    _ => {
                        fail!(ERR89); /* Unknown code; should never occur. */
                    }
                }
            } else {
                /* ---- Handle (*VERB) and (*VERB:NAME) ---- */
                let mut vn2 = verbnames.as_ptr();
                if read_name(
                    ptr, ptrend, utf, 0, &mut offset, name, namelen, &mut errorcode, cb,
                ) == FALSE
                {
                    fail!(errorcode);
                }
                if *ptr >= ptrend
                    || (**ptr as u32 != CHAR_COLON && **ptr as u32 != CHAR_RIGHT_PARENTHESIS)
                {
                    fail!(ERR60);
                }

                i = 0;
                while i < verbcount {
                    if *namelen == verbs[i as usize].len
                        && crate::string_utils::strncmp_c8(
                            *name,
                            vn2 as *const c_char,
                            *namelen as usize,
                        ) == 0
                    {
                        break;
                    }
                    vn2 = vn2.add(verbs[i as usize].len as usize + 1);
                    i += 1;
                }

                if i >= verbcount {
                    fail!(ERR60);
                }

                /* An empty argument is treated as no argument. */
                if **ptr as u32 == CHAR_COLON
                    && ptr.add(1) < ptrend
                    && *ptr.add(1) as u32 == CHAR_RIGHT_PARENTHESIS
                {
                    *ptr = ptr.add(1);
                }

                if verbs[i as usize].has_arg > 0 && **ptr as u32 != CHAR_COLON {
                    fail!(ERR66);
                }

                *verbstartptr = *parsed_pattern;
                *okquantifier = (verbs[i as usize].meta == META_ACCEPT) as BOOL;

                let was_colon = **ptr as u32 == CHAR_COLON;
                *ptr = ptr.add(1); /* Skip past : or ) */
                if was_colon {
                    if verbs[i as usize].has_arg < 0 {
                        *add_after_mark = verbs[i as usize].meta;
                        **parsed_pattern = META_MARK;
                        *parsed_pattern = (*parsed_pattern).add(1);
                    } else {
                        **parsed_pattern = verbs[i as usize].meta
                            + (if verbs[i as usize].meta != META_MARK {
                                0x00010000u32
                            } else {
                                0
                            });
                        *parsed_pattern = (*parsed_pattern).add(1);
                    }

                    *verblengthptr = *parsed_pattern;
                    *parsed_pattern = (*parsed_pattern).add(1);
                    *verbnamestart = *ptr;
                    *inverbname = TRUE;
                } else {
                    **parsed_pattern = verbs[i as usize].meta;
                    *parsed_pattern = (*parsed_pattern).add(1);
                }
                return ParenResult::Break;
            }
            /* If we set a pgoto above (alpha assertions), fall into the dispatch
            loop below. Otherwise we've already returned. */
        } else {
            /* ---- Items starting (? ---- */
            *ptr = ptr.add(1);
            if *ptr >= ptrend {
                return ParenResult::Failed(ERR14); /* UNCLOSED_PARENTHESIS */
            }

            match parse_paren_question(
                c,
                ptr,
                ptrend,
                options,
                xoptions,
                parsed_pattern,
                previous_callout,
                add_after_mark,
                nest_depth,
                class_mode_state,
                after_manual_callout,
                expect_cond_assert,
                prev_expect_cond_assert,
                utf,
                okquantifier,
                is_dupname,
                hash,
                ng,
                name,
                namelen,
                top_nest,
                end_nests,
                has_lookbehind,
                cb,
                &mut i,
                &mut offset,
                &mut terminator,
                &mut pgoto,
            ) {
                QResult::Break => return ParenResult::Break,
                QResult::FromPerlClass => return ParenResult::FromPerlClass,
                QResult::Goto => { /* pgoto set; fall into dispatch */ }
                QResult::Failed(e) => return ParenResult::Failed(e),
            }
        }

        /* ---- Shared goto-label dispatch ---- */
        loop {
            let target = match pgoto.take() {
                Some(t) => t,
                None => return ParenResult::Break,
            };
            match target {
                PGoto::AtomicGroup => {
                    **parsed_pattern = META_ATOMIC;
                    *parsed_pattern = (*parsed_pattern).add(1);
                    *nest_depth += 1;
                    *ptr = ptr.add(1);
                    return ParenResult::Break;
                }
                PGoto::PositiveLookAhead => {
                    **parsed_pattern = META_LOOKAHEAD;
                    *parsed_pattern = (*parsed_pattern).add(1);
                    *ptr = ptr.add(1);
                    pgoto = Some(PGoto::PostAssertion);
                }
                PGoto::PositiveNonatomicLookAhead => {
                    **parsed_pattern = META_LOOKAHEAD_NA;
                    *parsed_pattern = (*parsed_pattern).add(1);
                    *ptr = ptr.add(1);
                    pgoto = Some(PGoto::PostAssertion);
                }
                PGoto::NegativeLookAhead => {
                    **parsed_pattern = META_LOOKAHEADNOT;
                    *parsed_pattern = (*parsed_pattern).add(1);
                    *ptr = ptr.add(1);
                    pgoto = Some(PGoto::PostAssertion);
                }
                PGoto::PostLookbehind => {
                    *has_lookbehind = TRUE;
                    offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE - 2;
                    putoffset(offset, parsed_pattern);
                    *ptr = ptr.add(2);
                    /* Fall through to POST_ASSERTION */
                    pgoto = Some(PGoto::PostAssertion);
                }
                PGoto::PostAssertion => {
                    *nest_depth += 1;
                    if prev_expect_cond_assert > 0 {
                        if (*top_nest).is_null() {
                            *top_nest = (*cb).start_workspace as *mut nest_save;
                        } else {
                            *top_nest = (*top_nest).add(1);
                            if *top_nest >= end_nests {
                                fail!(ERR84);
                            }
                        }
                        (**top_nest).nest_depth = *nest_depth;
                        (**top_nest).flags = NSF_CONDASSERT;
                        (**top_nest).options = *options & PARSE_TRACKED_OPTIONS;
                        (**top_nest).xoptions = *xoptions & PARSE_TRACKED_EXTRA_OPTIONS;
                    }
                    return ParenResult::Break;
                }
                PGoto::DefineName => {
                    match paren_define_name(
                        ptr, ptrend, options, parsed_pattern, nest_depth, utf, is_dupname,
                        hash, ng, name, namelen, terminator, cb,
                    ) {
                        Ok(()) => return ParenResult::Break,
                        Err(e) => return ParenResult::Failed(e),
                    }
                }
                PGoto::RecurseByName => {
                    if read_name(
                        ptr, ptrend, utf, 0, &mut offset, name, namelen, &mut errorcode, cb,
                    ) == FALSE
                    {
                        fail!(errorcode);
                    }
                    **parsed_pattern = META_RECURSE_BYNAME;
                    *parsed_pattern = (*parsed_pattern).add(1);
                    **parsed_pattern = *namelen;
                    *parsed_pattern = (*parsed_pattern).add(1);
                    terminator = CHAR_NUL;
                    pgoto = Some(PGoto::ReadRecursionArguments);
                }
                PGoto::RecursionByNumber => {
                    if read_number(
                        ptr,
                        ptrend,
                        if IS_DIGIT(**ptr as u32) { -1 } else { (*cb).bracount as i32 },
                        MAX_GROUP_NUMBER,
                        ERR61 as u32,
                        &mut i,
                        &mut errorcode,
                    ) == FALSE
                    {
                        fail!(errorcode);
                    }
                    terminator = CHAR_NUL;
                    pgoto = Some(PGoto::SetRecursion);
                }
                PGoto::SetRecursion => {
                    **parsed_pattern = META_RECURSE | (i as u32);
                    *parsed_pattern = (*parsed_pattern).add(1);
                    offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                    pgoto = Some(PGoto::ReadRecursionArguments);
                }
                PGoto::ReadRecursionArguments => {
                    putoffset(offset, parsed_pattern);
                    *okquantifier = TRUE;

                    if terminator != CHAR_NUL {
                        return ParenResult::Break;
                    }

                    if *ptr < ptrend && **ptr as u32 == CHAR_LEFT_PARENTHESIS {
                        *parsed_pattern = parse_capture_list(
                            ptr,
                            ptrend,
                            utf,
                            *parsed_pattern,
                            offset,
                            &mut errorcode,
                            cb,
                        );
                        if (*parsed_pattern).is_null() {
                            fail!(errorcode);
                        }
                    }

                    if *ptr >= ptrend || **ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                        return ParenResult::Failed(ERR14); /* UNCLOSED_PARENTHESIS */
                    }
                    *ptr = ptr.add(1);
                    return ParenResult::Break;
                }
                PGoto::Unclosed => {
                    return ParenResult::Failed(ERR14);
                }
            }
        }
    }
}

/* The DEFINE_NAME label: define a named capturing group. `terminator` holds
the name terminator. Returns Ok for the C `break`, Err(code) for goto FAILED. */
unsafe fn paren_define_name(
    ptr: &mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    options: &mut u32,
    parsed_pattern: &mut *mut u32,
    nest_depth: &mut u16,
    utf: BOOL,
    is_dupname: &mut BOOL,
    hash_out: &mut u16,
    ng_out: &mut *mut named_group,
    name_out: &mut PCRE2_SPTR,
    namelen_out: &mut u32,
    terminator: u32,
    cb: *mut compile_block,
) -> Result<(), c_int> {
    unsafe {
        let mut errorcode: c_int = 0;
        let mut offset: PCRE2_SIZE = 0;
        let mut name: PCRE2_SPTR = *name_out;
        let mut namelen: u32 = *namelen_out;

        if read_name(
            ptr,
            ptrend,
            utf,
            terminator,
            &mut offset,
            &mut name,
            &mut namelen,
            &mut errorcode,
            cb,
        ) == FALSE
        {
            *name_out = name;
            *namelen_out = namelen;
            return Err(errorcode);
        }

        if (*cb).bracount >= MAX_GROUP_NUMBER {
            return Err(ERR97);
        }
        (*cb).bracount += 1;
        **parsed_pattern = META_CAPTURE | (*cb).bracount;
        *parsed_pattern = (*parsed_pattern).add(1);
        *nest_depth += 1;

        if (*cb).names_found >= MAX_NAME_COUNT {
            return Err(ERR49);
        }

        if namelen + IMM2_SIZE as u32 + 1 > (*cb).name_entry_size as u32 {
            (*cb).name_entry_size = (namelen + IMM2_SIZE as u32 + 1) as u16;
        }

        *is_dupname = FALSE;
        let mut hash = crate::compile_cgroup::get_hash_from_name(name, namelen);
        let mut ng = (*cb).named_groups;
        let mut i: c_int = 0;
        let mut discard = false;
        while i < (*cb).names_found as c_int {
            if namelen == (*ng).length as u32
                && hash == named_group_get_hash(ng)
                && crate::string_utils::strncmp(name, (*ng).name, namelen as PCRE2_SIZE) == 0
            {
                if (*ng).number == (*cb).bracount {
                    discard = true;
                    break;
                }
                if (*options & PCRE2_DUPNAMES) == 0 {
                    return Err(ERR43);
                }

                (*ng).hash_dup |= NAMED_GROUP_IS_DUPNAME;
                *is_dupname = TRUE;
                (*cb).dupnames = TRUE;

                name = (*ng).name;
                namelen = 0;

                /* Even duplicated names may refer to the same capture index. */
                while i < (*cb).names_found as c_int {
                    if (*ng).name == name && (*ng).number == (*cb).bracount {
                        break;
                    }
                    i += 1;
                    ng = ng.add(1);
                }
                discard = i < (*cb).names_found as c_int;
                break;
            } else if (*ng).number == (*cb).bracount {
                return Err(ERR65);
            }
            i += 1;
            ng = ng.add(1);
        }

        *hash_out = hash;
        *ng_out = ng;
        *name_out = name;
        *namelen_out = namelen;

        /* Ignore duplicate with same number. */
        if discard {
            return Ok(());
        }

        /* Increase the list size if necessary. */
        if (*cb).names_found as u32 >= (*cb).named_group_list_size {
            let newsize = (*cb).named_group_list_size * 2;
            let newspace = ((*(*cb).cx).memctl.malloc.unwrap())(
                newsize as usize * core::mem::size_of::<named_group>(),
                (*(*cb).cx).memctl.memory_data,
            ) as *mut named_group;
            if newspace.is_null() {
                return Err(ERR21);
            }

            memcpy(
                newspace,
                (*cb).named_groups,
                (*cb).named_group_list_size as usize,
            );
            if (*cb).named_group_list_size > NAMED_GROUP_LIST_SIZE as u32 {
                ((*(*cb).cx).memctl.free.unwrap())(
                    (*cb).named_groups as *mut core::ffi::c_void,
                    (*(*cb).cx).memctl.memory_data,
                );
            }
            (*cb).named_groups = newspace;
            (*cb).named_group_list_size = newsize;
        }

        if *is_dupname != FALSE {
            hash |= NAMED_GROUP_IS_DUPNAME;
        }

        let nf = (*cb).names_found as usize;
        (*(*cb).named_groups.add(nf)).name = name;
        (*(*cb).named_groups.add(nf)).length = namelen as u16;
        (*(*cb).named_groups.add(nf)).number = (*cb).bracount;
        (*(*cb).named_groups.add(nf)).hash_dup = hash;
        (*cb).names_found += 1;
        Ok(())
    }
}

/* Result of the (? sub-switch. */
enum QResult {
    Break,
    FromPerlClass,
    /* A shared goto label was requested; the caller reads `*pgoto`. */
    Goto,
    Failed(c_int),
}

/* Handle the items starting with `(?`. On entry `*ptr` points at the character
after `?`. */
unsafe fn parse_paren_question(
    c: &mut u32,
    ptr: &mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    options: &mut u32,
    xoptions: &mut u32,
    parsed_pattern: &mut *mut u32,
    previous_callout: &mut *mut u32,
    _add_after_mark: &mut u32,
    nest_depth: &mut u16,
    class_mode_state: &mut u32,
    after_manual_callout: &mut c_int,
    expect_cond_assert: &mut c_int,
    prev_expect_cond_assert: c_int,
    utf: BOOL,
    okquantifier: &mut BOOL,
    _is_dupname: &mut BOOL,
    _hash: &mut u16,
    _ng: &mut *mut named_group,
    name: &mut PCRE2_SPTR,
    namelen: &mut u32,
    top_nest: &mut *mut nest_save,
    end_nests: *mut nest_save,
    has_lookbehind: *mut BOOL,
    cb: *mut compile_block,
    i: &mut c_int,
    offset: &mut PCRE2_SIZE,
    terminator: &mut u32,
    pgoto: &mut Option<PGoto>,
) -> QResult {
    unsafe {
        let mut errorcode: c_int = 0;

        macro_rules! fail {
            ($e:expr) => {{
                return QResult::Failed($e);
            }};
        }
        macro_rules! fail_forward {
            ($e:expr) => {{
                *ptr = ptr.add(1);
                if utf != 0 {
                    forwardchartest(ptr, ptrend);
                }
                return QResult::Failed($e);
            }};
        }
        macro_rules! goto {
            ($g:expr) => {{
                *pgoto = Some($g);
                return QResult::Goto;
            }};
        }

        match **ptr as u32 {
            /* ---- Python syntax support ---- */
            CHAR_P => {
                *ptr = ptr.add(1);
                if *ptr >= ptrend {
                    fail!(ERR14);
                }

                if **ptr as u32 == CHAR_LESS_THAN_SIGN {
                    *terminator = CHAR_GREATER_THAN_SIGN;
                    goto!(PGoto::DefineName);
                }

                if **ptr as u32 == CHAR_GREATER_THAN_SIGN {
                    goto!(PGoto::RecurseByName);
                }

                if **ptr as u32 != CHAR_EQUALS_SIGN {
                    fail_forward!(ERR41);
                }
                if read_name(
                    ptr,
                    ptrend,
                    utf,
                    CHAR_RIGHT_PARENTHESIS,
                    offset,
                    name,
                    namelen,
                    &mut errorcode,
                    cb,
                ) == FALSE
                {
                    fail!(errorcode);
                }
                **parsed_pattern = META_BACKREF_BYNAME;
                *parsed_pattern = (*parsed_pattern).add(1);
                **parsed_pattern = *namelen;
                *parsed_pattern = (*parsed_pattern).add(1);
                putoffset(*offset, parsed_pattern);
                *okquantifier = TRUE;
                QResult::Break
            }

            /* ---- Recursion/subroutine calls by number ---- */
            CHAR_R => {
                *i = 0; /* (?R) == (?R0) */
                *ptr = ptr.add(1);
                if *ptr >= ptrend
                    || (**ptr as u32 != CHAR_RIGHT_PARENTHESIS
                        && **ptr as u32 != CHAR_LEFT_PARENTHESIS)
                {
                    fail!(ERR58);
                }
                *terminator = CHAR_NUL;
                goto!(PGoto::SetRecursion);
            }

            CHAR_PLUS => {
                if ptr.add(1) >= ptrend {
                    *ptr = ptr.add(1);
                    fail!(ERR14);
                }
                if !IS_DIGIT(*ptr.add(1) as u32) {
                    *ptr = ptr.add(1);
                    fail_forward!(ERR29);
                }
                goto!(PGoto::RecursionByNumber);
            }

            CHAR_0 | CHAR_1 | CHAR_2 | CHAR_3 | CHAR_4 | CHAR_5 | CHAR_6 | CHAR_7 | CHAR_8
            | CHAR_9 => {
                goto!(PGoto::RecursionByNumber);
            }

            /* ---- Recursion/subroutine calls by name ---- */
            CHAR_AMPERSAND => {
                goto!(PGoto::RecurseByName);
            }

            /* ---- Callout ---- */
            CHAR_C => {
                match parse_paren_callout(
                    ptr,
                    ptrend,
                    *options,
                    xoptions,
                    parsed_pattern,
                    previous_callout,
                    after_manual_callout,
                    expect_cond_assert,
                    prev_expect_cond_assert,
                    utf,
                    cb,
                ) {
                    Ok(()) => QResult::Break,
                    Err(e) => QResult::Failed(e),
                }
            }

            /* ---- Conditional group ---- */
            CHAR_LEFT_PARENTHESIS => {
                match parse_paren_cond(
                    ptr,
                    ptrend,
                    options,
                    parsed_pattern,
                    nest_depth,
                    expect_cond_assert,
                    utf,
                    name,
                    namelen,
                    cb,
                    i,
                    offset,
                    terminator,
                ) {
                    Ok(()) => QResult::Break,
                    Err(e) => QResult::Failed(e),
                }
            }

            /* ---- Atomic group ---- */
            CHAR_GREATER_THAN_SIGN => {
                goto!(PGoto::AtomicGroup);
            }

            /* ---- Lookahead assertions ---- */
            CHAR_EQUALS_SIGN => {
                goto!(PGoto::PositiveLookAhead);
            }

            CHAR_ASTERISK => {
                goto!(PGoto::PositiveNonatomicLookAhead);
            }

            CHAR_EXCLAMATION_MARK => {
                goto!(PGoto::NegativeLookAhead);
            }

            /* ---- Lookbehind assertions / named group ---- */
            CHAR_LESS_THAN_SIGN => {
                if ptrend.offset_from(*ptr) <= 1
                    || (*ptr.add(1) as u32 != CHAR_EQUALS_SIGN
                        && *ptr.add(1) as u32 != CHAR_EXCLAMATION_MARK
                        && *ptr.add(1) as u32 != CHAR_ASTERISK)
                {
                    *terminator = CHAR_GREATER_THAN_SIGN;
                    goto!(PGoto::DefineName);
                }
                **parsed_pattern = if *ptr.add(1) as u32 == CHAR_EQUALS_SIGN {
                    META_LOOKBEHIND
                } else if *ptr.add(1) as u32 == CHAR_EXCLAMATION_MARK {
                    META_LOOKBEHINDNOT
                } else {
                    META_LOOKBEHIND_NA
                };
                *parsed_pattern = (*parsed_pattern).add(1);
                goto!(PGoto::PostLookbehind);
            }

            /* ---- Define a named group with '...' ---- */
            CHAR_APOSTROPHE => {
                *terminator = CHAR_APOSTROPHE;
                goto!(PGoto::DefineName);
            }

            /* ---- Perl extended character class ---- */
            CHAR_LEFT_SQUARE_BRACKET => {
                *class_mode_state = CLASS_MODE_PERL_EXT;
                *c = **ptr as u32;
                *ptr = ptr.add(1);
                QResult::FromPerlClass
            }

            /* default */
            _ => {
                match parse_paren_default(
                    ptr,
                    ptrend,
                    options,
                    xoptions,
                    parsed_pattern,
                    nest_depth,
                    utf,
                    top_nest,
                    end_nests,
                    cb,
                    pgoto,
                ) {
                    Ok(true) => QResult::Goto,
                    Ok(false) => QResult::Break,
                    Err(e) => QResult::Failed(e),
                }
            }
        }
    }
}

/* The (?C...) callout case. */
unsafe fn parse_paren_callout(
    ptr: &mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    options: u32,
    xoptions: &mut u32,
    parsed_pattern: &mut *mut u32,
    previous_callout: &mut *mut u32,
    after_manual_callout: &mut c_int,
    expect_cond_assert: &mut c_int,
    prev_expect_cond_assert: c_int,
    utf: BOOL,
    cb: *mut compile_block,
) -> Result<(), c_int> {
    unsafe {
        let mut i: c_int;
        let mut offset: PCRE2_SIZE;
        let mut delimiter: u32;

        macro_rules! fail_forward {
            ($e:expr) => {{
                *ptr = ptr.add(1);
                if utf != 0 {
                    forwardchartest(ptr, ptrend);
                }
                return Err($e);
            }};
        }

        if (*xoptions & PCRE2_EXTRA_NEVER_CALLOUT) != 0 {
            *ptr = ptr.add(1);
            return Err(ERR103);
        }

        *ptr = ptr.add(1);
        if *ptr >= ptrend {
            return Err(ERR14); /* UNCLOSED_PARENTHESIS */
        }

        *expect_cond_assert = prev_expect_cond_assert - 1;

        /* Abolish a preceding automatic callout. */
        if !(*previous_callout).is_null()
            && (options & PCRE2_AUTO_CALLOUT) != 0
            && *previous_callout == (*parsed_pattern).sub(4)
            && *(*parsed_pattern).offset(-1) == 255
        {
            *parsed_pattern = *previous_callout;
        }

        *previous_callout = *parsed_pattern;
        *after_manual_callout = 1;

        /* Handle a string argument. */
        if **ptr as u32 != CHAR_RIGHT_PARENTHESIS && !IS_DIGIT(**ptr as u32) {
            let calloutlength: PCRE2_SIZE;
            let startptr = *ptr;

            delimiter = 0;
            i = 0;
            while CALLOUT_START_DELIMS[i as usize] != 0 {
                if **ptr as u32 == CALLOUT_START_DELIMS[i as usize] {
                    delimiter = CALLOUT_END_DELIMS[i as usize];
                    break;
                }
                i += 1;
            }
            if delimiter == 0 {
                fail_forward!(ERR82);
            }

            **parsed_pattern = META_CALLOUT_STRING;
            *parsed_pattern = (*parsed_pattern).add(3); /* Skip pattern info */

            loop {
                *ptr = ptr.add(1);
                if *ptr >= ptrend {
                    *ptr = startptr; /* To give a more useful message */
                    return Err(ERR81);
                }
                if **ptr as u32 == delimiter && {
                    *ptr = ptr.add(1);
                    *ptr >= ptrend || **ptr as u32 != delimiter
                } {
                    break;
                }
            }

            calloutlength = ptr.offset_from(startptr) as PCRE2_SIZE;
            if calloutlength > u32::MAX as PCRE2_SIZE {
                return Err(ERR72);
            }
            **parsed_pattern = calloutlength as u32;
            *parsed_pattern = (*parsed_pattern).add(1);
            offset = startptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
            putoffset(offset, parsed_pattern);
        }
        /* Handle an optional numerical argument, <= 255. */
        else {
            let mut n: c_int = 0;
            **parsed_pattern = META_CALLOUT_NUMBER; /* Numerical callout */
            *parsed_pattern = (*parsed_pattern).add(3); /* Skip pattern info */
            while *ptr < ptrend && IS_DIGIT(**ptr as u32) {
                n = n * 10 + (**ptr as c_int - CHAR_0 as c_int);
                *ptr = ptr.add(1);
                if n > 255 {
                    return Err(ERR38);
                }
            }
            **parsed_pattern = n as u32;
            *parsed_pattern = (*parsed_pattern).add(1);
        }

        /* Both formats must have a closing parenthesis. */
        if *ptr >= ptrend || **ptr as u32 != CHAR_RIGHT_PARENTHESIS {
            return Err(ERR39);
        }
        *ptr = ptr.add(1);

        *(*previous_callout).add(1) = ptr.offset_from((*cb).start_pattern) as u32;
        *(*previous_callout).add(2) = 0;
        Ok(())
    }
}

/* The (?(...) conditional group case. On entry `*ptr` points at the '(' after
'?('  -- actually at the char that is the '(' of the condition (the C did
`case CHAR_LEFT_PARENTHESIS:` then `if (++ptr >= ptrend)`). */
unsafe fn parse_paren_cond(
    ptr: &mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    _options: &mut u32,
    parsed_pattern: &mut *mut u32,
    nest_depth: &mut u16,
    expect_cond_assert: &mut c_int,
    utf: BOOL,
    name: &mut PCRE2_SPTR,
    namelen: &mut u32,
    cb: *mut compile_block,
    i: &mut c_int,
    offset: &mut PCRE2_SIZE,
    terminator: &mut u32,
) -> Result<(), c_int> {
    unsafe {
        let mut errorcode: c_int = 0;

        macro_rules! fail_forward {
            ($e:expr) => {{
                *ptr = ptr.add(1);
                if utf != 0 {
                    forwardchartest(ptr, ptrend);
                }
                return Err($e);
            }};
        }

        *ptr = ptr.add(1);
        if *ptr >= ptrend {
            return Err(ERR14); /* UNCLOSED_PARENTHESIS */
        }
        *nest_depth += 1;

        /* (?( followed by ? or * -> assertion condition. */
        if **ptr as u32 == CHAR_QUESTION_MARK || **ptr as u32 == CHAR_ASTERISK {
            **parsed_pattern = META_COND_ASSERT;
            *parsed_pattern = (*parsed_pattern).add(1);
            *ptr = ptr.sub(1); /* Pull pointer back to the opening parenthesis. */
            *expect_cond_assert = 2;
            return Ok(()); /* break */
        }

        /* Handle (?([+-]number)... */
        if read_number(
            ptr,
            ptrend,
            (*cb).bracount as i32,
            MAX_GROUP_NUMBER,
            ERR61 as u32,
            i,
            &mut errorcode,
        ) != FALSE
        {
            if *i <= 0 {
                return Err(ERR15);
            }
            **parsed_pattern = META_COND_NUMBER;
            *parsed_pattern = (*parsed_pattern).add(1);
            *offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE - 2;
            putoffset(*offset, parsed_pattern);
            **parsed_pattern = *i as u32;
            *parsed_pattern = (*parsed_pattern).add(1);
        } else if errorcode != 0 {
            return Err(errorcode); /* Number too big */
        }
        /* (?(VERSION[>]=n.m)... */
        else if ptrend.offset_from(*ptr) >= 10
            && crate::string_utils::strncmp_c8(
                *ptr,
                STRING_VERSION.as_ptr() as *const c_char,
                7,
            ) == 0
            && *ptr.add(7) as u32 != CHAR_RIGHT_PARENTHESIS
        {
            let mut ge: u32 = 0;
            let mut major: c_int = 0;
            let mut minor: c_int = 0;

            *ptr = ptr.add(7);
            if **ptr as u32 == CHAR_GREATER_THAN_SIGN {
                ge = 1;
                *ptr = ptr.add(1);
            }

            if **ptr as u32 != CHAR_EQUALS_SIGN || {
                *ptr = ptr.add(1);
                !IS_DIGIT(**ptr as u32)
            } {
                errorcode = ERR79;
                if ge == 0 {
                    fail_forward!(errorcode);
                }
                return Err(errorcode);
            }

            if read_number(ptr, ptrend, -1, 1000, ERR79 as u32, &mut major, &mut errorcode)
                == FALSE
            {
                return Err(errorcode);
            }

            if *ptr < ptrend && **ptr as u32 == CHAR_DOT {
                *ptr = ptr.add(1);
                if *ptr >= ptrend || !IS_DIGIT(**ptr as u32) {
                    errorcode = ERR79;
                    if *ptr < ptrend {
                        fail_forward!(errorcode);
                    }
                    return Err(errorcode);
                }
                if read_number(ptr, ptrend, -1, 1000, ERR79 as u32, &mut minor, &mut errorcode)
                    == FALSE
                {
                    return Err(errorcode);
                }
            }
            if *ptr >= ptrend || **ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                errorcode = ERR79;
                if *ptr < ptrend {
                    fail_forward!(errorcode);
                }
                return Err(errorcode);
            }

            **parsed_pattern = META_COND_VERSION;
            *parsed_pattern = (*parsed_pattern).add(1);
            **parsed_pattern = ge;
            *parsed_pattern = (*parsed_pattern).add(1);
            **parsed_pattern = major as u32;
            *parsed_pattern = (*parsed_pattern).add(1);
            **parsed_pattern = minor as u32;
            *parsed_pattern = (*parsed_pattern).add(1);
        }
        /* All remaining cases read a name. */
        else {
            let mut was_r_ampersand: BOOL = FALSE;

            if **ptr as u32 == CHAR_R && ptrend.offset_from(*ptr) > 1 && *ptr.add(1) as u32 == CHAR_AMPERSAND {
                *terminator = CHAR_RIGHT_PARENTHESIS;
                was_r_ampersand = TRUE;
                *ptr = ptr.add(1);
            } else if **ptr as u32 == CHAR_LESS_THAN_SIGN {
                *terminator = CHAR_GREATER_THAN_SIGN;
            } else if **ptr as u32 == CHAR_APOSTROPHE {
                *terminator = CHAR_APOSTROPHE;
            } else {
                *terminator = CHAR_RIGHT_PARENTHESIS;
                *ptr = ptr.sub(1); /* Point to char before name */
            }

            if read_name(
                ptr,
                ptrend,
                utf,
                *terminator,
                offset,
                name,
                namelen,
                &mut errorcode,
                cb,
            ) == FALSE
            {
                return Err(errorcode);
            }

            /* Handle (?(R&name) */
            if was_r_ampersand != 0 {
                **parsed_pattern = META_COND_RNAME;
                *ptr = ptr.sub(1); /* Back to closing parens */
            }
            /* Handle (?(name). */
            else if *terminator == CHAR_RIGHT_PARENTHESIS {
                if *namelen == 6
                    && crate::string_utils::strncmp_c8(
                        *name,
                        STRING_DEFINE.as_ptr() as *const c_char,
                        6,
                    ) == 0
                {
                    **parsed_pattern = META_COND_DEFINE;
                } else {
                    *i = 1;
                    while *i < *namelen as c_int {
                        if !IS_DIGIT(*(*name).add(*i as usize) as u32) {
                            break;
                        }
                        *i += 1;
                    }
                    **parsed_pattern = if *(*name) as u32 == CHAR_R && *i >= *namelen as c_int {
                        META_COND_RNUMBER
                    } else {
                        META_COND_NAME
                    };
                }
                *ptr = ptr.sub(1); /* Back to closing parens */
            }
            /* Handle (?('name') or (?(<name>) */
            else {
                **parsed_pattern = META_COND_NAME;
            }

            /* All these cases except DEFINE end with the name length and
            offset; DEFINE just has an offset. */
            let was_define = **parsed_pattern == META_COND_DEFINE;
            *parsed_pattern = (*parsed_pattern).add(1);
            if !was_define {
                **parsed_pattern = *namelen;
                *parsed_pattern = (*parsed_pattern).add(1);
            }
            putoffset(*offset, parsed_pattern);
        }

        /* Check the closing parenthesis of the condition. */
        if *ptr >= ptrend || **ptr as u32 != CHAR_RIGHT_PARENTHESIS {
            return Err(ERR24);
        }
        *ptr = ptr.add(1);
        Ok(())
    }
}

/* The default case after `(?`: (?| group, or an option setting, optionally
followed by a non-capturing group. Returns Ok(true) if a shared goto was
requested (pgoto set), Ok(false) for the C `break`, Err for goto FAILED. */
unsafe fn parse_paren_default(
    ptr: &mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    options: &mut u32,
    xoptions: &mut u32,
    parsed_pattern: &mut *mut u32,
    nest_depth: &mut u16,
    _utf: BOOL,
    top_nest: &mut *mut nest_save,
    end_nests: *mut nest_save,
    cb: *mut compile_block,
    pgoto: &mut Option<PGoto>,
) -> Result<bool, c_int> {
    unsafe {
        if **ptr as u32 == CHAR_MINUS
            && ptrend.offset_from(*ptr) > 1
            && IS_DIGIT(*ptr.add(1) as u32)
        {
            *pgoto = Some(PGoto::RecursionByNumber);
            return Ok(true);
        }

        *nest_depth += 1;
        if (*top_nest).is_null() {
            *top_nest = (*cb).start_workspace as *mut nest_save;
        } else {
            *top_nest = (*top_nest).add(1);
            if *top_nest >= end_nests {
                return Err(ERR84);
            }
        }
        (**top_nest).nest_depth = *nest_depth;
        (**top_nest).flags = 0;
        (**top_nest).options = *options & PARSE_TRACKED_OPTIONS;
        (**top_nest).xoptions = *xoptions & PARSE_TRACKED_EXTRA_OPTIONS;

        /* Start of (?| group. */
        if **ptr as u32 == CHAR_VERTICAL_LINE {
            (**top_nest).reset_group = (*cb).bracount as u16;
            (**top_nest).max_group = (*cb).bracount as u16;
            (**top_nest).flags |= NSF_RESET;
            (*cb).external_flags |= PCRE2_DUPCAPUSED;
            **parsed_pattern = META_NOCAPTURE;
            *parsed_pattern = (*parsed_pattern).add(1);
            *ptr = ptr.add(1);
            return Ok(false);
        }

        /* Scan for options imnrsxJU. */
        let mut hyphenok: BOOL = TRUE;
        let oldoptions = *options;
        let oldxoptions = *xoptions;

        (**top_nest).reset_group = 0;
        (**top_nest).max_group = 0;
        let mut set: u32 = 0;
        let mut unset: u32 = 0;
        let mut optset_is_unset = false;
        let mut xset: u32 = 0;
        let mut xunset: u32 = 0;
        let mut xoptset_is_unset = false;

        macro_rules! optset_or {
            ($v:expr) => {{
                if optset_is_unset {
                    unset |= $v;
                } else {
                    set |= $v;
                }
            }};
        }
        macro_rules! xoptset_or {
            ($v:expr) => {{
                if xoptset_is_unset {
                    xunset |= $v;
                } else {
                    xset |= $v;
                }
            }};
        }

        /* ^ at the start unsets irmnsx and disables '-'. */
        if *ptr < ptrend && **ptr as u32 == CHAR_CIRCUMFLEX_ACCENT {
            *options &= !(PCRE2_CASELESS
                | PCRE2_MULTILINE
                | PCRE2_NO_AUTO_CAPTURE
                | PCRE2_DOTALL
                | PCRE2_EXTENDED
                | PCRE2_EXTENDED_MORE);
            *xoptions &= !(PCRE2_EXTRA_CASELESS_RESTRICT);
            hyphenok = FALSE;
            *ptr = ptr.add(1);
        }

        while *ptr < ptrend
            && **ptr as u32 != CHAR_RIGHT_PARENTHESIS
            && **ptr as u32 != CHAR_COLON
        {
            let ch = **ptr as u32;
            *ptr = ptr.add(1);
            match ch {
                CHAR_MINUS => {
                    if hyphenok == FALSE {
                        return Err(ERR94);
                    }
                    optset_is_unset = true;
                    xoptset_is_unset = true;
                    hyphenok = FALSE;
                }

                CHAR_a => {
                    if *ptr < ptrend {
                        if **ptr as u32 == CHAR_D {
                            xoptset_or!(PCRE2_EXTRA_ASCII_BSD);
                            *ptr = ptr.add(1);
                            continue;
                        }
                        if **ptr as u32 == CHAR_P {
                            xoptset_or!(PCRE2_EXTRA_ASCII_POSIX | PCRE2_EXTRA_ASCII_DIGIT);
                            *ptr = ptr.add(1);
                            continue;
                        }
                        if **ptr as u32 == CHAR_S {
                            xoptset_or!(PCRE2_EXTRA_ASCII_BSS);
                            *ptr = ptr.add(1);
                            continue;
                        }
                        if **ptr as u32 == CHAR_T {
                            xoptset_or!(PCRE2_EXTRA_ASCII_DIGIT);
                            *ptr = ptr.add(1);
                            continue;
                        }
                        if **ptr as u32 == CHAR_W {
                            xoptset_or!(PCRE2_EXTRA_ASCII_BSW);
                            *ptr = ptr.add(1);
                            continue;
                        }
                    }
                    xoptset_or!(PCRE2_EXTRA_ASCII_BSD
                        | PCRE2_EXTRA_ASCII_BSS
                        | PCRE2_EXTRA_ASCII_BSW
                        | PCRE2_EXTRA_ASCII_DIGIT
                        | PCRE2_EXTRA_ASCII_POSIX);
                }

                CHAR_J => {
                    optset_or!(PCRE2_DUPNAMES);
                    (*cb).external_flags |= PCRE2_JCHANGED;
                }

                CHAR_i => optset_or!(PCRE2_CASELESS),
                CHAR_m => optset_or!(PCRE2_MULTILINE),
                CHAR_n => optset_or!(PCRE2_NO_AUTO_CAPTURE),
                CHAR_r => xoptset_or!(PCRE2_EXTRA_CASELESS_RESTRICT),
                CHAR_s => optset_or!(PCRE2_DOTALL),
                CHAR_U => optset_or!(PCRE2_UNGREEDY),

                CHAR_x => {
                    optset_or!(PCRE2_EXTENDED);
                    if *ptr < ptrend && **ptr as u32 == CHAR_x {
                        optset_or!(PCRE2_EXTENDED_MORE);
                        *ptr = ptr.add(1);
                    }
                }

                _ => {
                    return Err(ERR11);
                }
            }
        }

        /* Reconcile EXTENDED / EXTENDED_MORE. */
        if (set & (PCRE2_EXTENDED | PCRE2_EXTENDED_MORE)) == PCRE2_EXTENDED
            || (unset & PCRE2_EXTENDED) != 0
        {
            unset |= PCRE2_EXTENDED_MORE;
        }

        *options = (*options | set) & (!unset);
        *xoptions = (*xoptions | xset) & (!xunset);

        if *ptr >= ptrend {
            return Err(ERR14); /* UNCLOSED_PARENTHESIS */
        }
        let ch = **ptr as u32;
        *ptr = ptr.add(1);
        if ch == CHAR_RIGHT_PARENTHESIS {
            *nest_depth -= 1; /* Not a nested group after all. */
            if *top_nest > (*cb).start_workspace as *mut nest_save
                && (*(*top_nest).offset(-1)).nest_depth == *nest_depth
            {
                *top_nest = (*top_nest).sub(1);
            } else {
                (**top_nest).nest_depth = *nest_depth;
            }
        } else {
            **parsed_pattern = META_NOCAPTURE;
            *parsed_pattern = (*parsed_pattern).add(1);
        }

        /* If nothing changed, no need to record. */
        if *options != oldoptions || *xoptions != oldxoptions {
            **parsed_pattern = META_OPTIONS;
            *parsed_pattern = (*parsed_pattern).add(1);
            **parsed_pattern = *options;
            *parsed_pattern = (*parsed_pattern).add(1);
            **parsed_pattern = *xoptions;
            *parsed_pattern = (*parsed_pattern).add(1);
        }

        Ok(false)
    }
}
