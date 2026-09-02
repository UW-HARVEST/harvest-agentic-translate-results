//! Translation of PART 2 of `pcre2_compile.c`: the single large `static`
//! function `parse_regex()` (C source lines 3112..5966).
//!
//! `parse_regex()` performs the first pass of PCRE2 compilation: it scans the
//! input pattern and converts it into the intermediate "parsed pattern" vector
//! (`cb->parsed_pattern`), a sequence of 32-bit meta codes with associated
//! data. It handles literals, escapes, quantifiers, character classes
//! (including UTS#18 and Perl-extended classes), groups, assertions, verbs,
//! conditionals, callouts, options settings, recursion/subroutine calls, and
//! named groups.
//!
//! This is the 8-bit build (`PCRE2_CODE_UNIT_WIDTH == 8`) with
//! `SUPPORT_UNICODE` enabled, `SUPPORT_JIT` off, `PCRE2_DEBUG` off, and no
//! EBCDIC. Consequently all `#ifdef PCRE2_DEBUG` blocks are omitted, and the
//! `PARSED_LITERAL` macro is the non-32-bit form (`*p++ = c; okquantifier =
//! TRUE;`).
//!
//! The many C `goto` labels are modelled with Rust control flow: error labels
//! (`FAILED`, `FAILED_BACK`, `FAILED_FORWARD`, `UNCLOSED_PARENTHESIS`) are
//! handled by local macros that set `cb->erroroffset` and return; the
//! cross-`case` forward jumps in the main switch are handled by a dispatch
//! `loop` driven by a `Jump` enum.

use core::ffi::{c_char, c_int};
use core::ptr;

use crate::compile_cgroup::_pcre2_compile_get_hash_from_name8;
use crate::compile_h::*;
use crate::compile_local::*;
use crate::compile_tables::*;
use crate::consts::*;
use crate::internal::*;
// Disambiguate BOOL/TRUE/FALSE (defined in both consts and internal globs).
// The internal.rs versions are the `c_int`-typed ones we want.
use crate::internal::{BOOL, FALSE, TRUE};

use crate::compile_parse_util::{
    check_posix_name, check_posix_syntax, get_ucp, handle_escdsw, manage_callouts,
    parse_capture_list, read_name, read_number, read_repeat_counts, _pcre2_check_escape_8,
};

// ---------------------------------------------------------------------------
// ASCII CHAR_* constants used below (this build is not EBCDIC).
// ---------------------------------------------------------------------------
const CHAR_NUL: u32 = 0x00;
const CHAR_BS: u32 = 0x08;
const CHAR_HT: u32 = 0x09;
const CHAR_LF: u32 = 0x0a;
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
const CHAR_9: u32 = 0x39;
const CHAR_COLON: u32 = 0x3a;
const CHAR_LESS_THAN_SIGN: u32 = 0x3c;
const CHAR_EQUALS_SIGN: u32 = 0x3d;
const CHAR_GREATER_THAN_SIGN: u32 = 0x3e;
const CHAR_QUESTION_MARK: u32 = 0x3f;
const CHAR_A: u32 = 0x41;
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
const CHAR_TILDE: u32 = 0x7e;

// STRING_ constants (as byte literals).
const STRING_WEIRD_STARTWORD: &[u8; 6] = b"[:<:]]";
const STRING_WEIRD_ENDWORD: &[u8; 6] = b"[:>:]]";
const STRING_DEFINE: &[u8; 6] = b"DEFINE";
const STRING_VERSION: &[u8; 7] = b"VERSION";
// STR_Q STR_BACKSLASH STR_E == "Q\E"
const STR_Q_BACKSLASH_E: &[u8; 3] = b"Q\\E";

// ---------------------------------------------------------------------------
// Small helpers / macro-equivalents
// ---------------------------------------------------------------------------

/// `IS_DIGIT(c)`.
#[inline(always)]
const fn IS_DIGIT(c: u32) -> bool {
    c >= CHAR_0 && c <= CHAR_9
}

/// `PRIV(strncmp_c8)` — compare a PCRE2 string with an 8-bit byte slice.
#[inline(always)]
unsafe fn strncmp_c8(ptr: PCRE2_SPTR, s: &[u8], len: usize) -> c_int {
    unsafe { crate::string_utils::_pcre2_strncmp_c8_8(ptr, s.as_ptr() as *const c_char, len) }
}

/// `PRIV(strncmp)` — compare two PCRE2 strings for a given length.
#[inline(always)]
unsafe fn strncmp(a: PCRE2_SPTR, b: PCRE2_SPTR, len: usize) -> c_int {
    unsafe { crate::string_utils::_pcre2_strncmp_8(a, b, len) }
}

/// `PRIV(check_escape)` wrapper.
#[inline(always)]
unsafe fn check_escape(
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
        _pcre2_check_escape_8(
            ptrptr,
            ptrend,
            chptr,
            errorcodeptr,
            options,
            xoptions,
            bracount,
            isclass,
            cb,
        )
    }
}

// ---------------------------------------------------------------------------
// Cross-case dispatch labels for the main switch.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Jump {
    /// Run the main `switch (c)`.
    Switch,
    CheckQuantifier,
    AtomicGroup,
    PositiveLookAhead,
    PositiveNonatomicLookAhead,
    NegativeLookAhead,
    SetRecursion,
    RecursionByNumber,
    RecurseByName,
    ReadRecursionArguments,
    DefineName,
    /// The lookbehind-assertion tail beginning at POST_LOOKBEHIND.
    PostLookbehind,
    /// The assertion tail beginning at POST_ASSERTION.
    PostAssertion,
    /// The character class parser, entered at FROM_PERL_EXTENDED_CLASS.
    FromPerlExtendedClass,
    /// Finished processing this character; go to next iteration of main loop.
    NextChar,
}

// ---------------------------------------------------------------------------
// parse_regex
// ---------------------------------------------------------------------------

