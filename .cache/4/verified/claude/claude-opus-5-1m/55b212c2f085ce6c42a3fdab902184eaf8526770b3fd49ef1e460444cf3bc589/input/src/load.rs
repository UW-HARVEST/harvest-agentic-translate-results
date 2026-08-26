//! Translation of `src/load.c`.

#![allow(non_upper_case_globals)]

use crate::error::{jsonp_error_init, jsonp_error_set_msg};
use crate::memory::{jsonp_free, jsonp_malloc};
use crate::strbuffer::*;
use crate::strconv::jsonp_strtod;
use crate::types::*;
use crate::utf::{utf8_check_first, utf8_check_full, utf8_encode};
use crate::value::*;
use core::ffi::{c_char, c_int, c_void};

const STREAM_STATE_OK: c_int = 0;
const STREAM_STATE_EOF: c_int = -1;
const STREAM_STATE_ERROR: c_int = -2;

const TOKEN_INVALID: c_int = -1;
const TOKEN_EOF: c_int = 0;
const TOKEN_STRING: c_int = 256;
const TOKEN_INTEGER: c_int = 257;
const TOKEN_REAL: c_int = 258;
const TOKEN_TRUE: c_int = 259;
const TOKEN_FALSE: c_int = 260;
const TOKEN_NULL: c_int = 261;

/* Locale independent versions of isxxx() functions */
#[inline]
fn l_isupper(c: c_int) -> bool {
    'A' as c_int <= c && c <= 'Z' as c_int
}
#[inline]
fn l_islower(c: c_int) -> bool {
    'a' as c_int <= c && c <= 'z' as c_int
}
#[inline]
fn l_isalpha(c: c_int) -> bool {
    l_isupper(c) || l_islower(c)
}
#[inline]
fn l_isdigit(c: c_int) -> bool {
    '0' as c_int <= c && c <= '9' as c_int
}
#[inline]
fn l_isxdigit(c: c_int) -> bool {
    l_isdigit(c) || ('A' as c_int <= c && c <= 'F' as c_int) || ('a' as c_int <= c && c <= 'f' as c_int)
}

type get_func = Option<unsafe extern "C" fn(data: *mut c_void) -> c_int>;

