/* Translated from c_src/src/pcre2_compile.c lines 3112-5966 */

/*************************************************
*      Parse the pattern, remembering details    *
*************************************************/

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

const NSF_RESET: u16 = 0x0001;
const NSF_CONDASSERT: u16 = 0x0002;
const NSF_ATOMICSR: u16 = 0x0004;

/* Options that are changeable within the pattern must be tracked during
parsing. Some (e.g. PCRE2_EXTENDED) are implemented entirely during parsing,
but all must be tracked so that META_OPTIONS items set the correct values for
the main compiling phase. */

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

const RANGE_NO: u32 = 0; /* State after '[' (initial), or '[a-z'; hyphen is literal */
const RANGE_STARTED: u32 = 1; /* State after '[1-'; last-emitted code is META_RANGE_XYZ */
const RANGE_FORBID_NO: u32 = 2; /* State after '[\d'; '-]' is allowed but not '-1]' */
const RANGE_FORBID_STARTED: u32 = 3; /* State after '[\d-' */
const RANGE_OK_ESCAPED: u32 = 4; /* State after '[\1'; hyphen may be a range */
const RANGE_OK_LITERAL: u32 = 5; /* State after '[1'; hyphen may be a range */

/* States used for analyzing operators and operands in extended character
classes. */

const CLASS_OP_EMPTY: u32 = 0; /* At start of an expression; empty previous contents */
const CLASS_OP_OPERAND: u32 = 1; /* Have preceding operand; after "z" a "--" can follow */
const CLASS_OP_OPERATOR: u32 = 2; /* Have preceding operator; after "--" operand must follow */

/* States used for determining the parse mode in character classes. The two
PERL_EXT values must be last. */

const CLASS_MODE_NORMAL: u32 = 0; /* Ordinary PCRE2 '[...]' class. */
const CLASS_MODE_ALT_EXT: u32 = 1; /* UTS#18-style extended '[...]' class. */
const CLASS_MODE_PERL_EXT: u32 = 2; /* Perl extended '(?[...])' class. */
const CLASS_MODE_PERL_EXT_LEAF: u32 = 3; /* Leaf within extended '(?[ [...] ])' class. */

/* Here's the actual function. */

