//! Translated from pcre2_convert.c.
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use crate::consts::*;
use crate::types::*;
use crate::macros::*;
use core::ffi::{c_char, c_void};

use crate::chartables::_pcre2_default_tables_8;
use crate::context::{_pcre2_default_convert_context_8, _pcre2_memctl_malloc_8};
use crate::ord2utf::_pcre2_ord2utf_8;
use crate::string_utils::*;
use crate::valid_utf::_pcre2_valid_utf_8;

/* #define TYPE_OPTIONS (PCRE2_CONVERT_GLOB|
     PCRE2_CONVERT_POSIX_BASIC|PCRE2_CONVERT_POSIX_EXTENDED) */
const TYPE_OPTIONS: u32 =
    PCRE2_CONVERT_GLOB | PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_POSIX_EXTENDED;

/* #define ALL_OPTIONS (PCRE2_CONVERT_UTF|PCRE2_CONVERT_NO_UTF_CHECK|
     PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR|
     PCRE2_CONVERT_GLOB_NO_STARSTAR|
     TYPE_OPTIONS) */
const ALL_OPTIONS: u32 = PCRE2_CONVERT_UTF
    | PCRE2_CONVERT_NO_UTF_CHECK
    | PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR
    | PCRE2_CONVERT_GLOB_NO_STARSTAR
    | TYPE_OPTIONS;

const DUMMY_BUFFER_SIZE: usize = 100;

/* Generated pattern fragments */

/* STR_BACKSLASH STR_A */
const STR_BACKSLASH_A: &[u8] = b"\\A\0";
/* STR_BACKSLASH STR_z */
const STR_BACKSLASH_z: &[u8] = b"\\z\0";
/* STR_COLON STR_RIGHT_SQUARE_BRACKET */
const STR_COLON_RIGHT_SQUARE_BRACKET: &[u8] = b":]\0";
/* STR_DOT STR_ASTERISK STR_LEFT_PARENTHESIS STR_QUESTION_MARK STR_LESS_THAN_SIGN
STR_EQUALS_SIGN */
const STR_DOT_STAR_LOOKBEHIND: &[u8] = b".*(?<=\0";
/* STR_LEFT_PARENTHESIS STR_QUESTION_MARK STR_EXCLAMATION_MARK STR_BACKSLASH STR_DOT
STR_RIGHT_PARENTHESIS */
const STR_LOOKAHEAD_NOT_DOT: &[u8] = b"(?!\\.)\0";
/* STR_LEFT_PARENTHESIS STR_QUESTION_MARK STR_s STR_RIGHT_PARENTHESIS */
const STR_QUERY_s: &[u8] = b"(?s)\0";
/* STR_LEFT_PARENTHESIS STR_ASTERISK STR_N STR_U STR_L STR_RIGHT_PARENTHESIS */
const STR_STAR_NUL: &[u8] = b"(*NUL)\0";

/* Single-character fragments used with PUTCHARS. */

const STR_BACKSLASH: &[u8] = b"\\\0";
const STR_CIRCUMFLEX_ACCENT: &[u8] = b"^\0";
const STR_LEFT_SQUARE_BRACKET: &[u8] = b"[\0";
const STR_RIGHT_SQUARE_BRACKET: &[u8] = b"]\0";

/* States for POSIX processing */

const POSIX_START_REGEX: u32 = 0;
const POSIX_ANCHORED: u32 = 1;
const POSIX_NOT_BRACKET: u32 = 2;
const POSIX_CLASS_NOT_STARTED: u32 = 3;
const POSIX_CLASS_STARTING: u32 = 4;
const POSIX_CLASS_STARTED: u32 = 5;

/* Macro to add a character string to the output buffer, checking for overflow.
The Rust version takes `p` and `endp` explicitly because of macro hygiene. */

macro_rules! PUTCHARS {
    ($string:expr, $p:expr, $endp:expr) => {{
        let mut s: *const c_char = ($string).as_ptr() as *const c_char;
        while *s != 0 {
            if $p >= $endp {
                return PCRE2_ERROR_NOMEMORY;
            }
            *$p = *s as u8;
            $p = $p.add(1);
            s = s.add(1);
        }
    }};
}

/* Macro to check for lowercase characters. (Non-EBCDIC version.) */

macro_rules! ISLOWER {
    ($c:expr) => {
        ($c) >= b'a' as u32 && ($c) <= b'z' as u32
    };
}

/* A local clone of C's strchr() for the two static strings below. Note that, as
in C, a search for a zero byte succeeds and returns a pointer to the terminator. */

pub(crate) unsafe fn strchr(s: *const c_char, c: i32) -> *const c_char {
    let mut s = s;
    let ch = c as c_char;
    loop {
        if *s == ch {
            return s;
        }
        if *s == 0 {
            return core::ptr::null();
        }
        s = s.add(1);
    }
}

/* Literals that must be escaped: \ ? * + | . ^ $ { } [ ] ( ) */

static pcre2_escaped_literals: [u8; 15] = *b"\\?*+|.^${}[]()\0";

/* Recognized escaped metacharacters in POSIX basic patterns. */