#[repr(C)]
struct stream_t {
    get: get_func,
    data: *mut c_void,
    buffer: [c_char; 5],
    buffer_pos: usize,
    state: c_int,
    line: c_int,
    column: c_int,
    last_column: c_int,
    position: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct lex_string {
    val: *mut c_char,
    len: usize,
}

#[repr(C)]
union lex_value {
    string: lex_string,
    integer: json_int_t,
    real: f64,
}

#[repr(C)]
struct lex_t {
    stream: stream_t,
    saved_text: strbuffer_t,
    flags: usize,
    depth: usize,
    token: c_int,
    value: lex_value,
}

impl lex_t {
    fn zeroed() -> lex_t {
        lex_t {
            stream: stream_t {
                get: None,
                data: core::ptr::null_mut(),
                buffer: [0; 5],
                buffer_pos: 0,
                state: 0,
                line: 0,
                column: 0,
                last_column: 0,
                position: 0,
            },
            saved_text: strbuffer_t {
                value: core::ptr::null_mut(),
                length: 0,
                size: 0,
            },
            flags: 0,
            depth: 0,
            token: 0,
            value: lex_value {
                string: lex_string {
                    val: core::ptr::null_mut(),
                    len: 0,
                },
            },
        }
    }
}

#[inline]
unsafe fn stream_to_lex(stream: *mut stream_t) -> *mut lex_t {
    stream as *mut lex_t
}

/* ---------------------------------------------------------- error reporting */

/// Bytes of a NUL-terminated C string, up to `max` bytes.
unsafe fn cstr_bytes<'a>(p: *const c_char, max: usize) -> &'a [u8] {
    if p.is_null() {
        return &[];
    }
    let mut n = 0usize;
    while n < max && *p.add(n) != 0 {
        n += 1;
    }
    core::slice::from_raw_parts(p as *const u8, n)
}

unsafe fn error_set(error: *mut json_error_t, lex: *const lex_t, code_in: c_int, msg: &[u8]) {
    let mut code = code_in;
    let mut msg_text = [0u8; JSON_ERROR_TEXT_LENGTH];
    let mut msg_with_context = [0u8; JSON_ERROR_TEXT_LENGTH];

    let mut line: c_int = -1;
    let mut col: c_int = -1;
    let mut pos: usize = 0;
    let mut use_context = false;

    if error.is_null() {
        return;
    }

    /* vsnprintf(msg_text, JSON_ERROR_TEXT_LENGTH, msg, ap) */
    copy_trunc(&mut msg_text, msg);
    msg_text[JSON_ERROR_TEXT_LENGTH - 1] = 0;
    let msg_len = cstr_len(&msg_text);

    if !lex.is_null() {
        let saved_text = (*lex).saved_text.value;

        line = (*lex).stream.line;
        col = (*lex).stream.column;
        pos = (*lex).stream.position;

        if !saved_text.is_null() && *saved_text != 0 {
            if (*lex).saved_text.length <= 20 {
                /* snprintf(msg_with_context, N, "%s near '%s'", msg_text, saved_text) */
                let mut tmp: Vec<u8> = Vec::with_capacity(msg_len + 32);
                tmp.extend_from_slice(&msg_text[..msg_len]);
                tmp.extend_from_slice(b" near '");
                tmp.extend_from_slice(cstr_bytes(saved_text, 64));
                tmp.push(b'\'');
                copy_trunc(&mut msg_with_context, &tmp);
                msg_with_context[JSON_ERROR_TEXT_LENGTH - 1] = 0;
                use_context = true;
            }
        } else {
            if code == JSON_ERROR_INVALID_SYNTAX {
                /* More specific error code for premature end of file. */
                code = JSON_ERROR_PREMATURE_END_OF_INPUT;
            }
            if (*lex).stream.state == STREAM_STATE_ERROR {
                /* No context for UTF-8 decoding errors */
                use_context = false;
            } else {
                let mut tmp: Vec<u8> = Vec::with_capacity(msg_len + 32);
                tmp.extend_from_slice(&msg_text[..msg_len]);
                tmp.extend_from_slice(b" near end of file");
                copy_trunc(&mut msg_with_context, &tmp);
                msg_with_context[JSON_ERROR_TEXT_LENGTH - 1] = 0;
                use_context = true;
            }
        }
    }

    let result: &[u8] = if use_context {
        &msg_with_context[..]
    } else {
        &msg_text[..]
    };

    jsonp_error_set_msg(error, line, col, pos, code, result);
}

/* ------------------------------------------------------- lexical analyzer */

unsafe fn stream_init(stream: *mut stream_t, get: get_func, data: *mut c_void) {
    (*stream).get = get;
    (*stream).data = data;
    (*stream).buffer[0] = 0;
    (*stream).buffer_pos = 0;

    (*stream).state = STREAM_STATE_OK;
    (*stream).line = 1;
    (*stream).column = 0;
    (*stream).position = 0;
}

unsafe fn stream_get(stream: *mut stream_t, error: *mut json_error_t) -> c_int {
    let mut c: c_int;

    if (*stream).state != STREAM_STATE_OK {
        return (*stream).state;
    }

    if (*stream).buffer[(*stream).buffer_pos] == 0 {
        c = ((*stream).get.unwrap())((*stream).data);
        if c == EOF {
            (*stream).state = STREAM_STATE_EOF;
            return STREAM_STATE_EOF;
        }

        (*stream).buffer[0] = c as c_char;
        (*stream).buffer_pos = 0;

        if (0x80..=0xFF).contains(&c) {
            /* multi-byte UTF-8 sequence */
            let mut i: usize;
            let count: usize;

            count = utf8_check_first(c as c_char);
            if count == 0 {
                /* out: */
                (*stream).state = STREAM_STATE_ERROR;
                let msg = format!("unable to decode byte 0x{:x}", c);
                error_set(
                    error,
                    stream_to_lex(stream),
                    JSON_ERROR_INVALID_UTF8,
                    msg.as_bytes(),
                );
                return STREAM_STATE_ERROR;
            }

            debug_assert!(count >= 2);

            i = 1;
            while i < count {
                (*stream).buffer[i] = ((*stream).get.unwrap())((*stream).data) as c_char;
                i += 1;
            }

            if utf8_check_full((*stream).buffer.as_ptr(), count, core::ptr::null_mut()) == 0 {
                /* out: */
                (*stream).state = STREAM_STATE_ERROR;
                let msg = format!("unable to decode byte 0x{:x}", c);
                error_set(
                    error,
                    stream_to_lex(stream),
                    JSON_ERROR_INVALID_UTF8,
                    msg.as_bytes(),
                );
                return STREAM_STATE_ERROR;
            }

            (*stream).buffer[count] = 0;
        } else {
            (*stream).buffer[1] = 0;
        }
    }

    c = (*stream).buffer[(*stream).buffer_pos] as c_int;
    (*stream).buffer_pos += 1;

    (*stream).position += 1;
    if c == '\n' as c_int {
        (*stream).line += 1;
        (*stream).last_column = (*stream).column;
        (*stream).column = 0;
    } else if utf8_check_first(c as c_char) != 0 {
        /* track the Unicode character column, so increment only if
        this is the first character of a UTF-8 sequence */
        (*stream).column += 1;
    }

    c
}

unsafe fn stream_unget(stream: *mut stream_t, c: c_int) {
    if c == STREAM_STATE_EOF || c == STREAM_STATE_ERROR {
        return;
    }

    (*stream).position -= 1;
    if c == '\n' as c_int {
        (*stream).line -= 1;
        (*stream).column = (*stream).last_column;
    } else if utf8_check_first(c as c_char) != 0 {
        (*stream).column -= 1;
    }

    debug_assert!((*stream).buffer_pos > 0);
    (*stream).buffer_pos -= 1;
    debug_assert!((*stream).buffer[(*stream).buffer_pos] as c_int == c);
}

unsafe fn lex_get(lex: *mut lex_t, error: *mut json_error_t) -> c_int {
    stream_get(core::ptr::addr_of_mut!((*lex).stream), error)
}

unsafe fn lex_save(lex: *mut lex_t, c: c_int) {
    strbuffer_append_byte(core::ptr::addr_of_mut!((*lex).saved_text), c as c_char);
}

unsafe fn lex_get_save(lex: *mut lex_t, error: *mut json_error_t) -> c_int {
    let c = stream_get(core::ptr::addr_of_mut!((*lex).stream), error);
    if c != STREAM_STATE_EOF && c != STREAM_STATE_ERROR {
        lex_save(lex, c);
    }
    c
}

unsafe fn lex_unget(lex: *mut lex_t, c: c_int) {
    stream_unget(core::ptr::addr_of_mut!((*lex).stream), c);
}

unsafe fn lex_unget_unsave(lex: *mut lex_t, c: c_int) {
    if c != STREAM_STATE_EOF && c != STREAM_STATE_ERROR {
        stream_unget(core::ptr::addr_of_mut!((*lex).stream), c);
        let d = strbuffer_pop(core::ptr::addr_of_mut!((*lex).saved_text));
        debug_assert!(c == d as c_int);
        let _ = d;
    }
}

unsafe fn lex_save_cached(lex: *mut lex_t) {
    while (*lex).stream.buffer[(*lex).stream.buffer_pos] != 0 {
        lex_save(lex, (*lex).stream.buffer[(*lex).stream.buffer_pos] as c_int);
        (*lex).stream.buffer_pos += 1;
        (*lex).stream.position += 1;
    }
}

unsafe fn lex_free_string(lex: *mut lex_t) {
    jsonp_free((*lex).value.string.val as *mut c_void);
    (*lex).value.string.val = core::ptr::null_mut();
    (*lex).value.string.len = 0;
}

/* assumes that str points to 'u' plus at least 4 valid hex digits */
unsafe fn decode_unicode_escape(str_: *const c_char) -> i32 {
    let mut i: c_int;
    let mut value: i32 = 0;

    debug_assert!(*str_ == 'u' as c_char);

    i = 1;
    while i <= 4 {
        let c = *str_.offset(i as isize) as c_int;
        value <<= 4;
        if l_isdigit(c) {
            value += (c - '0' as c_int) as i32;
        } else if l_islower(c) {
            value += (c - 'a' as c_int + 10) as i32;
        } else if l_isupper(c) {
            value += (c - 'A' as c_int + 10) as i32;
        } else {
            return -1;
        }
        i += 1;
    }

    value
}

unsafe fn lex_scan_string(lex: *mut lex_t, error: *mut json_error_t) {
    let mut c: c_int;
    let mut p: *const c_char;
    let mut t: *mut c_char;
    let mut i: c_int;

    (*lex).value.string.val = core::ptr::null_mut();
    (*lex).token = TOKEN_INVALID;

    c = lex_get_save(lex, error);

    'out: {
        while c != '"' as c_int {
            if c == STREAM_STATE_ERROR {
                break 'out;
            } else if c == STREAM_STATE_EOF {
                error_set(
                    error,
                    lex,
                    JSON_ERROR_PREMATURE_END_OF_INPUT,
                    b"premature end of input",
                );
                break 'out;
            } else if (0..=0x1F).contains(&c) {
                /* control character */
                lex_unget_unsave(lex, c);
                if c == '\n' as c_int {
                    error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"unexpected newline");
                } else {
                    let msg = format!("control character 0x{:x}", c);
                    error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, msg.as_bytes());
                }
                break 'out;
            } else if c == '\\' as c_int {
                c = lex_get_save(lex, error);
                if c == 'u' as c_int {
                    c = lex_get_save(lex, error);
                    i = 0;
                    while i < 4 {
                        if !l_isxdigit(c) {
                            error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"invalid escape");
                            break 'out;
                        }
                        c = lex_get_save(lex, error);
                        i += 1;
                    }
                } else if c == '"' as c_int
                    || c == '\\' as c_int
                    || c == '/' as c_int
                    || c == 'b' as c_int
                    || c == 'f' as c_int
                    || c == 'n' as c_int
                    || c == 'r' as c_int
                    || c == 't' as c_int
                {
                    c = lex_get_save(lex, error);
                } else {
                    error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"invalid escape");
                    break 'out;
                }
            } else {
                c = lex_get_save(lex, error);
            }
        }

        /* the actual value is at most of the same length as the source
        string, because:
          - shortcut escapes (e.g. "\t") (length 2) are converted to 1 byte
          - a single \uXXXX escape (length 6) is converted to at most 3 bytes
          - two \uXXXX escapes (length 12) forming an UTF-16 surrogate pair
            are converted to 4 bytes
        */
        t = jsonp_malloc((*lex).saved_text.length + 1) as *mut c_char;
        if t.is_null() {
            /* this is not very nice, since TOKEN_INVALID is returned */
            break 'out;
        }
        (*lex).value.string.val = t;

        /* + 1 to skip the " */
        p = (*lex).saved_text.value.add(1);

        while *p != '"' as c_char {
            if *p == '\\' as c_char {
                p = p.add(1);
                if *p == 'u' as c_char {
                    let mut length: usize = 0;
                    let mut value: i32;

                    value = decode_unicode_escape(p);
                    if value < 0 {
                        let mut msg: Vec<u8> = Vec::new();
                        msg.extend_from_slice(b"invalid Unicode escape '");
                        msg.extend_from_slice(cstr_bytes(p.offset(-1), 6));
                        msg.push(b'\'');
                        error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, &msg);
                        break 'out;
                    }
                    p = p.add(5);

                    if (0xD800..=0xDBFF).contains(&value) {
                        /* surrogate pair */
                        if *p == '\\' as c_char && *p.add(1) == 'u' as c_char {
                            p = p.add(1);
                            let value2 = decode_unicode_escape(p);
                            if value2 < 0 {
                                let mut msg: Vec<u8> = Vec::new();
                                msg.extend_from_slice(b"invalid Unicode escape '");
                                msg.extend_from_slice(cstr_bytes(p.offset(-1), 6));
                                msg.push(b'\'');
                                error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, &msg);
                                break 'out;
                            }
                            p = p.add(5);

                            if (0xDC00..=0xDFFF).contains(&value2) {
                                /* valid second surrogate */
                                value = ((value - 0xD800) << 10) + (value2 - 0xDC00) + 0x10000;
                            } else {
                                /* invalid second surrogate */
                                let msg = format!(
                                    "invalid Unicode '\\u{:04X}\\u{:04X}'",
                                    value as u32, value2 as u32
                                );
                                error_set(
                                    error,
                                    lex,
                                    JSON_ERROR_INVALID_SYNTAX,
                                    msg.as_bytes(),
                                );
                                break 'out;
                            }
                        } else {
                            /* no second surrogate */
                            let msg = format!("invalid Unicode '\\u{:04X}'", value as u32);
                            error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, msg.as_bytes());
                            break 'out;
                        }
                    } else if (0xDC00..=0xDFFF).contains(&value) {
                        let msg = format!("invalid Unicode '\\u{:04X}'", value as u32);
                        error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, msg.as_bytes());
                        break 'out;
                    }

                    if utf8_encode(value, t, &mut length) != 0 {
                        debug_assert!(false);
                    }
                    t = t.add(length);
                } else {
                    match *p as u8 {
                        b'"' | b'\\' | b'/' => *t = *p,
                        b'b' => *t = 0x08,
                        b'f' => *t = 0x0c,
                        b'n' => *t = 0x0a,
                        b'r' => *t = 0x0d,
                        b't' => *t = 0x09,
                        _ => debug_assert!(false),
                    }
                    t = t.add(1);
                    p = p.add(1);
                }
            } else {
                *t = *p;
                t = t.add(1);
                p = p.add(1);
            }
        }
        *t = 0;
        (*lex).value.string.len = t.offset_from((*lex).value.string.val) as usize;
        (*lex).token = TOKEN_STRING;
        return;
    }

    /* out: */
    lex_free_string(lex);
}

