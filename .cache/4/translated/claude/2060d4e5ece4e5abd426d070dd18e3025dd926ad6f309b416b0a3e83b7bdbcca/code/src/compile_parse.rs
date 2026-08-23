// Translated from pcre2_compile.c lines 3112-5966 (parse_regex)
use crate::compile_h::*;
use crate::compile_tables::*;
use crate::compile_util::*;
use crate::internal::*;
use crate::pcre2_pub::*;
use crate::tables::*;
use crate::ucd_data::*;
use crate::ucp::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

/* A structure and some flags for dealing with nested groups. */

#[repr(C)]
#[derive(Copy, Clone)]
struct nest_save {
    nest_depth: u16,
    reset_group: u16,
    max_group: u16,
    flags: u16,
    options: u32,
    xoptions: u32,
}

const NSF_RESET: u16 = 0x0001u16;
const NSF_CONDASSERT: u16 = 0x0002u16;
const NSF_ATOMICSR: u16 = 0x0004u16;

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

/* States used for analyzing ranges in character classes. The two OK values
must be last. */

const RANGE_NO: u32 = 0;
const RANGE_STARTED: u32 = 1;
const RANGE_FORBID_NO: u32 = 2;
const RANGE_FORBID_STARTED: u32 = 3;
const RANGE_OK_ESCAPED: u32 = 4;
const RANGE_OK_LITERAL: u32 = 5;

/* States used for analyzing operators and operands in extended character
classes. */

const CLASS_OP_EMPTY: u32 = 0;
const CLASS_OP_OPERAND: u32 = 1;
const CLASS_OP_OPERATOR: u32 = 2;

/* States used for determining the parse mode in character classes. The two
PERL_EXT values must be last. */

const CLASS_MODE_NORMAL: u32 = 0;
const CLASS_MODE_ALT_EXT: u32 = 1;
const CLASS_MODE_PERL_EXT: u32 = 2;
const CLASS_MODE_PERL_EXT_LEAF: u32 = 3;

/* String constants used by the parser. */

static STRING_WEIRD_STARTWORD: [u8; 7] = [b'[', b':', b'<', b':', b']', b']', 0];
static STRING_WEIRD_ENDWORD: [u8; 7] = [b'[', b':', b'>', b':', b']', b']', 0];
static STR_Q_BACKSLASH_E: [u8; 4] = [b'Q', b'\\', b'E', 0];
static STRING_VERSION: [u8; 8] = [b'V', b'E', b'R', b'S', b'I', b'O', b'N', 0];
static STRING_DEFINE: [u8; 7] = [b'D', b'E', b'F', b'I', b'N', b'E', 0];

#[inline(always)]
unsafe fn NAMED_GROUP_GET_HASH(ng: *const named_group) -> u16 {
    (*ng).hash_dup & NAMED_GROUP_HASH_MASK
}

/* Here's the actual function. */

