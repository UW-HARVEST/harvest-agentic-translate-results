//! Translation of `c_src/src/pcre2_convert.c`.
//!
//! Converts POSIX and glob patterns into PCRE2 regular expression patterns.

#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types, unused_parens)]

use core::ffi::{c_char, c_int, c_void};

use crate::chars::*;
use crate::context::_pcre2_default_convert_context_8;
use crate::internal::*;
use crate::string_utils;

/* ------------------------- Option masks ------------------------- */

const TYPE_OPTIONS: u32 =
    PCRE2_CONVERT_GLOB | PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_POSIX_EXTENDED;

const ALL_OPTIONS: u32 = PCRE2_CONVERT_UTF
    | PCRE2_CONVERT_NO_UTF_CHECK
    | PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR
    | PCRE2_CONVERT_GLOB_NO_STARSTAR
    | TYPE_OPTIONS;

const DUMMY_BUFFER_SIZE: usize = 100;

/* States for POSIX processing */

const POSIX_START_REGEX: u32 = 0;
const POSIX_ANCHORED: u32 = 1;
const POSIX_NOT_BRACKET: u32 = 2;
const POSIX_CLASS_NOT_STARTED: u32 = 3;
const POSIX_CLASS_STARTING: u32 = 4;
const POSIX_CLASS_STARTED: u32 = 5;

/* Generated pattern fragments. In C these are concatenations of the single
byte STR_* macros; here they are byte literals with the same contents. */

/* STR_STAR_NUL == "(*NUL)" */
const STR_STAR_NUL: &[u8] = b"(*NUL)";

/* Literals that must be escaped, NUL-terminated: \ ? * + | . ^ $ { } [ ] ( ) */
static PCRE2_ESCAPED_LITERALS_C: &[u8] = b"\\?*+|.^${}[]()\0";

/* Recognized escaped metacharacters in POSIX basic patterns:
   ( ) { } 1 2 3 4 5 6 7 8 9 */
static POSIX_META_ESCAPES: &[u8] = b"(){}123456789\0";

/* Recognized POSIX classes, colon-separated. */
static POSIX_CLASSES: &[u8] =
    b"alpha:lower:upper:alnum:ascii:blank:cntrl:digit:graph:print:punct:space:word:xdigit:\0";

/* ------------------------- Helpers ------------------------- */

/// `ISLOWER(c)` -- EBCDIC not defined, so this is the ASCII range test.
#[inline]
fn islower(c: u32) -> bool {
    c >= CHAR_a && c <= CHAR_z
}

/// Equivalent of C `strchr(s, c)` where `s` is a NUL-terminated C string.
/// Returns a pointer to the found byte, or null. Matches the terminating NUL
/// when `c == 0`, exactly like libc `strchr`.
#[inline]
unsafe fn strchr(s: *const c_char, c: c_int) -> *const c_char {
    unsafe {
        let target = c as c_char;
        let mut p = s;
        loop {
            if *p == target {
                return p;
            }
            if *p == 0 {
                return core::ptr::null();
            }
            p = p.add(1);
        }
    }
}

/* ------------------------- convert_posix ------------------------- */