/// `parse_regex()` — parse the pattern into `cb->parsed_pattern`, returning 0
/// on success or a non-zero error code (with `cb->erroroffset` set).
///
/// `static` in C; called from `compile.rs`. Hence not `no_mangle` / `extern`.
pub(crate) unsafe fn parse_regex(
    ptr: PCRE2_SPTR,
    options: u32,
    xoptions: u32,
    has_lookbehind: *mut BOOL,
    cb: *mut compile_block,
) -> c_int {
    unsafe {
        let mut ptr = ptr;
        let mut options = options;
        let mut xoptions = xoptions;

        let mut c: u32;
        let mut delimiter: u32;
        let mut namelen: u32 = 0;
        let mut class_range_state: u32 = RANGE_NO;
        let mut class_op_state: u32 = CLASS_OP_EMPTY;
        let mut class_mode_state: u32 = CLASS_MODE_NORMAL;
        let mut class_start: *mut u32 = ptr::null_mut();
        let mut verblengthptr: *mut u32 = ptr::null_mut();
        let mut verbstartptr: *mut u32 = ptr::null_mut();
        let mut previous_callout: *mut u32 = ptr::null_mut();
        let mut parsed_pattern: *mut u32 = (*cb).parsed_pattern;
        let parsed_pattern_end: *mut u32 = (*cb).parsed_pattern_end;
        let mut this_parsed_item: *mut u32 = ptr::null_mut();
        let mut prev_parsed_item: *mut u32 = ptr::null_mut();
        let mut meta_quantifier: u32 = 0;
        let mut add_after_mark: u32 = 0;
        let mut nest_depth: u16 = 0;
        let mut class_depth_m1: i16 = -1; // The m1 means minus 1.
        let mut class_maxdepth_m1: i16 = -1;
        let mut hash: u16 = 0;
        let mut after_manual_callout: c_int = 0;
        let mut expect_cond_assert: c_int = 0;
        let mut errorcode: c_int = 0;
        let mut escape: c_int;
        let mut i: c_int = 0;
        let mut inescq: BOOL = FALSE;
        let mut inverbname: BOOL = FALSE;
        let utf: BOOL = ((options & PCRE2_UTF as u32) != 0) as BOOL;
        let auto_callout: BOOL = ((options & PCRE2_AUTO_CALLOUT as u32) != 0) as BOOL;
        let mut is_dupname: BOOL = FALSE;
        let mut negate_class: BOOL = FALSE;
        let mut okquantifier: BOOL = FALSE;
        let mut thisptr: PCRE2_SPTR;
        let mut name: PCRE2_SPTR = ptr::null();
        let ptrend: PCRE2_SPTR = (*cb).end_pattern;
        let mut verbnamestart: PCRE2_SPTR = ptr::null();
        let mut class_range_forbid_ptr: PCRE2_SPTR = ptr::null();
        let mut ng: *mut named_group = core::ptr::null_mut();
        let mut top_nest: *mut nest_save = ptr::null_mut();
        let end_nests: *mut nest_save;

        // A plain Rust bool for the char macros.
        let utf_b: bool = utf != 0;

        // Insert leading items for word and line matching (features provided
        // for the benefit of pcre2grep).

        if (xoptions & PCRE2_EXTRA_MATCH_LINE as u32) != 0 {
            *parsed_pattern = META_CIRCUMFLEX as u32;
            parsed_pattern = parsed_pattern.add(1);
            *parsed_pattern = META_NOCAPTURE as u32;
            parsed_pattern = parsed_pattern.add(1);
        } else if (xoptions & PCRE2_EXTRA_MATCH_WORD as u32) != 0 {
            *parsed_pattern = (META_ESCAPE as u32) + ESC_b;
            parsed_pattern = parsed_pattern.add(1);
            *parsed_pattern = META_NOCAPTURE as u32;
            parsed_pattern = parsed_pattern.add(1);
        }

        // If the pattern is actually a literal string, process it separately to
        // avoid cluttering up the main loop.

        if (options & PCRE2_LITERAL as u32) != 0 {
            while ptr < ptrend {
                // LCOV_EXCL_START
                if parsed_pattern >= parsed_pattern_end {
                    errorcode = ERR63; // Internal error (parsed pattern overflow)
                    (*cb).erroroffset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                    return errorcode;
                }
                // LCOV_EXCL_STOP

                thisptr = ptr;
                c = GETCHARINCTEST(&mut ptr, utf_b);
                if auto_callout != 0 {
                    parsed_pattern = manage_callouts(
                        thisptr,
                        &mut previous_callout,
                        auto_callout,
                        parsed_pattern,
                        cb,
                    );
                }
                // PARSED_LITERAL(c, parsed_pattern)
                *parsed_pattern = c;
                parsed_pattern = parsed_pattern.add(1);
                okquantifier = TRUE;
            }
            // goto PARSED_END
            return parse_regex_end(
                &mut ptr,
                ptrend,
                &mut parsed_pattern,
                parsed_pattern_end,
                &mut previous_callout,
                auto_callout,
                xoptions,
                inverbname,
                nest_depth,
                utf_b,
                &mut errorcode,
                cb,
            );
        }

        // Process a real regex which may contain meta-characters.

        top_nest = ptr::null_mut();
        end_nests = ((*cb).start_workspace as *mut PCRE2_UCHAR)
            .add((*cb).workspace_size as usize) as *mut nest_save;

        // Round down end_nests so as to avoid creating a nest_save that spans
        // the end of the workspace.
        let round_down = (((*cb).workspace_size as usize
            * core::mem::size_of::<PCRE2_UCHAR>())
            % core::mem::size_of::<nest_save>()) as isize;
        let end_nests = ((end_nests as *mut u8).offset(-round_down)) as *mut nest_save;

        // PCRE2_EXTENDED_MORE implies PCRE2_EXTENDED
        if (options & PCRE2_EXTENDED_MORE as u32) != 0 {
            options |= PCRE2_EXTENDED as u32;
        }

        // ------------------------------------------------------------------
        // Now scan the pattern
        // ------------------------------------------------------------------

        'main: while ptr < ptrend {
            let prev_expect_cond_assert: c_int;
            let mut min_repeat: u32 = 0;
            let mut max_repeat: u32 = 0;
            let mut set: u32;
            let mut unset: u32;
            let mut xset: u32;
            let mut xunset: u32;
            let mut terminator: u32 = 0;
            let prev_meta_quantifier: u32;
            let prev_okquantifier: BOOL;
            let mut tempptr: PCRE2_SPTR;
            let mut offset: PCRE2_SIZE = 0;

            // Macros for the error labels. They reference `ptr`, `errorcode`,
            // `cb`, `utf_b`, `ptrend`.
            macro_rules! failed {
                () => {{
                    (*cb).erroroffset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                    return errorcode;
                }};
            }
            macro_rules! failed_back {
                () => {{
                    ptr = ptr.sub(1);
                    if utf_b {
                        BACKCHAR(&mut ptr);
                    }
                    failed!();
                }};
            }
            #[allow(unused_macros)]
            macro_rules! failed_forward {
                () => {{
                    ptr = ptr.add(1);
                    if utf_b {
                        FORWARDCHARTEST(&mut ptr, ptrend);
                    }
                    failed!();
                }};
            }
            macro_rules! unclosed_parenthesis {
                () => {{
                    errorcode = ERR14;
                    failed!();
                }};
            }

            if nest_depth as u32 > (*(*cb).cx).parens_nest_limit {
                errorcode = ERR19;
                failed!(); // Parentheses too deeply nested
            }

            // LCOV_EXCL_START
            if parsed_pattern >= parsed_pattern_end {
                errorcode = ERR63; // Internal error (parsed pattern overflow)
                failed!();
            }
            // LCOV_EXCL_STOP

            // If the last time round this loop something was added,
            // parsed_pattern will no longer be equal to this_parsed_item.
            if this_parsed_item != parsed_pattern {
                prev_parsed_item = this_parsed_item;
                this_parsed_item = parsed_pattern;
            }

            // Get next input character, save its position for callout handling.
            thisptr = ptr;
            c = GETCHARINCTEST(&mut ptr, utf_b);

            // Copy quoted literals until \E, allowing for the possibility of
            // automatic callouts, except when processing a (*VERB) "name".
            if inescq != 0 {
                if c == CHAR_BACKSLASH && ptr < ptrend && *ptr as u32 == CHAR_E {
                    inescq = FALSE;
                    ptr = ptr.add(1); // Skip E
                } else {
                    if inverbname != 0 {
                        // Don't use PARSED_LITERAL() because it sets okquantifier.
                        *parsed_pattern = c;
                        parsed_pattern = parsed_pattern.add(1);
                    } else {
                        let tmp = after_manual_callout;
                        after_manual_callout -= 1;
                        if tmp <= 0 {
                            parsed_pattern = manage_callouts(
                                thisptr,
                                &mut previous_callout,
                                auto_callout,
                                parsed_pattern,
                                cb,
                            );
                        }
                        // PARSED_LITERAL(c, parsed_pattern)
                        *parsed_pattern = c;
                        parsed_pattern = parsed_pattern.add(1);
                        okquantifier = TRUE;
                    }
                    meta_quantifier = 0;
                }
                continue 'main; // Next character
            }

            // If we are processing the "name" part of a (*VERB:NAME) item, all
            // characters up to the closing parenthesis are literals except when
            // PCRE2_ALT_VERBNAMES is set.
            if inverbname != 0
                && (
                    // EITHER: not both options set
                    ((options & (PCRE2_EXTENDED as u32 | PCRE2_ALT_VERBNAMES as u32))
                        != (PCRE2_EXTENDED as u32 | PCRE2_ALT_VERBNAMES as u32))
                    // OR: character > 255 AND not Unicode Pattern White Space
                    || (c > 255 && (c | 1) != 0x200f && (c | 1) != 0x2029)
                    // OR: not a # comment or isspace() white space
                    || (c < 256
                        && c != CHAR_NUMBER_SIGN
                        && (*(*cb).ctypes.add(c as usize) as u32 & ctype_space as u32) == 0
                        && c != CHAR_NEL)
                )
            {
                let verbnamelength: PCRE2_SIZE;

                match c {
                    CHAR_RIGHT_PARENTHESIS => {
                        inverbname = FALSE;
                        // This is the length in characters
                        verbnamelength =
                            parsed_pattern.offset_from(verblengthptr) as PCRE2_SIZE - 1;
                        // But the limit on the length is in code units
                        if ptr.offset_from(verbnamestart) - 1 > MAX_MARK_U as isize {
                            ptr = ptr.sub(1);
                            errorcode = ERR76;
                            failed!();
                        }
                        *verblengthptr = verbnamelength as u32;

                        // If this name was on a verb such as (*ACCEPT) which
                        // does not continue, a (*MARK) was generated for the
                        // name. Add the original verb as the next item.
                        if add_after_mark != 0 {
                            *parsed_pattern = add_after_mark;
                            parsed_pattern = parsed_pattern.add(1);
                            add_after_mark = 0;
                        }
                    }

                    CHAR_BACKSLASH => {
                        if (options & PCRE2_ALT_VERBNAMES as u32) != 0 {
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
                                failed!();
                            }
                        } else {
                            escape = 0; // Treat all as literal
                        }

                        match escape {
                            0 => {
                                // Don't use PARSED_LITERAL() (sets okquantifier).
                                *parsed_pattern = c;
                                parsed_pattern = parsed_pattern.add(1);
                            }
                            e if e == ESC_ub as c_int => {
                                *parsed_pattern = CHAR_u;
                                parsed_pattern = parsed_pattern.add(1);
                                // PARSED_LITERAL(CHAR_LEFT_CURLY_BRACKET, ...)
                                *parsed_pattern = CHAR_LEFT_CURLY_BRACKET;
                                parsed_pattern = parsed_pattern.add(1);
                                okquantifier = TRUE;
                            }
                            e if e == ESC_Q as c_int => {
                                inescq = TRUE;
                            }
                            e if e == ESC_E as c_int => { /* Ignore */ }
                            _ => {
                                errorcode = ERR40; // Invalid in verb name
                                failed!();
                            }
                        }
                    }

                    _ => {
                        // default: Don't use PARSED_LITERAL() (sets okquantifier).
                        *parsed_pattern = c;
                        parsed_pattern = parsed_pattern.add(1);
                    }
                }
                continue 'main; // Next character in pattern
            }

            // Not a verb name character. Process everything that must not
            // change the quantification state (comments, \Q, \E).
            if c == CHAR_BACKSLASH && ptr < ptrend {
                if *ptr as u32 == CHAR_Q || *ptr as u32 == CHAR_E {
                    // A literal inside a \Q...\E is not allowed if we are
                    // expecting a conditional assertion, but an empty \Q\E is OK.
                    if expect_cond_assert > 0
                        && *ptr as u32 == CHAR_Q
                        && !(ptrend.offset_from(ptr) >= 3
                            && *ptr.add(1) as u32 == CHAR_BACKSLASH
                            && *ptr.add(2) as u32 == CHAR_E)
                    {
                        ptr = ptr.sub(1);
                        errorcode = ERR28;
                        failed!();
                    }
                    inescq = (*ptr as u32 == CHAR_Q) as BOOL;
                    ptr = ptr.add(1);
                    continue 'main;
                }
            }

            // Skip over whitespace and # comments in extended mode.
            if (options & PCRE2_EXTENDED as u32) != 0 {
                if c < 256 && (*(*cb).ctypes.add(c as usize) as u32 & ctype_space as u32) != 0 {
                    continue 'main;
                }
                if c == CHAR_NEL || (c | 1) == 0x200f || (c | 1) == 0x2029 {
                    continue 'main;
                }
                if c == CHAR_NUMBER_SIGN {
                    while ptr < ptrend {
                        if is_newline(ptr, ptrend, cb, utf_b) {
                            // IS_NEWLINE sets cb->nllen.
                            ptr = ptr.add((*cb).nllen as usize);
                            break;
                        }
                        ptr = ptr.add(1);
                        if utf_b {
                            FORWARDCHARTEST(&mut ptr, ptrend);
                        }
                    }
                    continue 'main; // Next character in pattern
                }
            }

            // Skip over bracketed comments
            if c == CHAR_LEFT_PARENTHESIS
                && ptrend.offset_from(ptr) >= 2
                && *ptr as u32 == CHAR_QUESTION_MARK
                && *ptr.add(1) as u32 == CHAR_NUMBER_SIGN
            {
                loop {
                    ptr = ptr.add(1);
                    if !(ptr < ptrend && *ptr as u32 != CHAR_RIGHT_PARENTHESIS) {
                        break;
                    }
                }
                if ptr >= ptrend {
                    errorcode = ERR18; // Missing ) in comment
                    failed!();
                }
                ptr = ptr.add(1);
                continue 'main; // Next character in pattern
            }

            // If the next item is not a quantifier, fill in length of any
            // previous callout and create an auto callout if required.
            if c != CHAR_ASTERISK
                && c != CHAR_PLUS
                && c != CHAR_QUESTION_MARK
                && (c != CHAR_LEFT_CURLY_BRACKET || {
                    tempptr = ptr;
                    let mut tp = tempptr;
                    let r = read_repeat_counts(
                        &mut tp,
                        ptrend,
                        ptr::null_mut(),
                        ptr::null_mut(),
                        &mut errorcode,
                    );
                    r == 0
                })
            {
                let tmp = after_manual_callout;
                after_manual_callout -= 1;
                if tmp <= 0 {
                    parsed_pattern = manage_callouts(
                        thisptr,
                        &mut previous_callout,
                        auto_callout,
                        parsed_pattern,
                        cb,
                    );
                    this_parsed_item = parsed_pattern; // New start for current item
                }
            }

            // Handle expect_cond_assert: an assertion is expected next.
            if expect_cond_assert > 0 {
                let mut ok: BOOL = (c == CHAR_LEFT_PARENTHESIS
                    && ptrend.offset_from(ptr) >= 3
                    && (*ptr as u32 == CHAR_QUESTION_MARK || *ptr as u32 == CHAR_ASTERISK))
                    as BOOL;
                if ok != 0 {
                    if *ptr as u32 == CHAR_ASTERISK {
                        // New alpha assertion format, possibly
                        ok = (MAX_255(*ptr.add(1) as u32)
                            && (*(*cb).ctypes.add(*ptr.add(1) as usize) as u32
                                & ctype_lcletter as u32)
                                != 0) as BOOL;
                    } else {
                        // Traditional symbolic format
                        match *ptr.add(1) as u32 {
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
                        failed!();
                    }
                    failed_back!();
                }
            }

            // Remember whether we are expecting a conditional assertion.
            prev_expect_cond_assert = expect_cond_assert;
            expect_cond_assert = 0;

            // Remember quantification status for the previous significant item.
            prev_okquantifier = okquantifier;
            prev_meta_quantifier = meta_quantifier;
            okquantifier = FALSE;
            meta_quantifier = 0;

            // If the previous significant item was a quantifier, adjust the
            // parsed code if there is a following modifier.
            if prev_meta_quantifier != 0 && (c == CHAR_QUESTION_MARK || c == CHAR_PLUS) {
                let idx: isize = if prev_meta_quantifier == META_MINMAX as u32 {
                    -3
                } else {
                    -1
                };
                *parsed_pattern.offset(idx) = prev_meta_quantifier
                    + (if c == CHAR_QUESTION_MARK {
                        0x00020000u32
                    } else {
                        0x00010000u32
                    });
                continue 'main; // Next character in pattern
            }

            // --------------------------------------------------------------
            // Process the next item in the main part of a pattern.
            //
            // The C code uses a `switch (c)` with numerous cross-case gotos.
            // We drive them with a dispatch loop over `Jump`.
            // --------------------------------------------------------------
            let mut jump = Jump::Switch;
            'dispatch: loop {
                match jump {
                    // ------------------------------------------------------
                    Jump::Switch => {
                        match c {
                            // default: Non-special character
                            _ if c != CHAR_BACKSLASH
                                && c != CHAR_CIRCUMFLEX_ACCENT
                                && c != CHAR_DOLLAR_SIGN
                                && c != CHAR_DOT
                                && c != CHAR_ASTERISK
                                && c != CHAR_PLUS
                                && c != CHAR_QUESTION_MARK
                                && c != CHAR_LEFT_CURLY_BRACKET
                                && c != CHAR_LEFT_SQUARE_BRACKET
                                && c != CHAR_LEFT_PARENTHESIS
                                && c != CHAR_VERTICAL_LINE
                                && c != CHAR_RIGHT_PARENTHESIS =>
                            {
                                // PARSED_LITERAL(c, parsed_pattern)
                                *parsed_pattern = c;
                                parsed_pattern = parsed_pattern.add(1);
                                okquantifier = TRUE;
                                break 'dispatch;
                            }

                            // ---- Escape sequence ----
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

                                // ESCAPE_FAILED handling: this is a label that,
                                // when errorcode != 0, either fails outright or
                                // (with EXTRA_BAD_ESCAPE_IS_LITERAL) recovers
                                // and treats the item as a literal. Several
                                // `goto ESCAPE_FAILED` occur below; we model it
                                // with an inner labelled loop.
                                'escape_processing: loop {
                                    macro_rules! escape_failed {
                                        () => {{
                                            if (xoptions
                                                & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL as u32)
                                                == 0
                                            {
                                                failed!();
                                            }
                                            ptr = tempptr;
                                            if ptr >= ptrend {
                                                c = CHAR_BACKSLASH;
                                            } else {
                                                c = GETCHARINCTEST(&mut ptr, utf_b);
                                            }
                                            // C: the ESCAPE_FAILED label sets escape = 0
                                            // and then falls through to
                                            //     if (escape == 0) PARSED_LITERAL(c, ...)
                                            // and breaks out of the switch. Every
                                            // `goto ESCAPE_FAILED` therefore emits one
                                            // literal code unit; reproduce that here.
                                            escape = 0;
                                            *parsed_pattern = c;
                                            parsed_pattern = parsed_pattern.add(1);
                                            okquantifier = TRUE;
                                            break 'escape_processing;
                                        }};
                                    }

                                    if errorcode != 0 {
                                        escape_failed!();
                                    }

                                    // The escape was a data escape or literal.
                                    if escape == 0 {
                                        // PARSED_LITERAL(c, parsed_pattern)
                                        *parsed_pattern = c;
                                        parsed_pattern = parsed_pattern.add(1);
                                        okquantifier = TRUE;
                                    }
                                    // The escape was a back (or forward) reference.
                                    else if escape < 0 {
                                        offset = ptr.offset_from((*cb).start_pattern)
                                            as PCRE2_SIZE;
                                        escape = -escape - 1;
                                        *parsed_pattern =
                                            (META_BACKREF as u32) | (escape as u32);
                                        parsed_pattern = parsed_pattern.add(1);
                                        if escape < 10 {
                                            if (*cb).small_ref_offset[escape as usize]
                                                == crate::internal::PCRE2_UNSET
                                            {
                                                (*cb).small_ref_offset[escape as usize] =
                                                    offset;
                                            }
                                        } else {
                                            PUTOFFSET(offset, &mut parsed_pattern);
                                        }
                                        okquantifier = TRUE;
                                    }
                                    // A character class escape / special escape.
                                    else {
                                        // else switch (escape)
                                        if escape == ESC_C as c_int {
                                            // NEVER_BACKSLASH_C not defined.
                                            if (options & PCRE2_NEVER_BACKSLASH_C as u32)
                                                != 0
                                            {
                                                errorcode = ERR83;
                                                // C: `goto ESCAPE_FAILED`, which emits
                                                // one literal and leaves the switch.
                                                escape_failed!();
                                            } else {
                                                okquantifier = TRUE;
                                                *parsed_pattern =
                                                    (META_ESCAPE as u32) + escape as u32;
                                                parsed_pattern = parsed_pattern.add(1);
                                                break 'escape_processing;
                                            }
                                        }
                                        {
                                            if escape == ESC_ub as c_int {
                                                *parsed_pattern = CHAR_u;
                                                parsed_pattern = parsed_pattern.add(1);
                                                // PARSED_LITERAL(CHAR_LEFT_CURLY_BRACKET)
                                                *parsed_pattern = CHAR_LEFT_CURLY_BRACKET;
                                                parsed_pattern = parsed_pattern.add(1);
                                                okquantifier = TRUE;
                                                break 'escape_processing;
                                            } else if escape == ESC_X as c_int
                                                || escape == ESC_H as c_int
                                                || escape == ESC_h as c_int
                                                || escape == ESC_N as c_int
                                                || escape == ESC_R as c_int
                                                || escape == ESC_V as c_int
                                                || escape == ESC_v as c_int
                                            {
                                                // ESC_X supported only with Unicode (it is).
                                                okquantifier = TRUE;
                                                *parsed_pattern =
                                                    (META_ESCAPE as u32) + escape as u32;
                                                parsed_pattern = parsed_pattern.add(1);
                                                break 'escape_processing;
                                            } else if escape == ESC_d as c_int
                                                || escape == ESC_D as c_int
                                                || escape == ESC_s as c_int
                                                || escape == ESC_S as c_int
                                                || escape == ESC_w as c_int
                                                || escape == ESC_W as c_int
                                            {
                                                okquantifier = TRUE;
                                                parsed_pattern = handle_escdsw(
                                                    escape,
                                                    parsed_pattern,
                                                    options,
                                                    xoptions,
                                                );
                                                break 'escape_processing;
                                            } else if escape == ESC_P as c_int
                                                || escape == ESC_p as c_int
                                            {
                                                // Unicode property matching.
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
                                                    escape_failed!();
                                                    // fall through to literal handling
                                                } else {
                                                    if negated != 0 {
                                                        escape = if escape == ESC_P as c_int
                                                        {
                                                            ESC_p as c_int
                                                        } else {
                                                            ESC_P as c_int
                                                        };
                                                    }
                                                    *parsed_pattern =
                                                        (META_ESCAPE as u32) + escape as u32;
                                                    parsed_pattern = parsed_pattern.add(1);
                                                    *parsed_pattern =
                                                        ((ptype as u32) << 16) | pdata as u32;
                                                    parsed_pattern = parsed_pattern.add(1);
                                                    okquantifier = TRUE;
                                                    break 'escape_processing;
                                                }
                                            } else if escape == ESC_g as c_int
                                                || escape == ESC_k as c_int
                                            {
                                                // \g and \k with quotes/braces.
                                                if ptr >= ptrend
                                                    || (*ptr as u32
                                                        != CHAR_LEFT_CURLY_BRACKET
                                                        && *ptr as u32
                                                            != CHAR_LESS_THAN_SIGN
                                                        && *ptr as u32 != CHAR_APOSTROPHE)
                                                {
                                                    errorcode = if escape == ESC_g as c_int {
                                                        ERR57
                                                    } else {
                                                        ERR69
                                                    };
                                                    escape_failed!();
                                                    // fall to literal handling below
                                                } else {
                                                    terminator = if *ptr as u32
                                                        == CHAR_LESS_THAN_SIGN
                                                    {
                                                        CHAR_GREATER_THAN_SIGN
                                                    } else if *ptr as u32 == CHAR_APOSTROPHE
                                                    {
                                                        CHAR_APOSTROPHE
                                                    } else {
                                                        CHAR_RIGHT_CURLY_BRACKET
                                                    };

                                                    // For a non-braced \g, check
                                                    // for a numerical recursion.
                                                    let mut did_recursion = false;
                                                    if escape == ESC_g as c_int
                                                        && terminator
                                                            != CHAR_RIGHT_CURLY_BRACKET
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
                                                        ) != 0
                                                        {
                                                            if p >= ptrend
                                                                || *p as u32 != terminator
                                                            {
                                                                ptr = p;
                                                                errorcode = ERR119;
                                                                // C: `goto ESCAPE_FAILED`
                                                                escape_failed!();
                                                            } else {
                                                                ptr = p.add(1);
                                                                // goto SET_RECURSION
                                                                jump = Jump::SetRecursion;
                                                                continue 'dispatch;
                                                            }
                                                        } else if errorcode != 0 {
                                                            escape_failed!();
                                                        }
                                                        let _ = did_recursion;
                                                    }

                                                    // Not a numerical recursion.
                                                    if escape != 0 {
                                                        // still non-zero unless
                                                        // escape_failed reset it
                                                        if !read_name_ok(
                                                            &mut ptr,
                                                            ptrend,
                                                            utf,
                                                            terminator,
                                                            &mut offset,
                                                            &mut name,
                                                            &mut namelen,
                                                            &mut errorcode,
                                                            cb,
                                                        ) {
                                                            // C: `goto ESCAPE_FAILED`
                                                            escape_failed!();
                                                        }

                                                        *parsed_pattern = if escape
                                                            == ESC_k as c_int
                                                            || terminator
                                                                == CHAR_RIGHT_CURLY_BRACKET
                                                        {
                                                            META_BACKREF_BYNAME as u32
                                                        } else {
                                                            META_RECURSE_BYNAME as u32
                                                        };
                                                        parsed_pattern =
                                                            parsed_pattern.add(1);
                                                        *parsed_pattern = namelen;
                                                        parsed_pattern =
                                                            parsed_pattern.add(1);
                                                        PUTOFFSET(offset, &mut parsed_pattern);
                                                        okquantifier = TRUE;
                                                        break 'escape_processing;
                                                    }
                                                }
                                            } else {
                                                // default: \A \B \b \G \K \Z \z
                                                // cannot be quantified.
                                                *parsed_pattern =
                                                    (META_ESCAPE as u32) + escape as u32;
                                                parsed_pattern = parsed_pattern.add(1);
                                                break 'escape_processing;
                                            }
                                        }
                                    }

                                    break 'escape_processing;
                                }
                                break 'dispatch;
                            }

                            // ---- Single-character special items ----
                            CHAR_CIRCUMFLEX_ACCENT => {
                                *parsed_pattern = META_CIRCUMFLEX as u32;
                                parsed_pattern = parsed_pattern.add(1);
                                break 'dispatch;
                            }

                            CHAR_DOLLAR_SIGN => {
                                *parsed_pattern = META_DOLLAR as u32;
                                parsed_pattern = parsed_pattern.add(1);
                                break 'dispatch;
                            }

                            CHAR_DOT => {
                                *parsed_pattern = META_DOT as u32;
                                parsed_pattern = parsed_pattern.add(1);
                                okquantifier = TRUE;
                                break 'dispatch;
                            }

                            // ---- Single-character quantifiers ----
                            CHAR_ASTERISK => {
                                meta_quantifier = META_ASTERISK as u32;
                                jump = Jump::CheckQuantifier;
                                continue 'dispatch;
                            }
                            CHAR_PLUS => {
                                meta_quantifier = META_PLUS as u32;
                                jump = Jump::CheckQuantifier;
                                continue 'dispatch;
                            }
                            CHAR_QUESTION_MARK => {
                                meta_quantifier = META_QUERY as u32;
                                jump = Jump::CheckQuantifier;
                                continue 'dispatch;
                            }

                            // ---- Potential {n,m} quantifier ----
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
                                        failed!(); // Error in quantifier.
                                    }
                                    // Not a quantifier
                                    *parsed_pattern = c;
                                    parsed_pattern = parsed_pattern.add(1);
                                    okquantifier = TRUE;
                                    break 'dispatch; // No more quantifier processing
                                }
                                meta_quantifier = META_MINMAX as u32;
                                // Fall through to CHECK_QUANTIFIER
                                jump = Jump::CheckQuantifier;
                                continue 'dispatch;
                            }

                            // ---- Character class ----
                            CHAR_LEFT_SQUARE_BRACKET => {
                                // [[:<:]] / [[:>:]] special word boundaries.
                                if ptrend.offset_from(ptr) >= 6
                                    && (strncmp_c8(ptr, STRING_WEIRD_STARTWORD, 6) == 0
                                        || strncmp_c8(ptr, STRING_WEIRD_ENDWORD, 6) == 0)
                                {
                                    *parsed_pattern = (META_ESCAPE as u32) + ESC_b;
                                    parsed_pattern = parsed_pattern.add(1);

                                    if *ptr.add(2) as u32 == CHAR_LESS_THAN_SIGN {
                                        *parsed_pattern = META_LOOKAHEAD as u32;
                                        parsed_pattern = parsed_pattern.add(1);
                                    } else {
                                        *parsed_pattern = META_LOOKBEHIND as u32;
                                        parsed_pattern = parsed_pattern.add(1);
                                        *has_lookbehind = TRUE;
                                        // The offset is used only for the
                                        // "non-fixed length" error; store zero.
                                        PUTOFFSET(0 as PCRE2_SIZE, &mut parsed_pattern);
                                    }

                                    if (options & PCRE2_UCP as u32) == 0 {
                                        *parsed_pattern = (META_ESCAPE as u32) + ESC_w;
                                        parsed_pattern = parsed_pattern.add(1);
                                    } else {
                                        *parsed_pattern = (META_ESCAPE as u32) + ESC_p;
                                        parsed_pattern = parsed_pattern.add(1);
                                        *parsed_pattern = (PT_WORD as u32) << 16;
                                        parsed_pattern = parsed_pattern.add(1);
                                    }
                                    *parsed_pattern = META_KET as u32;
                                    parsed_pattern = parsed_pattern.add(1);
                                    ptr = ptr.add(6);
                                    okquantifier = TRUE;
                                    break 'dispatch;
                                }

                                // POSIX class at top level is an error.
                                if ptr < ptrend
                                    && (*ptr as u32 == CHAR_COLON
                                        || *ptr as u32 == CHAR_DOT
                                        || *ptr as u32 == CHAR_EQUALS_SIGN)
                                    && {
                                        tempptr = ptr;
                                        check_posix_syntax(ptr, ptrend, &mut tempptr) != 0
                                    }
                                {
                                    errorcode = if *ptr as u32 == CHAR_COLON {
                                        ERR12
                                    } else {
                                        ERR13
                                    };
                                    ptr = ptr.sub(1);
                                    ptr = tempptr.add(2);
                                    failed!();
                                }

                                class_mode_state =
                                    if (options & PCRE2_ALT_EXTENDED_CLASS as u32) != 0 {
                                        CLASS_MODE_ALT_EXT
                                    } else {
                                        CLASS_MODE_NORMAL
                                    };

                                // FROM_PERL_EXTENDED_CLASS entry point.
                                jump = Jump::FromPerlExtendedClass;
                                continue 'dispatch;
                            }

                            // ---- Opening parenthesis ----
                            CHAR_LEFT_PARENTHESIS => {
                                // Handled in a dedicated inline block below.
                                match parse_open_paren(
                                    &mut ptr,
                                    ptrend,
                                    &mut c,
                                    &mut options,
                                    &mut xoptions,
                                    &mut parsed_pattern,
                                    &mut previous_callout,
                                    &mut verbstartptr,
                                    &mut verblengthptr,
                                    &mut verbnamestart,
                                    &mut inverbname,
                                    &mut add_after_mark,
                                    &mut okquantifier,
                                    &mut expect_cond_assert,
                                    prev_expect_cond_assert,
                                    &mut after_manual_callout,
                                    &mut nest_depth,
                                    &mut top_nest,
                                    end_nests,
                                    &mut class_mode_state,
                                    has_lookbehind,
                                    &mut offset,
                                    &mut terminator,
                                    &mut name,
                                    &mut namelen,
                                    &mut i,
                                    &mut errorcode,
                                    utf,
                                    cb,
                                ) {
                                    OpenParen::Break => break 'dispatch,
                                    OpenParen::NextChar => {
                                        continue 'main;
                                    }
                                    OpenParen::Failed => {
                                        (*cb).erroroffset = ptr.offset_from((*cb).start_pattern)
                                            as PCRE2_SIZE;
                                        return errorcode;
                                    }
                                    OpenParen::Jump(j) => {
                                        jump = j;
                                        continue 'dispatch;
                                    }
                                }
                            }

                            // ---- Branch terminators ----
                            CHAR_VERTICAL_LINE => {
                                // Alternation: reset capture count in (?| group.
                                if !top_nest.is_null()
                                    && (*top_nest).nest_depth == nest_depth
                                    && ((*top_nest).flags as u32 & NSF_RESET) != 0
                                {
                                    if (*cb).bracount > (*top_nest).max_group as u32 {
                                        (*top_nest).max_group = (*cb).bracount as u16;
                                    }
                                    (*cb).bracount = (*top_nest).reset_group as u32;
                                }
                                *parsed_pattern = META_ALT as u32;
                                parsed_pattern = parsed_pattern.add(1);
                                break 'dispatch;
                            }

                            CHAR_RIGHT_PARENTHESIS => {
                                okquantifier = TRUE;
                                if !top_nest.is_null()
                                    && (*top_nest).nest_depth == nest_depth
                                {
                                    options = (options & !(PARSE_TRACKED_OPTIONS))
                                        | (*top_nest).options;
                                    xoptions = (xoptions & !(PARSE_TRACKED_EXTRA_OPTIONS))
                                        | (*top_nest).xoptions;
                                    if ((*top_nest).flags as u32 & NSF_RESET) != 0
                                        && (*top_nest).max_group as u32 > (*cb).bracount
                                    {
                                        (*cb).bracount = (*top_nest).max_group as u32;
                                    }
                                    if ((*top_nest).flags as u32 & NSF_CONDASSERT) != 0 {
                                        okquantifier = FALSE;
                                    }

                                    if ((*top_nest).flags as u32 & NSF_ATOMICSR) != 0 {
                                        *parsed_pattern = META_KET as u32;
                                        parsed_pattern = parsed_pattern.add(1);
                                    }

                                    if top_nest == (*cb).start_workspace as *mut nest_save {
                                        top_nest = ptr::null_mut();
                                    } else {
                                        top_nest = top_nest.sub(1);
                                    }
                                }
                                if nest_depth == 0 {
                                    // Unmatched closing parenthesis
                                    errorcode = ERR22;
                                    failed!();
                                }
                                nest_depth -= 1;
                                *parsed_pattern = META_KET as u32;
                                parsed_pattern = parsed_pattern.add(1);
                                break 'dispatch;
                            }

                            // Unreachable: all cases covered above.
                            _ => {
                                break 'dispatch;
                            }
                        }
                    }

                    // ------------------------------------------------------
                    // CHECK_QUANTIFIER
                    // ------------------------------------------------------
                    Jump::CheckQuantifier => {
                        if prev_okquantifier == 0 {
                            errorcode = ERR9;
                            failed!();
                        }

                        // Allow (*ACCEPT) to be quantified by wrapping it.
                        if *prev_parsed_item == META_ACCEPT as u32 {
                            let mut p = parsed_pattern.sub(1);
                            while p >= verbstartptr {
                                *p.add(1) = *p;
                                p = p.sub(1);
                            }
                            *verbstartptr = META_NOCAPTURE as u32;
                            *parsed_pattern.add(1) = META_KET as u32;
                            parsed_pattern = parsed_pattern.add(2);
                        }

                        // Put the quantifier into the parsed pattern vector.
                        *parsed_pattern = meta_quantifier;
                        parsed_pattern = parsed_pattern.add(1);
                        if c == CHAR_LEFT_CURLY_BRACKET {
                            *parsed_pattern = min_repeat;
                            parsed_pattern = parsed_pattern.add(1);
                            *parsed_pattern = max_repeat;
                            parsed_pattern = parsed_pattern.add(1);
                        }
                        break 'dispatch;
                    }

                    // ------------------------------------------------------
                    // FROM_PERL_EXTENDED_CLASS — the class parser.
                    // ------------------------------------------------------
                    Jump::FromPerlExtendedClass => {
                        match parse_class(
                            &mut ptr,
                            ptrend,
                            &mut c,
                            options,
                            xoptions,
                            &mut parsed_pattern,
                            &mut inescq,
                            &mut class_mode_state,
                            &mut class_range_state,
                            &mut class_op_state,
                            &mut class_start,
                            &mut class_depth_m1,
                            &mut class_maxdepth_m1,
                            &mut class_range_forbid_ptr,
                            &mut negate_class,
                            &mut okquantifier,
                            &mut i,
                            &mut errorcode,
                            utf,
                            cb,
                        ) {
                            ClassResult::Ok => break 'dispatch,
                            ClassResult::Failed => {
                                (*cb).erroroffset =
                                    ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                                return errorcode;
                            }
                        }
                    }

                    // ------------------------------------------------------
                    // SET_RECURSION / RECURSION_BYNUMBER / RECURSE_BY_NAME /
                    // READ_RECURSION_ARGUMENTS
                    // ------------------------------------------------------
                    Jump::RecursionByNumber => {
                        if read_number(
                            &mut ptr,
                            ptrend,
                            if IS_DIGIT(*ptr as u32) {
                                -1
                            } else {
                                (*cb).bracount as i32
                            },
                            MAX_GROUP_NUMBER,
                            ERR61 as u32,
                            &mut i,
                            &mut errorcode,
                        ) == 0
                        {
                            failed!();
                        }
                        // PCRE2_ASSERT(i >= 0);
                        terminator = CHAR_NUL;
                        jump = Jump::SetRecursion;
                        continue 'dispatch;
                    }

                    Jump::SetRecursion => {
                        *parsed_pattern = (META_RECURSE as u32) | (i as u32);
                        parsed_pattern = parsed_pattern.add(1);
                        offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                        // goto READ_RECURSION_ARGUMENTS
                        jump = Jump::ReadRecursionArguments;
                        // terminator was set by the caller (CHAR_NUL for number).
                        continue 'dispatch;
                    }

                    Jump::RecurseByName => {
                        if !read_name_ok(
                            &mut ptr,
                            ptrend,
                            utf,
                            0,
                            &mut offset,
                            &mut name,
                            &mut namelen,
                            &mut errorcode,
                            cb,
                        ) {
                            failed!();
                        }
                        *parsed_pattern = META_RECURSE_BYNAME as u32;
                        parsed_pattern = parsed_pattern.add(1);
                        *parsed_pattern = namelen;
                        parsed_pattern = parsed_pattern.add(1);
                        terminator = CHAR_NUL;
                        jump = Jump::ReadRecursionArguments;
                        continue 'dispatch;
                    }

                    Jump::ReadRecursionArguments => {
                        PUTOFFSET(offset, &mut parsed_pattern);
                        okquantifier = TRUE;

                        // Arguments are not supported for \g construct.
                        if terminator != CHAR_NUL {
                            break 'dispatch;
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
                                failed!();
                            }
                        }

                        if ptr >= ptrend || *ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                            unclosed_parenthesis!();
                        }

                        ptr = ptr.add(1);
                        break 'dispatch;
                    }

                    // ------------------------------------------------------
                    // Assertion tails, reachable from (? cases and from the
                    // alpha-assertion dispatch inside parse_open_paren.
                    // ------------------------------------------------------
                    Jump::AtomicGroup => {
                        *parsed_pattern = META_ATOMIC as u32;
                        parsed_pattern = parsed_pattern.add(1);
                        nest_depth += 1;
                        ptr = ptr.add(1);
                        break 'dispatch;
                    }

                    Jump::PositiveLookAhead => {
                        *parsed_pattern = META_LOOKAHEAD as u32;
                        parsed_pattern = parsed_pattern.add(1);
                        ptr = ptr.add(1);
                        jump = Jump::PostAssertion;
                        continue 'dispatch;
                    }

                    Jump::PositiveNonatomicLookAhead => {
                        *parsed_pattern = META_LOOKAHEAD_NA as u32;
                        parsed_pattern = parsed_pattern.add(1);
                        ptr = ptr.add(1);
                        jump = Jump::PostAssertion;
                        continue 'dispatch;
                    }

                    Jump::NegativeLookAhead => {
                        *parsed_pattern = META_LOOKAHEADNOT as u32;
                        parsed_pattern = parsed_pattern.add(1);
                        ptr = ptr.add(1);
                        jump = Jump::PostAssertion;
                        continue 'dispatch;
                    }

                    Jump::PostLookbehind => {
                        *has_lookbehind = TRUE;
                        offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE - 2;
                        PUTOFFSET(offset, &mut parsed_pattern);
                        ptr = ptr.add(2);
                        // Fall through to POST_ASSERTION
                        jump = Jump::PostAssertion;
                        continue 'dispatch;
                    }

                    Jump::PostAssertion => {
                        nest_depth += 1;
                        if prev_expect_cond_assert > 0 {
                            if top_nest.is_null() {
                                top_nest = (*cb).start_workspace as *mut nest_save;
                            } else {
                                top_nest = top_nest.add(1);
                                if top_nest >= end_nests {
                                    errorcode = ERR84;
                                    failed!();
                                }
                            }
                            (*top_nest).nest_depth = nest_depth;
                            (*top_nest).flags = NSF_CONDASSERT as u16;
                            (*top_nest).options = options & PARSE_TRACKED_OPTIONS;
                            (*top_nest).xoptions = xoptions & PARSE_TRACKED_EXTRA_OPTIONS;
                        }
                        break 'dispatch;
                    }

                    Jump::DefineName => {
                        match parse_define_name(
                            &mut ptr,
                            ptrend,
                            terminator,
                            &mut parsed_pattern,
                            &mut nest_depth,
                            &mut is_dupname,
                            &mut hash,
                            &mut name,
                            &mut namelen,
                            &mut ng,
                            &mut i,
                            options,
                            &mut errorcode,
                            utf,
                            cb,
                        ) {
                            DefName::Break => break 'dispatch,
                            DefName::Failed => {
                                (*cb).erroroffset =
                                    ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                                return errorcode;
                            }
                        }
                    }

                    Jump::NextChar => {
                        continue 'main;
                    }
                }
            }
            // End of switch on pattern character; loop for next char.
        }

        // End of main character scan loop. Check for missing ) at the end of a
        // verb name.
        if inverbname != 0 && ptr >= ptrend {
            errorcode = ERR60;
            (*cb).erroroffset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
            return errorcode;
        }

        // PARSED_END and the trailing failure handling.
        parse_regex_end(
            &mut ptr,
            ptrend,
            &mut parsed_pattern,
            parsed_pattern_end,
            &mut previous_callout,
            auto_callout,
            xoptions,
            inverbname,
            nest_depth,
            utf_b,
            &mut errorcode,
            cb,
        )
    }
}

