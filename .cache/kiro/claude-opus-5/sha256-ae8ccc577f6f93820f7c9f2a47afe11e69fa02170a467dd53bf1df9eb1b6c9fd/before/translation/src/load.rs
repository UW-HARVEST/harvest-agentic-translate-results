//! Translation of `src/load.c`.

use crate::cffi;
use crate::jtypes::*;
use crate::memory::{jsonp_free, jsonp_malloc};
use crate::strbuffer::{
    strbuffer_append_byte, strbuffer_clear, strbuffer_close, strbuffer_init, strbuffer_pop,
    strbuffer_t, strbuffer_value,
};
use crate::strconv::jsonp_strtod;
use crate::utf::{utf8_check_first, utf8_check_full, utf8_encode};
use crate::value::*;
use core::ffi::{c_char, c_int, c_void};
use core::ptr::{null_mut, addr_of_mut};

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
    l_isdigit(c)
        || ('A' as c_int <= c && c <= 'F' as c_int)
        || ('a' as c_int <= c && c <= 'f' as c_int)
}

type get_func = unsafe extern "C" fn(data: *mut c_void) -> c_int;

#[repr(C)]
struct stream_t {
    get: Option<get_func>,
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
struct lex_t {
    stream: stream_t,
    saved_text: strbuffer_t,
    flags: usize,
    depth: usize,
    token: c_int,
    /* union { struct { char *val; size_t len; } string; json_int_t integer;
       double real; } value; -- expanded, each member is only ever read back
       for the token type that wrote it. */
    value_string_val: *mut c_char,
    value_string_len: usize,
    value_integer: json_int_t,
    value_real: f64,
}

impl lex_t {
    const fn new() -> Self {
        lex_t {
            stream: stream_t {
                get: None,
                data: null_mut(),
                buffer: [0; 5],
                buffer_pos: 0,
                state: 0,
                line: 0,
                column: 0,
                last_column: 0,
                position: 0,
            },
            saved_text: strbuffer_t::new(),
            flags: 0,
            depth: 0,
            token: 0,
            value_string_val: null_mut(),
            value_string_len: 0,
            value_integer: 0,
            value_real: 0.0,
        }
    }
}

#[inline]
unsafe fn stream_to_lex(stream: *mut stream_t) -> *mut lex_t {
    stream as *mut lex_t
}

/* ------------------------------------------------------------------ */
/* error reporting                                                    */
/* ------------------------------------------------------------------ */

/// `error_set()` with the message text already rendered by the caller
/// (the C function is variadic and renders it with vsnprintf into a
/// JSON_ERROR_TEXT_LENGTH sized buffer).
unsafe fn error_set_rendered(
    error: *mut json_error_t,
    lex: *const lex_t,
    code_in: c_int,
    msg_text: *const c_char,
) {
    unsafe {
        let mut msg_with_context = [0 as c_char; JSON_ERROR_TEXT_LENGTH];

        let mut line: c_int = -1;
        let mut col: c_int = -1;
        let mut pos: usize = 0;
        let mut code = code_in;
        let mut result: *const c_char = msg_text;

        if error.is_null() {
            return;
        }

        if !lex.is_null() {
            let saved_text = strbuffer_value(addr_of_mut!((*(lex as *mut lex_t)).saved_text));

            line = (*lex).stream.line;
            col = (*lex).stream.column;
            pos = (*lex).stream.position;

            if !saved_text.is_null() && *saved_text != 0 {
                if (*lex).saved_text.length <= 20 {
                    cffi::snprintf(
                        msg_with_context.as_mut_ptr(),
                        JSON_ERROR_TEXT_LENGTH,
                        c"%s near '%s'".as_ptr(),
                        msg_text,
                        saved_text,
                    );
                    msg_with_context[JSON_ERROR_TEXT_LENGTH - 1] = 0;
                    result = msg_with_context.as_ptr();
                }
            } else {
                if code == json_error_invalid_syntax {
                    /* More specific error code for premature end of file. */
                    code = json_error_premature_end_of_input;
                }
                if (*lex).stream.state == STREAM_STATE_ERROR {
                    /* No context for UTF-8 decoding errors */
                    result = msg_text;
                } else {
                    cffi::snprintf(
                        msg_with_context.as_mut_ptr(),
                        JSON_ERROR_TEXT_LENGTH,
                        c"%s near end of file".as_ptr(),
                        msg_text,
                    );
                    msg_with_context[JSON_ERROR_TEXT_LENGTH - 1] = 0;
                    result = msg_with_context.as_ptr();
                }
            }
        }

        crate::error::jsonp_error_set_str(error, line, col, pos, code, result);
    }
}

macro_rules! error_set {
    ($error:expr, $lex:expr, $code:expr, $fmt:expr $(, $arg:expr)* $(,)?) => {{
        let mut msg_text = [0 as c_char; JSON_ERROR_TEXT_LENGTH];
        cffi::snprintf(
            msg_text.as_mut_ptr(),
            JSON_ERROR_TEXT_LENGTH,
            $fmt $(, $arg)*
        );
        msg_text[JSON_ERROR_TEXT_LENGTH - 1] = 0;
        error_set_rendered($error, $lex, $code, msg_text.as_ptr());
    }};
}

/* ------------------------------------------------------------------ */
/* lexical analyzer                                                   */
/* ------------------------------------------------------------------ */

unsafe fn stream_init(stream: *mut stream_t, get: get_func, data: *mut c_void) {
    unsafe {
        (*stream).get = Some(get);
        (*stream).data = data;
        (*stream).buffer[0] = 0;
        (*stream).buffer_pos = 0;

        (*stream).state = STREAM_STATE_OK;
        (*stream).line = 1;
        (*stream).column = 0;
        (*stream).position = 0;
    }
}

unsafe fn stream_get(stream: *mut stream_t, error: *mut json_error_t) -> c_int {
    unsafe {
        let mut c: c_int;

        if (*stream).state != STREAM_STATE_OK {
            return (*stream).state;
        }

        if (*stream).buffer[(*stream).buffer_pos] == 0 {
            let getf = (*stream).get.unwrap();
            c = getf((*stream).data);
            if c == cffi::EOF {
                (*stream).state = STREAM_STATE_EOF;
                return STREAM_STATE_EOF;
            }

            (*stream).buffer[0] = c as c_char;
            (*stream).buffer_pos = 0;

            if 0x80 <= c && c <= 0xFF {
                /* multi-byte UTF-8 sequence */
                let count = utf8_check_first(c as c_char);
                if count == 0 {
                    /* goto out */
                    (*stream).state = STREAM_STATE_ERROR;
                    error_set!(
                        error,
                        stream_to_lex(stream),
                        json_error_invalid_utf8,
                        c"unable to decode byte 0x%x".as_ptr(),
                        c
                    );
                    return STREAM_STATE_ERROR;
                }

                let mut i = 1usize;
                while i < count {
                    (*stream).buffer[i] = getf((*stream).data) as c_char;
                    i += 1;
                }

                if utf8_check_full((*stream).buffer.as_ptr(), count, null_mut()) == 0 {
                    /* goto out */
                    (*stream).state = STREAM_STATE_ERROR;
                    error_set!(
                        error,
                        stream_to_lex(stream),
                        json_error_invalid_utf8,
                        c"unable to decode byte 0x%x".as_ptr(),
                        c
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
}

unsafe fn stream_unget(stream: *mut stream_t, c: c_int) {
    unsafe {
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

        (*stream).buffer_pos -= 1;
    }
}

#[inline]
unsafe fn lex_get(lex: *mut lex_t, error: *mut json_error_t) -> c_int {
    unsafe { stream_get(addr_of_mut!((*lex).stream), error) }
}

#[inline]
unsafe fn lex_save(lex: *mut lex_t, c: c_int) {
    unsafe {
        strbuffer_append_byte(addr_of_mut!((*lex).saved_text), c as c_char);
    }
}

unsafe fn lex_get_save(lex: *mut lex_t, error: *mut json_error_t) -> c_int {
    unsafe {
        let c = stream_get(addr_of_mut!((*lex).stream), error);
        if c != STREAM_STATE_EOF && c != STREAM_STATE_ERROR {
            lex_save(lex, c);
        }
        c
    }
}

#[inline]
unsafe fn lex_unget(lex: *mut lex_t, c: c_int) {
    unsafe { stream_unget(addr_of_mut!((*lex).stream), c) }
}

unsafe fn lex_unget_unsave(lex: *mut lex_t, c: c_int) {
    unsafe {
        if c != STREAM_STATE_EOF && c != STREAM_STATE_ERROR {
            stream_unget(addr_of_mut!((*lex).stream), c);
            strbuffer_pop(addr_of_mut!((*lex).saved_text));
        }
    }
}

unsafe fn lex_save_cached(lex: *mut lex_t) {
    unsafe {
        while (*lex).stream.buffer[(*lex).stream.buffer_pos] != 0 {
            lex_save(lex, (*lex).stream.buffer[(*lex).stream.buffer_pos] as c_int);
            (*lex).stream.buffer_pos += 1;
            (*lex).stream.position += 1;
        }
    }
}

unsafe fn lex_free_string(lex: *mut lex_t) {
    unsafe {
        jsonp_free((*lex).value_string_val as *mut c_void);
        (*lex).value_string_val = null_mut();
        (*lex).value_string_len = 0;
    }
}

/* assumes that str points to 'u' plus at least 4 valid hex digits */
unsafe fn decode_unicode_escape(str_: *const c_char) -> i32 {
    unsafe {
        let mut value: i32 = 0;

        for i in 1..=4 {
            let c = *str_.add(i) as c_int;
            value <<= 4;
            if l_isdigit(c) {
                value += c - '0' as c_int;
            } else if l_islower(c) {
                value += c - 'a' as c_int + 10;
            } else if l_isupper(c) {
                value += c - 'A' as c_int + 10;
            } else {
                return -1;
            }
        }

        value
    }
}

unsafe fn lex_scan_string(lex: *mut lex_t, error: *mut json_error_t) {
    unsafe {
        (*lex).value_string_val = null_mut();
        (*lex).token = TOKEN_INVALID;

        let mut c = lex_get_save(lex, error);

        while c != '"' as c_int {
            if c == STREAM_STATE_ERROR {
                lex_free_string(lex);
                return;
            } else if c == STREAM_STATE_EOF {
                error_set!(
                    error,
                    lex,
                    json_error_premature_end_of_input,
                    c"premature end of input".as_ptr()
                );
                lex_free_string(lex);
                return;
            } else if 0 <= c && c <= 0x1F {
                /* control character */
                lex_unget_unsave(lex, c);
                if c == '\n' as c_int {
                    error_set!(
                        error,
                        lex,
                        json_error_invalid_syntax,
                        c"unexpected newline".as_ptr()
                    );
                } else {
                    error_set!(
                        error,
                        lex,
                        json_error_invalid_syntax,
                        c"control character 0x%x".as_ptr(),
                        c
                    );
                }
                lex_free_string(lex);
                return;
            } else if c == '\\' as c_int {
                c = lex_get_save(lex, error);
                if c == 'u' as c_int {
                    c = lex_get_save(lex, error);
                    for _ in 0..4 {
                        if !l_isxdigit(c) {
                            error_set!(
                                error,
                                lex,
                                json_error_invalid_syntax,
                                c"invalid escape".as_ptr()
                            );
                            lex_free_string(lex);
                            return;
                        }
                        c = lex_get_save(lex, error);
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
                    error_set!(
                        error,
                        lex,
                        json_error_invalid_syntax,
                        c"invalid escape".as_ptr()
                    );
                    lex_free_string(lex);
                    return;
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
        let mut t = jsonp_malloc((*lex).saved_text.length + 1) as *mut c_char;
        if t.is_null() {
            /* this is not very nice, since TOKEN_INVALID is returned */
            lex_free_string(lex);
            return;
        }
        (*lex).value_string_val = t;

        /* + 1 to skip the " */
        let mut p = strbuffer_value(addr_of_mut!((*lex).saved_text)).add(1);

        while *p != '"' as c_char {
            if *p == '\\' as c_char {
                p = p.add(1);
                if *p == 'u' as c_char {
                    let mut length: usize = 0;

                    let mut value = decode_unicode_escape(p);
                    if value < 0 {
                        error_set!(
                            error,
                            lex,
                            json_error_invalid_syntax,
                            c"invalid Unicode escape '%.6s'".as_ptr(),
                            p.sub(1)
                        );
                        lex_free_string(lex);
                        return;
                    }
                    p = p.add(5);

                    if 0xD800 <= value && value <= 0xDBFF {
                        /* surrogate pair */
                        if *p == '\\' as c_char && *p.add(1) == 'u' as c_char {
                            p = p.add(1);
                            let value2 = decode_unicode_escape(p);
                            if value2 < 0 {
                                error_set!(
                                    error,
                                    lex,
                                    json_error_invalid_syntax,
                                    c"invalid Unicode escape '%.6s'".as_ptr(),
                                    p.sub(1)
                                );
                                lex_free_string(lex);
                                return;
                            }
                            p = p.add(5);

                            if 0xDC00 <= value2 && value2 <= 0xDFFF {
                                /* valid second surrogate */
                                value = ((value - 0xD800) << 10) + (value2 - 0xDC00) + 0x10000;
                            } else {
                                /* invalid second surrogate */
                                error_set!(
                                    error,
                                    lex,
                                    json_error_invalid_syntax,
                                    c"invalid Unicode '\\u%04X\\u%04X'".as_ptr(),
                                    value,
                                    value2
                                );
                                lex_free_string(lex);
                                return;
                            }
                        } else {
                            /* no second surrogate */
                            error_set!(
                                error,
                                lex,
                                json_error_invalid_syntax,
                                c"invalid Unicode '\\u%04X'".as_ptr(),
                                value
                            );
                            lex_free_string(lex);
                            return;
                        }
                    } else if 0xDC00 <= value && value <= 0xDFFF {
                        error_set!(
                            error,
                            lex,
                            json_error_invalid_syntax,
                            c"invalid Unicode '\\u%04X'".as_ptr(),
                            value
                        );
                        lex_free_string(lex);
                        return;
                    }

                    utf8_encode(value, t, &mut length);
                    t = t.add(length);
                } else {
                    match *p as u8 {
                        b'"' | b'\\' | b'/' => *t = *p,
                        b'b' => *t = 0x08,
                        b'f' => *t = 0x0C,
                        b'n' => *t = 0x0A,
                        b'r' => *t = 0x0D,
                        b't' => *t = 0x09,
                        _ => {}
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
        (*lex).value_string_len = t.offset_from((*lex).value_string_val) as usize;
        (*lex).token = TOKEN_STRING;
    }
}

unsafe fn lex_scan_number(lex: *mut lex_t, c_in: c_int, error: *mut json_error_t) -> c_int {
    unsafe {
        let mut c = c_in;
        let mut doubleval: f64 = 0.0;

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
            lex_unget_unsave(lex, c);

            let saved_text = strbuffer_value(addr_of_mut!((*lex).saved_text));

            cffi::set_errno(0);
            let mut end: *mut c_char = null_mut();
            let intval = cffi::strtoll(saved_text, &mut end, 10);
            if cffi::errno() == cffi::ERANGE {
                if intval < 0 {
                    error_set!(
                        error,
                        lex,
                        json_error_numeric_overflow,
                        c"too big negative integer".as_ptr()
                    );
                } else {
                    error_set!(
                        error,
                        lex,
                        json_error_numeric_overflow,
                        c"too big integer".as_ptr()
                    );
                }
                return -1;
            }

            (*lex).token = TOKEN_INTEGER;
            (*lex).value_integer = intval;
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

        if jsonp_strtod(addr_of_mut!((*lex).saved_text), &mut doubleval) != 0 {
            error_set!(
                error,
                lex,
                json_error_numeric_overflow,
                c"real number overflow".as_ptr()
            );
            return -1;
        }

        (*lex).token = TOKEN_REAL;
        (*lex).value_real = doubleval;
        0
    }
}

unsafe fn lex_scan(lex: *mut lex_t, error: *mut json_error_t) -> c_int {
    unsafe {
        let mut c: c_int;

        strbuffer_clear(addr_of_mut!((*lex).saved_text));

        if (*lex).token == TOKEN_STRING {
            lex_free_string(lex);
        }

        loop {
            c = lex_get(lex, error);
            if !(c == ' ' as c_int || c == '\t' as c_int || c == '\n' as c_int || c == '\r' as c_int)
            {
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
            loop {
                c = lex_get_save(lex, error);
                if !l_isalpha(c) {
                    break;
                }
            }
            lex_unget_unsave(lex, c);

            let saved_text = strbuffer_value(addr_of_mut!((*lex).saved_text));

            if cffi::c_streq(saved_text, b"true\0") {
                (*lex).token = TOKEN_TRUE;
            } else if cffi::c_streq(saved_text, b"false\0") {
                (*lex).token = TOKEN_FALSE;
            } else if cffi::c_streq(saved_text, b"null\0") {
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
}

unsafe fn lex_steal_string(lex: *mut lex_t, out_len: *mut usize) -> *mut c_char {
    unsafe {
        let mut result: *mut c_char = null_mut();
        if (*lex).token == TOKEN_STRING {
            result = (*lex).value_string_val;
            *out_len = (*lex).value_string_len;
            (*lex).value_string_val = null_mut();
            (*lex).value_string_len = 0;
        }
        result
    }
}

unsafe fn lex_init(
    lex: *mut lex_t,
    get: get_func,
    flags: usize,
    data: *mut c_void,
) -> c_int {
    unsafe {
        stream_init(addr_of_mut!((*lex).stream), get, data);
        if strbuffer_init(addr_of_mut!((*lex).saved_text)) != 0 {
            return -1;
        }

        (*lex).flags = flags;
        (*lex).token = TOKEN_INVALID;
        0
    }
}

unsafe fn lex_close(lex: *mut lex_t) {
    unsafe {
        if (*lex).token == TOKEN_STRING {
            lex_free_string(lex);
        }
        strbuffer_close(addr_of_mut!((*lex).saved_text));
    }
}

/* ------------------------------------------------------------------ */
/* parser                                                             */
/* ------------------------------------------------------------------ */

unsafe fn parse_object(lex: *mut lex_t, flags: usize, error: *mut json_error_t) -> *mut json_t {
    unsafe {
        let object = json_object();
        if object.is_null() {
            return null_mut();
        }

        lex_scan(lex, error);
        if (*lex).token == '}' as c_int {
            return object;
        }

        loop {
            let mut len: usize = 0;

            if (*lex).token != TOKEN_STRING {
                error_set!(
                    error,
                    lex,
                    json_error_invalid_syntax,
                    c"string or '}' expected".as_ptr()
                );
                json_decref(object);
                return null_mut();
            }

            let key = lex_steal_string(lex, &mut len);
            if key.is_null() {
                return null_mut();
            }
            if !cffi::c_memchr(key, 0, len).is_null() {
                jsonp_free(key as *mut c_void);
                error_set!(
                    error,
                    lex,
                    json_error_null_byte_in_key,
                    c"NUL byte in object key not supported".as_ptr()
                );
                json_decref(object);
                return null_mut();
            }

            if (flags & JSON_REJECT_DUPLICATES) != 0 {
                if !json_object_getn(object, key, len).is_null() {
                    jsonp_free(key as *mut c_void);
                    error_set!(
                        error,
                        lex,
                        json_error_duplicate_key,
                        c"duplicate object key".as_ptr()
                    );
                    json_decref(object);
                    return null_mut();
                }
            }

            lex_scan(lex, error);
            if (*lex).token != ':' as c_int {
                jsonp_free(key as *mut c_void);
                error_set!(
                    error,
                    lex,
                    json_error_invalid_syntax,
                    c"':' expected".as_ptr()
                );
                json_decref(object);
                return null_mut();
            }

            lex_scan(lex, error);
            let value = parse_value(lex, flags, error);
            if value.is_null() {
                jsonp_free(key as *mut c_void);
                json_decref(object);
                return null_mut();
            }

            if json_object_setn_new_nocheck(object, key, len, value) != 0 {
                jsonp_free(key as *mut c_void);
                json_decref(object);
                return null_mut();
            }

            jsonp_free(key as *mut c_void);

            lex_scan(lex, error);
            if (*lex).token != ',' as c_int {
                break;
            }

            lex_scan(lex, error);
        }

        if (*lex).token != '}' as c_int {
            error_set!(
                error,
                lex,
                json_error_invalid_syntax,
                c"'}' expected".as_ptr()
            );
            json_decref(object);
            return null_mut();
        }

        object
    }
}

unsafe fn parse_array(lex: *mut lex_t, flags: usize, error: *mut json_error_t) -> *mut json_t {
    unsafe {
        let array = json_array();
        if array.is_null() {
            return null_mut();
        }

        lex_scan(lex, error);
        if (*lex).token == ']' as c_int {
            return array;
        }

        while (*lex).token != 0 {
            let elem = parse_value(lex, flags, error);
            if elem.is_null() {
                json_decref(array);
                return null_mut();
            }

            if json_array_append_new(array, elem) != 0 {
                json_decref(array);
                return null_mut();
            }

            lex_scan(lex, error);
            if (*lex).token != ',' as c_int {
                break;
            }

            lex_scan(lex, error);
        }

        if (*lex).token != ']' as c_int {
            error_set!(
                error,
                lex,
                json_error_invalid_syntax,
                c"']' expected".as_ptr()
            );
            json_decref(array);
            return null_mut();
        }

        array
    }
}

unsafe fn parse_value(lex: *mut lex_t, flags: usize, error: *mut json_error_t) -> *mut json_t {
    unsafe {
        let json: *mut json_t;

        (*lex).depth += 1;
        if (*lex).depth > JSON_PARSER_MAX_DEPTH {
            error_set!(
                error,
                lex,
                json_error_stack_overflow,
                c"maximum parsing depth reached".as_ptr()
            );
            return null_mut();
        }

        match (*lex).token {
            TOKEN_STRING => {
                let value = (*lex).value_string_val;
                let len = (*lex).value_string_len;

                if (flags & JSON_ALLOW_NUL) == 0 {
                    if !cffi::c_memchr(value, 0, len).is_null() {
                        error_set!(
                            error,
                            lex,
                            json_error_null_character,
                            c"\\u0000 is not allowed without JSON_ALLOW_NUL".as_ptr()
                        );
                        return null_mut();
                    }
                }

                json = jsonp_stringn_nocheck_own(value, len);
                (*lex).value_string_val = null_mut();
                (*lex).value_string_len = 0;
            }

            TOKEN_INTEGER => {
                json = json_integer((*lex).value_integer);
            }

            TOKEN_REAL => {
                json = json_real((*lex).value_real);
            }

            TOKEN_TRUE => json = json_true(),

            TOKEN_FALSE => json = json_false(),

            TOKEN_NULL => json = json_null(),

            123 /* '{' */ => json = parse_object(lex, flags, error),

            91 /* '[' */ => json = parse_array(lex, flags, error),

            TOKEN_INVALID => {
                error_set!(
                    error,
                    lex,
                    json_error_invalid_syntax,
                    c"invalid token".as_ptr()
                );
                return null_mut();
            }

            _ => {
                error_set!(
                    error,
                    lex,
                    json_error_invalid_syntax,
                    c"unexpected token".as_ptr()
                );
                return null_mut();
            }
        }

        if json.is_null() {
            return null_mut();
        }

        (*lex).depth -= 1;
        json
    }
}

unsafe fn parse_json(lex: *mut lex_t, flags: usize, error: *mut json_error_t) -> *mut json_t {
    unsafe {
        (*lex).depth = 0;

        lex_scan(lex, error);
        if (flags & JSON_DECODE_ANY) == 0 {
            if (*lex).token != '[' as c_int && (*lex).token != '{' as c_int {
                error_set!(
                    error,
                    lex,
                    json_error_invalid_syntax,
                    c"'[' or '{' expected".as_ptr()
                );
                return null_mut();
            }
        }

        let result = parse_value(lex, flags, error);
        if result.is_null() {
            return null_mut();
        }

        if (flags & JSON_DISABLE_EOF_CHECK) == 0 {
            lex_scan(lex, error);
            if (*lex).token != TOKEN_EOF {
                error_set!(
                    error,
                    lex,
                    json_error_end_of_input_expected,
                    c"end of file expected".as_ptr()
                );
                json_decref(result);
                return null_mut();
            }
        }

        if !error.is_null() {
            /* Save the position even though there was no error */
            (*error).position = (*lex).stream.position as c_int;
        }

        result
    }
}

#[repr(C)]
struct string_data_t {
    data: *const c_char,
    pos: usize,
}

unsafe extern "C" fn string_get(data: *mut c_void) -> c_int {
    unsafe {
        let stream = data as *mut string_data_t;
        let c = *(*stream).data.add((*stream).pos);
        if c == 0 {
            cffi::EOF
        } else {
            (*stream).pos += 1;
            c as u8 as c_int
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loads(
    string: *const c_char,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    unsafe {
        let mut lex = lex_t::new();

        crate::error::jsonp_error_init(error, c"<string>".as_ptr());

        if string.is_null() {
            error_set!(
                error,
                null_mut::<lex_t>(),
                json_error_invalid_argument,
                c"wrong arguments".as_ptr()
            );
            return null_mut();
        }

        let mut stream_data = string_data_t {
            data: string,
            pos: 0,
        };

        if lex_init(
            addr_of_mut!(lex),
            string_get,
            flags,
            addr_of_mut!(stream_data) as *mut c_void,
        ) != 0
        {
            return null_mut();
        }

        let result = parse_json(addr_of_mut!(lex), flags, error);

        lex_close(addr_of_mut!(lex));
        result
    }
}

#[repr(C)]
struct buffer_data_t {
    data: *const c_char,
    len: usize,
    pos: usize,
}

unsafe extern "C" fn buffer_get(data: *mut c_void) -> c_int {
    unsafe {
        let stream = data as *mut buffer_data_t;
        if (*stream).pos >= (*stream).len {
            return cffi::EOF;
        }

        let c = *(*stream).data.add((*stream).pos);
        (*stream).pos += 1;
        c as u8 as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadb(
    buffer: *const c_char,
    buflen: usize,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    unsafe {
        let mut lex = lex_t::new();

        crate::error::jsonp_error_init(error, c"<buffer>".as_ptr());

        if buffer.is_null() {
            error_set!(
                error,
                null_mut::<lex_t>(),
                json_error_invalid_argument,
                c"wrong arguments".as_ptr()
            );
            return null_mut();
        }

        let mut stream_data = buffer_data_t {
            data: buffer,
            len: buflen,
            pos: 0,
        };

        if lex_init(
            addr_of_mut!(lex),
            buffer_get,
            flags,
            addr_of_mut!(stream_data) as *mut c_void,
        ) != 0
        {
            return null_mut();
        }

        let result = parse_json(addr_of_mut!(lex), flags, error);

        lex_close(addr_of_mut!(lex));
        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadf(
    input: *mut cffi::FILE,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    unsafe {
        let mut lex = lex_t::new();

        let source = if input == cffi::stdin {
            c"<stdin>".as_ptr()
        } else {
            c"<stream>".as_ptr()
        };

        crate::error::jsonp_error_init(error, source);

        if input.is_null() {
            error_set!(
                error,
                null_mut::<lex_t>(),
                json_error_invalid_argument,
                c"wrong arguments".as_ptr()
            );
            return null_mut();
        }

        let getf: get_func = core::mem::transmute(
            cffi::fgetc as unsafe extern "C" fn(*mut cffi::FILE) -> c_int,
        );

        if lex_init(addr_of_mut!(lex), getf, flags, input as *mut c_void) != 0 {
            return null_mut();
        }

        let result = parse_json(addr_of_mut!(lex), flags, error);

        lex_close(addr_of_mut!(lex));
        result
    }
}

unsafe extern "C" fn fd_get_func(fd: *mut c_int) -> c_int {
    unsafe {
        let mut c: u8 = 0;
        if cffi::read(*fd, &mut c as *mut u8 as *mut c_void, 1) == 1 {
            return c as c_int;
        }
        cffi::EOF
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadfd(
    input: c_int,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    unsafe {
        let mut lex = lex_t::new();

        let source = if input == cffi::STDIN_FILENO {
            c"<stdin>".as_ptr()
        } else {
            c"<stream>".as_ptr()
        };

        crate::error::jsonp_error_init(error, source);

        if input < 0 {
            error_set!(
                error,
                null_mut::<lex_t>(),
                json_error_invalid_argument,
                c"wrong arguments".as_ptr()
            );
            return null_mut();
        }

        let mut fd = input;
        let getf: get_func =
            core::mem::transmute(fd_get_func as unsafe extern "C" fn(*mut c_int) -> c_int);

        if lex_init(addr_of_mut!(lex), getf, flags, addr_of_mut!(fd) as *mut c_void) != 0 {
            return null_mut();
        }

        let result = parse_json(addr_of_mut!(lex), flags, error);

        lex_close(addr_of_mut!(lex));
        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_load_file(
    path: *const c_char,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    unsafe {
        crate::error::jsonp_error_init(error, path);

        if path.is_null() {
            error_set!(
                error,
                null_mut::<lex_t>(),
                json_error_invalid_argument,
                c"wrong arguments".as_ptr()
            );
            return null_mut();
        }

        let fp = cffi::fopen(path, c"rb".as_ptr());
        if fp.is_null() {
            error_set!(
                error,
                null_mut::<lex_t>(),
                json_error_cannot_open_file,
                c"unable to open %s: %s".as_ptr(),
                path,
                cffi::strerror(cffi::errno())
            );
            return null_mut();
        }

        let result = json_loadf(fp, flags, error);

        cffi::fclose(fp);
        result
    }
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
    unsafe {
        let stream = data as *mut callback_data_t;

        if (*stream).pos >= (*stream).len {
            (*stream).pos = 0;
            (*stream).len = ((*stream).callback.unwrap())(
                (*stream).data.as_mut_ptr() as *mut c_void,
                MAX_BUF_LEN,
                (*stream).arg,
            );
            if (*stream).len == 0 || (*stream).len == usize::MAX {
                return cffi::EOF;
            }
        }

        let c = (*stream).data[(*stream).pos];
        (*stream).pos += 1;
        c as u8 as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_load_callback(
    callback: json_load_callback_t,
    arg: *mut c_void,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    unsafe {
        let mut lex = lex_t::new();

        let mut stream_data = callback_data_t {
            data: [0; MAX_BUF_LEN],
            len: 0,
            pos: 0,
            callback,
            arg,
        };

        crate::error::jsonp_error_init(error, c"<callback>".as_ptr());

        if callback.is_none() {
            error_set!(
                error,
                null_mut::<lex_t>(),
                json_error_invalid_argument,
                c"wrong arguments".as_ptr()
            );
            return null_mut();
        }

        if lex_init(
            addr_of_mut!(lex),
            callback_get,
            flags,
            addr_of_mut!(stream_data) as *mut c_void,
        ) != 0
        {
            return null_mut();
        }

        let result = parse_json(addr_of_mut!(lex), flags, error);

        lex_close(addr_of_mut!(lex));
        result
    }
}