/* This function handles both basic and extended POSIX patterns.

Arguments:
  pattype        the pattern type
  pattern        the pattern
  plength        length in code units
  utf            TRUE if UTF
  use_buffer     where to put the output
  use_length     length of use_buffer
  bufflenptr     where to put the used length
  dummyrun       TRUE if a dummy run
  ccontext       the convert context

Returns:         0 => success
                !0 => error code
*/
unsafe fn convert_posix(
    pattype: u32,
    pattern: PCRE2_SPTR,
    mut plength: PCRE2_SIZE,
    utf: BOOL,
    use_buffer: *mut PCRE2_UCHAR,
    use_length: PCRE2_SIZE,
    bufflenptr: *mut PCRE2_SIZE,
    dummyrun: BOOL,
    _ccontext: *mut pcre2_real_convert_context,
) -> c_int {
    unsafe {
        let mut posix: PCRE2_SPTR = pattern;
        let mut p: *mut PCRE2_UCHAR = use_buffer;
        let mut pp: *mut PCRE2_UCHAR = p;
        let endp: *mut PCRE2_UCHAR = p.add(use_length - 1); /* Allow for trailing zero */
        let mut convlength: PCRE2_SIZE = 0;

        let mut bracount: u32 = 0;
        let mut posix_state: u32 = POSIX_START_REGEX;
        let mut lastspecial: u32 = 0;
        let extended: BOOL = ((pattype & PCRE2_CONVERT_POSIX_EXTENDED) != 0) as BOOL;
        let mut nextisliteral: BOOL = FALSE;

        /* Macro PUTCHARS(string): copy a NUL-terminated byte string to output,
        checking for overflow. Implemented as a closure-like local via a macro. */
        macro_rules! putchars {
            ($string:expr) => {{
                let s: &[u8] = $string;
                for &b in s.iter() {
                    if b == 0 {
                        break;
                    }
                    if p >= endp {
                        return PCRE2_ERROR_NOMEMORY;
                    }
                    *p = b;
                    p = p.add(1);
                }
            }};
        }

        /* Initialize default for error offset as end of input. */

        *bufflenptr = plength;
        putchars!(STR_STAR_NUL);

        /* Now scan the input. */

        while plength > 0 {
            let c: u32;
            let sc: u32;
            let mut clength: c_int = 1;

            /* Add in the length of the last item, then, if in the dummy run, pull the
            pointer back to the start of the (temporary) buffer and then remember the
            start of the next item. */

            convlength += p.offset_from(pp) as PCRE2_SIZE;
            if dummyrun != 0 {
                p = use_buffer;
            }
            pp = p;

            /* Pick up the next character (SUPPORT_UNICODE is defined). */

            let (ch, extra) = getcharlentest(posix, utf != FALSE);
            c = ch;
            clength += extra as c_int;
            posix = posix.add(clength as usize);
            plength -= clength as PCRE2_SIZE;

            sc = if nextisliteral != 0 { 0 } else { c };
            nextisliteral = FALSE;

            /* Handle a character within a class. */

            if posix_state >= POSIX_CLASS_NOT_STARTED {
                if c == CHAR_RIGHT_SQUARE_BRACKET {
                    putchars!(STR_RIGHT_SQUARE_BRACKET);
                    posix_state = POSIX_NOT_BRACKET;
                }
                /* Not the end of the class */
                else {
                    match posix_state {
                        POSIX_CLASS_STARTED => {
                            if islower(c) {
                                /* Remain in started state */
                            } else {
                                posix_state = POSIX_CLASS_NOT_STARTED;
                                if c == CHAR_COLON
                                    && plength > 0
                                    && *posix as u32 == CHAR_RIGHT_SQUARE_BRACKET
                                {
                                    putchars!(b":]\0");
                                    plength -= 1;
                                    posix = posix.add(1);
                                    continue; /* With next character after :] */
                                }
                                /* Fall through */
                                if c == CHAR_LEFT_SQUARE_BRACKET {
                                    posix_state = POSIX_CLASS_STARTING;
                                }
                            }
                        }
                        POSIX_CLASS_NOT_STARTED => {
                            if c == CHAR_LEFT_SQUARE_BRACKET {
                                posix_state = POSIX_CLASS_STARTING;
                            }
                        }
                        POSIX_CLASS_STARTING => {
                            if c == CHAR_COLON {
                                posix_state = POSIX_CLASS_STARTED;
                            }
                        }
                        _ => {}
                    }

                    if c == CHAR_BACKSLASH {
                        putchars!(STR_BACKSLASH);
                    }
                    if p.add(clength as usize) > endp {
                        return PCRE2_ERROR_NOMEMORY;
                    }
                    memcpy(p, posix.sub(clength as usize), cu2bytes(clength as usize));
                    p = p.add(clength as usize);
                }
            }
            /* Handle a character not within a class. */
            else {
                /* The C uses a switch on sc with fall-through and gotos. We
                reproduce the control flow using labelled loop-blocks for the two
                goto targets COPY_SPECIAL and ESCAPE_LITERAL. */

                #[derive(PartialEq)]
                enum Flow {
                    Normal,
                    CopySpecial,
                    EscapeLiteral,
                }
                let mut flow = Flow::Normal;

                match sc {
                    CHAR_LEFT_SQUARE_BRACKET => {
                        putchars!(STR_LEFT_SQUARE_BRACKET);

                        /* Handle start of "normal" character classes */
                        posix_state = POSIX_CLASS_NOT_STARTED;

                        /* Handle ^ and ] as first characters */
                        if plength > 0 {
                            if *posix as u32 == CHAR_CIRCUMFLEX_ACCENT {
                                posix = posix.add(1);
                                plength -= 1;
                                putchars!(STR_CIRCUMFLEX_ACCENT);
                            }
                            if plength > 0 && *posix as u32 == CHAR_RIGHT_SQUARE_BRACKET {
                                posix = posix.add(1);
                                plength -= 1;
                                putchars!(STR_RIGHT_SQUARE_BRACKET);
                            }
                        }
                    }

                    CHAR_BACKSLASH => {
                        if plength == 0 {
                            return PCRE2_ERROR_END_BACKSLASH;
                        }
                        if extended != 0 {
                            nextisliteral = TRUE;
                        } else {
                            if (*posix as u32) < 255
                                && !strchr(
                                    POSIX_META_ESCAPES.as_ptr() as *const c_char,
                                    *posix as c_int,
                                )
                                .is_null()
                            {
                                if *posix as u32 >= CHAR_0 && *posix as u32 <= CHAR_9 {
                                    putchars!(STR_BACKSLASH);
                                }
                                if p.add(1) > endp {
                                    return PCRE2_ERROR_NOMEMORY;
                                }
                                *p = *posix;
                                lastspecial = *p as u32;
                                p = p.add(1);
                                posix = posix.add(1);
                                plength -= 1;
                            } else {
                                nextisliteral = TRUE;
                            }
                        }
                    }

                    CHAR_RIGHT_PARENTHESIS => {
                        if extended == 0 || bracount == 0 {
                            flow = Flow::EscapeLiteral;
                        } else {
                            bracount -= 1;
                            flow = Flow::CopySpecial;
                        }
                    }

                    CHAR_LEFT_PARENTHESIS => {
                        bracount += 1;
                        /* Fall through */
                        if extended == 0 {
                            flow = Flow::EscapeLiteral;
                        } else {
                            posix_state = POSIX_NOT_BRACKET;
                            flow = Flow::CopySpecial;
                        }
                    }

                    CHAR_QUESTION_MARK
                    | CHAR_PLUS
                    | CHAR_LEFT_CURLY_BRACKET
                    | CHAR_RIGHT_CURLY_BRACKET
                    | CHAR_VERTICAL_LINE => {
                        if extended == 0 {
                            flow = Flow::EscapeLiteral;
                        } else {
                            /* Fall through to DOT/DOLLAR handling */
                            posix_state = POSIX_NOT_BRACKET;
                            flow = Flow::CopySpecial;
                        }
                    }

                    CHAR_DOT | CHAR_DOLLAR_SIGN => {
                        posix_state = POSIX_NOT_BRACKET;
                        flow = Flow::CopySpecial;
                    }

                    CHAR_ASTERISK => {
                        if lastspecial != CHAR_ASTERISK {
                            if extended == 0
                                && (posix_state < POSIX_NOT_BRACKET
                                    || lastspecial == CHAR_LEFT_PARENTHESIS)
                            {
                                flow = Flow::EscapeLiteral;
                            } else {
                                flow = Flow::CopySpecial;
                            }
                        }
                        /* else: Ignore second and subsequent asterisks */
                    }

                    CHAR_CIRCUMFLEX_ACCENT => {
                        if extended != 0 {
                            flow = Flow::CopySpecial;
                        } else if posix_state == POSIX_START_REGEX
                            || lastspecial == CHAR_LEFT_PARENTHESIS
                        {
                            posix_state = POSIX_ANCHORED;
                            flow = Flow::CopySpecial;
                        } else {
                            /* Fall through to default */
                            flow = Flow::EscapeLiteral; // tentative; resolved below
                            /* Reproduce default handling directly. */
                            flow = Flow::Normal;
                            if c < 255
                                && !strchr(
                                    PCRE2_ESCAPED_LITERALS_C.as_ptr() as *const c_char,
                                    c as c_int,
                                )
                                .is_null()
                            {
                                putchars!(STR_BACKSLASH);
                            }
                            lastspecial = 0xff; /* Indicates nothing special */
                            if p.add(clength as usize) > endp {
                                return PCRE2_ERROR_NOMEMORY;
                            }
                            memcpy(
                                p,
                                posix.sub(clength as usize),
                                cu2bytes(clength as usize),
                            );
                            p = p.add(clength as usize);
                            posix_state = POSIX_NOT_BRACKET;
                        }
                    }

                    _ => {
                        /* default */
                        if c < 255
                            && !strchr(
                                PCRE2_ESCAPED_LITERALS_C.as_ptr() as *const c_char,
                                c as c_int,
                            )
                            .is_null()
                        {
                            putchars!(STR_BACKSLASH);
                        }
                        lastspecial = 0xff; /* Indicates nothing special */
                        if p.add(clength as usize) > endp {
                            return PCRE2_ERROR_NOMEMORY;
                        }
                        memcpy(p, posix.sub(clength as usize), cu2bytes(clength as usize));
                        p = p.add(clength as usize);
                        posix_state = POSIX_NOT_BRACKET;
                    }
                }

                /* Handle the goto targets. */
                if flow == Flow::EscapeLiteral {
                    /* ESCAPE_LITERAL: */
                    putchars!(STR_BACKSLASH);
                    lastspecial = 0xff; /* Indicates nothing special */
                    if p.add(clength as usize) > endp {
                        return PCRE2_ERROR_NOMEMORY;
                    }
                    memcpy(p, posix.sub(clength as usize), cu2bytes(clength as usize));
                    p = p.add(clength as usize);
                    posix_state = POSIX_NOT_BRACKET;
                } else if flow == Flow::CopySpecial {
                    /* COPY_SPECIAL: */
                    lastspecial = c;
                    if p.add(1) > endp {
                        return PCRE2_ERROR_NOMEMORY;
                    }
                    *p = c as PCRE2_UCHAR;
                    p = p.add(1);
                }
            }
        }

        if posix_state >= POSIX_CLASS_NOT_STARTED {
            return PCRE2_ERROR_MISSING_SQUARE_BRACKET;
        }
        convlength += p.offset_from(pp) as PCRE2_SIZE; /* Final segment */
        *bufflenptr = convlength;
        *p = 0;
        0
    }
}