// ---------------------------------------------------------------------------
// CHAR_RIGHT_CURLY_BRACKET (used in escape processing)
// ---------------------------------------------------------------------------
const CHAR_RIGHT_CURLY_BRACKET: u32 = 0x7d;

// ---------------------------------------------------------------------------
// IS_NEWLINE — the IS_NEWLINE(p) macro with NLBLOCK == cb, PSEND == end_pattern.
// ---------------------------------------------------------------------------
#[inline(always)]
unsafe fn is_newline(p: PCRE2_SPTR, end_pattern: PCRE2_SPTR, cb: *mut compile_block, utf: bool) -> bool {
    unsafe {
        if (*cb).nltype != NLTYPE_FIXED as u32 {
            p < end_pattern
                && crate::newline::_pcre2_is_newline_8(
                    p,
                    (*cb).nltype,
                    end_pattern,
                    &mut (*cb).nllen,
                    utf as BOOL,
                ) != 0
        } else {
            p <= end_pattern.sub((*cb).nllen as usize)
                && *p == (*cb).nl[0]
                && ((*cb).nllen == 1 || *p.add(1) == (*cb).nl[1])
        }
    }
}

// ---------------------------------------------------------------------------
// read_name wrapper returning bool
// ---------------------------------------------------------------------------
#[inline(always)]
unsafe fn read_name_ok(
    ptrptr: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    utf: BOOL,
    terminator: u32,
    offsetptr: *mut PCRE2_SIZE,
    nameptr: *mut PCRE2_SPTR,
    namelenptr: *mut u32,
    errorcodeptr: *mut c_int,
    cb: *mut compile_block,
) -> bool {
    unsafe {
        read_name(
            ptrptr,
            ptrend,
            utf,
            terminator,
            offsetptr,
            nameptr,
            namelenptr,
            errorcodeptr,
            cb,
        ) != 0
    }
}