unsafe fn lex_scan_number(lex: *mut lex_t, c_in: c_int, error: *mut json_error_t) -> c_int {
    let saved_text: *const c_char;
    let mut end: *mut c_char = core::ptr::null_mut();
    let mut doubleval: f64 = 0.0;
    let mut c = c_in;

    (*lex).token = TOKEN_INVALID;

    if c == '-' as c_int {
        c = lex_get_save(lex, error);
    }

    if c == '0' as c_int {
        c = lex_get_save(lex, error);
        if l_isdigit(c) {
            lex_unget_unsave(lex, c);
            return -1;
        }
    } else if l_isdigit(c) {
        loop {
            c = lex_get_save(lex, error);
            if !l_isdigit(c) {
                break;
            }
        }
    } else {
        lex_unget_unsave(lex, c);
        return -1;
    }

    if ((*lex).flags & JSON_DECODE_INT_AS_REAL) == 0
        && c != '.' as c_int
        && c != 'E' as c_int
        && c != 'e' as c_int
    {
        let intval: json_int_t;

        lex_unget_unsave(lex, c);

        saved_text = (*lex).saved_text.value;

        set_errno(0);
        intval = strtoll(saved_text, &mut end, 10);
        if get_errno() == ERANGE {
            if intval < 0 {
                error_set(
                    error,
                    lex,
                    JSON_ERROR_NUMERIC_OVERFLOW,
                    b"too big negative integer",
                );
            } else {
                error_set(error, lex, JSON_ERROR_NUMERIC_OVERFLOW, b"too big integer");
            }
            return -1;
        }

        debug_assert!(end == (saved_text as *mut c_char).add((*lex).saved_text.length));

        (*lex).token = TOKEN_INTEGER;
        (*lex).value.integer = intval;
        return 0;
    }

    if c == '.' as c_int {
        c = lex_get(lex, error);
        if !l_isdigit(c) {
            lex_unget(lex, c);
            return -1;
        }
        lex_save(lex, c);

        loop {
            c = lex_get_save(lex, error);
            if !l_isdigit(c) {
                break;
            }
        }
    }

    if c == 'E' as c_int || c == 'e' as c_int {
        c = lex_get_save(lex, error);
        if c == '+' as c_int || c == '-' as c_int {
            c = lex_get_save(lex, error);
        }

        if !l_isdigit(c) {
            lex_unget_unsave(lex, c);
            return -1;
        }

        loop {
            c = lex_get_save(lex, error);
            if !l_isdigit(c) {
                break;
            }
        }
    }

    lex_unget_unsave(lex, c);

    if jsonp_strtod(core::ptr::addr_of_mut!((*lex).saved_text), &mut doubleval) != 0 {
        error_set(
            error,
            lex,
            JSON_ERROR_NUMERIC_OVERFLOW,
            b"real number overflow",
        );
        return -1;
    }

    (*lex).token = TOKEN_REAL;
    (*lex).value.real = doubleval;
    0
}