/* ------------------------- glob conversion ------------------------- */

/* Context for writing the output into a buffer. */
struct pcre2_output_context {
    output: *mut PCRE2_UCHAR,  /* current output position */
    output_end: PCRE2_SPTR,    /* output end */
    output_size: PCRE2_SIZE,   /* size of the output */
    out_str: [u8; 8],          /* string copied to the output */
}

/* Write a character into the output. */
unsafe fn convert_glob_write(out: *mut pcre2_output_context, chr: PCRE2_UCHAR) {
    unsafe {
        (*out).output_size += 1;

        if ((*out).output as PCRE2_SPTR) < (*out).output_end {
            *(*out).output = chr;
            (*out).output = (*out).output.add(1);
        }
    }
}

/* Write a string into the output (length bytes from out->out_str). */
unsafe fn convert_glob_write_str(out: *mut pcre2_output_context, mut length: PCRE2_SIZE) {
    unsafe {
        let mut out_str: *const u8 = (*out).out_str.as_ptr();
        let mut output: *mut PCRE2_UCHAR = (*out).output;
        let output_end: PCRE2_SPTR = (*out).output_end;
        let mut output_size: PCRE2_SIZE = (*out).output_size;

        loop {
            output_size += 1;

            if (output as PCRE2_SPTR) < output_end {
                *output = *out_str;
                output = output.add(1);
                out_str = out_str.add(1);
            }

            length -= 1;
            if length == 0 {
                break;
            }
        }

        (*out).output = output;
        (*out).output_size = output_size;
    }
}