// ---------------------------------------------------------------------------
// PARSED_END tail (shared by the literal fast-path and the main loop end).
// ---------------------------------------------------------------------------
unsafe fn parse_regex_end(
    ptr: *mut PCRE2_SPTR,
    _ptrend: PCRE2_SPTR,
    parsed_pattern_ref: *mut *mut u32,
    parsed_pattern_end: *mut u32,
    previous_callout: *mut *mut u32,
    auto_callout: BOOL,
    xoptions: u32,
    _inverbname: BOOL,
    nest_depth: u16,
    _utf: bool,
    errorcode_ref: *mut c_int,
    cb: *mut compile_block,
) -> c_int {
    unsafe {
        let mut parsed_pattern = *parsed_pattern_ref;
        let mut errorcode = *errorcode_ref;

        // Manage callout for the final item.
        parsed_pattern = manage_callouts(*ptr, previous_callout, auto_callout, parsed_pattern, cb);

        // Insert trailing items for word and line matching.
        if (xoptions & PCRE2_EXTRA_MATCH_LINE as u32) != 0 {
            *parsed_pattern = META_KET as u32;
            parsed_pattern = parsed_pattern.add(1);
            *parsed_pattern = META_DOLLAR as u32;
            parsed_pattern = parsed_pattern.add(1);
        } else if (xoptions & PCRE2_EXTRA_MATCH_WORD as u32) != 0 {
            *parsed_pattern = META_KET as u32;
            parsed_pattern = parsed_pattern.add(1);
            *parsed_pattern = (META_ESCAPE as u32) + ESC_b;
            parsed_pattern = parsed_pattern.add(1);
        }

        // Terminate the parsed pattern.
        // LCOV_EXCL_START
        if parsed_pattern >= parsed_pattern_end {
            errorcode = ERR63; // Internal error (parsed pattern overflow)
            *parsed_pattern_ref = parsed_pattern;
            *errorcode_ref = errorcode;
            (*cb).erroroffset = (*ptr).offset_from((*cb).start_pattern) as PCRE2_SIZE;
            return errorcode;
        }
        // LCOV_EXCL_STOP

        *parsed_pattern = META_END as u32;
        *parsed_pattern_ref = parsed_pattern;
        if nest_depth == 0 {
            return 0;
        }

        // UNCLOSED_PARENTHESIS / FAILED.
        errorcode = ERR14;
        *errorcode_ref = errorcode;
        (*cb).erroroffset = (*ptr).offset_from((*cb).start_pattern) as PCRE2_SIZE;
        errorcode
    }
}

// ---------------------------------------------------------------------------
// DEFINE_NAME — define a named capturing group.
// ---------------------------------------------------------------------------
enum DefName {
    Break,
    Failed,
}

unsafe fn parse_define_name(
    ptr_ref: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    terminator: u32,
    parsed_pattern_ref: *mut *mut u32,
    nest_depth_ref: *mut u16,
    is_dupname_ref: *mut BOOL,
    hash_ref: *mut u16,
    name_ref: *mut PCRE2_SPTR,
    namelen_ref: *mut u32,
    ng_ref: *mut *mut named_group,
    i_ref: *mut c_int,
    options: u32,
    errorcode_ref: *mut c_int,
    utf: BOOL,
    cb: *mut compile_block,
) -> DefName {
    unsafe {
        let mut ptr = *ptr_ref;
        let mut parsed_pattern = *parsed_pattern_ref;
        let mut errorcode = *errorcode_ref;
        let mut offset: PCRE2_SIZE = 0;
        let mut name: PCRE2_SPTR = *name_ref;
        let mut namelen: u32 = *namelen_ref;

        macro_rules! done_break {
            () => {{
                *ptr_ref = ptr;
                *parsed_pattern_ref = parsed_pattern;
                *errorcode_ref = errorcode;
                *name_ref = name;
                *namelen_ref = namelen;
                return DefName::Break;
            }};
        }
        macro_rules! fail {
            () => {{
                *ptr_ref = ptr;
                *parsed_pattern_ref = parsed_pattern;
                *errorcode_ref = errorcode;
                return DefName::Failed;
            }};
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
            fail!();
        }

        // We have a name for this capturing group. It is also assigned a number.
        if (*cb).bracount >= MAX_GROUP_NUMBER {
            errorcode = ERR97;
            fail!();
        }
        (*cb).bracount += 1;
        *parsed_pattern = (META_CAPTURE as u32) | (*cb).bracount;
        parsed_pattern = parsed_pattern.add(1);
        *nest_depth_ref += 1;

        // Check not too many names.
        if (*cb).names_found as i64 >= MAX_NAME_COUNT {
            errorcode = ERR49;
            fail!();
        }

        // Adjust the entry size to accommodate the longest name found.
        if namelen + IMM2_SIZE_U as u32 + 1 > (*cb).name_entry_size as u32 {
            (*cb).name_entry_size = (namelen + IMM2_SIZE_U as u32 + 1) as u16;
        }

        // Scan the list to check for duplicates.
        let mut is_dupname: BOOL = FALSE;
        let hash: u16 = _pcre2_compile_get_hash_from_name8(name, namelen);
        let mut ng: *mut named_group = (*cb).named_groups;
        let mut i: c_int = 0;
        while i < (*cb).names_found as c_int {
            if namelen == (*ng).length as u32
                && hash == NAMED_GROUP_GET_HASH(ng)
                && strncmp(name, (*ng).name, namelen as usize) == 0
            {
                // When referenced by the same name multiple times, not a dup.
                if (*ng).number == (*cb).bracount {
                    break;
                }
                if (options & PCRE2_DUPNAMES as u32) == 0 {
                    errorcode = ERR43;
                    fail!();
                }

                (*ng).hash_dup |= NAMED_GROUP_IS_DUPNAME_U;
                is_dupname = TRUE; // Mark as a duplicate
                (*cb).dupnames = TRUE; // Duplicate names exist

                // The entry represents a duplicate.
                name = (*ng).name;
                namelen = 0;

                // Even duplicated names may refer to the same capture index.
                while i < (*cb).names_found as c_int {
                    if (*ng).name == name && (*ng).number == (*cb).bracount {
                        break;
                    }
                    i += 1;
                    ng = ng.add(1);
                }
                break;
            } else if (*ng).number == (*cb).bracount {
                errorcode = ERR65;
                fail!();
            }
            i += 1;
            ng = ng.add(1);
        }

        // Ignore duplicate with same number.
        if i < (*cb).names_found as c_int {
            *is_dupname_ref = is_dupname;
            *hash_ref = hash;
            *ng_ref = ng;
            *i_ref = i;
            done_break!();
        }

        // Increase the list size if necessary.
        if (*cb).names_found as u32 >= (*cb).named_group_list_size {
            let newsize = (*cb).named_group_list_size * 2;
            let memctl = &(*(*cb).cx).memctl;
            let newspace = (memctl.malloc.unwrap())(
                newsize as usize * core::mem::size_of::<named_group>(),
                memctl.memory_data,
            ) as *mut named_group;
            if newspace.is_null() {
                errorcode = ERR21;
                fail!();
            }

            ptr::copy_nonoverlapping(
                (*cb).named_groups,
                newspace,
                (*cb).named_group_list_size as usize,
            );
            if (*cb).named_group_list_size as usize > NAMED_GROUP_LIST_SIZE {
                (memctl.free.unwrap())(
                    (*cb).named_groups as *mut core::ffi::c_void,
                    memctl.memory_data,
                );
            }
            (*cb).named_groups = newspace;
            (*cb).named_group_list_size = newsize;
        }

        // Add this name to the list.
        let mut hash = hash;
        if is_dupname != 0 {
            hash |= NAMED_GROUP_IS_DUPNAME_U;
        }

        let slot = (*cb).named_groups.add((*cb).names_found as usize);
        (*slot).name = name;
        (*slot).length = namelen as u16;
        (*slot).number = (*cb).bracount;
        (*slot).hash_dup = hash;
        (*cb).names_found += 1;

        *is_dupname_ref = is_dupname;
        *hash_ref = hash;
        *ng_ref = ng;
        *i_ref = i;
        done_break!();
    }
}

// ---------------------------------------------------------------------------
// FROM_PERL_EXTENDED_CLASS — the character-class parser.
//
// On entry `c` holds '[' and `ptr` points just after it; `class_mode_state`
// has been initialised by the caller.
// ---------------------------------------------------------------------------
enum ClassResult {
    Ok,
    Failed,
}

