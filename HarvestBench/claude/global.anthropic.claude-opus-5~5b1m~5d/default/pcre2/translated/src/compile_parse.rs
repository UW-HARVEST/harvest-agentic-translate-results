//! Translated from pcre2_compile.c, lines 3161-5967 (parse_regex).
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use crate::compile_tables::*;
use crate::compile::*;
use crate::compile_branch::*;
use crate::compile_aux::*;
use crate::compile_cgroup::*;
use crate::string_utils::*;
use crate::tables::*;
use core::ffi::{c_char, c_void};

/* ------------------------------------------------------------------------- */
/* Character constants (CHAR_xxx in the C source).  These are module-private  */
/* so that they can be used as `match` patterns without colliding with other  */
/* modules' glob-imported names.                                             */
/* ------------------------------------------------------------------------- */

const CHAR_NUL: u32 = 0x00;
const CHAR_HT: u32 = 0x09;
const CHAR_BS: u32 = 0x08;
const CHAR_NEL: u32 = 0x85;
const CHAR_SPACE: u32 = 0x20;
const CHAR_EXCLAMATION_MARK: u32 = 0x21;
const CHAR_NUMBER_SIGN: u32 = 0x23;
const CHAR_DOLLAR_SIGN: u32 = 0x24;
const CHAR_AMPERSAND: u32 = 0x26;
const CHAR_APOSTROPHE: u32 = 0x27;
const CHAR_LEFT_PARENTHESIS: u32 = 0x28;
const CHAR_RIGHT_PARENTHESIS: u32 = 0x29;
const CHAR_ASTERISK: u32 = 0x2a;
const CHAR_PLUS: u32 = 0x2b;
const CHAR_MINUS: u32 = 0x2d;
const CHAR_DOT: u32 = 0x2e;
const CHAR_0: u32 = 0x30;
const CHAR_1: u32 = 0x31;
const CHAR_2: u32 = 0x32;
const CHAR_3: u32 = 0x33;
const CHAR_4: u32 = 0x34;
const CHAR_5: u32 = 0x35;
const CHAR_6: u32 = 0x36;
const CHAR_7: u32 = 0x37;
const CHAR_8: u32 = 0x38;
const CHAR_9: u32 = 0x39;
const CHAR_COLON: u32 = 0x3a;
const CHAR_LESS_THAN_SIGN: u32 = 0x3c;
const CHAR_EQUALS_SIGN: u32 = 0x3d;
const CHAR_GREATER_THAN_SIGN: u32 = 0x3e;
const CHAR_QUESTION_MARK: u32 = 0x3f;
const CHAR_C: u32 = 0x43;
const CHAR_D: u32 = 0x44;
const CHAR_E: u32 = 0x45;
const CHAR_J: u32 = 0x4a;
const CHAR_P: u32 = 0x50;
const CHAR_Q: u32 = 0x51;
const CHAR_R: u32 = 0x52;
const CHAR_S: u32 = 0x53;
const CHAR_T: u32 = 0x54;
const CHAR_U: u32 = 0x55;
const CHAR_W: u32 = 0x57;
const CHAR_LEFT_SQUARE_BRACKET: u32 = 0x5b;
const CHAR_BACKSLASH: u32 = 0x5c;
const CHAR_RIGHT_SQUARE_BRACKET: u32 = 0x5d;
const CHAR_CIRCUMFLEX_ACCENT: u32 = 0x5e;
const CHAR_a: u32 = 0x61;
const CHAR_i: u32 = 0x69;
const CHAR_k: u32 = 0x6b;
const CHAR_m: u32 = 0x6d;
const CHAR_n: u32 = 0x6e;
const CHAR_r: u32 = 0x72;
const CHAR_s: u32 = 0x73;
const CHAR_u: u32 = 0x75;
const CHAR_x: u32 = 0x78;
const CHAR_LEFT_CURLY_BRACKET: u32 = 0x7b;
const CHAR_VERTICAL_LINE: u32 = 0x7c;
const CHAR_RIGHT_CURLY_BRACKET: u32 = 0x7d;
const CHAR_TILDE: u32 = 0x7e;

/* String literals from pcre2_internal.h, as NUL-terminated byte arrays. */

static STRING_WEIRD_STARTWORD: [u8; 7] = *b"[:<:]]\0";
static STRING_WEIRD_ENDWORD: [u8; 7] = *b"[:>:]]\0";
static STRING_VERSION: [u8; 8] = *b"VERSION\0";
static STRING_DEFINE: [u8; 7] = *b"DEFINE\0";
static STR_Q_BACKSLASH_E: [u8; 4] = *b"Q\\E\0";

/* pcre2_compile.c line 117 */
const MAX_GROUP_NUMBER: u32 = 65535u32;

/* pcre2_compile.c lines 3038-3045: a structure for dealing with nested
groups. */

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

/* pcre2_compile.c lines 3047-3049 */
const NSF_RESET: u16 = 0x0001u16;
const NSF_CONDASSERT: u16 = 0x0002u16;
const NSF_ATOMICSR: u16 = 0x0004u16;

/* pcre2_compile.c lines 3056-3062 */
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

/* States used for analyzing ranges in character classes (lines 3067-3074).
The two OK values must be last. */

const RANGE_NO: u32 = 0;
const RANGE_STARTED: u32 = 1;
const RANGE_FORBID_NO: u32 = 2;
const RANGE_FORBID_STARTED: u32 = 3;
const RANGE_OK_ESCAPED: u32 = 4;
const RANGE_OK_LITERAL: u32 = 5;

/* States used for analyzing operators and operands in extended character
classes (lines 3079-3083). */

const CLASS_OP_EMPTY: u32 = 0;
const CLASS_OP_OPERAND: u32 = 1;
const CLASS_OP_OPERATOR: u32 = 2;

/* States used for determining the parse mode in character classes (lines
3088-3093). The two PERL_EXT values must be last. */

const CLASS_MODE_NORMAL: u32 = 0;
const CLASS_MODE_ALT_EXT: u32 = 1;
const CLASS_MODE_PERL_EXT: u32 = 2;
const CLASS_MODE_PERL_EXT_LEAF: u32 = 3;

/* IS_DIGIT(x) from pcre2_internal.h */
macro_rules! IS_DIGIT {
    ($x:expr) => {
        ($x as u32) >= CHAR_0 && ($x as u32) <= CHAR_9
    };
}

/* PARSED_LITERAL(c, p) for the 8-bit case (pcre2_compile.c line 3107):
   *p++ = c; okquantifier = TRUE;
   The C macro refers to the enclosing function's `okquantifier`; Rust macros
   are not able to do that, so it is passed as a third argument. */
macro_rules! PARSED_LITERAL {
    ($c:expr, $p:expr, $okquantifier:expr) => {{
        *$p = $c;
        $p = $p.add(1);
        $okquantifier = TRUE;
    }};
}

/* *p++ = v  (a very common idiom in this function) */
macro_rules! PUTPP {
    ($p:expr, $v:expr) => {{
        *$p = $v;
        $p = $p.add(1);
    }};
}

/* IS_NEWLINE(p) with NLBLOCK == cb, PSEND == end_pattern. */
macro_rules! IS_NEWLINE {
    ($p:expr, $cb:expr, $utf:expr) => {
        crate::macros::is_newline_block(
            $p,
            (*$cb).nltype,
            &mut (*$cb).nllen,
            ((*$cb).nl).as_ptr(),
            (*$cb).end_pattern,
            $utf,
        )
    };
}

/* ------------------------------------------------------------------------- */
/* Inner state-machine labels.  The C function uses a large number of labels  */
/* inside the body of the main character-scan loop; each becomes a state of a */
/* state machine that replaces the `switch (c)` statement.                    */
/* ------------------------------------------------------------------------- */

const S_SWITCH: u32 = 0;
const S_ESCAPE_FAILED: u32 = 1;
const S_ESCAPE_TAIL: u32 = 2; /* the code following the ESCAPE_FAILED block */
const S_CHECK_QUANTIFIER: u32 = 3;
const S_FROM_PERL_EXTENDED_CLASS: u32 = 4;
const S_RECURSION_BYNUMBER: u32 = 5;
const S_SET_RECURSION: u32 = 6;
const S_RECURSE_BY_NAME: u32 = 7;
const S_READ_RECURSION_ARGUMENTS: u32 = 8;
const S_ATOMIC_GROUP: u32 = 9;
const S_POSITIVE_LOOK_AHEAD: u32 = 10;
const S_POSITIVE_NONATOMIC_LOOK_AHEAD: u32 = 11;
const S_NEGATIVE_LOOK_AHEAD: u32 = 12;
const S_POST_LOOKBEHIND: u32 = 13;
const S_POST_ASSERTION: u32 = 14;
const S_DEFINE_NAME: u32 = 15;

/* States for the class-content loop (labels CLASS_LITERAL and
CLASS_CONTINUE). */

const CL_TOP: u32 = 0;
const CL_LITERAL: u32 = 1;
const CL_CONTINUE: u32 = 2;

/*************************************************
*      Parse regex and identify named groups     *
*************************************************/

/* This function is called first of all. It scans the pattern and does two
things: (1) It identifies capturing groups and makes a table of named capturing
groups so that information about them is fully available to both the compiling
scans. (2) It writes a parsed version of the pattern with comments omitted and
escapes processed into the parsed_pattern vector.

Arguments:
  ptr             points to the start of the pattern
  options         compiling dynamic options (may change during the scan)
  has_lookbehind  points to a boolean, set TRUE if a lookbehind is found
  cb              pointer to the compile data block

Returns:   zero on success or a non-zero error code, with the
             error offset placed in the cb field
*/