static posix_meta_escapes: [u8; 14] = *b"(){}123456789\0";

/* Recognized POSIX classes, colon-separated. */

static posix_classes: [u8; 85] =
    *b"alpha:lower:upper:alnum:ascii:blank:cntrl:digit:graph:print:punct:space:word:xdigit:\0";

/*************************************************
*           Convert a POSIX pattern              *
*************************************************/

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

pub(crate) unsafe fn convert_posix(
    pattype: u32,
    pattern: PCRE2_SPTR,
    plength: PCRE2_SIZE,
    utf: BOOL,
    use_buffer: *mut PCRE2_UCHAR,
    use_length: PCRE2_SIZE,
    bufflenptr: *mut PCRE2_SIZE,
    dummyrun: BOOL,
    ccontext: *mut pcre2_real_convert_context,
) -> i32 {
    let mut plength: PCRE2_SIZE = plength;

    let mut posix: PCRE2_SPTR = pattern;
    let mut p: *mut PCRE2_UCHAR = use_buffer;
    let mut pp: *mut PCRE2_UCHAR = p;
    let endp: *mut PCRE2_UCHAR = p.wrapping_add(use_length).wrapping_sub(1); /* Allow for trailing zero */
    let mut convlength: PCRE2_SIZE = 0;

    let mut bracount: u32 = 0;
    let mut posix_state: u32 = POSIX_START_REGEX;
    let mut lastspecial: u32 = 0;
    let extended: BOOL = ((pattype & PCRE2_CONVERT_POSIX_EXTENDED) != 0) as BOOL;
    let mut nextisliteral: BOOL = FALSE;

    /* (void)utf;       Not used when Unicode not supported */
    /* (void)ccontext;  Not currently used */

    /* Initialize default for error offset as end of input. */

    *bufflenptr = plength;
    PUTCHARS!(STR_STAR_NUL, p, endp);

    /* Now scan the input. */

    'main: while plength > 0 {
        let mut c: u32;
        let mut sc: u32;
        let mut clength: i32 = 1;

        /* Add in the length of the last item, then, if in the dummy run, pull the
        pointer back to the start of the (temporary) buffer and then remember the
        start of the next item. */

        convlength = convlength.wrapping_add(p.offset_from(pp) as usize);
        if dummyrun != 0 {
            p = use_buffer;
        }
        pp = p;

        /* Pick up the next character */

        GETCHARLENTEST!(c, posix, clength, utf);
        posix = posix.add(clength as usize);
        plength -= clength as usize;

        sc = if nextisliteral != 0 { 0 } else { c };
        nextisliteral = FALSE;

        /* Handle a character within a class. */

        if posix_state >= POSIX_CLASS_NOT_STARTED {
            if c == b']' as u32 {
                PUTCHARS!(STR_RIGHT_SQUARE_BRACKET, p, endp);
                posix_state = POSIX_NOT_BRACKET;
            }
            /* Not the end of the class */
            else {
                /* switch (posix_state) */
                'sw_state: {
                    if posix_state == POSIX_CLASS_STARTED {
                        if ISLOWER!(c) {
                            break 'sw_state; /* Remain in started state */
                        }
                        posix_state = POSIX_CLASS_NOT_STARTED;
                        if c == b':' as u32 && plength > 0 && *posix == b']' {
                            PUTCHARS!(STR_COLON_RIGHT_SQUARE_BRACKET, p, endp);
                            plength -= 1;
                            posix = posix.add(1);
                            continue 'main; /* With next character after :] */
                        }
                        /* Fall through to POSIX_CLASS_NOT_STARTED */
                        if c == b'[' as u32 {
                            posix_state = POSIX_CLASS_STARTING;
                        }
                        break 'sw_state;
                    }

                    if posix_state == POSIX_CLASS_NOT_STARTED {
                        if c == b'[' as u32 {
                            posix_state = POSIX_CLASS_STARTING;
                        }
                        break 'sw_state;
                    }

                    if posix_state == POSIX_CLASS_STARTING {
                        if c == b':' as u32 {
                            posix_state = POSIX_CLASS_STARTED;
                        }
                        break 'sw_state;
                    }
                }

                if c == b'\\' as u32 {
                    PUTCHARS!(STR_BACKSLASH, p, endp);
                }
                if p.wrapping_add(clength as usize) > endp {
                    return PCRE2_ERROR_NOMEMORY;
                }
                core::ptr::copy_nonoverlapping(
                    posix.wrapping_sub(clength as usize),
                    p,
                    clength as usize,
                );
                p = p.add(clength as usize);
            }
        }
        /* Handle a character not within a class. */
        else {
            /* switch(sc) -- emulated with a small state machine because of the
            COPY_SPECIAL and ESCAPE_LITERAL labels in the middle. */

            const L_DISPATCH: u32 = 0;
            const L_CASE_LEFT_PARENTHESIS: u32 = 1;
            const L_CASE_QUESTION_MARK: u32 = 2;
            const L_CASE_DOT: u32 = 3;
            const L_COPY_SPECIAL: u32 = 4;
            const L_DEFAULT: u32 = 5;
            const L_ESCAPE_LITERAL: u32 = 6;
            const L_DEFAULT_REST: u32 = 7;

            let mut state: u32 = L_DISPATCH;
            'sw: loop {
                match state {
                    L_DISPATCH => {
                        match sc {
                            /* case CHAR_LEFT_SQUARE_BRACKET */
                            0x5b => {
                                PUTCHARS!(STR_LEFT_SQUARE_BRACKET, p, endp);

                                /* Handle start of "normal" character classes */

                                posix_state = POSIX_CLASS_NOT_STARTED;

                                /* Handle ^ and ] as first characters */

                                if plength > 0 {
                                    if *posix == b'^' {
                                        posix = posix.add(1);
                                        plength -= 1;
                                        PUTCHARS!(STR_CIRCUMFLEX_ACCENT, p, endp);
                                    }
                                    if plength > 0 && *posix == b']' {
                                        posix = posix.add(1);
                                        plength -= 1;
                                        PUTCHARS!(STR_RIGHT_SQUARE_BRACKET, p, endp);
                                    }
                                }
                                break 'sw;
                            }

                            /* case CHAR_BACKSLASH */
                            0x5c => {
                                if plength == 0 {
                                    return PCRE2_ERROR_END_BACKSLASH;
                                }
                                if extended != 0 {
                                    nextisliteral = TRUE;
                                } else {
                                    if (*posix as u32) < 255
                                        && !strchr(
                                            posix_meta_escapes.as_ptr() as *const c_char,
                                            *posix as i32,
                                        )
                                        .is_null()
                                    {
                                        if *posix >= b'0' && *posix <= b'9' {
                                            PUTCHARS!(STR_BACKSLASH, p, endp);
                                        }
                                        if p.wrapping_add(1) > endp {
                                            return PCRE2_ERROR_NOMEMORY;
                                        }
                                        /* lastspecial = *p++ = *posix++; */
                                        let t: PCRE2_UCHAR = *posix;
                                        posix = posix.add(1);
                                        *p = t;
                                        p = p.add(1);
                                        lastspecial = t as u32;
                                        plength -= 1;
                                    } else {
                                        nextisliteral = TRUE;
                                    }
                                }
                                break 'sw;
                            }

                            /* case CHAR_RIGHT_PARENTHESIS */
                            0x29 => {
                                if extended == 0 || bracount == 0 {
                                    /* goto ESCAPE_LITERAL */
                                    state = L_ESCAPE_LITERAL;
                                    continue 'sw;
                                }
                                bracount -= 1;
                                /* goto COPY_SPECIAL */
                                state = L_COPY_SPECIAL;
                                continue 'sw;
                            }

                            /* case CHAR_LEFT_PARENTHESIS */
                            0x28 => {
                                state = L_CASE_LEFT_PARENTHESIS;
                                continue 'sw;
                            }

                            /* case CHAR_QUESTION_MARK, CHAR_PLUS, CHAR_LEFT_CURLY_BRACKET,
                            CHAR_RIGHT_CURLY_BRACKET, CHAR_VERTICAL_LINE */
                            0x3f | 0x2b | 0x7b | 0x7d | 0x7c => {
                                state = L_CASE_QUESTION_MARK;
                                continue 'sw;
                            }

                            /* case CHAR_DOT, CHAR_DOLLAR_SIGN */
                            0x2e | 0x24 => {
                                state = L_CASE_DOT;
                                continue 'sw;
                            }

                            /* case CHAR_ASTERISK */
                            0x2a => {
                                if lastspecial != b'*' as u32 {
                                    if extended == 0
                                        && (posix_state < POSIX_NOT_BRACKET
                                            || lastspecial == b'(' as u32)
                                    {
                                        /* goto ESCAPE_LITERAL */
                                        state = L_ESCAPE_LITERAL;
                                        continue 'sw;
                                    }
                                    /* goto COPY_SPECIAL */
                                    state = L_COPY_SPECIAL;
                                    continue 'sw;
                                }
                                break 'sw; /* Ignore second and subsequent asterisks */
                            }

                            /* case CHAR_CIRCUMFLEX_ACCENT */
                            0x5e => {
                                if extended != 0 {
                                    /* goto COPY_SPECIAL */
                                    state = L_COPY_SPECIAL;
                                    continue 'sw;
                                }
                                if posix_state == POSIX_START_REGEX
                                    || lastspecial == b'(' as u32
                                {
                                    posix_state = POSIX_ANCHORED;
                                    /* goto COPY_SPECIAL */
                                    state = L_COPY_SPECIAL;
                                    continue 'sw;
                                }
                                /* Fall through to default */
                                state = L_DEFAULT;
                                continue 'sw;
                            }

                            /* default */
                            _ => {
                                state = L_DEFAULT;
                                continue 'sw;
                            }
                        }
                    }

                    L_CASE_LEFT_PARENTHESIS => {
                        bracount += 1;
                        /* Fall through */
                        state = L_CASE_QUESTION_MARK;
                        continue 'sw;
                    }

                    L_CASE_QUESTION_MARK => {
                        if extended == 0 {
                            /* goto ESCAPE_LITERAL */
                            state = L_ESCAPE_LITERAL;
                            continue 'sw;
                        }
                        /* Fall through */
                        state = L_CASE_DOT;
                        continue 'sw;
                    }

                    L_CASE_DOT => {
                        posix_state = POSIX_NOT_BRACKET;
                        /* COPY_SPECIAL: */
                        state = L_COPY_SPECIAL;
                        continue 'sw;
                    }

                    L_COPY_SPECIAL => {
                        lastspecial = c;
                        if p.wrapping_add(1) > endp {
                            return PCRE2_ERROR_NOMEMORY;
                        }
                        *p = c as PCRE2_UCHAR;
                        p = p.add(1);
                        break 'sw;
                    }

                    L_DEFAULT => {
                        if c < 255
                            && !strchr(
                                pcre2_escaped_literals.as_ptr() as *const c_char,
                                c as i32,
                            )
                            .is_null()
                        {
                            /* ESCAPE_LITERAL: */
                            state = L_ESCAPE_LITERAL;
                            continue 'sw;
                        }
                        state = L_DEFAULT_REST;
                        continue 'sw;
                    }

                    L_ESCAPE_LITERAL => {
                        PUTCHARS!(STR_BACKSLASH, p, endp);
                        state = L_DEFAULT_REST;
                        continue 'sw;
                    }

                    L_DEFAULT_REST => {
                        lastspecial = 0xff; /* Indicates nothing special */
                        if p.wrapping_add(clength as usize) > endp {
                            return PCRE2_ERROR_NOMEMORY;
                        }
                        core::ptr::copy_nonoverlapping(
                            posix.wrapping_sub(clength as usize),
                            p,
                            clength as usize,
                        );
                        p = p.add(clength as usize);
                        posix_state = POSIX_NOT_BRACKET;
                        break 'sw;
                    }

                    _ => {
                        break 'sw;
                    }
                }
            }
        }
    }

    if posix_state >= POSIX_CLASS_NOT_STARTED {
        return PCRE2_ERROR_MISSING_SQUARE_BRACKET;
    }
    convlength = convlength.wrapping_add(p.offset_from(pp) as usize); /* Final segment */
    *bufflenptr = convlength;
    *p = 0;
    p = p.add(1);
    0
}