unsafe fn parse_class(
    ptr_ref: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    c_ref: *mut u32,
    options: u32,
    xoptions: u32,
    parsed_pattern_ref: *mut *mut u32,
    inescq_ref: *mut BOOL,
    class_mode_state_ref: *mut u32,
    class_range_state_ref: *mut u32,
    class_op_state_ref: *mut u32,
    class_start_ref: *mut *mut u32,
    class_depth_m1_ref: *mut i16,
    class_maxdepth_m1_ref: *mut i16,
    class_range_forbid_ptr_ref: *mut PCRE2_SPTR,
    negate_class_ref: *mut BOOL,
    okquantifier_ref: *mut BOOL,
    _i_ref: *mut c_int,
    errorcode_ref: *mut c_int,
    utf: BOOL,
    cb: *mut compile_block,
) -> ClassResult {
    unsafe {
        let utf_b = utf != 0;
        let mut ptr = *ptr_ref;
        let mut c = *c_ref;
        let mut parsed_pattern = *parsed_pattern_ref;
        let mut inescq = *inescq_ref;
        let mut class_mode_state = *class_mode_state_ref;
        let mut class_range_state = *class_range_state_ref;
        let mut class_op_state = *class_op_state_ref;
        let mut class_start = *class_start_ref;
        let mut class_depth_m1 = *class_depth_m1_ref;
        let mut class_maxdepth_m1 = *class_maxdepth_m1_ref;
        let mut class_range_forbid_ptr = *class_range_forbid_ptr_ref;
        let mut errorcode = *errorcode_ref;
        let mut tempptr: PCRE2_SPTR;

        macro_rules! fail {
            () => {{
                sync_out!();
                return ClassResult::Failed;
            }};
        }
        macro_rules! sync_out {
            () => {{
                *ptr_ref = ptr;
                *c_ref = c;
                *parsed_pattern_ref = parsed_pattern;
                *inescq_ref = inescq;
                *class_mode_state_ref = class_mode_state;
                *class_range_state_ref = class_range_state;
                *class_op_state_ref = class_op_state;
                *class_start_ref = class_start;
                *class_depth_m1_ref = class_depth_m1;
                *class_maxdepth_m1_ref = class_maxdepth_m1;
                *class_range_forbid_ptr_ref = class_range_forbid_ptr;
                *errorcode_ref = errorcode;
            }};
        }

        *okquantifier_ref = TRUE;

        class_depth_m1 = -1;
        class_maxdepth_m1 = -1;
        class_range_state = RANGE_NO;
        class_op_state = CLASS_OP_EMPTY;
        class_start = ptr::null_mut();

        // Loop for the contents of the class.
        'class_loop: loop {
            let mut char_is_literal: BOOL = TRUE;
            // Whether, after the dispatch, we should run the CLASS_LITERAL body.
            let mut run_class_literal = false;
            // Whether we should run the CLASS_CONTINUE tail (vs. `continue`).
            let mut run_class_continue = true;

            'dispatch: {
                // Inside \Q...\E everything is literal except \E.
                if inescq != 0 {
                    if c == CHAR_BACKSLASH && ptr < ptrend && *ptr as u32 == CHAR_E {
                        inescq = FALSE; // Reset literal state
                        ptr = ptr.add(1); // Skip the 'E'
                        // goto CLASS_CONTINUE
                        break 'dispatch;
                    }

                    // \Q..\E cannot escape a char inside a Perl extended class.
                    if class_mode_state == CLASS_MODE_PERL_EXT {
                        errorcode = ERR116;
                        fail!();
                    }

                    // goto CLASS_LITERAL
                    run_class_literal = true;
                    break 'dispatch;
                }

                // Skip space/tab in extended-more or Perl extended class.
                if (c == CHAR_SPACE || c == CHAR_HT)
                    && ((options & PCRE2_EXTENDED_MORE as u32) != 0
                        || class_mode_state >= CLASS_MODE_PERL_EXT)
                {
                    // goto CLASS_CONTINUE
                    break 'dispatch;
                }

                // Handle POSIX class names.
                if class_depth_m1 >= 0
                    && c == CHAR_LEFT_SQUARE_BRACKET
                    && ptrend.offset_from(ptr) >= 3
                    && (*ptr as u32 == CHAR_COLON
                        || *ptr as u32 == CHAR_DOT
                        || *ptr as u32 == CHAR_EQUALS_SIGN)
                    && {
                        tempptr = ptr;
                        check_posix_syntax(ptr, ptrend, &mut tempptr) != 0
                    }
                {
                    let mut posix_negate: BOOL = FALSE;
                    let posix_class: c_int;

                    // Hyphen before a POSIX class: error.
                    if class_range_state == RANGE_STARTED {
                        ptr = tempptr.add(2);
                        errorcode = ERR50;
                        fail!();
                    }

                    if class_range_state == RANGE_FORBID_STARTED {
                        ptr = class_range_forbid_ptr;
                        errorcode = ERR50;
                        fail!();
                    }

                    // Disallow implicit union in Perl extended classes.
                    if class_op_state == CLASS_OP_OPERAND
                        && class_mode_state == CLASS_MODE_PERL_EXT
                    {
                        ptr = tempptr.add(2);
                        errorcode = ERR113;
                        fail!();
                    }

                    if *ptr as u32 != CHAR_COLON {
                        ptr = tempptr.add(2);
                        errorcode = ERR13;
                        fail!();
                    }

                    ptr = ptr.add(1);
                    if *ptr as u32 == CHAR_CIRCUMFLEX_ACCENT {
                        posix_negate = TRUE;
                        ptr = ptr.add(1);
                    }

                    posix_class = check_posix_name(ptr, tempptr.offset_from(ptr) as c_int);
                    ptr = tempptr.add(2);
                    if posix_class < 0 {
                        errorcode = ERR30;
                        fail!();
                    }

                    // "a hyphen is forbidden to be the start of a range".
                    class_range_state = RANGE_FORBID_NO;
                    class_op_state = CLASS_OP_OPERAND;

                    // PCRE2_UCP conversions of some POSIX classes.
                    if (options & PCRE2_UCP as u32) != 0
                        && (xoptions & PCRE2_EXTRA_ASCII_POSIX as u32) == 0
                        && !((xoptions & PCRE2_EXTRA_ASCII_DIGIT as u32) != 0
                            && (posix_class as i64 == PC_DIGIT
                                || posix_class as i64 == PC_XDIGIT))
                    {
                        let ptype = POSIX_SUBSTITUTES[2 * posix_class as usize];
                        let pvalue = POSIX_SUBSTITUTES[2 * posix_class as usize + 1];

                        if ptype >= 0 {
                            *parsed_pattern = (META_ESCAPE as u32)
                                + if posix_negate != 0 { ESC_P } else { ESC_p };
                            parsed_pattern = parsed_pattern.add(1);
                            *parsed_pattern =
                                ((ptype as u32) << 16) | pvalue as u32;
                            parsed_pattern = parsed_pattern.add(1);
                            // goto CLASS_CONTINUE
                            break 'dispatch;
                        }

                        if pvalue != 0 {
                            *parsed_pattern = (META_ESCAPE as u32)
                                + if posix_negate != 0 { ESC_H } else { ESC_h };
                            parsed_pattern = parsed_pattern.add(1);
                            // goto CLASS_CONTINUE
                            break 'dispatch;
                        }

                        // Fall through
                    }

                    // Non-UCP POSIX class.
                    *parsed_pattern = if posix_negate != 0 {
                        META_POSIX_NEG as u32
                    } else {
                        META_POSIX as u32
                    };
                    parsed_pattern = parsed_pattern.add(1);
                    *parsed_pattern = posix_class as u32;
                    parsed_pattern = parsed_pattern.add(1);
                    // fall through to CLASS_CONTINUE
                    break 'dispatch;
                }
                // Check for the start of a class, or a nested class.
                else if (c == CHAR_LEFT_SQUARE_BRACKET
                    && (class_depth_m1 < 0
                        || class_mode_state == CLASS_MODE_ALT_EXT
                        || class_mode_state == CLASS_MODE_PERL_EXT))
                    || (c == CHAR_LEFT_PARENTHESIS
                        && class_mode_state == CLASS_MODE_PERL_EXT)
                {
                    let start_c = c;
                    let new_class_mode_state: u32;

                    if start_c == CHAR_LEFT_SQUARE_BRACKET
                        && class_mode_state == CLASS_MODE_PERL_EXT
                        && class_depth_m1 >= 0
                    {
                        new_class_mode_state = CLASS_MODE_PERL_EXT_LEAF;
                    } else {
                        new_class_mode_state = class_mode_state;
                    }

                    // -[ beginning a nested class is a literal '-'
                    if class_range_state == RANGE_STARTED {
                        *parsed_pattern.offset(-1) = CHAR_MINUS;
                    }

                    if class_op_state == CLASS_OP_OPERAND
                        && class_mode_state == CLASS_MODE_PERL_EXT
                    {
                        errorcode = ERR113;
                        fail!();
                    }

                    // Validate nesting depth.
                    if class_depth_m1 as i64 >= ECLASS_NEST_LIMIT - 1 {
                        ptr = ptr.sub(1);
                        errorcode = ERR107; // Classes too deeply nested
                        fail!();
                    }

                    // Process the class start. Skip a leading '^' and \Q\E etc.
                    let mut negate_class: BOOL = FALSE;
                    loop {
                        if ptr >= ptrend {
                            errorcode = if start_c == CHAR_LEFT_PARENTHESIS {
                                ERR14
                            } else {
                                ERR6
                            };
                            fail!();
                        }

                        c = GETCHARINCTEST(&mut ptr, utf_b);
                        if new_class_mode_state == CLASS_MODE_PERL_EXT {
                            break;
                        } else if c == CHAR_BACKSLASH {
                            if ptr < ptrend && *ptr as u32 == CHAR_E {
                                ptr = ptr.add(1);
                            } else if ptrend.offset_from(ptr) >= 3
                                && strncmp_c8(ptr, STR_Q_BACKSLASH_E, 3) == 0
                            {
                                ptr = ptr.add(3);
                            } else {
                                break;
                            }
                        } else if (c == CHAR_SPACE || c == CHAR_HT)
                            && ((options & PCRE2_EXTENDED_MORE as u32) != 0
                                || new_class_mode_state >= CLASS_MODE_PERL_EXT)
                        {
                            continue;
                        } else if negate_class == 0 && c == CHAR_CIRCUMFLEX_ACCENT {
                            negate_class = TRUE;
                        } else {
                            break;
                        }
                    }

                    // Empty classes.
                    if c == CHAR_RIGHT_SQUARE_BRACKET
                        && ((*cb).external_options & PCRE2_ALLOW_EMPTY_CLASS as u32) != 0
                        && new_class_mode_state < CLASS_MODE_PERL_EXT
                    {
                        if !class_start.is_null() {
                            *class_start |= CLASS_IS_ECLASS as u32;
                            class_start = ptr::null_mut();
                        }

                        *parsed_pattern = if negate_class != 0 {
                            META_CLASS_EMPTY_NOT as u32
                        } else {
                            META_CLASS_EMPTY as u32
                        };
                        parsed_pattern = parsed_pattern.add(1);

                        if class_depth_m1 < 0 {
                            break 'class_loop;
                        }

                        class_range_state = RANGE_NO;
                        class_op_state = CLASS_OP_OPERAND;
                        // goto CLASS_CONTINUE
                        break 'dispatch;
                    }

                    // Enter a non-empty class.
                    if !class_start.is_null() {
                        *class_start |= CLASS_IS_ECLASS as u32;
                        class_start = ptr::null_mut();
                    }

                    class_start = parsed_pattern;
                    *parsed_pattern = if negate_class != 0 {
                        META_CLASS_NOT as u32
                    } else {
                        META_CLASS as u32
                    };
                    parsed_pattern = parsed_pattern.add(1);
                    class_range_state = RANGE_NO;
                    class_op_state = CLASS_OP_EMPTY;
                    class_mode_state = new_class_mode_state;
                    class_depth_m1 += 1;
                    if class_maxdepth_m1 < class_depth_m1 {
                        class_maxdepth_m1 = class_depth_m1;
                    }
                    (*cb).class_op_used[class_depth_m1 as usize] = 0;

                    // Special start-of-class literal meaning of ']'.
                    if c == CHAR_RIGHT_SQUARE_BRACKET
                        && new_class_mode_state != CLASS_MODE_PERL_EXT
                    {
                        class_range_state = RANGE_OK_LITERAL;
                        class_op_state = CLASS_OP_OPERAND;
                        // PARSED_LITERAL(c, parsed_pattern)
                        *parsed_pattern = c;
                        parsed_pattern = parsed_pattern.add(1);
                        // goto CLASS_CONTINUE
                        break 'dispatch;
                    }

                    // We have already loaded c with the next character.
                    run_class_continue = false;
                    break 'dispatch;
                }
                // Check for the end of the class.
                else if c == CHAR_RIGHT_SQUARE_BRACKET
                    || (c == CHAR_RIGHT_PARENTHESIS
                        && class_mode_state == CLASS_MODE_PERL_EXT)
                {
                    if class_mode_state == CLASS_MODE_PERL_EXT {
                        if c == CHAR_RIGHT_SQUARE_BRACKET && class_depth_m1 != 0 {
                            errorcode = ERR14;
                            ptr = ptr.sub(1);
                            fail!();
                        }
                        if c == CHAR_RIGHT_PARENTHESIS && class_depth_m1 < 1 {
                            errorcode = ERR22;
                            fail!();
                        }
                    }

                    // Check no trailing operator.
                    if class_op_state == CLASS_OP_OPERATOR {
                        errorcode = ERR110;
                        fail!();
                    }

                    // Check no empty expression for Perl extended expressions.
                    if class_mode_state == CLASS_MODE_PERL_EXT
                        && class_op_state == CLASS_OP_EMPTY
                    {
                        errorcode = ERR114;
                        fail!();
                    }

                    // -] at the end of a class is a literal '-'
                    if class_range_state == RANGE_STARTED {
                        *parsed_pattern.offset(-1) = CHAR_MINUS;
                    }

                    *parsed_pattern = META_CLASS_END as u32;
                    parsed_pattern = parsed_pattern.add(1);

                    class_depth_m1 -= 1;
                    if class_depth_m1 < 0 {
                        // Consume ')' after '(?[...]'.
                        if class_mode_state == CLASS_MODE_PERL_EXT {
                            if ptr >= ptrend || *ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                                errorcode = ERR115;
                                fail!();
                            }
                            ptr = ptr.add(1);
                        }
                        break 'class_loop;
                    }

                    class_range_state = RANGE_NO;
                    class_op_state = CLASS_OP_OPERAND;
                    if class_mode_state == CLASS_MODE_PERL_EXT_LEAF {
                        class_mode_state = CLASS_MODE_PERL_EXT;
                    }
                    class_start = ptr::null_mut();
                    // fall through to CLASS_CONTINUE
                    break 'dispatch;
                }
                // Handle a Perl set binary operator.
                else if class_mode_state == CLASS_MODE_PERL_EXT
                    && (c == CHAR_PLUS
                        || c == CHAR_VERTICAL_LINE
                        || c == CHAR_MINUS
                        || c == CHAR_AMPERSAND
                        || c == CHAR_CIRCUMFLEX_ACCENT)
                {
                    if class_op_state != CLASS_OP_OPERAND {
                        errorcode = ERR109;
                        fail!();
                    }

                    if !class_start.is_null() {
                        *class_start |= CLASS_IS_ECLASS as u32;
                        class_start = ptr::null_mut();
                    }

                    *parsed_pattern = if c == CHAR_PLUS {
                        META_ECLASS_OR as u32
                    } else if c == CHAR_VERTICAL_LINE {
                        META_ECLASS_OR as u32
                    } else if c == CHAR_MINUS {
                        META_ECLASS_SUB as u32
                    } else if c == CHAR_AMPERSAND {
                        META_ECLASS_AND as u32
                    } else {
                        META_ECLASS_XOR as u32
                    };
                    parsed_pattern = parsed_pattern.add(1);
                    class_range_state = RANGE_NO;
                    class_op_state = CLASS_OP_OPERATOR;
                    break 'dispatch;
                }
                // Handle a Perl set unary operator.
                else if class_mode_state == CLASS_MODE_PERL_EXT
                    && c == CHAR_EXCLAMATION_MARK
                {
                    if class_op_state == CLASS_OP_OPERAND {
                        errorcode = ERR113;
                        fail!();
                    }

                    if !class_start.is_null() {
                        *class_start |= CLASS_IS_ECLASS as u32;
                        class_start = ptr::null_mut();
                    }

                    *parsed_pattern = META_ECLASS_NOT as u32;
                    parsed_pattern = parsed_pattern.add(1);
                    class_range_state = RANGE_NO;
                    class_op_state = CLASS_OP_OPERATOR;
                    break 'dispatch;
                }
                // Handle a UTS#18 set operator.
                else if class_mode_state == CLASS_MODE_ALT_EXT
                    && (c == CHAR_VERTICAL_LINE
                        || c == CHAR_MINUS
                        || c == CHAR_AMPERSAND
                        || c == CHAR_TILDE)
                    && ptr < ptrend
                    && *ptr as u32 == c
                {
                    ptr = ptr.add(1);

                    // Check there isn't a triple-repetition.
                    if ptr < ptrend && *ptr as u32 == c {
                        while ptr < ptrend && *ptr as u32 == c {
                            ptr = ptr.add(1);
                        }
                        errorcode = ERR108;
                        fail!();
                    }

                    if class_op_state != CLASS_OP_OPERAND {
                        errorcode = ERR109;
                        fail!();
                    }

                    // Check for mixed precedence. Forbid [A--B&&C].
                    if (*cb).class_op_used[class_depth_m1 as usize] != 0
                        && (*cb).class_op_used[class_depth_m1 as usize] != c as u8
                    {
                        errorcode = ERR111;
                        fail!();
                    }

                    if !class_start.is_null() {
                        *class_start |= CLASS_IS_ECLASS as u32;
                        class_start = ptr::null_mut();
                    }

                    // Dangling '-' before an operator is a literal.
                    if class_range_state == RANGE_STARTED {
                        *parsed_pattern.offset(-1) = CHAR_MINUS;
                    }

                    *parsed_pattern = if c == CHAR_VERTICAL_LINE {
                        META_ECLASS_OR as u32
                    } else if c == CHAR_MINUS {
                        META_ECLASS_SUB as u32
                    } else if c == CHAR_AMPERSAND {
                        META_ECLASS_AND as u32
                    } else {
                        META_ECLASS_XOR as u32
                    };
                    parsed_pattern = parsed_pattern.add(1);
                    class_range_state = RANGE_NO;
                    class_op_state = CLASS_OP_OPERATOR;
                    (*cb).class_op_used[class_depth_m1 as usize] = c as u8;
                    break 'dispatch;
                }
                // Handle escapes in a class.
                else if c == CHAR_BACKSLASH {
                    tempptr = ptr;
                    let mut escape = check_escape(
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
                        if (xoptions & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL as u32) == 0
                            || class_mode_state >= CLASS_MODE_PERL_EXT
                        {
                            fail!();
                        }
                        ptr = tempptr;
                        if ptr >= ptrend {
                            c = CHAR_BACKSLASH;
                        } else {
                            c = GETCHARINCTEST(&mut ptr, utf_b);
                        }
                        escape = 0; // Treat as literal character
                    }

                    // switch (escape)
                    if escape == 0 {
                        char_is_literal = FALSE;
                        run_class_literal = true; // goto CLASS_LITERAL
                        break 'dispatch;
                    } else if escape == ESC_b as c_int {
                        c = CHAR_BS; // \b is backspace in a class
                        char_is_literal = FALSE;
                        run_class_literal = true; // goto CLASS_LITERAL
                        break 'dispatch;
                    } else if escape == ESC_k as c_int {
                        c = CHAR_k; // \k is not special in a class
                        char_is_literal = FALSE;
                        run_class_literal = true; // goto CLASS_LITERAL
                        break 'dispatch;
                    } else if escape == ESC_Q as c_int {
                        inescq = TRUE; // Enter literal mode
                        break 'dispatch; // goto CLASS_CONTINUE
                    } else if escape == ESC_E as c_int {
                        break 'dispatch; // Ignore orphan \E, goto CLASS_CONTINUE
                    } else if escape == ESC_B as c_int
                        || escape == ESC_R as c_int
                        || escape == ESC_X as c_int
                    {
                        errorcode = ERR7;
                        fail!();
                    } else if escape == ESC_N as c_int {
                        errorcode = ERR71;
                        fail!();
                    } else if escape == ESC_H as c_int
                        || escape == ESC_h as c_int
                        || escape == ESC_V as c_int
                        || escape == ESC_v as c_int
                    {
                        *parsed_pattern = (META_ESCAPE as u32) + escape as u32;
                        parsed_pattern = parsed_pattern.add(1);
                    } else if escape == ESC_d as c_int
                        || escape == ESC_D as c_int
                        || escape == ESC_s as c_int
                        || escape == ESC_S as c_int
                        || escape == ESC_w as c_int
                        || escape == ESC_W as c_int
                    {
                        parsed_pattern =
                            handle_escdsw(escape, parsed_pattern, options, xoptions);
                    } else if escape == ESC_P as c_int || escape == ESC_p as c_int {
                        // Explicit Unicode property matching.
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
                            fail!();
                        }

                        // Caseless Lu/Ll/Lt -> L&.
                        if (options & PCRE2_CASELESS as u32) != 0
                            && ptype as i64 == PT_PC
                            && (pdata as u32 == ucp_Lu
                                || pdata as u32 == ucp_Ll
                                || pdata as u32 == ucp_Lt)
                        {
                            ptype = PT_LAMP as u16;
                            pdata = 0;
                        }

                        if negated != 0 {
                            escape = if escape == ESC_P as c_int {
                                ESC_p as c_int
                            } else {
                                ESC_P as c_int
                            };
                        }
                        *parsed_pattern = (META_ESCAPE as u32) + escape as u32;
                        parsed_pattern = parsed_pattern.add(1);
                        *parsed_pattern = ((ptype as u32) << 16) | pdata as u32;
                        parsed_pattern = parsed_pattern.add(1);
                    } else {
                        // default + ESC_A ESC_Z ESC_z ESC_G ESC_K ESC_C
                        errorcode = ERR7;
                        fail!();
                    }

                    // The "break" switch-cases describe a set of characters;
                    // none may start a range.
                    if class_range_state == RANGE_STARTED {
                        errorcode = ERR50;
                        fail!();
                    }

                    if class_range_state == RANGE_FORBID_STARTED {
                        ptr = class_range_forbid_ptr;
                        errorcode = ERR50;
                        fail!();
                    }

                    if class_op_state == CLASS_OP_OPERAND
                        && class_mode_state == CLASS_MODE_PERL_EXT
                    {
                        errorcode = ERR113;
                        fail!();
                    }

                    class_range_state = RANGE_FORBID_NO;
                    class_op_state = CLASS_OP_OPERAND;
                    break 'dispatch;
                }
                // Forbid unescaped literals / '-' in a Perl extended class.
                else if class_mode_state == CLASS_MODE_PERL_EXT {
                    errorcode = ERR116;
                    fail!();
                }
                // Handle potential start of range.
                else if c == CHAR_MINUS && class_range_state >= RANGE_OK_ESCAPED {
                    *parsed_pattern = if class_range_state == RANGE_OK_LITERAL {
                        META_RANGE_LITERAL as u32
                    } else {
                        META_RANGE_ESCAPED as u32
                    };
                    parsed_pattern = parsed_pattern.add(1);
                    class_range_state = RANGE_STARTED;
                    break 'dispatch;
                }
                // Handle forbidden start of range.
                else if c == CHAR_MINUS && class_range_state == RANGE_FORBID_NO {
                    *parsed_pattern = CHAR_MINUS;
                    parsed_pattern = parsed_pattern.add(1);
                    class_range_state = RANGE_FORBID_STARTED;
                    class_range_forbid_ptr = ptr;
                    break 'dispatch;
                }
                // Handle a literal character (else / CLASS_LITERAL).
                else {
                    run_class_literal = true;
                    break 'dispatch;
                }
            } // 'dispatch

            // CLASS_LITERAL body.
            if run_class_literal {
                if class_op_state == CLASS_OP_OPERAND
                    && class_mode_state == CLASS_MODE_PERL_EXT
                {
                    errorcode = ERR113;
                    fail!();
                }

                if class_range_state == RANGE_STARTED {
                    if c == *parsed_pattern.offset(-2) {
                        // Optimize one-char range
                        parsed_pattern = parsed_pattern.sub(1);
                    } else if *parsed_pattern.offset(-2) > c {
                        // Check range is in order
                        errorcode = ERR8;
                        fail!();
                    } else {
                        if char_is_literal == 0
                            && *parsed_pattern.offset(-1) == META_RANGE_LITERAL as u32
                        {
                            *parsed_pattern.offset(-1) = META_RANGE_ESCAPED as u32;
                        }
                        // PARSED_LITERAL(c, parsed_pattern)
                        *parsed_pattern = c;
                        parsed_pattern = parsed_pattern.add(1);
                    }
                    class_range_state = RANGE_NO;
                    class_op_state = CLASS_OP_OPERAND;
                } else if class_range_state == RANGE_FORBID_STARTED {
                    ptr = class_range_forbid_ptr;
                    errorcode = ERR50;
                    fail!();
                } else {
                    // Potential start of range
                    class_range_state = if char_is_literal != 0 {
                        RANGE_OK_LITERAL
                    } else {
                        RANGE_OK_ESCAPED
                    };
                    class_op_state = CLASS_OP_OPERAND;
                    // PARSED_LITERAL(c, parsed_pattern)
                    *parsed_pattern = c;
                    parsed_pattern = parsed_pattern.add(1);
                }
            }

            // CLASS_CONTINUE.
            if run_class_continue {
                if ptr >= ptrend {
                    if class_mode_state == CLASS_MODE_PERL_EXT && class_depth_m1 > 0 {
                        errorcode = ERR14; // Missing terminating ')'
                    }
                    if class_mode_state == CLASS_MODE_ALT_EXT
                        && class_depth_m1 == 0
                        && class_maxdepth_m1 == 1
                    {
                        errorcode = ERR112; // saw '[ [ ]...'
                    } else {
                        errorcode = ERR6; // Missing terminating ']'
                    }
                    fail!();
                }
                c = GETCHARINCTEST(&mut ptr, utf_b);
            }
            // Loop back for next thing in the class.
        } // 'class_loop

        // End of character class.
        sync_out!();
        ClassResult::Ok
    }
}

