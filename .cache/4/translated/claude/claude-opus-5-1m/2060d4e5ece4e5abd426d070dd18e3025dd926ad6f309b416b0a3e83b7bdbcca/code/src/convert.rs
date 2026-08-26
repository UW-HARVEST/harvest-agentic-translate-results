// Translated from pcre2_convert.c
use crate::internal::*;
use crate::pcre2_pub::*;
use crate::tables::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

extern "C" {
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
}

const TYPE_OPTIONS: u32 =
    PCRE2_CONVERT_GLOB | PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_POSIX_EXTENDED;

const ALL_OPTIONS: u32 = PCRE2_CONVERT_UTF
    | PCRE2_CONVERT_NO_UTF_CHECK
    | PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR
    | PCRE2_CONVERT_GLOB_NO_STARSTAR
    | TYPE_OPTIONS;

const DUMMY_BUFFER_SIZE: usize = 100;

/* Generated pattern fragments */

const STR_BACKSLASH_A: &[u8] = b"\\A\0";
const STR_BACKSLASH_z: &[u8] = b"\\z\0";
const STR_COLON_RIGHT_SQUARE_BRACKET: &[u8] = b":]\0";
const STR_DOT_STAR_LOOKBEHIND: &[u8] = b".*(?<=\0";
const STR_LOOKAHEAD_NOT_DOT: &[u8] = b"(?!\\.)\0";
const STR_QUERY_s: &[u8] = b"(?s)\0";
const STR_STAR_NUL: &[u8] = b"(*NUL)\0";

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

/* Macro to add a character string to the output buffer, checking for overflow. */

macro_rules! PUTCHARS {
    ($p:ident, $endp:ident, $string:expr) => {{
        let mut s: *const u8 = ($string).as_ptr();
        while *s != 0 {
            if $p >= $endp {
                return PCRE2_ERROR_NOMEMORY;
            }
            *$p = *s;
            $p = $p.add(1);
            s = s.add(1);
        }
    }};
}

/* Macro to check for lowercase characters. */

#[inline(always)]
fn ISLOWER(c: u32) -> bool {
    c >= CHAR_a && c <= CHAR_z
}

/* Literals that must be escaped: \ ? * + | . ^ $ { } [ ] ( ) */

static pcre2_escaped_literals: &[u8] = b"\\?*+|.^${}[]()\0";

/* Recognized escaped metacharacters in POSIX basic patterns. */

static posix_meta_escapes: &[u8] = b"(){}123456789\0";

/* Recognized POSIX classes, colon-separated. */

static posix_classes: &[u8] =
    b"alpha:lower:upper:alnum:ascii:blank:cntrl:digit:graph:print:punct:space:word:xdigit:\0";

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