unsafe fn lex_scan(lex: *mut lex_t, error: *mut json_error_t) -> c_int {
    let mut c: c_int;

    strbuffer_clear(core::ptr::addr_of_mut!((*lex).saved_text));

    if (*lex).token == TOKEN_STRING {
        lex_free_string(lex);
    }

    loop {
        c = lex_get(lex, error);
        if !(c == ' ' as c_int || c == '\t' as c_int || c == '\n' as c_int || c == '\r' as c_int) {
            break;
        }
    }

    if c == STREAM_STATE_EOF {
        (*lex).token = TOKEN_EOF;
        return (*lex).token;
    }

    if c == STREAM_STATE_ERROR {
        (*lex).token = TOKEN_INVALID;
        return (*lex).token;
    }

    lex_save(lex, c);

    if c == '{' as c_int
        || c == '}' as c_int
        || c == '[' as c_int
        || c == ']' as c_int
        || c == ':' as c_int
        || c == ',' as c_int
    {
        (*lex).token = c;
    } else if c == '"' as c_int {
        lex_scan_string(lex, error);
    } else if l_isdigit(c) || c == '-' as c_int {
        if lex_scan_number(lex, c, error) != 0 {
            return (*lex).token;
        }
    } else if l_isalpha(c) {
        /* eat up the whole identifier for clearer error messages */
        let saved_text: *const c_char;

        loop {
            c = lex_get_save(lex, error);
            if !l_isalpha(c) {
                break;
            }
        }
        lex_unget_unsave(lex, c);

        saved_text = (*lex).saved_text.value;

        if strcmp(saved_text, b"true\0".as_ptr() as *const c_char) == 0 {
            (*lex).token = TOKEN_TRUE;
        } else if strcmp(saved_text, b"false\0".as_ptr() as *const c_char) == 0 {
            (*lex).token = TOKEN_FALSE;
        } else if strcmp(saved_text, b"null\0".as_ptr() as *const c_char) == 0 {
            (*lex).token = TOKEN_NULL;
        } else {
            (*lex).token = TOKEN_INVALID;
        }
    } else {
        /* save the rest of the input UTF-8 sequence to get an error
        message of valid UTF-8 */
        lex_save_cached(lex);
        (*lex).token = TOKEN_INVALID;
    }

    (*lex).token
}