/*************************************************
*           Convert a glob pattern               *
*************************************************/

/* Context for writing the output into a buffer. */

#[repr(C)]
pub(crate) struct pcre2_output_context {
    pub output: *mut PCRE2_UCHAR,   /* current output position */
    pub output_end: PCRE2_SPTR,     /* output end */
    pub output_size: PCRE2_SIZE,    /* size of the output */
    pub out_str: [u8; 8],           /* string copied to the output */
}

/* Write a character into the output.

Arguments:
  out            output context
  chr            the next character
*/

pub(crate) unsafe fn convert_glob_write(out: *mut pcre2_output_context, chr: PCRE2_UCHAR) {
    (*out).output_size += 1;

    if ((*out).output as PCRE2_SPTR) < (*out).output_end {
        *(*out).output = chr;
        (*out).output = (*out).output.add(1);
    }
}

/* Write a string into the output.

Arguments:
  out            output context
  length         length of out->out_str
*/

pub(crate) unsafe fn convert_glob_write_str(out: *mut pcre2_output_context, length: PCRE2_SIZE) {
    let mut length: PCRE2_SIZE = length;
    let mut out_str: *mut u8 = (*out).out_str.as_mut_ptr();
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

        length = length.wrapping_sub(1);
        if length == 0 {
            break;
        }
    }

    (*out).output = output;
    (*out).output_size = output_size;
}