/* Prints the separator into the output. */
unsafe fn convert_glob_print_separator(
    out: *mut pcre2_output_context,
    separator: PCRE2_UCHAR,
    with_escape: BOOL,
) {
    unsafe {
        if with_escape != 0 {
            convert_glob_write(out, CHAR_BACKSLASH as PCRE2_UCHAR);
        }
        convert_glob_write(out, separator);
    }
}

/* Prints a wildcard into the output. */
unsafe fn convert_glob_print_wildcard(
    out: *mut pcre2_output_context,
    separator: PCRE2_UCHAR,
    with_escape: BOOL,
) {
    unsafe {
        (*out).out_str[0] = CHAR_LEFT_SQUARE_BRACKET as u8;
        (*out).out_str[1] = CHAR_CIRCUMFLEX_ACCENT as u8;
        convert_glob_write_str(out, 2);

        convert_glob_print_separator(out, separator, with_escape);

        convert_glob_write(out, CHAR_RIGHT_SQUARE_BRACKET as PCRE2_UCHAR);
    }
}

/* Parse a posix class.

Returns:  >0 => class index
          0  => malformed class
*/
unsafe fn convert_glob_parse_class(
    from: *mut PCRE2_SPTR,
    pattern_end: PCRE2_SPTR,
    out: *mut pcre2_output_context,
) -> c_int {
    unsafe {
        let mut start: PCRE2_SPTR = (*from).add(1);
        let mut pattern: PCRE2_SPTR = start;
        let mut class_ptr: *const u8;
        let mut c: PCRE2_UCHAR;
        let mut class_index: c_int;

        loop {
            if pattern >= pattern_end {
                return 0;
            }

            c = *pattern;
            pattern = pattern.add(1);

            if (c as u32) < CHAR_a || (c as u32) > CHAR_z {
                break;
            }
        }

        if c as u32 != CHAR_COLON
            || pattern >= pattern_end
            || *pattern as u32 != CHAR_RIGHT_SQUARE_BRACKET
        {
            return 0;
        }

        class_ptr = POSIX_CLASSES.as_ptr();
        class_index = 1;

        loop {
            if *class_ptr == 0 {
                return 0;
            }

            pattern = start;

            while *pattern == *class_ptr {
                if *pattern as u32 == CHAR_COLON {
                    pattern = pattern.add(2);
                    start = start.sub(2);

                    loop {
                        convert_glob_write(out, *start);
                        start = start.add(1);
                        if start >= pattern {
                            break;
                        }
                    }

                    *from = pattern;
                    return class_index;
                }
                pattern = pattern.add(1);
                class_ptr = class_ptr.add(1);
            }

            while *class_ptr as u32 != CHAR_COLON {
                class_ptr = class_ptr.add(1);
            }
            class_ptr = class_ptr.add(1);
            class_index += 1;
        }
    }
}

/* Checks whether the character is in the class.

Returns:   !0 => character is found in the class
            0 => otherwise
*/
unsafe fn convert_glob_char_in_class(class_index: c_int, c: PCRE2_UCHAR) -> BOOL {
    unsafe {
        let cbits: *const u8 =
            (&raw const crate::context::_pcre2_default_tables_8 as *const u8).add(cbits_offset);
        let cbit: usize;

        /* PCRE2_CODE_UNIT_WIDTH == 8, so no c > 0xff check. */

        let cc = c as usize;
        match class_index {
            1 => {
                /* alpha */
                if c as u32 == CHAR_UNDERSCORE {
                    return FALSE;
                }
                if (*cbits.add(cbit_digit + cc / 8) & (1u8 << (cc & 7))) != 0 {
                    return FALSE;
                }
                cbit = cbit_word;
            }

            2 => cbit = cbit_lower, /* lower */
            3 => cbit = cbit_upper, /* upper */

            4 => {
                /* alnum */
                if c as u32 == CHAR_UNDERSCORE {
                    return FALSE;
                }
                cbit = cbit_word;
            }

            5 => {
                /* ascii */
                if (*cbits.add(cbit_cntrl + cc / 8) & (1u8 << (cc & 7))) != 0 {
                    return TRUE;
                }
                cbit = cbit_print;
            }

            6 => {
                /* blank */
                if c as u32 == CHAR_LF
                    || c as u32 == CHAR_VT
                    || c as u32 == CHAR_FF
                    || c as u32 == CHAR_CR
                {
                    return FALSE;
                }
                cbit = cbit_space;
            }

            7 => cbit = cbit_cntrl,   /* cntrl */
            8 => cbit = cbit_digit,   /* digit */
            9 => cbit = cbit_graph,   /* graph */
            10 => cbit = cbit_print,  /* print */
            11 => cbit = cbit_punct,  /* punct */
            12 => cbit = cbit_space,  /* space */
            13 => cbit = cbit_word,   /* word */
            14 => cbit = cbit_xdigit, /* xdigit */
            _ => return FALSE,
        }

        ((*cbits.add(cbit + cc / 8) & (1u8 << (cc & 7))) != 0) as BOOL
    }
}