unsafe fn convert_posix(
    pattype: u32,
    pattern: PCRE2_SPTR,
    plength: PCRE2_SIZE,
    utf: BOOL,
    use_buffer: *mut PCRE2_UCHAR,
    use_length: PCRE2_SIZE,
    bufflenptr: *mut PCRE2_SIZE,
    dummyrun: BOOL,
    ccontext: *mut pcre2_real_convert_context,
) -> c_int {
    let mut plength = plength;
    let mut posix: PCRE2_SPTR = pattern;
    let mut p: *mut PCRE2_UCHAR = use_buffer;
    let mut pp: *mut PCRE2_UCHAR = p;
    let endp: *mut PCRE2_UCHAR = p.wrapping_add(use_length).wrapping_sub(1); /* Allow for trailing zero */
    let mut convlength: PCRE2_SIZE = 0;

    let mut bracount: u32 = 0;
    let mut posix_state: u32 = POSIX_START_REGEX;
    let mut lastspecial: u32 = 0;
    let extended: BOOL = if (pattype & PCRE2_CONVERT_POSIX_EXTENDED) != 0 {
        TRUE
    } else {
        FALSE
    };
    let mut nextisliteral: BOOL = FALSE;

    /* Initialize default for error offset as end of input. */

    *bufflenptr = plength;
    PUTCHARS!(p, endp, STR_STAR_NUL);

    /* Now scan the input. */

    'scan: while plength > 0 {
        let mut c: u32;
        let sc: u32;
        let mut clength: c_int = 1;

        /* Add in the length of the last item, then, if in the dummy run, pull the
        pointer back to the start of the (temporary) buffer and then remember the
        start of the next item. */

        convlength = convlength.wrapping_add(p.offset_from(pp) as PCRE2_SIZE);
        if dummyrun != FALSE {
            p = use_buffer;
        }
        pp = p;

        /* Pick up the next character */

        /* GETCHARLENTEST(c, posix, clength); */
        c = *posix as u32;
        if utf != 0 && c >= 0xc0 {
            clength += utf8_extra(c) as c_int;
            c = getutf8(c, posix);
        }
        posix = posix.add(clength as usize);
        plength = plength.wrapping_sub(clength as PCRE2_SIZE);

        sc = if nextisliteral != FALSE { 0 } else { c };
        nextisliteral = FALSE;

        /* Handle a character within a class. */

        if posix_state >= POSIX_CLASS_NOT_STARTED {
            if c == CHAR_RIGHT_SQUARE_BRACKET {
                PUTCHARS!(p, endp, STR_RIGHT_SQUARE_BRACKET);
                posix_state = POSIX_NOT_BRACKET;
            }
            /* Not the end of the class */
            else {
                'insw: {
                    if posix_state == POSIX_CLASS_STARTED {
                        if ISLOWER(c) {
                            break 'insw; /* Remain in started state */
                        }
                        posix_state = POSIX_CLASS_NOT_STARTED;
                        if c == CHAR_COLON
                            && plength > 0
                            && *posix as u32 == CHAR_RIGHT_SQUARE_BRACKET
                        {
                            PUTCHARS!(p, endp, STR_COLON_RIGHT_SQUARE_BRACKET);
                            plength -= 1;
                            posix = posix.add(1);
                            continue 'scan; /* With next character after :] */
                        }
                        /* Fall through */

                        /* case POSIX_CLASS_NOT_STARTED: */
                        if c == CHAR_LEFT_SQUARE_BRACKET {
                            posix_state = POSIX_CLASS_STARTING;
                        }
                        break 'insw;
                    }

                    if posix_state == POSIX_CLASS_NOT_STARTED {
                        if c == CHAR_LEFT_SQUARE_BRACKET {
                            posix_state = POSIX_CLASS_STARTING;
                        }
                        break 'insw;
                    }

                    if posix_state == POSIX_CLASS_STARTING {
                        if c == CHAR_COLON {
                            posix_state = POSIX_CLASS_STARTED;
                        }
                        break 'insw;
                    }
                }

                if c == CHAR_BACKSLASH {
                    PUTCHARS!(p, endp, STR_BACKSLASH);
                }
                if p.wrapping_add(clength as usize) > endp {
                    return PCRE2_ERROR_NOMEMORY;
                }
                memcpy(
                    p as *mut c_void,
                    posix.sub(clength as usize) as *const c_void,
                    CU2BYTES(clength as usize),
                );
                p = p.add(clength as usize);
            }
        }
        /* Handle a character not within a class. */
        else {
            'sw: {
                'default_tail: {
                    'do_copy_special: {
                        'do_escape_literal: {
                            'do_default: {
                                match sc {
                                    CHAR_LEFT_SQUARE_BRACKET => {
                                        PUTCHARS!(p, endp, STR_LEFT_SQUARE_BRACKET);

                                        /* Handle start of "normal" character classes */

                                        posix_state = POSIX_CLASS_NOT_STARTED;

                                        /* Handle ^ and ] as first characters */

                                        if plength > 0 {
                                            if *posix as u32 == CHAR_CIRCUMFLEX_ACCENT {
                                                posix = posix.add(1);
                                                plength -= 1;
                                                PUTCHARS!(p, endp, STR_CIRCUMFLEX_ACCENT);
                                            }
                                            if plength > 0
                                                && *posix as u32 == CHAR_RIGHT_SQUARE_BRACKET
                                            {
                                                posix = posix.add(1);
                                                plength -= 1;
                                                PUTCHARS!(p, endp, STR_RIGHT_SQUARE_BRACKET);
                                            }
                                        }
                                        break 'sw;
                                    }

                                    CHAR_BACKSLASH => {
                                        if plength == 0 {
                                            return PCRE2_ERROR_END_BACKSLASH;
                                        }
                                        if extended != FALSE {
                                            nextisliteral = TRUE;
                                        } else {
                                            if (*posix as u32) < 255
                                                && !strchr(
                                                    posix_meta_escapes.as_ptr() as *const c_char,
                                                    *posix as c_int,
                                                )
                                                .is_null()
                                            {
                                                if *posix as u32 >= CHAR_0
                                                    && *posix as u32 <= CHAR_9
                                                {
                                                    PUTCHARS!(p, endp, STR_BACKSLASH);
                                                }
                                                if p.wrapping_add(1) > endp {
                                                    return PCRE2_ERROR_NOMEMORY;
                                                }
                                                let v: PCRE2_UCHAR = *posix;
                                                posix = posix.add(1);
                                                *p = v;
                                                p = p.add(1);
                                                lastspecial = v as u32;
                                                plength -= 1;
                                            } else {
                                                nextisliteral = TRUE;
                                            }
                                        }
                                        break 'sw;
                                    }

                                    CHAR_RIGHT_PARENTHESIS => {
                                        if extended == FALSE || bracount == 0 {
                                            break 'do_escape_literal;
                                        }
                                        bracount -= 1;
                                        break 'do_copy_special;
                                    }

                                    CHAR_LEFT_PARENTHESIS => {
                                        bracount += 1;
                                        /* Fall through */

                                        if extended == FALSE {
                                            break 'do_escape_literal;
                                        }
                                        /* Fall through */

                                        posix_state = POSIX_NOT_BRACKET;
                                        break 'do_copy_special;
                                    }

                                    CHAR_QUESTION_MARK
                                    | CHAR_PLUS
                                    | CHAR_LEFT_CURLY_BRACKET
                                    | CHAR_RIGHT_CURLY_BRACKET
                                    | CHAR_VERTICAL_LINE => {
                                        if extended == FALSE {
                                            break 'do_escape_literal;
                                        }
                                        /* Fall through */

                                        posix_state = POSIX_NOT_BRACKET;
                                        break 'do_copy_special;
                                    }

                                    CHAR_DOT | CHAR_DOLLAR_SIGN => {
                                        posix_state = POSIX_NOT_BRACKET;
                                        break 'do_copy_special;
                                    }

                                    CHAR_ASTERISK => {
                                        if lastspecial != CHAR_ASTERISK {
                                            if extended == FALSE
                                                && (posix_state < POSIX_NOT_BRACKET
                                                    || lastspecial == CHAR_LEFT_PARENTHESIS)
                                            {
                                                break 'do_escape_literal;
                                            }
                                            break 'do_copy_special;
                                        }
                                        break 'sw; /* Ignore second and subsequent asterisks */
                                    }

                                    CHAR_CIRCUMFLEX_ACCENT => {
                                        if extended != FALSE {
                                            break 'do_copy_special;
                                        }
                                        if posix_state == POSIX_START_REGEX
                                            || lastspecial == CHAR_LEFT_PARENTHESIS
                                        {
                                            posix_state = POSIX_ANCHORED;
                                            break 'do_copy_special;
                                        }
                                        /* Fall through */
                                        break 'do_default;
                                    }

                                    _ => {
                                        break 'do_default;
                                    }
                                }
                            }
                            /* default: */
                            if c < 255
                                && !strchr(
                                    pcre2_escaped_literals.as_ptr() as *const c_char,
                                    c as c_int,
                                )
                                .is_null()
                            {
                                break 'do_escape_literal;
                            }
                            break 'default_tail;
                        }
                        /* ESCAPE_LITERAL: */
                        PUTCHARS!(p, endp, STR_BACKSLASH);
                        break 'default_tail;
                    }
                    /* COPY_SPECIAL: */
                    lastspecial = c;
                    if p.wrapping_add(1) > endp {
                        return PCRE2_ERROR_NOMEMORY;
                    }
                    *p = c as PCRE2_UCHAR;
                    p = p.add(1);
                    break 'sw;
                }
                /* Tail of the default case. */
                lastspecial = 0xff; /* Indicates nothing special */
                if p.wrapping_add(clength as usize) > endp {
                    return PCRE2_ERROR_NOMEMORY;
                }
                memcpy(
                    p as *mut c_void,
                    posix.sub(clength as usize) as *const c_void,
                    CU2BYTES(clength as usize),
                );
                p = p.add(clength as usize);
                posix_state = POSIX_NOT_BRACKET;
            }
        }
    }

    if posix_state >= POSIX_CLASS_NOT_STARTED {
        return PCRE2_ERROR_MISSING_SQUARE_BRACKET;
    }
    convlength = convlength.wrapping_add(p.offset_from(pp) as PCRE2_SIZE); /* Final segment */
    *bufflenptr = convlength;
    *p = 0;
    p = p.add(1);
    return 0;
}