/* Prints the separator into the output.

Arguments:
  out            output context
  separator      glob separator
  with_escape    backslash is needed before separator
*/

pub(crate) unsafe fn convert_glob_print_separator(
    out: *mut pcre2_output_context,
    separator: PCRE2_UCHAR,
    with_escape: BOOL,
) {
    if with_escape != 0 {
        convert_glob_write(out, b'\\');
    }

    convert_glob_write(out, separator);
}

/* Prints a wildcard into the output.

Arguments:
  out            output context
  separator      glob separator
  with_escape    backslash is needed before separator
*/

pub(crate) unsafe fn convert_glob_print_wildcard(
    out: *mut pcre2_output_context,
    separator: PCRE2_UCHAR,
    with_escape: BOOL,
) {
    (*out).out_str[0] = b'[';
    (*out).out_str[1] = b'^';
    convert_glob_write_str(out, 2);

    convert_glob_print_separator(out, separator, with_escape);

    convert_glob_write(out, b']');
}

/* Parse a posix class.

Arguments:
  from           starting point of scanning the range
  pattern_end    end of pattern
  out            output context

Returns:  >0 => class index
          0  => malformed class
*/

pub(crate) unsafe fn convert_glob_parse_class(
    from: *mut PCRE2_SPTR,
    pattern_end: PCRE2_SPTR,
    out: *mut pcre2_output_context,
) -> i32 {
    let mut start: PCRE2_SPTR = (*from).add(1);
    let mut pattern: PCRE2_SPTR = start;
    let mut class_ptr: *const c_char;
    let mut c: PCRE2_UCHAR = 0;
    let mut class_index: i32;

    loop {
        if pattern >= pattern_end {
            return 0;
        }

        c = *pattern;
        pattern = pattern.add(1);

        if c < b'a' || c > b'z' {
            break;
        }
    }

    if c != b':' || pattern >= pattern_end || *pattern != b']' {
        return 0;
    }

    class_ptr = posix_classes.as_ptr() as *const c_char;
    class_index = 1;

    loop {
        if *class_ptr == 0 {
            return 0;
        }

        pattern = start;

        while *pattern == (*class_ptr as PCRE2_UCHAR) {
            if *pattern == b':' {
                pattern = pattern.add(2);
                start = start.wrapping_sub(2);

                loop {
                    let t: PCRE2_UCHAR = *start;
                    start = start.add(1);
                    convert_glob_write(out, t);
                    if !(start < pattern) {
                        break;
                    }
                }

                *from = pattern;
                return class_index;
            }
            pattern = pattern.add(1);
            class_ptr = class_ptr.add(1);
        }

        while *class_ptr != b':' as c_char {
            class_ptr = class_ptr.add(1);
        }
        class_ptr = class_ptr.add(1);
        class_index += 1;
    }
}