/* Parse a range of characters.

Returns:         0 => success
                !0 => error code
*/
unsafe fn convert_glob_parse_range(
    from: *mut PCRE2_SPTR,
    pattern_end: PCRE2_SPTR,
    out: *mut pcre2_output_context,
    _utf: BOOL,
    separator: PCRE2_UCHAR,
    with_escape: BOOL,
    escape: PCRE2_UCHAR,
    no_wildsep: BOOL,
) -> c_int {
    unsafe {
        let mut is_negative: BOOL = FALSE;
        let mut separator_seen: BOOL = FALSE;
        let mut has_prev_c: BOOL;
        let mut pattern: PCRE2_SPTR = *from;
        let mut char_start: PCRE2_SPTR;
        let mut c: u32;
        let mut prev_c: u32;
        let mut len: c_int;
        let mut class_index: c_int;

        if pattern >= pattern_end {
            *from = pattern;
            return PCRE2_ERROR_MISSING_SQUARE_BRACKET;
        }

        if *pattern as u32 == CHAR_EXCLAMATION_MARK
            || *pattern as u32 == CHAR_CIRCUMFLEX_ACCENT
        {
            pattern = pattern.add(1);

            if pattern >= pattern_end {
                *from = pattern;
                return PCRE2_ERROR_MISSING_SQUARE_BRACKET;
            }

            is_negative = TRUE;

            (*out).out_str[0] = CHAR_LEFT_SQUARE_BRACKET as u8;
            (*out).out_str[1] = CHAR_CIRCUMFLEX_ACCENT as u8;
            len = 2;

            if no_wildsep == 0 {
                if with_escape != 0 {
                    (*out).out_str[len as usize] = CHAR_BACKSLASH as u8;
                    len += 1;
                }
                (*out).out_str[len as usize] = separator;
            }

            convert_glob_write_str(out, (len + 1) as PCRE2_SIZE);
        } else {
            convert_glob_write(out, CHAR_LEFT_SQUARE_BRACKET as PCRE2_UCHAR);
        }

        has_prev_c = FALSE;
        prev_c = 0;

        if *pattern as u32 == CHAR_RIGHT_SQUARE_BRACKET {
            (*out).out_str[0] = CHAR_BACKSLASH as u8;
            (*out).out_str[1] = CHAR_RIGHT_SQUARE_BRACKET as u8;
            convert_glob_write_str(out, 2);
            has_prev_c = TRUE;
            prev_c = CHAR_RIGHT_SQUARE_BRACKET;
            pattern = pattern.add(1);
        }

        while pattern < pattern_end {
            char_start = pattern;
            c = getcharinctest(&mut pattern, true);

            if c == CHAR_RIGHT_SQUARE_BRACKET {
                convert_glob_write(out, c as PCRE2_UCHAR);

                if is_negative == 0 && no_wildsep == 0 && separator_seen != 0 {
                    (*out).out_str[0] = CHAR_LEFT_PARENTHESIS as u8;
                    (*out).out_str[1] = CHAR_QUESTION_MARK as u8;
                    (*out).out_str[2] = CHAR_LESS_THAN_SIGN as u8;
                    (*out).out_str[3] = CHAR_EXCLAMATION_MARK as u8;
                    convert_glob_write_str(out, 4);

                    convert_glob_print_separator(out, separator, with_escape);
                    convert_glob_write(out, CHAR_RIGHT_PARENTHESIS as PCRE2_UCHAR);
                }

                *from = pattern;
                return 0;
            }

            if pattern >= pattern_end {
                break;
            }

            if c == CHAR_LEFT_SQUARE_BRACKET && *pattern as u32 == CHAR_COLON {
                *from = pattern;
                class_index = convert_glob_parse_class(from, pattern_end, out);

                if class_index != 0 {
                    pattern = *from;

                    has_prev_c = FALSE;
                    prev_c = 0;

                    if is_negative == 0
                        && convert_glob_char_in_class(class_index, separator) != 0
                    {
                        separator_seen = TRUE;
                    }
                    continue;
                }
            } else if c == CHAR_MINUS
                && has_prev_c != 0
                && *pattern as u32 != CHAR_RIGHT_SQUARE_BRACKET
            {
                convert_glob_write(out, CHAR_MINUS as PCRE2_UCHAR);

                char_start = pattern;
                c = getcharinctest(&mut pattern, true);

                if pattern >= pattern_end {
                    break;
                }

                if escape as u32 != 0 && c == escape as u32 {
                    char_start = pattern;
                    c = getcharinctest(&mut pattern, true);
                } else if c == CHAR_LEFT_SQUARE_BRACKET && *pattern as u32 == CHAR_COLON {
                    *from = pattern;
                    return PCRE2_ERROR_CONVERT_SYNTAX;
                }

                if prev_c > c {
                    *from = pattern;
                    return PCRE2_ERROR_CONVERT_SYNTAX;
                }

                if prev_c < separator as u32 && (separator as u32) < c {
                    separator_seen = TRUE;
                }

                has_prev_c = FALSE;
                prev_c = 0;
            } else {
                if escape as u32 != 0 && c == escape as u32 {
                    char_start = pattern;
                    c = getcharinctest(&mut pattern, true);

                    if pattern >= pattern_end {
                        break;
                    }
                }

                has_prev_c = TRUE;
                prev_c = c;
            }

            if c == CHAR_LEFT_SQUARE_BRACKET
                || c == CHAR_RIGHT_SQUARE_BRACKET
                || c == CHAR_BACKSLASH
                || c == CHAR_MINUS
            {
                convert_glob_write(out, CHAR_BACKSLASH as PCRE2_UCHAR);
            }

            if c == separator as u32 {
                separator_seen = TRUE;
            }

            loop {
                convert_glob_write(out, *char_start);
                char_start = char_start.add(1);
                if char_start >= pattern {
                    break;
                }
            }
        }

        *from = pattern;
        PCRE2_ERROR_MISSING_SQUARE_BRACKET
    }
}

