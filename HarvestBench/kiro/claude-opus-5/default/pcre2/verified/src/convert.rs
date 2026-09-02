//! Translation of `pcre2_convert.c`.
//!
//! Implements POSIX BRE/ERE and glob to PCRE2 pattern conversion. This is a
//! faithful, byte-for-byte port of the C implementation for the 8-bit library
//! with `SUPPORT_UNICODE` enabled.

use crate::internal::*;
use crate::tables;
use core::ffi::{c_int, c_void};
use core::ptr;

// ---------------------------------------------------------------------------
// Option masks (mirrors the #defines at the top of pcre2_convert.c)
// ---------------------------------------------------------------------------

const TYPE_OPTIONS: u32 = (PCRE2_CONVERT_GLOB
    | PCRE2_CONVERT_POSIX_BASIC
    | PCRE2_CONVERT_POSIX_EXTENDED) as u32;

const ALL_OPTIONS: u32 = (PCRE2_CONVERT_UTF
    | PCRE2_CONVERT_NO_UTF_CHECK
    | PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR
    | PCRE2_CONVERT_GLOB_NO_STARSTAR) as u32
    | TYPE_OPTIONS;

const DUMMY_BUFFER_SIZE: usize = 100;

// ---------------------------------------------------------------------------
// ASCII character constants (from pcre2_internal.h, non-EBCDIC)
// ---------------------------------------------------------------------------

const CHAR_NUL: u8 = 0x00;
const CHAR_LF: u8 = 0x0a;
const CHAR_VT: u8 = 0x0b;
const CHAR_FF: u8 = 0x0c;
const CHAR_CR: u8 = 0x0d;
const CHAR_EXCLAMATION_MARK: u8 = b'!';
const CHAR_DOLLAR_SIGN: u8 = b'$';
const CHAR_LEFT_PARENTHESIS: u8 = b'(';
const CHAR_RIGHT_PARENTHESIS: u8 = b')';
const CHAR_ASTERISK: u8 = b'*';
const CHAR_PLUS: u8 = b'+';
const CHAR_MINUS: u8 = b'-';
const CHAR_DOT: u8 = b'.';
const CHAR_0: u8 = b'0';
const CHAR_9: u8 = b'9';
const CHAR_COLON: u8 = b':';
const CHAR_LESS_THAN_SIGN: u8 = b'<';
const CHAR_GREATER_THAN_SIGN: u8 = b'>';
const CHAR_QUESTION_MARK: u8 = b'?';
const CHAR_A: u8 = b'A';
const CHAR_C: u8 = b'C';
const CHAR_I: u8 = b'I';
const CHAR_M: u8 = b'M';
const CHAR_O: u8 = b'O';
const CHAR_T: u8 = b'T';
const CHAR_LEFT_SQUARE_BRACKET: u8 = b'[';
const CHAR_BACKSLASH: u8 = b'\\';
const CHAR_RIGHT_SQUARE_BRACKET: u8 = b']';
const CHAR_CIRCUMFLEX_ACCENT: u8 = b'^';
const CHAR_UNDERSCORE: u8 = b'_';
const CHAR_a: u8 = b'a';
const CHAR_s: u8 = b's';
const CHAR_z: u8 = b'z';
const CHAR_VERTICAL_LINE: u8 = b'|';

// ---------------------------------------------------------------------------
// States for POSIX processing
// ---------------------------------------------------------------------------

const POSIX_START_REGEX: u32 = 0;
const POSIX_ANCHORED: u32 = 1;
const POSIX_NOT_BRACKET: u32 = 2;
const POSIX_CLASS_NOT_STARTED: u32 = 3;
const POSIX_CLASS_STARTING: u32 = 4;
const POSIX_CLASS_STARTED: u32 = 5;

// ---------------------------------------------------------------------------
// Static string data
// ---------------------------------------------------------------------------

/// Literals that must be escaped: `\ ? * + | . ^ $ { } [ ] ( )`.
/// NUL-terminated so it can be scanned like the C `char *`.
static PCRE2_ESCAPED_LITERALS: &[u8] = b"\\?*+|.^$\
{}[]()\0";

/// Recognized escaped metacharacters in POSIX basic patterns.
/// `( ) { } 1 2 3 4 5 6 7 8 9`, NUL-terminated.
static POSIX_META_ESCAPES: &[u8] = b"(){}123456789\0";

/// Recognized POSIX classes, colon-separated (each class name followed by a
/// colon). The C literal is implicitly NUL-terminated.
static POSIX_CLASSES: &[u8] = b"alpha:lower:upper:alnum:ascii:blank:cntrl:digit:graph:print:punct:space:word:xdigit:\0";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `ISLOWER(c)` for the non-EBCDIC build.
#[inline(always)]
fn is_lower(c: u32) -> bool {
    c >= CHAR_a as u32 && c <= CHAR_z as u32
}