/* Checks whether the character is in the class.

Arguments:
  class_index    class index
  c              character

Returns:   !0 => character is found in the class
            0 => otherwise
*/

pub(crate) unsafe fn convert_glob_char_in_class(class_index: i32, c: PCRE2_UCHAR) -> BOOL {
    let cbits: *const u8 = _pcre2_default_tables_8.as_ptr().add(cbits_offset);
    let cbit: usize;

    /* See posix_class_maps. This is a small local clone of that.
    Note that we don't know exactly what character tables will be used at
    match time, but, for the purposes of pattern conversion, it should be
    sufficient to use PCRE2's built-in default tables. */

    match class_index {
        /* alpha */
        1 => {
            if c == b'_' {
                return FALSE;
            }
            if (*cbits.add(cbit_digit).add((c / 8) as usize) as u32 & (1u32 << (c & 7))) != 0 {
                return FALSE;
            }
            cbit = cbit_word;
        }

        2 => {
            cbit = cbit_lower;
        } /* lower */
        3 => {
            cbit = cbit_upper;
        } /* upper */

        /* alnum */
        4 => {
            if c == b'_' {
                return FALSE;
            }
            cbit = cbit_word;
        }

        /* ascii */
        5 => {
            if (*cbits.add(cbit_cntrl).add((c / 8) as usize) as u32 & (1u32 << (c & 7))) != 0 {
                return TRUE;
            }
            cbit = cbit_print;
        }

        /* blank */
        6 => {
            if c == 0x0a /* CHAR_LF */ || c == 0x0b /* CHAR_VT */
                || c == 0x0c /* CHAR_FF */ || c == 0x0d
            /* CHAR_CR */
            {
                return FALSE;
            }
            cbit = cbit_space;
        }

        7 => {
            cbit = cbit_cntrl;
        } /* cntrl */
        8 => {
            cbit = cbit_digit;
        } /* digit */
        9 => {
            cbit = cbit_graph;
        } /* graph */
        10 => {
            cbit = cbit_print;
        } /* print */
        11 => {
            cbit = cbit_punct;
        } /* punct */
        12 => {
            cbit = cbit_space;
        } /* space */
        13 => {
            cbit = cbit_word;
        } /* word */
        14 => {
            cbit = cbit_xdigit;
        } /* xdigit */
        _ => {
            return FALSE;
        }
    }

    ((*cbits.add(cbit).add((c / 8) as usize) as u32 & (1u32 << (c & 7))) != 0) as BOOL
}

/* Parse a range of characters.

Arguments:
  from           starting point of scanning the range
  pattern_end    end of pattern
  out            output context
  separator      glob separator
  with_escape    backslash is needed before separator

Returns:         0 => success
                !0 => error code
*/