unsafe fn lex_steal_string(lex: *mut lex_t, out_len: *mut usize) -> *mut c_char {
    let mut result: *mut c_char = core::ptr::null_mut();
    if (*lex).token == TOKEN_STRING {
        result = (*lex).value.string.val;
        *out_len = (*lex).value.string.len;
        (*lex).value.string.val = core::ptr::null_mut();
        (*lex).value.string.len = 0;
    }
    result
}

unsafe fn lex_init(lex: *mut lex_t, get: get_func, flags: usize, data: *mut c_void) -> c_int {
    stream_init(core::ptr::addr_of_mut!((*lex).stream), get, data);
    if strbuffer_init(core::ptr::addr_of_mut!((*lex).saved_text)) != 0 {
        return -1;
    }

    (*lex).flags = flags;
    (*lex).token = TOKEN_INVALID;
    0
}

unsafe fn lex_close(lex: *mut lex_t) {
    if (*lex).token == TOKEN_STRING {
        lex_free_string(lex);
    }
    strbuffer_close(core::ptr::addr_of_mut!((*lex).saved_text));
}

/* ---------------------------------------------------------------- parser */

unsafe fn parse_object(lex: *mut lex_t, flags: usize, error: *mut json_error_t) -> *mut json_t {
    let object = json_object();
    if object.is_null() {
        return core::ptr::null_mut();
    }

    lex_scan(lex, error);
    if (*lex).token == '}' as c_int {
        return object;
    }

    'error: {
        loop {
            let key: *mut c_char;
            let mut len: usize = 0;
            let value: *mut json_t;

            if (*lex).token != TOKEN_STRING {
                error_set(
                    error,
                    lex,
                    JSON_ERROR_INVALID_SYNTAX,
                    b"string or '}' expected",
                );
                break 'error;
            }

            key = lex_steal_string(lex, &mut len);
            if key.is_null() {
                return core::ptr::null_mut();
            }
            if !memchr(key as *const c_void, 0, len).is_null() {
                jsonp_free(key as *mut c_void);
                error_set(
                    error,
                    lex,
                    JSON_ERROR_NULL_BYTE_IN_KEY,
                    b"NUL byte in object key not supported",
                );
                break 'error;
            }

            if (flags & JSON_REJECT_DUPLICATES) != 0
                && !json_object_getn(object, key, len).is_null()
            {
                jsonp_free(key as *mut c_void);
                error_set(
                    error,
                    lex,
                    JSON_ERROR_DUPLICATE_KEY,
                    b"duplicate object key",
                );
                break 'error;
            }

            lex_scan(lex, error);
            if (*lex).token != ':' as c_int {
                jsonp_free(key as *mut c_void);
                error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"':' expected");
                break 'error;
            }

            lex_scan(lex, error);
            value = parse_value(lex, flags, error);
            if value.is_null() {
                jsonp_free(key as *mut c_void);
                break 'error;
            }

            if json_object_setn_new_nocheck(object, key, len, value) != 0 {
                jsonp_free(key as *mut c_void);
                break 'error;
            }

            jsonp_free(key as *mut c_void);

            lex_scan(lex, error);
            if (*lex).token != ',' as c_int {
                break;
            }

            lex_scan(lex, error);
        }

        if (*lex).token != '}' as c_int {
            error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"'}' expected");
            break 'error;
        }

        return object;
    }

    /* error: */
    json_decref(object);
    core::ptr::null_mut()
}