pub(crate) unsafe fn parse_regex(
    ptr: PCRE2_SPTR,
    options: u32,
    xoptions: u32,
    has_lookbehind: *mut BOOL,
    cb: *mut compile_block,
) -> c_int {
    let mut ptr = ptr;
    let mut options = options;
    let mut xoptions = xoptions;
    let mut c: u32 = 0;
    let mut delimiter: u32;
    let mut namelen: u32 = 0;
    let mut class_range_state: u32 = 0;
    let mut class_op_state: u32 = 0;
    let mut class_mode_state: u32 = 0;
    let mut class_start: *mut u32 = core::ptr::null_mut();
    let mut verblengthptr: *mut u32 = core::ptr::null_mut(); /* Value avoids compiler warning */
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
    let mut hash: u16;
    let mut after_manual_callout: c_int = 0;
    let mut expect_cond_assert: c_int = 0;
    let mut errorcode: c_int = 0;
    let mut escape: c_int;
    let mut i: c_int = 0;
    let mut inescq: BOOL = FALSE;
    let mut inverbname: BOOL = FALSE;
    let utf: BOOL = ((options & PCRE2_UTF) != 0) as BOOL;
    let auto_callout: BOOL = ((options & PCRE2_AUTO_CALLOUT) != 0) as BOOL;
    let mut is_dupname: BOOL;
    let mut negate_class: BOOL;
    let mut okquantifier: BOOL = FALSE;
    let mut thisptr: PCRE2_SPTR;
    let mut name: PCRE2_SPTR = core::ptr::null();
    let ptrend: PCRE2_SPTR = (*cb).end_pattern;
    let mut verbnamestart: PCRE2_SPTR = core::ptr::null(); /* Value avoids compiler warning */
    let mut class_range_forbid_ptr: PCRE2_SPTR = core::ptr::null();
    let mut ng: *mut named_group;
    let mut top_nest: *mut nest_save;
    let mut end_nests: *mut nest_save;

    /* Insert leading items for word and line matching (features provided for the
    benefit of pcre2grep). */

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

    'failed: {
        'failed_back: {
            'failed_forward: {
                'unclosed_parenthesis: {
                    'parsed_end: {
                        /* If the pattern is actually a literal string, process it separately to
                        avoid cluttering up the main loop. */

                        if (options & PCRE2_LITERAL) != 0 {
                            while ptr < ptrend {
                                if parsed_pattern >= parsed_pattern_end {
                                    errorcode = ERR(63); /* Internal error (parsed pattern overflow) */
                                    break 'failed;
                                }

                                thisptr = ptr;
                                /* GETCHARINCTEST(c, ptr) */
                                c = *ptr as u32;
                                ptr = ptr.add(1);
                                if utf != 0 && c >= 0xc0u32 {
                                    let r = getutf8inc(c, ptr);
                                    c = r.0;
                                    ptr = r.1;
                                }
                                if auto_callout != 0 {
                                    parsed_pattern = manage_callouts(
                                        thisptr,
                                        &mut previous_callout,
                                        auto_callout,
                                        parsed_pattern,
                                        cb,
                                    );
                                }
                                /* PARSED_LITERAL(c, parsed_pattern) */
                                *parsed_pattern = c;
                                parsed_pattern = parsed_pattern.add(1);
                                okquantifier = TRUE;
                            }
                            break 'parsed_end;
                        }

                        /* Process a real regex which may contain meta-characters. */

                        top_nest = core::ptr::null_mut();
                        end_nests =
                            (*cb).start_workspace.add((*cb).workspace_size) as *mut nest_save;

                        /* The size of the nest_save structure might not be a factor of the size
                        of the workspace. Therefore we must round down end_nests so as to
                        correctly avoid creating a nest_save that spans the end of the
                        workspace. */

                        end_nests = (end_nests as *mut c_char).sub(
                            ((*cb).workspace_size * core::mem::size_of::<PCRE2_UCHAR>())
                                % core::mem::size_of::<nest_save>(),
                        ) as *mut nest_save;

                        /* PCRE2_EXTENDED_MORE implies PCRE2_EXTENDED */

                        if (options & PCRE2_EXTENDED_MORE) != 0 {
                            options |= PCRE2_EXTENDED;
                        }

                        /* Now scan the pattern */

                        'mainloop: while ptr < ptrend {
                            let prev_expect_cond_assert: c_int;
                            let mut min_repeat: u32 = 0;
                            let mut max_repeat: u32 = 0;
                            let mut set: u32;
                            let mut unset: u32;
                            let mut optset: *mut u32;
                            let mut xset: u32;
                            let mut xunset: u32;
                            let mut xoptset: *mut u32;
                            let mut terminator: u32 = 0;
                            let prev_meta_quantifier: u32;
                            let prev_okquantifier: BOOL;
                            let mut tempptr: PCRE2_SPTR = core::ptr::null();
                            let mut offset: PCRE2_SIZE = 0;

                            if nest_depth as u32 > (*(*cb).cx).parens_nest_limit {
                                errorcode = ERR(19);
                                break 'failed; /* Parentheses too deeply nested */
                            }

                            /* Check that we haven't emitted too much into parsed_pattern. */

                            if parsed_pattern >= parsed_pattern_end {
                                errorcode = ERR(63); /* Internal error (parsed pattern overflow) */
                                break 'failed;
                            }

                            /* If the last time round this loop something was added,
                            parsed_pattern will no longer be equal to this_parsed_item. Remember
                            where the previous item started and reset for the next item. */

                            if this_parsed_item != parsed_pattern {
                                prev_parsed_item = this_parsed_item;
                                this_parsed_item = parsed_pattern;
                            }

                            /* Get next input character, save its position for callout
                            handling. */

                            thisptr = ptr;
                            /* GETCHARINCTEST(c, ptr) */
                            c = *ptr as u32;
                            ptr = ptr.add(1);
                            if utf != 0 && c >= 0xc0u32 {
                                let r = getutf8inc(c, ptr);
                                c = r.0;
                                ptr = r.1;
                            }

                            /* Copy quoted literals until \E, allowing for the possibility of
                            automatic callouts, except when processing a (*VERB) "name". */

                            if inescq != 0 {
                                if c == CHAR_BACKSLASH && ptr < ptrend && *ptr as u32 == CHAR_E {
                                    inescq = FALSE;
                                    ptr = ptr.add(1); /* Skip E */
                                } else {
                                    if inverbname != 0 {
                                        /* Don't use PARSED_LITERAL() because it sets
                                        okquantifier. */
                                        *parsed_pattern = c;
                                        parsed_pattern = parsed_pattern.add(1);
                                    } else {
                                        let old = after_manual_callout;
                                        after_manual_callout = after_manual_callout - 1;
                                        if old <= 0 {
                                            parsed_pattern = manage_callouts(
                                                thisptr,
                                                &mut previous_callout,
                                                auto_callout,
                                                parsed_pattern,
                                                cb,
                                            );
                                        }
                                        /* PARSED_LITERAL(c, parsed_pattern) */
                                        *parsed_pattern = c;
                                        parsed_pattern = parsed_pattern.add(1);
                                        okquantifier = TRUE;
                                    }
                                    meta_quantifier = 0;
                                }
                                continue 'mainloop; /* Next character */
                            }

                            /* If we are processing the "name" part of a (*VERB:NAME) item, all
                            characters up to the closing parenthesis are literals except when
                            PCRE2_ALT_VERBNAMES is set. */

                            if inverbname != 0
                                && (
                                    /* EITHER: not both options set */
                                    ((options & (PCRE2_EXTENDED | PCRE2_ALT_VERBNAMES))
                                        != (PCRE2_EXTENDED | PCRE2_ALT_VERBNAMES))
                                        ||
                                        /* OR: character > 255 AND not Unicode Pattern White Space */
                                        (c > 255 && (c | 1) != 0x200f && (c | 1) != 0x2029)
                                        ||
                                        /* OR: not a # comment or isspace() white space */
                                        (c < 256
                                            && c != CHAR_NUMBER_SIGN
                                            && (*(*cb).ctypes.add(c as usize) & ctype_space) == 0
                                            /* and not CHAR_NEL when Unicode is supported */
                                            && c != CHAR_NEL)
                                )
                            {
                                let verbnamelength: PCRE2_SIZE;

                                'verb_switch: {
                                    if c == CHAR_RIGHT_PARENTHESIS {
                                        inverbname = FALSE;
                                        /* This is the length in characters */
                                        verbnamelength = parsed_pattern
                                            .offset_from(verblengthptr)
                                            as PCRE2_SIZE
                                            - 1;
                                        /* But the limit on the length is in code units */
                                        if ptr.offset_from(verbnamestart) - 1 > MAX_MARK as isize {
                                            ptr = ptr.sub(1);
                                            errorcode = ERR(76);
                                            break 'failed;
                                        }
                                        *verblengthptr = verbnamelength as u32;

                                        /* If this name was on a verb such as (*ACCEPT) which does
                                        not continue, a (*MARK) was generated for the name. We now
                                        add the original verb as the next item. */

                                        if add_after_mark != 0 {
                                            *parsed_pattern = add_after_mark;
                                            parsed_pattern = parsed_pattern.add(1);
                                            add_after_mark = 0;
                                        }
                                        break 'verb_switch;
                                    }

                                    if c == CHAR_BACKSLASH {
                                        if (options & PCRE2_ALT_VERBNAMES) != 0 {
                                            escape = _pcre2_check_escape_8(
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
                                                break 'failed;
                                            }
                                        } else {
                                            escape = 0; /* Treat all as literal */
                                        }

                                        if escape == 0 {
                                            /* Don't use PARSED_LITERAL() because it sets
                                            okquantifier. */
                                            *parsed_pattern = c;
                                            parsed_pattern = parsed_pattern.add(1);
                                        } else if escape == ESC_ub {
                                            *parsed_pattern = CHAR_u;
                                            parsed_pattern = parsed_pattern.add(1);
                                            /* PARSED_LITERAL(CHAR_LEFT_CURLY_BRACKET, parsed_pattern) */
                                            *parsed_pattern = CHAR_LEFT_CURLY_BRACKET;
                                            parsed_pattern = parsed_pattern.add(1);
                                            okquantifier = TRUE;
                                        } else if escape == ESC_Q {
                                            inescq = TRUE;
                                        } else if escape == ESC_E {
                                            /* Ignore */
                                        } else {
                                            errorcode = ERR(40); /* Invalid in verb name */
                                            break 'failed;
                                        }
                                        break 'verb_switch;
                                    }

                                    /* default: Don't use PARSED_LITERAL() because it sets
                                    okquantifier. */
                                    *parsed_pattern = c;
                                    parsed_pattern = parsed_pattern.add(1);
                                }
                                continue 'mainloop; /* Next character in pattern */
                            }

                            /* Not a verb name character. At this point we must process everything
                            that must not change the quantification state. */

                            if c == CHAR_BACKSLASH && ptr < ptrend {
                                if *ptr as u32 == CHAR_Q || *ptr as u32 == CHAR_E {
                                    /* A literal inside a \Q...\E is not allowed if we are
                                    expecting a conditional assertion, but an empty \Q\E sequence
                                    is OK. */
                                    if expect_cond_assert > 0
                                        && *ptr as u32 == CHAR_Q
                                        && !(ptrend.offset_from(ptr) >= 3
                                            && *ptr.add(1) as u32 == CHAR_BACKSLASH
                                            && *ptr.add(2) as u32 == CHAR_E)
                                    {
                                        ptr = ptr.sub(1);
                                        errorcode = ERR(28);
                                        break 'failed;
                                    }
                                    inescq = (*ptr as u32 == CHAR_Q) as BOOL;
                                    ptr = ptr.add(1);
                                    continue 'mainloop;
                                }
                            }

                            /* Skip over whitespace and # comments in extended mode. */

                            if (options & PCRE2_EXTENDED) != 0 {
                                if c < 256 && (*(*cb).ctypes.add(c as usize) & ctype_space) != 0 {
                                    continue 'mainloop;
                                }
                                if c == CHAR_NEL || (c | 1) == 0x200f || (c | 1) == 0x2029 {
                                    continue 'mainloop;
                                }
                                if c == CHAR_NUMBER_SIGN {
                                    while ptr < ptrend {
                                        /* IS_NEWLINE(ptr) : for non-fixed-length newline cases,
                                        IS_NEWLINE sets cb->nllen. */
                                        let is_nl: bool = if (*cb).nltype != NLTYPE_FIXED {
                                            ptr < (*cb).end_pattern
                                                && crate::newline::_pcre2_is_newline_8(
                                                    ptr,
                                                    (*cb).nltype,
                                                    (*cb).end_pattern,
                                                    &mut (*cb).nllen,
                                                    utf,
                                                ) != 0
                                        } else {
                                            ptr <= (*cb).end_pattern.sub((*cb).nllen as usize)
                                                && *ptr == (*cb).nl[0]
                                                && ((*cb).nllen == 1
                                                    || *ptr.add(1) == (*cb).nl[1])
                                        };
                                        if is_nl {
                                            ptr = ptr.add((*cb).nllen as usize);
                                            break;
                                        }
                                        ptr = ptr.add(1);
                                        if utf != 0 {
                                            /* FORWARDCHARTEST(ptr, ptrend) */
                                            while ptr < ptrend && (*ptr & 0xc0u8) == 0x80u8 {
                                                ptr = ptr.add(1);
                                            }
                                        }
                                    }
                                    continue 'mainloop; /* Next character in pattern */
                                }
                            }

                            /* Skip over bracketed comments */

                            if c == CHAR_LEFT_PARENTHESIS
                                && ptrend.offset_from(ptr) >= 2
                                && *ptr.add(0) as u32 == CHAR_QUESTION_MARK
                                && *ptr.add(1) as u32 == CHAR_NUMBER_SIGN
                            {
                                loop {
                                    ptr = ptr.add(1);
                                    if !(ptr < ptrend && *ptr as u32 != CHAR_RIGHT_PARENTHESIS) {
                                        break;
                                    }
                                }
                                if ptr >= ptrend {
                                    errorcode = ERR(18); /* A special error for missing ) in a comment */
                                    break 'failed; /* to make it easier to debug. */
                                }
                                ptr = ptr.add(1);
                                continue 'mainloop; /* Next character in pattern */
                            }

                            /* If the next item is not a quantifier, fill in length of any previous
                            callout and create an auto callout if required. */

                            if c != CHAR_ASTERISK
                                && c != CHAR_PLUS
                                && c != CHAR_QUESTION_MARK
                                && (c != CHAR_LEFT_CURLY_BRACKET || {
                                    tempptr = ptr;
                                    read_repeat_counts(
                                        &mut tempptr,
                                        ptrend,
                                        core::ptr::null_mut(),
                                        core::ptr::null_mut(),
                                        &mut errorcode,
                                    ) == 0
                                })
                            {
                                let old = after_manual_callout;
                                after_manual_callout = after_manual_callout - 1;
                                if old <= 0 {
                                    parsed_pattern = manage_callouts(
                                        thisptr,
                                        &mut previous_callout,
                                        auto_callout,
                                        parsed_pattern,
                                        cb,
                                    );
                                    this_parsed_item = parsed_pattern; /* New start for current item */
                                }
                            }

                            /* If expect_cond_assert is 2, we have just passed (?( and are
                            expecting an assertion, possibly preceded by a callout. */

                            if expect_cond_assert > 0 {
                                let mut ok: BOOL = (c == CHAR_LEFT_PARENTHESIS
                                    && ptrend.offset_from(ptr) >= 3
                                    && (*ptr.add(0) as u32 == CHAR_QUESTION_MARK
                                        || *ptr.add(0) as u32 == CHAR_ASTERISK))
                                    as BOOL;
                                if ok != 0 {
                                    if *ptr.add(0) as u32 == CHAR_ASTERISK {
                                        /* New alpha assertion format, possibly */
                                        ok = ((*(*cb).ctypes.add(*ptr.add(1) as usize)
                                            & ctype_lcletter)
                                            != 0) as BOOL;
                                    } else {
                                        /* Traditional symbolic format */
                                        let p1 = *ptr.add(1) as u32;
                                        if p1 == CHAR_C {
                                            ok = (expect_cond_assert == 2) as BOOL;
                                        } else if p1 == CHAR_EQUALS_SIGN
                                            || p1 == CHAR_EXCLAMATION_MARK
                                        {
                                            /* nothing */
                                        } else if p1 == CHAR_LESS_THAN_SIGN {
                                            ok = (*ptr.add(2) as u32 == CHAR_EQUALS_SIGN
                                                || *ptr.add(2) as u32 == CHAR_EXCLAMATION_MARK)
                                                as BOOL;
                                        } else {
                                            ok = FALSE;
                                        }
                                    }
                                }

                                if ok == 0 {
                                    errorcode = ERR(28);
                                    if expect_cond_assert == 2 {
                                        break 'failed;
                                    }
                                    break 'failed_back;
                                }
                            }

                            /* Remember whether we are expecting a conditional assertion, and set
                            the default for this item. */

                            prev_expect_cond_assert = expect_cond_assert;
                            expect_cond_assert = 0;

                            /* Remember quantification status for the previous significant item,
                            then set default for this item. */

                            prev_okquantifier = okquantifier;
                            prev_meta_quantifier = meta_quantifier;
                            okquantifier = FALSE;
                            meta_quantifier = 0;

                            /* If the previous significant item was a quantifier, adjust the parsed
                            code if there is a following modifier. */

                            if prev_meta_quantifier != 0
                                && (c == CHAR_QUESTION_MARK || c == CHAR_PLUS)
                            {
                                *parsed_pattern.offset(if prev_meta_quantifier == META_MINMAX {
                                    -3
                                } else {
                                    -1
                                }) = prev_meta_quantifier
                                    + (if c == CHAR_QUESTION_MARK {
                                        0x00020000u32
                                    } else {
                                        0x00010000u32
                                    });
                                continue 'mainloop; /* Next character in pattern */
                            }

                            /* Process the next item in the main part of a pattern. */

                            let mut sw_target: c_int = 0;
                            'sw: loop {
                                'l_rparen: {
                                'l_vbar: {
                                'l_q_lbracket: {
                                'l_define_name: {
                                'l_q_apostrophe: {
                                'l_post_assertion: {
                                'l_post_lookbehind: {
                                'l_q_lt: {
                                'l_negative_look_ahead: {
                                'l_q_bang: {
                                'l_positive_nonatomic_look_ahead: {
                                'l_q_asterisk: {
                                'l_positive_look_ahead: {
                                'l_q_eq: {
                                'l_atomic_group: {
                                'l_q_gt: {
                                'l_q_lparen: {
                                'l_q_c: {
                                'l_read_recursion_arguments: {
                                'l_recurse_by_name: {
                                'l_q_ampersand: {
                                'l_set_recursion: {
                                'l_recursion_bynumber: {
                                'l_q_digit: {
                                'l_q_plus: {
                                'l_q_r: {
                                'l_q_p: {
                                'l_q_default: {
                                'l_lparen: {
                                'l_from_perl_extended_class: {
                                'l_lbracket: {
                                'l_check_quantifier: {
                                'l_lcurly: {
                                'l_query: {
                                'l_plus: {
                                'l_asterisk: {
                                'l_dot: {
                                'l_dollar: {
                                'l_circumflex: {
                                'l_backslash: {
                                'l_default: {
                                    /* switch(c) dispatch */
                                    if sw_target != 0 {
                                        /* goto FROM_PERL_EXTENDED_CLASS */
                                        sw_target = 0;
                                        break 'l_from_perl_extended_class;
                                    }
                                    match c {
                                        CHAR_BACKSLASH => break 'l_backslash,
                                        CHAR_CIRCUMFLEX_ACCENT => break 'l_circumflex,
                                        CHAR_DOLLAR_SIGN => break 'l_dollar,
                                        CHAR_DOT => break 'l_dot,
                                        CHAR_ASTERISK => break 'l_asterisk,
                                        CHAR_PLUS => break 'l_plus,
                                        CHAR_QUESTION_MARK => break 'l_query,
                                        CHAR_LEFT_CURLY_BRACKET => break 'l_lcurly,
                                        CHAR_LEFT_SQUARE_BRACKET => break 'l_lbracket,
                                        CHAR_LEFT_PARENTHESIS => break 'l_lparen,
                                        CHAR_VERTICAL_LINE => break 'l_vbar,
                                        CHAR_RIGHT_PARENTHESIS => break 'l_rparen,
                                        _ => break 'l_default,
                                    }
                                }
                                /* ---- default: Non-special character ---- */
                                /* PARSED_LITERAL(c, parsed_pattern) */
                                *parsed_pattern = c;
                                parsed_pattern = parsed_pattern.add(1);
                                okquantifier = TRUE;
                                break 'sw;
                                }
                                /* ---- Escape sequence ---- */
                                {
                                    tempptr = ptr;
                                    escape = _pcre2_check_escape_8(
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
                                    let mut goto_escape_failed: bool = errorcode != 0;
                                    'escape_retry: loop {
                                        if goto_escape_failed {
                                            /* ESCAPE_FAILED: */
                                            goto_escape_failed = false;
                                            if (xoptions & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL) == 0 {
                                                break 'failed;
                                            }
                                            ptr = tempptr;
                                            if ptr >= ptrend {
                                                c = CHAR_BACKSLASH;
                                            } else {
                                                /* GETCHARINCTEST(c, ptr) */
                                                c = *ptr as u32;
                                                ptr = ptr.add(1);
                                                if utf != 0 && c >= 0xc0u32 {
                                                    let r = getutf8inc(c, ptr);
                                                    c = r.0;
                                                    ptr = r.1;
                                                }
                                            }
                                            escape = 0; /* Treat as literal character */
                                        }

                                        /* The escape was a data escape or literal character. */

                                        if escape == 0 {
                                            /* PARSED_LITERAL(c, parsed_pattern) */
                                            *parsed_pattern = c;
                                            parsed_pattern = parsed_pattern.add(1);
                                            okquantifier = TRUE;
                                        }
                                        /* The escape was a back (or forward) reference. */
                                        else if escape < 0 {
                                            offset = ptr.offset_from((*cb).start_pattern)
                                                as PCRE2_SIZE;
                                            escape = -escape - 1;
                                            *parsed_pattern = META_BACKREF | escape as u32;
                                            parsed_pattern = parsed_pattern.add(1);
                                            if escape < 10 {
                                                if (*cb).small_ref_offset[escape as usize]
                                                    == PCRE2_UNSET
                                                {
                                                    (*cb).small_ref_offset[escape as usize] =
                                                        offset;
                                                }
                                            } else {
                                                /* PUTOFFSET(offset, parsed_pattern) */
                                                *parsed_pattern = (offset >> 32) as u32;
                                                parsed_pattern = parsed_pattern.add(1);
                                                *parsed_pattern = (offset & 0xffffffff) as u32;
                                                parsed_pattern = parsed_pattern.add(1);
                                            }
                                            okquantifier = TRUE;
                                        }
                                        /* The escape was a character class such as \d etc. */
                                        else {
                                            'esc_sw: {
                                                if escape == ESC_C {
                                                    if (options & PCRE2_NEVER_BACKSLASH_C) != 0 {
                                                        errorcode = ERR(83);
                                                        goto_escape_failed = true;
                                                        continue 'escape_retry;
                                                    }
                                                    okquantifier = TRUE;
                                                    *parsed_pattern =
                                                        META_ESCAPE + escape as u32;
                                                    parsed_pattern = parsed_pattern.add(1);
                                                    break 'esc_sw;
                                                }

                                                /* This is a special return that happens only in
                                                EXTRA_ALT_BSUX mode, when \u{ is not followed by
                                                hex digits and }. */

                                                if escape == ESC_ub {
                                                    *parsed_pattern = CHAR_u;
                                                    parsed_pattern = parsed_pattern.add(1);
                                                    /* PARSED_LITERAL(CHAR_LEFT_CURLY_BRACKET, parsed_pattern) */
                                                    *parsed_pattern = CHAR_LEFT_CURLY_BRACKET;
                                                    parsed_pattern = parsed_pattern.add(1);
                                                    okquantifier = TRUE;
                                                    break 'esc_sw;
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
                                                    *parsed_pattern =
                                                        META_ESCAPE + escape as u32;
                                                    parsed_pattern = parsed_pattern.add(1);
                                                    break 'esc_sw;
                                                }

                                                /* Escapes that may change in UCP mode. */

                                                if escape == ESC_d
                                                    || escape == ESC_D
                                                    || escape == ESC_s
                                                    || escape == ESC_S
                                                    || escape == ESC_w
                                                    || escape == ESC_W
                                                {
                                                    okquantifier = TRUE;
                                                    parsed_pattern = handle_escdsw(
                                                        escape,
                                                        parsed_pattern,
                                                        options,
                                                        xoptions,
                                                    );
                                                    break 'esc_sw;
                                                }

                                                /* Unicode property matching */

                                                if escape == ESC_P || escape == ESC_p {
                                                    let mut negated: BOOL = 0;
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
                                                    ) == 0
                                                    {
                                                        goto_escape_failed = true;
                                                        continue 'escape_retry;
                                                    }
                                                    if negated != 0 {
                                                        escape = if escape == ESC_P {
                                                            ESC_p
                                                        } else {
                                                            ESC_P
                                                        };
                                                    }
                                                    *parsed_pattern =
                                                        META_ESCAPE + escape as u32;
                                                    parsed_pattern = parsed_pattern.add(1);
                                                    *parsed_pattern =
                                                        ((ptype as u32) << 16) | pdata as u32;
                                                    parsed_pattern = parsed_pattern.add(1);
                                                    okquantifier = TRUE;
                                                    break 'esc_sw; /* End \P and \p */
                                                }

                                                /* When \g is used with quotes or angle brackets as
                                                delimiters, it is a numerical or named subroutine
                                                call, and control comes here. */

                                                if escape == ESC_g || escape == ESC_k {
                                                    if ptr >= ptrend
                                                        || (*ptr as u32
                                                            != CHAR_LEFT_CURLY_BRACKET
                                                            && *ptr as u32
                                                                != CHAR_LESS_THAN_SIGN
                                                            && *ptr as u32 != CHAR_APOSTROPHE)
                                                    {
                                                        errorcode = if escape == ESC_g {
                                                            ERR(57)
                                                        } else {
                                                            ERR(69)
                                                        };
                                                        goto_escape_failed = true;
                                                        continue 'escape_retry;
                                                    }
                                                    terminator = if *ptr as u32
                                                        == CHAR_LESS_THAN_SIGN
                                                    {
                                                        CHAR_GREATER_THAN_SIGN
                                                    } else if *ptr as u32 == CHAR_APOSTROPHE {
                                                        CHAR_APOSTROPHE
                                                    } else {
                                                        CHAR_RIGHT_CURLY_BRACKET
                                                    };

                                                    /* For a non-braced \g, check for a numerical
                                                    recursion. */

                                                    if escape == ESC_g
                                                        && terminator != CHAR_RIGHT_CURLY_BRACKET
                                                    {
                                                        let mut p: PCRE2_SPTR = ptr.add(1);

                                                        if read_number(
                                                            &mut p,
                                                            ptrend,
                                                            (*cb).bracount as i32,
                                                            MAX_GROUP_NUMBER,
                                                            ERR(61) as u32,
                                                            &mut i,
                                                            &mut errorcode,
                                                        ) != 0
                                                        {
                                                            if p >= ptrend
                                                                || *p as u32 != terminator
                                                            {
                                                                ptr = p;
                                                                errorcode = ERR(119); /* Missing terminator for number */
                                                                goto_escape_failed = true;
                                                                continue 'escape_retry;
                                                            }
                                                            ptr = p.add(1);
                                                            break 'l_set_recursion;
                                                        }
                                                        if errorcode != 0 {
                                                            goto_escape_failed = true;
                                                            continue 'escape_retry;
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
                                                    ) == 0
                                                    {
                                                        goto_escape_failed = true;
                                                        continue 'escape_retry;
                                                    }

                                                    /* \k and \g when used with braces are back
                                                    references, whereas \g used with quotes or
                                                    angle brackets is a recursion */

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

                                                    /* PUTOFFSET(offset, parsed_pattern) */
                                                    *parsed_pattern = (offset >> 32) as u32;
                                                    parsed_pattern = parsed_pattern.add(1);
                                                    *parsed_pattern =
                                                        (offset & 0xffffffff) as u32;
                                                    parsed_pattern = parsed_pattern.add(1);
                                                    okquantifier = TRUE;
                                                    break 'esc_sw; /* End special escape processing */
                                                }

                                                /* default: \A, \B, \b, \G, \K, \Z, \z cannot be
                                                quantified. */
                                                *parsed_pattern = META_ESCAPE + escape as u32;
                                                parsed_pattern = parsed_pattern.add(1);
                                            }
                                        }
                                        break;
                                    }
                                }
                                break 'sw; /* End escape sequence processing */
                                }
                                /* ---- Single-character special items ---- */
                                *parsed_pattern = META_CIRCUMFLEX;
                                parsed_pattern = parsed_pattern.add(1);
                                break 'sw;
                                }
                                *parsed_pattern = META_DOLLAR;
                                parsed_pattern = parsed_pattern.add(1);
                                break 'sw;
                                }
                                *parsed_pattern = META_DOT;
                                parsed_pattern = parsed_pattern.add(1);
                                okquantifier = TRUE;
                                break 'sw;
                                }
                                /* ---- Single-character quantifiers ---- */
                                meta_quantifier = META_ASTERISK;
                                break 'l_check_quantifier;
                                }
                                meta_quantifier = META_PLUS;
                                break 'l_check_quantifier;
                                }
                                meta_quantifier = META_QUERY;
                                break 'l_check_quantifier;
                                }
                                /* ---- Potential {n,m} quantifier ---- */
                                if read_repeat_counts(
                                    &mut ptr,
                                    ptrend,
                                    &mut min_repeat,
                                    &mut max_repeat,
                                    &mut errorcode,
                                ) == 0
                                {
                                    if errorcode != 0 {
                                        break 'failed; /* Error in quantifier. */
                                    }
                                    /* PARSED_LITERAL(c, parsed_pattern) - Not a quantifier */
                                    *parsed_pattern = c;
                                    parsed_pattern = parsed_pattern.add(1);
                                    okquantifier = TRUE;
                                    break 'sw; /* No more quantifier processing */
                                }
                                meta_quantifier = META_MINMAX;
                                /* Fall through */
                                }
                                /* ---- Quantifier post-processing ---- */
                                /* CHECK_QUANTIFIER: */
                                if prev_okquantifier == 0 {
                                    errorcode = ERR(9);
                                    break 'failed;
                                }

                                /* Most (*VERB)s are not allowed to be quantified, but an ungreedy
                                quantifier can be useful for (*ACCEPT). */

                                if *prev_parsed_item == META_ACCEPT {
                                    let mut p: *mut u32 = parsed_pattern.sub(1);
                                    while p >= verbstartptr {
                                        *p.add(1) = *p.add(0);
                                        p = p.sub(1);
                                    }
                                    *verbstartptr = META_NOCAPTURE;
                                    *parsed_pattern.add(1) = META_KET;
                                    parsed_pattern = parsed_pattern.add(2);
                                }

                                /* Now we can put the quantifier into the parsed pattern vector. */

                                *parsed_pattern = meta_quantifier;
                                parsed_pattern = parsed_pattern.add(1);
                                if c == CHAR_LEFT_CURLY_BRACKET {
                                    *parsed_pattern = min_repeat;
                                    parsed_pattern = parsed_pattern.add(1);
                                    *parsed_pattern = max_repeat;
                                    parsed_pattern = parsed_pattern.add(1);
                                }
                                break 'sw;
                                }
                                /* ---- Character class ---- */
                                /* In another (POSIX) regex library, the ugly syntax [[:<:]] and
                                [[:>:]] is used for "start of word" and "end of word". */

                                if ptrend.offset_from(ptr) >= 6
                                    && (crate::string_utils::_pcre2_strncmp_c8_8(
                                        ptr,
                                        STRING_WEIRD_STARTWORD.as_ptr() as *const c_char,
                                        6,
                                    ) == 0
                                        || crate::string_utils::_pcre2_strncmp_c8_8(
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

                                        /* The offset is used only for the "non-fixed length"
                                        error; this won't occur here, so just store zero. */

                                        /* PUTOFFSET((PCRE2_SIZE)0, parsed_pattern) */
                                        *parsed_pattern = (0usize >> 32) as u32;
                                        parsed_pattern = parsed_pattern.add(1);
                                        *parsed_pattern = (0usize & 0xffffffff) as u32;
                                        parsed_pattern = parsed_pattern.add(1);
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
                                    break 'sw;
                                }

                                /* PCRE supports POSIX class stuff inside a class. Perl gives an
                                error if they are encountered at the top level, so we'll do that
                                too. */

                                if ptr < ptrend
                                    && (*ptr as u32 == CHAR_COLON
                                        || *ptr as u32 == CHAR_DOT
                                        || *ptr as u32 == CHAR_EQUALS_SIGN)
                                    && check_posix_syntax(ptr, ptrend, &mut tempptr) != 0
                                {
                                    let old = *ptr as u32;
                                    ptr = ptr.sub(1);
                                    errorcode = if old == CHAR_COLON { ERR(12) } else { ERR(13) };
                                    ptr = tempptr.add(2);
                                    break 'failed;
                                }

                                class_mode_state = if (options & PCRE2_ALT_EXTENDED_CLASS) != 0 {
                                    CLASS_MODE_ALT_EXT
                                } else {
                                    CLASS_MODE_NORMAL
                                };

                                /* Fall through to FROM_PERL_EXTENDED_CLASS */
                                }
                                /* FROM_PERL_EXTENDED_CLASS: */
                                /* Jump here from '(?[...])'. That jump must initialize
                                class_mode_state, set c to the '[' character, and ptr to just
                                after the '['. */
                                okquantifier = TRUE;

                                /* Loop for the contents of the class. Classes may be nested, if
                                PCRE2_ALT_EXTENDED_CLASS is set, or the class is of the form
                                (?[...]). */

                                /* c is still set to '[' so the loop will handle the start of the
                                class. */

                                class_depth_m1 = -1;
                                class_maxdepth_m1 = -1;
                                class_range_state = RANGE_NO;
                                class_op_state = CLASS_OP_EMPTY;
                                class_start = core::ptr::null_mut();

                                'class_loop: loop {
                                    let mut char_is_literal: BOOL = TRUE;

                                    'class_continue: {
                                        'class_literal: {
                                            /* Inside \Q...\E everything is literal except \E */

                                            if inescq != 0 {
                                                if c == CHAR_BACKSLASH
                                                    && ptr < ptrend
                                                    && *ptr as u32 == CHAR_E
                                                {
                                                    inescq = FALSE; /* Reset literal state */
                                                    ptr = ptr.add(1); /* Skip the 'E' */
                                                    break 'class_continue;
                                                }

                                                /* Surprisingly, you cannot use \Q..\E to escape a
                                                character inside a Perl extended class. */

                                                if class_mode_state == CLASS_MODE_PERL_EXT {
                                                    errorcode = ERR(116);
                                                    break 'failed;
                                                }

                                                break 'class_literal;
                                            }

                                            /* Skip over space and tab (only) in extended-more
                                            mode, or anywhere inside a Perl extended class. */

                                            if (c == CHAR_SPACE || c == CHAR_HT)
                                                && ((options & PCRE2_EXTENDED_MORE) != 0
                                                    || class_mode_state >= CLASS_MODE_PERL_EXT)
                                            {
                                                break 'class_continue;
                                            }

                                            /* Handle POSIX class names. */

                                            if class_depth_m1 >= 0
                                                && c == CHAR_LEFT_SQUARE_BRACKET
                                                && ptrend.offset_from(ptr) >= 3
                                                && (*ptr as u32 == CHAR_COLON
                                                    || *ptr as u32 == CHAR_DOT
                                                    || *ptr as u32 == CHAR_EQUALS_SIGN)
                                                && check_posix_syntax(ptr, ptrend, &mut tempptr)
                                                    != 0
                                            {
                                                let mut posix_negate: BOOL = FALSE;
                                                let posix_class: c_int;

                                                /* Perl treats a hyphen before a POSIX class as a
                                                literal, not the start of a range. */

                                                if class_range_state == RANGE_STARTED {
                                                    ptr = tempptr.add(2);
                                                    errorcode = ERR(50);
                                                    break 'failed;
                                                }

                                                /* Perl treats a hyphen after a POSIX class as a
                                                literal. Roll back to the hyphen for the error
                                                position. */

                                                if class_range_state == RANGE_FORBID_STARTED {
                                                    ptr = class_range_forbid_ptr;
                                                    errorcode = ERR(50);
                                                    break 'failed;
                                                }

                                                /* Disallow implicit union in Perl extended
                                                classes. */

                                                if class_op_state == CLASS_OP_OPERAND
                                                    && class_mode_state == CLASS_MODE_PERL_EXT
                                                {
                                                    ptr = tempptr.add(2);
                                                    errorcode = ERR(113);
                                                    break 'failed;
                                                }

                                                if *ptr as u32 != CHAR_COLON {
                                                    ptr = tempptr.add(2);
                                                    errorcode = ERR(13);
                                                    break 'failed;
                                                }

                                                ptr = ptr.add(1);
                                                if *ptr as u32 == CHAR_CIRCUMFLEX_ACCENT {
                                                    posix_negate = TRUE;
                                                    ptr = ptr.add(1);
                                                }

                                                posix_class = check_posix_name(
                                                    ptr,
                                                    tempptr.offset_from(ptr) as c_int,
                                                );
                                                ptr = tempptr.add(2);
                                                if posix_class < 0 {
                                                    errorcode = ERR(30);
                                                    break 'failed;
                                                }

                                                /* Set "a hyphen is forbidden to be the start of a
                                                range". */

                                                class_range_state = RANGE_FORBID_NO;
                                                class_op_state = CLASS_OP_OPERAND;

                                                /* When PCRE2_UCP is set, unless
                                                PCRE2_EXTRA_ASCII_POSIX is set, some of the POSIX
                                                classes are converted to use Unicode properties. */

                                                if (options & PCRE2_UCP) != 0
                                                    && (xoptions & PCRE2_EXTRA_ASCII_POSIX) == 0
                                                    && !((xoptions & PCRE2_EXTRA_ASCII_DIGIT) != 0
                                                        && (posix_class == PC_DIGIT as c_int
                                                            || posix_class == PC_XDIGIT as c_int))
                                                {
                                                    let ptype: c_int =
                                                        posix_substitutes[(2 * posix_class) as usize];
                                                    let pvalue: c_int = posix_substitutes
                                                        [(2 * posix_class + 1) as usize];

                                                    if ptype >= 0 {
                                                        *parsed_pattern = META_ESCAPE
                                                            + (if posix_negate != 0 {
                                                                ESC_P
                                                            } else {
                                                                ESC_p
                                                            }) as u32;
                                                        parsed_pattern = parsed_pattern.add(1);
                                                        *parsed_pattern = ((ptype as u32) << 16)
                                                            | pvalue as u32;
                                                        parsed_pattern = parsed_pattern.add(1);
                                                        break 'class_continue;
                                                    }

                                                    if pvalue != 0 {
                                                        *parsed_pattern = META_ESCAPE
                                                            + (if posix_negate != 0 {
                                                                ESC_H
                                                            } else {
                                                                ESC_h
                                                            }) as u32;
                                                        parsed_pattern = parsed_pattern.add(1);
                                                        break 'class_continue;
                                                    }

                                                    /* Fall through */
                                                }

                                                /* Non-UCP POSIX class */

                                                *parsed_pattern = if posix_negate != 0 {
                                                    META_POSIX_NEG
                                                } else {
                                                    META_POSIX
                                                };
                                                parsed_pattern = parsed_pattern.add(1);
                                                *parsed_pattern = posix_class as u32;
                                                parsed_pattern = parsed_pattern.add(1);
                                            }
                                            /* Check for the start of the outermost class, or the
                                            start of a nested class. */
                                            else if (c == CHAR_LEFT_SQUARE_BRACKET
                                                && (class_depth_m1 < 0
                                                    || class_mode_state == CLASS_MODE_ALT_EXT
                                                    || class_mode_state == CLASS_MODE_PERL_EXT))
                                                || (c == CHAR_LEFT_PARENTHESIS
                                                    && class_mode_state == CLASS_MODE_PERL_EXT)
                                            {
                                                let start_c: u32 = c;
                                                let new_class_mode_state: u32;

                                                /* Update the class mode, if moving into a 'leaf'
                                                inside a Perl extended class. */

                                                if start_c == CHAR_LEFT_SQUARE_BRACKET
                                                    && class_mode_state == CLASS_MODE_PERL_EXT
                                                    && class_depth_m1 >= 0
                                                {
                                                    new_class_mode_state =
                                                        CLASS_MODE_PERL_EXT_LEAF;
                                                } else {
                                                    new_class_mode_state = class_mode_state;
                                                }

                                                /* Tidy up the other class before starting the
                                                nested class. -[ beginning a nested class is a
                                                literal '-' */

                                                if class_range_state == RANGE_STARTED {
                                                    *parsed_pattern.offset(-1) = CHAR_MINUS;
                                                }

                                                /* Disallow implicit union in Perl extended
                                                classes. */

                                                if class_op_state == CLASS_OP_OPERAND
                                                    && class_mode_state == CLASS_MODE_PERL_EXT
                                                {
                                                    errorcode = ERR(113);
                                                    break 'failed;
                                                }

                                                /* Validate nesting depth */
                                                if class_depth_m1 as isize
                                                    >= ECLASS_NEST_LIMIT as isize - 1
                                                {
                                                    ptr = ptr.sub(1); /* Point rightwards at the paren, same as ERR19. */
                                                    errorcode = ERR(107); /* Classes too deeply nested */
                                                    break 'failed;
                                                }

                                                /* Process the character class start. */

                                                negate_class = FALSE;
                                                loop {
                                                    if ptr >= ptrend {
                                                        if start_c == CHAR_LEFT_PARENTHESIS {
                                                            errorcode = ERR(14); /* Missing terminating ')' */
                                                        } else {
                                                            errorcode = ERR(6); /* Missing terminating ']' */
                                                        }
                                                        break 'failed;
                                                    }

                                                    /* GETCHARINCTEST(c, ptr) */
                                                    c = *ptr as u32;
                                                    ptr = ptr.add(1);
                                                    if utf != 0 && c >= 0xc0u32 {
                                                        let r = getutf8inc(c, ptr);
                                                        c = r.0;
                                                        ptr = r.1;
                                                    }
                                                    if new_class_mode_state
                                                        == CLASS_MODE_PERL_EXT
                                                    {
                                                        break;
                                                    } else if c == CHAR_BACKSLASH {
                                                        if ptr < ptrend && *ptr as u32 == CHAR_E {
                                                            ptr = ptr.add(1);
                                                        } else if ptrend.offset_from(ptr) >= 3
                                                            && crate::string_utils::_pcre2_strncmp_c8_8(
                                                                ptr,
                                                                STR_Q_BACKSLASH_E.as_ptr()
                                                                    as *const c_char,
                                                                3,
                                                            ) == 0
                                                        {
                                                            ptr = ptr.add(3);
                                                        } else {
                                                            break;
                                                        }
                                                    } else if (c == CHAR_SPACE || c == CHAR_HT)
                                                        && ((options & PCRE2_EXTENDED_MORE) != 0
                                                            || new_class_mode_state
                                                                >= CLASS_MODE_PERL_EXT)
                                                    {
                                                        continue;
                                                    } else if negate_class == 0
                                                        && c == CHAR_CIRCUMFLEX_ACCENT
                                                    {
                                                        negate_class = TRUE;
                                                    } else {
                                                        break;
                                                    }
                                                }

                                                /* Now the real contents of the class; c has the
                                                first "real" character. Empty classes are permitted
                                                only if the option is set, and if it's not a
                                                Perl-extended class. */

                                                if c == CHAR_RIGHT_SQUARE_BRACKET
                                                    && ((*cb).external_options
                                                        & PCRE2_ALLOW_EMPTY_CLASS)
                                                        != 0
                                                    && new_class_mode_state
                                                        < CLASS_MODE_PERL_EXT
                                                {
                                                    if class_start != core::ptr::null_mut() {
                                                        /* Represents that the class is an extended
                                                        class. */
                                                        *class_start |= CLASS_IS_ECLASS;
                                                        class_start = core::ptr::null_mut();
                                                    }

                                                    *parsed_pattern = if negate_class != 0 {
                                                        META_CLASS_EMPTY_NOT
                                                    } else {
                                                        META_CLASS_EMPTY
                                                    };
                                                    parsed_pattern = parsed_pattern.add(1);

                                                    /* Leave nesting depth unchanged; but check for
                                                    zero depth to handle the very first (top-level)
                                                    class being empty. */
                                                    if class_depth_m1 < 0 {
                                                        break 'class_loop;
                                                    }

                                                    class_range_state = RANGE_NO; /* for processing the containing class */
                                                    class_op_state = CLASS_OP_OPERAND;
                                                    break 'class_continue;
                                                }

                                                /* Enter a non-empty class. */

                                                if class_start != core::ptr::null_mut() {
                                                    /* Represents that the class is an extended
                                                    class. */
                                                    *class_start |= CLASS_IS_ECLASS;
                                                    class_start = core::ptr::null_mut();
                                                }

                                                class_start = parsed_pattern;
                                                *parsed_pattern = if negate_class != 0 {
                                                    META_CLASS_NOT
                                                } else {
                                                    META_CLASS
                                                };
                                                parsed_pattern = parsed_pattern.add(1);
                                                class_range_state = RANGE_NO;
                                                class_op_state = CLASS_OP_EMPTY;
                                                class_mode_state = new_class_mode_state;
                                                class_depth_m1 += 1;
                                                if class_maxdepth_m1 < class_depth_m1 {
                                                    class_maxdepth_m1 = class_depth_m1;
                                                }
                                                /* Reset; no op seen yet at new depth. */
                                                *(*cb)
                                                    .class_op_used
                                                    .as_mut_ptr()
                                                    .offset(class_depth_m1 as isize) = 0;

                                                /* Implement the special start-of-class literal
                                                meaning of ']'. */
                                                if c == CHAR_RIGHT_SQUARE_BRACKET
                                                    && new_class_mode_state
                                                        != CLASS_MODE_PERL_EXT
                                                {
                                                    class_range_state = RANGE_OK_LITERAL;
                                                    class_op_state = CLASS_OP_OPERAND;
                                                    /* PARSED_LITERAL(c, parsed_pattern) */
                                                    *parsed_pattern = c;
                                                    parsed_pattern = parsed_pattern.add(1);
                                                    okquantifier = TRUE;
                                                    break 'class_continue;
                                                }

                                                continue 'class_loop; /* We have already loaded c with the next character */
                                            }
                                            /* Check for the end of the class. */
                                            else if c == CHAR_RIGHT_SQUARE_BRACKET
                                                || (c == CHAR_RIGHT_PARENTHESIS
                                                    && class_mode_state == CLASS_MODE_PERL_EXT)
                                            {
                                                /* In Perl extended mode, the ']' can only be used
                                                to match the opening '[', and ')' must match an
                                                opening parenthesis. */
                                                if class_mode_state == CLASS_MODE_PERL_EXT {
                                                    if c == CHAR_RIGHT_SQUARE_BRACKET
                                                        && class_depth_m1 != 0
                                                    {
                                                        errorcode = ERR(14);
                                                        ptr = ptr.sub(1); /* Correct the offset */
                                                        break 'failed;
                                                    }
                                                    if c == CHAR_RIGHT_PARENTHESIS
                                                        && class_depth_m1 < 1
                                                    {
                                                        errorcode = ERR(22);
                                                        break 'failed;
                                                    }
                                                }

                                                /* Check no trailing operator. */
                                                if class_op_state == CLASS_OP_OPERATOR {
                                                    errorcode = ERR(110);
                                                    break 'failed;
                                                }

                                                /* Check no empty expression for Perl extended
                                                expressions. */
                                                if class_mode_state == CLASS_MODE_PERL_EXT
                                                    && class_op_state == CLASS_OP_EMPTY
                                                {
                                                    errorcode = ERR(114);
                                                    break 'failed;
                                                }

                                                /* -] at the end of a class is a literal '-' */
                                                if class_range_state == RANGE_STARTED {
                                                    *parsed_pattern.offset(-1) = CHAR_MINUS;
                                                }

                                                *parsed_pattern = META_CLASS_END;
                                                parsed_pattern = parsed_pattern.add(1);

                                                class_depth_m1 -= 1;
                                                if class_depth_m1 < 0 {
                                                    /* Check for and consume ')' after '(?[...]'. */
                                                    if class_mode_state == CLASS_MODE_PERL_EXT {
                                                        if ptr >= ptrend
                                                            || *ptr as u32
                                                                != CHAR_RIGHT_PARENTHESIS
                                                        {
                                                            errorcode = ERR(115);
                                                            break 'failed;
                                                        }

                                                        ptr = ptr.add(1);
                                                    }

                                                    break 'class_loop;
                                                }

                                                class_range_state = RANGE_NO; /* for processing the containing class */
                                                class_op_state = CLASS_OP_OPERAND;
                                                if class_mode_state == CLASS_MODE_PERL_EXT_LEAF {
                                                    class_mode_state = CLASS_MODE_PERL_EXT;
                                                }
                                                /* The extended class flag has already been set for
                                                the parent class. */
                                                class_start = core::ptr::null_mut();
                                            }
                                            /* Handle a Perl set binary operator */
                                            else if class_mode_state == CLASS_MODE_PERL_EXT
                                                && (c == CHAR_PLUS
                                                    || c == CHAR_VERTICAL_LINE
                                                    || c == CHAR_MINUS
                                                    || c == CHAR_AMPERSAND
                                                    || c == CHAR_CIRCUMFLEX_ACCENT)
                                            {
                                                /* Check that there was a preceding operand. */
                                                if class_op_state != CLASS_OP_OPERAND {
                                                    errorcode = ERR(109);
                                                    break 'failed;
                                                }

                                                if class_start != core::ptr::null_mut() {
                                                    /* Represents that the class is an extended
                                                    class. */
                                                    *class_start |= CLASS_IS_ECLASS;
                                                    class_start = core::ptr::null_mut();
                                                }

                                                *parsed_pattern = if c == CHAR_PLUS {
                                                    META_ECLASS_OR
                                                } else if c == CHAR_VERTICAL_LINE {
                                                    META_ECLASS_OR
                                                } else if c == CHAR_MINUS {
                                                    META_ECLASS_SUB
                                                } else if c == CHAR_AMPERSAND {
                                                    META_ECLASS_AND
                                                } else {
                                                    META_ECLASS_XOR
                                                };
                                                parsed_pattern = parsed_pattern.add(1);
                                                class_range_state = RANGE_NO;
                                                class_op_state = CLASS_OP_OPERATOR;
                                            }
                                            /* Handle a Perl set unary operator */
                                            else if class_mode_state == CLASS_MODE_PERL_EXT
                                                && c == CHAR_EXCLAMATION_MARK
                                            {
                                                /* Check that the "!" has not got a preceding
                                                operand. */
                                                if class_op_state == CLASS_OP_OPERAND {
                                                    errorcode = ERR(113);
                                                    break 'failed;
                                                }

                                                if class_start != core::ptr::null_mut() {
                                                    /* Represents that the class is an extended
                                                    class. */
                                                    *class_start |= CLASS_IS_ECLASS;
                                                    class_start = core::ptr::null_mut();
                                                }

                                                *parsed_pattern = META_ECLASS_NOT;
                                                parsed_pattern = parsed_pattern.add(1);
                                                class_range_state = RANGE_NO;
                                                class_op_state = CLASS_OP_OPERATOR;
                                            }
                                            /* Handle a UTS#18 set operator */
                                            else if class_mode_state == CLASS_MODE_ALT_EXT
                                                && (c == CHAR_VERTICAL_LINE
                                                    || c == CHAR_MINUS
                                                    || c == CHAR_AMPERSAND
                                                    || c == CHAR_TILDE)
                                                && ptr < ptrend
                                                && *ptr as u32 == c
                                            {
                                                ptr = ptr.add(1);

                                                /* Check there isn't a triple-repetition. */
                                                if ptr < ptrend && *ptr as u32 == c {
                                                    while ptr < ptrend && *ptr as u32 == c {
                                                        ptr = ptr.add(1); /* Improve error offset. */
                                                    }
                                                    errorcode = ERR(108);
                                                    break 'failed;
                                                }

                                                /* Check for a preceding operand. */
                                                if class_op_state != CLASS_OP_OPERAND {
                                                    errorcode = ERR(109);
                                                    break 'failed;
                                                }

                                                /* Check for mixed precedence. Forbid [A--B&&C]. */
                                                if *(*cb)
                                                    .class_op_used
                                                    .as_mut_ptr()
                                                    .offset(class_depth_m1 as isize)
                                                    != 0
                                                    && *(*cb)
                                                        .class_op_used
                                                        .as_mut_ptr()
                                                        .offset(class_depth_m1 as isize)
                                                        != c as u8
                                                {
                                                    errorcode = ERR(111);
                                                    break 'failed;
                                                }

                                                if class_start != core::ptr::null_mut() {
                                                    /* Represents that the class is an extended
                                                    class. */
                                                    *class_start |= CLASS_IS_ECLASS;
                                                    class_start = core::ptr::null_mut();
                                                }

                                                /* Dangling '-' before an operator is a literal */
                                                if class_range_state == RANGE_STARTED {
                                                    *parsed_pattern.offset(-1) = CHAR_MINUS;
                                                }

                                                *parsed_pattern = if c == CHAR_VERTICAL_LINE {
                                                    META_ECLASS_OR
                                                } else if c == CHAR_MINUS {
                                                    META_ECLASS_SUB
                                                } else if c == CHAR_AMPERSAND {
                                                    META_ECLASS_AND
                                                } else {
                                                    META_ECLASS_XOR
                                                };
                                                parsed_pattern = parsed_pattern.add(1);
                                                class_range_state = RANGE_NO;
                                                class_op_state = CLASS_OP_OPERATOR;
                                                *(*cb)
                                                    .class_op_used
                                                    .as_mut_ptr()
                                                    .offset(class_depth_m1 as isize) = c as u8;
                                            }
                                            /* Handle escapes in a class */
                                            else if c == CHAR_BACKSLASH {
                                                tempptr = ptr;
                                                escape = _pcre2_check_escape_8(
                                                    &mut ptr,
                                                    ptrend,
                                                    &mut c,
                                                    &mut errorcode,
                                                    options,
                                                    xoptions,
                                                    (*cb).bracount,
                                                    TRUE,
                                                    cb,
                                                );

                                                if errorcode != 0 {
                                                    if (xoptions
                                                        & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL)
                                                        == 0
                                                        || class_mode_state
                                                            >= CLASS_MODE_PERL_EXT
                                                    {
                                                        break 'failed;
                                                    }
                                                    ptr = tempptr;
                                                    if ptr >= ptrend {
                                                        c = CHAR_BACKSLASH;
                                                    } else {
                                                        /* GETCHARINCTEST(c, ptr) */
                                                        c = *ptr as u32;
                                                        ptr = ptr.add(1);
                                                        if utf != 0 && c >= 0xc0u32 {
                                                            let r = getutf8inc(c, ptr);
                                                            c = r.0;
                                                            ptr = r.1;
                                                        }
                                                    }
                                                    escape = 0; /* Treat as literal character */
                                                }

                                                'cls_esc_sw: {
                                                    if escape == 0 {
                                                        /* Escaped character code point is in c */
                                                        char_is_literal = FALSE;
                                                        break 'class_literal; /* (a few lines above) */
                                                    }
                                                    if escape == ESC_b {
                                                        c = CHAR_BS; /* \b is backspace in a class */
                                                        char_is_literal = FALSE;
                                                        break 'class_literal;
                                                    }
                                                    if escape == ESC_k {
                                                        c = CHAR_k; /* \k is not special in a class, just like \g */
                                                        char_is_literal = FALSE;
                                                        break 'class_literal;
                                                    }
                                                    if escape == ESC_Q {
                                                        inescq = TRUE; /* Enter literal mode */
                                                        break 'class_continue;
                                                    }
                                                    if escape == ESC_E {
                                                        /* Ignore orphan \E */
                                                        break 'class_continue;
                                                    }
                                                    if escape == ESC_B
                                                        || escape == ESC_R
                                                        || escape == ESC_X
                                                    {
                                                        /* Always an error in a class */
                                                        errorcode = ERR(7);
                                                        break 'failed;
                                                    }
                                                    if escape == ESC_N {
                                                        /* Not permitted by Perl either */
                                                        errorcode = ERR(71);
                                                        break 'failed;
                                                    }
                                                    if escape == ESC_H
                                                        || escape == ESC_h
                                                        || escape == ESC_V
                                                        || escape == ESC_v
                                                    {
                                                        *parsed_pattern =
                                                            META_ESCAPE + escape as u32;
                                                        parsed_pattern = parsed_pattern.add(1);
                                                        break 'cls_esc_sw;
                                                    }

                                                    /* These escapes may be converted to Unicode
                                                    property tests when PCRE2_UCP is set. */

                                                    if escape == ESC_d
                                                        || escape == ESC_D
                                                        || escape == ESC_s
                                                        || escape == ESC_S
                                                        || escape == ESC_w
                                                        || escape == ESC_W
                                                    {
                                                        parsed_pattern = handle_escdsw(
                                                            escape,
                                                            parsed_pattern,
                                                            options,
                                                            xoptions,
                                                        );
                                                        break 'cls_esc_sw;
                                                    }

                                                    /* Explicit Unicode property matching */

                                                    if escape == ESC_P || escape == ESC_p {
                                                        let mut negated: BOOL = 0;
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
                                                        ) == 0
                                                        {
                                                            break 'failed;
                                                        }

                                                        /* In caseless matching, particular
                                                        characteristics Lu, Ll, and Lt get
                                                        converted to the general characteristic
                                                        L&. */

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
                                                            escape = if escape == ESC_P {
                                                                ESC_p
                                                            } else {
                                                                ESC_P
                                                            };
                                                        }
                                                        *parsed_pattern =
                                                            META_ESCAPE + escape as u32;
                                                        parsed_pattern = parsed_pattern.add(1);
                                                        *parsed_pattern =
                                                            ((ptype as u32) << 16) | pdata as u32;
                                                        parsed_pattern = parsed_pattern.add(1);
                                                        break 'cls_esc_sw; /* End \P and \p */
                                                    }

                                                    /* All others are not allowed in a class:
                                                    default, ESC_A, ESC_Z, ESC_z, ESC_G, ESC_K,
                                                    ESC_C */
                                                    errorcode = ERR(7);
                                                    break 'failed;
                                                }

                                                /* All the switch-cases above which end in "break"
                                                describe a set of characters. None may start a
                                                range. */

                                                if class_range_state == RANGE_STARTED {
                                                    errorcode = ERR(50);
                                                    break 'failed;
                                                }

                                                /* Perl gives a warning unless the hyphen following
                                                a multi-character escape is the last character in
                                                the class. PCRE throws an error. */

                                                if class_range_state == RANGE_FORBID_STARTED {
                                                    ptr = class_range_forbid_ptr;
                                                    errorcode = ERR(50);
                                                    break 'failed;
                                                }

                                                /* Disallow implicit union in Perl extended
                                                classes. */

                                                if class_op_state == CLASS_OP_OPERAND
                                                    && class_mode_state == CLASS_MODE_PERL_EXT
                                                {
                                                    errorcode = ERR(113);
                                                    break 'failed;
                                                }

                                                class_range_state = RANGE_FORBID_NO;
                                                class_op_state = CLASS_OP_OPERAND;
                                            }
                                            /* Forbid unescaped literals, and the special meaning
                                            of '-', inside a Perl extended class. */
                                            else if class_mode_state == CLASS_MODE_PERL_EXT {
                                                errorcode = ERR(116);
                                                break 'failed;
                                            }
                                            /* Handle potential start of range */
                                            else if c == CHAR_MINUS
                                                && class_range_state >= RANGE_OK_ESCAPED
                                            {
                                                *parsed_pattern =
                                                    if class_range_state == RANGE_OK_LITERAL {
                                                        META_RANGE_LITERAL
                                                    } else {
                                                        META_RANGE_ESCAPED
                                                    };
                                                parsed_pattern = parsed_pattern.add(1);
                                                class_range_state = RANGE_STARTED;
                                            }
                                            /* Handle forbidden start of range */
                                            else if c == CHAR_MINUS
                                                && class_range_state == RANGE_FORBID_NO
                                            {
                                                *parsed_pattern = CHAR_MINUS;
                                                parsed_pattern = parsed_pattern.add(1);
                                                class_range_state = RANGE_FORBID_STARTED;
                                                class_range_forbid_ptr = ptr;
                                            }
                                            /* Handle a literal character */
                                            else {
                                                break 'class_literal;
                                            }

                                            break 'class_continue;
                                        }

                                        /* CLASS_LITERAL: */

                                        /* Disallow implicit union in Perl extended classes. */

                                        if class_op_state == CLASS_OP_OPERAND
                                            && class_mode_state == CLASS_MODE_PERL_EXT
                                        {
                                            errorcode = ERR(113);
                                            break 'failed;
                                        }

                                        if class_range_state == RANGE_STARTED {
                                            if c == *parsed_pattern.offset(-2) {
                                                /* Optimize one-char range */
                                                parsed_pattern = parsed_pattern.sub(1);
                                            } else if *parsed_pattern.offset(-2) > c {
                                                /* Check range is in order */
                                                errorcode = ERR(8);
                                                break 'failed;
                                            } else {
                                                if char_is_literal == 0
                                                    && *parsed_pattern.offset(-1)
                                                        == META_RANGE_LITERAL
                                                {
                                                    *parsed_pattern.offset(-1) =
                                                        META_RANGE_ESCAPED;
                                                }
                                                /* PARSED_LITERAL(c, parsed_pattern) */
                                                *parsed_pattern = c;
                                                parsed_pattern = parsed_pattern.add(1);
                                                okquantifier = TRUE;
                                            }
                                            class_range_state = RANGE_NO;
                                            class_op_state = CLASS_OP_OPERAND;
                                        } else if class_range_state == RANGE_FORBID_STARTED {
                                            ptr = class_range_forbid_ptr;
                                            errorcode = ERR(50);
                                            break 'failed;
                                        } else {
                                            /* Potential start of range */
                                            class_range_state = if char_is_literal != 0 {
                                                RANGE_OK_LITERAL
                                            } else {
                                                RANGE_OK_ESCAPED
                                            };
                                            class_op_state = CLASS_OP_OPERAND;
                                            /* PARSED_LITERAL(c, parsed_pattern) */
                                            *parsed_pattern = c;
                                            parsed_pattern = parsed_pattern.add(1);
                                            okquantifier = TRUE;
                                        }
                                    }

                                    /* CLASS_CONTINUE: */
                                    /* Proceed to next thing in the class. */

                                    if ptr >= ptrend {
                                        if class_mode_state == CLASS_MODE_PERL_EXT
                                            && class_depth_m1 > 0
                                        {
                                            errorcode = ERR(14); /* Missing terminating ')' */
                                        }
                                        if class_mode_state == CLASS_MODE_ALT_EXT
                                            && class_depth_m1 == 0
                                            && class_maxdepth_m1 == 1
                                        {
                                            errorcode = ERR(112); /* Missing terminating ']', but we saw '[ [ ]...' */
                                        } else {
                                            errorcode = ERR(6); /* Missing terminating ']' */
                                        }
                                        break 'failed;
                                    }
                                    /* GETCHARINCTEST(c, ptr) */
                                    c = *ptr as u32;
                                    ptr = ptr.add(1);
                                    if utf != 0 && c >= 0xc0u32 {
                                        let r = getutf8inc(c, ptr);
                                        c = r.0;
                                        ptr = r.1;
                                    }
                                } /* End of class-processing loop */

                                break 'sw; /* End of character class */
                                }
                                /* ---- Opening parenthesis ---- */
                                if ptr >= ptrend {
                                    break 'unclosed_parenthesis;
                                }

                                /* If ( is not followed by ? it is either a capture or a special
                                verb or an alpha assertion or a positive non-atomic lookahead. */

                                if *ptr as u32 != CHAR_QUESTION_MARK {
                                    let mut vn: *const c_char;

                                    /* Handle capturing brackets (or non-capturing if auto-capture
                                    is turned off). */

                                    if *ptr as u32 != CHAR_ASTERISK {
                                        nest_depth += 1;
                                        if (options & PCRE2_NO_AUTO_CAPTURE) == 0 {
                                            if (*cb).bracount >= MAX_GROUP_NUMBER {
                                                errorcode = ERR(97);
                                                break 'failed;
                                            }
                                            (*cb).bracount += 1;
                                            *parsed_pattern = META_CAPTURE | (*cb).bracount;
                                            parsed_pattern = parsed_pattern.add(1);
                                        } else {
                                            *parsed_pattern = META_NOCAPTURE;
                                            parsed_pattern = parsed_pattern.add(1);
                                        }
                                    }
                                    /* Do nothing for (* followed by end of pattern or ) so it
                                    gives a "bad quantifier" error rather than "(*MARK) must have
                                    an argument". */
                                    else if ptrend.offset_from(ptr) <= 1 || {
                                        c = *ptr.add(1) as u32;
                                        c == CHAR_RIGHT_PARENTHESIS
                                    } {
                                        break 'sw;
                                    }
                                    /* Handle "alpha assertions" such as (*pla:...). */
                                    else if CHMAX_255(c)
                                        && (*(*cb).ctypes.add(c as usize) & ctype_lcletter) != 0
                                    {
                                        let meta: u32;

                                        vn = alasnames.as_ptr() as *const c_char;
                                        if read_name(
                                            &mut ptr,
                                            ptrend,
                                            utf,
                                            0,
                                            &mut offset,
                                            &mut name,
                                            &mut namelen,
                                            &mut errorcode,
                                            cb,
                                        ) == 0
                                        {
                                            break 'failed;
                                        }
                                        if ptr >= ptrend {
                                            break 'unclosed_parenthesis;
                                        }
                                        if *ptr as u32 != CHAR_COLON {
                                            errorcode = ERR(95); /* Malformed */
                                            break 'failed_forward;
                                        }

                                        /* Scan the table of alpha assertion names */

                                        i = 0;
                                        while i < alascount {
                                            if namelen == alasmeta[i as usize].len
                                                && crate::string_utils::_pcre2_strncmp_c8_8(
                                                    name,
                                                    vn,
                                                    namelen as usize,
                                                ) == 0
                                            {
                                                break;
                                            }
                                            vn = vn.add((alasmeta[i as usize].len + 1) as usize);
                                            i += 1;
                                        }

                                        if i >= alascount {
                                            errorcode = ERR(95); /* Alpha assertion not recognized */
                                            break 'failed;
                                        }

                                        /* Check for expecting an assertion condition. If so, only
                                        atomic lookaround assertions are valid. */

                                        meta = alasmeta[i as usize].meta;
                                        if prev_expect_cond_assert > 0
                                            && (meta < META_LOOKAHEAD || meta > META_LOOKBEHINDNOT)
                                        {
                                            errorcode = ERR(28); /* Atomic assertion expected */
                                            break 'failed;
                                        }

                                        /* The lookaround alphabetic synonyms can mostly be handled
                                        by jumping to the code that handles the traditional
                                        symbolic forms. */

                                        if meta == META_ATOMIC {
                                            break 'l_atomic_group;
                                        } else if meta == META_LOOKAHEAD {
                                            break 'l_positive_look_ahead;
                                        } else if meta == META_LOOKAHEAD_NA {
                                            break 'l_positive_nonatomic_look_ahead;
                                        } else if meta == META_LOOKAHEADNOT {
                                            break 'l_negative_look_ahead;
                                        } else if meta == META_SCS {
                                            ptr = ptr.add(1);
                                            *parsed_pattern = META_SCS;
                                            parsed_pattern = parsed_pattern.add(1);

                                            parsed_pattern = parse_capture_list(
                                                &mut ptr,
                                                ptrend,
                                                utf,
                                                parsed_pattern,
                                                0,
                                                &mut errorcode,
                                                cb,
                                            );
                                            if parsed_pattern == core::ptr::null_mut() {
                                                break 'failed;
                                            }
                                            break 'l_post_assertion;
                                        } else if meta == META_LOOKBEHIND
                                            || meta == META_LOOKBEHINDNOT
                                            || meta == META_LOOKBEHIND_NA
                                        {
                                            *parsed_pattern = meta;
                                            parsed_pattern = parsed_pattern.add(1);
                                            ptr = ptr.sub(1);
                                            break 'l_post_lookbehind;
                                        } else if meta == META_SCRIPT_RUN
                                            || meta == META_ATOMIC_SCRIPT_RUN
                                        {
                                            /* The script run facilities are handled here. */
                                            *parsed_pattern = META_SCRIPT_RUN;
                                            parsed_pattern = parsed_pattern.add(1);
                                            nest_depth += 1;
                                            ptr = ptr.add(1);
                                            if meta == META_ATOMIC_SCRIPT_RUN {
                                                *parsed_pattern = META_ATOMIC;
                                                parsed_pattern = parsed_pattern.add(1);
                                                if top_nest == core::ptr::null_mut() {
                                                    top_nest =
                                                        (*cb).start_workspace as *mut nest_save;
                                                } else {
                                                    top_nest = top_nest.add(1);
                                                    if top_nest >= end_nests {
                                                        errorcode = ERR(84);
                                                        break 'failed;
                                                    }
                                                }
                                                (*top_nest).nest_depth = nest_depth;
                                                (*top_nest).flags = NSF_ATOMICSR;
                                                (*top_nest).options =
                                                    options & PARSE_TRACKED_OPTIONS;
                                                (*top_nest).xoptions =
                                                    xoptions & PARSE_TRACKED_EXTRA_OPTIONS;
                                            }
                                            /* break out of switch(meta) */
                                        } else {
                                            errorcode = ERR(89); /* Unknown code; should never occur because */
                                            break 'failed; /* the meta values come from a table above. */
                                        }
                                    }
                                    /* ---- Handle (*VERB) and (*VERB:NAME) ---- */
                                    else {
                                        vn = verbnames.as_ptr() as *const c_char;
                                        if read_name(
                                            &mut ptr,
                                            ptrend,
                                            utf,
                                            0,
                                            &mut offset,
                                            &mut name,
                                            &mut namelen,
                                            &mut errorcode,
                                            cb,
                                        ) == 0
                                        {
                                            break 'failed;
                                        }
                                        if ptr >= ptrend
                                            || (*ptr as u32 != CHAR_COLON
                                                && *ptr as u32 != CHAR_RIGHT_PARENTHESIS)
                                        {
                                            errorcode = ERR(60); /* Malformed */
                                            break 'failed;
                                        }

                                        /* Scan the table of verb names */

                                        i = 0;
                                        while i < verbcount {
                                            if namelen == verbs[i as usize].len
                                                && crate::string_utils::_pcre2_strncmp_c8_8(
                                                    name,
                                                    vn,
                                                    namelen as usize,
                                                ) == 0
                                            {
                                                break;
                                            }
                                            vn = vn.add((verbs[i as usize].len + 1) as usize);
                                            i += 1;
                                        }

                                        if i >= verbcount {
                                            errorcode = ERR(60); /* Verb not recognized */
                                            break 'failed;
                                        }

                                        /* An empty argument is treated as no argument. */

                                        if *ptr as u32 == CHAR_COLON
                                            && ptr.add(1) < ptrend
                                            && *ptr.add(1) as u32 == CHAR_RIGHT_PARENTHESIS
                                        {
                                            ptr = ptr.add(1); /* Advance to the closing parens */
                                        }

                                        /* Check for mandatory non-empty argument; this is (*MARK) */

                                        if verbs[i as usize].has_arg > 0
                                            && *ptr as u32 != CHAR_COLON
                                        {
                                            errorcode = ERR(66);
                                            break 'failed;
                                        }

                                        /* Remember where this verb, possibly with a preceding
                                        (*MARK), starts, for handling quantified (*ACCEPT). */

                                        verbstartptr = parsed_pattern;
                                        okquantifier =
                                            (verbs[i as usize].meta == META_ACCEPT) as BOOL;

                                        /* It appears that Perl allows any characters whatsoever,
                                        other than a closing parenthesis, to appear in arguments
                                        ("names"). */

                                        let sep = *ptr as u32;
                                        ptr = ptr.add(1);
                                        if sep == CHAR_COLON {
                                            /* Skip past : or ) */
                                            /* Some optional arguments can be treated as a
                                            preceding (*MARK) */

                                            if verbs[i as usize].has_arg < 0 {
                                                add_after_mark = verbs[i as usize].meta;
                                                *parsed_pattern = META_MARK;
                                                parsed_pattern = parsed_pattern.add(1);
                                            }
                                            /* The remaining verbs with arguments (except *MARK)
                                            need a different opcode. */
                                            else {
                                                *parsed_pattern = verbs[i as usize].meta
                                                    + (if verbs[i as usize].meta != META_MARK {
                                                        0x00010000u32
                                                    } else {
                                                        0
                                                    });
                                                parsed_pattern = parsed_pattern.add(1);
                                            }

                                            /* Set up for reading the name in the main loop. */

                                            verblengthptr = parsed_pattern;
                                            parsed_pattern = parsed_pattern.add(1);
                                            verbnamestart = ptr;
                                            inverbname = TRUE;
                                        } else {
                                            /* No verb "name" argument */
                                            *parsed_pattern = verbs[i as usize].meta;
                                            parsed_pattern = parsed_pattern.add(1);
                                        }
                                    } /* End of (*VERB) handling */
                                    break 'sw; /* Done with this parenthesis */
                                } /* End of groups that don't start with (? */

                                /* ---- Items starting (? ---- */

                                ptr = ptr.add(1);
                                if ptr >= ptrend {
                                    break 'unclosed_parenthesis;
                                }

                                /* switch(*ptr) dispatch */
                                match *ptr as u32 {
                                    CHAR_P => break 'l_q_p,
                                    CHAR_R => break 'l_q_r,
                                    CHAR_PLUS => break 'l_q_plus,
                                    CHAR_0 | CHAR_1 | CHAR_2 | CHAR_3 | CHAR_4 | CHAR_5
                                    | CHAR_6 | CHAR_7 | CHAR_8 | CHAR_9 => break 'l_q_digit,
                                    CHAR_AMPERSAND => break 'l_q_ampersand,
                                    CHAR_C => break 'l_q_c,
                                    CHAR_LEFT_PARENTHESIS => break 'l_q_lparen,
                                    CHAR_GREATER_THAN_SIGN => break 'l_q_gt,
                                    CHAR_EQUALS_SIGN => break 'l_q_eq,
                                    CHAR_ASTERISK => break 'l_q_asterisk,
                                    CHAR_EXCLAMATION_MARK => break 'l_q_bang,
                                    CHAR_LESS_THAN_SIGN => break 'l_q_lt,
                                    CHAR_APOSTROPHE => break 'l_q_apostrophe,
                                    CHAR_LEFT_SQUARE_BRACKET => break 'l_q_lbracket,
                                    _ => break 'l_q_default,
                                }
                                }
                                /* ---- default case after (? ---- */
                                if *ptr as u32 == CHAR_MINUS
                                    && ptrend.offset_from(ptr) > 1
                                    && IS_DIGIT(*ptr.add(1) as u32)
                                {
                                    break 'l_recursion_bynumber; /* The + case is handled by CHAR_PLUS */
                                }

                                /* We now have either (?| or a (possibly empty) option setting,
                                optionally followed by a non-capturing group. */

                                nest_depth += 1;
                                if top_nest == core::ptr::null_mut() {
                                    top_nest = (*cb).start_workspace as *mut nest_save;
                                } else {
                                    top_nest = top_nest.add(1);
                                    if top_nest >= end_nests {
                                        errorcode = ERR(84);
                                        break 'failed;
                                    }
                                }
                                (*top_nest).nest_depth = nest_depth;
                                (*top_nest).flags = 0;
                                (*top_nest).options = options & PARSE_TRACKED_OPTIONS;
                                (*top_nest).xoptions = xoptions & PARSE_TRACKED_EXTRA_OPTIONS;

                                /* Start of non-capturing group that resets the capture count for
                                each branch. */

                                if *ptr as u32 == CHAR_VERTICAL_LINE {
                                    (*top_nest).reset_group = (*cb).bracount as u16;
                                    (*top_nest).max_group = (*cb).bracount as u16;
                                    (*top_nest).flags |= NSF_RESET;
                                    (*cb).external_flags |= PCRE2_DUPCAPUSED;
                                    *parsed_pattern = META_NOCAPTURE;
                                    parsed_pattern = parsed_pattern.add(1);
                                    ptr = ptr.add(1);
                                }
                                /* Scan for options imnrsxJU to be set or unset. */
                                else {
                                    let mut hyphenok: BOOL = TRUE;
                                    let oldoptions: u32 = options;
                                    let oldxoptions: u32 = xoptions;

                                    (*top_nest).reset_group = 0;
                                    (*top_nest).max_group = 0;
                                    unset = 0;
                                    set = 0;
                                    optset = &mut set;
                                    xunset = 0;
                                    xset = 0;
                                    xoptset = &mut xset;

                                    /* ^ at the start unsets irmnsx and disables the subsequent use
                                    of - */

                                    if ptr < ptrend && *ptr as u32 == CHAR_CIRCUMFLEX_ACCENT {
                                        options &= !(PCRE2_CASELESS
                                            | PCRE2_MULTILINE
                                            | PCRE2_NO_AUTO_CAPTURE
                                            | PCRE2_DOTALL
                                            | PCRE2_EXTENDED
                                            | PCRE2_EXTENDED_MORE);
                                        xoptions &= !(PCRE2_EXTRA_CASELESS_RESTRICT);
                                        hyphenok = FALSE;
                                        ptr = ptr.add(1);
                                    }

                                    while ptr < ptrend
                                        && *ptr as u32 != CHAR_RIGHT_PARENTHESIS
                                        && *ptr as u32 != CHAR_COLON
                                    {
                                        let optc = *ptr as u32;
                                        ptr = ptr.add(1);
                                        'opt_sw: {
                                            if optc == CHAR_MINUS {
                                                if hyphenok == 0 {
                                                    errorcode = ERR(94);
                                                    break 'failed;
                                                }
                                                optset = &mut unset;
                                                xoptset = &mut xunset;
                                                hyphenok = FALSE;
                                                break 'opt_sw;
                                            }

                                            /* There are some two-character sequences that start
                                            with 'a'. */

                                            if optc == CHAR_a {
                                                if ptr < ptrend {
                                                    if *ptr as u32 == CHAR_D {
                                                        *xoptset |= PCRE2_EXTRA_ASCII_BSD;
                                                        ptr = ptr.add(1);
                                                        break 'opt_sw;
                                                    }
                                                    if *ptr as u32 == CHAR_P {
                                                        *xoptset |= PCRE2_EXTRA_ASCII_POSIX
                                                            | PCRE2_EXTRA_ASCII_DIGIT;
                                                        ptr = ptr.add(1);
                                                        break 'opt_sw;
                                                    }
                                                    if *ptr as u32 == CHAR_S {
                                                        *xoptset |= PCRE2_EXTRA_ASCII_BSS;
                                                        ptr = ptr.add(1);
                                                        break 'opt_sw;
                                                    }
                                                    if *ptr as u32 == CHAR_T {
                                                        *xoptset |= PCRE2_EXTRA_ASCII_DIGIT;
                                                        ptr = ptr.add(1);
                                                        break 'opt_sw;
                                                    }
                                                    if *ptr as u32 == CHAR_W {
                                                        *xoptset |= PCRE2_EXTRA_ASCII_BSW;
                                                        ptr = ptr.add(1);
                                                        break 'opt_sw;
                                                    }
                                                }
                                                *xoptset |= PCRE2_EXTRA_ASCII_BSD
                                                    | PCRE2_EXTRA_ASCII_BSS
                                                    | PCRE2_EXTRA_ASCII_BSW
                                                    | PCRE2_EXTRA_ASCII_DIGIT
                                                    | PCRE2_EXTRA_ASCII_POSIX;
                                                break 'opt_sw;
                                            }

                                            if optc == CHAR_J {
                                                /* Record that it changed in the external options */
                                                *optset |= PCRE2_DUPNAMES;
                                                (*cb).external_flags |= PCRE2_JCHANGED;
                                                break 'opt_sw;
                                            }

                                            if optc == CHAR_i {
                                                *optset |= PCRE2_CASELESS;
                                                break 'opt_sw;
                                            }
                                            if optc == CHAR_m {
                                                *optset |= PCRE2_MULTILINE;
                                                break 'opt_sw;
                                            }
                                            if optc == CHAR_n {
                                                *optset |= PCRE2_NO_AUTO_CAPTURE;
                                                break 'opt_sw;
                                            }
                                            if optc == CHAR_r {
                                                *xoptset |= PCRE2_EXTRA_CASELESS_RESTRICT;
                                                break 'opt_sw;
                                            }
                                            if optc == CHAR_s {
                                                *optset |= PCRE2_DOTALL;
                                                break 'opt_sw;
                                            }
                                            if optc == CHAR_U {
                                                *optset |= PCRE2_UNGREEDY;
                                                break 'opt_sw;
                                            }

                                            /* If x appears twice it sets the extended extended
                                            option. */

                                            if optc == CHAR_x {
                                                *optset |= PCRE2_EXTENDED;
                                                if ptr < ptrend && *ptr as u32 == CHAR_x {
                                                    *optset |= PCRE2_EXTENDED_MORE;
                                                    ptr = ptr.add(1);
                                                }
                                                break 'opt_sw;
                                            }

                                            errorcode = ERR(11);
                                            break 'failed;
                                        }
                                    }

                                    /* If we are setting extended without extended-more, ensure
                                    that any existing extended-more gets unset. Also, unsetting
                                    extended must also unset extended-more. */

                                    if (set & (PCRE2_EXTENDED | PCRE2_EXTENDED_MORE))
                                        == PCRE2_EXTENDED
                                        || (unset & PCRE2_EXTENDED) != 0
                                    {
                                        unset |= PCRE2_EXTENDED_MORE;
                                    }

                                    options = (options | set) & (!unset);
                                    xoptions = (xoptions | xset) & (!xunset);

                                    /* If the options ended with ')' this is not the start of a
                                    nested group with option changes, so the options change at this
                                    level. */

                                    if ptr >= ptrend {
                                        break 'unclosed_parenthesis;
                                    }
                                    let endc = *ptr as u32;
                                    ptr = ptr.add(1);
                                    if endc == CHAR_RIGHT_PARENTHESIS {
                                        nest_depth -= 1; /* This is not a nested group after all. */
                                        if top_nest > (*cb).start_workspace as *mut nest_save
                                            && (*top_nest.sub(1)).nest_depth == nest_depth
                                        {
                                            top_nest = top_nest.sub(1);
                                        } else {
                                            (*top_nest).nest_depth = nest_depth;
                                        }
                                    } else {
                                        *parsed_pattern = META_NOCAPTURE;
                                        parsed_pattern = parsed_pattern.add(1);
                                    }

                                    /* If nothing changed, no need to record. */

                                    if options != oldoptions || xoptions != oldxoptions {
                                        *parsed_pattern = META_OPTIONS;
                                        parsed_pattern = parsed_pattern.add(1);
                                        *parsed_pattern = options;
                                        parsed_pattern = parsed_pattern.add(1);
                                        *parsed_pattern = xoptions;
                                        parsed_pattern = parsed_pattern.add(1);
                                    }
                                } /* End options processing */
                                break 'sw; /* End default case after (? */
                                }
                                /* ---- Python syntax support ---- */
                                ptr = ptr.add(1);
                                if ptr >= ptrend {
                                    break 'unclosed_parenthesis;
                                }

                                /* (?P<name> is the same as (?<name>, which defines a named
                                group. */

                                if *ptr as u32 == CHAR_LESS_THAN_SIGN {
                                    terminator = CHAR_GREATER_THAN_SIGN;
                                    break 'l_define_name;
                                }

                                /* (?P>name) is the same as (?&name), which is a recursion or
                                subroutine call. */

                                if *ptr as u32 == CHAR_GREATER_THAN_SIGN {
                                    break 'l_recurse_by_name;
                                }

                                /* (?P=name) is the same as \k<name>, a back reference by name.
                                Anything else after (?P is an error. */

                                if *ptr as u32 != CHAR_EQUALS_SIGN {
                                    errorcode = ERR(41);
                                    break 'failed_forward;
                                }
                                if read_name(
                                    &mut ptr,
                                    ptrend,
                                    utf,
                                    CHAR_RIGHT_PARENTHESIS,
                                    &mut offset,
                                    &mut name,
                                    &mut namelen,
                                    &mut errorcode,
                                    cb,
                                ) == 0
                                {
                                    break 'failed;
                                }
                                *parsed_pattern = META_BACKREF_BYNAME;
                                parsed_pattern = parsed_pattern.add(1);
                                *parsed_pattern = namelen;
                                parsed_pattern = parsed_pattern.add(1);
                                /* PUTOFFSET(offset, parsed_pattern) */
                                *parsed_pattern = (offset >> 32) as u32;
                                parsed_pattern = parsed_pattern.add(1);
                                *parsed_pattern = (offset & 0xffffffff) as u32;
                                parsed_pattern = parsed_pattern.add(1);
                                okquantifier = TRUE;
                                break 'sw; /* End of (?P processing */
                                }
                                /* ---- Recursion/subroutine calls by number ---- */
                                i = 0; /* (?R) == (?R0) */
                                ptr = ptr.add(1);
                                if ptr >= ptrend
                                    || (*ptr as u32 != CHAR_RIGHT_PARENTHESIS
                                        && *ptr as u32 != CHAR_LEFT_PARENTHESIS)
                                {
                                    errorcode = ERR(58);
                                    break 'failed;
                                }
                                terminator = CHAR_NUL;
                                break 'l_set_recursion;
                                }
                                /* An item starting (?- followed by a digit comes here via the
                                "default" case because (?- followed by a non-digit is an options
                                setting. */
                                if ptr.add(1) >= ptrend {
                                    ptr = ptr.add(1);
                                    break 'unclosed_parenthesis;
                                }
                                if !IS_DIGIT(*ptr.add(1) as u32) {
                                    errorcode = ERR(29); /* Missing number */
                                    ptr = ptr.add(1);
                                    break 'failed_forward;
                                }
                                /* Fall through */
                                }
                                /* case CHAR_0 .. CHAR_9 : fall through */
                                }
                                /* RECURSION_BYNUMBER: */
                                if read_number(
                                    &mut ptr,
                                    ptrend,
                                    if IS_DIGIT(*ptr as u32) {
                                        -1
                                    } else {
                                        (*cb).bracount as i32
                                    }, /* + and - are relative */
                                    MAX_GROUP_NUMBER,
                                    ERR(61) as u32,
                                    &mut i,
                                    &mut errorcode,
                                ) == 0
                                {
                                    break 'failed;
                                }
                                terminator = CHAR_NUL;
                                /* Fall through to SET_RECURSION */
                                }
                                /* SET_RECURSION: */
                                *parsed_pattern = META_RECURSE | i as u32;
                                parsed_pattern = parsed_pattern.add(1);
                                offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                                /* End of recursive call by number handling */
                                break 'l_read_recursion_arguments;
                                }
                                /* ---- Recursion/subroutine calls by name ---- */
                                /* case CHAR_AMPERSAND: fall through */
                                }
                                /* RECURSE_BY_NAME: */
                                if read_name(
                                    &mut ptr,
                                    ptrend,
                                    utf,
                                    0,
                                    &mut offset,
                                    &mut name,
                                    &mut namelen,
                                    &mut errorcode,
                                    cb,
                                ) == 0
                                {
                                    break 'failed;
                                }
                                *parsed_pattern = META_RECURSE_BYNAME;
                                parsed_pattern = parsed_pattern.add(1);
                                *parsed_pattern = namelen;
                                parsed_pattern = parsed_pattern.add(1);
                                terminator = CHAR_NUL;
                                /* Fall through to READ_RECURSION_ARGUMENTS */
                                }
                                /* READ_RECURSION_ARGUMENTS: */
                                /* PUTOFFSET(offset, parsed_pattern) */
                                *parsed_pattern = (offset >> 32) as u32;
                                parsed_pattern = parsed_pattern.add(1);
                                *parsed_pattern = (offset & 0xffffffff) as u32;
                                parsed_pattern = parsed_pattern.add(1);
                                okquantifier = TRUE;

                                /* Arguments are not supported for \g construct. */
                                if terminator != CHAR_NUL {
                                    break 'sw;
                                }

                                if ptr < ptrend && *ptr as u32 == CHAR_LEFT_PARENTHESIS {
                                    parsed_pattern = parse_capture_list(
                                        &mut ptr,
                                        ptrend,
                                        utf,
                                        parsed_pattern,
                                        offset,
                                        &mut errorcode,
                                        cb,
                                    );
                                    if parsed_pattern == core::ptr::null_mut() {
                                        break 'failed;
                                    }
                                }

                                if ptr >= ptrend || *ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                                    break 'unclosed_parenthesis;
                                }

                                ptr = ptr.add(1);
                                break 'sw;
                                }
                                /* ---- Callout with numerical or string argument ---- */
                                if (xoptions & PCRE2_EXTRA_NEVER_CALLOUT) != 0 {
                                    ptr = ptr.add(1);
                                    errorcode = ERR(103);
                                    break 'failed;
                                }

                                ptr = ptr.add(1);
                                if ptr >= ptrend {
                                    break 'unclosed_parenthesis;
                                }

                                /* If the previous item was a condition starting (?(? an assertion,
                                optionally preceded by a callout, is expected. */

                                expect_cond_assert = prev_expect_cond_assert - 1;

                                /* If previous_callout is not NULL, it means this follows a previous
                                callout. */

                                if previous_callout != core::ptr::null_mut()
                                    && (options & PCRE2_AUTO_CALLOUT) != 0
                                    && previous_callout == parsed_pattern.offset(-4)
                                    && *parsed_pattern.offset(-1) == 255
                                {
                                    parsed_pattern = previous_callout;
                                }

                                /* Save for updating next pattern item length, and skip one item
                                before completing. */

                                previous_callout = parsed_pattern;
                                after_manual_callout = 1;

                                /* Handle a string argument; specific delimiter is required. */

                                if *ptr as u32 != CHAR_RIGHT_PARENTHESIS
                                    && !IS_DIGIT(*ptr as u32)
                                {
                                    let calloutlength: PCRE2_SIZE;
                                    let startptr: PCRE2_SPTR = ptr;

                                    delimiter = 0;
                                    i = 0;
                                    while _pcre2_callout_start_delims_8[i as usize] != 0 {
                                        if *ptr as u32
                                            == _pcre2_callout_start_delims_8[i as usize]
                                        {
                                            delimiter =
                                                _pcre2_callout_end_delims_8[i as usize];
                                            break;
                                        }
                                        i += 1;
                                    }
                                    if delimiter == 0 {
                                        errorcode = ERR(82);
                                        break 'failed_forward;
                                    }

                                    *parsed_pattern = META_CALLOUT_STRING;
                                    parsed_pattern = parsed_pattern.add(3); /* Skip pattern info */

                                    loop {
                                        ptr = ptr.add(1);
                                        if ptr >= ptrend {
                                            errorcode = ERR(81);
                                            ptr = startptr; /* To give a more useful message */
                                            break 'failed;
                                        }
                                        if *ptr as u32 == delimiter {
                                            ptr = ptr.add(1);
                                            if ptr >= ptrend || *ptr as u32 != delimiter {
                                                break;
                                            }
                                        }
                                    }

                                    calloutlength = ptr.offset_from(startptr) as PCRE2_SIZE;
                                    if calloutlength > u32::MAX as PCRE2_SIZE {
                                        errorcode = ERR(72);
                                        break 'failed;
                                    }
                                    *parsed_pattern = calloutlength as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                    offset =
                                        startptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                                    /* PUTOFFSET(offset, parsed_pattern) */
                                    *parsed_pattern = (offset >> 32) as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                    *parsed_pattern = (offset & 0xffffffff) as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                }
                                /* Handle a callout with an optional numerical argument, which must
                                be less than or equal to 255. A missing argument gives 0. */
                                else {
                                    let mut n: c_int = 0;
                                    *parsed_pattern = META_CALLOUT_NUMBER; /* Numerical callout */
                                    parsed_pattern = parsed_pattern.add(3); /* Skip pattern info */
                                    while ptr < ptrend && IS_DIGIT(*ptr as u32) {
                                        let d = *ptr as u32;
                                        ptr = ptr.add(1);
                                        n = n * 10 + (d - CHAR_0) as c_int;
                                        if n > 255 {
                                            errorcode = ERR(38);
                                            break 'failed;
                                        }
                                    }
                                    *parsed_pattern = n as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                }

                                /* Both formats must have a closing parenthesis */

                                if ptr >= ptrend || *ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                                    errorcode = ERR(39);
                                    break 'failed;
                                }
                                ptr = ptr.add(1);

                                /* Remember the offset to the next item in the pattern, and set a
                                default length. */

                                *previous_callout.add(1) =
                                    ptr.offset_from((*cb).start_pattern) as u32;
                                *previous_callout.add(2) = 0;
                                break 'sw; /* End callout */
                                }
                                /* ---- Conditional group ---- */
                                ptr = ptr.add(1);
                                if ptr >= ptrend {
                                    break 'unclosed_parenthesis;
                                }
                                nest_depth += 1;

                                /* If the next character is ? or * there must be an assertion next
                                (optionally preceded by a callout). */

                                if *ptr as u32 == CHAR_QUESTION_MARK
                                    || *ptr as u32 == CHAR_ASTERISK
                                {
                                    *parsed_pattern = META_COND_ASSERT;
                                    parsed_pattern = parsed_pattern.add(1);
                                    ptr = ptr.sub(1); /* Pull pointer back to the opening parenthesis. */
                                    expect_cond_assert = 2;
                                    break 'sw; /* End of conditional */
                                }

                                /* Handle (?([+-]number)... */

                                if read_number(
                                    &mut ptr,
                                    ptrend,
                                    (*cb).bracount as i32,
                                    MAX_GROUP_NUMBER,
                                    ERR(61) as u32,
                                    &mut i,
                                    &mut errorcode,
                                ) != 0
                                {
                                    if i <= 0 {
                                        errorcode = ERR(15);
                                        break 'failed;
                                    }
                                    *parsed_pattern = META_COND_NUMBER;
                                    parsed_pattern = parsed_pattern.add(1);
                                    offset = (ptr.offset_from((*cb).start_pattern) - 2) as PCRE2_SIZE;
                                    /* PUTOFFSET(offset, parsed_pattern) */
                                    *parsed_pattern = (offset >> 32) as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                    *parsed_pattern = (offset & 0xffffffff) as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                    *parsed_pattern = i as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                } else if errorcode != 0 {
                                    break 'failed; /* Number too big */
                                }
                                /* No number found. Handle the special case
                                (?(VERSION[>]=n.m)... */
                                else if ptrend.offset_from(ptr) >= 10
                                    && crate::string_utils::_pcre2_strncmp_c8_8(
                                        ptr,
                                        STRING_VERSION.as_ptr() as *const c_char,
                                        7,
                                    ) == 0
                                    && *ptr.add(7) as u32 != CHAR_RIGHT_PARENTHESIS
                                {
                                    let mut ge: u32 = 0;
                                    let mut major: c_int = 0;
                                    let mut minor: c_int = 0;

                                    ptr = ptr.add(7);
                                    if *ptr as u32 == CHAR_GREATER_THAN_SIGN {
                                        ge = 1;
                                        ptr = ptr.add(1);
                                    }

                                    /* NOTE: cannot write IS_DIGIT(*(++ptr)) here because IS_DIGIT
                                    references its argument twice. */

                                    if *ptr as u32 != CHAR_EQUALS_SIGN || {
                                        ptr = ptr.add(1);
                                        !IS_DIGIT(*ptr as u32)
                                    } {
                                        errorcode = ERR(79);
                                        if ge == 0 {
                                            break 'failed_forward;
                                        }
                                        break 'failed;
                                    }

                                    if read_number(
                                        &mut ptr,
                                        ptrend,
                                        -1,
                                        1000,
                                        ERR(79) as u32,
                                        &mut major,
                                        &mut errorcode,
                                    ) == 0
                                    {
                                        break 'failed;
                                    }

                                    if ptr < ptrend && *ptr as u32 == CHAR_DOT {
                                        ptr = ptr.add(1);
                                        if ptr >= ptrend || !IS_DIGIT(*ptr as u32) {
                                            errorcode = ERR(79);
                                            if ptr < ptrend {
                                                break 'failed_forward;
                                            }
                                            break 'failed;
                                        }
                                        if read_number(
                                            &mut ptr,
                                            ptrend,
                                            -1,
                                            1000,
                                            ERR(79) as u32,
                                            &mut minor,
                                            &mut errorcode,
                                        ) == 0
                                        {
                                            break 'failed;
                                        }
                                    }
                                    if ptr >= ptrend || *ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                                        errorcode = ERR(79);
                                        if ptr < ptrend {
                                            break 'failed_forward;
                                        }
                                        break 'failed;
                                    }

                                    *parsed_pattern = META_COND_VERSION;
                                    parsed_pattern = parsed_pattern.add(1);
                                    *parsed_pattern = ge;
                                    parsed_pattern = parsed_pattern.add(1);
                                    *parsed_pattern = major as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                    *parsed_pattern = minor as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                }
                                /* All the remaining cases now require us to read a name. */
                                else {
                                    let mut was_r_ampersand: BOOL = FALSE;

                                    if *ptr as u32 == CHAR_R
                                        && ptrend.offset_from(ptr) > 1
                                        && *ptr.add(1) as u32 == CHAR_AMPERSAND
                                    {
                                        terminator = CHAR_RIGHT_PARENTHESIS;
                                        was_r_ampersand = TRUE;
                                        ptr = ptr.add(1);
                                    } else if *ptr as u32 == CHAR_LESS_THAN_SIGN {
                                        terminator = CHAR_GREATER_THAN_SIGN;
                                    } else if *ptr as u32 == CHAR_APOSTROPHE {
                                        terminator = CHAR_APOSTROPHE;
                                    } else {
                                        terminator = CHAR_RIGHT_PARENTHESIS;
                                        ptr = ptr.sub(1); /* Point to char before name */
                                    }

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
                                    ) == 0
                                    {
                                        break 'failed;
                                    }

                                    /* Handle (?(R&name) */

                                    if was_r_ampersand != 0 {
                                        *parsed_pattern = META_COND_RNAME;
                                        ptr = ptr.sub(1); /* Back to closing parens */
                                    }
                                    /* Handle (?(name). If the name is "DEFINE" we identify it with
                                    a special code. */
                                    else if terminator == CHAR_RIGHT_PARENTHESIS {
                                        if namelen == 6
                                            && crate::string_utils::_pcre2_strncmp_c8_8(
                                                name,
                                                STRING_DEFINE.as_ptr() as *const c_char,
                                                6,
                                            ) == 0
                                        {
                                            *parsed_pattern = META_COND_DEFINE;
                                        } else {
                                            i = 1;
                                            while i < namelen as c_int {
                                                if !IS_DIGIT(*name.add(i as usize) as u32) {
                                                    break;
                                                }
                                                i += 1;
                                            }
                                            *parsed_pattern = if *name as u32 == CHAR_R
                                                && i >= namelen as c_int
                                            {
                                                META_COND_RNUMBER
                                            } else {
                                                META_COND_NAME
                                            };
                                        }
                                        ptr = ptr.sub(1); /* Back to closing parens */
                                    }
                                    /* Handle (?('name') or (?(<name>) */
                                    else {
                                        *parsed_pattern = META_COND_NAME;
                                    }

                                    /* All these cases except DEFINE end with the name length and
                                    offset; DEFINE just has an offset (for the "too many branches"
                                    error). */

                                    let pv = *parsed_pattern;
                                    parsed_pattern = parsed_pattern.add(1);
                                    if pv != META_COND_DEFINE {
                                        *parsed_pattern = namelen;
                                        parsed_pattern = parsed_pattern.add(1);
                                    }
                                    /* PUTOFFSET(offset, parsed_pattern) */
                                    *parsed_pattern = (offset >> 32) as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                    *parsed_pattern = (offset & 0xffffffff) as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                } /* End cases that read a name */

                                /* Check the closing parenthesis of the condition */

                                if ptr >= ptrend || *ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                                    errorcode = ERR(24);
                                    break 'failed;
                                }
                                ptr = ptr.add(1);
                                break 'sw; /* End of condition processing */
                                }
                                /* ---- Atomic group ---- */
                                /* case CHAR_GREATER_THAN_SIGN: fall through */
                                }
                                /* ATOMIC_GROUP: Come from (*atomic: */
                                *parsed_pattern = META_ATOMIC;
                                parsed_pattern = parsed_pattern.add(1);
                                nest_depth += 1;
                                ptr = ptr.add(1);
                                break 'sw;
                                }
                                /* ---- Lookahead assertions ---- */
                                /* case CHAR_EQUALS_SIGN: fall through */
                                }
                                /* POSITIVE_LOOK_AHEAD: Come from (*pla: */
                                *parsed_pattern = META_LOOKAHEAD;
                                parsed_pattern = parsed_pattern.add(1);
                                ptr = ptr.add(1);
                                break 'l_post_assertion;
                                }
                                /* case CHAR_ASTERISK: fall through */
                                }
                                /* POSITIVE_NONATOMIC_LOOK_AHEAD: Come from (*napla: */
                                *parsed_pattern = META_LOOKAHEAD_NA;
                                parsed_pattern = parsed_pattern.add(1);
                                ptr = ptr.add(1);
                                break 'l_post_assertion;
                                }
                                /* case CHAR_EXCLAMATION_MARK: fall through */
                                }
                                /* NEGATIVE_LOOK_AHEAD: Come from (*nla: */
                                *parsed_pattern = META_LOOKAHEADNOT;
                                parsed_pattern = parsed_pattern.add(1);
                                ptr = ptr.add(1);
                                break 'l_post_assertion;
                                }
                                /* ---- Lookbehind assertions ---- */
                                /* (?< followed by = or ! or * is a lookbehind assertion.
                                Otherwise (?< is the start of the name of a capturing group. */
                                if ptrend.offset_from(ptr) <= 1
                                    || (*ptr.add(1) as u32 != CHAR_EQUALS_SIGN
                                        && *ptr.add(1) as u32 != CHAR_EXCLAMATION_MARK
                                        && *ptr.add(1) as u32 != CHAR_ASTERISK)
                                {
                                    terminator = CHAR_GREATER_THAN_SIGN;
                                    break 'l_define_name;
                                }
                                *parsed_pattern = if *ptr.add(1) as u32 == CHAR_EQUALS_SIGN {
                                    META_LOOKBEHIND
                                } else if *ptr.add(1) as u32 == CHAR_EXCLAMATION_MARK {
                                    META_LOOKBEHINDNOT
                                } else {
                                    META_LOOKBEHIND_NA
                                };
                                parsed_pattern = parsed_pattern.add(1);
                                /* Fall through to POST_LOOKBEHIND */
                                }
                                /* POST_LOOKBEHIND: Come from (*plb: (*naplb: and (*nlb: */
                                *has_lookbehind = TRUE;
                                offset = (ptr.offset_from((*cb).start_pattern) - 2) as PCRE2_SIZE;
                                /* PUTOFFSET(offset, parsed_pattern) */
                                *parsed_pattern = (offset >> 32) as u32;
                                parsed_pattern = parsed_pattern.add(1);
                                *parsed_pattern = (offset & 0xffffffff) as u32;
                                parsed_pattern = parsed_pattern.add(1);
                                ptr = ptr.add(2);
                                /* Fall through to POST_ASSERTION */
                                }
                                /* POST_ASSERTION: */
                                nest_depth += 1;
                                if prev_expect_cond_assert > 0 {
                                    if top_nest == core::ptr::null_mut() {
                                        top_nest = (*cb).start_workspace as *mut nest_save;
                                    } else {
                                        top_nest = top_nest.add(1);
                                        if top_nest >= end_nests {
                                            errorcode = ERR(84);
                                            break 'failed;
                                        }
                                    }
                                    (*top_nest).nest_depth = nest_depth;
                                    (*top_nest).flags = NSF_CONDASSERT;
                                    (*top_nest).options = options & PARSE_TRACKED_OPTIONS;
                                    (*top_nest).xoptions =
                                        xoptions & PARSE_TRACKED_EXTRA_OPTIONS;
                                }
                                break 'sw;
                                }
                                /* ---- Define a named group ---- */
                                /* A named group may be defined as (?'name') or (?<name>). */
                                terminator = CHAR_APOSTROPHE; /* Terminator */
                                /* Fall through to DEFINE_NAME */
                                }
                                /* DEFINE_NAME: */
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
                                ) == 0
                                {
                                    break 'failed;
                                }

                                /* We have a name for this capturing group. It is also assigned a
                                number, which is its primary means of identification. */

                                if (*cb).bracount >= MAX_GROUP_NUMBER {
                                    errorcode = ERR(97);
                                    break 'failed;
                                }
                                (*cb).bracount += 1;
                                *parsed_pattern = META_CAPTURE | (*cb).bracount;
                                parsed_pattern = parsed_pattern.add(1);
                                nest_depth += 1;

                                /* Check not too many names */

                                if (*cb).names_found as u32 >= MAX_NAME_COUNT {
                                    errorcode = ERR(49);
                                    break 'failed;
                                }

                                /* Adjust the entry size to accommodate the longest name found. */

                                if namelen + IMM2_SIZE as u32 + 1 > (*cb).name_entry_size as u32 {
                                    (*cb).name_entry_size =
                                        (namelen + IMM2_SIZE as u32 + 1) as u16;
                                }

                                /* Scan the list to check for duplicates. */

                                is_dupname = FALSE;
                                hash = crate::compile_cgroup::_pcre2_compile_get_hash_from_name8(
                                    name, namelen,
                                );
                                ng = (*cb).named_groups;
                                i = 0;
                                while i < (*cb).names_found as c_int {
                                    if namelen == (*ng).length as u32
                                        && hash == NAMED_GROUP_GET_HASH(ng)
                                        && crate::string_utils::_pcre2_strncmp_8(
                                            name,
                                            (*ng).name,
                                            namelen as PCRE2_SIZE,
                                        ) == 0
                                    {
                                        /* When a bracket is referenced by the same name multiple
                                        times, is not considered as a duplicate and ignored. */
                                        if (*ng).number == (*cb).bracount {
                                            break;
                                        }
                                        if (options & PCRE2_DUPNAMES) == 0 {
                                            errorcode = ERR(43);
                                            break 'failed;
                                        }

                                        (*ng).hash_dup |= NAMED_GROUP_IS_DUPNAME;
                                        is_dupname = TRUE; /* Mark as a duplicate */
                                        (*cb).dupnames = TRUE; /* Duplicate names exist */

                                        /* The entry represents a duplicate. */
                                        name = (*ng).name;
                                        namelen = 0;

                                        /* Even duplicated names may refer to the same capture
                                        index. These references are also ignored. */
                                        while i < (*cb).names_found as c_int {
                                            if (*ng).name == name
                                                && (*ng).number == (*cb).bracount
                                            {
                                                break;
                                            }
                                            i += 1;
                                            ng = ng.add(1);
                                        }
                                        break;
                                    } else if (*ng).number == (*cb).bracount {
                                        errorcode = ERR(65);
                                        break 'failed;
                                    }
                                    i += 1;
                                    ng = ng.add(1);
                                }

                                /* Ignore duplicate with same number. */
                                if i < (*cb).names_found as c_int {
                                    break 'sw;
                                }

                                /* Increase the list size if necessary */

                                if (*cb).names_found as u32 >= (*cb).named_group_list_size {
                                    let newsize: u32 = (*cb).named_group_list_size * 2;
                                    let newspace: *mut named_group =
                                        ((*(*cb).cx).memctl.malloc.unwrap())(
                                            newsize as usize
                                                * core::mem::size_of::<named_group>(),
                                            (*(*cb).cx).memctl.memory_data,
                                        ) as *mut named_group;
                                    if newspace == core::ptr::null_mut() {
                                        errorcode = ERR(21);
                                        break 'failed;
                                    }

                                    memcpy(
                                        newspace as *mut c_void,
                                        (*cb).named_groups as *const c_void,
                                        (*cb).named_group_list_size as usize
                                            * core::mem::size_of::<named_group>(),
                                    );
                                    if (*cb).named_group_list_size
                                        > NAMED_GROUP_LIST_SIZE as u32
                                    {
                                        ((*(*cb).cx).memctl.free.unwrap())(
                                            (*cb).named_groups as *mut c_void,
                                            (*(*cb).cx).memctl.memory_data,
                                        );
                                    }
                                    (*cb).named_groups = newspace;
                                    (*cb).named_group_list_size = newsize;
                                }

                                /* Add this name to the list */
                                if is_dupname != 0 {
                                    hash |= NAMED_GROUP_IS_DUPNAME;
                                }

                                (*(*cb).named_groups.add((*cb).names_found as usize)).name = name;
                                (*(*cb).named_groups.add((*cb).names_found as usize)).length =
                                    namelen as u16;
                                (*(*cb).named_groups.add((*cb).names_found as usize)).number =
                                    (*cb).bracount;
                                (*(*cb).named_groups.add((*cb).names_found as usize)).hash_dup =
                                    hash;
                                (*cb).names_found += 1;
                                break 'sw;
                                }
                                /* ---- Perl extended character class ---- */
                                /* These are of the form '(?[...])'. */
                                class_mode_state = CLASS_MODE_PERL_EXT;
                                c = *ptr as u32;
                                ptr = ptr.add(1);
                                /* goto FROM_PERL_EXTENDED_CLASS */
                                sw_target = 1;
                                continue 'sw;
                                }
                                /* ---- Branch terminators ---- */
                                /* Alternation: reset the capture count if we are in a (?| group. */
                                if top_nest != core::ptr::null_mut()
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
                                break 'sw;
                                }
                                /* End of group; reset the capture count to the maximum if we are in
                                a (?| group and/or reset the options that are tracked during
                                parsing. Disallow quantifier for a condition that is an
                                assertion. */
                                okquantifier = TRUE;
                                if top_nest != core::ptr::null_mut()
                                    && (*top_nest).nest_depth == nest_depth
                                {
                                    options =
                                        (options & !PARSE_TRACKED_OPTIONS) | (*top_nest).options;
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
                                    /* Unmatched closing parenthesis */
                                    errorcode = ERR(22);
                                    break 'failed;
                                }
                                nest_depth -= 1;
                                *parsed_pattern = META_KET;
                                parsed_pattern = parsed_pattern.add(1);
                                break 'sw;
                            } /* End of switch on pattern character */
                        } /* End of main character scan loop */

                        /* End of pattern reached. Check for missing ) at the end of a verb
                        name. */

                        if inverbname != 0 && ptr >= ptrend {
                            errorcode = ERR(60);
                            break 'failed;
                        }
                    }
                    /* PARSED_END: */

                    /* Manage callout for the final item */

                    parsed_pattern = manage_callouts(
                        ptr,
                        &mut previous_callout,
                        auto_callout,
                        parsed_pattern,
                        cb,
                    );

                    /* Insert trailing items for word and line matching (features provided for
                    the benefit of pcre2grep). */

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

                    /* Terminate the parsed pattern, then return success if all groups are
                    closed. Otherwise we have unclosed parentheses. */

                    if parsed_pattern >= parsed_pattern_end {
                        errorcode = ERR(63); /* Internal error (parsed pattern overflow) */
                        break 'failed;
                    }

                    *parsed_pattern = META_END;
                    if nest_depth == 0 {
                        return 0;
                    }
                }
                /* UNCLOSED_PARENTHESIS: */
                errorcode = ERR(14);
                break 'failed;
            }
            /* FAILED_FORWARD: Some errors need to indicate the next character. */
            ptr = ptr.add(1);
            if utf != 0 {
                /* FORWARDCHARTEST(ptr, ptrend) */
                while ptr < ptrend && (*ptr & 0xc0u8) == 0x80u8 {
                    ptr = ptr.add(1);
                }
            }
            break 'failed;
        }
        /* FAILED_BACK: Some errors need to indicate the previous character. */
        ptr = ptr.sub(1);
        if utf != 0 {
            /* BACKCHAR(ptr) */
            while (*ptr & 0xc0u8) == 0x80u8 {
                ptr = ptr.sub(1);
            }
        }
    }
    /* FAILED: Come here for all failures. */
    (*cb).erroroffset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
    errorcode
}