// ---------------------------------------------------------------------------
// CHAR_LEFT_PARENTHESIS — opening parenthesis handling, including the (?
// sub-switch. Cross-case jumps to assertion/recursion/define labels are
// returned as OpenParen::Jump(..) for the main dispatch loop to service.
// ---------------------------------------------------------------------------
enum OpenParen {
    Break,
    NextChar,
    Failed,
    Jump(Jump),
}

unsafe fn parse_open_paren(
    ptr_ref: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    c_ref: *mut u32,
    options_ref: *mut u32,
    xoptions_ref: *mut u32,
    parsed_pattern_ref: *mut *mut u32,
    previous_callout_ref: *mut *mut u32,
    verbstartptr_ref: *mut *mut u32,
    verblengthptr_ref: *mut *mut u32,
    verbnamestart_ref: *mut PCRE2_SPTR,
    inverbname_ref: *mut BOOL,
    add_after_mark_ref: *mut u32,
    okquantifier_ref: *mut BOOL,
    expect_cond_assert_ref: *mut c_int,
    prev_expect_cond_assert: c_int,
    after_manual_callout_ref: *mut c_int,
    nest_depth_ref: *mut u16,
    top_nest_ref: *mut *mut nest_save,
    end_nests: *mut nest_save,
    class_mode_state_ref: *mut u32,
    has_lookbehind: *mut BOOL,
    offset_ref: *mut PCRE2_SIZE,
    terminator_ref: *mut u32,
    name_ref: *mut PCRE2_SPTR,
    namelen_ref: *mut u32,
    i_ref: *mut c_int,
    errorcode_ref: *mut c_int,
    utf: BOOL,
    cb: *mut compile_block,
) -> OpenParen {
    unsafe {
        let utf_b = utf != 0;
        let mut ptr = *ptr_ref;
        let mut c = *c_ref;
        let mut options = *options_ref;
        let mut xoptions = *xoptions_ref;
        let mut parsed_pattern = *parsed_pattern_ref;
        let mut nest_depth = *nest_depth_ref;
        let mut top_nest = *top_nest_ref;
        let mut errorcode = *errorcode_ref;
        let mut offset: PCRE2_SIZE = *offset_ref;
        let mut terminator: u32 = *terminator_ref;
        let mut name: PCRE2_SPTR = *name_ref;
        let mut namelen: u32 = *namelen_ref;
        let mut i: c_int = *i_ref;

        macro_rules! sync_out {
            () => {{
                *ptr_ref = ptr;
                *c_ref = c;
                *options_ref = options;
                *xoptions_ref = xoptions;
                *parsed_pattern_ref = parsed_pattern;
                *nest_depth_ref = nest_depth;
                *top_nest_ref = top_nest;
                *errorcode_ref = errorcode;
                *offset_ref = offset;
                *terminator_ref = terminator;
                *name_ref = name;
                *namelen_ref = namelen;
                *i_ref = i;
            }};
        }
        macro_rules! failed {
            () => {{
                sync_out!();
                return OpenParen::Failed;
            }};
        }
        macro_rules! unclosed_parenthesis {
            () => {{
                errorcode = ERR14;
                failed!();
            }};
        }
        macro_rules! failed_forward {
            () => {{
                ptr = ptr.add(1);
                if utf_b {
                    FORWARDCHARTEST(&mut ptr, ptrend);
                }
                failed!();
            }};
        }
        macro_rules! ret_jump {
            ($j:expr) => {{
                sync_out!();
                return OpenParen::Jump($j);
            }};
        }
        macro_rules! ret_break {
            () => {{
                sync_out!();
                return OpenParen::Break;
            }};
        }

        if ptr >= ptrend {
            unclosed_parenthesis!();
        }

        // If ( is not followed by ? it is a capture, verb, alpha assertion, or
        // positive non-atomic lookahead.
        if *ptr as u32 != CHAR_QUESTION_MARK {
            // Handle capturing brackets (or non-capturing if disabled).
            if *ptr as u32 != CHAR_ASTERISK {
                nest_depth += 1;
                if (options & PCRE2_NO_AUTO_CAPTURE as u32) == 0 {
                    if (*cb).bracount >= MAX_GROUP_NUMBER {
                        errorcode = ERR97;
                        failed!();
                    }
                    (*cb).bracount += 1;
                    *parsed_pattern = (META_CAPTURE as u32) | (*cb).bracount;
                    parsed_pattern = parsed_pattern.add(1);
                } else {
                    *parsed_pattern = META_NOCAPTURE as u32;
                    parsed_pattern = parsed_pattern.add(1);
                }
            }
            // (* followed by end or ) -> "bad quantifier".
            else if ptrend.offset_from(ptr) <= 1 || {
                c = *ptr.add(1) as u32;
                c == CHAR_RIGHT_PARENTHESIS
            } {
                ret_break!();
            }
            // Alpha assertions such as (*pla:...).
            else if CHMAX_255(c) && (*(*cb).ctypes.add(c as usize) as u32 & ctype_lcletter as u32) != 0
            {
                let mut vn_off: usize; // offset into ALASNAMES
                let meta: u32;

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
                    failed!();
                }
                if ptr >= ptrend {
                    unclosed_parenthesis!();
                }
                if *ptr as u32 != CHAR_COLON {
                    errorcode = ERR95; // Malformed
                    failed_forward!();
                }

                // Scan the table of alpha assertion names.
                vn_off = 0;
                i = 0;
                while i < ALASCOUNT {
                    if namelen == ALASMETA[i as usize].len as u32
                        && strncmp_c8(name, &ALASNAMES[vn_off..], namelen as usize) == 0
                    {
                        break;
                    }
                    vn_off += ALASMETA[i as usize].len as usize + 1;
                    i += 1;
                }

                if i >= ALASCOUNT {
                    errorcode = ERR95; // Alpha assertion not recognized
                    failed!();
                }

                // Check for expecting an assertion condition.
                meta = ALASMETA[i as usize].meta;
                if prev_expect_cond_assert > 0
                    && (meta < META_LOOKAHEAD as u32 || meta > META_LOOKBEHINDNOT as u32)
                {
                    errorcode = ERR28; // Atomic assertion expected
                    failed!();
                }

                // Dispatch based on the resolved meta value.
                if meta == META_ATOMIC as u32 {
                    ret_jump!(Jump::AtomicGroup);
                } else if meta == META_LOOKAHEAD as u32 {
                    ret_jump!(Jump::PositiveLookAhead);
                } else if meta == META_LOOKAHEAD_NA as u32 {
                    ret_jump!(Jump::PositiveNonatomicLookAhead);
                } else if meta == META_LOOKAHEADNOT as u32 {
                    ret_jump!(Jump::NegativeLookAhead);
                } else if meta == META_SCS as u32 {
                    ptr = ptr.add(1);
                    *parsed_pattern = META_SCS as u32;
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
                    if parsed_pattern.is_null() {
                        // parsed_pattern is null: cannot sync it; fail directly.
                        *ptr_ref = ptr;
                        *errorcode_ref = errorcode;
                        return OpenParen::Failed;
                    }
                    ret_jump!(Jump::PostAssertion);
                } else if meta == META_LOOKBEHIND as u32
                    || meta == META_LOOKBEHINDNOT as u32
                    || meta == META_LOOKBEHIND_NA as u32
                {
                    *parsed_pattern = meta;
                    parsed_pattern = parsed_pattern.add(1);
                    ptr = ptr.sub(1);
                    ret_jump!(Jump::PostLookbehind);
                } else if meta == META_SCRIPT_RUN as u32 || meta == META_ATOMIC_SCRIPT_RUN as u32
                {
                    // Script run facilities (Unicode is available).
                    *parsed_pattern = META_SCRIPT_RUN as u32;
                    parsed_pattern = parsed_pattern.add(1);
                    nest_depth += 1;
                    ptr = ptr.add(1);
                    if meta == META_ATOMIC_SCRIPT_RUN as u32 {
                        *parsed_pattern = META_ATOMIC as u32;
                        parsed_pattern = parsed_pattern.add(1);
                        if top_nest.is_null() {
                            top_nest = (*cb).start_workspace as *mut nest_save;
                        } else {
                            top_nest = top_nest.add(1);
                            if top_nest >= end_nests {
                                errorcode = ERR84;
                                failed!();
                            }
                        }
                        (*top_nest).nest_depth = nest_depth;
                        (*top_nest).flags = NSF_ATOMICSR as u16;
                        (*top_nest).options = options & PARSE_TRACKED_OPTIONS;
                        (*top_nest).xoptions = xoptions & PARSE_TRACKED_EXTRA_OPTIONS;
                    }
                    ret_break!();
                } else {
                    // LCOV_EXCL: unknown code.
                    errorcode = ERR89;
                    failed!();
                }
            }
            // ---- Handle (*VERB) and (*VERB:NAME) ----
            else {
                let mut vn_off: usize; // offset into VERBNAMES
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
                    failed!();
                }
                if ptr >= ptrend
                    || (*ptr as u32 != CHAR_COLON && *ptr as u32 != CHAR_RIGHT_PARENTHESIS)
                {
                    errorcode = ERR60; // Malformed
                    failed!();
                }

                // Scan the table of verb names.
                vn_off = 0;
                i = 0;
                while i < VERBCOUNT {
                    if namelen == VERBS[i as usize].len as u32
                        && strncmp_c8(name, &VERBNAMES[vn_off..], namelen as usize) == 0
                    {
                        break;
                    }
                    vn_off += VERBS[i as usize].len as usize + 1;
                    i += 1;
                }

                if i >= VERBCOUNT {
                    errorcode = ERR60; // Verb not recognized
                    failed!();
                }

                // An empty argument is treated as no argument.
                if *ptr as u32 == CHAR_COLON
                    && ptr.add(1) < ptrend
                    && *ptr.add(1) as u32 == CHAR_RIGHT_PARENTHESIS
                {
                    ptr = ptr.add(1); // Advance to the closing parens
                }

                // Check for mandatory non-empty argument; this is (*MARK).
                if VERBS[i as usize].has_arg > 0 && *ptr as u32 != CHAR_COLON {
                    errorcode = ERR66;
                    failed!();
                }

                // Remember where this verb starts.
                *verbstartptr_ref = parsed_pattern;
                *okquantifier_ref = (VERBS[i as usize].meta == META_ACCEPT as u32) as BOOL;

                // inverbname handling below.
                let ptr_was_colon = *ptr as u32 == CHAR_COLON;
                ptr = ptr.add(1); // Skip past : or )
                if ptr_was_colon {
                    // Some optional arguments treated as a preceding (*MARK).
                    if VERBS[i as usize].has_arg < 0 {
                        *add_after_mark_ref = VERBS[i as usize].meta;
                        *parsed_pattern = META_MARK as u32;
                        parsed_pattern = parsed_pattern.add(1);
                    } else {
                        // Other verbs with arguments need a different opcode.
                        *parsed_pattern = VERBS[i as usize].meta
                            + (if VERBS[i as usize].meta != META_MARK as u32 {
                                0x00010000u32
                            } else {
                                0
                            });
                        parsed_pattern = parsed_pattern.add(1);
                    }

                    // Set up for reading the name in the main loop.
                    *verblengthptr_ref = parsed_pattern;
                    parsed_pattern = parsed_pattern.add(1);
                    *verbnamestart_ref = ptr;
                    *inverbname_ref = TRUE;
                } else {
                    // No verb "name" argument.
                    *parsed_pattern = VERBS[i as usize].meta;
                    parsed_pattern = parsed_pattern.add(1);
                }
            } // End of (*VERB) handling
            ret_break!(); // Done with this parenthesis
        } // End of groups that don't start with (?

        // ---- Items starting (? ----
        ptr = ptr.add(1);
        if ptr >= ptrend {
            unclosed_parenthesis!();
        }

        // Sync locals back to the shared refs, then delegate to the (?
        // sub-switch handler, which operates directly on the refs.
        sync_out!();
        parse_open_paren_question(
            ptr_ref,
            ptrend,
            c_ref,
            options_ref,
            xoptions_ref,
            parsed_pattern_ref,
            previous_callout_ref,
            after_manual_callout_ref,
            nest_depth_ref,
            top_nest_ref,
            end_nests,
            class_mode_state_ref,
            has_lookbehind,
            offset_ref,
            terminator_ref,
            name_ref,
            namelen_ref,
            i_ref,
            errorcode_ref,
            expect_cond_assert_ref,
            prev_expect_cond_assert,
            okquantifier_ref,
            utf,
            cb,
        )
    }
}