unsafe fn parse_array(lex: *mut lex_t, flags: usize, error: *mut json_error_t) -> *mut json_t {
    let array = json_array();
    if array.is_null() {
        return core::ptr::null_mut();
    }

    lex_scan(lex, error);
    if (*lex).token == ']' as c_int {
        return array;
    }

    'error: {
        while (*lex).token != 0 {
            let elem = parse_value(lex, flags, error);
            if elem.is_null() {
                break 'error;
            }

            if json_array_append_new(array, elem) != 0 {
                break 'error;
            }

            lex_scan(lex, error);
            if (*lex).token != ',' as c_int {
                break;
            }

            lex_scan(lex, error);
        }

        if (*lex).token != ']' as c_int {
            error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"']' expected");
            break 'error;
        }

        return array;
    }

    /* error: */
    json_decref(array);
    core::ptr::null_mut()
}

unsafe fn parse_value(lex: *mut lex_t, flags: usize, error: *mut json_error_t) -> *mut json_t {
    let json: *mut json_t;

    (*lex).depth += 1;
    if (*lex).depth > JSON_PARSER_MAX_DEPTH {
        error_set(
            error,
            lex,
            JSON_ERROR_STACK_OVERFLOW,
            b"maximum parsing depth reached",
        );
        return core::ptr::null_mut();
    }

    match (*lex).token {
        TOKEN_STRING => {
            let value = (*lex).value.string.val;
            let len = (*lex).value.string.len;

            if (flags & JSON_ALLOW_NUL) == 0 && !memchr(value as *const c_void, 0, len).is_null() {
                error_set(
                    error,
                    lex,
                    JSON_ERROR_NULL_CHARACTER,
                    b"\\u0000 is not allowed without JSON_ALLOW_NUL",
                );
                return core::ptr::null_mut();
            }

            json = jsonp_stringn_nocheck_own(value, len);
            (*lex).value.string.val = core::ptr::null_mut();
            (*lex).value.string.len = 0;
        }

        TOKEN_INTEGER => {
            json = json_integer((*lex).value.integer);
        }

        TOKEN_REAL => {
            json = json_real((*lex).value.real);
        }

        TOKEN_TRUE => json = json_true(),

        TOKEN_FALSE => json = json_false(),

        TOKEN_NULL => json = json_null(),

        0x7b /* '{' */ => {
            json = parse_object(lex, flags, error);
        }

        0x5b /* '[' */ => {
            json = parse_array(lex, flags, error);
        }

        TOKEN_INVALID => {
            error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"invalid token");
            return core::ptr::null_mut();
        }

        _ => {
            error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"unexpected token");
            return core::ptr::null_mut();
        }
    }

    if json.is_null() {
        return core::ptr::null_mut();
    }

    (*lex).depth -= 1;
    json
}