pub(crate) unsafe fn parse_regex(
    mut ptr: PCRE2_SPTR,
    mut options: u32,
    mut xoptions: u32,
    has_lookbehind: *mut BOOL,
    cb: *mut compile_block,
) -> i32 {
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
    let mut hash: u16 = 0;
    let mut after_manual_callout: i32 = 0;
    let mut expect_cond_assert: i32 = 0;
    let mut errorcode: i32 = 0;
    let mut escape: i32 = 0;
    let mut i: i32 = 0;
    let mut inescq: BOOL = FALSE;
    let mut inverbname: BOOL = FALSE;
    let utf: BOOL = ((options & PCRE2_UTF) != 0) as BOOL;
    let auto_callout: BOOL = ((options & PCRE2_AUTO_CALLOUT) != 0) as BOOL;
    let mut is_dupname: BOOL = FALSE;
    let mut negate_class: BOOL = FALSE;
    let mut okquantifier: BOOL = FALSE;
    let mut thisptr: PCRE2_SPTR = core::ptr::null();
    let mut name: PCRE2_SPTR = core::ptr::null();
    let ptrend: PCRE2_SPTR = (*cb).end_pattern;
    let mut verbnamestart: PCRE2_SPTR = core::ptr::null(); /* Value avoids compiler warning */
    let mut class_range_forbid_ptr: PCRE2_SPTR = core::ptr::null();
    let mut ng: *mut named_group = core::ptr::null_mut();
    let mut top_nest: *mut nest_save = core::ptr::null_mut();
    let mut end_nests: *mut nest_save = core::ptr::null_mut();

    /* Emulation of the trailing FAILED_BACK / FAILED_FORWARD labels: 0 means
    "come to FAILED directly", 1 means FAILED_BACK, 2 means FAILED_FORWARD. */
    let mut fail_mode: i32 = 0;

    /* PCRE2_ASSERT(parsed_pattern != NULL); */

    'failed: {
        'unclosed_parenthesis: {
            'parsed_end: {
                /* Insert leading items for word and line matching (features provided
                for the benefit of pcre2grep). */

                if (xoptions & PCRE2_EXTRA_MATCH_LINE) != 0 {
                    PUTPP!(parsed_pattern, META_CIRCUMFLEX);
                    PUTPP!(parsed_pattern, META_NOCAPTURE);
                } else if (xoptions & PCRE2_EXTRA_MATCH_WORD) != 0 {
                    PUTPP!(parsed_pattern, META_ESCAPE + ESC_b);
                    PUTPP!(parsed_pattern, META_NOCAPTURE);
                }

                /* If the pattern is actually a literal string, process it separately
                to avoid cluttering up the main loop. */

                if (options & PCRE2_LITERAL) != 0 {
                    while ptr < ptrend {
                        /* LCOV_EXCL_START */
                        if parsed_pattern >= parsed_pattern_end {
                            errorcode = ERR63; /* Internal error (parsed pattern overflow) */
                            break 'failed; /* goto FAILED */
                        }
                        /* LCOV_EXCL_STOP */

                        thisptr = ptr;
                        GETCHARINCTEST!(c, ptr, utf);
                        if auto_callout != 0 {
                            parsed_pattern = manage_callouts(
                                thisptr,
                                &mut previous_callout,
                                auto_callout,
                                parsed_pattern,
                                cb,
                            );
                        }
                        PARSED_LITERAL!(c, parsed_pattern, okquantifier);
                    }
                    break 'parsed_end; /* goto PARSED_END */
                }

                /* Process a real regex which may contain meta-characters. */

                top_nest = core::ptr::null_mut();
                end_nests =
                    (*cb).start_workspace.add((*cb).workspace_size) as *mut nest_save;

                /* The size of the nest_save structure might not be a factor of the
                size of the workspace. Therefore we must round down end_nests so as to
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
                    let mut prev_expect_cond_assert: i32;
                    let mut min_repeat: u32 = 0;
                    let mut max_repeat: u32 = 0;
                    let mut set: u32 = 0;
                    let mut unset: u32 = 0;
                    /* `optset`/`xoptset` are `uint32_t *` in C, pointing at either
                    `set`/`unset` (resp. `xset`/`xunset`). */
                    let mut optset: *mut u32 = core::ptr::null_mut();
                    let mut xset: u32 = 0;
                    let mut xunset: u32 = 0;
                    let mut xoptset: *mut u32 = core::ptr::null_mut();
                    let mut terminator: u32 = 0;
                    let mut prev_meta_quantifier: u32 = 0;
                    let mut prev_okquantifier: BOOL = FALSE;
                    let mut tempptr: PCRE2_SPTR = core::ptr::null();
                    let mut offset: PCRE2_SIZE = 0;

                    if nest_depth as u32 > (*(*cb).cx).parens_nest_limit {
                        errorcode = ERR19;
                        break 'failed; /* goto FAILED - Parentheses too deeply nested */
                    }

                    /* Check that we haven't emitted too much into parsed_pattern. */

                    /* LCOV_EXCL_START */
                    if parsed_pattern >= parsed_pattern_end {
                        errorcode = ERR63; /* Internal error (parsed pattern overflow) */
                        break 'failed; /* goto FAILED */
                    }
                    /* LCOV_EXCL_STOP */

                    /* If the last time round this loop something was added,
                    parsed_pattern will no longer be equal to this_parsed_item.
                    Remember where the previous item started and reset for the next
                    item. */

                    if this_parsed_item != parsed_pattern {
                        prev_parsed_item = this_parsed_item;
                        this_parsed_item = parsed_pattern;
                    }

                    /* Get next input character, save its position for callout
                    handling. */

                    thisptr = ptr;
                    GETCHARINCTEST!(c, ptr, utf);

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
                                PUTPP!(parsed_pattern, c);
                            } else {
                                if {
                                    let t = after_manual_callout;
                                    after_manual_callout -= 1;
                                    t <= 0
                                } {
                                    parsed_pattern = manage_callouts(
                                        thisptr,
                                        &mut previous_callout,
                                        auto_callout,
                                        parsed_pattern,
                                        cb,
                                    );
                                }
                                PARSED_LITERAL!(c, parsed_pattern, okquantifier);
                            }
                            meta_quantifier = 0;
                        }
                        continue 'mainloop; /* Next character */
                    }

                    /* If we are processing the "name" part of a (*VERB:NAME) item,
                    all characters up to the closing parenthesis are literals except
                    when PCRE2_ALT_VERBNAMES is set. */

                    if inverbname != 0
                        && (
                            /* EITHER: not both options set */
                            ((options & (PCRE2_EXTENDED | PCRE2_ALT_VERBNAMES))
                                != (PCRE2_EXTENDED | PCRE2_ALT_VERBNAMES))
                                /* OR: character > 255 AND not Unicode Pattern White Space */
                                || (c > 255 && (c | 1) != 0x200f && (c | 1) != 0x2029)
                                /* OR: not a # comment or isspace() white space */
                                || (c < 256
                                    && c != CHAR_NUMBER_SIGN
                                    && (*(*cb).ctypes.add(c as usize) & ctype_space) == 0
                                    /* and not CHAR_NEL when Unicode is supported */
                                    && c != CHAR_NEL)
                        )
                    {
                        let mut verbnamelength: PCRE2_SIZE;

                        match c {
                            CHAR_RIGHT_PARENTHESIS => {
                                inverbname = FALSE;
                                /* This is the length in characters */
                                verbnamelength =
                                    (parsed_pattern.offset_from(verblengthptr) - 1) as PCRE2_SIZE;
                                /* But the limit on the length is in code units */
                                if ptr.offset_from(verbnamestart) - 1 > MAX_MARK as i32 as isize {
                                    ptr = ptr.wrapping_sub(1);
                                    errorcode = ERR76;
                                    break 'failed; /* goto FAILED */
                                }
                                *verblengthptr = verbnamelength as u32;

                                /* If this name was on a verb such as (*ACCEPT) which
                                does not continue, a (*MARK) was generated for the
                                name. We now add the original verb as the next item. */

                                if add_after_mark != 0 {
                                    PUTPP!(parsed_pattern, add_after_mark);
                                    add_after_mark = 0;
                                }
                            }

                            CHAR_BACKSLASH => {
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
                                        break 'failed; /* goto FAILED */
                                    }
                                } else {
                                    escape = 0; /* Treat all as literal */
                                }

                                match escape as u32 {
                                    0 => {
                                        /* Don't use PARSED_LITERAL() because it sets
                                        okquantifier. */
                                        PUTPP!(parsed_pattern, c);
                                    }

                                    ESC_ub => {
                                        PUTPP!(parsed_pattern, CHAR_u);
                                        PARSED_LITERAL!(
                                            CHAR_LEFT_CURLY_BRACKET,
                                            parsed_pattern,
                                            okquantifier
                                        );
                                    }

                                    ESC_Q => {
                                        inescq = TRUE;
                                    }

                                    ESC_E => { /* Ignore */ }

                                    _ => {
                                        errorcode = ERR40; /* Invalid in verb name */
                                        break 'failed; /* goto FAILED */
                                    }
                                }
                            }

                            _ => {
                                /* Don't use PARSED_LITERAL() because it sets
                                okquantifier. */
                                PUTPP!(parsed_pattern, c);
                            }
                        }
                        continue 'mainloop; /* Next character in pattern */
                    }

                    /* Not a verb name character. At this point we must process
                    everything that must not change the quantification state. */

                    if c == CHAR_BACKSLASH && ptr < ptrend {
                        if *ptr as u32 == CHAR_Q || *ptr as u32 == CHAR_E {
                            /* A literal inside a \Q...\E is not allowed if we are
                            expecting a conditional assertion, but an empty \Q\E
                            sequence is OK. */
                            if expect_cond_assert > 0
                                && *ptr as u32 == CHAR_Q
                                && !(ptrend.offset_from(ptr) >= 3
                                    && *ptr.add(1) as u32 == CHAR_BACKSLASH
                                    && *ptr.add(2) as u32 == CHAR_E)
                            {
                                ptr = ptr.wrapping_sub(1);
                                errorcode = ERR28;
                                break 'failed; /* goto FAILED */
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
                                if IS_NEWLINE!(ptr, cb, utf) != 0 {
                                    /* For non-fixed-length newline cases, IS_NEWLINE
                                    sets cb->nllen. */
                                    ptr = ptr.add((*cb).nllen as usize);
                                    break;
                                }
                                ptr = ptr.add(1);
                                if utf != 0 {
                                    FORWARDCHARTEST!(ptr, ptrend);
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
                            errorcode = ERR18; /* A special error for missing ) in a comment */
                            break 'failed; /* goto FAILED - to make it easier to debug. */
                        }
                        ptr = ptr.add(1);
                        continue 'mainloop; /* Next character in pattern */
                    }

                    /* If the next item is not a quantifier, fill in length of any
                    previous callout and create an auto callout if required. */

                    if c != CHAR_ASTERISK
                        && c != CHAR_PLUS
                        && c != CHAR_QUESTION_MARK
                        && (c != CHAR_LEFT_CURLY_BRACKET
                            || {
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
                        if {
                            let t = after_manual_callout;
                            after_manual_callout -= 1;
                            t <= 0
                        } {
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
                                ok = (MAX_255!(*ptr.add(1)) != 0
                                    && (*(*cb).ctypes.add(*ptr.add(1) as usize)
                                        & ctype_lcletter)
                                        != 0) as BOOL;
                            } else {
                                match *ptr.add(1) as u32 {
                                    /* Traditional symbolic format */
                                    CHAR_C => {
                                        ok = (expect_cond_assert == 2) as BOOL;
                                    }

                                    CHAR_EQUALS_SIGN | CHAR_EXCLAMATION_MARK => {}

                                    CHAR_LESS_THAN_SIGN => {
                                        ok = (*ptr.add(2) as u32 == CHAR_EQUALS_SIGN
                                            || *ptr.add(2) as u32 == CHAR_EXCLAMATION_MARK)
                                            as BOOL;
                                    }

                                    _ => {
                                        ok = FALSE;
                                    }
                                }
                            }
                        }

                        if ok == 0 {
                            errorcode = ERR28;
                            if expect_cond_assert == 2 {
                                break 'failed; /* goto FAILED */
                            }
                            fail_mode = 1;
                            break 'failed; /* goto FAILED_BACK */
                        }
                    }

                    /* Remember whether we are expecting a conditional assertion, and
                    set the default for this item. */

                    prev_expect_cond_assert = expect_cond_assert;
                    expect_cond_assert = 0;

                    /* Remember quantification status for the previous significant
                    item, then set default for this item. */

                    prev_okquantifier = okquantifier;
                    prev_meta_quantifier = meta_quantifier;
                    okquantifier = FALSE;
                    meta_quantifier = 0;

                    /* If the previous significant item was a quantifier, adjust the
                    parsed code if there is a following modifier. */

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

                    let mut istate: u32 = S_SWITCH;
                    'isw: loop {
                        match istate {
                            /* ---- switch(c) ---- */
                            S_SWITCH => {
                                match c {
                                    /* ---- Escape sequence ---- */
                                    CHAR_BACKSLASH => {
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
                                        if errorcode != 0 {
                                            /* falls into the ESCAPE_FAILED label */
                                            istate = S_ESCAPE_FAILED;
                                            continue 'isw;
                                        }
                                        istate = S_ESCAPE_TAIL;
                                        continue 'isw;
                                    }

                                    /* ---- Single-character special items ---- */
                                    CHAR_CIRCUMFLEX_ACCENT => {
                                        PUTPP!(parsed_pattern, META_CIRCUMFLEX);
                                    }

                                    CHAR_DOLLAR_SIGN => {
                                        PUTPP!(parsed_pattern, META_DOLLAR);
                                    }

                                    CHAR_DOT => {
                                        PUTPP!(parsed_pattern, META_DOT);
                                        okquantifier = TRUE;
                                    }

                                    /* ---- Single-character quantifiers ---- */
                                    CHAR_ASTERISK => {
                                        meta_quantifier = META_ASTERISK;
                                        istate = S_CHECK_QUANTIFIER; /* goto CHECK_QUANTIFIER */
                                        continue 'isw;
                                    }

                                    CHAR_PLUS => {
                                        meta_quantifier = META_PLUS;
                                        istate = S_CHECK_QUANTIFIER; /* goto CHECK_QUANTIFIER */
                                        continue 'isw;
                                    }

                                    CHAR_QUESTION_MARK => {
                                        meta_quantifier = META_QUERY;
                                        istate = S_CHECK_QUANTIFIER; /* goto CHECK_QUANTIFIER */
                                        continue 'isw;
                                    }

                                    /* ---- Potential {n,m} quantifier ---- */
                                    CHAR_LEFT_CURLY_BRACKET => {
                                        if read_repeat_counts(
                                            &mut ptr,
                                            ptrend,
                                            &mut min_repeat,
                                            &mut max_repeat,
                                            &mut errorcode,
                                        ) == 0
                                        {
                                            if errorcode != 0 {
                                                break 'failed; /* goto FAILED - Error in quantifier. */
                                            }
                                            /* Not a quantifier */
                                            PARSED_LITERAL!(
                                                c,
                                                parsed_pattern,
                                                okquantifier
                                            );
                                            break 'isw; /* No more quantifier processing */
                                        }
                                        meta_quantifier = META_MINMAX;
                                        /* Fall through to CHECK_QUANTIFIER */
                                        istate = S_CHECK_QUANTIFIER;
                                        continue 'isw;
                                    }

                                    /* ---- Character class ---- */
                                    CHAR_LEFT_SQUARE_BRACKET => {
                                        /* In another (POSIX) regex library, the ugly
                                        syntax [[:<:]] and [[:>:]] is used for "start
                                        of word" and "end of word". */

                                        if ptrend.offset_from(ptr) >= 6
                                            && (_pcre2_strncmp_c8_8(
                                                ptr,
                                                STRING_WEIRD_STARTWORD.as_ptr() as *const c_char,
                                                6,
                                            ) == 0
                                                || _pcre2_strncmp_c8_8(
                                                    ptr,
                                                    STRING_WEIRD_ENDWORD.as_ptr() as *const c_char,
                                                    6,
                                                ) == 0)
                                        {
                                            PUTPP!(parsed_pattern, META_ESCAPE + ESC_b);

                                            if *ptr.add(2) as u32 == CHAR_LESS_THAN_SIGN {
                                                PUTPP!(parsed_pattern, META_LOOKAHEAD);
                                            } else {
                                                PUTPP!(parsed_pattern, META_LOOKBEHIND);
                                                *has_lookbehind = TRUE;

                                                /* The offset is used only for the
                                                "non-fixed length" error; this won't
                                                occur here, so just store zero. */

                                                PUTOFFSET!(0 as PCRE2_SIZE, parsed_pattern);
                                            }

                                            if (options & PCRE2_UCP) == 0 {
                                                PUTPP!(parsed_pattern, META_ESCAPE + ESC_w);
                                            } else {
                                                PUTPP!(parsed_pattern, META_ESCAPE + ESC_p);
                                                PUTPP!(parsed_pattern, PT_WORD << 16);
                                            }
                                            PUTPP!(parsed_pattern, META_KET);
                                            ptr = ptr.add(6);
                                            okquantifier = TRUE;
                                            break 'isw;
                                        }

                                        /* PCRE supports POSIX class stuff inside a
                                        class. Perl gives an error if they are
                                        encountered at the top level, so we'll do that
                                        too. */

                                        if ptr < ptrend
                                            && (*ptr as u32 == CHAR_COLON
                                                || *ptr as u32 == CHAR_DOT
                                                || *ptr as u32 == CHAR_EQUALS_SIGN)
                                            && check_posix_syntax(ptr, ptrend, &mut tempptr) != 0
                                        {
                                            errorcode = if {
                                                let t = *ptr as u32;
                                                ptr = ptr.wrapping_sub(1);
                                                t == CHAR_COLON
                                            } {
                                                ERR12
                                            } else {
                                                ERR13
                                            };
                                            ptr = tempptr.add(2);
                                            break 'failed; /* goto FAILED */
                                        }

                                        class_mode_state =
                                            if (options & PCRE2_ALT_EXTENDED_CLASS) != 0 {
                                                CLASS_MODE_ALT_EXT
                                            } else {
                                                CLASS_MODE_NORMAL
                                            };

                                        /* Falls into FROM_PERL_EXTENDED_CLASS */
                                        istate = S_FROM_PERL_EXTENDED_CLASS;
                                        continue 'isw;
                                    }

                                    /* ---- Opening parenthesis ---- */
                                    CHAR_LEFT_PARENTHESIS => {
                                        if ptr >= ptrend {
                                            break 'unclosed_parenthesis; /* goto UNCLOSED_PARENTHESIS */
                                        }

                                        /* If ( is not followed by ? it is either a
                                        capture or a special verb or an alpha assertion
                                        or a positive non-atomic lookahead. */

                                        if *ptr as u32 != CHAR_QUESTION_MARK {
                                            let mut vn: *const c_char;

                                            /* Handle capturing brackets (or
                                            non-capturing if auto-capture is turned
                                            off). */

                                            if *ptr as u32 != CHAR_ASTERISK {
                                                nest_depth += 1;
                                                if (options & PCRE2_NO_AUTO_CAPTURE) == 0 {
                                                    if (*cb).bracount >= MAX_GROUP_NUMBER {
                                                        errorcode = ERR97;
                                                        break 'failed; /* goto FAILED */
                                                    }
                                                    (*cb).bracount += 1;
                                                    PUTPP!(
                                                        parsed_pattern,
                                                        META_CAPTURE | (*cb).bracount
                                                    );
                                                } else {
                                                    PUTPP!(parsed_pattern, META_NOCAPTURE);
                                                }
                                            }
                                            /* Do nothing for (* followed by end of
                                            pattern or ) so it gives a "bad quantifier"
                                            error rather than "(*MARK) must have an
                                            argument". */
                                            else if ptrend.offset_from(ptr) <= 1
                                                || {
                                                    c = *ptr.add(1) as u32;
                                                    c == CHAR_RIGHT_PARENTHESIS
                                                }
                                            {
                                                break 'isw;
                                            }
                                            /* Handle "alpha assertions" such as
                                            (*pla:...). */
                                            else if CHMAX_255!(c) != 0
                                                && (*(*cb).ctypes.add(c as usize)
                                                    & ctype_lcletter)
                                                    != 0
                                            {
                                                let mut meta: u32;

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
                                                    break 'failed; /* goto FAILED */
                                                }
                                                if ptr >= ptrend {
                                                    break 'unclosed_parenthesis; /* goto UNCLOSED_PARENTHESIS */
                                                }
                                                if *ptr as u32 != CHAR_COLON {
                                                    errorcode = ERR95; /* Malformed */
                                                    fail_mode = 2;
                                                    break 'failed; /* goto FAILED_FORWARD */
                                                }

                                                /* Scan the table of alpha assertion
                                                names */

                                                i = 0;
                                                while i < alascount {
                                                    if namelen == alasmeta[i as usize].len
                                                        && _pcre2_strncmp_c8_8(
                                                            name,
                                                            vn,
                                                            namelen as usize,
                                                        ) == 0
                                                    {
                                                        break;
                                                    }
                                                    vn = vn
                                                        .add((alasmeta[i as usize].len + 1)
                                                            as usize);
                                                    i += 1;
                                                }

                                                if i >= alascount {
                                                    errorcode = ERR95; /* Alpha assertion not recognized */
                                                    break 'failed; /* goto FAILED */
                                                }

                                                /* Check for expecting an assertion
                                                condition. If so, only atomic lookaround
                                                assertions are valid. */

                                                meta = alasmeta[i as usize].meta;
                                                if prev_expect_cond_assert > 0
                                                    && (meta < META_LOOKAHEAD
                                                        || meta > META_LOOKBEHINDNOT)
                                                {
                                                    errorcode = ERR28; /* Atomic assertion expected */
                                                    break 'failed; /* goto FAILED */
                                                }

                                                /* The lookaround alphabetic synonyms
                                                can mostly be handled by jumping to the
                                                code that handles the traditional
                                                symbolic forms. */

                                                match meta {
                                                    META_ATOMIC => {
                                                        istate = S_ATOMIC_GROUP; /* goto ATOMIC_GROUP */
                                                        continue 'isw;
                                                    }

                                                    META_LOOKAHEAD => {
                                                        istate = S_POSITIVE_LOOK_AHEAD; /* goto POSITIVE_LOOK_AHEAD */
                                                        continue 'isw;
                                                    }

                                                    META_LOOKAHEAD_NA => {
                                                        istate =
                                                            S_POSITIVE_NONATOMIC_LOOK_AHEAD; /* goto POSITIVE_NONATOMIC_LOOK_AHEAD */
                                                        continue 'isw;
                                                    }

                                                    META_LOOKAHEADNOT => {
                                                        istate = S_NEGATIVE_LOOK_AHEAD; /* goto NEGATIVE_LOOK_AHEAD */
                                                        continue 'isw;
                                                    }

                                                    META_SCS => {
                                                        ptr = ptr.add(1);
                                                        PUTPP!(parsed_pattern, META_SCS);

                                                        parsed_pattern = parse_capture_list(
                                                            &mut ptr,
                                                            ptrend,
                                                            utf,
                                                            parsed_pattern,
                                                            0,
                                                            &mut errorcode,
                                                            cb,
                                                        );
                                                        if parsed_pattern.is_null() {
                                                            break 'failed; /* goto FAILED */
                                                        }
                                                        istate = S_POST_ASSERTION; /* goto POST_ASSERTION */
                                                        continue 'isw;
                                                    }

                                                    META_LOOKBEHIND
                                                    | META_LOOKBEHINDNOT
                                                    | META_LOOKBEHIND_NA => {
                                                        PUTPP!(parsed_pattern, meta);
                                                        ptr = ptr.wrapping_sub(1);
                                                        istate = S_POST_LOOKBEHIND; /* goto POST_LOOKBEHIND */
                                                        continue 'isw;
                                                    }

                                                    /* The script run facilities are
                                                    handled here. Always record a
                                                    META_SCRIPT_RUN item. Then, for the
                                                    atomic version, insert META_ATOMIC
                                                    and remember that we need two
                                                    META_KETs at the end. */

                                                    META_SCRIPT_RUN
                                                    | META_ATOMIC_SCRIPT_RUN => {
                                                        PUTPP!(
                                                            parsed_pattern,
                                                            META_SCRIPT_RUN
                                                        );
                                                        nest_depth += 1;
                                                        ptr = ptr.add(1);
                                                        if meta == META_ATOMIC_SCRIPT_RUN {
                                                            PUTPP!(
                                                                parsed_pattern,
                                                                META_ATOMIC
                                                            );
                                                            if top_nest.is_null() {
                                                                top_nest = (*cb)
                                                                    .start_workspace
                                                                    as *mut nest_save;
                                                            } else {
                                                                top_nest = top_nest.add(1);
                                                                if top_nest >= end_nests {
                                                                    errorcode = ERR84;
                                                                    break 'failed; /* goto FAILED */
                                                                }
                                                            }
                                                            (*top_nest).nest_depth =
                                                                nest_depth;
                                                            (*top_nest).flags = NSF_ATOMICSR;
                                                            (*top_nest).options = options
                                                                & PARSE_TRACKED_OPTIONS;
                                                            (*top_nest).xoptions = xoptions
                                                                & PARSE_TRACKED_EXTRA_OPTIONS;
                                                        }
                                                    }

                                                    /* LCOV_EXCL_START */
                                                    _ => {
                                                        errorcode = ERR89; /* Unknown code; should never occur because */
                                                        break 'failed; /* goto FAILED - the meta values come from a table above. */
                                                    }
                                                    /* LCOV_EXCL_STOP */
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
                                                    break 'failed; /* goto FAILED */
                                                }
                                                if ptr >= ptrend
                                                    || (*ptr as u32 != CHAR_COLON
                                                        && *ptr as u32
                                                            != CHAR_RIGHT_PARENTHESIS)
                                                {
                                                    errorcode = ERR60; /* Malformed */
                                                    break 'failed; /* goto FAILED */
                                                }

                                                /* Scan the table of verb names */

                                                i = 0;
                                                while i < verbcount {
                                                    if namelen == verbs[i as usize].len
                                                        && _pcre2_strncmp_c8_8(
                                                            name,
                                                            vn,
                                                            namelen as usize,
                                                        ) == 0
                                                    {
                                                        break;
                                                    }
                                                    vn = vn.add(
                                                        (verbs[i as usize].len + 1) as usize,
                                                    );
                                                    i += 1;
                                                }

                                                if i >= verbcount {
                                                    errorcode = ERR60; /* Verb not recognized */
                                                    break 'failed; /* goto FAILED */
                                                }

                                                /* An empty argument is treated as no
                                                argument. */

                                                if *ptr as u32 == CHAR_COLON
                                                    && ptr.add(1) < ptrend
                                                    && *ptr.add(1) as u32
                                                        == CHAR_RIGHT_PARENTHESIS
                                                {
                                                    ptr = ptr.add(1); /* Advance to the closing parens */
                                                }

                                                /* Check for mandatory non-empty
                                                argument; this is (*MARK) */

                                                if verbs[i as usize].has_arg > 0
                                                    && *ptr as u32 != CHAR_COLON
                                                {
                                                    errorcode = ERR66;
                                                    break 'failed; /* goto FAILED */
                                                }

                                                /* Remember where this verb, possibly
                                                with a preceding (*MARK), starts, for
                                                handling quantified (*ACCEPT). */

                                                verbstartptr = parsed_pattern;
                                                okquantifier = (verbs[i as usize].meta
                                                    == META_ACCEPT)
                                                    as BOOL;

                                                /* It appears that Perl allows any
                                                characters whatsoever, other than a
                                                closing parenthesis, to appear in
                                                arguments ("names"). We set inverbname
                                                TRUE here, and let the main loop take
                                                care of this. */

                                                if {
                                                    let t = *ptr as u32;
                                                    ptr = ptr.add(1);
                                                    t == CHAR_COLON
                                                } {
                                                    /* Skip past : or ) */

                                                    /* Some optional arguments can be
                                                    treated as a preceding (*MARK) */

                                                    if verbs[i as usize].has_arg < 0 {
                                                        add_after_mark =
                                                            verbs[i as usize].meta;
                                                        PUTPP!(parsed_pattern, META_MARK);
                                                    }
                                                    /* The remaining verbs with arguments
                                                    (except *MARK) need a different
                                                    opcode. */
                                                    else {
                                                        PUTPP!(
                                                            parsed_pattern,
                                                            verbs[i as usize].meta
                                                                + (if verbs[i as usize].meta
                                                                    != META_MARK
                                                                {
                                                                    0x00010000u32
                                                                } else {
                                                                    0
                                                                })
                                                        );
                                                    }

                                                    /* Set up for reading the name in the
                                                    main loop. */

                                                    verblengthptr = parsed_pattern;
                                                    parsed_pattern = parsed_pattern.add(1);
                                                    verbnamestart = ptr;
                                                    inverbname = TRUE;
                                                } else {
                                                    /* No verb "name" argument */
                                                    PUTPP!(
                                                        parsed_pattern,
                                                        verbs[i as usize].meta
                                                    );
                                                }
                                            } /* End of (*VERB) handling */
                                            break 'isw; /* Done with this parenthesis */
                                        } /* End of groups that don't start with (? */

                                        /* ---- Items starting (? ---- */

                                        ptr = ptr.add(1);
                                        if ptr >= ptrend {
                                            break 'unclosed_parenthesis; /* goto UNCLOSED_PARENTHESIS */
                                        }

                                        match *ptr as u32 {
                                            /* ---- Python syntax support ---- */
                                            CHAR_P => {
                                                ptr = ptr.add(1);
                                                if ptr >= ptrend {
                                                    break 'unclosed_parenthesis; /* goto UNCLOSED_PARENTHESIS */
                                                }

                                                /* (?P<name> is the same as (?<name>,
                                                which defines a named group. */

                                                if *ptr as u32 == CHAR_LESS_THAN_SIGN {
                                                    terminator = CHAR_GREATER_THAN_SIGN;
                                                    istate = S_DEFINE_NAME; /* goto DEFINE_NAME */
                                                    continue 'isw;
                                                }

                                                /* (?P>name) is the same as (?&name),
                                                which is a recursion or subroutine
                                                call. */

                                                if *ptr as u32 == CHAR_GREATER_THAN_SIGN {
                                                    istate = S_RECURSE_BY_NAME; /* goto RECURSE_BY_NAME */
                                                    continue 'isw;
                                                }

                                                /* (?P=name) is the same as \k<name>, a
                                                back reference by name. Anything else
                                                after (?P is an error. */

                                                if *ptr as u32 != CHAR_EQUALS_SIGN {
                                                    errorcode = ERR41;
                                                    fail_mode = 2;
                                                    break 'failed; /* goto FAILED_FORWARD */
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
                                                    break 'failed; /* goto FAILED */
                                                }
                                                PUTPP!(parsed_pattern, META_BACKREF_BYNAME);
                                                PUTPP!(parsed_pattern, namelen);
                                                PUTOFFSET!(offset, parsed_pattern);
                                                okquantifier = TRUE;
                                            } /* End of (?P processing */

                                            /* ---- Recursion/subroutine calls by number ---- */
                                            CHAR_R => {
                                                i = 0; /* (?R) == (?R0) */
                                                ptr = ptr.add(1);
                                                if ptr >= ptrend
                                                    || (*ptr as u32 != CHAR_RIGHT_PARENTHESIS
                                                        && *ptr as u32
                                                            != CHAR_LEFT_PARENTHESIS)
                                                {
                                                    errorcode = ERR58;
                                                    break 'failed; /* goto FAILED */
                                                }
                                                terminator = CHAR_NUL;
                                                istate = S_SET_RECURSION; /* goto SET_RECURSION */
                                                continue 'isw;
                                            }

                                            /* An item starting (?- followed by a digit
                                            comes here via the "default" case because
                                            (?- followed by a non-digit is an options
                                            setting. */
                                            CHAR_PLUS => {
                                                if ptr.add(1) >= ptrend {
                                                    ptr = ptr.add(1);
                                                    break 'unclosed_parenthesis; /* goto UNCLOSED_PARENTHESIS */
                                                }
                                                if !IS_DIGIT!(*ptr.add(1)) {
                                                    errorcode = ERR29; /* Missing number */
                                                    ptr = ptr.add(1);
                                                    fail_mode = 2;
                                                    break 'failed; /* goto FAILED_FORWARD */
                                                }
                                                /* Fall through to RECURSION_BYNUMBER */
                                                istate = S_RECURSION_BYNUMBER;
                                                continue 'isw;
                                            }

                                            CHAR_0 | CHAR_1 | CHAR_2 | CHAR_3 | CHAR_4
                                            | CHAR_5 | CHAR_6 | CHAR_7 | CHAR_8
                                            | CHAR_9 => {
                                                istate = S_RECURSION_BYNUMBER; /* RECURSION_BYNUMBER: */
                                                continue 'isw;
                                            }

                                            /* ---- Recursion/subroutine calls by name ---- */
                                            CHAR_AMPERSAND => {
                                                istate = S_RECURSE_BY_NAME; /* RECURSE_BY_NAME: */
                                                continue 'isw;
                                            }

                                            /* ---- Callout with numerical or string argument ---- */
                                            CHAR_C => {
                                                if (xoptions & PCRE2_EXTRA_NEVER_CALLOUT) != 0
                                                {
                                                    ptr = ptr.add(1);
                                                    errorcode = ERR103;
                                                    break 'failed; /* goto FAILED */
                                                }

                                                ptr = ptr.add(1);
                                                if ptr >= ptrend {
                                                    break 'unclosed_parenthesis; /* goto UNCLOSED_PARENTHESIS */
                                                }

                                                /* If the previous item was a condition
                                                starting (?(? an assertion, optionally
                                                preceded by a callout, is expected. */

                                                expect_cond_assert =
                                                    prev_expect_cond_assert - 1;

                                                /* If previous_callout is not NULL, it
                                                means this follows a previous callout. */

                                                if !previous_callout.is_null()
                                                    && (options & PCRE2_AUTO_CALLOUT) != 0
                                                    && previous_callout
                                                        == parsed_pattern.offset(-4)
                                                    && *parsed_pattern.offset(-1) == 255
                                                {
                                                    parsed_pattern = previous_callout;
                                                }

                                                /* Save for updating next pattern item
                                                length, and skip one item before
                                                completing. */

                                                previous_callout = parsed_pattern;
                                                after_manual_callout = 1;

                                                /* Handle a string argument; specific
                                                delimiter is required. */

                                                if *ptr as u32 != CHAR_RIGHT_PARENTHESIS
                                                    && !IS_DIGIT!(*ptr)
                                                {
                                                    let calloutlength: PCRE2_SIZE;
                                                    let startptr: PCRE2_SPTR = ptr;

                                                    delimiter = 0;
                                                    i = 0;
                                                    while _pcre2_callout_start_delims_8
                                                        [i as usize]
                                                        != 0
                                                    {
                                                        if *ptr as u32
                                                            == _pcre2_callout_start_delims_8
                                                                [i as usize]
                                                        {
                                                            delimiter =
                                                                _pcre2_callout_end_delims_8
                                                                    [i as usize];
                                                            break;
                                                        }
                                                        i += 1;
                                                    }
                                                    if delimiter == 0 {
                                                        errorcode = ERR82;
                                                        fail_mode = 2;
                                                        break 'failed; /* goto FAILED_FORWARD */
                                                    }

                                                    *parsed_pattern = META_CALLOUT_STRING;
                                                    parsed_pattern = parsed_pattern.add(3); /* Skip pattern info */

                                                    loop {
                                                        ptr = ptr.add(1);
                                                        if ptr >= ptrend {
                                                            errorcode = ERR81;
                                                            ptr = startptr; /* To give a more useful message */
                                                            break 'failed; /* goto FAILED */
                                                        }
                                                        if *ptr as u32 == delimiter && {
                                                            ptr = ptr.add(1);
                                                            ptr >= ptrend
                                                                || *ptr as u32 != delimiter
                                                        } {
                                                            break;
                                                        }
                                                    }

                                                    calloutlength = ptr
                                                        .offset_from(startptr)
                                                        as PCRE2_SIZE;
                                                    if calloutlength > u32::MAX as PCRE2_SIZE {
                                                        errorcode = ERR72;
                                                        break 'failed; /* goto FAILED */
                                                    }
                                                    PUTPP!(
                                                        parsed_pattern,
                                                        calloutlength as u32
                                                    );
                                                    offset = startptr
                                                        .offset_from((*cb).start_pattern)
                                                        as PCRE2_SIZE;
                                                    PUTOFFSET!(offset, parsed_pattern);
                                                }
                                                /* Handle a callout with an optional
                                                numerical argument, which must be less
                                                than or equal to 255. A missing argument
                                                gives 0. */
                                                else {
                                                    let mut n: i32 = 0;
                                                    *parsed_pattern = META_CALLOUT_NUMBER; /* Numerical callout */
                                                    parsed_pattern = parsed_pattern.add(3); /* Skip pattern info */
                                                    while ptr < ptrend && IS_DIGIT!(*ptr) {
                                                        n = n * 10
                                                            + ({
                                                                let t = *ptr as u32;
                                                                ptr = ptr.add(1);
                                                                t
                                                            } - CHAR_0)
                                                                as i32;
                                                        if n > 255 {
                                                            errorcode = ERR38;
                                                            break 'failed; /* goto FAILED */
                                                        }
                                                    }
                                                    PUTPP!(parsed_pattern, n as u32);
                                                }

                                                /* Both formats must have a closing
                                                parenthesis */

                                                if ptr >= ptrend
                                                    || *ptr as u32 != CHAR_RIGHT_PARENTHESIS
                                                {
                                                    errorcode = ERR39;
                                                    break 'failed; /* goto FAILED */
                                                }
                                                ptr = ptr.add(1);

                                                /* Remember the offset to the next item
                                                in the pattern, and set a default
                                                length. */

                                                *previous_callout.add(1) = ptr
                                                    .offset_from((*cb).start_pattern)
                                                    as u32;
                                                *previous_callout.add(2) = 0;
                                            } /* End callout */

                                            /* ---- Conditional group ---- */
                                            CHAR_LEFT_PARENTHESIS => {
                                                ptr = ptr.add(1);
                                                if ptr >= ptrend {
                                                    break 'unclosed_parenthesis; /* goto UNCLOSED_PARENTHESIS */
                                                }
                                                nest_depth += 1;

                                                /* If the next character is ? or * there
                                                must be an assertion next (optionally
                                                preceded by a callout). */

                                                if *ptr as u32 == CHAR_QUESTION_MARK
                                                    || *ptr as u32 == CHAR_ASTERISK
                                                {
                                                    PUTPP!(
                                                        parsed_pattern,
                                                        META_COND_ASSERT
                                                    );
                                                    ptr = ptr.wrapping_sub(1); /* Pull pointer back to the opening parenthesis. */
                                                    expect_cond_assert = 2;
                                                    break 'isw; /* End of conditional */
                                                }

                                                /* Handle (?([+-]number)... */

                                                if read_number(
                                                    &mut ptr,
                                                    ptrend,
                                                    (*cb).bracount as i32,
                                                    MAX_GROUP_NUMBER,
                                                    ERR61 as u32,
                                                    &mut i,
                                                    &mut errorcode,
                                                ) != 0
                                                {
                                                    if i <= 0 {
                                                        errorcode = ERR15;
                                                        break 'failed; /* goto FAILED */
                                                    }
                                                    PUTPP!(
                                                        parsed_pattern,
                                                        META_COND_NUMBER
                                                    );
                                                    offset = (ptr
                                                        .offset_from((*cb).start_pattern)
                                                        - 2) as PCRE2_SIZE;
                                                    PUTOFFSET!(offset, parsed_pattern);
                                                    PUTPP!(parsed_pattern, i as u32);
                                                } else if errorcode != 0 {
                                                    break 'failed; /* goto FAILED - Number too big */
                                                }
                                                /* No number found. Handle the special
                                                case (?(VERSION[>]=n.m)... */
                                                else if ptrend.offset_from(ptr) >= 10
                                                    && _pcre2_strncmp_c8_8(
                                                        ptr,
                                                        STRING_VERSION.as_ptr()
                                                            as *const c_char,
                                                        7,
                                                    ) == 0
                                                    && *ptr.add(7) as u32
                                                        != CHAR_RIGHT_PARENTHESIS
                                                {
                                                    let mut ge: u32 = 0;
                                                    let mut major: i32 = 0;
                                                    let mut minor: i32 = 0;

                                                    ptr = ptr.add(7);
                                                    if *ptr as u32 == CHAR_GREATER_THAN_SIGN
                                                    {
                                                        ge = 1;
                                                        ptr = ptr.add(1);
                                                    }

                                                    /* NOTE: cannot write
                                                    IS_DIGIT(*(++ptr)) here because
                                                    IS_DIGIT references its argument
                                                    twice. */

                                                    if *ptr as u32 != CHAR_EQUALS_SIGN || {
                                                        ptr = ptr.add(1);
                                                        !IS_DIGIT!(*ptr)
                                                    } {
                                                        errorcode = ERR79;
                                                        if ge == 0 {
                                                            fail_mode = 2;
                                                            break 'failed; /* goto FAILED_FORWARD */
                                                        }
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    if read_number(
                                                        &mut ptr,
                                                        ptrend,
                                                        -1,
                                                        1000,
                                                        ERR79 as u32,
                                                        &mut major,
                                                        &mut errorcode,
                                                    ) == 0
                                                    {
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    if ptr < ptrend && *ptr as u32 == CHAR_DOT
                                                    {
                                                        ptr = ptr.add(1);
                                                        if ptr >= ptrend || !IS_DIGIT!(*ptr) {
                                                            errorcode = ERR79;
                                                            if ptr < ptrend {
                                                                fail_mode = 2;
                                                                break 'failed; /* goto FAILED_FORWARD */
                                                            }
                                                            break 'failed; /* goto FAILED */
                                                        }
                                                        if read_number(
                                                            &mut ptr,
                                                            ptrend,
                                                            -1,
                                                            1000,
                                                            ERR79 as u32,
                                                            &mut minor,
                                                            &mut errorcode,
                                                        ) == 0
                                                        {
                                                            break 'failed; /* goto FAILED */
                                                        }
                                                    }
                                                    if ptr >= ptrend
                                                        || *ptr as u32
                                                            != CHAR_RIGHT_PARENTHESIS
                                                    {
                                                        errorcode = ERR79;
                                                        if ptr < ptrend {
                                                            fail_mode = 2;
                                                            break 'failed; /* goto FAILED_FORWARD */
                                                        }
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    PUTPP!(
                                                        parsed_pattern,
                                                        META_COND_VERSION
                                                    );
                                                    PUTPP!(parsed_pattern, ge);
                                                    PUTPP!(parsed_pattern, major as u32);
                                                    PUTPP!(parsed_pattern, minor as u32);
                                                }
                                                /* All the remaining cases now require us
                                                to read a name. */
                                                else {
                                                    let mut was_r_ampersand: BOOL = FALSE;

                                                    if *ptr as u32 == CHAR_R
                                                        && ptrend.offset_from(ptr) > 1
                                                        && *ptr.add(1) as u32
                                                            == CHAR_AMPERSAND
                                                    {
                                                        terminator = CHAR_RIGHT_PARENTHESIS;
                                                        was_r_ampersand = TRUE;
                                                        ptr = ptr.add(1);
                                                    } else if *ptr as u32
                                                        == CHAR_LESS_THAN_SIGN
                                                    {
                                                        terminator = CHAR_GREATER_THAN_SIGN;
                                                    } else if *ptr as u32 == CHAR_APOSTROPHE
                                                    {
                                                        terminator = CHAR_APOSTROPHE;
                                                    } else {
                                                        terminator = CHAR_RIGHT_PARENTHESIS;
                                                        ptr = ptr.wrapping_sub(1); /* Point to char before name */
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
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    /* Handle (?(R&name) */

                                                    if was_r_ampersand != 0 {
                                                        *parsed_pattern = META_COND_RNAME;
                                                        ptr = ptr.wrapping_sub(1); /* Back to closing parens */
                                                    }
                                                    /* Handle (?(name). If the name is
                                                    "DEFINE" we identify it with a
                                                    special code. */
                                                    else if terminator
                                                        == CHAR_RIGHT_PARENTHESIS
                                                    {
                                                        if namelen == 6
                                                            && _pcre2_strncmp_c8_8(
                                                                name,
                                                                STRING_DEFINE.as_ptr()
                                                                    as *const c_char,
                                                                6,
                                                            ) == 0
                                                        {
                                                            *parsed_pattern =
                                                                META_COND_DEFINE;
                                                        } else {
                                                            i = 1;
                                                            while i < namelen as i32 {
                                                                if !IS_DIGIT!(
                                                                    *name.add(i as usize)
                                                                ) {
                                                                    break;
                                                                }
                                                                i += 1;
                                                            }
                                                            *parsed_pattern = if *name as u32
                                                                == CHAR_R
                                                                && i >= namelen as i32
                                                            {
                                                                META_COND_RNUMBER
                                                            } else {
                                                                META_COND_NAME
                                                            };
                                                        }
                                                        ptr = ptr.wrapping_sub(1); /* Back to closing parens */
                                                    }
                                                    /* Handle (?('name') or (?(<name>) */
                                                    else {
                                                        *parsed_pattern = META_COND_NAME;
                                                    }

                                                    /* All these cases except DEFINE end
                                                    with the name length and offset;
                                                    DEFINE just has an offset (for the
                                                    "too many branches" error). */

                                                    if {
                                                        let t = *parsed_pattern;
                                                        parsed_pattern =
                                                            parsed_pattern.add(1);
                                                        t
                                                    } != META_COND_DEFINE
                                                    {
                                                        PUTPP!(parsed_pattern, namelen);
                                                    }
                                                    PUTOFFSET!(offset, parsed_pattern);
                                                } /* End cases that read a name */

                                                /* Check the closing parenthesis of the
                                                condition */

                                                if ptr >= ptrend
                                                    || *ptr as u32 != CHAR_RIGHT_PARENTHESIS
                                                {
                                                    errorcode = ERR24;
                                                    break 'failed; /* goto FAILED */
                                                }
                                                ptr = ptr.add(1);
                                            } /* End of condition processing */

                                            /* ---- Atomic group ---- */
                                            CHAR_GREATER_THAN_SIGN => {
                                                istate = S_ATOMIC_GROUP; /* ATOMIC_GROUP: */
                                                continue 'isw;
                                            }

                                            /* ---- Lookahead assertions ---- */
                                            CHAR_EQUALS_SIGN => {
                                                istate = S_POSITIVE_LOOK_AHEAD; /* POSITIVE_LOOK_AHEAD: */
                                                continue 'isw;
                                            }

                                            CHAR_ASTERISK => {
                                                istate = S_POSITIVE_NONATOMIC_LOOK_AHEAD; /* POSITIVE_NONATOMIC_LOOK_AHEAD: */
                                                continue 'isw;
                                            }

                                            CHAR_EXCLAMATION_MARK => {
                                                istate = S_NEGATIVE_LOOK_AHEAD; /* NEGATIVE_LOOK_AHEAD: */
                                                continue 'isw;
                                            }

                                            /* ---- Lookbehind assertions ---- */

                                            /* (?< followed by = or ! or * is a
                                            lookbehind assertion. Otherwise (?< is the
                                            start of the name of a capturing group. */
                                            CHAR_LESS_THAN_SIGN => {
                                                if ptrend.offset_from(ptr) <= 1
                                                    || (*ptr.add(1) as u32
                                                        != CHAR_EQUALS_SIGN
                                                        && *ptr.add(1) as u32
                                                            != CHAR_EXCLAMATION_MARK
                                                        && *ptr.add(1) as u32
                                                            != CHAR_ASTERISK)
                                                {
                                                    terminator = CHAR_GREATER_THAN_SIGN;
                                                    istate = S_DEFINE_NAME; /* goto DEFINE_NAME */
                                                    continue 'isw;
                                                }
                                                PUTPP!(
                                                    parsed_pattern,
                                                    if *ptr.add(1) as u32 == CHAR_EQUALS_SIGN
                                                    {
                                                        META_LOOKBEHIND
                                                    } else if *ptr.add(1) as u32
                                                        == CHAR_EXCLAMATION_MARK
                                                    {
                                                        META_LOOKBEHINDNOT
                                                    } else {
                                                        META_LOOKBEHIND_NA
                                                    }
                                                );

                                                /* Falls into POST_LOOKBEHIND */
                                                istate = S_POST_LOOKBEHIND;
                                                continue 'isw;
                                            }

                                            /* ---- Define a named group ---- */

                                            /* A named group may be defined as
                                            (?'name') or (?<name>). */
                                            CHAR_APOSTROPHE => {
                                                terminator = CHAR_APOSTROPHE; /* Terminator */
                                                /* Falls into DEFINE_NAME */
                                                istate = S_DEFINE_NAME;
                                                continue 'isw;
                                            }

                                            /* ---- Perl extended character class ---- */

                                            /* These are of the form '(?[...])'. */
                                            CHAR_LEFT_SQUARE_BRACKET => {
                                                class_mode_state = CLASS_MODE_PERL_EXT;
                                                c = {
                                                    let t = *ptr as u32;
                                                    ptr = ptr.add(1);
                                                    t
                                                };
                                                istate = S_FROM_PERL_EXTENDED_CLASS; /* goto FROM_PERL_EXTENDED_CLASS */
                                                continue 'isw;
                                            }

                                            _ => {
                                                if *ptr as u32 == CHAR_MINUS
                                                    && ptrend.offset_from(ptr) > 1
                                                    && IS_DIGIT!(*ptr.add(1))
                                                {
                                                    istate = S_RECURSION_BYNUMBER; /* goto RECURSION_BYNUMBER - The + case is handled by CHAR_PLUS */
                                                    continue 'isw;
                                                }

                                                /* We now have either (?| or a (possibly
                                                empty) option setting, optionally
                                                followed by a non-capturing group. */

                                                nest_depth += 1;
                                                if top_nest.is_null() {
                                                    top_nest = (*cb).start_workspace
                                                        as *mut nest_save;
                                                } else {
                                                    top_nest = top_nest.add(1);
                                                    if top_nest >= end_nests {
                                                        errorcode = ERR84;
                                                        break 'failed; /* goto FAILED */
                                                    }
                                                }
                                                (*top_nest).nest_depth = nest_depth;
                                                (*top_nest).flags = 0;
                                                (*top_nest).options =
                                                    options & PARSE_TRACKED_OPTIONS;
                                                (*top_nest).xoptions =
                                                    xoptions & PARSE_TRACKED_EXTRA_OPTIONS;

                                                /* Start of non-capturing group that
                                                resets the capture count for each
                                                branch. */

                                                if *ptr as u32 == CHAR_VERTICAL_LINE {
                                                    (*top_nest).reset_group =
                                                        (*cb).bracount as u16;
                                                    (*top_nest).max_group =
                                                        (*cb).bracount as u16;
                                                    (*top_nest).flags |= NSF_RESET;
                                                    (*cb).external_flags |=
                                                        PCRE2_DUPCAPUSED;
                                                    PUTPP!(parsed_pattern, META_NOCAPTURE);
                                                    ptr = ptr.add(1);
                                                }
                                                /* Scan for options imnrsxJU to be set or
                                                unset. */
                                                else {
                                                    let mut hyphenok: BOOL = TRUE;
                                                    let oldoptions: u32 = options;
                                                    let oldxoptions: u32 = xoptions;

                                                    (*top_nest).reset_group = 0;
                                                    (*top_nest).max_group = 0;
                                                    unset = 0;
                                                    set = unset;
                                                    optset = &mut set;
                                                    xunset = 0;
                                                    xset = xunset;
                                                    xoptset = &mut xset;

                                                    /* ^ at the start unsets irmnsx and
                                                    disables the subsequent use of - */

                                                    if ptr < ptrend
                                                        && *ptr as u32
                                                            == CHAR_CIRCUMFLEX_ACCENT
                                                    {
                                                        options &= !(PCRE2_CASELESS
                                                            | PCRE2_MULTILINE
                                                            | PCRE2_NO_AUTO_CAPTURE
                                                            | PCRE2_DOTALL
                                                            | PCRE2_EXTENDED
                                                            | PCRE2_EXTENDED_MORE);
                                                        xoptions &=
                                                            !(PCRE2_EXTRA_CASELESS_RESTRICT);
                                                        hyphenok = FALSE;
                                                        ptr = ptr.add(1);
                                                    }

                                                    while ptr < ptrend
                                                        && *ptr as u32
                                                            != CHAR_RIGHT_PARENTHESIS
                                                        && *ptr as u32 != CHAR_COLON
                                                    {
                                                        match {
                                                            let t = *ptr as u32;
                                                            ptr = ptr.add(1);
                                                            t
                                                        } {
                                                            CHAR_MINUS => {
                                                                if hyphenok == 0 {
                                                                    errorcode = ERR94;
                                                                    break 'failed; /* goto FAILED */
                                                                }
                                                                optset = &mut unset;
                                                                xoptset = &mut xunset;
                                                                hyphenok = FALSE;
                                                            }

                                                            /* There are some two-character
                                                            sequences that start with 'a'. */
                                                            CHAR_a => {
                                                                let mut handled = false;
                                                                if ptr < ptrend {
                                                                    if *ptr as u32 == CHAR_D {
                                                                        *xoptset |=
                                                                            PCRE2_EXTRA_ASCII_BSD;
                                                                        ptr = ptr.add(1);
                                                                        handled = true;
                                                                    } else if *ptr as u32
                                                                        == CHAR_P
                                                                    {
                                                                        *xoptset |= PCRE2_EXTRA_ASCII_POSIX
                                                                            | PCRE2_EXTRA_ASCII_DIGIT;
                                                                        ptr = ptr.add(1);
                                                                        handled = true;
                                                                    } else if *ptr as u32
                                                                        == CHAR_S
                                                                    {
                                                                        *xoptset |=
                                                                            PCRE2_EXTRA_ASCII_BSS;
                                                                        ptr = ptr.add(1);
                                                                        handled = true;
                                                                    } else if *ptr as u32
                                                                        == CHAR_T
                                                                    {
                                                                        *xoptset |=
                                                                            PCRE2_EXTRA_ASCII_DIGIT;
                                                                        ptr = ptr.add(1);
                                                                        handled = true;
                                                                    } else if *ptr as u32
                                                                        == CHAR_W
                                                                    {
                                                                        *xoptset |=
                                                                            PCRE2_EXTRA_ASCII_BSW;
                                                                        ptr = ptr.add(1);
                                                                        handled = true;
                                                                    }
                                                                }
                                                                if !handled {
                                                                    *xoptset |=
                                                                        PCRE2_EXTRA_ASCII_BSD
                                                                            | PCRE2_EXTRA_ASCII_BSS
                                                                            | PCRE2_EXTRA_ASCII_BSW
                                                                            | PCRE2_EXTRA_ASCII_DIGIT
                                                                            | PCRE2_EXTRA_ASCII_POSIX;
                                                                }
                                                            }

                                                            /* Record that it changed in
                                                            the external options */
                                                            CHAR_J => {
                                                                *optset |= PCRE2_DUPNAMES;
                                                                (*cb).external_flags |=
                                                                    PCRE2_JCHANGED;
                                                            }

                                                            CHAR_i => {
                                                                *optset |= PCRE2_CASELESS;
                                                            }
                                                            CHAR_m => {
                                                                *optset |= PCRE2_MULTILINE;
                                                            }
                                                            CHAR_n => {
                                                                *optset |=
                                                                    PCRE2_NO_AUTO_CAPTURE;
                                                            }
                                                            CHAR_r => {
                                                                *xoptset |=
                                                                    PCRE2_EXTRA_CASELESS_RESTRICT;
                                                            }
                                                            CHAR_s => {
                                                                *optset |= PCRE2_DOTALL;
                                                            }
                                                            CHAR_U => {
                                                                *optset |= PCRE2_UNGREEDY;
                                                            }

                                                            /* If x appears twice it sets
                                                            the extended extended option. */
                                                            CHAR_x => {
                                                                *optset |= PCRE2_EXTENDED;
                                                                if ptr < ptrend
                                                                    && *ptr as u32 == CHAR_x
                                                                {
                                                                    *optset |=
                                                                        PCRE2_EXTENDED_MORE;
                                                                    ptr = ptr.add(1);
                                                                }
                                                            }

                                                            _ => {
                                                                errorcode = ERR11;
                                                                break 'failed; /* goto FAILED */
                                                            }
                                                        }
                                                    }

                                                    /* If we are setting extended without
                                                    extended-more, ensure that any
                                                    existing extended-more gets unset. */

                                                    if (set
                                                        & (PCRE2_EXTENDED
                                                            | PCRE2_EXTENDED_MORE))
                                                        == PCRE2_EXTENDED
                                                        || (unset & PCRE2_EXTENDED) != 0
                                                    {
                                                        unset |= PCRE2_EXTENDED_MORE;
                                                    }

                                                    options = (options | set) & (!unset);
                                                    xoptions = (xoptions | xset) & (!xunset);

                                                    /* If the options ended with ')' this
                                                    is not the start of a nested group
                                                    with option changes, so the options
                                                    change at this level. */

                                                    if ptr >= ptrend {
                                                        break 'unclosed_parenthesis; /* goto UNCLOSED_PARENTHESIS */
                                                    }
                                                    if {
                                                        let t = *ptr as u32;
                                                        ptr = ptr.add(1);
                                                        t == CHAR_RIGHT_PARENTHESIS
                                                    } {
                                                        nest_depth -= 1; /* This is not a nested group after all. */
                                                        if top_nest
                                                            > (*cb).start_workspace
                                                                as *mut nest_save
                                                            && (*top_nest.sub(1)).nest_depth
                                                                == nest_depth
                                                        {
                                                            top_nest = top_nest.sub(1);
                                                        } else {
                                                            (*top_nest).nest_depth =
                                                                nest_depth;
                                                        }
                                                    } else {
                                                        PUTPP!(
                                                            parsed_pattern,
                                                            META_NOCAPTURE
                                                        );
                                                    }

                                                    /* If nothing changed, no need to
                                                    record. */

                                                    if options != oldoptions
                                                        || xoptions != oldxoptions
                                                    {
                                                        PUTPP!(parsed_pattern, META_OPTIONS);
                                                        PUTPP!(parsed_pattern, options);
                                                        PUTPP!(parsed_pattern, xoptions);
                                                    }
                                                } /* End options processing */
                                            } /* End default case after (? */
                                        } /* End of (? switch */
                                    } /* End of ( handling */

                                    /* ---- Branch terminators ---- */

                                    /* Alternation: reset the capture count if we are in
                                    a (?| group. */
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
                                        PUTPP!(parsed_pattern, META_ALT);
                                    }

                                    /* End of group; reset the capture count to the
                                    maximum if we are in a (?| group and/or reset the
                                    options that are tracked during parsing. Disallow
                                    quantifier for a condition that is an assertion. */
                                    CHAR_RIGHT_PARENTHESIS => {
                                        okquantifier = TRUE;
                                        if !top_nest.is_null()
                                            && (*top_nest).nest_depth == nest_depth
                                        {
                                            options = (options & !PARSE_TRACKED_OPTIONS)
                                                | (*top_nest).options;
                                            xoptions = (xoptions
                                                & !PARSE_TRACKED_EXTRA_OPTIONS)
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
                                                PUTPP!(parsed_pattern, META_KET);
                                            }

                                            if top_nest
                                                == (*cb).start_workspace as *mut nest_save
                                            {
                                                top_nest = core::ptr::null_mut();
                                            } else {
                                                top_nest = top_nest.sub(1);
                                            }
                                        }
                                        if nest_depth == 0 {
                                            /* Unmatched closing parenthesis */
                                            errorcode = ERR22;
                                            break 'failed; /* goto FAILED */
                                        }
                                        nest_depth -= 1;
                                        PUTPP!(parsed_pattern, META_KET);
                                    }

                                    /* Non-special character */
                                    _ => {
                                        PARSED_LITERAL!(c, parsed_pattern, okquantifier);
                                    }
                                }
                                break 'isw; /* End of switch on pattern character */
                            }

                            /* ESCAPE_FAILED: (in the middle of case CHAR_BACKSLASH) */
                            S_ESCAPE_FAILED => {
                                if (xoptions & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL) == 0 {
                                    break 'failed; /* goto FAILED */
                                }
                                ptr = tempptr;
                                if ptr >= ptrend {
                                    c = CHAR_BACKSLASH;
                                } else {
                                    /* Get character value, increment pointer */
                                    GETCHARINCTEST!(c, ptr, utf);
                                }
                                escape = 0; /* Treat as literal character */
                                istate = S_ESCAPE_TAIL;
                                continue 'isw;
                            }

                            /* The code that follows the ESCAPE_FAILED block. */
                            S_ESCAPE_TAIL => {
                                /* The escape was a data escape or literal character. */

                                if escape == 0 {
                                    PARSED_LITERAL!(c, parsed_pattern, okquantifier);
                                }
                                /* The escape was a back (or forward) reference. */
                                else if escape < 0 {
                                    offset =
                                        ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                                    escape = -escape - 1;
                                    PUTPP!(parsed_pattern, META_BACKREF | escape as u32);
                                    if escape < 10 {
                                        if *(*cb).small_ref_offset.as_mut_ptr()
                                            .add(escape as usize)
                                            == PCRE2_UNSET
                                        {
                                            *(*cb).small_ref_offset.as_mut_ptr()
                                                .add(escape as usize) = offset;
                                        }
                                    } else {
                                        PUTOFFSET!(offset, parsed_pattern);
                                    }
                                    okquantifier = TRUE;
                                }
                                /* The escape was a character class such as \d etc. or
                                other special escape indicator such as \A or \X. */
                                else {
                                    match escape as u32 {
                                        ESC_C => {
                                            if (options & PCRE2_NEVER_BACKSLASH_C) != 0 {
                                                errorcode = ERR83;
                                                istate = S_ESCAPE_FAILED; /* goto ESCAPE_FAILED */
                                                continue 'isw;
                                            }
                                            okquantifier = TRUE;
                                            PUTPP!(
                                                parsed_pattern,
                                                META_ESCAPE + escape as u32
                                            );
                                        }

                                        /* This is a special return that happens only in
                                        EXTRA_ALT_BSUX mode, when \u{ is not followed by
                                        hex digits and }. */

                                        ESC_ub => {
                                            PUTPP!(parsed_pattern, CHAR_u);
                                            PARSED_LITERAL!(
                                                CHAR_LEFT_CURLY_BRACKET,
                                                parsed_pattern,
                                                okquantifier
                                            );
                                        }

                                        ESC_X | ESC_H | ESC_h | ESC_N | ESC_R | ESC_V
                                        | ESC_v => {
                                            okquantifier = TRUE;
                                            PUTPP!(
                                                parsed_pattern,
                                                META_ESCAPE + escape as u32
                                            );
                                        }

                                        /* Escapes that may change in UCP mode. */

                                        ESC_d | ESC_D | ESC_s | ESC_S | ESC_w | ESC_W => {
                                            okquantifier = TRUE;
                                            parsed_pattern = handle_escdsw(
                                                escape,
                                                parsed_pattern,
                                                options,
                                                xoptions,
                                            );
                                        }

                                        /* Unicode property matching */

                                        ESC_P | ESC_p => {
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
                                            ) == 0
                                            {
                                                istate = S_ESCAPE_FAILED; /* goto ESCAPE_FAILED */
                                                continue 'isw;
                                            }
                                            if negated != 0 {
                                                escape = if escape as u32 == ESC_P {
                                                    ESC_p as i32
                                                } else {
                                                    ESC_P as i32
                                                };
                                            }
                                            PUTPP!(
                                                parsed_pattern,
                                                META_ESCAPE + escape as u32
                                            );
                                            PUTPP!(
                                                parsed_pattern,
                                                ((ptype as u32) << 16) | pdata as u32
                                            );
                                            okquantifier = TRUE;
                                        } /* End \P and \p */

                                        /* When \g is used with quotes or angle brackets
                                        as delimiters, it is a numerical or named
                                        subroutine call. \k is always a named back
                                        reference. */

                                        ESC_g | ESC_k => {
                                            if ptr >= ptrend
                                                || (*ptr as u32 != CHAR_LEFT_CURLY_BRACKET
                                                    && *ptr as u32 != CHAR_LESS_THAN_SIGN
                                                    && *ptr as u32 != CHAR_APOSTROPHE)
                                            {
                                                errorcode = if escape as u32 == ESC_g {
                                                    ERR57
                                                } else {
                                                    ERR69
                                                };
                                                istate = S_ESCAPE_FAILED; /* goto ESCAPE_FAILED */
                                                continue 'isw;
                                            }
                                            terminator = if *ptr as u32 == CHAR_LESS_THAN_SIGN
                                            {
                                                CHAR_GREATER_THAN_SIGN
                                            } else if *ptr as u32 == CHAR_APOSTROPHE {
                                                CHAR_APOSTROPHE
                                            } else {
                                                CHAR_RIGHT_CURLY_BRACKET
                                            };

                                            /* For a non-braced \g, check for a numerical
                                            recursion. */

                                            if escape as u32 == ESC_g
                                                && terminator != CHAR_RIGHT_CURLY_BRACKET
                                            {
                                                let mut p: PCRE2_SPTR = ptr.add(1);

                                                if read_number(
                                                    &mut p,
                                                    ptrend,
                                                    (*cb).bracount as i32,
                                                    MAX_GROUP_NUMBER,
                                                    ERR61 as u32,
                                                    &mut i,
                                                    &mut errorcode,
                                                ) != 0
                                                {
                                                    if p >= ptrend || *p as u32 != terminator {
                                                        ptr = p;
                                                        errorcode = ERR119; /* Missing terminator for number */
                                                        istate = S_ESCAPE_FAILED; /* goto ESCAPE_FAILED */
                                                        continue 'isw;
                                                    }
                                                    ptr = p.add(1);
                                                    istate = S_SET_RECURSION; /* goto SET_RECURSION */
                                                    continue 'isw;
                                                }
                                                if errorcode != 0 {
                                                    istate = S_ESCAPE_FAILED; /* goto ESCAPE_FAILED */
                                                    continue 'isw;
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
                                                istate = S_ESCAPE_FAILED; /* goto ESCAPE_FAILED */
                                                continue 'isw;
                                            }

                                            /* \k and \g when used with braces are back
                                            references, whereas \g used with quotes or
                                            angle brackets is a recursion */

                                            PUTPP!(
                                                parsed_pattern,
                                                if escape as u32 == ESC_k
                                                    || terminator
                                                        == CHAR_RIGHT_CURLY_BRACKET
                                                {
                                                    META_BACKREF_BYNAME
                                                } else {
                                                    META_RECURSE_BYNAME
                                                }
                                            );
                                            PUTPP!(parsed_pattern, namelen);

                                            PUTOFFSET!(offset, parsed_pattern);
                                            okquantifier = TRUE;
                                        } /* End special escape processing */

                                        /* \A, \B, \b, \G, \K, \Z, \z cannot be
                                        quantified. */
                                        _ => {
                                            PUTPP!(
                                                parsed_pattern,
                                                META_ESCAPE + escape as u32
                                            );
                                        }
                                    }
                                }
                                break 'isw; /* End escape sequence processing */
                            }

                            /* ---- Quantifier post-processing ---- */

                            /* Check that a quantifier is allowed after the previous
                            item. This guarantees that there is a previous item. */

                            /* CHECK_QUANTIFIER: */
                            S_CHECK_QUANTIFIER => {
                                if prev_okquantifier == 0 {
                                    errorcode = ERR9;
                                    break 'failed; /* goto FAILED */
                                }

                                /* Most (*VERB)s are not allowed to be quantified, but an
                                ungreedy quantifier can be useful for (*ACCEPT). We
                                therefore allow (*ACCEPT) to be quantified by wrapping it
                                in non-capturing brackets. */

                                if *prev_parsed_item == META_ACCEPT {
                                    let mut p: *mut u32 = parsed_pattern.wrapping_sub(1);
                                    while p >= verbstartptr {
                                        *p.add(1) = *p.add(0);
                                        p = p.wrapping_sub(1);
                                    }
                                    *verbstartptr = META_NOCAPTURE;
                                    *parsed_pattern.add(1) = META_KET;
                                    parsed_pattern = parsed_pattern.add(2);
                                }

                                /* Now we can put the quantifier into the parsed pattern
                                vector. */

                                PUTPP!(parsed_pattern, meta_quantifier);
                                if c == CHAR_LEFT_CURLY_BRACKET {
                                    PUTPP!(parsed_pattern, min_repeat);
                                    PUTPP!(parsed_pattern, max_repeat);
                                }
                                break 'isw;
                            }

                            /* Jump here from '(?[...])'. That jump must initialize
                            class_mode_state, set c to the '[' character, and ptr to just
                            after the '['. */

                            /* FROM_PERL_EXTENDED_CLASS: */
                            S_FROM_PERL_EXTENDED_CLASS => {
                                okquantifier = TRUE;

                                /* Loop for the contents of the class. Classes may be
                                nested, if PCRE2_ALT_EXTENDED_CLASS is set, or the class
                                is of the form (?[...]). */

                                /* c is still set to '[' so the loop will handle the start
                                of the class. */

                                class_depth_m1 = -1;
                                class_maxdepth_m1 = -1;
                                class_range_state = RANGE_NO;
                                class_op_state = CLASS_OP_EMPTY;
                                class_start = core::ptr::null_mut();

                                'class_loop: loop {
                                    let mut char_is_literal: BOOL = TRUE;

                                    let mut clstate: u32 = CL_TOP;
                                    'clsm: loop {
                                        match clstate {
                                            CL_TOP => {
                                                /* Inside \Q...\E everything is literal
                                                except \E */

                                                if inescq != 0 {
                                                    if c == CHAR_BACKSLASH
                                                        && ptr < ptrend
                                                        && *ptr as u32 == CHAR_E
                                                    {
                                                        inescq = FALSE; /* Reset literal state */
                                                        ptr = ptr.add(1); /* Skip the 'E' */
                                                        clstate = CL_CONTINUE; /* goto CLASS_CONTINUE */
                                                        continue 'clsm;
                                                    }

                                                    /* Surprisingly, you cannot use \Q..\E
                                                    to escape a character inside a Perl
                                                    extended class. */

                                                    if class_mode_state
                                                        == CLASS_MODE_PERL_EXT
                                                    {
                                                        errorcode = ERR116;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    clstate = CL_LITERAL; /* goto CLASS_LITERAL */
                                                    continue 'clsm;
                                                }

                                                /* Skip over space and tab (only) in
                                                extended-more mode, or anywhere inside a
                                                Perl extended class. */

                                                if (c == CHAR_SPACE || c == CHAR_HT)
                                                    && ((options & PCRE2_EXTENDED_MORE) != 0
                                                        || class_mode_state
                                                            >= CLASS_MODE_PERL_EXT)
                                                {
                                                    clstate = CL_CONTINUE; /* goto CLASS_CONTINUE */
                                                    continue 'clsm;
                                                }

                                                /* Handle POSIX class names. */

                                                if class_depth_m1 >= 0
                                                    && c == CHAR_LEFT_SQUARE_BRACKET
                                                    && ptrend.offset_from(ptr) >= 3
                                                    && (*ptr as u32 == CHAR_COLON
                                                        || *ptr as u32 == CHAR_DOT
                                                        || *ptr as u32 == CHAR_EQUALS_SIGN)
                                                    && check_posix_syntax(
                                                        ptr,
                                                        ptrend,
                                                        &mut tempptr,
                                                    ) != 0
                                                {
                                                    let mut posix_negate: BOOL = FALSE;
                                                    let posix_class: i32;

                                                    /* Perl treats a hyphen before a POSIX
                                                    class as a literal, not the start of a
                                                    range. PCRE gives an error. */

                                                    if class_range_state == RANGE_STARTED {
                                                        ptr = tempptr.add(2);
                                                        errorcode = ERR50;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    /* Roll back to the hyphen for the
                                                    error position. */

                                                    if class_range_state
                                                        == RANGE_FORBID_STARTED
                                                    {
                                                        ptr = class_range_forbid_ptr;
                                                        errorcode = ERR50;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    /* Disallow implicit union in Perl
                                                    extended classes. */

                                                    if class_op_state == CLASS_OP_OPERAND
                                                        && class_mode_state
                                                            == CLASS_MODE_PERL_EXT
                                                    {
                                                        ptr = tempptr.add(2);
                                                        errorcode = ERR113;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    if *ptr as u32 != CHAR_COLON {
                                                        ptr = tempptr.add(2);
                                                        errorcode = ERR13;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    ptr = ptr.add(1);
                                                    if *ptr as u32 == CHAR_CIRCUMFLEX_ACCENT
                                                    {
                                                        posix_negate = TRUE;
                                                        ptr = ptr.add(1);
                                                    }

                                                    posix_class = check_posix_name(
                                                        ptr,
                                                        tempptr.offset_from(ptr) as i32,
                                                    );
                                                    ptr = tempptr.add(2);
                                                    if posix_class < 0 {
                                                        errorcode = ERR30;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    /* Set "a hyphen is forbidden to be the
                                                    start of a range". */

                                                    class_range_state = RANGE_FORBID_NO;
                                                    class_op_state = CLASS_OP_OPERAND;

                                                    /* When PCRE2_UCP is set, unless
                                                    PCRE2_EXTRA_ASCII_POSIX is set, some of
                                                    the POSIX classes are converted to use
                                                    Unicode properties. */

                                                    if (options & PCRE2_UCP) != 0
                                                        && (xoptions
                                                            & PCRE2_EXTRA_ASCII_POSIX)
                                                            == 0
                                                        && !((xoptions
                                                            & PCRE2_EXTRA_ASCII_DIGIT)
                                                            != 0
                                                            && (posix_class
                                                                == PC_DIGIT as i32
                                                                || posix_class
                                                                    == PC_XDIGIT as i32))
                                                    {
                                                        let ptype: i32 = posix_substitutes
                                                            [(2 * posix_class) as usize];
                                                        let pvalue: i32 = posix_substitutes
                                                            [(2 * posix_class + 1) as usize];

                                                        if ptype >= 0 {
                                                            PUTPP!(
                                                                parsed_pattern,
                                                                META_ESCAPE
                                                                    + if posix_negate != 0 {
                                                                        ESC_P
                                                                    } else {
                                                                        ESC_p
                                                                    }
                                                            );
                                                            PUTPP!(
                                                                parsed_pattern,
                                                                ((ptype as u32) << 16)
                                                                    | pvalue as u32
                                                            );
                                                            clstate = CL_CONTINUE; /* goto CLASS_CONTINUE */
                                                            continue 'clsm;
                                                        }

                                                        if pvalue != 0 {
                                                            PUTPP!(
                                                                parsed_pattern,
                                                                META_ESCAPE
                                                                    + if posix_negate != 0 {
                                                                        ESC_H
                                                                    } else {
                                                                        ESC_h
                                                                    }
                                                            );
                                                            clstate = CL_CONTINUE; /* goto CLASS_CONTINUE */
                                                            continue 'clsm;
                                                        }

                                                        /* Fall through */
                                                    }

                                                    /* Non-UCP POSIX class */

                                                    PUTPP!(
                                                        parsed_pattern,
                                                        if posix_negate != 0 {
                                                            META_POSIX_NEG
                                                        } else {
                                                            META_POSIX
                                                        }
                                                    );
                                                    PUTPP!(
                                                        parsed_pattern,
                                                        posix_class as u32
                                                    );
                                                }
                                                /* Check for the start of the outermost
                                                class, or the start of a nested class. */
                                                else if (c == CHAR_LEFT_SQUARE_BRACKET
                                                    && (class_depth_m1 < 0
                                                        || class_mode_state
                                                            == CLASS_MODE_ALT_EXT
                                                        || class_mode_state
                                                            == CLASS_MODE_PERL_EXT))
                                                    || (c == CHAR_LEFT_PARENTHESIS
                                                        && class_mode_state
                                                            == CLASS_MODE_PERL_EXT)
                                                {
                                                    let start_c: u32 = c;
                                                    let new_class_mode_state: u32;

                                                    /* Update the class mode, if moving
                                                    into a 'leaf' inside a Perl extended
                                                    class. */

                                                    if start_c == CHAR_LEFT_SQUARE_BRACKET
                                                        && class_mode_state
                                                            == CLASS_MODE_PERL_EXT
                                                        && class_depth_m1 >= 0
                                                    {
                                                        new_class_mode_state =
                                                            CLASS_MODE_PERL_EXT_LEAF;
                                                    } else {
                                                        new_class_mode_state =
                                                            class_mode_state;
                                                    }

                                                    /* Tidy up the other class before
                                                    starting the nested class. */
                                                    /* -[ beginning a nested class is a
                                                    literal '-' */

                                                    if class_range_state == RANGE_STARTED {
                                                        *parsed_pattern.offset(-1) =
                                                            CHAR_MINUS;
                                                    }

                                                    /* Disallow implicit union in Perl
                                                    extended classes. */

                                                    if class_op_state == CLASS_OP_OPERAND
                                                        && class_mode_state
                                                            == CLASS_MODE_PERL_EXT
                                                    {
                                                        errorcode = ERR113;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    /* Validate nesting depth */
                                                    if class_depth_m1
                                                        >= (ECLASS_NEST_LIMIT - 1) as i16
                                                    {
                                                        ptr = ptr.wrapping_sub(1); /* Point rightwards at the paren, same as ERR19. */
                                                        errorcode = ERR107; /* Classes too deeply nested */
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    /* Process the character class start.
                                                    If the first character is '^', set the
                                                    negation flag. */

                                                    negate_class = FALSE;
                                                    'negloop: loop {
                                                        if ptr >= ptrend {
                                                            if start_c
                                                                == CHAR_LEFT_PARENTHESIS
                                                            {
                                                                errorcode = ERR14; /* Missing terminating ')' */
                                                            } else {
                                                                errorcode = ERR6; /* Missing terminating ']' */
                                                            }
                                                            break 'failed; /* goto FAILED */
                                                        }

                                                        GETCHARINCTEST!(c, ptr, utf);
                                                        if new_class_mode_state
                                                            == CLASS_MODE_PERL_EXT
                                                        {
                                                            break 'negloop;
                                                        } else if c == CHAR_BACKSLASH {
                                                            if ptr < ptrend
                                                                && *ptr as u32 == CHAR_E
                                                            {
                                                                ptr = ptr.add(1);
                                                            } else if ptrend.offset_from(ptr)
                                                                >= 3
                                                                && _pcre2_strncmp_c8_8(
                                                                    ptr,
                                                                    STR_Q_BACKSLASH_E
                                                                        .as_ptr()
                                                                        as *const c_char,
                                                                    3,
                                                                ) == 0
                                                            {
                                                                ptr = ptr.add(3);
                                                            } else {
                                                                break 'negloop;
                                                            }
                                                        } else if (c == CHAR_SPACE
                                                            || c == CHAR_HT)
                                                            && ((options
                                                                & PCRE2_EXTENDED_MORE)
                                                                != 0
                                                                || new_class_mode_state
                                                                    >= CLASS_MODE_PERL_EXT)
                                                        {
                                                            /* Note: just these two */
                                                            continue 'negloop;
                                                        } else if negate_class == 0
                                                            && c == CHAR_CIRCUMFLEX_ACCENT
                                                        {
                                                            negate_class = TRUE;
                                                        } else {
                                                            break 'negloop;
                                                        }
                                                    }

                                                    /* Now the real contents of the class;
                                                    c has the first "real" character. Empty
                                                    classes are permitted only if the
                                                    option is set, and if it's not a
                                                    Perl-extended class. */

                                                    if c == CHAR_RIGHT_SQUARE_BRACKET
                                                        && ((*cb).external_options
                                                            & PCRE2_ALLOW_EMPTY_CLASS)
                                                            != 0
                                                        && new_class_mode_state
                                                            < CLASS_MODE_PERL_EXT
                                                    {
                                                        if !class_start.is_null() {
                                                            /* Represents that the class is
                                                            an extended class. */
                                                            *class_start |= CLASS_IS_ECLASS;
                                                            class_start =
                                                                core::ptr::null_mut();
                                                        }

                                                        PUTPP!(
                                                            parsed_pattern,
                                                            if negate_class != 0 {
                                                                META_CLASS_EMPTY_NOT
                                                            } else {
                                                                META_CLASS_EMPTY
                                                            }
                                                        );

                                                        /* Leave nesting depth unchanged;
                                                        but check for zero depth to handle
                                                        the very first (top-level) class
                                                        being empty. */
                                                        if class_depth_m1 < 0 {
                                                            break 'class_loop;
                                                        }

                                                        class_range_state = RANGE_NO; /* for processing the containing class */
                                                        class_op_state = CLASS_OP_OPERAND;
                                                        clstate = CL_CONTINUE; /* goto CLASS_CONTINUE */
                                                        continue 'clsm;
                                                    }

                                                    /* Enter a non-empty class. */

                                                    if !class_start.is_null() {
                                                        /* Represents that the class is an
                                                        extended class. */
                                                        *class_start |= CLASS_IS_ECLASS;
                                                        class_start = core::ptr::null_mut();
                                                    }

                                                    class_start = parsed_pattern;
                                                    PUTPP!(
                                                        parsed_pattern,
                                                        if negate_class != 0 {
                                                            META_CLASS_NOT
                                                        } else {
                                                            META_CLASS
                                                        }
                                                    );
                                                    class_range_state = RANGE_NO;
                                                    class_op_state = CLASS_OP_EMPTY;
                                                    class_mode_state = new_class_mode_state;
                                                    class_depth_m1 += 1;
                                                    if class_maxdepth_m1 < class_depth_m1 {
                                                        class_maxdepth_m1 = class_depth_m1;
                                                    }
                                                    /* Reset; no op seen yet at new depth. */
                                                    *(*cb).class_op_used.as_mut_ptr()
                                                        .add(class_depth_m1 as usize) = 0;

                                                    /* Implement the special
                                                    start-of-class literal meaning of ']'. */
                                                    if c == CHAR_RIGHT_SQUARE_BRACKET
                                                        && new_class_mode_state
                                                            != CLASS_MODE_PERL_EXT
                                                    {
                                                        class_range_state = RANGE_OK_LITERAL;
                                                        class_op_state = CLASS_OP_OPERAND;
                                                        PARSED_LITERAL!(
                                                            c,
                                                            parsed_pattern,
                                                            okquantifier
                                                        );
                                                        clstate = CL_CONTINUE; /* goto CLASS_CONTINUE */
                                                        continue 'clsm;
                                                    }

                                                    continue 'class_loop; /* We have already loaded c with the next character */
                                                }
                                                /* Check for the end of the class. */
                                                else if c == CHAR_RIGHT_SQUARE_BRACKET
                                                    || (c == CHAR_RIGHT_PARENTHESIS
                                                        && class_mode_state
                                                            == CLASS_MODE_PERL_EXT)
                                                {
                                                    /* In Perl extended mode, the ']' can
                                                    only be used to match the opening '[',
                                                    and ')' must match an opening
                                                    parenthesis. */
                                                    if class_mode_state
                                                        == CLASS_MODE_PERL_EXT
                                                    {
                                                        if c == CHAR_RIGHT_SQUARE_BRACKET
                                                            && class_depth_m1 != 0
                                                        {
                                                            errorcode = ERR14;
                                                            ptr = ptr.wrapping_sub(1); /* Correct the offset */
                                                            break 'failed; /* goto FAILED */
                                                        }
                                                        if c == CHAR_RIGHT_PARENTHESIS
                                                            && class_depth_m1 < 1
                                                        {
                                                            errorcode = ERR22;
                                                            break 'failed; /* goto FAILED */
                                                        }
                                                    }

                                                    /* Check no trailing operator. */
                                                    if class_op_state == CLASS_OP_OPERATOR {
                                                        errorcode = ERR110;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    /* Check no empty expression for Perl
                                                    extended expressions. */
                                                    if class_mode_state
                                                        == CLASS_MODE_PERL_EXT
                                                        && class_op_state == CLASS_OP_EMPTY
                                                    {
                                                        errorcode = ERR114;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    /* -] at the end of a class is a
                                                    literal '-' */
                                                    if class_range_state == RANGE_STARTED {
                                                        *parsed_pattern.offset(-1) =
                                                            CHAR_MINUS;
                                                    }

                                                    PUTPP!(parsed_pattern, META_CLASS_END);

                                                    class_depth_m1 -= 1;
                                                    if class_depth_m1 < 0 {
                                                        /* Check for and consume ')' after
                                                        '(?[...]'. */
                                                        if class_mode_state
                                                            == CLASS_MODE_PERL_EXT
                                                        {
                                                            if ptr >= ptrend
                                                                || *ptr as u32
                                                                    != CHAR_RIGHT_PARENTHESIS
                                                            {
                                                                errorcode = ERR115;
                                                                break 'failed; /* goto FAILED */
                                                            }

                                                            ptr = ptr.add(1);
                                                        }

                                                        break 'class_loop;
                                                    }

                                                    class_range_state = RANGE_NO; /* for processing the containing class */
                                                    class_op_state = CLASS_OP_OPERAND;
                                                    if class_mode_state
                                                        == CLASS_MODE_PERL_EXT_LEAF
                                                    {
                                                        class_mode_state =
                                                            CLASS_MODE_PERL_EXT;
                                                    }
                                                    /* The extended class flag has already
                                                    been set for the parent class. */
                                                    class_start = core::ptr::null_mut();
                                                }
                                                /* Handle a Perl set binary operator */
                                                else if class_mode_state
                                                    == CLASS_MODE_PERL_EXT
                                                    && (c == CHAR_PLUS
                                                        || c == CHAR_VERTICAL_LINE
                                                        || c == CHAR_MINUS
                                                        || c == CHAR_AMPERSAND
                                                        || c == CHAR_CIRCUMFLEX_ACCENT)
                                                {
                                                    /* Check that there was a preceding
                                                    operand. */
                                                    if class_op_state != CLASS_OP_OPERAND {
                                                        errorcode = ERR109;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    if !class_start.is_null() {
                                                        /* Represents that the class is an
                                                        extended class. */
                                                        *class_start |= CLASS_IS_ECLASS;
                                                        class_start = core::ptr::null_mut();
                                                    }

                                                    PUTPP!(
                                                        parsed_pattern,
                                                        if c == CHAR_PLUS {
                                                            META_ECLASS_OR
                                                        } else if c == CHAR_VERTICAL_LINE {
                                                            META_ECLASS_OR
                                                        } else if c == CHAR_MINUS {
                                                            META_ECLASS_SUB
                                                        } else if c == CHAR_AMPERSAND {
                                                            META_ECLASS_AND
                                                        } else {
                                                            META_ECLASS_XOR
                                                        }
                                                    );
                                                    class_range_state = RANGE_NO;
                                                    class_op_state = CLASS_OP_OPERATOR;
                                                }
                                                /* Handle a Perl set unary operator */
                                                else if class_mode_state
                                                    == CLASS_MODE_PERL_EXT
                                                    && c == CHAR_EXCLAMATION_MARK
                                                {
                                                    /* Check that the "!" has not got a
                                                    preceding operand. */
                                                    if class_op_state == CLASS_OP_OPERAND {
                                                        errorcode = ERR113;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    if !class_start.is_null() {
                                                        /* Represents that the class is an
                                                        extended class. */
                                                        *class_start |= CLASS_IS_ECLASS;
                                                        class_start = core::ptr::null_mut();
                                                    }

                                                    PUTPP!(parsed_pattern, META_ECLASS_NOT);
                                                    class_range_state = RANGE_NO;
                                                    class_op_state = CLASS_OP_OPERATOR;
                                                }
                                                /* Handle a UTS#18 set operator */
                                                else if class_mode_state
                                                    == CLASS_MODE_ALT_EXT
                                                    && (c == CHAR_VERTICAL_LINE
                                                        || c == CHAR_MINUS
                                                        || c == CHAR_AMPERSAND
                                                        || c == CHAR_TILDE)
                                                    && ptr < ptrend
                                                    && *ptr as u32 == c
                                                {
                                                    ptr = ptr.add(1);

                                                    /* Check there isn't a
                                                    triple-repetition. */
                                                    if ptr < ptrend && *ptr as u32 == c {
                                                        while ptr < ptrend
                                                            && *ptr as u32 == c
                                                        {
                                                            ptr = ptr.add(1); /* Improve error offset. */
                                                        }
                                                        errorcode = ERR108;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    /* Check for a preceding operand. */
                                                    if class_op_state != CLASS_OP_OPERAND {
                                                        errorcode = ERR109;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    /* Check for mixed precedence. Forbid
                                                    [A--B&&C]. */
                                                    if *(*cb).class_op_used.as_mut_ptr()
                                                        .add(class_depth_m1 as usize)
                                                        != 0
                                                        && *(*cb).class_op_used.as_mut_ptr()
                                                            .add(class_depth_m1 as usize)
                                                            != c as u8
                                                    {
                                                        errorcode = ERR111;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    if !class_start.is_null() {
                                                        /* Represents that the class is an
                                                        extended class. */
                                                        *class_start |= CLASS_IS_ECLASS;
                                                        class_start = core::ptr::null_mut();
                                                    }

                                                    /* Dangling '-' before an operator is a
                                                    literal */
                                                    if class_range_state == RANGE_STARTED {
                                                        *parsed_pattern.offset(-1) =
                                                            CHAR_MINUS;
                                                    }

                                                    PUTPP!(
                                                        parsed_pattern,
                                                        if c == CHAR_VERTICAL_LINE {
                                                            META_ECLASS_OR
                                                        } else if c == CHAR_MINUS {
                                                            META_ECLASS_SUB
                                                        } else if c == CHAR_AMPERSAND {
                                                            META_ECLASS_AND
                                                        } else {
                                                            META_ECLASS_XOR
                                                        }
                                                    );
                                                    class_range_state = RANGE_NO;
                                                    class_op_state = CLASS_OP_OPERATOR;
                                                    *(*cb).class_op_used.as_mut_ptr()
                                                        .add(class_depth_m1 as usize) =
                                                        c as u8;
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
                                                            break 'failed; /* goto FAILED */
                                                        }
                                                        ptr = tempptr;
                                                        if ptr >= ptrend {
                                                            c = CHAR_BACKSLASH;
                                                        } else {
                                                            /* Get character value,
                                                            increment pointer */
                                                            GETCHARINCTEST!(c, ptr, utf);
                                                        }
                                                        escape = 0; /* Treat as literal character */
                                                    }

                                                    match escape as u32 {
                                                        0 => {
                                                            /* Escaped character code point
                                                            is in c */
                                                            char_is_literal = FALSE;
                                                            clstate = CL_LITERAL; /* goto CLASS_LITERAL */
                                                            continue 'clsm;
                                                        }

                                                        ESC_b => {
                                                            c = CHAR_BS; /* \b is backspace in a class */
                                                            char_is_literal = FALSE;
                                                            clstate = CL_LITERAL; /* goto CLASS_LITERAL */
                                                            continue 'clsm;
                                                        }

                                                        ESC_k => {
                                                            c = CHAR_k; /* \k is not special in a class, just like \g */
                                                            char_is_literal = FALSE;
                                                            clstate = CL_LITERAL; /* goto CLASS_LITERAL */
                                                            continue 'clsm;
                                                        }

                                                        ESC_Q => {
                                                            inescq = TRUE; /* Enter literal mode */
                                                            clstate = CL_CONTINUE; /* goto CLASS_CONTINUE */
                                                            continue 'clsm;
                                                        }

                                                        ESC_E => {
                                                            /* Ignore orphan \E */
                                                            clstate = CL_CONTINUE; /* goto CLASS_CONTINUE */
                                                            continue 'clsm;
                                                        }

                                                        ESC_B | ESC_R | ESC_X => {
                                                            /* Always an error in a class */
                                                            errorcode = ERR7;
                                                            break 'failed; /* goto FAILED */
                                                        }

                                                        ESC_N => {
                                                            /* Not permitted by Perl either */
                                                            errorcode = ERR71;
                                                            break 'failed; /* goto FAILED */
                                                        }

                                                        ESC_H | ESC_h | ESC_V | ESC_v => {
                                                            PUTPP!(
                                                                parsed_pattern,
                                                                META_ESCAPE + escape as u32
                                                            );
                                                        }

                                                        /* These escapes may be converted to
                                                        Unicode property tests when
                                                        PCRE2_UCP is set. */

                                                        ESC_d | ESC_D | ESC_s | ESC_S
                                                        | ESC_w | ESC_W => {
                                                            parsed_pattern = handle_escdsw(
                                                                escape,
                                                                parsed_pattern,
                                                                options,
                                                                xoptions,
                                                            );
                                                        }

                                                        /* Explicit Unicode property
                                                        matching */

                                                        ESC_P | ESC_p => {
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
                                                            ) == 0
                                                            {
                                                                break 'failed; /* goto FAILED */
                                                            }

                                                            /* In caseless matching,
                                                            particular characteristics Lu,
                                                            Ll, and Lt get converted to the
                                                            general characteristic L&. */

                                                            if (options & PCRE2_CASELESS)
                                                                != 0
                                                                && ptype as u32 == PT_PC
                                                                && (pdata as u32 == ucp_Lu
                                                                    || pdata as u32
                                                                        == ucp_Ll
                                                                    || pdata as u32
                                                                        == ucp_Lt)
                                                            {
                                                                ptype = PT_LAMP as u16;
                                                                pdata = 0;
                                                            }

                                                            if negated != 0 {
                                                                escape = if escape as u32
                                                                    == ESC_P
                                                                {
                                                                    ESC_p as i32
                                                                } else {
                                                                    ESC_P as i32
                                                                };
                                                            }
                                                            PUTPP!(
                                                                parsed_pattern,
                                                                META_ESCAPE + escape as u32
                                                            );
                                                            PUTPP!(
                                                                parsed_pattern,
                                                                ((ptype as u32) << 16)
                                                                    | pdata as u32
                                                            );
                                                        } /* End \P and \p */

                                                        /* All others are not allowed in a
                                                        class */

                                                        /* LCOV_EXCL_START */
                                                        /* default falls through */
                                                        /* LCOV_EXCL_STOP */
                                                        _ => {
                                                            /* ESC_A, ESC_Z, ESC_z, ESC_G,
                                                            ESC_K, ESC_C and any other */
                                                            errorcode = ERR7;
                                                            break 'failed; /* goto FAILED */
                                                        }
                                                    }

                                                    /* All the switch-cases above which end
                                                    in "break" describe a set of
                                                    characters. None may start a range. */

                                                    if class_range_state == RANGE_STARTED {
                                                        errorcode = ERR50;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    /* Perl gives a warning unless the
                                                    hyphen following a multi-character
                                                    escape is the last character in the
                                                    class. PCRE throws an error. */

                                                    if class_range_state
                                                        == RANGE_FORBID_STARTED
                                                    {
                                                        ptr = class_range_forbid_ptr;
                                                        errorcode = ERR50;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    /* Disallow implicit union in Perl
                                                    extended classes. */

                                                    if class_op_state == CLASS_OP_OPERAND
                                                        && class_mode_state
                                                            == CLASS_MODE_PERL_EXT
                                                    {
                                                        errorcode = ERR113;
                                                        break 'failed; /* goto FAILED */
                                                    }

                                                    class_range_state = RANGE_FORBID_NO;
                                                    class_op_state = CLASS_OP_OPERAND;
                                                }
                                                /* Forbid unescaped literals, and the
                                                special meaning of '-', inside a Perl
                                                extended class. */
                                                else if class_mode_state
                                                    == CLASS_MODE_PERL_EXT
                                                {
                                                    errorcode = ERR116;
                                                    break 'failed; /* goto FAILED */
                                                }
                                                /* Handle potential start of range */
                                                else if c == CHAR_MINUS
                                                    && class_range_state >= RANGE_OK_ESCAPED
                                                {
                                                    PUTPP!(
                                                        parsed_pattern,
                                                        if class_range_state
                                                            == RANGE_OK_LITERAL
                                                        {
                                                            META_RANGE_LITERAL
                                                        } else {
                                                            META_RANGE_ESCAPED
                                                        }
                                                    );
                                                    class_range_state = RANGE_STARTED;
                                                }
                                                /* Handle forbidden start of range */
                                                else if c == CHAR_MINUS
                                                    && class_range_state == RANGE_FORBID_NO
                                                {
                                                    PUTPP!(parsed_pattern, CHAR_MINUS);
                                                    class_range_state =
                                                        RANGE_FORBID_STARTED;
                                                    class_range_forbid_ptr = ptr;
                                                }
                                                /* Handle a literal character */
                                                else {
                                                    clstate = CL_LITERAL;
                                                    continue 'clsm;
                                                }

                                                /* Proceed to next thing in the class. */
                                                clstate = CL_CONTINUE;
                                                continue 'clsm;
                                            }

                                            /* CLASS_LITERAL: */
                                            CL_LITERAL => {
                                                /* Disallow implicit union in Perl extended
                                                classes. */

                                                if class_op_state == CLASS_OP_OPERAND
                                                    && class_mode_state
                                                        == CLASS_MODE_PERL_EXT
                                                {
                                                    errorcode = ERR113;
                                                    break 'failed; /* goto FAILED */
                                                }

                                                if class_range_state == RANGE_STARTED {
                                                    if c == *parsed_pattern.offset(-2) {
                                                        /* Optimize one-char range */
                                                        parsed_pattern =
                                                            parsed_pattern.offset(-1);
                                                    } else if *parsed_pattern.offset(-2) > c
                                                    {
                                                        /* Check range is in order */
                                                        errorcode = ERR8;
                                                        break 'failed; /* goto FAILED */
                                                    } else {
                                                        if char_is_literal == 0
                                                            && *parsed_pattern.offset(-1)
                                                                == META_RANGE_LITERAL
                                                        {
                                                            *parsed_pattern.offset(-1) =
                                                                META_RANGE_ESCAPED;
                                                        }
                                                        PARSED_LITERAL!(
                                                            c,
                                                            parsed_pattern,
                                                            okquantifier
                                                        );
                                                    }
                                                    class_range_state = RANGE_NO;
                                                    class_op_state = CLASS_OP_OPERAND;
                                                } else if class_range_state
                                                    == RANGE_FORBID_STARTED
                                                {
                                                    ptr = class_range_forbid_ptr;
                                                    errorcode = ERR50;
                                                    break 'failed; /* goto FAILED */
                                                } else {
                                                    /* Potential start of range */
                                                    class_range_state = if char_is_literal
                                                        != 0
                                                    {
                                                        RANGE_OK_LITERAL
                                                    } else {
                                                        RANGE_OK_ESCAPED
                                                    };
                                                    class_op_state = CLASS_OP_OPERAND;
                                                    PARSED_LITERAL!(
                                                        c,
                                                        parsed_pattern,
                                                        okquantifier
                                                    );
                                                }

                                                clstate = CL_CONTINUE;
                                                continue 'clsm;
                                            }

                                            /* CLASS_CONTINUE: */
                                            CL_CONTINUE => {
                                                if ptr >= ptrend {
                                                    if class_mode_state
                                                        == CLASS_MODE_PERL_EXT
                                                        && class_depth_m1 > 0
                                                    {
                                                        errorcode = ERR14; /* Missing terminating ')' */
                                                    }
                                                    if class_mode_state
                                                        == CLASS_MODE_ALT_EXT
                                                        && class_depth_m1 == 0
                                                        && class_maxdepth_m1 == 1
                                                    {
                                                        errorcode = ERR112; /* Missing terminating ']', but we saw '[ [ ]...' */
                                                    } else {
                                                        errorcode = ERR6; /* Missing terminating ']' */
                                                    }
                                                    break 'failed; /* goto FAILED */
                                                }
                                                GETCHARINCTEST!(c, ptr, utf);
                                                break 'clsm;
                                            }

                                            _ => {
                                                break 'clsm;
                                            }
                                        }
                                    }
                                } /* End of class-processing loop */

                                break 'isw; /* End of character class */
                            }

                            /* RECURSION_BYNUMBER: */
                            S_RECURSION_BYNUMBER => {
                                if read_number(
                                    &mut ptr,
                                    ptrend,
                                    if IS_DIGIT!(*ptr) {
                                        -1
                                    } else {
                                        (*cb).bracount as i32
                                    }, /* + and - are relative */
                                    MAX_GROUP_NUMBER,
                                    ERR61 as u32,
                                    &mut i,
                                    &mut errorcode,
                                ) == 0
                                {
                                    break 'failed; /* goto FAILED */
                                }
                                /* NB (?0) is permitted, represented by i=0 */
                                terminator = CHAR_NUL;

                                /* Falls into SET_RECURSION */
                                istate = S_SET_RECURSION;
                                continue 'isw;
                            }

                            /* SET_RECURSION: */
                            S_SET_RECURSION => {
                                PUTPP!(parsed_pattern, META_RECURSE | i as u32);
                                offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                                /* End of recursive call by number handling */
                                istate = S_READ_RECURSION_ARGUMENTS; /* goto READ_RECURSION_ARGUMENTS */
                                continue 'isw;
                            }

                            /* RECURSE_BY_NAME: */
                            S_RECURSE_BY_NAME => {
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
                                    break 'failed; /* goto FAILED */
                                }
                                PUTPP!(parsed_pattern, META_RECURSE_BYNAME);
                                PUTPP!(parsed_pattern, namelen);
                                terminator = CHAR_NUL;

                                /* Falls into READ_RECURSION_ARGUMENTS */
                                istate = S_READ_RECURSION_ARGUMENTS;
                                continue 'isw;
                            }

                            /* READ_RECURSION_ARGUMENTS: */
                            S_READ_RECURSION_ARGUMENTS => {
                                PUTOFFSET!(offset, parsed_pattern);
                                okquantifier = TRUE;

                                /* Arguments are not supported for \g construct. */
                                if terminator != CHAR_NUL {
                                    break 'isw;
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
                                    if parsed_pattern.is_null() {
                                        break 'failed; /* goto FAILED */
                                    }
                                }

                                if ptr >= ptrend || *ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                                    break 'unclosed_parenthesis; /* goto UNCLOSED_PARENTHESIS */
                                }

                                ptr = ptr.add(1);
                                break 'isw;
                            }

                            /* ATOMIC_GROUP: Come from (*atomic: */
                            S_ATOMIC_GROUP => {
                                PUTPP!(parsed_pattern, META_ATOMIC);
                                nest_depth += 1;
                                ptr = ptr.add(1);
                                break 'isw;
                            }

                            /* POSITIVE_LOOK_AHEAD: Come from (*pla: */
                            S_POSITIVE_LOOK_AHEAD => {
                                PUTPP!(parsed_pattern, META_LOOKAHEAD);
                                ptr = ptr.add(1);
                                istate = S_POST_ASSERTION; /* goto POST_ASSERTION */
                                continue 'isw;
                            }

                            /* POSITIVE_NONATOMIC_LOOK_AHEAD: Come from (*napla: */
                            S_POSITIVE_NONATOMIC_LOOK_AHEAD => {
                                PUTPP!(parsed_pattern, META_LOOKAHEAD_NA);
                                ptr = ptr.add(1);
                                istate = S_POST_ASSERTION; /* goto POST_ASSERTION */
                                continue 'isw;
                            }

                            /* NEGATIVE_LOOK_AHEAD: Come from (*nla: */
                            S_NEGATIVE_LOOK_AHEAD => {
                                PUTPP!(parsed_pattern, META_LOOKAHEADNOT);
                                ptr = ptr.add(1);
                                istate = S_POST_ASSERTION; /* goto POST_ASSERTION */
                                continue 'isw;
                            }

                            /* POST_LOOKBEHIND: Come from (*plb: (*naplb: and (*nlb: */
                            S_POST_LOOKBEHIND => {
                                *has_lookbehind = TRUE;
                                offset =
                                    (ptr.offset_from((*cb).start_pattern) - 2) as PCRE2_SIZE;
                                PUTOFFSET!(offset, parsed_pattern);
                                ptr = ptr.add(2);
                                /* Fall through to POST_ASSERTION */
                                istate = S_POST_ASSERTION;
                                continue 'isw;
                            }

                            /* POST_ASSERTION: */
                            S_POST_ASSERTION => {
                                nest_depth += 1;
                                if prev_expect_cond_assert > 0 {
                                    if top_nest.is_null() {
                                        top_nest = (*cb).start_workspace as *mut nest_save;
                                    } else {
                                        top_nest = top_nest.add(1);
                                        if top_nest >= end_nests {
                                            errorcode = ERR84;
                                            break 'failed; /* goto FAILED */
                                        }
                                    }
                                    (*top_nest).nest_depth = nest_depth;
                                    (*top_nest).flags = NSF_CONDASSERT;
                                    (*top_nest).options = options & PARSE_TRACKED_OPTIONS;
                                    (*top_nest).xoptions =
                                        xoptions & PARSE_TRACKED_EXTRA_OPTIONS;
                                }
                                break 'isw;
                            }

                            /* DEFINE_NAME: */
                            S_DEFINE_NAME => {
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
                                    break 'failed; /* goto FAILED */
                                }

                                /* We have a name for this capturing group. It is also
                                assigned a number, which is its primary means of
                                identification. */

                                if (*cb).bracount >= MAX_GROUP_NUMBER {
                                    errorcode = ERR97;
                                    break 'failed; /* goto FAILED */
                                }
                                (*cb).bracount += 1;
                                PUTPP!(parsed_pattern, META_CAPTURE | (*cb).bracount);
                                nest_depth += 1;

                                /* Check not too many names */

                                if (*cb).names_found as u32 >= MAX_NAME_COUNT {
                                    errorcode = ERR49;
                                    break 'failed; /* goto FAILED */
                                }

                                /* Adjust the entry size to accommodate the longest name
                                found. */

                                if namelen + IMM2_SIZE as u32 + 1
                                    > (*cb).name_entry_size as u32
                                {
                                    (*cb).name_entry_size =
                                        (namelen + IMM2_SIZE as u32 + 1) as u16;
                                }

                                /* Scan the list to check for duplicates. */

                                is_dupname = FALSE;
                                hash = _pcre2_compile_get_hash_from_name8(name, namelen);
                                ng = (*cb).named_groups;
                                i = 0;
                                'nameloop: while i < (*cb).names_found as i32 {
                                    if namelen == (*ng).length as u32
                                        && hash == ((*ng).hash_dup & NAMED_GROUP_HASH_MASK)
                                        && _pcre2_strncmp_8(
                                            name,
                                            (*ng).name,
                                            namelen as PCRE2_SIZE,
                                        ) == 0
                                    {
                                        /* When a bracket is referenced by the same name
                                        multiple times, is not considered as a duplicate
                                        and ignored. */
                                        if (*ng).number == (*cb).bracount {
                                            break 'nameloop;
                                        }
                                        if (options & PCRE2_DUPNAMES) == 0 {
                                            errorcode = ERR43;
                                            break 'failed; /* goto FAILED */
                                        }

                                        (*ng).hash_dup |= NAMED_GROUP_IS_DUPNAME;
                                        is_dupname = TRUE; /* Mark as a duplicate */
                                        (*cb).dupnames = TRUE; /* Duplicate names exist */

                                        /* The entry represents a duplicate. */
                                        name = (*ng).name;
                                        namelen = 0;

                                        /* Even duplicated names may refer to the same
                                        capture index. These references are also ignored. */
                                        while i < (*cb).names_found as i32 {
                                            if (*ng).name == name
                                                && (*ng).number == (*cb).bracount
                                            {
                                                break;
                                            }
                                            i += 1;
                                            ng = ng.add(1);
                                        }
                                        break 'nameloop;
                                    } else if (*ng).number == (*cb).bracount {
                                        errorcode = ERR65;
                                        break 'failed; /* goto FAILED */
                                    }
                                    i += 1;
                                    ng = ng.add(1);
                                }

                                /* Ignore duplicate with same number. */
                                if i < (*cb).names_found as i32 {
                                    break 'isw;
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
                                    if newspace.is_null() {
                                        errorcode = ERR21;
                                        break 'failed; /* goto FAILED */
                                    }

                                    core::ptr::copy_nonoverlapping(
                                        (*cb).named_groups as *const u8,
                                        newspace as *mut u8,
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

                                (*(*cb).named_groups.add((*cb).names_found as usize)).name =
                                    name;
                                (*(*cb).named_groups.add((*cb).names_found as usize)).length =
                                    namelen as u16;
                                (*(*cb).named_groups.add((*cb).names_found as usize)).number =
                                    (*cb).bracount;
                                (*(*cb).named_groups.add((*cb).names_found as usize))
                                    .hash_dup = hash;
                                (*cb).names_found += 1;
                                break 'isw;
                            }

                            _ => {
                                break 'isw;
                            }
                        }
                    }
                } /* End of main character scan loop */

                /* End of pattern reached. Check for missing ) at the end of a verb
                name. */

                if inverbname != 0 && ptr >= ptrend {
                    errorcode = ERR60;
                    break 'failed; /* goto FAILED */
                }

                /* Falls through to PARSED_END */
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

            /* Insert trailing items for word and line matching (features provided
            for the benefit of pcre2grep). */

            if (xoptions & PCRE2_EXTRA_MATCH_LINE) != 0 {
                PUTPP!(parsed_pattern, META_KET);
                PUTPP!(parsed_pattern, META_DOLLAR);
            } else if (xoptions & PCRE2_EXTRA_MATCH_WORD) != 0 {
                PUTPP!(parsed_pattern, META_KET);
                PUTPP!(parsed_pattern, META_ESCAPE + ESC_b);
            }

            /* Terminate the parsed pattern, then return success if all groups are
            closed. Otherwise we have unclosed parentheses. */

            /* LCOV_EXCL_START */
            if parsed_pattern >= parsed_pattern_end {
                errorcode = ERR63; /* Internal error (parsed pattern overflow) */
                break 'failed; /* goto FAILED */
            }
            /* LCOV_EXCL_STOP */

            *parsed_pattern = META_END;
            if nest_depth == 0 {
                return 0;
            }

            /* Falls through to UNCLOSED_PARENTHESIS */
        }

        /* UNCLOSED_PARENTHESIS: */
        errorcode = ERR14;

        /* Falls through to FAILED */
    }

    /* Come here for all failures. */

    /* FAILED_BACK / FAILED_FORWARD adjust ptr before joining FAILED. */
    if fail_mode == 1 {
        /* FAILED_BACK: some errors need to indicate the previous character. */
        ptr = ptr.wrapping_sub(1);
        if utf != 0 {
            BACKCHAR!(ptr);
        }
    } else if fail_mode == 2 {
        /* FAILED_FORWARD: some errors need to indicate the next character. */
        ptr = ptr.add(1);
        if utf != 0 {
            FORWARDCHARTEST!(ptr, ptrend);
        }
    }

    /* FAILED: */
    (*cb).erroroffset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
    errorcode
}