// ---------------------------------------------------------------------------
// The (? sub-switch. On entry `ptr` points at the character after "(?".
// ---------------------------------------------------------------------------
unsafe fn parse_open_paren_question(
    ptr_ref: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    c_ref: *mut u32,
    options_ref: *mut u32,
    xoptions_ref: *mut u32,
    parsed_pattern_ref: *mut *mut u32,
    previous_callout_ref: *mut *mut u32,
    after_manual_callout_ref: *mut c_int,
    nest_depth_ref: *mut u16,
    top_nest_ref: *mut *mut nest_save,
    end_nests: *mut nest_save,
    _class_mode_state_ref: *mut u32,
    has_lookbehind: *mut BOOL,
    offset_ref: *mut PCRE2_SIZE,
    terminator_ref: *mut u32,
    name_ref: *mut PCRE2_SPTR,
    namelen_ref: *mut u32,
    i_ref: *mut c_int,
    errorcode_ref: *mut c_int,
    expect_cond_assert_ref: *mut c_int,
    _prev_expect_cond_assert: c_int,
    okquantifier_ref: *mut BOOL,
    utf: BOOL,
    cb: *mut compile_block,
) -> OpenParen {
    unsafe {
        let mut ptr = *ptr_ref;
        let mut options = *options_ref;
        let mut xoptions = *xoptions_ref;
        let mut parsed_pattern = *parsed_pattern_ref;
        let mut nest_depth = *nest_depth_ref;
        let mut top_nest = *top_nest_ref;
        let mut errorcode = *errorcode_ref;
        let mut offset: PCRE2_SIZE = *offset_ref;
        let mut terminator: u32 = *terminator_ref;
        let mut name: PCRE2_SPTR = *name_ref;
        let mut namelen: u32 = *namelen_ref;
        let mut i: c_int = *i_ref;
        let utf_b = utf != 0;

        macro_rules! sync_out {
            () => {{
                *ptr_ref = ptr;
                *options_ref = options;
                *xoptions_ref = xoptions;
                *parsed_pattern_ref = parsed_pattern;
                *nest_depth_ref = nest_depth;
                *top_nest_ref = top_nest;
                *errorcode_ref = errorcode;
                *offset_ref = offset;
                *terminator_ref = terminator;
                *name_ref = name;
                *namelen_ref = namelen;
                *i_ref = i;
            }};
        }
        macro_rules! failed {
            () => {{
                sync_out!();
                return OpenParen::Failed;
            }};
        }
        macro_rules! unclosed_parenthesis {
            () => {{
                errorcode = ERR14;
                failed!();
            }};
        }
        macro_rules! failed_forward {
            () => {{
                ptr = ptr.add(1);
                if utf_b {
                    FORWARDCHARTEST(&mut ptr, ptrend);
                }
                failed!();
            }};
        }
        macro_rules! ret_break {
            () => {{
                sync_out!();
                return OpenParen::Break;
            }};
        }
        macro_rules! ret_jump {
            ($j:expr) => {{
                sync_out!();
                return OpenParen::Jump($j);
            }};
        }

        // Local labels reachable within this (? handling are implemented via a
        // dispatch loop over QJump.
        #[derive(Clone, Copy, PartialEq)]
        enum QJump {
            Switch,
            RecursionByNumber,
            SetRecursion,
            RecurseByName,
            ReadRecursionArguments,
            DefineName,
        }

        let mut qjump = QJump::Switch;
        'qdispatch: loop {
            match qjump {
                QJump::Switch => {
                    let ch = *ptr as u32;
                    // The C switch(*ptr) with a big default. We special-case the
                    // known first characters; everything else is "default".
                    if ch == CHAR_P {
                        // ---- Python syntax support ----
                        ptr = ptr.add(1);
                        if ptr >= ptrend {
                            unclosed_parenthesis!();
                        }

                        if *ptr as u32 == CHAR_LESS_THAN_SIGN {
                            terminator = CHAR_GREATER_THAN_SIGN;
                            qjump = QJump::DefineName;
                            continue 'qdispatch;
                        }

                        if *ptr as u32 == CHAR_GREATER_THAN_SIGN {
                            qjump = QJump::RecurseByName;
                            continue 'qdispatch;
                        }

                        if *ptr as u32 != CHAR_EQUALS_SIGN {
                            errorcode = ERR41;
                            failed_forward!();
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
                            failed!();
                        }
                        *parsed_pattern = META_BACKREF_BYNAME as u32;
                        parsed_pattern = parsed_pattern.add(1);
                        *parsed_pattern = namelen;
                        parsed_pattern = parsed_pattern.add(1);
                        PUTOFFSET(offset, &mut parsed_pattern);
                        *okquantifier_ref = TRUE;
                        ret_break!();
                    } else if ch == CHAR_R {
                        // ---- Recursion/subroutine calls by number ----
                        i = 0; // (?R) == (?R0)
                        ptr = ptr.add(1);
                        if ptr >= ptrend
                            || (*ptr as u32 != CHAR_RIGHT_PARENTHESIS
                                && *ptr as u32 != CHAR_LEFT_PARENTHESIS)
                        {
                            errorcode = ERR58;
                            failed!();
                        }
                        terminator = CHAR_NUL;
                        qjump = QJump::SetRecursion;
                        continue 'qdispatch;
                    } else if ch == CHAR_PLUS {
                        if ptr.add(1) >= ptrend {
                            ptr = ptr.add(1);
                            unclosed_parenthesis!();
                        }
                        if !IS_DIGIT(*ptr.add(1) as u32) {
                            errorcode = ERR29; // Missing number
                            ptr = ptr.add(1);
                            failed_forward!();
                        }
                        // Fall through to RECURSION_BYNUMBER
                        qjump = QJump::RecursionByNumber;
                        continue 'qdispatch;
                    } else if IS_DIGIT(ch) {
                        // case CHAR_0..CHAR_9
                        qjump = QJump::RecursionByNumber;
                        continue 'qdispatch;
                    } else if ch == CHAR_AMPERSAND {
                        qjump = QJump::RecurseByName;
                        continue 'qdispatch;
                    } else if ch == CHAR_C {
                        // ---- Callout ----
                        match parse_callout(
                            &mut ptr,
                            ptrend,
                            &mut parsed_pattern,
                            previous_callout_ref,
                            after_manual_callout_ref,
                            expect_cond_assert_ref,
                            _prev_expect_cond_assert,
                            &mut offset,
                            &mut i,
                            &mut errorcode,
                            options,
                            xoptions,
                            utf,
                            cb,
                        ) {
                            CalloutR::Break => ret_break!(),
                            CalloutR::Failed => {
                                sync_out!();
                                return OpenParen::Failed;
                            }
                            CalloutR::FailedForward => {
                                failed_forward!();
                            }
                        }
                    } else if ch == CHAR_LEFT_PARENTHESIS {
                        // ---- Conditional group ----
                        match parse_conditional(
                            &mut ptr,
                            ptrend,
                            &mut parsed_pattern,
                            &mut nest_depth,
                            &mut offset,
                            &mut terminator,
                            &mut name,
                            &mut namelen,
                            &mut i,
                            expect_cond_assert_ref,
                            &mut errorcode,
                            utf,
                            cb,
                        ) {
                            CondR::Break => ret_break!(),
                            CondR::Failed => {
                                sync_out!();
                                return OpenParen::Failed;
                            }
                            CondR::FailedForward => {
                                failed_forward!();
                            }
                        }
                    } else if ch == CHAR_GREATER_THAN_SIGN {
                        // ---- Atomic group ----
                        ret_jump!(Jump::AtomicGroup);
                    } else if ch == CHAR_EQUALS_SIGN {
                        ret_jump!(Jump::PositiveLookAhead);
                    } else if ch == CHAR_ASTERISK {
                        ret_jump!(Jump::PositiveNonatomicLookAhead);
                    } else if ch == CHAR_EXCLAMATION_MARK {
                        ret_jump!(Jump::NegativeLookAhead);
                    } else if ch == CHAR_LESS_THAN_SIGN {
                        // (?< : lookbehind or start of a group name.
                        if ptrend.offset_from(ptr) <= 1
                            || (*ptr.add(1) as u32 != CHAR_EQUALS_SIGN
                                && *ptr.add(1) as u32 != CHAR_EXCLAMATION_MARK
                                && *ptr.add(1) as u32 != CHAR_ASTERISK)
                        {
                            terminator = CHAR_GREATER_THAN_SIGN;
                            qjump = QJump::DefineName;
                            continue 'qdispatch;
                        }
                        *parsed_pattern = if *ptr.add(1) as u32 == CHAR_EQUALS_SIGN {
                            META_LOOKBEHIND as u32
                        } else if *ptr.add(1) as u32 == CHAR_EXCLAMATION_MARK {
                            META_LOOKBEHINDNOT as u32
                        } else {
                            META_LOOKBEHIND_NA as u32
                        };
                        parsed_pattern = parsed_pattern.add(1);
                        // POST_LOOKBEHIND
                        *has_lookbehind = TRUE;
                        offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE - 2;
                        PUTOFFSET(offset, &mut parsed_pattern);
                        ptr = ptr.add(2);
                        // Fall through to POST_ASSERTION (handled by main dispatch).
                        ret_jump!(Jump::PostAssertion);
                    } else if ch == CHAR_APOSTROPHE {
                        terminator = CHAR_APOSTROPHE;
                        qjump = QJump::DefineName;
                        continue 'qdispatch;
                    } else if ch == CHAR_LEFT_SQUARE_BRACKET {
                        // ---- Perl extended character class '(?[...])' ----
                        *_class_mode_state_ref = CLASS_MODE_PERL_EXT;
                        c_set(c_ref, *ptr as u32);
                        ptr = ptr.add(1);
                        sync_out!();
                        return OpenParen::Jump(Jump::FromPerlExtendedClass);
                    } else {
                        // ---- default: (?| or option setting ----
                        match parse_options_group(
                            &mut ptr,
                            ptrend,
                            &mut options,
                            &mut xoptions,
                            &mut parsed_pattern,
                            &mut nest_depth,
                            &mut top_nest,
                            end_nests,
                            &mut errorcode,
                            cb,
                        ) {
                            OptR::Break => ret_break!(),
                            OptR::Failed => {
                                sync_out!();
                                return OpenParen::Failed;
                            }
                            OptR::RecursionByNumber => {
                                qjump = QJump::RecursionByNumber;
                                continue 'qdispatch;
                            }
                        }
                    }
                }

                // ------------------------------------------------------
                QJump::RecursionByNumber => {
                    if read_number(
                        &mut ptr,
                        ptrend,
                        if IS_DIGIT(*ptr as u32) {
                            -1
                        } else {
                            (*cb).bracount as i32
                        },
                        MAX_GROUP_NUMBER,
                        ERR61 as u32,
                        &mut i,
                        &mut errorcode,
                    ) == 0
                    {
                        failed!();
                    }
                    // PCRE2_ASSERT(i >= 0);
                    terminator = CHAR_NUL;
                    qjump = QJump::SetRecursion;
                    continue 'qdispatch;
                }

                QJump::SetRecursion => {
                    *parsed_pattern = (META_RECURSE as u32) | (i as u32);
                    parsed_pattern = parsed_pattern.add(1);
                    offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                    qjump = QJump::ReadRecursionArguments;
                    continue 'qdispatch;
                }

                QJump::RecurseByName => {
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
                        failed!();
                    }
                    *parsed_pattern = META_RECURSE_BYNAME as u32;
                    parsed_pattern = parsed_pattern.add(1);
                    *parsed_pattern = namelen;
                    parsed_pattern = parsed_pattern.add(1);
                    terminator = CHAR_NUL;
                    qjump = QJump::ReadRecursionArguments;
                    continue 'qdispatch;
                }

                QJump::ReadRecursionArguments => {
                    PUTOFFSET(offset, &mut parsed_pattern);
                    *okquantifier_ref = TRUE;

                    if terminator != CHAR_NUL {
                        ret_break!();
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
                            *ptr_ref = ptr;
                            *errorcode_ref = errorcode;
                            return OpenParen::Failed;
                        }
                    }

                    if ptr >= ptrend || *ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                        unclosed_parenthesis!();
                    }
                    ptr = ptr.add(1);
                    ret_break!();
                }

                QJump::DefineName => {
                    // Delegate to the shared DEFINE_NAME handler via main dispatch.
                    ret_jump!(Jump::DefineName);
                }
            }
        }
    }
}

/// Set `*c_ref = v` (helper to avoid borrow gymnastics in the caller).
#[inline(always)]
unsafe fn c_set(c_ref: *mut u32, v: u32) {
    unsafe {
        *c_ref = v;
    }
}

// ---------------------------------------------------------------------------
// The "default" case of the (? switch: (?| or an option setting, optionally
// followed by a non-capturing group.
// ---------------------------------------------------------------------------
enum OptR {
    Break,
    Failed,
    RecursionByNumber,
}

unsafe fn parse_options_group(
    ptr_ref: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    options_ref: *mut u32,
    xoptions_ref: *mut u32,
    parsed_pattern_ref: *mut *mut u32,
    nest_depth_ref: *mut u16,
    top_nest_ref: *mut *mut nest_save,
    end_nests: *mut nest_save,
    errorcode_ref: *mut c_int,
    cb: *mut compile_block,
) -> OptR {
    unsafe {
        let mut ptr = *ptr_ref;
        let mut options = *options_ref;
        let mut xoptions = *xoptions_ref;
        let mut parsed_pattern = *parsed_pattern_ref;
        let mut nest_depth = *nest_depth_ref;
        let mut top_nest = *top_nest_ref;
        let mut errorcode = *errorcode_ref;

        macro_rules! sync_out {
            () => {{
                *ptr_ref = ptr;
                *options_ref = options;
                *xoptions_ref = xoptions;
                *parsed_pattern_ref = parsed_pattern;
                *nest_depth_ref = nest_depth;
                *top_nest_ref = top_nest;
                *errorcode_ref = errorcode;
            }};
        }
        macro_rules! failed {
            () => {{
                sync_out!();
                return OptR::Failed;
            }};
        }
        macro_rules! unclosed_parenthesis {
            () => {{
                errorcode = ERR14;
                failed!();
            }};
        }

        // (?- followed by a digit is a relative recursion (handled by caller).
        if *ptr as u32 == CHAR_MINUS && ptrend.offset_from(ptr) > 1 && IS_DIGIT(*ptr.add(1) as u32)
        {
            sync_out!();
            return OptR::RecursionByNumber;
        }

        // We now have either (?| or a (possibly empty) option setting.
        nest_depth += 1;
        if top_nest.is_null() {
            top_nest = (*cb).start_workspace as *mut nest_save;
        } else {
            top_nest = top_nest.add(1);
            if top_nest >= end_nests {
                errorcode = ERR84;
                failed!();
            }
        }
        (*top_nest).nest_depth = nest_depth;
        (*top_nest).flags = 0;
        (*top_nest).options = options & PARSE_TRACKED_OPTIONS;
        (*top_nest).xoptions = xoptions & PARSE_TRACKED_EXTRA_OPTIONS;

        // Non-capturing group that resets the capture count for each branch.
        if *ptr as u32 == CHAR_VERTICAL_LINE {
            (*top_nest).reset_group = (*cb).bracount as u16;
            (*top_nest).max_group = (*cb).bracount as u16;
            (*top_nest).flags |= NSF_RESET as u16;
            (*cb).external_flags |= PCRE2_DUPCAPUSED as u32;
            *parsed_pattern = META_NOCAPTURE as u32;
            parsed_pattern = parsed_pattern.add(1);
            ptr = ptr.add(1);
        }
        // Scan for options imnrsxJU to be set or unset.
        else {
            let mut hyphenok: BOOL = TRUE;
            let oldoptions = options;
            let oldxoptions = xoptions;

            (*top_nest).reset_group = 0;
            (*top_nest).max_group = 0;
            let mut set: u32 = 0;
            let mut unset: u32 = 0;
            // optset selects between `set` and `unset`.
            let mut optset_is_set = true;
            let mut xset: u32 = 0;
            let mut xunset: u32 = 0;
            let mut xoptset_is_set = true;

            // ^ at the start unsets irmnsx and disables subsequent use of -.
            if ptr < ptrend && *ptr as u32 == CHAR_CIRCUMFLEX_ACCENT {
                options &= !(PCRE2_CASELESS as u32
                    | PCRE2_MULTILINE as u32
                    | PCRE2_NO_AUTO_CAPTURE as u32
                    | PCRE2_DOTALL as u32
                    | PCRE2_EXTENDED as u32
                    | PCRE2_EXTENDED_MORE as u32);
                xoptions &= !(PCRE2_EXTRA_CASELESS_RESTRICT as u32);
                hyphenok = FALSE;
                ptr = ptr.add(1);
            }

            while ptr < ptrend
                && *ptr as u32 != CHAR_RIGHT_PARENTHESIS
                && *ptr as u32 != CHAR_COLON
            {
                let ch = *ptr as u32;
                ptr = ptr.add(1);
                macro_rules! optset {
                    () => {
                        if optset_is_set { &mut set } else { &mut unset }
                    };
                }
                macro_rules! xoptset {
                    () => {
                        if xoptset_is_set { &mut xset } else { &mut xunset }
                    };
                }
                match ch {
                    CHAR_MINUS => {
                        if hyphenok == 0 {
                            errorcode = ERR94;
                            failed!();
                        }
                        optset_is_set = false;
                        xoptset_is_set = false;
                        hyphenok = FALSE;
                    }
                    CHAR_a => {
                        // Two-character sequences starting with 'a'.
                        if ptr < ptrend {
                            if *ptr as u32 == CHAR_D {
                                *xoptset!() |= PCRE2_EXTRA_ASCII_BSD as u32;
                                ptr = ptr.add(1);
                                continue;
                            }
                            if *ptr as u32 == CHAR_P {
                                *xoptset!() |= (PCRE2_EXTRA_ASCII_POSIX
                                    | PCRE2_EXTRA_ASCII_DIGIT)
                                    as u32;
                                ptr = ptr.add(1);
                                continue;
                            }
                            if *ptr as u32 == CHAR_S {
                                *xoptset!() |= PCRE2_EXTRA_ASCII_BSS as u32;
                                ptr = ptr.add(1);
                                continue;
                            }
                            if *ptr as u32 == CHAR_T {
                                *xoptset!() |= PCRE2_EXTRA_ASCII_DIGIT as u32;
                                ptr = ptr.add(1);
                                continue;
                            }
                            if *ptr as u32 == CHAR_W {
                                *xoptset!() |= PCRE2_EXTRA_ASCII_BSW as u32;
                                ptr = ptr.add(1);
                                continue;
                            }
                        }
                        *xoptset!() |= (PCRE2_EXTRA_ASCII_BSD
                            | PCRE2_EXTRA_ASCII_BSS
                            | PCRE2_EXTRA_ASCII_BSW
                            | PCRE2_EXTRA_ASCII_DIGIT
                            | PCRE2_EXTRA_ASCII_POSIX)
                            as u32;
                    }
                    CHAR_J => {
                        *optset!() |= PCRE2_DUPNAMES as u32;
                        (*cb).external_flags |= PCRE2_JCHANGED as u32;
                    }
                    CHAR_i => *optset!() |= PCRE2_CASELESS as u32,
                    CHAR_m => *optset!() |= PCRE2_MULTILINE as u32,
                    CHAR_n => *optset!() |= PCRE2_NO_AUTO_CAPTURE as u32,
                    CHAR_r => *xoptset!() |= PCRE2_EXTRA_CASELESS_RESTRICT as u32,
                    CHAR_s => *optset!() |= PCRE2_DOTALL as u32,
                    CHAR_U => *optset!() |= PCRE2_UNGREEDY as u32,
                    CHAR_x => {
                        *optset!() |= PCRE2_EXTENDED as u32;
                        if ptr < ptrend && *ptr as u32 == CHAR_x {
                            *optset!() |= PCRE2_EXTENDED_MORE as u32;
                            ptr = ptr.add(1);
                        }
                    }
                    _ => {
                        errorcode = ERR11;
                        failed!();
                    }
                }
            }

            // Handle extended / extended-more interactions.
            if (set & (PCRE2_EXTENDED as u32 | PCRE2_EXTENDED_MORE as u32))
                == PCRE2_EXTENDED as u32
                || (unset & PCRE2_EXTENDED as u32) != 0
            {
                unset |= PCRE2_EXTENDED_MORE as u32;
            }

            options = (options | set) & (!unset);
            xoptions = (xoptions | xset) & (!xunset);

            // ')' -> option change at this level; ':' -> non-capturing group.
            if ptr >= ptrend {
                unclosed_parenthesis!();
            }
            let ended_paren = *ptr as u32 == CHAR_RIGHT_PARENTHESIS;
            ptr = ptr.add(1);
            if ended_paren {
                nest_depth -= 1; // This is not a nested group after all.
                if top_nest > (*cb).start_workspace as *mut nest_save
                    && (*top_nest.sub(1)).nest_depth == nest_depth
                {
                    top_nest = top_nest.sub(1);
                } else {
                    (*top_nest).nest_depth = nest_depth;
                }
            } else {
                *parsed_pattern = META_NOCAPTURE as u32;
                parsed_pattern = parsed_pattern.add(1);
            }

            // If nothing changed, no need to record.
            if options != oldoptions || xoptions != oldxoptions {
                *parsed_pattern = META_OPTIONS as u32;
                parsed_pattern = parsed_pattern.add(1);
                *parsed_pattern = options;
                parsed_pattern = parsed_pattern.add(1);
                *parsed_pattern = xoptions;
                parsed_pattern = parsed_pattern.add(1);
            }
        }

        sync_out!();
        OptR::Break
    }
}