unsafe fn parse_json(lex: *mut lex_t, flags: usize, error: *mut json_error_t) -> *mut json_t {
    let result: *mut json_t;

    (*lex).depth = 0;

    lex_scan(lex, error);
    if (flags & JSON_DECODE_ANY) == 0
        && (*lex).token != '[' as c_int
        && (*lex).token != '{' as c_int
    {
        error_set(
            error,
            lex,
            JSON_ERROR_INVALID_SYNTAX,
            b"'[' or '{' expected",
        );
        return core::ptr::null_mut();
    }

    result = parse_value(lex, flags, error);
    if result.is_null() {
        return core::ptr::null_mut();
    }

    if (flags & JSON_DISABLE_EOF_CHECK) == 0 {
        lex_scan(lex, error);
        if (*lex).token != TOKEN_EOF {
            error_set(
                error,
                lex,
                JSON_ERROR_END_OF_INPUT_EXPECTED,
                b"end of file expected",
            );
            json_decref(result);
            return core::ptr::null_mut();
        }
    }

    if !error.is_null() {
        /* Save the position even though there was no error */
        (*error).position = (*lex).stream.position as c_int;
    }

    result
}

/* --------------------------------------------------------------- json_loads */

#[repr(C)]
struct string_data_t {
    data: *const c_char,
    pos: usize,
}