/* Prints a (*COMMIT) into the output. */
unsafe fn convert_glob_print_commit(out: *mut pcre2_output_context) {
    unsafe {
        (*out).out_str[0] = CHAR_LEFT_PARENTHESIS as u8;
        (*out).out_str[1] = CHAR_ASTERISK as u8;
        (*out).out_str[2] = CHAR_C as u8;
        (*out).out_str[3] = CHAR_O as u8;
        (*out).out_str[4] = CHAR_M as u8;
        (*out).out_str[5] = CHAR_M as u8;
        (*out).out_str[6] = CHAR_I as u8;
        (*out).out_str[7] = CHAR_T as u8;
        convert_glob_write_str(out, 8);
        convert_glob_write(out, CHAR_RIGHT_PARENTHESIS as PCRE2_UCHAR);
    }
}

/* Bash glob converter.

Returns:         0 => success
                !0 => error code
*/
unsafe fn convert_glob(
    options: u32,
    pattern_in: PCRE2_SPTR,
    plength: PCRE2_SIZE,
    utf: BOOL,
    use_buffer: *mut PCRE2_UCHAR,
    use_length: PCRE2_SIZE,
    bufflenptr: *mut PCRE2_SIZE,
    dummyrun: BOOL,
    ccontext: *mut pcre2_real_convert_context,
) -> c_int {
    unsafe {
        let mut out = pcre2_output_context {
            output: core::ptr::null_mut(),
            output_end: core::ptr::null(),
            output_size: 0,
            out_str: [0u8; 8],
        };
        let pattern_start: PCRE2_SPTR = pattern_in;
        let mut pattern: PCRE2_SPTR = pattern_in;
        let pattern_end: PCRE2_SPTR = pattern_in.add(plength);
        let separator: PCRE2_UCHAR = (*ccontext).glob_separator as PCRE2_UCHAR;
        let escape: PCRE2_UCHAR = (*ccontext).glob_escape as PCRE2_UCHAR;
        let mut c: PCRE2_UCHAR;
        let no_wildsep: BOOL =
            ((options & PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR) != 0) as BOOL;
        let no_starstar: BOOL = ((options & PCRE2_CONVERT_GLOB_NO_STARSTAR) != 0) as BOOL;
        let mut in_atomic: BOOL = FALSE;
        let mut after_starstar: BOOL = FALSE;
        let mut no_slash_z: BOOL = FALSE;
        let with_escape: BOOL;
        let mut is_start: BOOL;
        let mut after_separator: BOOL;
        let mut result: c_int = 0;

        /* SUPPORT_UNICODE is defined. */
        if utf != 0 && (separator as u32 >= 128 || escape as u32 >= 128) {
            /* Currently only ASCII characters are supported. */
            *bufflenptr = 0;
            return PCRE2_ERROR_CONVERT_SYNTAX;
        }

        with_escape = (!strchr(
            PCRE2_ESCAPED_LITERALS_C.as_ptr() as *const c_char,
            separator as c_int,
        )
        .is_null()) as BOOL;

        /* Initialize default for error offset as end of input. */
        out.output = use_buffer;
        out.output_end = use_buffer.add(use_length);
        out.output_size = 0;

        out.out_str[0] = CHAR_LEFT_PARENTHESIS as u8;
        out.out_str[1] = CHAR_QUESTION_MARK as u8;
        out.out_str[2] = CHAR_s as u8;
        out.out_str[3] = CHAR_RIGHT_PARENTHESIS as u8;
        convert_glob_write_str(&mut out, 4);

        is_start = TRUE;

        if pattern < pattern_end && *pattern.add(0) as u32 == CHAR_ASTERISK {
            if no_wildsep != 0 {
                is_start = FALSE;
            } else if no_starstar == 0
                && pattern.add(1) < pattern_end
                && *pattern.add(1) as u32 == CHAR_ASTERISK
            {
                is_start = FALSE;
            }
        }

        if is_start != 0 {
            out.out_str[0] = CHAR_BACKSLASH as u8;
            out.out_str[1] = CHAR_A as u8;
            convert_glob_write_str(&mut out, 2);
        }

        while pattern < pattern_end {
            c = *pattern;
            pattern = pattern.add(1);

            if c as u32 == CHAR_ASTERISK {
                is_start = (pattern == pattern_start.add(1)) as BOOL;

                if in_atomic != 0 {
                    convert_glob_write(&mut out, CHAR_RIGHT_PARENTHESIS as PCRE2_UCHAR);
                    in_atomic = FALSE;
                }

                if no_starstar == 0 && pattern < pattern_end && *pattern as u32 == CHAR_ASTERISK
                {
                    after_separator =
                        (is_start != 0 || (*pattern.sub(2) == separator)) as BOOL;

                    loop {
                        pattern = pattern.add(1);
                        if !(pattern < pattern_end && *pattern as u32 == CHAR_ASTERISK) {
                            break;
                        }
                    }

                    if pattern >= pattern_end {
                        no_slash_z = TRUE;
                        break;
                    }

                    after_starstar = TRUE;

                    if after_separator != 0
                        && escape as u32 != 0
                        && *pattern == escape
                        && pattern.add(1) < pattern_end
                        && *pattern.add(1) == separator
                    {
                        pattern = pattern.add(1);
                    }

                    if is_start != 0 {
                        if *pattern != separator {
                            continue;
                        }

                        out.out_str[0] = CHAR_LEFT_PARENTHESIS as u8;
                        out.out_str[1] = CHAR_QUESTION_MARK as u8;
                        out.out_str[2] = CHAR_COLON as u8;
                        out.out_str[3] = CHAR_BACKSLASH as u8;
                        out.out_str[4] = CHAR_A as u8;
                        out.out_str[5] = CHAR_VERTICAL_LINE as u8;
                        convert_glob_write_str(&mut out, 6);

                        convert_glob_print_separator(&mut out, separator, with_escape);
                        convert_glob_write(&mut out, CHAR_RIGHT_PARENTHESIS as PCRE2_UCHAR);

                        pattern = pattern.add(1);
                        continue;
                    }

                    convert_glob_print_commit(&mut out);

                    if after_separator == 0 || *pattern != separator {
                        out.out_str[0] = CHAR_DOT as u8;
                        out.out_str[1] = CHAR_ASTERISK as u8;
                        out.out_str[2] = CHAR_QUESTION_MARK as u8;
                        convert_glob_write_str(&mut out, 3);
                        continue;
                    }

                    out.out_str[0] = CHAR_LEFT_PARENTHESIS as u8;
                    out.out_str[1] = CHAR_QUESTION_MARK as u8;
                    out.out_str[2] = CHAR_COLON as u8;
                    out.out_str[3] = CHAR_DOT as u8;
                    out.out_str[4] = CHAR_ASTERISK as u8;
                    out.out_str[5] = CHAR_QUESTION_MARK as u8;

                    convert_glob_write_str(&mut out, 6);

                    convert_glob_print_separator(&mut out, separator, with_escape);

                    out.out_str[0] = CHAR_RIGHT_PARENTHESIS as u8;
                    out.out_str[1] = CHAR_QUESTION_MARK as u8;
                    out.out_str[2] = CHAR_QUESTION_MARK as u8;
                    convert_glob_write_str(&mut out, 3);

                    pattern = pattern.add(1);
                    continue;
                }

                if pattern < pattern_end && *pattern as u32 == CHAR_ASTERISK {
                    loop {
                        pattern = pattern.add(1);
                        if !(pattern < pattern_end && *pattern as u32 == CHAR_ASTERISK) {
                            break;
                        }
                    }
                }

                if no_wildsep != 0 {
                    if pattern >= pattern_end {
                        no_slash_z = TRUE;
                        break;
                    }

                    /* Start check must be after the end check. */
                    if is_start != 0 {
                        continue;
                    }
                }

                if is_start == 0 {
                    if after_starstar != 0 {
                        out.out_str[0] = CHAR_LEFT_PARENTHESIS as u8;
                        out.out_str[1] = CHAR_QUESTION_MARK as u8;
                        out.out_str[2] = CHAR_GREATER_THAN_SIGN as u8;
                        convert_glob_write_str(&mut out, 3);
                        in_atomic = TRUE;
                    } else {
                        convert_glob_print_commit(&mut out);
                    }
                }

                if no_wildsep != 0 {
                    convert_glob_write(&mut out, CHAR_DOT as PCRE2_UCHAR);
                } else {
                    convert_glob_print_wildcard(&mut out, separator, with_escape);
                }

                out.out_str[0] = CHAR_ASTERISK as u8;
                out.out_str[1] = CHAR_QUESTION_MARK as u8;
                if pattern >= pattern_end {
                    out.out_str[1] = CHAR_PLUS as u8;
                }
                convert_glob_write_str(&mut out, 2);
                continue;
            }

            if c as u32 == CHAR_QUESTION_MARK {
                if no_wildsep != 0 {
                    convert_glob_write(&mut out, CHAR_DOT as PCRE2_UCHAR);
                } else {
                    convert_glob_print_wildcard(&mut out, separator, with_escape);
                }
                continue;
            }

            if c as u32 == CHAR_LEFT_SQUARE_BRACKET {
                result = convert_glob_parse_range(
                    &mut pattern,
                    pattern_end,
                    &mut out,
                    utf,
                    separator,
                    with_escape,
                    escape,
                    no_wildsep,
                );
                if result != 0 {
                    break;
                }
                continue;
            }

            if escape as u32 != 0 && c == escape {
                if pattern >= pattern_end {
                    result = PCRE2_ERROR_CONVERT_SYNTAX;
                    break;
                }
                c = *pattern;
                pattern = pattern.add(1);
            }

            if (c as u32) < 255
                && !strchr(
                    PCRE2_ESCAPED_LITERALS_C.as_ptr() as *const c_char,
                    c as c_int,
                )
                .is_null()
            {
                convert_glob_write(&mut out, CHAR_BACKSLASH as PCRE2_UCHAR);
            }

            convert_glob_write(&mut out, c);
        }

        if result == 0 {
            if no_slash_z == 0 {
                out.out_str[0] = CHAR_BACKSLASH as u8;
                out.out_str[1] = CHAR_z as u8;
                convert_glob_write_str(&mut out, 2);
            }

            if in_atomic != 0 {
                convert_glob_write(&mut out, CHAR_RIGHT_PARENTHESIS as PCRE2_UCHAR);
            }

            convert_glob_write(&mut out, CHAR_NUL as PCRE2_UCHAR);

            if dummyrun == 0
                && out.output_size != (out.output.offset_from(use_buffer) as PCRE2_SIZE)
            {
                result = PCRE2_ERROR_NOMEMORY;
            }
        }

        if result != 0 {
            *bufflenptr = pattern.offset_from(pattern_start) as PCRE2_SIZE;
            return result;
        }

        *bufflenptr = out.output_size - 1;
        0
    }
}