pub(crate) unsafe fn convert_glob_parse_range(
    from: *mut PCRE2_SPTR,
    pattern_end: PCRE2_SPTR,
    out: *mut pcre2_output_context,
    utf: BOOL,
    separator: PCRE2_UCHAR,
    with_escape: BOOL,
    escape: PCRE2_UCHAR,
    no_wildsep: BOOL,
) -> i32 {
    let mut is_negative: BOOL = FALSE;
    let mut separator_seen: BOOL = FALSE;
    let mut has_prev_c: BOOL;
    let mut pattern: PCRE2_SPTR = *from;
    let mut char_start: PCRE2_SPTR = core::ptr::null();
    let mut c: u32 = 0;
    let mut prev_c: u32;
    let mut len: i32;
    let mut class_index: i32;

    /* (void)utf; Avoid compiler warning. */

    if pattern >= pattern_end {
        *from = pattern;
        return PCRE2_ERROR_MISSING_SQUARE_BRACKET;
    }

    if *pattern == b'!' || *pattern == b'^' {
        pattern = pattern.add(1);

        if pattern >= pattern_end {
            *from = pattern;
            return PCRE2_ERROR_MISSING_SQUARE_BRACKET;
        }

        is_negative = TRUE;

        (*out).out_str[0] = b'[';
        (*out).out_str[1] = b'^';
        len = 2;

        if no_wildsep == 0 {
            if with_escape != 0 {
                (*out).out_str[len as usize] = b'\\';
                len += 1;
            }
            (*out).out_str[len as usize] = separator as u8;
        }

        convert_glob_write_str(out, (len + 1) as PCRE2_SIZE);
    } else {
        convert_glob_write(out, b'[');
    }

    has_prev_c = FALSE;
    prev_c = 0;

    if *pattern == b']' {
        (*out).out_str[0] = b'\\';
        (*out).out_str[1] = b']';
        convert_glob_write_str(out, 2);
        has_prev_c = TRUE;
        prev_c = b']' as u32;
        pattern = pattern.add(1);
    }

    while pattern < pattern_end {
        char_start = pattern;
        GETCHARINCTEST!(c, pattern, utf);

        if c == b']' as u32 {
            convert_glob_write(out, c as PCRE2_UCHAR);

            if is_negative == 0 && no_wildsep == 0 && separator_seen != 0 {
                (*out).out_str[0] = b'(';
                (*out).out_str[1] = b'?';
                (*out).out_str[2] = b'<';
                (*out).out_str[3] = b'!';
                convert_glob_write_str(out, 4);

                convert_glob_print_separator(out, separator, with_escape);
                convert_glob_write(out, b')');
            }

            *from = pattern;
            return 0;
        }

        if pattern >= pattern_end {
            break;
        }

        if c == b'[' as u32 && *pattern == b':' {
            *from = pattern;
            class_index = convert_glob_parse_class(from, pattern_end, out);

            if class_index != 0 {
                pattern = *from;

                has_prev_c = FALSE;
                prev_c = 0;

                if is_negative == 0 && convert_glob_char_in_class(class_index, separator) != 0 {
                    separator_seen = TRUE;
                }
                continue;
            }
        } else if c == b'-' as u32 && has_prev_c != 0 && *pattern != b']' {
            convert_glob_write(out, b'-');

            char_start = pattern;
            GETCHARINCTEST!(c, pattern, utf);

            if pattern >= pattern_end {
                break;
            }

            if escape != 0 && c == escape as u32 {
                char_start = pattern;
                GETCHARINCTEST!(c, pattern, utf);
            } else if c == b'[' as u32 && *pattern == b':' {
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
            if escape != 0 && c == escape as u32 {
                char_start = pattern;
                GETCHARINCTEST!(c, pattern, utf);

                if pattern >= pattern_end {
                    break;
                }
            }

            has_prev_c = TRUE;
            prev_c = c;
        }

        if c == b'[' as u32 || c == b']' as u32 || c == b'\\' as u32 || c == b'-' as u32 {
            convert_glob_write(out, b'\\');
        }

        if c == separator as u32 {
            separator_seen = TRUE;
        }

        loop {
            let t: PCRE2_UCHAR = *char_start;
            char_start = char_start.add(1);
            convert_glob_write(out, t);
            if !(char_start < pattern) {
                break;
            }
        }
    }

    *from = pattern;
    PCRE2_ERROR_MISSING_SQUARE_BRACKET
}

/* Prints a (*COMMIT) into the output.

Arguments:
  out            output context
*/

pub(crate) unsafe fn convert_glob_print_commit(out: *mut pcre2_output_context) {
    (*out).out_str[0] = b'(';
    (*out).out_str[1] = b'*';
    (*out).out_str[2] = b'C';
    (*out).out_str[3] = b'O';
    (*out).out_str[4] = b'M';
    (*out).out_str[5] = b'M';
    (*out).out_str[6] = b'I';
    (*out).out_str[7] = b'T';
    convert_glob_write_str(out, 8);
    convert_glob_write(out, b')');
}

/* Bash glob converter.

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

pub(crate) unsafe fn convert_glob(
    options: u32,
    pattern: PCRE2_SPTR,
    plength: PCRE2_SIZE,
    utf: BOOL,
    use_buffer: *mut PCRE2_UCHAR,
    use_length: PCRE2_SIZE,
    bufflenptr: *mut PCRE2_SIZE,
    dummyrun: BOOL,
    ccontext: *mut pcre2_real_convert_context,
) -> i32 {
    let mut out = pcre2_output_context {
        output: core::ptr::null_mut(),
        output_end: core::ptr::null(),
        output_size: 0,
        out_str: [0; 8],
    };
    let pattern_start: PCRE2_SPTR = pattern;
    let mut pattern: PCRE2_SPTR = pattern;
    let pattern_end: PCRE2_SPTR = pattern.wrapping_add(plength);
    let separator: PCRE2_UCHAR = (*ccontext).glob_separator as PCRE2_UCHAR;
    let escape: PCRE2_UCHAR = (*ccontext).glob_escape as PCRE2_UCHAR;
    let mut c: PCRE2_UCHAR;
    let no_wildsep: BOOL = ((options & PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR) != 0) as BOOL;
    let no_starstar: BOOL = ((options & PCRE2_CONVERT_GLOB_NO_STARSTAR) != 0) as BOOL;
    let mut in_atomic: BOOL = FALSE;
    let mut after_starstar: BOOL = FALSE;
    let mut no_slash_z: BOOL = FALSE;
    let with_escape: BOOL;
    let mut is_start: BOOL;
    let mut after_separator: BOOL;
    let mut result: i32 = 0;

    /* (void)utf; Avoid compiler warning. */

    if utf != 0 && (separator >= 128 || escape >= 128) {
        /* Currently only ASCII characters are supported. */
        *bufflenptr = 0;
        return PCRE2_ERROR_CONVERT_SYNTAX;
    }

    with_escape = (!strchr(
        pcre2_escaped_literals.as_ptr() as *const c_char,
        separator as i32,
    )
    .is_null()) as BOOL;

    /* Initialize default for error offset as end of input. */
    out.output = use_buffer;
    out.output_end = use_buffer.wrapping_add(use_length) as PCRE2_SPTR;
    out.output_size = 0;

    out.out_str[0] = b'(';
    out.out_str[1] = b'?';
    out.out_str[2] = b's';
    out.out_str[3] = b')';
    convert_glob_write_str(&mut out, 4);

    is_start = TRUE;

    if pattern < pattern_end && *pattern.add(0) == b'*' {
        if no_wildsep != 0 {
            is_start = FALSE;
        } else if no_starstar == 0
            && pattern.wrapping_add(1) < pattern_end
            && *pattern.add(1) == b'*'
        {
            is_start = FALSE;
        }
    }

    if is_start != 0 {
        out.out_str[0] = b'\\';
        out.out_str[1] = b'A';
        convert_glob_write_str(&mut out, 2);
    }

    while pattern < pattern_end {
        c = *pattern;
        pattern = pattern.add(1);

        if c == b'*' {
            is_start = (pattern == pattern_start.wrapping_add(1)) as BOOL;

            if in_atomic != 0 {
                convert_glob_write(&mut out, b')');
                in_atomic = FALSE;
            }

            if no_starstar == 0 && pattern < pattern_end && *pattern == b'*' {
                after_separator =
                    (is_start != 0 || (*pattern.offset(-2) == separator)) as BOOL;

                loop {
                    pattern = pattern.add(1);
                    if !(pattern < pattern_end && *pattern == b'*') {
                        break;
                    }
                }

                if pattern >= pattern_end {
                    no_slash_z = TRUE;
                    break;
                }

                after_starstar = TRUE;

                if after_separator != 0
                    && escape != 0
                    && *pattern == escape
                    && pattern.wrapping_add(1) < pattern_end
                    && *pattern.add(1) == separator
                {
                    pattern = pattern.add(1);
                }

                if is_start != 0 {
                    if *pattern != separator {
                        continue;
                    }

                    out.out_str[0] = b'(';
                    out.out_str[1] = b'?';
                    out.out_str[2] = b':';
                    out.out_str[3] = b'\\';
                    out.out_str[4] = b'A';
                    out.out_str[5] = b'|';
                    convert_glob_write_str(&mut out, 6);

                    convert_glob_print_separator(&mut out, separator, with_escape);
                    convert_glob_write(&mut out, b')');

                    pattern = pattern.add(1);
                    continue;
                }

                convert_glob_print_commit(&mut out);

                if after_separator == 0 || *pattern != separator {
                    out.out_str[0] = b'.';
                    out.out_str[1] = b'*';
                    out.out_str[2] = b'?';
                    convert_glob_write_str(&mut out, 3);
                    continue;
                }

                out.out_str[0] = b'(';
                out.out_str[1] = b'?';
                out.out_str[2] = b':';
                out.out_str[3] = b'.';
                out.out_str[4] = b'*';
                out.out_str[5] = b'?';

                convert_glob_write_str(&mut out, 6);

                convert_glob_print_separator(&mut out, separator, with_escape);

                out.out_str[0] = b')';
                out.out_str[1] = b'?';
                out.out_str[2] = b'?';
                convert_glob_write_str(&mut out, 3);

                pattern = pattern.add(1);
                continue;
            }

            if pattern < pattern_end && *pattern == b'*' {
                loop {
                    pattern = pattern.add(1);
                    if !(pattern < pattern_end && *pattern == b'*') {
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
                    out.out_str[0] = b'(';
                    out.out_str[1] = b'?';
                    out.out_str[2] = b'>';
                    convert_glob_write_str(&mut out, 3);
                    in_atomic = TRUE;
                } else {
                    convert_glob_print_commit(&mut out);
                }
            }

            if no_wildsep != 0 {
                convert_glob_write(&mut out, b'.');
            } else {
                convert_glob_print_wildcard(&mut out, separator, with_escape);
            }

            out.out_str[0] = b'*';
            out.out_str[1] = b'?';
            if pattern >= pattern_end {
                out.out_str[1] = b'+';
            }
            convert_glob_write_str(&mut out, 2);
            continue;
        }

        if c == b'?' {
            if no_wildsep != 0 {
                convert_glob_write(&mut out, b'.');
            } else {
                convert_glob_print_wildcard(&mut out, separator, with_escape);
            }
            continue;
        }

        if c == b'[' {
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

        if escape != 0 && c == escape {
            if pattern >= pattern_end {
                result = PCRE2_ERROR_CONVERT_SYNTAX;
                break;
            }
            c = *pattern;
            pattern = pattern.add(1);
        }

        if (c as u32) < 255
            && !strchr(pcre2_escaped_literals.as_ptr() as *const c_char, c as i32).is_null()
        {
            convert_glob_write(&mut out, b'\\');
        }

        convert_glob_write(&mut out, c);
    }

    if result == 0 {
        if no_slash_z == 0 {
            out.out_str[0] = b'\\';
            out.out_str[1] = b'z';
            convert_glob_write_str(&mut out, 2);
        }

        if in_atomic != 0 {
            convert_glob_write(&mut out, b')');
        }

        convert_glob_write(&mut out, 0 /* CHAR_NUL */);

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

/*************************************************
*                Convert pattern                 *
*************************************************/

/* This is the external-facing function for converting other forms of pattern
into PCRE2 regular expression patterns. On error, the bufflenptr argument is
used to return an offset in the original pattern.

Arguments:
  pattern     the input pattern
  plength     length of input, or PCRE2_ZERO_TERMINATED
  options     options bits
  buffptr     pointer to pointer to output buffer
  bufflenptr  pointer to length of output buffer
  ccontext    convert context or NULL

Returns:      0 for success, else an error code (+ve or -ve)
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_pattern_convert_8(pattern: PCRE2_SPTR, plength: PCRE2_SIZE, options: u32, buffptr: *mut *mut PCRE2_UCHAR, bufflenptr: *mut PCRE2_SIZE, ccontext: *mut pcre2_real_convert_context) -> i32 {
    let mut pattern: PCRE2_SPTR = pattern;
    let mut plength: PCRE2_SIZE = plength;
    let mut ccontext: *mut pcre2_real_convert_context = ccontext;

    let mut rc: i32;
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

    if (options & !ALL_OPTIONS) != 0 ||        /* Undefined bit set */
       (pattype & (!pattype).wrapping_add(1)) != pattype ||  /* More than one type set */
       pattype == 0
    /* No type set */
    {
        *bufflenptr = 0; /* Error offset */
        return PCRE2_ERROR_BADOPTION;
    }

    if plength == PCRE2_ZERO_TERMINATED {
        plength = _pcre2_strlen_8(pattern);
    }
    if ccontext.is_null() {
        ccontext = core::ptr::addr_of_mut!(_pcre2_default_convert_context_8)
            as *mut pcre2_real_convert_context;
    }

    /* Check UTF if required. */

    if utf != 0 && (options & PCRE2_CONVERT_NO_UTF_CHECK) == 0 {
        let mut erroroffset: PCRE2_SIZE = 0;
        rc = _pcre2_valid_utf_8(pattern, plength, &mut erroroffset);
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

    let mut i: i32 = 0;
    while i < 2 {
        let allocated: *mut c_void;
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

        if rc != 0 ||                  /* Error */
           buffptr.is_null() ||        /* Just the length is required */
           !(*buffptr).is_null()
        /* Buffer was provided or allocated */
        {
            return rc;
        }

        /* Allocate memory for the buffer, with hidden space for an allocator at
        the start. The next time round the loop runs the conversion for real. */

        allocated = _pcre2_memctl_malloc_8(
            core::mem::size_of::<pcre2_memctl>() + (*bufflenptr + 1) * 8, /* PCRE2_CODE_UNIT_WIDTH */
            ccontext as *mut pcre2_memctl,
        );
        if allocated.is_null() {
            *bufflenptr = 0; /* Error offset */
            return PCRE2_ERROR_NOMEMORY;
        }
        *buffptr = (allocated as *mut c_char).add(core::mem::size_of::<pcre2_memctl>())
            as *mut PCRE2_UCHAR;

        use_buffer = *buffptr;
        use_length = *bufflenptr + 1;

        i += 1;
    }

    /* Running the loop above ought to have succeeded the second time. */
    *bufflenptr = 0; /* Error offset */
    PCRE2_ERROR_INTERNAL
}

/*************************************************
*            Free converted pattern              *
*************************************************/

/* This frees a converted pattern that was put in newly-allocated memory.

Argument:   the converted pattern
Returns:    nothing
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_converted_pattern_free_8(converted: *mut PCRE2_UCHAR) {
    if !converted.is_null() {
        let memctl: *mut pcre2_memctl = (converted as *mut c_char)
            .sub(core::mem::size_of::<pcre2_memctl>()) as *mut pcre2_memctl;
        ((*memctl).free.unwrap())(memctl as *mut c_void, (*memctl).memory_data);
    }
}

/* End of pcre2_convert.c */