/*************************************************
*           Convert a glob pattern               *
*************************************************/

/* Context for writing the output into a buffer. */

#[repr(C)]
struct pcre2_output_context {
    output: *mut PCRE2_UCHAR,  /* current output position */
    output_end: PCRE2_SPTR,    /* output end */
    output_size: PCRE2_SIZE,   /* size of the output */
    out_str: [u8; 8],          /* string copied to the output */
}

/* Write a character into the output.

Arguments:
  out            output context
  chr            the next character
*/

unsafe fn convert_glob_write(out: *mut pcre2_output_context, chr: PCRE2_UCHAR) {
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

unsafe fn convert_glob_write_str(out: *mut pcre2_output_context, length: PCRE2_SIZE) {
    let mut length = length;
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

unsafe fn convert_glob_print_separator(
    out: *mut pcre2_output_context,
    separator: PCRE2_UCHAR,
    with_escape: BOOL,
) {
    if with_escape != FALSE {
        convert_glob_write(out, CHAR_BACKSLASH as PCRE2_UCHAR);
    }

    convert_glob_write(out, separator);
}

/* Prints a wildcard into the output.

Arguments:
  out            output context
  separator      glob separator
  with_escape    backslash is needed before separator
*/

unsafe fn convert_glob_print_wildcard(
    out: *mut pcre2_output_context,
    separator: PCRE2_UCHAR,
    with_escape: BOOL,
) {
    (*out).out_str[0] = CHAR_LEFT_SQUARE_BRACKET as u8;
    (*out).out_str[1] = CHAR_CIRCUMFLEX_ACCENT as u8;
    convert_glob_write_str(out, 2);

    convert_glob_print_separator(out, separator, with_escape);

    convert_glob_write(out, CHAR_RIGHT_SQUARE_BRACKET as PCRE2_UCHAR);
}

/* Parse a posix class.

Arguments:
  from           starting point of scanning the range
  pattern_end    end of pattern
  out            output context

Returns:  >0 => class index
          0  => malformed class
*/

unsafe fn convert_glob_parse_class(
    from: *mut PCRE2_SPTR,
    pattern_end: PCRE2_SPTR,
    out: *mut pcre2_output_context,
) -> c_int {
    let mut start: PCRE2_SPTR = (*from).add(1);
    let mut pattern: PCRE2_SPTR = start;
    let mut class_ptr: *const u8;
    let mut c: PCRE2_UCHAR = 0;
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

    class_ptr = posix_classes.as_ptr();
    class_index = 1;

    loop {
        if *class_ptr == 0 {
            return 0;
        }

        pattern = start;

        while *pattern == *class_ptr as PCRE2_UCHAR {
            if *pattern as u32 == CHAR_COLON {
                pattern = pattern.add(2);
                start = start.sub(2);

                loop {
                    convert_glob_write(out, *start);
                    start = start.add(1);
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

        while *class_ptr as u32 != CHAR_COLON {
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

unsafe fn convert_glob_char_in_class(class_index: c_int, c: PCRE2_UCHAR) -> BOOL {
    let cbits: *const u8 = _pcre2_default_tables_8.as_ptr().add(cbits_offset);
    let cbit: usize;

    /* See posix_class_maps. This is a small local clone of that.
    Note that we don't know exactly what character tables will be used at
    match time, but, for the purposes of pattern conversion, it should be
    sufficient to use PCRE2's built-in default tables. */

    match class_index {
        1 => {
            /* alpha */
            if c as u32 == CHAR_UNDERSCORE {
                return FALSE;
            }
            if (*cbits.add(cbit_digit + (c as usize) / 8) as u32 & (1u32 << (c & 7))) != 0 {
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
            if (*cbits.add(cbit_cntrl + (c as usize) / 8) as u32 & (1u32 << (c & 7))) != 0 {
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

    return if (*cbits.add(cbit + (c as usize) / 8) as u32 & (1u32 << (c & 7))) != 0 {
        TRUE
    } else {
        FALSE
    };
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

unsafe fn convert_glob_parse_range(
    from: *mut PCRE2_SPTR,
    pattern_end: PCRE2_SPTR,
    out: *mut pcre2_output_context,
    utf: BOOL,
    separator: PCRE2_UCHAR,
    with_escape: BOOL,
    escape: PCRE2_UCHAR,
    no_wildsep: BOOL,
) -> c_int {
    let mut is_negative: BOOL = FALSE;
    let mut separator_seen: BOOL = FALSE;
    let mut has_prev_c: BOOL;
    let mut pattern: PCRE2_SPTR = *from;
    let mut char_start: PCRE2_SPTR = core::ptr::null();
    let mut c: u32;
    let mut prev_c: u32;
    let mut len: c_int;
    let mut class_index: c_int;

    if pattern >= pattern_end {
        *from = pattern;
        return PCRE2_ERROR_MISSING_SQUARE_BRACKET;
    }

    if *pattern as u32 == CHAR_EXCLAMATION_MARK || *pattern as u32 == CHAR_CIRCUMFLEX_ACCENT {
        pattern = pattern.add(1);

        if pattern >= pattern_end {
            *from = pattern;
            return PCRE2_ERROR_MISSING_SQUARE_BRACKET;
        }

        is_negative = TRUE;

        (*out).out_str[0] = CHAR_LEFT_SQUARE_BRACKET as u8;
        (*out).out_str[1] = CHAR_CIRCUMFLEX_ACCENT as u8;
        len = 2;

        if no_wildsep == FALSE {
            if with_escape != FALSE {
                (*out).out_str[len as usize] = CHAR_BACKSLASH as u8;
                len += 1;
            }
            (*out).out_str[len as usize] = separator as u8;
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
        /* GETCHARINCTEST(c, pattern); */
        c = *pattern as u32;
        pattern = pattern.add(1);
        if utf != 0 && c >= 0xc0 {
            let r = getutf8inc(c, pattern);
            c = r.0;
            pattern = r.1;
        }

        if c == CHAR_RIGHT_SQUARE_BRACKET {
            convert_glob_write(out, c as PCRE2_UCHAR);

            if is_negative == FALSE && no_wildsep == FALSE && separator_seen != FALSE {
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

                if is_negative == FALSE
                    && convert_glob_char_in_class(class_index, separator) != FALSE
                {
                    separator_seen = TRUE;
                }
                continue;
            }
        } else if c == CHAR_MINUS
            && has_prev_c != FALSE
            && *pattern as u32 != CHAR_RIGHT_SQUARE_BRACKET
        {
            convert_glob_write(out, CHAR_MINUS as PCRE2_UCHAR);

            char_start = pattern;
            /* GETCHARINCTEST(c, pattern); */
            c = *pattern as u32;
            pattern = pattern.add(1);
            if utf != 0 && c >= 0xc0 {
                let r = getutf8inc(c, pattern);
                c = r.0;
                pattern = r.1;
            }

            if pattern >= pattern_end {
                break;
            }

            if escape != 0 && c == escape as u32 {
                char_start = pattern;
                /* GETCHARINCTEST(c, pattern); */
                c = *pattern as u32;
                pattern = pattern.add(1);
                if utf != 0 && c >= 0xc0 {
                    let r = getutf8inc(c, pattern);
                    c = r.0;
                    pattern = r.1;
                }
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
            if escape != 0 && c == escape as u32 {
                char_start = pattern;
                /* GETCHARINCTEST(c, pattern); */
                c = *pattern as u32;
                pattern = pattern.add(1);
                if utf != 0 && c >= 0xc0 {
                    let r = getutf8inc(c, pattern);
                    c = r.0;
                    pattern = r.1;
                }

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
            if !(char_start < pattern) {
                break;
            }
        }
    }

    *from = pattern;
    return PCRE2_ERROR_MISSING_SQUARE_BRACKET;
}

/* Prints a (*COMMIT) into the output.

Arguments:
  out            output context
*/

unsafe fn convert_glob_print_commit(out: *mut pcre2_output_context) {
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

unsafe fn convert_glob(
    options: u32,
    pattern: PCRE2_SPTR,
    plength: PCRE2_SIZE,
    utf: BOOL,
    use_buffer: *mut PCRE2_UCHAR,
    use_length: PCRE2_SIZE,
    bufflenptr: *mut PCRE2_SIZE,
    dummyrun: BOOL,
    ccontext: *mut pcre2_real_convert_context,
) -> c_int {
    let mut out = pcre2_output_context {
        output: core::ptr::null_mut(),
        output_end: core::ptr::null(),
        output_size: 0,
        out_str: [0u8; 8],
    };
    let mut pattern = pattern;
    let pattern_start: PCRE2_SPTR = pattern;
    let pattern_end: PCRE2_SPTR = pattern.add(plength);
    let separator: PCRE2_UCHAR = (*ccontext).glob_separator as PCRE2_UCHAR;
    let escape: PCRE2_UCHAR = (*ccontext).glob_escape as PCRE2_UCHAR;
    let mut c: PCRE2_UCHAR;
    let no_wildsep: BOOL = if (options & PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR) != 0 {
        TRUE
    } else {
        FALSE
    };
    let no_starstar: BOOL = if (options & PCRE2_CONVERT_GLOB_NO_STARSTAR) != 0 {
        TRUE
    } else {
        FALSE
    };
    let mut in_atomic: BOOL = FALSE;
    let mut after_starstar: BOOL = FALSE;
    let mut no_slash_z: BOOL = FALSE;
    let with_escape: BOOL;
    let mut is_start: BOOL;
    let mut after_separator: BOOL;
    let mut result: c_int = 0;

    if utf != 0 && (separator >= 128 || escape >= 128) {
        /* Currently only ASCII characters are supported. */
        *bufflenptr = 0;
        return PCRE2_ERROR_CONVERT_SYNTAX;
    }

    with_escape = if !strchr(
        pcre2_escaped_literals.as_ptr() as *const c_char,
        separator as c_int,
    )
    .is_null()
    {
        TRUE
    } else {
        FALSE
    };

    /* Initialize default for error offset as end of input. */
    out.output = use_buffer;
    out.output_end = use_buffer.add(use_length) as PCRE2_SPTR;
    out.output_size = 0;

    out.out_str[0] = CHAR_LEFT_PARENTHESIS as u8;
    out.out_str[1] = CHAR_QUESTION_MARK as u8;
    out.out_str[2] = CHAR_s as u8;
    out.out_str[3] = CHAR_RIGHT_PARENTHESIS as u8;
    convert_glob_write_str(&mut out, 4);

    is_start = TRUE;

    if pattern < pattern_end && *pattern.add(0) as u32 == CHAR_ASTERISK {
        if no_wildsep != FALSE {
            is_start = FALSE;
        } else if no_starstar == FALSE
            && pattern.add(1) < pattern_end
            && *pattern.add(1) as u32 == CHAR_ASTERISK
        {
            is_start = FALSE;
        }
    }

    if is_start != FALSE {
        out.out_str[0] = CHAR_BACKSLASH as u8;
        out.out_str[1] = CHAR_A as u8;
        convert_glob_write_str(&mut out, 2);
    }

    while pattern < pattern_end {
        c = *pattern;
        pattern = pattern.add(1);

        if c as u32 == CHAR_ASTERISK {
            is_start = if pattern == pattern_start.add(1) { TRUE } else { FALSE };

            if in_atomic != FALSE {
                convert_glob_write(&mut out, CHAR_RIGHT_PARENTHESIS as PCRE2_UCHAR);
                in_atomic = FALSE;
            }

            if no_starstar == FALSE && pattern < pattern_end && *pattern as u32 == CHAR_ASTERISK {
                after_separator = if is_start != FALSE || (*pattern.offset(-2) == separator) {
                    TRUE
                } else {
                    FALSE
                };

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

                if after_separator != FALSE
                    && escape != 0
                    && *pattern == escape
                    && pattern.add(1) < pattern_end
                    && *pattern.add(1) == separator
                {
                    pattern = pattern.add(1);
                }

                if is_start != FALSE {
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

                if after_separator == FALSE || *pattern != separator {
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

            if no_wildsep != FALSE {
                if pattern >= pattern_end {
                    no_slash_z = TRUE;
                    break;
                }

                /* Start check must be after the end check. */
                if is_start != FALSE {
                    continue;
                }
            }

            if is_start == FALSE {
                if after_starstar != FALSE {
                    out.out_str[0] = CHAR_LEFT_PARENTHESIS as u8;
                    out.out_str[1] = CHAR_QUESTION_MARK as u8;
                    out.out_str[2] = CHAR_GREATER_THAN_SIGN as u8;
                    convert_glob_write_str(&mut out, 3);
                    in_atomic = TRUE;
                } else {
                    convert_glob_print_commit(&mut out);
                }
            }

            if no_wildsep != FALSE {
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
            if no_wildsep != FALSE {
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

        if escape != 0 && c == escape {
            if pattern >= pattern_end {
                result = PCRE2_ERROR_CONVERT_SYNTAX;
                break;
            }
            c = *pattern;
            pattern = pattern.add(1);
        }

        if (c as u32) < 255
            && !strchr(pcre2_escaped_literals.as_ptr() as *const c_char, c as c_int).is_null()
        {
            convert_glob_write(&mut out, CHAR_BACKSLASH as PCRE2_UCHAR);
        }

        convert_glob_write(&mut out, c);
    }

    if result == 0 {
        if no_slash_z == FALSE {
            out.out_str[0] = CHAR_BACKSLASH as u8;
            out.out_str[1] = CHAR_z as u8;
            convert_glob_write_str(&mut out, 2);
        }

        if in_atomic != FALSE {
            convert_glob_write(&mut out, CHAR_RIGHT_PARENTHESIS as PCRE2_UCHAR);
        }

        convert_glob_write(&mut out, CHAR_NUL as PCRE2_UCHAR);

        if dummyrun == FALSE
            && out.output_size != out.output.offset_from(use_buffer) as PCRE2_SIZE
        {
            result = PCRE2_ERROR_NOMEMORY;
        }
    }

    if result != 0 {
        *bufflenptr = pattern.offset_from(pattern_start) as PCRE2_SIZE;
        return result;
    }

    *bufflenptr = out.output_size - 1;
    return 0;
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
pub unsafe extern "C" fn pcre2_pattern_convert_8(
    pattern: PCRE2_SPTR,
    plength: PCRE2_SIZE,
    options: u32,
    buffptr: *mut *mut PCRE2_UCHAR,
    bufflenptr: *mut PCRE2_SIZE,
    ccontext: *mut pcre2_real_convert_context,
) -> c_int {
    let mut pattern = pattern;
    let mut plength = plength;
    let mut ccontext = ccontext;
    let rc: c_int;
    let null_str: [PCRE2_UCHAR; 1] = [0xcd];
    let mut dummy_buffer: [PCRE2_UCHAR; DUMMY_BUFFER_SIZE] = [0; DUMMY_BUFFER_SIZE];
    let mut use_buffer: *mut PCRE2_UCHAR = dummy_buffer.as_mut_ptr();
    let mut use_length: PCRE2_SIZE = DUMMY_BUFFER_SIZE;
    let utf: BOOL = if (options & PCRE2_CONVERT_UTF) != 0 {
        TRUE
    } else {
        FALSE
    };
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
        plength = crate::string_utils::_pcre2_strlen_8(pattern);
    }
    if ccontext.is_null() {
        ccontext = &raw mut crate::context::_pcre2_default_convert_context_8;
    }

    /* Check UTF if required. */

    if utf != 0 && (options & PCRE2_CONVERT_NO_UTF_CHECK) == 0 {
        let mut erroroffset: PCRE2_SIZE = 0;
        rc = crate::valid_utf::_pcre2_valid_utf_8(pattern, plength, &mut erroroffset);
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

    let mut i: c_int = 0;
    while i < 2 {
        let allocated: *mut PCRE2_UCHAR;
        let dummyrun: BOOL = if buffptr.is_null() || (*buffptr).is_null() {
            TRUE
        } else {
            FALSE
        };
        let rc: c_int;

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

        if rc != 0 ||           /* Error */
            buffptr.is_null() ||   /* Just the length is required */
            !(*buffptr).is_null()
        /* Buffer was provided or allocated */
        {
            return rc;
        }

        /* Allocate memory for the buffer, with hidden space for an allocator at
        the start. The next time round the loop runs the conversion for real. */

        allocated = crate::context::_pcre2_memctl_malloc_8(
            core::mem::size_of::<pcre2_memctl>() + (*bufflenptr + 1) * 8,
            ccontext as *mut pcre2_memctl,
        ) as *mut PCRE2_UCHAR;
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
    return PCRE2_ERROR_INTERNAL;
}

/*************************************************
*            Free converted pattern              *
*************************************************/

/* This frees a converted pattern that was put in newly-allocated memory.

Argument:   the converted pattern
Returns:    nothing
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_converted_pattern_free_8(converted_pattern: *mut PCRE2_UCHAR) {
    if !converted_pattern.is_null() {
        let memctl: *mut pcre2_memctl = (converted_pattern as *mut c_char)
            .sub(core::mem::size_of::<pcre2_memctl>()) as *mut pcre2_memctl;
        ((*memctl).free.unwrap())(memctl as *mut c_void, (*memctl).memory_data);
    }
}

/* End of pcre2_convert.c */