unsafe extern "C" fn string_get(data: *mut c_void) -> c_int {
    let c: c_char;
    let stream = data as *mut string_data_t;
    c = *(*stream).data.add((*stream).pos);
    if c == 0 {
        EOF
    } else {
        (*stream).pos += 1;
        c as u8 as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loads(
    string: *const c_char,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    let mut lex = lex_t::zeroed();
    let result: *mut json_t;
    let mut stream_data = string_data_t {
        data: core::ptr::null(),
        pos: 0,
    };

    jsonp_error_init(error, b"<string>\0".as_ptr() as *const c_char);

    if string.is_null() {
        error_set(
            error,
            core::ptr::null(),
            JSON_ERROR_INVALID_ARGUMENT,
            b"wrong arguments",
        );
        return core::ptr::null_mut();
    }

    stream_data.data = string;
    stream_data.pos = 0;

    if lex_init(
        &mut lex,
        Some(string_get),
        flags,
        &mut stream_data as *mut string_data_t as *mut c_void,
    ) != 0
    {
        return core::ptr::null_mut();
    }

    result = parse_json(&mut lex, flags, error);

    lex_close(&mut lex);
    result
}

#[repr(C)]
struct buffer_data_t {
    data: *const c_char,
    len: usize,
    pos: usize,
}

unsafe extern "C" fn buffer_get(data: *mut c_void) -> c_int {
    let c: c_char;
    let stream = data as *mut buffer_data_t;
    if (*stream).pos >= (*stream).len {
        return EOF;
    }

    c = *(*stream).data.add((*stream).pos);
    (*stream).pos += 1;
    c as u8 as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadb(
    buffer: *const c_char,
    buflen: usize,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    let mut lex = lex_t::zeroed();
    let result: *mut json_t;
    let mut stream_data = buffer_data_t {
        data: core::ptr::null(),
        len: 0,
        pos: 0,
    };

    jsonp_error_init(error, b"<buffer>\0".as_ptr() as *const c_char);

    if buffer.is_null() {
        error_set(
            error,
            core::ptr::null(),
            JSON_ERROR_INVALID_ARGUMENT,
            b"wrong arguments",
        );
        return core::ptr::null_mut();
    }

    stream_data.data = buffer;
    stream_data.pos = 0;
    stream_data.len = buflen;

    if lex_init(
        &mut lex,
        Some(buffer_get),
        flags,
        &mut stream_data as *mut buffer_data_t as *mut c_void,
    ) != 0
    {
        return core::ptr::null_mut();
    }

    result = parse_json(&mut lex, flags, error);

    lex_close(&mut lex);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadf(
    input: *mut FILE,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    let mut lex = lex_t::zeroed();
    let source: *const c_char;
    let result: *mut json_t;

    if input == stdin {
        source = b"<stdin>\0".as_ptr() as *const c_char;
    } else {
        source = b"<stream>\0".as_ptr() as *const c_char;
    }

    jsonp_error_init(error, source);

    if input.is_null() {
        error_set(
            error,
            core::ptr::null(),
            JSON_ERROR_INVALID_ARGUMENT,
            b"wrong arguments",
        );
        return core::ptr::null_mut();
    }

    if lex_init(&mut lex, Some(fgetc), flags, input) != 0 {
        return core::ptr::null_mut();
    }

    result = parse_json(&mut lex, flags, error);

    lex_close(&mut lex);
    result
}

unsafe extern "C" fn fd_get_func(fd: *mut c_void) -> c_int {
    let fd = fd as *mut c_int;
    let mut c: u8 = 0;
    if read(*fd, &mut c as *mut u8 as *mut c_void, 1) == 1 {
        return c as c_int;
    }
    EOF
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadfd(
    input: c_int,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    let mut lex = lex_t::zeroed();
    let source: *const c_char;
    let result: *mut json_t;
    let mut input = input;

    if input == STDIN_FILENO {
        source = b"<stdin>\0".as_ptr() as *const c_char;
    } else {
        source = b"<stream>\0".as_ptr() as *const c_char;
    }

    jsonp_error_init(error, source);

    if input < 0 {
        error_set(
            error,
            core::ptr::null(),
            JSON_ERROR_INVALID_ARGUMENT,
            b"wrong arguments",
        );
        return core::ptr::null_mut();
    }

    if lex_init(
        &mut lex,
        Some(fd_get_func),
        flags,
        &mut input as *mut c_int as *mut c_void,
    ) != 0
    {
        return core::ptr::null_mut();
    }

    result = parse_json(&mut lex, flags, error);

    lex_close(&mut lex);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_load_file(
    path: *const c_char,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    let result: *mut json_t;
    let fp: *mut FILE;

    jsonp_error_init(error, path);

    if path.is_null() {
        error_set(
            error,
            core::ptr::null(),
            JSON_ERROR_INVALID_ARGUMENT,
            b"wrong arguments",
        );
        return core::ptr::null_mut();
    }

    fp = fopen(path, b"rb\0".as_ptr() as *const c_char);
    if fp.is_null() {
        let mut msg = [0u8; JSON_ERROR_TEXT_LENGTH];
        snprintf(
            msg.as_mut_ptr() as *mut c_char,
            JSON_ERROR_TEXT_LENGTH,
            b"unable to open %s: %s\0".as_ptr() as *const c_char,
            path,
            strerror(get_errno()),
        );
        let n = cstr_len(&msg);
        error_set(
            error,
            core::ptr::null(),
            JSON_ERROR_CANNOT_OPEN_FILE,
            &msg[..n],
        );
        return core::ptr::null_mut();
    }

    result = json_loadf(fp, flags, error);

    fclose(fp);
    result
}

const MAX_BUF_LEN: usize = 1024;

#[repr(C)]
struct callback_data_t {
    data: [c_char; MAX_BUF_LEN],
    len: usize,
    pos: usize,
    callback: json_load_callback_t,
    arg: *mut c_void,
}

unsafe extern "C" fn callback_get(data: *mut c_void) -> c_int {
    let c: c_char;
    let stream = data as *mut callback_data_t;

    if (*stream).pos >= (*stream).len {
        (*stream).pos = 0;
        (*stream).len = ((*stream).callback.unwrap())(
            (*stream).data.as_mut_ptr() as *mut c_void,
            MAX_BUF_LEN,
            (*stream).arg,
        );
        if (*stream).len == 0 || (*stream).len == usize::MAX {
            return EOF;
        }
    }

    c = (*stream).data[(*stream).pos];
    (*stream).pos += 1;
    c as u8 as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_load_callback(
    callback: json_load_callback_t,
    arg: *mut c_void,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    let mut lex = lex_t::zeroed();
    let result: *mut json_t;

    let mut stream_data = core::mem::MaybeUninit::<callback_data_t>::zeroed();
    let stream_data = stream_data.as_mut_ptr();
    (*stream_data).callback = callback;
    (*stream_data).arg = arg;

    jsonp_error_init(error, b"<callback>\0".as_ptr() as *const c_char);

    if callback.is_none() {
        error_set(
            error,
            core::ptr::null(),
            JSON_ERROR_INVALID_ARGUMENT,
            b"wrong arguments",
        );
        return core::ptr::null_mut();
    }

    if lex_init(
        &mut lex,
        Some(callback_get),
        flags,
        stream_data as *mut c_void,
    ) != 0
    {
        return core::ptr::null_mut();
    }

    result = parse_json(&mut lex, flags, error);

    lex_close(&mut lex);
    result
}