unsafe fn parse_regex(
    mut ptr: PCRE2_SPTR,
    mut options: u32,
    mut xoptions: u32,
    has_lookbehind: *mut BOOL,
    cb: *mut compile_block,
) -> c_int {
    let mut c: u32 = 0;
    let mut delimiter: u32 = 0;
    let mut namelen: u32 = 0;
    let mut class_range_state: u32 = 0;
    let mut class_op_state: u32 = 0;
    let mut class_mode_state: u32 = 0;
    let mut class_start: *mut u32 = std::ptr::null_mut();
    let mut verblengthptr: *mut u32 = std::ptr::null_mut(); /* Value avoids compiler warning */
    let mut verbstartptr: *mut u32 = std::ptr::null_mut();
    let mut previous_callout: *mut u32 = std::ptr::null_mut();
    let mut parsed_pattern: *mut u32 = (*cb).parsed_pattern;
    let parsed_pattern_end: *mut u32 = (*cb).parsed_pattern_end;
    let mut this_parsed_item: *mut u32 = std::ptr::null_mut();
    let mut prev_parsed_item: *mut u32 = std::ptr::null_mut();
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
    let mut thisptr: PCRE2_SPTR = std::ptr::null();
    let mut name: PCRE2_SPTR = std::ptr::null();
    let ptrend: PCRE2_SPTR = (*cb).end_pattern;
    let mut verbnamestart: PCRE2_SPTR = std::ptr::null(); /* Value avoids compiler warning */
    let mut class_range_forbid_ptr: PCRE2_SPTR = std::ptr::null();
    let mut ng: *mut named_group = std::ptr::null_mut();
    let mut top_nest: *mut nest_save = std::ptr::null_mut();
    let mut end_nests: *mut nest_save = std::ptr::null_mut();

    /* PCRE2_ASSERT(parsed_pattern != NULL); */

    'failed: {
        'failed_forward: {
            'failed_back: {
                'unclosed_parenthesis: {
                    'parsed_end: {
                        /* Insert leading items for word and line matching (features provided
                        for the benefit of pcre2grep). */

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

                        /* If the pattern is actually a literal string, process it separately
                        to avoid cluttering up the main loop. */

                        if (options & PCRE2_LITERAL) != 0 {
                            while ptr < ptrend {
                                /* LCOV_EXCL_START */
                                if parsed_pattern >= parsed_pattern_end {
                                    errorcode = ERR63; /* Internal error (parsed pattern overflow) */
                                    break 'failed;
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
                                *parsed_pattern = c;
                                parsed_pattern = parsed_pattern.add(1);
                                okquantifier = TRUE;
                            }
                            break 'parsed_end;
                        }

                        /* Process a real regex which may contain meta-characters. */

                        top_nest = std::ptr::null_mut();
                        end_nests = (*cb).start_workspace.add((*cb).workspace_size) as *mut nest_save;

                        /* The size of the nest_save structure might not be a factor of the
                        size of the workspace. Therefore we must round down end_nests so as to
                        correctly avoid creating a nest_save that spans the end of the
                        workspace. */

                        end_nests = (end_nests as *mut u8).sub(
                            ((*cb).workspace_size * size_of::<PCRE2_UCHAR>())
                                % size_of::<nest_save>(),
                        ) as *mut nest_save;

                        /* PCRE2_EXTENDED_MORE implies PCRE2_EXTENDED */

                        if (options & PCRE2_EXTENDED_MORE) != 0 {
                            options |= PCRE2_EXTENDED;
                        }

                        /* Now scan the pattern */

                        'main_loop: while ptr < ptrend {
                            let prev_expect_cond_assert: c_int;
                            let mut min_repeat: u32 = 0;
                            let mut max_repeat: u32 = 0;
                            let mut set: u32 = 0;
                            let mut unset: u32 = 0;
                            let mut optset: *mut u32 = std::ptr::null_mut();
                            let mut xset: u32 = 0;
                            let mut xunset: u32 = 0;
                            let mut xoptset: *mut u32 = std::ptr::null_mut();
                            let mut terminator: u32 = 0;
                            let prev_meta_quantifier: u32;
                            let prev_okquantifier: BOOL;
                            let mut tempptr: PCRE2_SPTR = std::ptr::null();
                            let mut offset: PCRE2_SIZE = 0;

                            if nest_depth as u32 > (*(*cb).cx).parens_nest_limit {
                                errorcode = ERR19;
                                break 'failed; /* Parentheses too deeply nested */
                            }

                            /* Check that we haven't emitted too much into parsed_pattern. */

                            /* LCOV_EXCL_START */
                            if parsed_pattern >= parsed_pattern_end {
                                errorcode = ERR63; /* Internal error (parsed pattern overflow) */
                                break 'failed;
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
                                        *parsed_pattern = c;
                                        parsed_pattern = parsed_pattern.add(1);
                                    } else {
                                        let t = after_manual_callout;
                                        after_manual_callout -= 1;
                                        if t <= 0 {
                                            parsed_pattern = manage_callouts(
                                                thisptr,
                                                &mut previous_callout,
                                                auto_callout,
                                                parsed_pattern,
                                                cb,
                                            );
                                        }
                                        *parsed_pattern = c;
                                        parsed_pattern = parsed_pattern.add(1);
                                        okquantifier = TRUE;
                                    }
                                    meta_quantifier = 0;
                                }
                                continue 'main_loop; /* Next character */
                            }

                            /* If we are processing the "name" part of a (*VERB:NAME) item, all
                            characters up to the closing parenthesis are literals except when
                            PCRE2_ALT_VERBNAMES is set. */

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
                                let verbnamelength: PCRE2_SIZE;

                                /* switch(c) */
                                if c == CHAR_RIGHT_PARENTHESIS {
                                    inverbname = FALSE;
                                    /* This is the length in characters */
                                    verbnamelength =
                                        (parsed_pattern.offset_from(verblengthptr) - 1) as PCRE2_SIZE;
                                    /* But the limit on the length is in code units */
                                    if ptr.offset_from(verbnamestart) - 1 > MAX_MARK as isize {
                                        ptr = ptr.sub(1);
                                        errorcode = ERR76;
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
                                } else if c == CHAR_BACKSLASH {
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
                                        *parsed_pattern = CHAR_LEFT_CURLY_BRACKET;
                                        parsed_pattern = parsed_pattern.add(1);
                                        okquantifier = TRUE;
                                    } else if escape == ESC_Q {
                                        inescq = TRUE;
                                    } else if escape == ESC_E {
                                        /* Ignore */
                                    } else {
                                        errorcode = ERR40; /* Invalid in verb name */
                                        break 'failed;
                                    }
                                } else {
                                    /* default: don't use PARSED_LITERAL() because it sets
                                    okquantifier. */
                                    *parsed_pattern = c;
                                    parsed_pattern = parsed_pattern.add(1);
                                }
                                continue 'main_loop; /* Next character in pattern */
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
                                        ptr = ptr.sub(1);
                                        errorcode = ERR28;
                                        break 'failed;
                                    }
                                    inescq = (*ptr as u32 == CHAR_Q) as BOOL;
                                    ptr = ptr.add(1);
                                    continue 'main_loop;
                                }
                            }

                            /* Skip over whitespace and # comments in extended mode. */

                            if (options & PCRE2_EXTENDED) != 0 {
                                if c < 256 && (*(*cb).ctypes.add(c as usize) & ctype_space) != 0 {
                                    continue 'main_loop;
                                }
                                if c == CHAR_NEL || (c | 1) == 0x200f || (c | 1) == 0x2029 {
                                    continue 'main_loop;
                                }
                                if c == CHAR_NUMBER_SIGN {
                                    while ptr < ptrend {
                                        if (IS_NEWLINE!(ptr, cb, (*cb).end_pattern, utf)) {
                                            /* For non-fixed-length newline cases,
                                            IS_NEWLINE sets cb->nllen. */
                                            ptr = ptr.add((*cb).nllen as usize);
                                            break;
                                        }
                                        ptr = ptr.add(1);
                                        if utf != 0 {
                                            FORWARDCHARTEST!(ptr, ptrend);
                                        }
                                    }
                                    continue 'main_loop; /* Next character in pattern */
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
                                    break 'failed; /* to make it easier to debug. */
                                }
                                ptr = ptr.add(1);
                                continue 'main_loop; /* Next character in pattern */
                            }

                            /* If the next item is not a quantifier, fill in length of any
                            previous callout and create an auto callout if required. */

                            if c != CHAR_ASTERISK
                                && c != CHAR_PLUS
                                && c != CHAR_QUESTION_MARK
                                && (c != CHAR_LEFT_CURLY_BRACKET || ({
                                    tempptr = ptr;
                                    read_repeat_counts(
                                        &mut tempptr,
                                        ptrend,
                                        std::ptr::null_mut(),
                                        std::ptr::null_mut(),
                                        &mut errorcode,
                                    ) == 0
                                }))
                            {
                                let t = after_manual_callout;
                                after_manual_callout -= 1;
                                if t <= 0 {
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
                                    errorcode = ERR28;
                                    if expect_cond_assert == 2 {
                                        break 'failed;
                                    }
                                    break 'failed_back;
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
                                continue 'main_loop; /* Next character in pattern */
                            }

                            /* Process the next item in the main part of a pattern. */

                            /* switch(c) */
                            'switch_end: {
                                'define_name: {
                                'post_assertion: {
                                'post_lookbehind: {
                                'negative_look_ahead: {
                                'positive_nonatomic_look_ahead: {
                                'positive_look_ahead: {
                                'atomic_group: {
                                'read_recursion_arguments: {
                                'recurse_by_name: {
                                'set_recursion: {
                                'recursion_bynumber: {
                                'from_perl_extended_class: {
                                'check_quantifier: {

                                /* ---- Escape sequence ---- */

                                if c == CHAR_BACKSLASH {
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
                                    let mut do_escape_failed: BOOL = (errorcode != 0) as BOOL;

                                    'escape_failed: loop {
                                        if do_escape_failed != 0 {
                                            /* ESCAPE_FAILED: */
                                            if (xoptions & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL) == 0 {
                                                break 'failed;
                                            }
                                            ptr = tempptr;
                                            if ptr >= ptrend {
                                                c = CHAR_BACKSLASH;
                                            } else {
                                                /* Get character value, increment pointer */
                                                GETCHARINCTEST!(c, ptr, utf);
                                            }
                                            escape = 0; /* Treat as literal character */
                                        }

                                        /* The escape was a data escape or literal character. */

                                        if escape == 0 {
                                            *parsed_pattern = c;
                                            parsed_pattern = parsed_pattern.add(1);
                                            okquantifier = TRUE;
                                        }
                                        /* The escape was a back (or forward) reference. */
                                        else if escape < 0 {
                                            offset =
                                                ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                                            escape = -escape - 1;
                                            *parsed_pattern = META_BACKREF | escape as u32;
                                            parsed_pattern = parsed_pattern.add(1);
                                            if escape < 10 {
                                                if *(*cb).small_ref_offset.as_ptr().add(escape as usize)
                                                    == PCRE2_UNSET
                                                {
                                                    *(*cb)
                                                        .small_ref_offset
                                                        .as_mut_ptr()
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
                                            /* switch (escape) */
                                            if escape == ESC_C {
                                                if (options & PCRE2_NEVER_BACKSLASH_C) != 0 {
                                                    errorcode = ERR83;
                                                    do_escape_failed = TRUE;
                                                    continue 'escape_failed;
                                                }
                                                okquantifier = TRUE;
                                                *parsed_pattern = META_ESCAPE + escape as u32;
                                                parsed_pattern = parsed_pattern.add(1);
                                            }
                                            /* This is a special return that happens only in
                                            EXTRA_ALT_BSUX mode, when \u{ is not followed by hex
                                            digits and }. */
                                            else if escape == ESC_ub {
                                                *parsed_pattern = CHAR_u;
                                                parsed_pattern = parsed_pattern.add(1);
                                                *parsed_pattern = CHAR_LEFT_CURLY_BRACKET;
                                                parsed_pattern = parsed_pattern.add(1);
                                                okquantifier = TRUE;
                                            } else if escape == ESC_X
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
                                            }
                                            /* Escapes that may change in UCP mode. */
                                            else if escape == ESC_d
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
                                            }
                                            /* Unicode property matching */
                                            else if escape == ESC_P || escape == ESC_p {
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
                                                    do_escape_failed = TRUE;
                                                    continue 'escape_failed;
                                                }
                                                if negated != 0 {
                                                    escape = if escape == ESC_P {
                                                        ESC_p
                                                    } else {
                                                        ESC_P
                                                    };
                                                }
                                                *parsed_pattern = META_ESCAPE + escape as u32;
                                                parsed_pattern = parsed_pattern.add(1);
                                                *parsed_pattern =
                                                    ((ptype as u32) << 16) | pdata as u32;
                                                parsed_pattern = parsed_pattern.add(1);
                                                okquantifier = TRUE;
                                            }
                                            /* When \g is used with quotes or angle brackets as
                                            delimiters, it is a numerical or named subroutine
                                            call. \k is always a named back reference. */
                                            else if escape == ESC_g || escape == ESC_k {
                                                if ptr >= ptrend
                                                    || (*ptr as u32 != CHAR_LEFT_CURLY_BRACKET
                                                        && *ptr as u32 != CHAR_LESS_THAN_SIGN
                                                        && *ptr as u32 != CHAR_APOSTROPHE)
                                                {
                                                    errorcode = if escape == ESC_g {
                                                        ERR57
                                                    } else {
                                                        ERR69
                                                    };
                                                    do_escape_failed = TRUE;
                                                    continue 'escape_failed;
                                                }
                                                terminator = if *ptr as u32 == CHAR_LESS_THAN_SIGN {
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
                                                        ERR61 as u32,
                                                        &mut i,
                                                        &mut errorcode,
                                                    ) != 0
                                                    {
                                                        if p >= ptrend || *p as u32 != terminator {
                                                            ptr = p;
                                                            errorcode = ERR119; /* Missing terminator for number */
                                                            do_escape_failed = TRUE;
                                                            continue 'escape_failed;
                                                        }
                                                        ptr = p.add(1);
                                                        break 'set_recursion;
                                                    }
                                                    if errorcode != 0 {
                                                        do_escape_failed = TRUE;
                                                        continue 'escape_failed;
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
                                                    do_escape_failed = TRUE;
                                                    continue 'escape_failed;
                                                }

                                                /* \k and \g when used with braces are back
                                                references, whereas \g used with quotes or angle
                                                brackets is a recursion */

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

                                                PUTOFFSET!(offset, parsed_pattern);
                                                okquantifier = TRUE;
                                            }
                                            /* default: \A, \B, \b, \G, \K, \Z, \z cannot be
                                            quantified. */
                                            else {
                                                *parsed_pattern = META_ESCAPE + escape as u32;
                                                parsed_pattern = parsed_pattern.add(1);
                                            }
                                        }
                                        break;
                                    }
                                    break 'switch_end; /* End escape sequence processing */
                                }

                                /* ---- Single-character special items ---- */

                                else if c == CHAR_CIRCUMFLEX_ACCENT {
                                    *parsed_pattern = META_CIRCUMFLEX;
                                    parsed_pattern = parsed_pattern.add(1);
                                    break 'switch_end;
                                } else if c == CHAR_DOLLAR_SIGN {
                                    *parsed_pattern = META_DOLLAR;
                                    parsed_pattern = parsed_pattern.add(1);
                                    break 'switch_end;
                                } else if c == CHAR_DOT {
                                    *parsed_pattern = META_DOT;
                                    parsed_pattern = parsed_pattern.add(1);
                                    okquantifier = TRUE;
                                    break 'switch_end;
                                }
                                /* ---- Single-character quantifiers ---- */
                                else if c == CHAR_ASTERISK {
                                    meta_quantifier = META_ASTERISK;
                                    break 'check_quantifier;
                                } else if c == CHAR_PLUS {
                                    meta_quantifier = META_PLUS;
                                    break 'check_quantifier;
                                } else if c == CHAR_QUESTION_MARK {
                                    meta_quantifier = META_QUERY;
                                    break 'check_quantifier;
                                }
                                /* ---- Potential {n,m} quantifier ---- */
                                else if c == CHAR_LEFT_CURLY_BRACKET {
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
                                        *parsed_pattern = c; /* Not a quantifier */
                                        parsed_pattern = parsed_pattern.add(1);
                                        okquantifier = TRUE;
                                        break 'switch_end; /* No more quantifier processing */
                                    }
                                    meta_quantifier = META_MINMAX;
                                    break 'check_quantifier; /* Fall through */
                                }
                                /* ---- Character class ---- */
                                else if c == CHAR_LEFT_SQUARE_BRACKET {
                                    /* In another (POSIX) regex library, the ugly syntax
                                    [[:<:]] and [[:>:]] is used for "start of word" and "end of
                                    word". They are replaced by \b(?=\w) and \b(?<=\w). */

                                    if ptrend.offset_from(ptr) >= 6
                                        && (_pcre2_strncmp_c8_8(
                                            ptr,
                                            b"[:<:]]\0".as_ptr() as *const c_char,
                                            6,
                                        ) == 0
                                            || _pcre2_strncmp_c8_8(
                                                ptr,
                                                b"[:>:]]\0".as_ptr() as *const c_char,
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

                                            /* The offset is used only for the "non-fixed
                                            length" error; this won't occur here, so just store
                                            zero. */

                                            let zero: PCRE2_SIZE = 0;
                                            PUTOFFSET!(zero, parsed_pattern);
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
                                        break 'switch_end;
                                    }

                                    /* PCRE supports POSIX class stuff inside a class. Perl
                                    gives an error if they are encountered at the top level, so
                                    we'll do that too. */

                                    if ptr < ptrend
                                        && (*ptr as u32 == CHAR_COLON
                                            || *ptr as u32 == CHAR_DOT
                                            || *ptr as u32 == CHAR_EQUALS_SIGN)
                                        && check_posix_syntax(ptr, ptrend, &mut tempptr) != 0
                                    {
                                        let t = *ptr;
                                        ptr = ptr.sub(1);
                                        errorcode = if t as u32 == CHAR_COLON { ERR12 } else { ERR13 };
                                        ptr = tempptr.add(2);
                                        break 'failed;
                                    }

                                    class_mode_state = if (options & PCRE2_ALT_EXTENDED_CLASS) != 0
                                    {
                                        CLASS_MODE_ALT_EXT
                                    } else {
                                        CLASS_MODE_NORMAL
                                    };

                                    /* Jump here from '(?[...])'. That jump must initialize
                                    class_mode_state, set c to the '[' character, and ptr to
                                    just after the '['. */

                                    break 'from_perl_extended_class;
                                }
                                /* ---- Opening parenthesis ---- */
                                else if c == CHAR_LEFT_PARENTHESIS {
                                    if ptr >= ptrend {
                                        break 'unclosed_parenthesis;
                                    }

                                    /* If ( is not followed by ? it is either a capture or a
                                    special verb or an alpha assertion or a positive non-atomic
                                    lookahead. */

                                    if *ptr as u32 != CHAR_QUESTION_MARK {
                                        let mut vn: *const c_char = std::ptr::null();

                                        /* Handle capturing brackets (or non-capturing if
                                        auto-capture is turned off). */

                                        if *ptr as u32 != CHAR_ASTERISK {
                                            nest_depth += 1;
                                            if (options & PCRE2_NO_AUTO_CAPTURE) == 0 {
                                                if (*cb).bracount >= MAX_GROUP_NUMBER {
                                                    errorcode = ERR97;
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
                                        /* Do nothing for (* followed by end of pattern or ) so
                                        it gives a "bad quantifier" error rather than "(*MARK)
                                        must have an argument". */
                                        else if ptrend.offset_from(ptr) <= 1 || ({
                                            c = *ptr.add(1) as u32;
                                            c == CHAR_RIGHT_PARENTHESIS
                                        }) {
                                            break 'switch_end;
                                        }
                                        /* Handle "alpha assertions" such as (*pla:...). */
                                        else if CHMAX_255!(c) != 0
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
                                                errorcode = ERR95; /* Malformed */
                                                break 'failed_forward;
                                            }

                                            /* Scan the table of alpha assertion names */

                                            i = 0;
                                            while i < alascount {
                                                if namelen
                                                    == (*alasmeta.as_ptr().add(i as usize)).len
                                                    && _pcre2_strncmp_c8_8(
                                                        name,
                                                        vn,
                                                        namelen as usize,
                                                    ) == 0
                                                {
                                                    break;
                                                }
                                                vn = vn.add(
                                                    (*alasmeta.as_ptr().add(i as usize)).len
                                                        as usize
                                                        + 1,
                                                );
                                                i += 1;
                                            }

                                            if i >= alascount {
                                                errorcode = ERR95; /* Alpha assertion not recognized */
                                                break 'failed;
                                            }

                                            /* Check for expecting an assertion condition. If
                                            so, only atomic lookaround assertions are valid. */

                                            meta = (*alasmeta.as_ptr().add(i as usize)).meta;
                                            if prev_expect_cond_assert > 0
                                                && (meta < META_LOOKAHEAD
                                                    || meta > META_LOOKBEHINDNOT)
                                            {
                                                errorcode = ERR28; /* Atomic assertion expected */
                                                break 'failed;
                                            }

                                            /* switch(meta) */
                                            if meta == META_ATOMIC {
                                                break 'atomic_group;
                                            } else if meta == META_LOOKAHEAD {
                                                break 'positive_look_ahead;
                                            } else if meta == META_LOOKAHEAD_NA {
                                                break 'positive_nonatomic_look_ahead;
                                            } else if meta == META_LOOKAHEADNOT {
                                                break 'negative_look_ahead;
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
                                                if parsed_pattern.is_null() {
                                                    break 'failed;
                                                }
                                                break 'post_assertion;
                                            } else if meta == META_LOOKBEHIND
                                                || meta == META_LOOKBEHINDNOT
                                                || meta == META_LOOKBEHIND_NA
                                            {
                                                *parsed_pattern = meta;
                                                parsed_pattern = parsed_pattern.add(1);
                                                ptr = ptr.sub(1);
                                                break 'post_lookbehind;
                                            }
                                            /* The script run facilities are handled here. */
                                            else if meta == META_SCRIPT_RUN
                                                || meta == META_ATOMIC_SCRIPT_RUN
                                            {
                                                *parsed_pattern = META_SCRIPT_RUN;
                                                parsed_pattern = parsed_pattern.add(1);
                                                nest_depth += 1;
                                                ptr = ptr.add(1);
                                                if meta == META_ATOMIC_SCRIPT_RUN {
                                                    *parsed_pattern = META_ATOMIC;
                                                    parsed_pattern = parsed_pattern.add(1);
                                                    if top_nest.is_null() {
                                                        top_nest = (*cb).start_workspace
                                                            as *mut nest_save;
                                                    } else {
                                                        top_nest = top_nest.add(1);
                                                        if top_nest >= end_nests {
                                                            errorcode = ERR84;
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
                                            } else {
                                                /* LCOV_EXCL_START */
                                                errorcode = ERR89; /* Unknown code; should never occur because */
                                                break 'failed; /* the meta values come from a table above. */
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
                                                break 'failed;
                                            }
                                            if ptr >= ptrend
                                                || (*ptr as u32 != CHAR_COLON
                                                    && *ptr as u32 != CHAR_RIGHT_PARENTHESIS)
                                            {
                                                errorcode = ERR60; /* Malformed */
                                                break 'failed;
                                            }

                                            /* Scan the table of verb names */

                                            i = 0;
                                            while i < verbcount {
                                                if namelen == (*verbs.as_ptr().add(i as usize)).len
                                                    && _pcre2_strncmp_c8_8(
                                                        name,
                                                        vn,
                                                        namelen as usize,
                                                    ) == 0
                                                {
                                                    break;
                                                }
                                                vn = vn.add(
                                                    (*verbs.as_ptr().add(i as usize)).len as usize
                                                        + 1,
                                                );
                                                i += 1;
                                            }

                                            if i >= verbcount {
                                                errorcode = ERR60; /* Verb not recognized */
                                                break 'failed;
                                            }

                                            /* An empty argument is treated as no argument. */

                                            if *ptr as u32 == CHAR_COLON
                                                && ptr.add(1) < ptrend
                                                && *ptr.add(1) as u32 == CHAR_RIGHT_PARENTHESIS
                                            {
                                                ptr = ptr.add(1); /* Advance to the closing parens */
                                            }

                                            /* Check for mandatory non-empty argument; this is
                                            (*MARK) */

                                            if (*verbs.as_ptr().add(i as usize)).has_arg > 0
                                                && *ptr as u32 != CHAR_COLON
                                            {
                                                errorcode = ERR66;
                                                break 'failed;
                                            }

                                            /* Remember where this verb, possibly with a
                                            preceding (*MARK), starts, for handling quantified
                                            (*ACCEPT). */

                                            verbstartptr = parsed_pattern;
                                            okquantifier = ((*verbs.as_ptr().add(i as usize)).meta
                                                == META_ACCEPT)
                                                as BOOL;

                                            /* We set inverbname TRUE here, and let the main loop
                                            take care of this. */

                                            if ({
                                                let t = *ptr;
                                                ptr = ptr.add(1);
                                                t as u32 == CHAR_COLON
                                            }) {
                                                /* Some optional arguments can be treated as a
                                                preceding (*MARK) */

                                                if (*verbs.as_ptr().add(i as usize)).has_arg < 0 {
                                                    add_after_mark =
                                                        (*verbs.as_ptr().add(i as usize)).meta;
                                                    *parsed_pattern = META_MARK;
                                                    parsed_pattern = parsed_pattern.add(1);
                                                }
                                                /* The remaining verbs with arguments (except
                                                *MARK) need a different opcode. */
                                                else {
                                                    *parsed_pattern = (*verbs
                                                        .as_ptr()
                                                        .add(i as usize))
                                                    .meta
                                                        + (if (*verbs.as_ptr().add(i as usize)).meta
                                                            != META_MARK
                                                        {
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
                                            }
                                            /* No verb "name" argument */
                                            else {
                                                *parsed_pattern =
                                                    (*verbs.as_ptr().add(i as usize)).meta;
                                                parsed_pattern = parsed_pattern.add(1);
                                            }
                                        } /* End of (*VERB) handling */
                                        break 'switch_end; /* Done with this parenthesis */
                                    } /* End of groups that don't start with (? */

                                    /* ---- Items starting (? ---- */

                                    ptr = ptr.add(1);
                                    if ptr >= ptrend {
                                        break 'unclosed_parenthesis;
                                    }

                                    /* switch(*ptr) */
                                    let pc: u32 = *ptr as u32;

                                    /* ---- Python syntax support ---- */

                                    if pc == CHAR_P {
                                        ptr = ptr.add(1);
                                        if ptr >= ptrend {
                                            break 'unclosed_parenthesis;
                                        }

                                        /* (?P<name> is the same as (?<name>. */

                                        if *ptr as u32 == CHAR_LESS_THAN_SIGN {
                                            terminator = CHAR_GREATER_THAN_SIGN;
                                            break 'define_name;
                                        }

                                        /* (?P>name) is the same as (?&name). */

                                        if *ptr as u32 == CHAR_GREATER_THAN_SIGN {
                                            break 'recurse_by_name;
                                        }

                                        /* (?P=name) is the same as \k<name>. */

                                        if *ptr as u32 != CHAR_EQUALS_SIGN {
                                            errorcode = ERR41;
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
                                        PUTOFFSET!(offset, parsed_pattern);
                                        okquantifier = TRUE;
                                        break 'switch_end; /* End of (?P processing */
                                    }
                                    /* ---- Recursion/subroutine calls by number ---- */
                                    else if pc == CHAR_R {
                                        i = 0; /* (?R) == (?R0) */
                                        ptr = ptr.add(1);
                                        if ptr >= ptrend
                                            || (*ptr as u32 != CHAR_RIGHT_PARENTHESIS
                                                && *ptr as u32 != CHAR_LEFT_PARENTHESIS)
                                        {
                                            errorcode = ERR58;
                                            break 'failed;
                                        }
                                        terminator = CHAR_NUL;
                                        break 'set_recursion;
                                    }
                                    /* An item starting (?- followed by a digit comes here via
                                    the "default" case because (?- followed by a non-digit is an
                                    options setting. */
                                    else if pc == CHAR_PLUS {
                                        if ptr.add(1) >= ptrend {
                                            ptr = ptr.add(1);
                                            break 'unclosed_parenthesis;
                                        }
                                        if !(*ptr.add(1) as u32 >= CHAR_0
                                            && *ptr.add(1) as u32 <= CHAR_9)
                                        {
                                            errorcode = ERR29; /* Missing number */
                                            ptr = ptr.add(1);
                                            break 'failed_forward;
                                        }
                                        break 'recursion_bynumber; /* Fall through */
                                    } else if pc >= CHAR_0 && pc <= CHAR_9 {
                                        break 'recursion_bynumber;
                                    }
                                    /* ---- Recursion/subroutine calls by name ---- */
                                    else if pc == CHAR_AMPERSAND {
                                        break 'recurse_by_name;
                                    }
                                    /* ---- Callout with numerical or string argument ---- */
                                    else if pc == CHAR_C {
                                        if (xoptions & PCRE2_EXTRA_NEVER_CALLOUT) != 0 {
                                            ptr = ptr.add(1);
                                            errorcode = ERR103;
                                            break 'failed;
                                        }

                                        ptr = ptr.add(1);
                                        if ptr >= ptrend {
                                            break 'unclosed_parenthesis;
                                        }

                                        /* If the previous item was a condition starting (?( an
                                        assertion, optionally preceded by a callout, is
                                        expected. */

                                        expect_cond_assert = prev_expect_cond_assert - 1;

                                        /* If previous_callout is not NULL, it means this follows
                                        a previous callout. */

                                        if !previous_callout.is_null()
                                            && (options & PCRE2_AUTO_CALLOUT) != 0
                                            && previous_callout == parsed_pattern.sub(4)
                                            && *parsed_pattern.offset(-1) == 255
                                        {
                                            parsed_pattern = previous_callout;
                                        }

                                        /* Save for updating next pattern item length, and skip
                                        one item before completing. */

                                        previous_callout = parsed_pattern;
                                        after_manual_callout = 1;

                                        /* Handle a string argument; specific delimiter is
                                        required. */

                                        if *ptr as u32 != CHAR_RIGHT_PARENTHESIS
                                            && !(*ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_9)
                                        {
                                            let calloutlength: PCRE2_SIZE;
                                            let startptr: PCRE2_SPTR = ptr;

                                            delimiter = 0;
                                            i = 0;
                                            while *_pcre2_callout_start_delims_8
                                                .as_ptr()
                                                .add(i as usize)
                                                != 0
                                            {
                                                if *ptr as u32
                                                    == *_pcre2_callout_start_delims_8
                                                        .as_ptr()
                                                        .add(i as usize)
                                                {
                                                    delimiter = *_pcre2_callout_end_delims_8
                                                        .as_ptr()
                                                        .add(i as usize);
                                                    break;
                                                }
                                                i += 1;
                                            }
                                            if delimiter == 0 {
                                                errorcode = ERR82;
                                                break 'failed_forward;
                                            }

                                            *parsed_pattern = META_CALLOUT_STRING;
                                            parsed_pattern = parsed_pattern.add(3); /* Skip pattern info */

                                            loop {
                                                ptr = ptr.add(1);
                                                if ptr >= ptrend {
                                                    errorcode = ERR81;
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

                                            calloutlength =
                                                ptr.offset_from(startptr) as PCRE2_SIZE;
                                            if calloutlength > u32::MAX as PCRE2_SIZE {
                                                errorcode = ERR72;
                                                break 'failed;
                                            }
                                            *parsed_pattern = calloutlength as u32;
                                            parsed_pattern = parsed_pattern.add(1);
                                            offset = startptr.offset_from((*cb).start_pattern)
                                                as PCRE2_SIZE;
                                            PUTOFFSET!(offset, parsed_pattern);
                                        }
                                        /* Handle a callout with an optional numerical argument,
                                        which must be less than or equal to 255. */
                                        else {
                                            let mut n: c_int = 0;
                                            *parsed_pattern = META_CALLOUT_NUMBER; /* Numerical callout */
                                            parsed_pattern = parsed_pattern.add(3); /* Skip pattern info */
                                            while ptr < ptrend
                                                && (*ptr as u32 >= CHAR_0
                                                    && *ptr as u32 <= CHAR_9)
                                            {
                                                let d = *ptr;
                                                ptr = ptr.add(1);
                                                n = n * 10 + (d as u32 - CHAR_0) as c_int;
                                                if n > 255 {
                                                    errorcode = ERR38;
                                                    break 'failed;
                                                }
                                            }
                                            *parsed_pattern = n as u32;
                                            parsed_pattern = parsed_pattern.add(1);
                                        }

                                        /* Both formats must have a closing parenthesis */

                                        if ptr >= ptrend
                                            || *ptr as u32 != CHAR_RIGHT_PARENTHESIS
                                        {
                                            errorcode = ERR39;
                                            break 'failed;
                                        }
                                        ptr = ptr.add(1);

                                        /* Remember the offset to the next item in the pattern,
                                        and set a default length. */

                                        *previous_callout.add(1) =
                                            ptr.offset_from((*cb).start_pattern) as u32;
                                        *previous_callout.add(2) = 0;
                                        break 'switch_end; /* End callout */
                                    }
                                    /* ---- Conditional group ---- */
                                    else if pc == CHAR_LEFT_PARENTHESIS {
                                        ptr = ptr.add(1);
                                        if ptr >= ptrend {
                                            break 'unclosed_parenthesis;
                                        }
                                        nest_depth += 1;

                                        /* If the next character is ? or * there must be an
                                        assertion next (optionally preceded by a callout). */

                                        if *ptr as u32 == CHAR_QUESTION_MARK
                                            || *ptr as u32 == CHAR_ASTERISK
                                        {
                                            *parsed_pattern = META_COND_ASSERT;
                                            parsed_pattern = parsed_pattern.add(1);
                                            ptr = ptr.sub(1); /* Pull pointer back to the opening parenthesis. */
                                            expect_cond_assert = 2;
                                            break 'switch_end; /* End of conditional */
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
                                                break 'failed;
                                            }
                                            *parsed_pattern = META_COND_NUMBER;
                                            parsed_pattern = parsed_pattern.add(1);
                                            offset = (ptr.offset_from((*cb).start_pattern) - 2)
                                                as PCRE2_SIZE;
                                            PUTOFFSET!(offset, parsed_pattern);
                                            *parsed_pattern = i as u32;
                                            parsed_pattern = parsed_pattern.add(1);
                                        } else if errorcode != 0 {
                                            break 'failed; /* Number too big */
                                        }
                                        /* No number found. Handle the special case
                                        (?(VERSION[>]=n.m)... */
                                        else if ptrend.offset_from(ptr) >= 10
                                            && _pcre2_strncmp_c8_8(
                                                ptr,
                                                b"VERSION\0".as_ptr() as *const c_char,
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

                                            /* NOTE: cannot write IS_DIGIT(*(++ptr)) here because
                                            IS_DIGIT references its argument twice. */

                                            if *ptr as u32 != CHAR_EQUALS_SIGN || ({
                                                ptr = ptr.add(1);
                                                !(*ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_9)
                                            }) {
                                                errorcode = ERR79;
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
                                                ERR79 as u32,
                                                &mut major,
                                                &mut errorcode,
                                            ) == 0
                                            {
                                                break 'failed;
                                            }

                                            if ptr < ptrend && *ptr as u32 == CHAR_DOT {
                                                ptr = ptr.add(1);
                                                if ptr >= ptrend
                                                    || !(*ptr as u32 >= CHAR_0
                                                        && *ptr as u32 <= CHAR_9)
                                                {
                                                    errorcode = ERR79;
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
                                                    ERR79 as u32,
                                                    &mut minor,
                                                    &mut errorcode,
                                                ) == 0
                                                {
                                                    break 'failed;
                                                }
                                            }
                                            if ptr >= ptrend
                                                || *ptr as u32 != CHAR_RIGHT_PARENTHESIS
                                            {
                                                errorcode = ERR79;
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
                                        /* All the remaining cases now require us to read a
                                        name. */
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
                                            /* Handle (?(name). */
                                            else if terminator == CHAR_RIGHT_PARENTHESIS {
                                                if namelen == 6
                                                    && _pcre2_strncmp_c8_8(
                                                        name,
                                                        b"DEFINE\0".as_ptr() as *const c_char,
                                                        6,
                                                    ) == 0
                                                {
                                                    *parsed_pattern = META_COND_DEFINE;
                                                } else {
                                                    i = 1;
                                                    while i < namelen as c_int {
                                                        if !(*name.add(i as usize) as u32
                                                            >= CHAR_0
                                                            && *name.add(i as usize) as u32
                                                                <= CHAR_9)
                                                        {
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

                                            /* All these cases except DEFINE end with the name
                                            length and offset; DEFINE just has an offset. */

                                            let v = *parsed_pattern;
                                            parsed_pattern = parsed_pattern.add(1);
                                            if v != META_COND_DEFINE {
                                                *parsed_pattern = namelen;
                                                parsed_pattern = parsed_pattern.add(1);
                                            }
                                            PUTOFFSET!(offset, parsed_pattern);
                                        } /* End cases that read a name */

                                        /* Check the closing parenthesis of the condition */

                                        if ptr >= ptrend
                                            || *ptr as u32 != CHAR_RIGHT_PARENTHESIS
                                        {
                                            errorcode = ERR24;
                                            break 'failed;
                                        }
                                        ptr = ptr.add(1);
                                        break 'switch_end; /* End of condition processing */
                                    }
                                    /* ---- Atomic group ---- */
                                    else if pc == CHAR_GREATER_THAN_SIGN {
                                        break 'atomic_group;
                                    }
                                    /* ---- Lookahead assertions ---- */
                                    else if pc == CHAR_EQUALS_SIGN {
                                        break 'positive_look_ahead;
                                    } else if pc == CHAR_ASTERISK {
                                        break 'positive_nonatomic_look_ahead;
                                    } else if pc == CHAR_EXCLAMATION_MARK {
                                        break 'negative_look_ahead;
                                    }
                                    /* ---- Lookbehind assertions ---- */
                                    else if pc == CHAR_LESS_THAN_SIGN {
                                        if ptrend.offset_from(ptr) <= 1
                                            || (*ptr.add(1) as u32 != CHAR_EQUALS_SIGN
                                                && *ptr.add(1) as u32 != CHAR_EXCLAMATION_MARK
                                                && *ptr.add(1) as u32 != CHAR_ASTERISK)
                                        {
                                            terminator = CHAR_GREATER_THAN_SIGN;
                                            break 'define_name;
                                        }
                                        *parsed_pattern =
                                            if *ptr.add(1) as u32 == CHAR_EQUALS_SIGN {
                                                META_LOOKBEHIND
                                            } else if *ptr.add(1) as u32 == CHAR_EXCLAMATION_MARK {
                                                META_LOOKBEHINDNOT
                                            } else {
                                                META_LOOKBEHIND_NA
                                            };
                                        parsed_pattern = parsed_pattern.add(1);
                                        break 'post_lookbehind;
                                    }
                                    /* ---- Define a named group ---- */
                                    else if pc == CHAR_APOSTROPHE {
                                        terminator = CHAR_APOSTROPHE; /* Terminator */
                                        break 'define_name;
                                    }
                                    /* ---- Perl extended character class ---- */
                                    else if pc == CHAR_LEFT_SQUARE_BRACKET {
                                        class_mode_state = CLASS_MODE_PERL_EXT;
                                        c = *ptr as u32;
                                        ptr = ptr.add(1);
                                        break 'from_perl_extended_class;
                                    }
                                    /* default */
                                    else {
                                        if *ptr as u32 == CHAR_MINUS
                                            && ptrend.offset_from(ptr) > 1
                                            && (*ptr.add(1) as u32 >= CHAR_0
                                                && *ptr.add(1) as u32 <= CHAR_9)
                                        {
                                            break 'recursion_bynumber; /* The + case is handled by CHAR_PLUS */
                                        }

                                        /* We now have either (?| or a (possibly empty) option
                                        setting, optionally followed by a non-capturing group. */

                                        nest_depth += 1;
                                        if top_nest.is_null() {
                                            top_nest = (*cb).start_workspace as *mut nest_save;
                                        } else {
                                            top_nest = top_nest.add(1);
                                            if top_nest >= end_nests {
                                                errorcode = ERR84;
                                                break 'failed;
                                            }
                                        }
                                        (*top_nest).nest_depth = nest_depth;
                                        (*top_nest).flags = 0;
                                        (*top_nest).options = options & PARSE_TRACKED_OPTIONS;
                                        (*top_nest).xoptions =
                                            xoptions & PARSE_TRACKED_EXTRA_OPTIONS;

                                        /* Start of non-capturing group that resets the capture
                                        count for each branch. */

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
                                            set = 0;
                                            unset = 0;
                                            optset = &mut set as *mut u32;
                                            xset = 0;
                                            xunset = 0;
                                            xoptset = &mut xset as *mut u32;

                                            /* ^ at the start unsets irmnsx and disables the
                                            subsequent use of - */

                                            if ptr < ptrend
                                                && *ptr as u32 == CHAR_CIRCUMFLEX_ACCENT
                                            {
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
                                                let oc = *ptr as u32;
                                                ptr = ptr.add(1);
                                                if oc == CHAR_MINUS {
                                                    if hyphenok == 0 {
                                                        errorcode = ERR94;
                                                        break 'failed;
                                                    }
                                                    optset = &mut unset as *mut u32;
                                                    xoptset = &mut xunset as *mut u32;
                                                    hyphenok = FALSE;
                                                }
                                                /* There are some two-character sequences that
                                                start with 'a'. */
                                                else if oc == CHAR_a {
                                                    let mut handled = false;
                                                    if ptr < ptrend {
                                                        if *ptr as u32 == CHAR_D {
                                                            *xoptset |= PCRE2_EXTRA_ASCII_BSD;
                                                            ptr = ptr.add(1);
                                                            handled = true;
                                                        } else if *ptr as u32 == CHAR_P {
                                                            *xoptset |= PCRE2_EXTRA_ASCII_POSIX
                                                                | PCRE2_EXTRA_ASCII_DIGIT;
                                                            ptr = ptr.add(1);
                                                            handled = true;
                                                        } else if *ptr as u32 == CHAR_S {
                                                            *xoptset |= PCRE2_EXTRA_ASCII_BSS;
                                                            ptr = ptr.add(1);
                                                            handled = true;
                                                        } else if *ptr as u32 == CHAR_T {
                                                            *xoptset |= PCRE2_EXTRA_ASCII_DIGIT;
                                                            ptr = ptr.add(1);
                                                            handled = true;
                                                        } else if *ptr as u32 == CHAR_W {
                                                            *xoptset |= PCRE2_EXTRA_ASCII_BSW;
                                                            ptr = ptr.add(1);
                                                            handled = true;
                                                        }
                                                    }
                                                    if !handled {
                                                        *xoptset |= PCRE2_EXTRA_ASCII_BSD
                                                            | PCRE2_EXTRA_ASCII_BSS
                                                            | PCRE2_EXTRA_ASCII_BSW
                                                            | PCRE2_EXTRA_ASCII_DIGIT
                                                            | PCRE2_EXTRA_ASCII_POSIX;
                                                    }
                                                }
                                                /* Record that it changed in the external
                                                options */
                                                else if oc == CHAR_J {
                                                    *optset |= PCRE2_DUPNAMES;
                                                    (*cb).external_flags |= PCRE2_JCHANGED;
                                                } else if oc == CHAR_i {
                                                    *optset |= PCRE2_CASELESS;
                                                } else if oc == CHAR_m {
                                                    *optset |= PCRE2_MULTILINE;
                                                } else if oc == CHAR_n {
                                                    *optset |= PCRE2_NO_AUTO_CAPTURE;
                                                } else if oc == CHAR_r {
                                                    *xoptset |= PCRE2_EXTRA_CASELESS_RESTRICT;
                                                } else if oc == CHAR_s {
                                                    *optset |= PCRE2_DOTALL;
                                                } else if oc == CHAR_U {
                                                    *optset |= PCRE2_UNGREEDY;
                                                }
                                                /* If x appears twice it sets the extended
                                                extended option. */
                                                else if oc == CHAR_x {
                                                    *optset |= PCRE2_EXTENDED;
                                                    if ptr < ptrend && *ptr as u32 == CHAR_x {
                                                        *optset |= PCRE2_EXTENDED_MORE;
                                                        ptr = ptr.add(1);
                                                    }
                                                } else {
                                                    errorcode = ERR11;
                                                    break 'failed;
                                                }
                                            }

                                            /* If we are setting extended without extended-more,
                                            ensure that any existing extended-more gets unset.
                                            Also, unsetting extended must also unset
                                            extended-more. */

                                            if (set & (PCRE2_EXTENDED | PCRE2_EXTENDED_MORE))
                                                == PCRE2_EXTENDED
                                                || (unset & PCRE2_EXTENDED) != 0
                                            {
                                                unset |= PCRE2_EXTENDED_MORE;
                                            }

                                            options = (options | set) & (!unset);
                                            xoptions = (xoptions | xset) & (!xunset);

                                            /* If the options ended with ')' this is not the
                                            start of a nested group with option changes. */

                                            if ptr >= ptrend {
                                                break 'unclosed_parenthesis;
                                            }
                                            if ({
                                                let t = *ptr;
                                                ptr = ptr.add(1);
                                                t as u32 == CHAR_RIGHT_PARENTHESIS
                                            }) {
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
                                        break 'switch_end; /* End default case after (? */
                                    }
                                }
                                /* ---- Branch terminators ---- */
                                /* Alternation: reset the capture count if we are in a (?|
                                group. */
                                else if c == CHAR_VERTICAL_LINE {
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
                                    break 'switch_end;
                                }
                                /* End of group; reset the capture count to the maximum if we
                                are in a (?| group and/or reset the options that are tracked
                                during parsing. */
                                else if c == CHAR_RIGHT_PARENTHESIS {
                                    okquantifier = TRUE;
                                    if !top_nest.is_null()
                                        && (*top_nest).nest_depth == nest_depth
                                    {
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
                                            top_nest = std::ptr::null_mut();
                                        } else {
                                            top_nest = top_nest.sub(1);
                                        }
                                    }
                                    if nest_depth == 0 {
                                        /* Unmatched closing parenthesis */
                                        errorcode = ERR22;
                                        break 'failed;
                                    }
                                    nest_depth -= 1;
                                    *parsed_pattern = META_KET;
                                    parsed_pattern = parsed_pattern.add(1);
                                    break 'switch_end;
                                }
                                /* default: Non-special character */
                                else {
                                    *parsed_pattern = c;
                                    parsed_pattern = parsed_pattern.add(1);
                                    okquantifier = TRUE;
                                    break 'switch_end;
                                }
                                } /* End 'check_quantifier block */

                                /* ---- Quantifier post-processing ---- */

                                /* CHECK_QUANTIFIER: check that a quantifier is allowed after
                                the previous item. This guarantees that there is a previous
                                item. */

                                if prev_okquantifier == 0 {
                                    errorcode = ERR9;
                                    break 'failed;
                                }

                                /* Most (*VERB)s are not allowed to be quantified, but an
                                ungreedy quantifier can be useful for (*ACCEPT). We therefore
                                allow (*ACCEPT) to be quantified by wrapping it in
                                non-capturing brackets, but we have to allow for a preceding
                                (*MARK) for when (*ACCEPT) has an argument. */

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

                                /* Now we can put the quantifier into the parsed pattern
                                vector. */

                                *parsed_pattern = meta_quantifier;
                                parsed_pattern = parsed_pattern.add(1);
                                if c == CHAR_LEFT_CURLY_BRACKET {
                                    *parsed_pattern = min_repeat;
                                    parsed_pattern = parsed_pattern.add(1);
                                    *parsed_pattern = max_repeat;
                                    parsed_pattern = parsed_pattern.add(1);
                                }
                                break 'switch_end;
                                } /* End 'from_perl_extended_class block */

                                /* FROM_PERL_EXTENDED_CLASS: */
                                okquantifier = TRUE;

                                /* Loop for the contents of the class. Classes may be nested, if
                                PCRE2_ALT_EXTENDED_CLASS is set, or the class is of the form
                                (?[...]).  c is still set to '[' so the loop will handle the
                                start of the class. */

                                class_depth_m1 = -1;
                                class_maxdepth_m1 = -1;
                                class_range_state = RANGE_NO;
                                class_op_state = CLASS_OP_EMPTY;
                                class_start = std::ptr::null_mut();

                                'class_loop: loop {
                                let mut char_is_literal: BOOL = TRUE;

                                'class_continue: {
                                'class_literal: {

                                /* Inside \Q...\E everything is literal except \E */

                                if inescq != 0 {
                                    if c == CHAR_BACKSLASH && ptr < ptrend && *ptr as u32 == CHAR_E
                                    {
                                        inescq = FALSE; /* Reset literal state */
                                        ptr = ptr.add(1); /* Skip the 'E' */
                                        break 'class_continue;
                                    }

                                    /* Surprisingly, you cannot use \Q..\E to escape a character
                                    inside a Perl extended class. */

                                    if class_mode_state == CLASS_MODE_PERL_EXT {
                                        errorcode = ERR116;
                                        break 'failed;
                                    }

                                    break 'class_literal;
                                }

                                /* Skip over space and tab (only) in extended-more mode, or
                                anywhere inside a Perl extended class (which implies /xx). */

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
                                    && check_posix_syntax(ptr, ptrend, &mut tempptr) != 0
                                {
                                    let mut posix_negate: BOOL = FALSE;
                                    let posix_class: c_int;

                                    /* Perl treats a hyphen before a POSIX class as a literal,
                                    not the start of a range. PCRE gives an error. */

                                    if class_range_state == RANGE_STARTED {
                                        ptr = tempptr.add(2);
                                        errorcode = ERR50;
                                        break 'failed;
                                    }

                                    /* Roll back to the hyphen for the error position. */

                                    if class_range_state == RANGE_FORBID_STARTED {
                                        ptr = class_range_forbid_ptr;
                                        errorcode = ERR50;
                                        break 'failed;
                                    }

                                    /* Disallow implicit union in Perl extended classes. */

                                    if class_op_state == CLASS_OP_OPERAND
                                        && class_mode_state == CLASS_MODE_PERL_EXT
                                    {
                                        ptr = tempptr.add(2);
                                        errorcode = ERR113;
                                        break 'failed;
                                    }

                                    if *ptr as u32 != CHAR_COLON {
                                        ptr = tempptr.add(2);
                                        errorcode = ERR13;
                                        break 'failed;
                                    }

                                    ptr = ptr.add(1);
                                    if *ptr as u32 == CHAR_CIRCUMFLEX_ACCENT {
                                        posix_negate = TRUE;
                                        ptr = ptr.add(1);
                                    }

                                    posix_class =
                                        check_posix_name(ptr, tempptr.offset_from(ptr) as c_int);
                                    ptr = tempptr.add(2);
                                    if posix_class < 0 {
                                        errorcode = ERR30;
                                        break 'failed;
                                    }

                                    /* Set "a hyphen is forbidden to be the start of a range". */

                                    class_range_state = RANGE_FORBID_NO;
                                    class_op_state = CLASS_OP_OPERAND;

                                    /* When PCRE2_UCP is set, unless PCRE2_EXTRA_ASCII_POSIX is
                                    set, some of the POSIX classes are converted to use Unicode
                                    properties \p or \P or, in one case, \h or \H. */

                                    if (options & PCRE2_UCP) != 0
                                        && (xoptions & PCRE2_EXTRA_ASCII_POSIX) == 0
                                        && !((xoptions & PCRE2_EXTRA_ASCII_DIGIT) != 0
                                            && (posix_class == PC_DIGIT as c_int
                                                || posix_class == PC_XDIGIT as c_int))
                                    {
                                        let ptype: c_int = *posix_substitutes
                                            .as_ptr()
                                            .add((2 * posix_class) as usize);
                                        let pvalue: c_int = *posix_substitutes
                                            .as_ptr()
                                            .add((2 * posix_class + 1) as usize);

                                        if ptype >= 0 {
                                            *parsed_pattern = META_ESCAPE
                                                + (if posix_negate != 0 { ESC_P } else { ESC_p })
                                                    as u32;
                                            parsed_pattern = parsed_pattern.add(1);
                                            *parsed_pattern =
                                                ((ptype as u32) << 16) | pvalue as u32;
                                            parsed_pattern = parsed_pattern.add(1);
                                            break 'class_continue;
                                        }

                                        if pvalue != 0 {
                                            *parsed_pattern = META_ESCAPE
                                                + (if posix_negate != 0 { ESC_H } else { ESC_h })
                                                    as u32;
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
                                    break 'class_continue;
                                }
                                /* Check for the start of the outermost class, or the start of a
                                nested class. */
                                else if (c == CHAR_LEFT_SQUARE_BRACKET
                                    && (class_depth_m1 < 0
                                        || class_mode_state == CLASS_MODE_ALT_EXT
                                        || class_mode_state == CLASS_MODE_PERL_EXT))
                                    || (c == CHAR_LEFT_PARENTHESIS
                                        && class_mode_state == CLASS_MODE_PERL_EXT)
                                {
                                    let start_c: u32 = c;
                                    let new_class_mode_state: u32;

                                    /* Update the class mode, if moving into a 'leaf' inside a
                                    Perl extended class. */

                                    if start_c == CHAR_LEFT_SQUARE_BRACKET
                                        && class_mode_state == CLASS_MODE_PERL_EXT
                                        && class_depth_m1 >= 0
                                    {
                                        new_class_mode_state = CLASS_MODE_PERL_EXT_LEAF;
                                    } else {
                                        new_class_mode_state = class_mode_state;
                                    }

                                    /* Tidy up the other class before starting the nested class.
                                    -[ beginning a nested class is a literal '-' */

                                    if class_range_state == RANGE_STARTED {
                                        *parsed_pattern.offset(-1) = CHAR_MINUS;
                                    }

                                    /* Disallow implicit union in Perl extended classes. */

                                    if class_op_state == CLASS_OP_OPERAND
                                        && class_mode_state == CLASS_MODE_PERL_EXT
                                    {
                                        errorcode = ERR113;
                                        break 'failed;
                                    }

                                    /* Validate nesting depth */
                                    if class_depth_m1 as c_int
                                        >= ECLASS_NEST_LIMIT as c_int - 1
                                    {
                                        ptr = ptr.sub(1); /* Point rightwards at the paren, same as ERR19. */
                                        errorcode = ERR107; /* Classes too deeply nested */
                                        break 'failed;
                                    }

                                    /* Process the character class start. */

                                    negate_class = FALSE;
                                    loop {
                                        if ptr >= ptrend {
                                            if start_c == CHAR_LEFT_PARENTHESIS {
                                                errorcode = ERR14; /* Missing terminating ')' */
                                            } else {
                                                errorcode = ERR6; /* Missing terminating ']' */
                                            }
                                            break 'failed;
                                        }

                                        GETCHARINCTEST!(c, ptr, utf);
                                        if new_class_mode_state == CLASS_MODE_PERL_EXT {
                                            break;
                                        } else if c == CHAR_BACKSLASH {
                                            if ptr < ptrend && *ptr as u32 == CHAR_E {
                                                ptr = ptr.add(1);
                                            } else if ptrend.offset_from(ptr) >= 3
                                                && _pcre2_strncmp_c8_8(
                                                    ptr,
                                                    b"Q\\E\0".as_ptr() as *const c_char,
                                                    3,
                                                ) == 0
                                            {
                                                ptr = ptr.add(3);
                                            } else {
                                                break;
                                            }
                                        } else if (c == CHAR_SPACE || c == CHAR_HT) /* Note: just these two */
                                            && ((options & PCRE2_EXTENDED_MORE) != 0
                                                || new_class_mode_state >= CLASS_MODE_PERL_EXT)
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

                                    /* Now the real contents of the class; c has the first "real"
                                    character. Empty classes are permitted only if the option is
                                    set, and if it's not a Perl-extended class. */

                                    if c == CHAR_RIGHT_SQUARE_BRACKET
                                        && ((*cb).external_options & PCRE2_ALLOW_EMPTY_CLASS) != 0
                                        && new_class_mode_state < CLASS_MODE_PERL_EXT
                                    {
                                        if !class_start.is_null() {
                                            /* Represents that the class is an extended class. */
                                            *class_start |= CLASS_IS_ECLASS;
                                            class_start = std::ptr::null_mut();
                                        }

                                        *parsed_pattern = if negate_class != 0 {
                                            META_CLASS_EMPTY_NOT
                                        } else {
                                            META_CLASS_EMPTY
                                        };
                                        parsed_pattern = parsed_pattern.add(1);

                                        /* Leave nesting depth unchanged; but check for zero
                                        depth to handle the very first (top-level) class being
                                        empty. */
                                        if class_depth_m1 < 0 {
                                            break 'class_loop;
                                        }

                                        class_range_state = RANGE_NO; /* for processing the containing class */
                                        class_op_state = CLASS_OP_OPERAND;
                                        break 'class_continue;
                                    }

                                    /* Enter a non-empty class. */

                                    if !class_start.is_null() {
                                        /* Represents that the class is an extended class. */
                                        *class_start |= CLASS_IS_ECLASS;
                                        class_start = std::ptr::null_mut();
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
                                        .add(class_depth_m1 as usize) = 0;

                                    /* Implement the special start-of-class literal meaning of
                                    ']'. */
                                    if c == CHAR_RIGHT_SQUARE_BRACKET
                                        && new_class_mode_state != CLASS_MODE_PERL_EXT
                                    {
                                        class_range_state = RANGE_OK_LITERAL;
                                        class_op_state = CLASS_OP_OPERAND;
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
                                    /* In Perl extended mode, the ']' can only be used to match
                                    the opening '[', and ')' must match an opening parenthesis. */

                                    if class_mode_state == CLASS_MODE_PERL_EXT {
                                        if c == CHAR_RIGHT_SQUARE_BRACKET && class_depth_m1 != 0 {
                                            errorcode = ERR14;
                                            ptr = ptr.sub(1); /* Correct the offset */
                                            break 'failed;
                                        }
                                        if c == CHAR_RIGHT_PARENTHESIS && class_depth_m1 < 1 {
                                            errorcode = ERR22;
                                            break 'failed;
                                        }
                                    }

                                    /* Check no trailing operator. */
                                    if class_op_state == CLASS_OP_OPERATOR {
                                        errorcode = ERR110;
                                        break 'failed;
                                    }

                                    /* Check no empty expression for Perl extended
                                    expressions. */
                                    if class_mode_state == CLASS_MODE_PERL_EXT
                                        && class_op_state == CLASS_OP_EMPTY
                                    {
                                        errorcode = ERR114;
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
                                                || *ptr as u32 != CHAR_RIGHT_PARENTHESIS
                                            {
                                                errorcode = ERR115;
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
                                    /* The extended class flag has already been set for the
                                    parent class. */
                                    class_start = std::ptr::null_mut();
                                    break 'class_continue;
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
                                        errorcode = ERR109;
                                        break 'failed;
                                    }

                                    if !class_start.is_null() {
                                        /* Represents that the class is an extended class. */
                                        *class_start |= CLASS_IS_ECLASS;
                                        class_start = std::ptr::null_mut();
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
                                    break 'class_continue;
                                }
                                /* Handle a Perl set unary operator */
                                else if class_mode_state == CLASS_MODE_PERL_EXT
                                    && c == CHAR_EXCLAMATION_MARK
                                {
                                    /* Check that the "!" has not got a preceding operand. */
                                    if class_op_state == CLASS_OP_OPERAND {
                                        errorcode = ERR113;
                                        break 'failed;
                                    }

                                    if !class_start.is_null() {
                                        /* Represents that the class is an extended class. */
                                        *class_start |= CLASS_IS_ECLASS;
                                        class_start = std::ptr::null_mut();
                                    }

                                    *parsed_pattern = META_ECLASS_NOT;
                                    parsed_pattern = parsed_pattern.add(1);
                                    class_range_state = RANGE_NO;
                                    class_op_state = CLASS_OP_OPERATOR;
                                    break 'class_continue;
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
                                        errorcode = ERR108;
                                        break 'failed;
                                    }

                                    /* Check for a preceding operand. */
                                    if class_op_state != CLASS_OP_OPERAND {
                                        errorcode = ERR109;
                                        break 'failed;
                                    }

                                    /* Check for mixed precedence. Forbid [A--B&&C]. */
                                    if *(*cb).class_op_used.as_ptr().add(class_depth_m1 as usize)
                                        != 0
                                        && *(*cb)
                                            .class_op_used
                                            .as_ptr()
                                            .add(class_depth_m1 as usize)
                                            != c as u8
                                    {
                                        errorcode = ERR111;
                                        break 'failed;
                                    }

                                    if !class_start.is_null() {
                                        /* Represents that the class is an extended class. */
                                        *class_start |= CLASS_IS_ECLASS;
                                        class_start = std::ptr::null_mut();
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
                                        .add(class_depth_m1 as usize) = c as u8;
                                    break 'class_continue;
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
                                        if (xoptions & PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL) == 0
                                            || class_mode_state >= CLASS_MODE_PERL_EXT
                                        {
                                            break 'failed;
                                        }
                                        ptr = tempptr;
                                        if ptr >= ptrend {
                                            c = CHAR_BACKSLASH;
                                        } else {
                                            /* Get character value, increment pointer */
                                            GETCHARINCTEST!(c, ptr, utf);
                                        }
                                        escape = 0; /* Treat as literal character */
                                    }

                                    /* switch(escape) */
                                    if escape == 0 {
                                        /* Escaped character code point is in c */
                                        char_is_literal = FALSE;
                                        break 'class_literal; /* (a few lines above) */
                                    } else if escape == ESC_b {
                                        c = CHAR_BS; /* \b is backspace in a class */
                                        char_is_literal = FALSE;
                                        break 'class_literal;
                                    } else if escape == ESC_k {
                                        c = CHAR_k; /* \k is not special in a class, just like \g */
                                        char_is_literal = FALSE;
                                        break 'class_literal;
                                    } else if escape == ESC_Q {
                                        inescq = TRUE; /* Enter literal mode */
                                        break 'class_continue;
                                    } else if escape == ESC_E {
                                        /* Ignore orphan \E */
                                        break 'class_continue;
                                    } else if escape == ESC_B || escape == ESC_R || escape == ESC_X
                                    {
                                        /* Always an error in a class */
                                        errorcode = ERR7;
                                        break 'failed;
                                    } else if escape == ESC_N {
                                        /* Not permitted by Perl either */
                                        errorcode = ERR71;
                                        break 'failed;
                                    } else if escape == ESC_H
                                        || escape == ESC_h
                                        || escape == ESC_V
                                        || escape == ESC_v
                                    {
                                        *parsed_pattern = META_ESCAPE + escape as u32;
                                        parsed_pattern = parsed_pattern.add(1);
                                    }
                                    /* These escapes may be converted to Unicode property tests
                                    when PCRE2_UCP is set. */
                                    else if escape == ESC_d
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
                                    }
                                    /* Explicit Unicode property matching */
                                    else if escape == ESC_P || escape == ESC_p {
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
                                            break 'failed;
                                        }

                                        /* In caseless matching, particular characteristics Lu,
                                        Ll, and Lt get converted to the general characteristic
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
                                            escape = if escape == ESC_P { ESC_p } else { ESC_P };
                                        }
                                        *parsed_pattern = META_ESCAPE + escape as u32;
                                        parsed_pattern = parsed_pattern.add(1);
                                        *parsed_pattern = ((ptype as u32) << 16) | pdata as u32;
                                        parsed_pattern = parsed_pattern.add(1);
                                    }
                                    /* All others are not allowed in a class: default and
                                    ESC_A, ESC_Z, ESC_z, ESC_G, ESC_K, ESC_C */
                                    else {
                                        errorcode = ERR7;
                                        break 'failed;
                                    }

                                    /* All the switch-cases above which end in "break" describe a
                                    set of characters. None may start a range. */

                                    if class_range_state == RANGE_STARTED {
                                        errorcode = ERR50;
                                        break 'failed;
                                    }

                                    /* Perl gives a warning unless the hyphen following a
                                    multi-character escape is the last character in the class.
                                    PCRE throws an error. */

                                    if class_range_state == RANGE_FORBID_STARTED {
                                        ptr = class_range_forbid_ptr;
                                        errorcode = ERR50;
                                        break 'failed;
                                    }

                                    /* Disallow implicit union in Perl extended classes. */

                                    if class_op_state == CLASS_OP_OPERAND
                                        && class_mode_state == CLASS_MODE_PERL_EXT
                                    {
                                        errorcode = ERR113;
                                        break 'failed;
                                    }

                                    class_range_state = RANGE_FORBID_NO;
                                    class_op_state = CLASS_OP_OPERAND;
                                    break 'class_continue;
                                }
                                /* Forbid unescaped literals, and the special meaning of '-',
                                inside a Perl extended class. */
                                else if class_mode_state == CLASS_MODE_PERL_EXT {
                                    errorcode = ERR116;
                                    break 'failed;
                                }
                                /* Handle potential start of range */
                                else if c == CHAR_MINUS
                                    && class_range_state >= RANGE_OK_ESCAPED
                                {
                                    *parsed_pattern = if class_range_state == RANGE_OK_LITERAL {
                                        META_RANGE_LITERAL
                                    } else {
                                        META_RANGE_ESCAPED
                                    };
                                    parsed_pattern = parsed_pattern.add(1);
                                    class_range_state = RANGE_STARTED;
                                    break 'class_continue;
                                }
                                /* Handle forbidden start of range */
                                else if c == CHAR_MINUS
                                    && class_range_state == RANGE_FORBID_NO
                                {
                                    *parsed_pattern = CHAR_MINUS;
                                    parsed_pattern = parsed_pattern.add(1);
                                    class_range_state = RANGE_FORBID_STARTED;
                                    class_range_forbid_ptr = ptr;
                                    break 'class_continue;
                                }
                                /* Handle a literal character: falls into CLASS_LITERAL */

                                } /* End 'class_literal block */

                                /* CLASS_LITERAL: */

                                /* Disallow implicit union in Perl extended classes. */

                                if class_op_state == CLASS_OP_OPERAND
                                    && class_mode_state == CLASS_MODE_PERL_EXT
                                {
                                    errorcode = ERR113;
                                    break 'failed;
                                }

                                if class_range_state == RANGE_STARTED {
                                    if c == *parsed_pattern.offset(-2) {
                                        /* Optimize one-char range */
                                        parsed_pattern = parsed_pattern.sub(1);
                                    } else if *parsed_pattern.offset(-2) > c {
                                        /* Check range is in order */
                                        errorcode = ERR8;
                                        break 'failed;
                                    } else {
                                        if char_is_literal == 0
                                            && *parsed_pattern.offset(-1) == META_RANGE_LITERAL
                                        {
                                            *parsed_pattern.offset(-1) = META_RANGE_ESCAPED;
                                        }
                                        *parsed_pattern = c;
                                        parsed_pattern = parsed_pattern.add(1);
                                        okquantifier = TRUE;
                                    }
                                    class_range_state = RANGE_NO;
                                    class_op_state = CLASS_OP_OPERAND;
                                } else if class_range_state == RANGE_FORBID_STARTED {
                                    ptr = class_range_forbid_ptr;
                                    errorcode = ERR50;
                                    break 'failed;
                                } else {
                                    /* Potential start of range */
                                    class_range_state = if char_is_literal != 0 {
                                        RANGE_OK_LITERAL
                                    } else {
                                        RANGE_OK_ESCAPED
                                    };
                                    class_op_state = CLASS_OP_OPERAND;
                                    *parsed_pattern = c;
                                    parsed_pattern = parsed_pattern.add(1);
                                    okquantifier = TRUE;
                                }

                                } /* End 'class_continue block */

                                /* Proceed to next thing in the class. */

                                /* CLASS_CONTINUE: */
                                if ptr >= ptrend {
                                    if class_mode_state == CLASS_MODE_PERL_EXT
                                        && class_depth_m1 > 0
                                    {
                                        errorcode = ERR14; /* Missing terminating ')' */
                                    }
                                    if class_mode_state == CLASS_MODE_ALT_EXT
                                        && class_depth_m1 == 0
                                        && class_maxdepth_m1 == 1
                                    {
                                        errorcode = ERR112; /* Missing terminating ']', but we saw '[ [ ]...' */
                                    } else {
                                        errorcode = ERR6; /* Missing terminating ']' */
                                    }
                                    break 'failed;
                                }
                                GETCHARINCTEST!(c, ptr, utf);
                                } /* End of class-processing loop */

                                break 'switch_end; /* End of character class */
                                } /* End 'recursion_bynumber block */

                                /* RECURSION_BYNUMBER: */
                                if read_number(
                                    &mut ptr,
                                    ptrend,
                                    if *ptr as u32 >= CHAR_0 && *ptr as u32 <= CHAR_9 {
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
                                    break 'failed;
                                }
                                /* PCRE2_ASSERT(i >= 0); NB (?0) is permitted, represented by i=0 */
                                terminator = CHAR_NUL;
                                } /* End 'set_recursion block */

                                /* SET_RECURSION: */
                                *parsed_pattern = META_RECURSE | i as u32;
                                parsed_pattern = parsed_pattern.add(1);
                                offset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
                                /* End of recursive call by number handling */
                                break 'read_recursion_arguments;
                                } /* End 'recurse_by_name block */

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
                                } /* End 'read_recursion_arguments block */

                                /* READ_RECURSION_ARGUMENTS: */
                                PUTOFFSET!(offset, parsed_pattern);
                                okquantifier = TRUE;

                                /* Arguments are not supported for \g construct. */
                                if terminator != CHAR_NUL {
                                    break 'switch_end;
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
                                        break 'failed;
                                    }
                                }

                                if ptr >= ptrend || *ptr as u32 != CHAR_RIGHT_PARENTHESIS {
                                    break 'unclosed_parenthesis;
                                }

                                ptr = ptr.add(1);
                                break 'switch_end;
                                } /* End 'atomic_group block */

                                /* ATOMIC_GROUP: come from (*atomic: */
                                *parsed_pattern = META_ATOMIC;
                                parsed_pattern = parsed_pattern.add(1);
                                nest_depth += 1;
                                ptr = ptr.add(1);
                                break 'switch_end;
                                } /* End 'positive_look_ahead block */

                                /* POSITIVE_LOOK_AHEAD: come from (*pla: */
                                *parsed_pattern = META_LOOKAHEAD;
                                parsed_pattern = parsed_pattern.add(1);
                                ptr = ptr.add(1);
                                break 'post_assertion;
                                } /* End 'positive_nonatomic_look_ahead block */

                                /* POSITIVE_NONATOMIC_LOOK_AHEAD: come from (*napla: */
                                *parsed_pattern = META_LOOKAHEAD_NA;
                                parsed_pattern = parsed_pattern.add(1);
                                ptr = ptr.add(1);
                                break 'post_assertion;
                                } /* End 'negative_look_ahead block */

                                /* NEGATIVE_LOOK_AHEAD: come from (*nla: */
                                *parsed_pattern = META_LOOKAHEADNOT;
                                parsed_pattern = parsed_pattern.add(1);
                                ptr = ptr.add(1);
                                break 'post_assertion;
                                } /* End 'post_lookbehind block */

                                /* POST_LOOKBEHIND: come from (*plb: (*naplb: and (*nlb: */
                                *has_lookbehind = TRUE;
                                offset = (ptr.offset_from((*cb).start_pattern) - 2) as PCRE2_SIZE;
                                PUTOFFSET!(offset, parsed_pattern);
                                ptr = ptr.add(2);
                                /* Fall through */
                                } /* End 'post_assertion block */

                                /* POST_ASSERTION: */
                                nest_depth += 1;
                                if prev_expect_cond_assert > 0 {
                                    if top_nest.is_null() {
                                        top_nest = (*cb).start_workspace as *mut nest_save;
                                    } else {
                                        top_nest = top_nest.add(1);
                                        if top_nest >= end_nests {
                                            errorcode = ERR84;
                                            break 'failed;
                                        }
                                    }
                                    (*top_nest).nest_depth = nest_depth;
                                    (*top_nest).flags = NSF_CONDASSERT;
                                    (*top_nest).options = options & PARSE_TRACKED_OPTIONS;
                                    (*top_nest).xoptions = xoptions & PARSE_TRACKED_EXTRA_OPTIONS;
                                }
                                break 'switch_end;
                                } /* End 'define_name block */

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
                                    errorcode = ERR97;
                                    break 'failed;
                                }
                                (*cb).bracount += 1;
                                *parsed_pattern = META_CAPTURE | (*cb).bracount;
                                parsed_pattern = parsed_pattern.add(1);
                                nest_depth += 1;

                                /* Check not too many names */

                                if (*cb).names_found as u32 >= MAX_NAME_COUNT {
                                    errorcode = ERR49;
                                    break 'failed;
                                }

                                /* Adjust the entry size to accommodate the longest name found. */

                                if namelen + IMM2_SIZE as u32 + 1 > (*cb).name_entry_size as u32 {
                                    (*cb).name_entry_size =
                                        (namelen + IMM2_SIZE as u32 + 1) as u16;
                                }

                                /* Scan the list to check for duplicates. */

                                is_dupname = FALSE;
                                hash = _pcre2_compile_get_hash_from_name8(name, namelen);
                                ng = (*cb).named_groups;
                                i = 0;
                                while i < (*cb).names_found as c_int {
                                    if namelen == (*ng).length as u32
                                        && hash == NAMED_GROUP_GET_HASH(ng)
                                        && _pcre2_strncmp_8(name, (*ng).name, namelen as usize) == 0
                                    {
                                        /* When a bracket is referenced by the same name multiple
                                        times, is not considered as a duplicate and ignored. */
                                        if (*ng).number == (*cb).bracount {
                                            break;
                                        }
                                        if (options & PCRE2_DUPNAMES) == 0 {
                                            errorcode = ERR43;
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
                                        errorcode = ERR65;
                                        break 'failed;
                                    }
                                    i += 1;
                                    ng = ng.add(1);
                                }

                                /* Ignore duplicate with same number. */
                                if i < (*cb).names_found as c_int {
                                    break 'switch_end;
                                }

                                /* Increase the list size if necessary */

                                if (*cb).names_found as u32 >= (*cb).named_group_list_size {
                                    let newsize: u32 = (*cb).named_group_list_size * 2;
                                    let newspace: *mut named_group =
                                        ((*(*cb).cx).memctl.malloc.unwrap())(
                                            newsize as usize * size_of::<named_group>(),
                                            (*(*cb).cx).memctl.memory_data,
                                        ) as *mut named_group;
                                    if newspace.is_null() {
                                        errorcode = ERR21;
                                        break 'failed;
                                    }

                                    memcpy(
                                        newspace as *mut c_void,
                                        (*cb).named_groups as *const c_void,
                                        (*cb).named_group_list_size as usize
                                            * size_of::<named_group>(),
                                    );
                                    if (*cb).named_group_list_size > NAMED_GROUP_LIST_SIZE as u32 {
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
                                break 'switch_end;
                            } /* End 'switch_end block: end of switch on pattern character */
                        } /* End of main character scan loop */

                        /* End of pattern reached. Check for missing ) at the end of a verb
                        name. */

                        if inverbname != 0 && ptr >= ptrend {
                            errorcode = ERR60;
                            break 'failed;
                        }
                    } /* End 'parsed_end block */

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

                    /* LCOV_EXCL_START */
                    if parsed_pattern >= parsed_pattern_end {
                        errorcode = ERR63; /* Internal error (parsed pattern overflow) */
                        break 'failed;
                    }
                    /* LCOV_EXCL_STOP */

                    *parsed_pattern = META_END;
                    if nest_depth == 0 {
                        return 0;
                    }
                } /* End 'unclosed_parenthesis block */

                /* UNCLOSED_PARENTHESIS: */
                errorcode = ERR14;

                /* Come here for all failures. */
                break 'failed;
            } /* End 'failed_back block */

            /* FAILED_BACK: some errors need to indicate the previous character. */
            ptr = ptr.sub(1);
            if utf != 0 {
                BACKCHAR!(ptr);
            }
            break 'failed;
        } /* End 'failed_forward block */

        /* FAILED_FORWARD: some errors need to indicate the next character. */
        ptr = ptr.add(1);
        if utf != 0 {
            FORWARDCHARTEST!(ptr, ptrend);
        }
        break 'failed;
    }

    /* FAILED: */
    (*cb).erroroffset = ptr.offset_from((*cb).start_pattern) as PCRE2_SIZE;
    return errorcode;
}