// ---------------------------------------------------------------------------
// case CHAR_C — callout with numerical or string argument.
// On entry `ptr` points at the 'C'.
// ---------------------------------------------------------------------------
enum CalloutR {
    Break,
    Failed,
    FailedForward,
}

unsafe fn parse_callout(
    ptr_ref: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    parsed_pattern_ref: *mut *mut u32,
    previous_callout_ref: *mut *mut u32,
    after_manual_callout_ref: *mut c_int,
    expect_cond_assert_ref: *mut c_int,
    prev_expect_cond_assert: c_int,
    offset_ref: *mut PCRE2_SIZE,
    i_ref: *mut c_int,
    errorcode_ref: *mut c_int,
    options: u32,
    xoptions: u32,
    _utf: BOOL,
    cb: *mut compile_block,
) -> CalloutR {
    unsafe {
        let mut ptr = *ptr_ref;
        let mut parsed_pattern = *parsed_pattern_ref;
        let mut previous_callout = *previous_callout_ref;
        let mut offset: PCRE2_SIZE = *offset_ref;
        let mut errorcode = *errorcode_ref;

        macro_rules! sync_out {
            () => {{
                *ptr_ref = ptr;
                *parsed_pattern_ref = parsed_pattern;
                *previous_callout_ref = previous_callout;
                *offset_ref = offset;
                *errorcode_ref = errorcode;
            }};
        }
        macro_rules! failed {
            () => {{
                sync_out!();
                return CalloutR::Failed;
            }};
        }
        macro_rules! failed_forward {
            () => {{
                sync_out!();
                return CalloutR::FailedForward;
            }};
        }

        if (xoptions & PCRE2_EXTRA_NEVER_CALLOUT as u32) != 0 {
            ptr = ptr.add(1);
            errorcode = ERR103;
            failed!();
        }

        ptr = ptr.add(1);
        if ptr >= ptrend {
            errorcode = ERR14;
            failed!();
        }

        // Expect an assertion after (?(? ; decrement to identify the assertion.
        *expect_cond_assert_ref = prev_expect_cond_assert - 1;

        // If this follows a previous callout: abolish an automatic one.
        if !previous_callout.is_null()
            && (options & PCRE2_AUTO_CALLOUT as u32) != 0
            && previous_callout == parsed_pattern.sub(4)
            && *parsed_pattern.offset(-1) == 255
        {
            parsed_pattern = previous_callout;
        }

        previous_callout = parsed_pattern;
        *after_manual_callout_ref = 1;

        // Handle a string argument; specific delimiter is required.
        if *ptr as u32 != CHAR_RIGHT_PARENTHESIS && !IS_DIGIT(*ptr as u32) {
            let calloutlength: PCRE2_SIZE;
            let startptr: PCRE2_SPTR = ptr;

            let mut delimiter: u32 = 0;
            let mut j: usize = 0;
            while crate::tables::_pcre2_callout_start_delims_8[j] != 0 {
                if *ptr as u32 == crate::tables::_pcre2_callout_start_delims_8[j] {
                    delimiter = crate::tables::_pcre2_callout_end_delims_8[j];
                    break;
                }
                j += 1;
            }
            if delimiter == 0 {
                errorcode = ERR82;
                failed_forward!();
            }

            *parsed_pattern = META_CALLOUT_STRING as u32;
            parsed_pattern = parsed_pattern.add(3); // Skip pattern info

            loop {
                ptr = ptr.add(1);
                if ptr >= ptrend {
                    errorcode = ERR81;
                    ptr = startptr; // more useful message
                    failed!();
                }
                if *ptr as u32 == delimiter && {
                    ptr = ptr.add(1);
                    ptr >= ptrend || *ptr as u32 != delimiter
                } {
                    break;
                }
            }

            calloutlength = ptr.offset_from(startptr) as PCRE2_SIZE;
            if calloutlength as u64 > u32::MAX as u64 {
                errorcode = ERR72;
                failed!();
            }
            *parsed_pattern = calloutlength as u32;
            parsed_pattern = parsed_pattern.add(1);
            offset = startptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
            PUTOFFSET(offset, &mut parsed_pattern);
        }
        // Handle a callout with an optional numerical argument (<= 255).
        else {
            let mut n: c_int = 0;
            *parsed_pattern = META_CALLOUT_NUMBER as u32; // Numerical callout
            parsed_pattern = parsed_pattern.add(3); // Skip pattern info
            while ptr < ptrend && IS_DIGIT(*ptr as u32) {
                n = n * 10 + (*ptr as u32 - CHAR_0) as c_int;
                ptr = ptr.add(1);
                if n > 255 {
                    errorcode = ERR38;
                    failed!();
                }
            }
            *parsed_pattern = n as u32;
            parsed_pattern = parsed_pattern.add(1);
        }

        // Both formats must have a closing parenthesis.
        if ptr >= ptrend || *ptr as u32 != CHAR_RIGHT_PARENTHESIS {
            errorcode = ERR39;
            failed!();
        }
        ptr = ptr.add(1);

        // Remember offset to next item and set a default length.
        *previous_callout.add(1) = ptr.offset_from((*cb).start_pattern) as u32;
        *previous_callout.add(2) = 0;

        let _ = i_ref;
        sync_out!();
        CalloutR::Break
    }
}

// ---------------------------------------------------------------------------
// case CHAR_LEFT_PARENTHESIS (within (?) — conditional group.
// On entry `ptr` points at the '(' of "(?(".
// ---------------------------------------------------------------------------
enum CondR {
    Break,
    Failed,
    FailedForward,
}

unsafe fn parse_conditional(
    ptr_ref: *mut PCRE2_SPTR,
    ptrend: PCRE2_SPTR,
    parsed_pattern_ref: *mut *mut u32,
    nest_depth_ref: *mut u16,
    offset_ref: *mut PCRE2_SIZE,
    terminator_ref: *mut u32,
    name_ref: *mut PCRE2_SPTR,
    namelen_ref: *mut u32,
    i_ref: *mut c_int,
    expect_cond_assert_ref: *mut c_int,
    errorcode_ref: *mut c_int,
    utf: BOOL,
    cb: *mut compile_block,
) -> CondR {
    unsafe {
        let mut ptr = *ptr_ref;
        let mut parsed_pattern = *parsed_pattern_ref;
        let mut nest_depth = *nest_depth_ref;
        let mut offset: PCRE2_SIZE = *offset_ref;
        let mut terminator: u32 = *terminator_ref;
        let mut name: PCRE2_SPTR = *name_ref;
        let mut namelen: u32 = *namelen_ref;
        let mut i: c_int = *i_ref;

        macro_rules! sync_out {
            () => {{
                *ptr_ref = ptr;
                *parsed_pattern_ref = parsed_pattern;
                *nest_depth_ref = nest_depth;
                *offset_ref = offset;
                *terminator_ref = terminator;
                *name_ref = name;
                *namelen_ref = namelen;
                *i_ref = i;
            }};
        }
        macro_rules! failed {
            () => {{
                sync_out!();
                return CondR::Failed;
            }};
        }
        macro_rules! failed_forward {
            () => {{
                sync_out!();
                return CondR::FailedForward;
            }};
        }
        macro_rules! unclosed_parenthesis {
            () => {{
                *errorcode_ref = ERR14;
                sync_out!();
                return CondR::Failed;
            }};
        }

        ptr = ptr.add(1);
        if ptr >= ptrend {
            unclosed_parenthesis!();
        }
        nest_depth += 1;

        // If next is ? or * an assertion is expected next.
        if *ptr as u32 == CHAR_QUESTION_MARK || *ptr as u32 == CHAR_ASTERISK {
            *parsed_pattern = META_COND_ASSERT as u32;
            parsed_pattern = parsed_pattern.add(1);
            ptr = ptr.sub(1); // Pull pointer back to the opening parenthesis.
            *expect_cond_assert_ref = 2;
            sync_out!();
            return CondR::Break;
        }

        // Handle (?([+-]number)...
        if read_number(
            &mut ptr,
            ptrend,
            (*cb).bracount as i32,
            MAX_GROUP_NUMBER,
            ERR61 as u32,
            &mut i,
            errorcode_ref,
        ) != 0
        {
            // PCRE2_ASSERT(i >= 0);
            if i <= 0 {
                *errorcode_ref = ERR15;
                failed!();
            }
            *parsed_pattern = META_COND_NUMBER as u32;
            parsed_pattern = parsed_pattern.add(1);
            offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE - 2;
            PUTOFFSET(offset, &mut parsed_pattern);
            *parsed_pattern = i as u32;
            parsed_pattern = parsed_pattern.add(1);
        } else if *errorcode_ref != 0 {
            failed!(); // Number too big
        }
        // (?(VERSION[>]=n.m)...
        else if ptrend.offset_from(ptr) >= 10
            && strncmp_c8(ptr, STRING_VERSION, 7) == 0
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

            if *ptr as u32 != CHAR_EQUALS_SIGN || {
                ptr = ptr.add(1);
                !IS_DIGIT(*ptr as u32)
            } {
                *errorcode_ref = ERR79;
                if ge == 0 {
                    failed_forward!();
                }
                failed!();
            }

            if read_number(&mut ptr, ptrend, -1, 1000, ERR79 as u32, &mut major, errorcode_ref)
                == 0
            {
                failed!();
            }

            if ptr < ptrend && *ptr as u32 == CHAR_DOT {
                ptr = ptr.add(1);
                if ptr >= ptrend || !IS_DIGIT(*ptr as u32) {
                    *errorcode_ref = ERR79;
                    if ptr < ptrend {
                        failed_forward!();
                    }
                    failed!();
                }
                if read_number(
                    &mut ptr,
                    ptrend,
                    -1,
                    1000,
                    ERR79 as u32,
                    &mut minor,
                    errorcode_ref,
                ) == 0
                {
                    failed!();
                }
            }
            if ptr >= ptrend || *ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                *errorcode_ref = ERR79;
                if ptr < ptrend {
                    failed_forward!();
                }
                failed!();
            }

            *parsed_pattern = META_COND_VERSION as u32;
            parsed_pattern = parsed_pattern.add(1);
            *parsed_pattern = ge;
            parsed_pattern = parsed_pattern.add(1);
            *parsed_pattern = major as u32;
            parsed_pattern = parsed_pattern.add(1);
            *parsed_pattern = minor as u32;
            parsed_pattern = parsed_pattern.add(1);
        }
        // Cases that read a name.
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
                ptr = ptr.sub(1); // Point to char before name
            }

            if read_name(
                &mut ptr,
                ptrend,
                utf,
                terminator,
                &mut offset,
                &mut name,
                &mut namelen,
                errorcode_ref,
                cb,
            ) == 0
            {
                failed!();
            }

            // Handle (?(R&name)
            if was_r_ampersand != 0 {
                *parsed_pattern = META_COND_RNAME as u32;
                ptr = ptr.sub(1); // Back to closing parens
            }
            // Handle (?(name). DEFINE, R<digits>, or a quoted name.
            else if terminator == CHAR_RIGHT_PARENTHESIS {
                if namelen == 6 && strncmp_c8(name, STRING_DEFINE, 6) == 0 {
                    *parsed_pattern = META_COND_DEFINE as u32;
                } else {
                    i = 1;
                    while i < namelen as c_int {
                        if !IS_DIGIT(*name.add(i as usize) as u32) {
                            break;
                        }
                        i += 1;
                    }
                    *parsed_pattern =
                        if *name as u32 == CHAR_R && i >= namelen as c_int {
                            META_COND_RNUMBER as u32
                        } else {
                            META_COND_NAME as u32
                        };
                }
                ptr = ptr.sub(1); // Back to closing parens
            }
            // Handle (?('name') or (?(<name>)
            else {
                *parsed_pattern = META_COND_NAME as u32;
            }

            // All these except DEFINE end with the name length and offset.
            let was_define = *parsed_pattern == META_COND_DEFINE as u32;
            parsed_pattern = parsed_pattern.add(1);
            if !was_define {
                *parsed_pattern = namelen;
                parsed_pattern = parsed_pattern.add(1);
            }
            PUTOFFSET(offset, &mut parsed_pattern);
        }

        // Check the closing parenthesis of the condition.
        if ptr >= ptrend || *ptr as u32 != CHAR_RIGHT_PARENTHESIS {
            *errorcode_ref = ERR24;
            failed!();
        }
        ptr = ptr.add(1);
        sync_out!();
        CondR::Break
    }
}