/* ------------------------- Public entry points ------------------------- */

/* This is the external-facing function for converting other forms of pattern
into PCRE2 regular expression patterns. On error, the bufflenptr argument is
used to return an offset in the original pattern.

Returns:      0 for success, else an error code (+ve or -ve)
*/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_pattern_convert_8(
    mut pattern: PCRE2_SPTR,
    mut plength: PCRE2_SIZE,
    options: u32,
    buffptr: *mut *mut PCRE2_UCHAR,
    bufflenptr: *mut PCRE2_SIZE,
    mut ccontext: *mut pcre2_real_convert_context,
) -> c_int {
    unsafe {
        let mut rc: c_int;
        let null_str: [PCRE2_UCHAR; 1] = [0xcd];
        let mut dummy_buffer: [PCRE2_UCHAR; DUMMY_BUFFER_SIZE] = [0; DUMMY_BUFFER_SIZE];
        let mut use_buffer: *mut PCRE2_UCHAR = dummy_buffer.as_mut_ptr();
        let mut use_length: PCRE2_SIZE = DUMMY_BUFFER_SIZE;
        let utf: BOOL = ((options & PCRE2_CONVERT_UTF) != 0) as BOOL;
        let pattype: u32 = options & TYPE_OPTIONS;

        if pattern.is_null() && plength == 0 {
            pattern = null_str.as_ptr();
        }

        if pattern.is_null() || bufflenptr.is_null() {
            if !bufflenptr.is_null() {
                *bufflenptr = 0; /* Error offset */
            }
            return PCRE2_ERROR_NULL;
        }

        if (options & !ALL_OPTIONS) != 0                       /* Undefined bit set */
            || (pattype & (!pattype).wrapping_add(1)) != pattype /* More than one type set */
            || pattype == 0
        /* No type set */
        {
            *bufflenptr = 0; /* Error offset */
            return PCRE2_ERROR_BADOPTION;
        }

        if plength == PCRE2_ZERO_TERMINATED {
            plength = string_utils::strlen(pattern);
        }
        if ccontext.is_null() {
            ccontext = &raw const _pcre2_default_convert_context_8
                as *mut pcre2_real_convert_context;
        }

        /* Check UTF if required. SUPPORT_UNICODE is defined. */

        if utf != 0 && (options & PCRE2_CONVERT_NO_UTF_CHECK) == 0 {
            let mut erroroffset: PCRE2_SIZE = 0;
            rc = crate::valid_utf::valid_utf(pattern, plength, &mut erroroffset);
            if rc != 0 {
                *bufflenptr = erroroffset;
                return rc;
            }
        }

        /* If buffptr is not NULL, and what it points to is not NULL, we are being
        provided with a buffer and a length, so set them as the buffer to use. */

        if !buffptr.is_null() && !(*buffptr).is_null() {
            use_buffer = *buffptr;
            use_length = *bufflenptr;
        }

        /* Call an individual converter, either just once (if a buffer was provided or
        just the length is needed), or twice (if a memory allocation is required). */

        let mut i = 0;
        while i < 2 {
            let allocated: *mut PCRE2_UCHAR;
            let dummyrun: BOOL = (buffptr.is_null() || (*buffptr).is_null()) as BOOL;

            match pattype {
                PCRE2_CONVERT_GLOB => {
                    rc = convert_glob(
                        options & !PCRE2_CONVERT_GLOB,
                        pattern,
                        plength,
                        utf,
                        use_buffer,
                        use_length,
                        bufflenptr,
                        dummyrun,
                        ccontext,
                    );
                }

                PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_POSIX_EXTENDED => {
                    rc = convert_posix(
                        pattype,
                        pattern,
                        plength,
                        utf,
                        use_buffer,
                        use_length,
                        bufflenptr,
                        dummyrun,
                        ccontext,
                    );
                }

                /* We have already validated pattype. */
                _ => {
                    *bufflenptr = 0; /* Error offset */
                    return PCRE2_ERROR_INTERNAL;
                }
            }

            if rc != 0                     /* Error */
                || buffptr.is_null()       /* Just the length is required */
                || !(*buffptr).is_null()
            /* Buffer was provided or allocated */
            {
                return rc;
            }

            /* Allocate memory for the buffer, with hidden space for an allocator at
            the start. The next time round the loop runs the conversion for real. */

            allocated = memctl_malloc(
                core::mem::size_of::<pcre2_memctl>()
                    + (*bufflenptr + 1) * (PCRE2_CODE_UNIT_WIDTH as usize),
                ccontext as *mut pcre2_memctl,
            ) as *mut PCRE2_UCHAR;
            if allocated.is_null() {
                *bufflenptr = 0; /* Error offset */
                return PCRE2_ERROR_NOMEMORY;
            }
            *buffptr = (allocated as *mut c_char)
                .add(core::mem::size_of::<pcre2_memctl>())
                as *mut PCRE2_UCHAR;

            use_buffer = *buffptr;
            use_length = *bufflenptr + 1;

            i += 1;
        }

        /* Running the loop above ought to have succeeded the second time. */
        *bufflenptr = 0; /* Error offset */
        PCRE2_ERROR_INTERNAL
    }
}

/* This frees a converted pattern that was put in newly-allocated memory.

Argument:   the converted pattern
Returns:    nothing
*/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_converted_pattern_free_8(converted: *mut PCRE2_UCHAR) {
    unsafe {
        if !converted.is_null() {
            let memctl = (converted as *mut c_char)
                .sub(core::mem::size_of::<pcre2_memctl>())
                as *mut pcre2_memctl;
            ((*memctl).free.unwrap())(memctl as *mut c_void, (*memctl).memory_data);
        }
    }
}

/* End of pcre2_convert.rs */