/// Equivalent of `strchr(s, c) != NULL` where `s` is a NUL-terminated byte
/// slice. The terminating NUL is part of the searched string (so searching for
/// 0 succeeds), matching C's `strchr`.
#[inline]
fn strchr_found(s: &[u8], c: u8) -> bool {
    for &b in s {
        if b == c {
            return true;
        }
        if b == 0 {
            return false;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Convert a POSIX pattern
// ---------------------------------------------------------------------------

/// This function handles both basic and extended POSIX patterns.
unsafe fn convert_posix(
    pattype: u32,
    pattern: PCRE2_SPTR,
    plength: PCRE2_SIZE,
    utf: bool,
    use_buffer: *mut PCRE2_UCHAR,
    use_length: PCRE2_SIZE,
    bufflenptr: *mut PCRE2_SIZE,
    dummyrun: bool,
    ccontext: *mut pcre2_convert_context,
) -> c_int {
    unsafe {
        let mut posix: PCRE2_SPTR = pattern;
        let mut p: *mut PCRE2_UCHAR = use_buffer;
        let mut pp: *mut PCRE2_UCHAR = p;
        let endp: *mut PCRE2_UCHAR = p.add(use_length - 1); // Allow for trailing zero
        let mut convlength: PCRE2_SIZE = 0;

        let mut bracount: u32 = 0;
        let mut posix_state: u32 = POSIX_START_REGEX;
        let mut lastspecial: u32 = 0;
        let extended: bool = (pattype & PCRE2_CONVERT_POSIX_EXTENDED as u32) != 0;
        let mut nextisliteral: bool = false;

        let _ = utf;
        let _ = ccontext;

        let mut plength = plength;

        // Macro `PUTCHARS(string)`: emit each byte of a NUL-terminated slice,
        // checking for overflow.
        macro_rules! putchars {
            ($string:expr) => {{
                let s: &[u8] = $string;
                for &ch in s.iter() {
                    if ch == 0 {
                        break;
                    }
                    if p >= endp {
                        return PCRE2_ERROR_NOMEMORY as c_int;
                    }
                    *p = ch;
                    p = p.add(1);
                }
            }};
        }

        // Initialize default for error offset as end of input.
        *bufflenptr = plength;

        // STR_STAR_NUL == "(*NUL)"
        putchars!(b"(*NUL)");

        // Now scan the input.
        while plength > 0 {
            let c: u32;
            let sc: u32;
            let mut clength: c_int = 1;

            // Add in the length of the last item; if in the dummy run, pull the
            // pointer back to the start of the buffer and remember the start of
            // the next item.
            convlength += (p as usize - pp as usize) as PCRE2_SIZE;
            if dummyrun {
                p = use_buffer;
            }
            pp = p;

            // Pick up the next character (GETCHARLENTEST).
            {
                let mut len: u32 = 1;
                c = GETCHARLENTEST(posix, &mut len, utf);
                clength = len as c_int;
            }
            posix = posix.add(clength as usize);
            plength -= clength as usize;

            sc = if nextisliteral { 0 } else { c };
            nextisliteral = false;

            // Handle a character within a class.
            if posix_state >= POSIX_CLASS_NOT_STARTED {
                if c == CHAR_RIGHT_SQUARE_BRACKET as u32 {
                    putchars!(b"]");
                    posix_state = POSIX_NOT_BRACKET;
                }
                // Not the end of the class
                else {
                    match posix_state {
                        POSIX_CLASS_STARTED => {
                            if is_lower(c) {
                                // Remain in started state; fall out of match,
                                // then continue below.
                            } else {
                                posix_state = POSIX_CLASS_NOT_STARTED;
                                if c == CHAR_COLON as u32
                                    && plength > 0
                                    && *posix == CHAR_RIGHT_SQUARE_BRACKET
                                {
                                    putchars!(b":]");
                                    plength -= 1;
                                    posix = posix.add(1);
                                    continue; // With next character after :]
                                }
                                // Fall through to POSIX_CLASS_NOT_STARTED logic.
                                if c == CHAR_LEFT_SQUARE_BRACKET as u32 {
                                    posix_state = POSIX_CLASS_STARTING;
                                }
                            }
                        }
                        POSIX_CLASS_NOT_STARTED => {
                            if c == CHAR_LEFT_SQUARE_BRACKET as u32 {
                                posix_state = POSIX_CLASS_STARTING;
                            }
                        }
                        POSIX_CLASS_STARTING => {
                            if c == CHAR_COLON as u32 {
                                posix_state = POSIX_CLASS_STARTED;
                            }
                        }
                        _ => {}
                    }

                    if c == CHAR_BACKSLASH as u32 {
                        putchars!(b"\\");
                    }
                    if p.add(clength as usize) > endp {
                        return PCRE2_ERROR_NOMEMORY as c_int;
                    }
                    c_memcpy(
                        p as *mut c_void,
                        posix.sub(clength as usize) as *const c_void,
                        CU2BYTES(clength as usize),
                    );
                    p = p.add(clength as usize);
                }
            }
            // Handle a character not within a class.
            else {
                // Emulate C's switch/goto with labeled loop + flags.
                //
                // Labels: ESCAPE_LITERAL and COPY_SPECIAL.
                // We use a small state machine to reproduce the fallthroughs
                // and gotos exactly.
                enum Action {
                    Break,
                    CopySpecial,
                    EscapeLiteralThenDefault,
                    Default,
                }

                let action: Action;

                // Reproduce the switch(sc)
                if sc == CHAR_LEFT_SQUARE_BRACKET as u32 {
                    putchars!(b"[");

                    // Handle start of "normal" character classes
                    posix_state = POSIX_CLASS_NOT_STARTED;

                    // Handle ^ and ] as first characters
                    if plength > 0 {
                        if *posix == CHAR_CIRCUMFLEX_ACCENT {
                            posix = posix.add(1);
                            plength -= 1;
                            putchars!(b"^");
                        }
                        if plength > 0 && *posix == CHAR_RIGHT_SQUARE_BRACKET {
                            posix = posix.add(1);
                            plength -= 1;
                            putchars!(b"]");
                        }
                    }
                    action = Action::Break;
                } else if sc == CHAR_BACKSLASH as u32 {
                    if plength == 0 {
                        return PCRE2_ERROR_END_BACKSLASH as c_int;
                    }
                    if extended {
                        nextisliteral = true;
                    } else {
                        if (*posix as u32) < 255
                            && strchr_found(POSIX_META_ESCAPES, *posix)
                        {
                            if *posix >= CHAR_0 && *posix <= CHAR_9 {
                                putchars!(b"\\");
                            }
                            if p.add(1) > endp {
                                return PCRE2_ERROR_NOMEMORY as c_int;
                            }
                            lastspecial = *posix as u32;
                            *p = *posix;
                            p = p.add(1);
                            posix = posix.add(1);
                            plength -= 1;
                        } else {
                            nextisliteral = true;
                        }
                    }
                    action = Action::Break;
                } else if sc == CHAR_RIGHT_PARENTHESIS as u32 {
                    if !extended || bracount == 0 {
                        action = Action::EscapeLiteralThenDefault;
                    } else {
                        bracount -= 1;
                        action = Action::CopySpecial;
                    }
                } else if sc == CHAR_LEFT_PARENTHESIS as u32 {
                    bracount += 1;
                    // Fall through to the (? + { } |) group:
                    if !extended {
                        action = Action::EscapeLiteralThenDefault;
                    } else {
                        // Fall through to CHAR_DOT/CHAR_DOLLAR handling.
                        posix_state = POSIX_NOT_BRACKET;
                        action = Action::CopySpecial;
                    }
                } else if sc == CHAR_QUESTION_MARK as u32
                    || sc == CHAR_PLUS as u32
                    || sc == b'{' as u32
                    || sc == b'}' as u32
                    || sc == CHAR_VERTICAL_LINE as u32
                {
                    if !extended {
                        action = Action::EscapeLiteralThenDefault;
                    } else {
                        posix_state = POSIX_NOT_BRACKET;
                        action = Action::CopySpecial;
                    }
                } else if sc == CHAR_DOT as u32 || sc == CHAR_DOLLAR_SIGN as u32 {
                    posix_state = POSIX_NOT_BRACKET;
                    action = Action::CopySpecial;
                } else if sc == CHAR_ASTERISK as u32 {
                    if lastspecial != CHAR_ASTERISK as u32 {
                        if !extended
                            && (posix_state < POSIX_NOT_BRACKET
                                || lastspecial == CHAR_LEFT_PARENTHESIS as u32)
                        {
                            action = Action::EscapeLiteralThenDefault;
                        } else {
                            action = Action::CopySpecial;
                        }
                    } else {
                        // Ignore second and subsequent asterisks
                        action = Action::Break;
                    }
                } else if sc == CHAR_CIRCUMFLEX_ACCENT as u32 {
                    if extended {
                        action = Action::CopySpecial;
                    } else if posix_state == POSIX_START_REGEX
                        || lastspecial == CHAR_LEFT_PARENTHESIS as u32
                    {
                        posix_state = POSIX_ANCHORED;
                        action = Action::CopySpecial;
                    } else {
                        // Fall through to default.
                        action = Action::Default;
                    }
                } else {
                    action = Action::Default;
                }

                match action {
                    Action::Break => {}
                    Action::CopySpecial => {
                        // COPY_SPECIAL:
                        lastspecial = c;
                        if p.add(1) > endp {
                            return PCRE2_ERROR_NOMEMORY as c_int;
                        }
                        *p = c as u8;
                        p = p.add(1);
                    }
                    Action::EscapeLiteralThenDefault | Action::Default => {
                        // default:
                        let mut do_escape = false;
                        if let Action::EscapeLiteralThenDefault = action {
                            // Jumped straight to ESCAPE_LITERAL.
                            do_escape = true;
                        } else {
                            // default: check pcre2_escaped_literals
                            if c < 255 && strchr_found(PCRE2_ESCAPED_LITERALS, c as u8) {
                                do_escape = true;
                            }
                        }
                        if do_escape {
                            // ESCAPE_LITERAL:
                            putchars!(b"\\");
                        }
                        lastspecial = 0xff; // Indicates nothing special
                        if p.add(clength as usize) > endp {
                            return PCRE2_ERROR_NOMEMORY as c_int;
                        }
                        c_memcpy(
                            p as *mut c_void,
                            posix.sub(clength as usize) as *const c_void,
                            CU2BYTES(clength as usize),
                        );
                        p = p.add(clength as usize);
                        posix_state = POSIX_NOT_BRACKET;
                    }
                }
            }
        }

        if posix_state >= POSIX_CLASS_NOT_STARTED {
            return PCRE2_ERROR_MISSING_SQUARE_BRACKET as c_int;
        }
        convlength += (p as usize - pp as usize) as PCRE2_SIZE; // Final segment
        *bufflenptr = convlength;
        *p = 0;
        // p++ in C, but p is not used afterwards.
        0
    }
}

// ---------------------------------------------------------------------------
// Glob conversion
// ---------------------------------------------------------------------------

/// Context for writing the output into a buffer.
struct PcreOutputContext {
    output: *mut PCRE2_UCHAR,
    output_end: PCRE2_SPTR,
    output_size: PCRE2_SIZE,
    out_str: [u8; 8],
}

/// Write a character into the output.
#[inline]
unsafe fn convert_glob_write(out: &mut PcreOutputContext, chr: PCRE2_UCHAR) {
    unsafe {
        out.output_size += 1;
        if (out.output as PCRE2_SPTR) < out.output_end {
            *out.output = chr;
            out.output = out.output.add(1);
        }
    }
}

/// Write a string (the first `length` bytes of `out.out_str`) into the output.
#[inline]
unsafe fn convert_glob_write_str(out: &mut PcreOutputContext, length: PCRE2_SIZE) {
    unsafe {
        let mut idx = 0usize;
        let mut output = out.output;
        let output_end = out.output_end;
        let mut output_size = out.output_size;
        let mut length = length;

        loop {
            output_size += 1;
            if (output as PCRE2_SPTR) < output_end {
                *output = out.out_str[idx];
                output = output.add(1);
                idx += 1;
            }
            length -= 1;
            if length == 0 {
                break;
            }
        }

        out.output = output;
        out.output_size = output_size;
    }
}

/// Prints the separator into the output.
#[inline]
unsafe fn convert_glob_print_separator(
    out: &mut PcreOutputContext,
    separator: PCRE2_UCHAR,
    with_escape: bool,
) {
    unsafe {
        if with_escape {
            convert_glob_write(out, CHAR_BACKSLASH);
        }
        convert_glob_write(out, separator);
    }
}

/// Prints a wildcard into the output.
#[inline]
unsafe fn convert_glob_print_wildcard(
    out: &mut PcreOutputContext,
    separator: PCRE2_UCHAR,
    with_escape: bool,
) {
    unsafe {
        out.out_str[0] = CHAR_LEFT_SQUARE_BRACKET;
        out.out_str[1] = CHAR_CIRCUMFLEX_ACCENT;
        convert_glob_write_str(out, 2);

        convert_glob_print_separator(out, separator, with_escape);

        convert_glob_write(out, CHAR_RIGHT_SQUARE_BRACKET);
    }
}

/// Parse a posix class.
///
/// Returns `>0` => class index, `0` => malformed class.
unsafe fn convert_glob_parse_class(
    from: &mut PCRE2_SPTR,
    pattern_end: PCRE2_SPTR,
    out: &mut PcreOutputContext,
) -> c_int {
    unsafe {
        let mut start: PCRE2_SPTR = (*from).add(1);
        let mut pattern: PCRE2_SPTR = start;
        let mut class_ptr: usize; // index into POSIX_CLASSES
        let mut c: PCRE2_UCHAR;
        let mut class_index: c_int;

        loop {
            if pattern >= pattern_end {
                return 0;
            }
            c = *pattern;
            pattern = pattern.add(1);
            if c < CHAR_a || c > CHAR_z {
                break;
            }
        }

        if c != CHAR_COLON
            || pattern >= pattern_end
            || *pattern != CHAR_RIGHT_SQUARE_BRACKET
        {
            return 0;
        }

        class_ptr = 0;
        class_index = 1;

        loop {
            if POSIX_CLASSES[class_ptr] == 0 {
                return 0;
            }

            pattern = start;

            while *pattern == POSIX_CLASSES[class_ptr] {
                if *pattern == CHAR_COLON {
                    pattern = pattern.add(2);
                    start = start.sub(2);

                    // do { write(*start++); } while (start < pattern);
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
                class_ptr += 1;
            }

            while POSIX_CLASSES[class_ptr] != CHAR_COLON {
                class_ptr += 1;
            }
            class_ptr += 1;
            class_index += 1;
        }
    }
}

/// Checks whether the character is in the class.
///
/// Returns `!0` => character found in class, `0` => otherwise.
unsafe fn convert_glob_char_in_class(class_index: c_int, c: PCRE2_UCHAR) -> bool {
    unsafe {
        let cbits = tables::_pcre2_default_tables_8
            .as_ptr()
            .add(cbits_offset as usize);
        let cbit: i64;
        let c = c as usize;

        // In 8-bit mode c is always <= 0xff.

        match class_index {
            1 => {
                // alpha
                if c as u8 == CHAR_UNDERSCORE {
                    return false;
                }
                if (*cbits.add(cbit_digit as usize + c / 8) & (1u8 << (c & 7))) != 0 {
                    return false;
                }
                cbit = cbit_word;
            }
            2 => cbit = cbit_lower,
            3 => cbit = cbit_upper,
            4 => {
                // alnum
                if c as u8 == CHAR_UNDERSCORE {
                    return false;
                }
                cbit = cbit_word;
            }
            5 => {
                // ascii
                if (*cbits.add(cbit_cntrl as usize + c / 8) & (1u8 << (c & 7))) != 0 {
                    return true;
                }
                cbit = cbit_print;
            }
            6 => {
                // blank
                let cc = c as u8;
                if cc == CHAR_LF || cc == CHAR_VT || cc == CHAR_FF || cc == CHAR_CR {
                    return false;
                }
                cbit = cbit_space;
            }
            7 => cbit = cbit_cntrl,
            8 => cbit = cbit_digit,
            9 => cbit = cbit_graph,
            10 => cbit = cbit_print,
            11 => cbit = cbit_punct,
            12 => cbit = cbit_space,
            13 => cbit = cbit_word,
            14 => cbit = cbit_xdigit,
            _ => return false,
        }

        (*cbits.add(cbit as usize + c / 8) & (1u8 << (c & 7))) != 0
    }
}

/// Parse a range of characters.
///
/// Returns `0` => success, `!0` => error code.
unsafe fn convert_glob_parse_range(
    from: &mut PCRE2_SPTR,
    pattern_end: PCRE2_SPTR,
    out: &mut PcreOutputContext,
    utf: bool,
    separator: PCRE2_UCHAR,
    with_escape: bool,
    escape: PCRE2_UCHAR,
    no_wildsep: bool,
) -> c_int {
    unsafe {
        let mut is_negative = false;
        let mut separator_seen = false;
        let mut has_prev_c: bool;
        let mut pattern: PCRE2_SPTR = *from;
        let mut char_start: PCRE2_SPTR;
        let mut c: u32;
        let mut prev_c: u32;
        let mut len: c_int;
        let mut class_index: c_int;

        let _ = utf;

        if pattern >= pattern_end {
            *from = pattern;
            return PCRE2_ERROR_MISSING_SQUARE_BRACKET as c_int;
        }

        if *pattern == CHAR_EXCLAMATION_MARK || *pattern == CHAR_CIRCUMFLEX_ACCENT {
            pattern = pattern.add(1);

            if pattern >= pattern_end {
                *from = pattern;
                return PCRE2_ERROR_MISSING_SQUARE_BRACKET as c_int;
            }

            is_negative = true;

            out.out_str[0] = CHAR_LEFT_SQUARE_BRACKET;
            out.out_str[1] = CHAR_CIRCUMFLEX_ACCENT;
            len = 2;

            if !no_wildsep {
                if with_escape {
                    out.out_str[len as usize] = CHAR_BACKSLASH;
                    len += 1;
                }
                out.out_str[len as usize] = separator;
            }

            convert_glob_write_str(out, (len + 1) as PCRE2_SIZE);
        } else {
            convert_glob_write(out, CHAR_LEFT_SQUARE_BRACKET);
        }

        has_prev_c = false;
        prev_c = 0;

        if *pattern == CHAR_RIGHT_SQUARE_BRACKET {
            out.out_str[0] = CHAR_BACKSLASH;
            out.out_str[1] = CHAR_RIGHT_SQUARE_BRACKET;
            convert_glob_write_str(out, 2);
            has_prev_c = true;
            prev_c = CHAR_RIGHT_SQUARE_BRACKET as u32;
            pattern = pattern.add(1);
        }

        while pattern < pattern_end {
            char_start = pattern;
            c = GETCHARINCTEST(&mut pattern, utf);

            if c == CHAR_RIGHT_SQUARE_BRACKET as u32 {
                convert_glob_write(out, c as u8);

                if !is_negative && !no_wildsep && separator_seen {
                    out.out_str[0] = CHAR_LEFT_PARENTHESIS;
                    out.out_str[1] = CHAR_QUESTION_MARK;
                    out.out_str[2] = CHAR_LESS_THAN_SIGN;
                    out.out_str[3] = CHAR_EXCLAMATION_MARK;
                    convert_glob_write_str(out, 4);

                    convert_glob_print_separator(out, separator, with_escape);
                    convert_glob_write(out, CHAR_RIGHT_PARENTHESIS);
                }

                *from = pattern;
                return 0;
            }

            if pattern >= pattern_end {
                break;
            }

            if c == CHAR_LEFT_SQUARE_BRACKET as u32 && *pattern == CHAR_COLON {
                *from = pattern;
                class_index = convert_glob_parse_class(from, pattern_end, out);

                if class_index != 0 {
                    pattern = *from;

                    has_prev_c = false;
                    prev_c = 0;

                    if !is_negative
                        && convert_glob_char_in_class(class_index, separator)
                    {
                        separator_seen = true;
                    }
                    continue;
                }
            } else if c == CHAR_MINUS as u32
                && has_prev_c
                && *pattern != CHAR_RIGHT_SQUARE_BRACKET
            {
                convert_glob_write(out, CHAR_MINUS);

                char_start = pattern;
                c = GETCHARINCTEST(&mut pattern, utf);

                if pattern >= pattern_end {
                    break;
                }

                if escape != 0 && c == escape as u32 {
                    char_start = pattern;
                    c = GETCHARINCTEST(&mut pattern, utf);
                } else if c == CHAR_LEFT_SQUARE_BRACKET as u32 && *pattern == CHAR_COLON {
                    *from = pattern;
                    return PCRE2_ERROR_CONVERT_SYNTAX as c_int;
                }

                if prev_c > c {
                    *from = pattern;
                    return PCRE2_ERROR_CONVERT_SYNTAX as c_int;
                }

                if prev_c < separator as u32 && (separator as u32) < c {
                    separator_seen = true;
                }

                has_prev_c = false;
                prev_c = 0;
            } else {
                if escape != 0 && c == escape as u32 {
                    char_start = pattern;
                    c = GETCHARINCTEST(&mut pattern, utf);

                    if pattern >= pattern_end {
                        break;
                    }
                }

                has_prev_c = true;
                prev_c = c;
            }

            if c == CHAR_LEFT_SQUARE_BRACKET as u32
                || c == CHAR_RIGHT_SQUARE_BRACKET as u32
                || c == CHAR_BACKSLASH as u32
                || c == CHAR_MINUS as u32
            {
                convert_glob_write(out, CHAR_BACKSLASH);
            }

            if c == separator as u32 {
                separator_seen = true;
            }

            // do { write(*char_start++); } while (char_start < pattern);
            loop {
                convert_glob_write(out, *char_start);
                char_start = char_start.add(1);
                if char_start >= pattern {
                    break;
                }
            }
        }

        *from = pattern;
        PCRE2_ERROR_MISSING_SQUARE_BRACKET as c_int
    }
}

/// Prints a `(*COMMIT)` into the output.
#[inline]
unsafe fn convert_glob_print_commit(out: &mut PcreOutputContext) {
    unsafe {
        out.out_str[0] = CHAR_LEFT_PARENTHESIS;
        out.out_str[1] = CHAR_ASTERISK;
        out.out_str[2] = CHAR_C;
        out.out_str[3] = CHAR_O;
        out.out_str[4] = CHAR_M;
        out.out_str[5] = CHAR_M;
        out.out_str[6] = CHAR_I;
        out.out_str[7] = CHAR_T;
        convert_glob_write_str(out, 8);
        convert_glob_write(out, CHAR_RIGHT_PARENTHESIS);
    }
}

/// Bash glob converter.
unsafe fn convert_glob(
    options: u32,
    pattern: PCRE2_SPTR,
    plength: PCRE2_SIZE,
    utf: bool,
    use_buffer: *mut PCRE2_UCHAR,
    use_length: PCRE2_SIZE,
    bufflenptr: *mut PCRE2_SIZE,
    dummyrun: bool,
    ccontext: *mut pcre2_convert_context,
) -> c_int {
    unsafe {
        let mut out = PcreOutputContext {
            output: ptr::null_mut(),
            output_end: ptr::null(),
            output_size: 0,
            out_str: [0u8; 8],
        };
        let pattern_start: PCRE2_SPTR = pattern;
        let pattern_end: PCRE2_SPTR = pattern.add(plength);
        let separator: PCRE2_UCHAR = (*ccontext).glob_separator as u8;
        let escape: PCRE2_UCHAR = (*ccontext).glob_escape as u8;
        let mut c: PCRE2_UCHAR;
        let no_wildsep: bool =
            (options & PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR as u32) != 0;
        let no_starstar: bool = (options & PCRE2_CONVERT_GLOB_NO_STARSTAR as u32) != 0;
        let mut in_atomic: bool = false;
        let mut after_starstar: bool = false;
        let mut no_slash_z: bool = false;
        let with_escape: bool;
        let mut is_start: bool;
        let mut after_separator: bool;
        let mut result: c_int = 0;

        let mut pattern = pattern;

        // Note: separator/escape values come from the convert context and are
        // full u32 there; the C code stores them into PCRE2_UCHAR. We use u32
        // comparisons where the C uses PCRE2_UCHAR values.
        let sep_full = (*ccontext).glob_separator;
        let esc_full = (*ccontext).glob_escape;

        if utf && (sep_full >= 128 || esc_full >= 128) {
            // Currently only ASCII characters are supported.
            *bufflenptr = 0;
            return PCRE2_ERROR_CONVERT_SYNTAX as c_int;
        }

        with_escape = strchr_found(PCRE2_ESCAPED_LITERALS, separator);

        // Initialize default for error offset as end of input.
        out.output = use_buffer;
        out.output_end = use_buffer.add(use_length) as PCRE2_SPTR;
        out.output_size = 0;

        out.out_str[0] = CHAR_LEFT_PARENTHESIS;
        out.out_str[1] = CHAR_QUESTION_MARK;
        out.out_str[2] = CHAR_s;
        out.out_str[3] = CHAR_RIGHT_PARENTHESIS;
        convert_glob_write_str(&mut out, 4);

        is_start = true;

        if pattern < pattern_end && *pattern == CHAR_ASTERISK {
            if no_wildsep {
                is_start = false;
            } else if !no_starstar
                && pattern.add(1) < pattern_end
                && *pattern.add(1) == CHAR_ASTERISK
            {
                is_start = false;
            }
        }

        if is_start {
            out.out_str[0] = CHAR_BACKSLASH;
            out.out_str[1] = CHAR_A;
            convert_glob_write_str(&mut out, 2);
        }

        'outer: while pattern < pattern_end {
            c = *pattern;
            pattern = pattern.add(1);

            if c == CHAR_ASTERISK {
                is_start = pattern == pattern_start.add(1);

                if in_atomic {
                    convert_glob_write(&mut out, CHAR_RIGHT_PARENTHESIS);
                    in_atomic = false;
                }

                if !no_starstar && pattern < pattern_end && *pattern == CHAR_ASTERISK {
                    after_separator = is_start || (*pattern.sub(2) == separator);

                    // do pattern++; while (...)
                    loop {
                        pattern = pattern.add(1);
                        if !(pattern < pattern_end && *pattern == CHAR_ASTERISK) {
                            break;
                        }
                    }

                    if pattern >= pattern_end {
                        no_slash_z = true;
                        break 'outer;
                    }

                    after_starstar = true;

                    if after_separator
                        && escape != 0
                        && *pattern == escape
                        && pattern.add(1) < pattern_end
                        && *pattern.add(1) == separator
                    {
                        pattern = pattern.add(1);
                    }

                    if is_start {
                        if *pattern != separator {
                            continue;
                        }

                        out.out_str[0] = CHAR_LEFT_PARENTHESIS;
                        out.out_str[1] = CHAR_QUESTION_MARK;
                        out.out_str[2] = CHAR_COLON;
                        out.out_str[3] = CHAR_BACKSLASH;
                        out.out_str[4] = CHAR_A;
                        out.out_str[5] = CHAR_VERTICAL_LINE;
                        convert_glob_write_str(&mut out, 6);

                        convert_glob_print_separator(&mut out, separator, with_escape);
                        convert_glob_write(&mut out, CHAR_RIGHT_PARENTHESIS);

                        pattern = pattern.add(1);
                        continue;
                    }

                    convert_glob_print_commit(&mut out);

                    if !after_separator || *pattern != separator {
                        out.out_str[0] = CHAR_DOT;
                        out.out_str[1] = CHAR_ASTERISK;
                        out.out_str[2] = CHAR_QUESTION_MARK;
                        convert_glob_write_str(&mut out, 3);
                        continue;
                    }

                    out.out_str[0] = CHAR_LEFT_PARENTHESIS;
                    out.out_str[1] = CHAR_QUESTION_MARK;
                    out.out_str[2] = CHAR_COLON;
                    out.out_str[3] = CHAR_DOT;
                    out.out_str[4] = CHAR_ASTERISK;
                    out.out_str[5] = CHAR_QUESTION_MARK;

                    convert_glob_write_str(&mut out, 6);

                    convert_glob_print_separator(&mut out, separator, with_escape);

                    out.out_str[0] = CHAR_RIGHT_PARENTHESIS;
                    out.out_str[1] = CHAR_QUESTION_MARK;
                    out.out_str[2] = CHAR_QUESTION_MARK;
                    convert_glob_write_str(&mut out, 3);

                    pattern = pattern.add(1);
                    continue;
                }

                if pattern < pattern_end && *pattern == CHAR_ASTERISK {
                    loop {
                        pattern = pattern.add(1);
                        if !(pattern < pattern_end && *pattern == CHAR_ASTERISK) {
                            break;
                        }
                    }
                }

                if no_wildsep {
                    if pattern >= pattern_end {
                        no_slash_z = true;
                        break 'outer;
                    }

                    // Start check must be after the end check.
                    if is_start {
                        continue;
                    }
                }

                if !is_start {
                    if after_starstar {
                        out.out_str[0] = CHAR_LEFT_PARENTHESIS;
                        out.out_str[1] = CHAR_QUESTION_MARK;
                        out.out_str[2] = CHAR_GREATER_THAN_SIGN;
                        convert_glob_write_str(&mut out, 3);
                        in_atomic = true;
                    } else {
                        convert_glob_print_commit(&mut out);
                    }
                }

                if no_wildsep {
                    convert_glob_write(&mut out, CHAR_DOT);
                } else {
                    convert_glob_print_wildcard(&mut out, separator, with_escape);
                }

                out.out_str[0] = CHAR_ASTERISK;
                out.out_str[1] = CHAR_QUESTION_MARK;
                if pattern >= pattern_end {
                    out.out_str[1] = CHAR_PLUS;
                }
                convert_glob_write_str(&mut out, 2);
                continue;
            }

            if c == CHAR_QUESTION_MARK {
                if no_wildsep {
                    convert_glob_write(&mut out, CHAR_DOT);
                } else {
                    convert_glob_print_wildcard(&mut out, separator, with_escape);
                }
                continue;
            }

            if c == CHAR_LEFT_SQUARE_BRACKET {
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
                    break 'outer;
                }
                continue;
            }

            if escape != 0 && c == escape {
                if pattern >= pattern_end {
                    result = PCRE2_ERROR_CONVERT_SYNTAX as c_int;
                    break 'outer;
                }
                c = *pattern;
                pattern = pattern.add(1);
            }

            if (c as u32) < 255 && strchr_found(PCRE2_ESCAPED_LITERALS, c) {
                convert_glob_write(&mut out, CHAR_BACKSLASH);
            }

            convert_glob_write(&mut out, c);
        }

        if result == 0 {
            if !no_slash_z {
                out.out_str[0] = CHAR_BACKSLASH;
                out.out_str[1] = CHAR_z;
                convert_glob_write_str(&mut out, 2);
            }

            if in_atomic {
                convert_glob_write(&mut out, CHAR_RIGHT_PARENTHESIS);
            }

            convert_glob_write(&mut out, CHAR_NUL);

            if !dummyrun
                && out.output_size != (out.output as usize - use_buffer as usize) as PCRE2_SIZE
            {
                result = PCRE2_ERROR_NOMEMORY as c_int;
            }
        }

        if result != 0 {
            *bufflenptr = (pattern as usize - pattern_start as usize) as PCRE2_SIZE;
            return result;
        }

        *bufflenptr = out.output_size - 1;
        0
    }
}

// ---------------------------------------------------------------------------
// Public: convert pattern
// ---------------------------------------------------------------------------

/// `pcre2_pattern_convert()` — external-facing pattern conversion.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_pattern_convert_8(
    pattern: PCRE2_SPTR,
    plength: PCRE2_SIZE,
    options: u32,
    buffptr: *mut *mut PCRE2_UCHAR,
    bufflenptr: *mut PCRE2_SIZE,
    ccontext: *mut pcre2_convert_context,
) -> c_int {
    unsafe {
        let mut rc: c_int;
        let null_str: [PCRE2_UCHAR; 1] = [0xcd];
        let mut dummy_buffer: [PCRE2_UCHAR; DUMMY_BUFFER_SIZE] = [0; DUMMY_BUFFER_SIZE];
        let mut use_buffer: *mut PCRE2_UCHAR = dummy_buffer.as_mut_ptr();
        let mut use_length: PCRE2_SIZE = DUMMY_BUFFER_SIZE;
        let utf: bool = (options & PCRE2_CONVERT_UTF as u32) != 0;
        let pattype: u32 = options & TYPE_OPTIONS;

        let mut pattern = pattern;
        let mut ccontext = ccontext;

        if pattern.is_null() && plength == 0 {
            pattern = null_str.as_ptr();
        }

        if pattern.is_null() || bufflenptr.is_null() {
            if !bufflenptr.is_null() {
                *bufflenptr = 0; // Error offset
            }
            return PCRE2_ERROR_NULL as c_int;
        }

        if (options & !ALL_OPTIONS) != 0                 // Undefined bit set
            || (pattype & (!pattype).wrapping_add(1)) != pattype  // More than one type set
            || pattype == 0
        {
            *bufflenptr = 0; // Error offset
            return PCRE2_ERROR_BADOPTION as c_int;
        }

        let mut plength = plength;
        if plength == PCRE2_ZERO_TERMINATED {
            plength = crate::string_utils::_pcre2_strlen_8(pattern);
        }
        if ccontext.is_null() {
            ccontext = &raw mut crate::context::_pcre2_default_convert_context_8;
        }

        // Check UTF if required.
        if utf && (options & PCRE2_CONVERT_NO_UTF_CHECK as u32) == 0 {
            let mut erroroffset: PCRE2_SIZE = 0;
            rc = crate::valid_utf::_pcre2_valid_utf_8(pattern, plength, &mut erroroffset);
            if rc != 0 {
                *bufflenptr = erroroffset;
                return rc;
            }
        }

        // If buffptr is not NULL, and what it points to is not NULL, we are
        // being provided with a buffer and a length.
        if !buffptr.is_null() && !(*buffptr).is_null() {
            use_buffer = *buffptr;
            use_length = *bufflenptr;
        }

        // Call an individual converter, either once or twice.
        let mut _i = 0;
        while _i < 2 {
            let allocated: *mut PCRE2_UCHAR;
            let dummyrun: bool = buffptr.is_null() || (*buffptr).is_null();

            match pattype as i64 {
                PCRE2_CONVERT_GLOB => {
                    rc = convert_glob(
                        options & !(PCRE2_CONVERT_GLOB as u32),
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
                _ => {
                    // We have already validated pattype.
                    *bufflenptr = 0; // Error offset
                    return PCRE2_ERROR_INTERNAL as c_int;
                }
            }

            if rc != 0                       // Error
                || buffptr.is_null()          // Just the length is required
                || !(*buffptr).is_null()
            {
                return rc;
            }

            // Allocate memory for the buffer, with hidden space for an
            // allocator at the start.
            // In 8-bit mode the C `PCRE2_CODE_UNIT_WIDTH` macro expands to 8 and
            // is used directly in this expression (matching pcre2_convert.c).
            allocated = crate::context::_pcre2_memctl_malloc_8(
                core::mem::size_of::<pcre2_memctl>() + (*bufflenptr + 1) * 8,
                ccontext as *mut pcre2_memctl,
            ) as *mut PCRE2_UCHAR;
            if allocated.is_null() {
                *bufflenptr = 0; // Error offset
                return PCRE2_ERROR_NOMEMORY as c_int;
            }
            *buffptr = (allocated as *mut u8)
                .add(core::mem::size_of::<pcre2_memctl>())
                as *mut PCRE2_UCHAR;

            use_buffer = *buffptr;
            use_length = *bufflenptr + 1;

            _i += 1;
        }

        // Running the loop above ought to have succeeded the second time.
        *bufflenptr = 0; // Error offset
        PCRE2_ERROR_INTERNAL as c_int
    }
}

// ---------------------------------------------------------------------------
// Public: free converted pattern
// ---------------------------------------------------------------------------

/// `pcre2_converted_pattern_free()` — free a converted pattern that was placed
/// in newly-allocated memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_converted_pattern_free_8(converted: *mut PCRE2_UCHAR) {
    unsafe {
        if !converted.is_null() {
            let memctl = (converted as *mut u8).sub(core::mem::size_of::<pcre2_memctl>())
                as *mut pcre2_memctl;
            ((*memctl).free.unwrap())(memctl as *mut c_void, (*memctl).memory_data);
        }
    }
}
