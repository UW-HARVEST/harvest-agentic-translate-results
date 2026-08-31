//! Translation of `src/load.c`.

use crate::cfmt::CBuf;
use crate::error::*;
use crate::memory::*;
use crate::strbuffer::*;
use crate::strconv::jsonp_strtod;
use crate::types::*;
use crate::utf::*;
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
    (b'A' as c_int) <= c && c <= (b'Z' as c_int)
}
#[inline]
fn l_islower(c: c_int) -> bool {
    (b'a' as c_int) <= c && c <= (b'z' as c_int)
}
#[inline]
fn l_isalpha(c: c_int) -> bool {
    l_isupper(c) || l_islower(c)
}
#[inline]
fn l_isdigit(c: c_int) -> bool {
    (b'0' as c_int) <= c && c <= (b'9' as c_int)
}
#[inline]
fn l_isxdigit(c: c_int) -> bool {
    l_isdigit(c)
        || ((b'A' as c_int) <= c && c <= (b'F' as c_int))
        || ((b'a' as c_int) <= c && c <= (b'f' as c_int))
}

pub type GetFunc = unsafe extern "C" fn(data: *mut c_void) -> c_int;
pub type JsonLoadCallbackT =
    unsafe extern "C" fn(buffer: *mut c_void, buflen: usize, data: *mut c_void) -> usize;

struct StreamT {
    get: GetFunc,
    data: *mut c_void,
    buffer: [c_char; 5],
    buffer_pos: usize,
    state: c_int,
    line: c_int,
    column: c_int,
    last_column: c_int,
    position: usize,
}

struct LexT {
    stream: StreamT,
    saved_text: StrbufferT,
    flags: usize,
    depth: usize,
    token: c_int,
    /* union { struct { char *val; size_t len; } string; json_int_t integer; double real; } */
    string_val: *mut c_char,
    string_len: usize,
    integer: JsonIntT,
    real: f64,
}

/*** error reporting ***/

unsafe fn error_set(error: *mut JsonErrorT, lex: *const LexT, code_in: c_int, msg_text: &[u8]) {
    let mut msg_with_context: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();

    let mut line: c_int = -1;
    let mut col: c_int = -1;
    let mut pos: usize = 0;
    let mut code = code_in;
    let mut use_context = false;

    if error.is_null() {
        return;
    }

    /* msg_text has already been formatted with vsnprintf() semantics */

    if !lex.is_null() {
        let saved_text = strbuffer_value(&(*lex).saved_text);

        line = (*lex).stream.line;
        col = (*lex).stream.column;
        pos = (*lex).stream.position;

        if !saved_text.is_null() && *saved_text != 0 {
            if (*lex).saved_text.length <= 20 {
                /* snprintf(msg_with_context, ..., "%s near '%s'", msg_text, saved_text) */
                msg_with_context.push(cstr_prefix(msg_text));
                msg_with_context.push(b" near '");
                msg_with_context.push_cstr(saved_text);
                msg_with_context.push(b"'");
                use_context = true;
            }
        } else {
            if code == JSON_ERROR_INVALID_SYNTAX {
                /* More specific error code for premature end of file. */
                code = JSON_ERROR_PREMATURE_END_OF_INPUT;
            }
            if (*lex).stream.state == STREAM_STATE_ERROR {
                /* No context for UTF-8 decoding errors */
            } else {
                msg_with_context.push(cstr_prefix(msg_text));
                msg_with_context.push(b" near end of file");
                use_context = true;
            }
        }
    }

    let result: &[u8] = if use_context {
        msg_with_context.as_cstr_bytes()
    } else {
        cstr_prefix(msg_text)
    };

    /* jsonp_error_set(error, line, col, pos, code, "%s", result) */
    jsonp_error_set_bytes(error, line, col, pos, code, result);
}

/// The bytes of `s` up to the first NUL, i.e. what `%s` would print.
fn cstr_prefix(s: &[u8]) -> &[u8] {
    match s.iter().position(|&c| c == 0) {
        Some(i) => &s[..i],
        None => s,
    }
}

/*** lexical analyzer ***/

unsafe fn stream_init(stream: *mut StreamT, get: GetFunc, data: *mut c_void) {
    (*stream).get = get;
    (*stream).data = data;
    (*stream).buffer[0] = 0;
    (*stream).buffer_pos = 0;

    (*stream).state = STREAM_STATE_OK;
    (*stream).line = 1;
    (*stream).column = 0;
    (*stream).position = 0;
}

unsafe fn stream_get(lex: *mut LexT, error: *mut JsonErrorT) -> c_int {
    let stream: *mut StreamT = &mut (*lex).stream;
    let mut c: c_int;

    if (*stream).state != STREAM_STATE_OK {
        return (*stream).state;
    }

    if (*stream).buffer[(*stream).buffer_pos] == 0 {
        c = ((*stream).get)((*stream).data);
        if c == EOF {
            (*stream).state = STREAM_STATE_EOF;
            return STREAM_STATE_EOF;
        }

        (*stream).buffer[0] = c as u8 as c_char;
        (*stream).buffer_pos = 0;

        if (0x80..=0xFF).contains(&c) {
            /* multi-byte UTF-8 sequence */
            let count = utf8_check_first(c as u8 as c_char);
            if count == 0 {
                (*stream).state = STREAM_STATE_ERROR;
                let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
                b.push(b"unable to decode byte 0x");
                b.push_hex_lower(c as u32);
                error_set(error, lex, JSON_ERROR_INVALID_UTF8, b.as_bytes());
                return STREAM_STATE_ERROR;
            }

            for i in 1..count {
                (*stream).buffer[i] = ((*stream).get)((*stream).data) as u8 as c_char;
            }

            if utf8_check_full((*stream).buffer.as_ptr(), count, core::ptr::null_mut()) == 0 {
                (*stream).state = STREAM_STATE_ERROR;
                let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
                b.push(b"unable to decode byte 0x");
                b.push_hex_lower(c as u32);
                error_set(error, lex, JSON_ERROR_INVALID_UTF8, b.as_bytes());
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
    if c == b'\n' as c_int {
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

unsafe fn stream_unget(stream: *mut StreamT, c: c_int) {
    if c == STREAM_STATE_EOF || c == STREAM_STATE_ERROR {
        return;
    }

    (*stream).position -= 1;
    if c == b'\n' as c_int {
        (*stream).line -= 1;
        (*stream).column = (*stream).last_column;
    } else if utf8_check_first(c as c_char) != 0 {
        (*stream).column -= 1;
    }

    (*stream).buffer_pos -= 1;
}

unsafe fn lex_get(lex: *mut LexT, error: *mut JsonErrorT) -> c_int {
    stream_get(lex, error)
}

unsafe fn lex_save(lex: *mut LexT, c: c_int) {
    strbuffer_append_byte(&mut (*lex).saved_text, c as u8 as c_char);
}

unsafe fn lex_get_save(lex: *mut LexT, error: *mut JsonErrorT) -> c_int {
    let c = stream_get(lex, error);
    if c != STREAM_STATE_EOF && c != STREAM_STATE_ERROR {
        lex_save(lex, c);
    }
    c
}

unsafe fn lex_unget(lex: *mut LexT, c: c_int) {
    stream_unget(&mut (*lex).stream, c);
}

unsafe fn lex_unget_unsave(lex: *mut LexT, c: c_int) {
    if c != STREAM_STATE_EOF && c != STREAM_STATE_ERROR {
        stream_unget(&mut (*lex).stream, c);
        strbuffer_pop(&mut (*lex).saved_text);
    }
}

unsafe fn lex_save_cached(lex: *mut LexT) {
    while (*lex).stream.buffer[(*lex).stream.buffer_pos] != 0 {
        lex_save(lex, (*lex).stream.buffer[(*lex).stream.buffer_pos] as c_int);
        (*lex).stream.buffer_pos += 1;
        (*lex).stream.position += 1;
    }
}

unsafe fn lex_free_string(lex: *mut LexT) {
    jsonp_free((*lex).string_val as *mut c_void);
    (*lex).string_val = core::ptr::null_mut();
    (*lex).string_len = 0;
}

/* assumes that str points to 'u' plus at least 4 valid hex digits */
unsafe fn decode_unicode_escape(str_: *const c_char) -> i32 {
    let mut value: i32 = 0;

    for i in 1..=4 {
        let c = *str_.offset(i) as c_int;
        value <<= 4;
        if l_isdigit(c) {
            value += c - b'0' as c_int;
        } else if l_islower(c) {
            value += c - b'a' as c_int + 10;
        } else if l_isupper(c) {
            value += c - b'A' as c_int + 10;
        } else {
            return -1;
        }
    }

    value
}

unsafe fn lex_scan_string(lex: *mut LexT, error: *mut JsonErrorT) {
    let mut c: c_int;
    let mut p: *const c_char;
    let mut t: *mut c_char;

    (*lex).string_val = core::ptr::null_mut();
    (*lex).token = TOKEN_INVALID;

    c = lex_get_save(lex, error);

    while c != b'"' as c_int {
        if c == STREAM_STATE_ERROR {
            lex_free_string(lex);
            return;
        } else if c == STREAM_STATE_EOF {
            error_set(
                error,
                lex,
                JSON_ERROR_PREMATURE_END_OF_INPUT,
                b"premature end of input",
            );
            lex_free_string(lex);
            return;
        } else if (0..=0x1F).contains(&c) {
            /* control character */
            lex_unget_unsave(lex, c);
            if c == b'\n' as c_int {
                error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"unexpected newline");
            } else {
                let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
                b.push(b"control character 0x");
                b.push_hex_lower(c as u32);
                error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b.as_bytes());
            }
            lex_free_string(lex);
            return;
        } else if c == b'\\' as c_int {
            c = lex_get_save(lex, error);
            if c == b'u' as c_int {
                c = lex_get_save(lex, error);
                for _ in 0..4 {
                    if !l_isxdigit(c) {
                        error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"invalid escape");
                        lex_free_string(lex);
                        return;
                    }
                    c = lex_get_save(lex, error);
                }
            } else if c == b'"' as c_int
                || c == b'\\' as c_int
                || c == b'/' as c_int
                || c == b'b' as c_int
                || c == b'f' as c_int
                || c == b'n' as c_int
                || c == b'r' as c_int
                || c == b't' as c_int
            {
                c = lex_get_save(lex, error);
            } else {
                error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"invalid escape");
                lex_free_string(lex);
                return;
            }
        } else {
            c = lex_get_save(lex, error);
        }
    }

    /* the actual value is at most of the same length as the source string */
    t = jsonp_malloc((*lex).saved_text.length + 1) as *mut c_char;
    if t.is_null() {
        /* this is not very nice, since TOKEN_INVALID is returned */
        lex_free_string(lex);
        return;
    }
    (*lex).string_val = t;

    /* + 1 to skip the " */
    p = strbuffer_value(&(*lex).saved_text).add(1);

    while *p != b'"' as c_char {
        if *p == b'\\' as c_char {
            p = p.add(1);
            if *p == b'u' as c_char {
                let mut length: usize = 0;
                let mut value: i32;

                value = decode_unicode_escape(p);
                if value < 0 {
                    let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
                    b.push(b"invalid Unicode escape '");
                    b.push_cstr_prec(p.offset(-1), 6);
                    b.push(b"'");
                    error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b.as_bytes());
                    lex_free_string(lex);
                    return;
                }
                p = p.add(5);

                if (0xD800..=0xDBFF).contains(&value) {
                    /* surrogate pair */
                    if *p == b'\\' as c_char && *p.add(1) == b'u' as c_char {
                        p = p.add(1);
                        let value2 = decode_unicode_escape(p);
                        if value2 < 0 {
                            let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
                            b.push(b"invalid Unicode escape '");
                            b.push_cstr_prec(p.offset(-1), 6);
                            b.push(b"'");
                            error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b.as_bytes());
                            lex_free_string(lex);
                            return;
                        }
                        p = p.add(5);

                        if (0xDC00..=0xDFFF).contains(&value2) {
                            /* valid second surrogate */
                            value = ((value - 0xD800) << 10) + (value2 - 0xDC00) + 0x10000;
                        } else {
                            /* invalid second surrogate */
                            let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
                            b.push(b"invalid Unicode '\\u");
                            push_hex4(&mut b, value as u32);
                            b.push(b"\\u");
                            push_hex4(&mut b, value2 as u32);
                            b.push(b"'");
                            error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b.as_bytes());
                            lex_free_string(lex);
                            return;
                        }
                    } else {
                        /* no second surrogate */
                        let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
                        b.push(b"invalid Unicode '\\u");
                        push_hex4(&mut b, value as u32);
                        b.push(b"'");
                        error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b.as_bytes());
                        lex_free_string(lex);
                        return;
                    }
                } else if (0xDC00..=0xDFFF).contains(&value) {
                    let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
                    b.push(b"invalid Unicode '\\u");
                    push_hex4(&mut b, value as u32);
                    b.push(b"'");
                    error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b.as_bytes());
                    lex_free_string(lex);
                    return;
                }

                utf8_encode(value, t, &mut length);
                t = t.add(length);
            } else {
                match *p as u8 {
                    b'"' | b'\\' | b'/' => *t = *p,
                    b'b' => *t = 0x08,
                    b'f' => *t = 0x0c,
                    b'n' => *t = b'\n' as c_char,
                    b'r' => *t = b'\r' as c_char,
                    b't' => *t = b'\t' as c_char,
                    _ => (),
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
    (*lex).string_len = t.offset_from((*lex).string_val) as usize;
    (*lex).token = TOKEN_STRING;
}

fn push_hex4(b: &mut CBuf<{ JSON_ERROR_TEXT_LENGTH }>, v: u32) {
    let mut hx = [0u8; 8];
    let n = crate::cfmt::hex_upper_pad4(v, &mut hx);
    b.push(&hx[..n]);
}

unsafe fn lex_scan_number(lex: *mut LexT, c_in: c_int, error: *mut JsonErrorT) -> c_int {
    let saved_text: *const c_char;
    let mut end: *mut c_char = core::ptr::null_mut();
    let mut doubleval: f64 = 0.0;
    let mut c = c_in;

    (*lex).token = TOKEN_INVALID;

    if c == b'-' as c_int {
        c = lex_get_save(lex, error);
    }

    if c == b'0' as c_int {
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
        && c != b'.' as c_int
        && c != b'E' as c_int
        && c != b'e' as c_int
    {
        let intval: JsonIntT;

        lex_unget_unsave(lex, c);

        saved_text = strbuffer_value(&(*lex).saved_text);

        set_errno(0);
        intval = strtoll(saved_text, &mut end, 10);
        if errno() == ERANGE {
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

        (*lex).token = TOKEN_INTEGER;
        (*lex).integer = intval;
        return 0;
    }

    if c == b'.' as c_int {
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

    if c == b'E' as c_int || c == b'e' as c_int {
        c = lex_get_save(lex, error);
        if c == b'+' as c_int || c == b'-' as c_int {
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

    if jsonp_strtod(&mut (*lex).saved_text, &mut doubleval) != 0 {
        error_set(
            error,
            lex,
            JSON_ERROR_NUMERIC_OVERFLOW,
            b"real number overflow",
        );
        return -1;
    }

    (*lex).token = TOKEN_REAL;
    (*lex).real = doubleval;
    0
}

unsafe fn lex_scan(lex: *mut LexT, error: *mut JsonErrorT) -> c_int {
    let mut c: c_int;

    strbuffer_clear(&mut (*lex).saved_text);

    if (*lex).token == TOKEN_STRING {
        lex_free_string(lex);
    }

    loop {
        c = lex_get(lex, error);
        if !(c == b' ' as c_int || c == b'\t' as c_int || c == b'\n' as c_int || c == b'\r' as c_int)
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

    if c == b'{' as c_int
        || c == b'}' as c_int
        || c == b'[' as c_int
        || c == b']' as c_int
        || c == b':' as c_int
        || c == b',' as c_int
    {
        (*lex).token = c;
    } else if c == b'"' as c_int {
        lex_scan_string(lex, error);
    } else if l_isdigit(c) || c == b'-' as c_int {
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

        saved_text = strbuffer_value(&(*lex).saved_text);

        if cstr_eq(saved_text, b"true") {
            (*lex).token = TOKEN_TRUE;
        } else if cstr_eq(saved_text, b"false") {
            (*lex).token = TOKEN_FALSE;
        } else if cstr_eq(saved_text, b"null") {
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

unsafe fn cstr_eq(s: *const c_char, lit: &[u8]) -> bool {
    for (i, &b) in lit.iter().enumerate() {
        if *s.add(i) as u8 != b {
            return false;
        }
    }
    *s.add(lit.len()) == 0
}

unsafe fn lex_steal_string(lex: *mut LexT, out_len: *mut usize) -> *mut c_char {
    let mut result: *mut c_char = core::ptr::null_mut();
    if (*lex).token == TOKEN_STRING {
        result = (*lex).string_val;
        *out_len = (*lex).string_len;
        (*lex).string_val = core::ptr::null_mut();
        (*lex).string_len = 0;
    }
    result
}

unsafe fn lex_init(lex: *mut LexT, get: GetFunc, flags: usize, data: *mut c_void) -> c_int {
    stream_init(&mut (*lex).stream, get, data);
    if strbuffer_init(&mut (*lex).saved_text) != 0 {
        return -1;
    }

    (*lex).flags = flags;
    (*lex).token = TOKEN_INVALID;
    0
}

unsafe fn lex_close(lex: *mut LexT) {
    if (*lex).token == TOKEN_STRING {
        lex_free_string(lex);
    }
    strbuffer_close(&mut (*lex).saved_text);
}

unsafe fn new_lex(get: GetFunc, data: *mut c_void) -> LexT {
    LexT {
        stream: StreamT {
            get,
            data,
            buffer: [0; 5],
            buffer_pos: 0,
            state: 0,
            line: 0,
            column: 0,
            last_column: 0,
            position: 0,
        },
        saved_text: StrbufferT {
            value: core::ptr::null_mut(),
            length: 0,
            size: 0,
        },
        flags: 0,
        depth: 0,
        token: 0,
        string_val: core::ptr::null_mut(),
        string_len: 0,
        integer: 0,
        real: 0.0,
    }
}

/*** parser ***/

unsafe fn parse_object(lex: *mut LexT, flags: usize, error: *mut JsonErrorT) -> *mut JsonT {
    let object = json_object();
    if object.is_null() {
        return core::ptr::null_mut();
    }

    lex_scan(lex, error);
    if (*lex).token == b'}' as c_int {
        return object;
    }

    loop {
        let key: *mut c_char;
        let mut len: usize = 0;
        let value: *mut JsonT;

        if (*lex).token != TOKEN_STRING {
            error_set(
                error,
                lex,
                JSON_ERROR_INVALID_SYNTAX,
                b"string or '}' expected",
            );
            json_decref(object);
            return core::ptr::null_mut();
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
            json_decref(object);
            return core::ptr::null_mut();
        }

        if (flags & JSON_REJECT_DUPLICATES) != 0 && !json_object_getn(object, key, len).is_null() {
            jsonp_free(key as *mut c_void);
            error_set(
                error,
                lex,
                JSON_ERROR_DUPLICATE_KEY,
                b"duplicate object key",
            );
            json_decref(object);
            return core::ptr::null_mut();
        }

        lex_scan(lex, error);
        if (*lex).token != b':' as c_int {
            jsonp_free(key as *mut c_void);
            error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"':' expected");
            json_decref(object);
            return core::ptr::null_mut();
        }

        lex_scan(lex, error);
        value = parse_value(lex, flags, error);
        if value.is_null() {
            jsonp_free(key as *mut c_void);
            json_decref(object);
            return core::ptr::null_mut();
        }

        if json_object_setn_new_nocheck(object, key, len, value) != 0 {
            jsonp_free(key as *mut c_void);
            json_decref(object);
            return core::ptr::null_mut();
        }

        jsonp_free(key as *mut c_void);

        lex_scan(lex, error);
        if (*lex).token != b',' as c_int {
            break;
        }

        lex_scan(lex, error);
    }

    if (*lex).token != b'}' as c_int {
        error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"'}' expected");
        json_decref(object);
        return core::ptr::null_mut();
    }

    object
}

unsafe fn parse_array(lex: *mut LexT, flags: usize, error: *mut JsonErrorT) -> *mut JsonT {
    let array = json_array();
    if array.is_null() {
        return core::ptr::null_mut();
    }

    lex_scan(lex, error);
    if (*lex).token == b']' as c_int {
        return array;
    }

    while (*lex).token != 0 {
        let elem = parse_value(lex, flags, error);
        if elem.is_null() {
            json_decref(array);
            return core::ptr::null_mut();
        }

        if json_array_append_new(array, elem) != 0 {
            json_decref(array);
            return core::ptr::null_mut();
        }

        lex_scan(lex, error);
        if (*lex).token != b',' as c_int {
            break;
        }

        lex_scan(lex, error);
    }

    if (*lex).token != b']' as c_int {
        error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"']' expected");
        json_decref(array);
        return core::ptr::null_mut();
    }

    array
}

unsafe fn parse_value(lex: *mut LexT, flags: usize, error: *mut JsonErrorT) -> *mut JsonT {
    let json: *mut JsonT;

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
            let value = (*lex).string_val;
            let len = (*lex).string_len;

            if (flags & JSON_ALLOW_NUL) == 0
                && !memchr(value as *const c_void, 0, len).is_null()
            {
                error_set(
                    error,
                    lex,
                    JSON_ERROR_NULL_CHARACTER,
                    b"\\u0000 is not allowed without JSON_ALLOW_NUL",
                );
                return core::ptr::null_mut();
            }

            json = jsonp_stringn_nocheck_own(value, len);
            (*lex).string_val = core::ptr::null_mut();
            (*lex).string_len = 0;
        }

        TOKEN_INTEGER => {
            json = json_integer((*lex).integer);
        }

        TOKEN_REAL => {
            json = json_real((*lex).real);
        }

        TOKEN_TRUE => json = json_true(),

        TOKEN_FALSE => json = json_false(),

        TOKEN_NULL => json = json_null(),

        123 => json = parse_object(lex, flags, error), /* '{' */

        91 => json = parse_array(lex, flags, error), /* '[' */

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

unsafe fn parse_json(lex: *mut LexT, flags: usize, error: *mut JsonErrorT) -> *mut JsonT {
    let result: *mut JsonT;

    (*lex).depth = 0;

    lex_scan(lex, error);
    if (flags & JSON_DECODE_ANY) == 0
        && (*lex).token != b'[' as c_int
        && (*lex).token != b'{' as c_int
    {
        error_set(error, lex, JSON_ERROR_INVALID_SYNTAX, b"'[' or '{' expected");
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

#[repr(C)]
struct StringDataT {
    data: *const c_char,
    pos: usize,
}

unsafe extern "C" fn string_get(data: *mut c_void) -> c_int {
    let c: c_char;
    let stream = data as *mut StringDataT;
    c = *(*stream).data.add((*stream).pos);
    if c == 0 {
        EOF
    } else {
        (*stream).pos += 1;
        (c as u8) as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loads(
    string: *const c_char,
    flags: usize,
    error: *mut JsonErrorT,
) -> *mut JsonT {
    let mut lex = new_lex(string_get, core::ptr::null_mut());
    let result: *mut JsonT;
    let mut stream_data = StringDataT {
        data: core::ptr::null(),
        pos: 0,
    };

    jsonp_error_init(error, b"<string>\0".as_ptr() as *const c_char);

    if string.is_null() {
        error_set_no_lex(error, JSON_ERROR_INVALID_ARGUMENT, b"wrong arguments");
        return core::ptr::null_mut();
    }

    stream_data.data = string;
    stream_data.pos = 0;

    if lex_init(
        &mut lex,
        string_get,
        flags,
        &mut stream_data as *mut StringDataT as *mut c_void,
    ) != 0
    {
        return core::ptr::null_mut();
    }

    result = parse_json(&mut lex, flags, error);

    lex_close(&mut lex);
    result
}

/// `error_set(error, NULL, code, msg)`
unsafe fn error_set_no_lex(error: *mut JsonErrorT, code: c_int, msg: &[u8]) {
    error_set(error, core::ptr::null(), code, msg);
}

#[repr(C)]
struct BufferDataT {
    data: *const c_char,
    len: usize,
    pos: usize,
}

unsafe extern "C" fn buffer_get(data: *mut c_void) -> c_int {
    let c: c_char;
    let stream = data as *mut BufferDataT;
    if (*stream).pos >= (*stream).len {
        return EOF;
    }

    c = *(*stream).data.add((*stream).pos);
    (*stream).pos += 1;
    (c as u8) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadb(
    buffer: *const c_char,
    buflen: usize,
    flags: usize,
    error: *mut JsonErrorT,
) -> *mut JsonT {
    let mut lex = new_lex(buffer_get, core::ptr::null_mut());
    let result: *mut JsonT;
    let mut stream_data = BufferDataT {
        data: core::ptr::null(),
        len: 0,
        pos: 0,
    };

    jsonp_error_init(error, b"<buffer>\0".as_ptr() as *const c_char);

    if buffer.is_null() {
        error_set_no_lex(error, JSON_ERROR_INVALID_ARGUMENT, b"wrong arguments");
        return core::ptr::null_mut();
    }

    stream_data.data = buffer;
    stream_data.pos = 0;
    stream_data.len = buflen;

    if lex_init(
        &mut lex,
        buffer_get,
        flags,
        &mut stream_data as *mut BufferDataT as *mut c_void,
    ) != 0
    {
        return core::ptr::null_mut();
    }

    result = parse_json(&mut lex, flags, error);

    lex_close(&mut lex);
    result
}

unsafe extern "C" fn file_get(data: *mut c_void) -> c_int {
    fgetc(data)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadf(
    input: *mut c_void,
    flags: usize,
    error: *mut JsonErrorT,
) -> *mut JsonT {
    let mut lex = new_lex(file_get, core::ptr::null_mut());
    let source: *const c_char;
    let result: *mut JsonT;

    if input == stdin {
        source = b"<stdin>\0".as_ptr() as *const c_char;
    } else {
        source = b"<stream>\0".as_ptr() as *const c_char;
    }

    jsonp_error_init(error, source);

    if input.is_null() {
        error_set_no_lex(error, JSON_ERROR_INVALID_ARGUMENT, b"wrong arguments");
        return core::ptr::null_mut();
    }

    if lex_init(&mut lex, file_get, flags, input) != 0 {
        return core::ptr::null_mut();
    }

    result = parse_json(&mut lex, flags, error);

    lex_close(&mut lex);
    result
}

unsafe extern "C" fn fd_get_func(fd: *mut c_void) -> c_int {
    let mut c: u8 = 0;
    if read(*(fd as *mut c_int), &mut c as *mut u8 as *mut c_void, 1) == 1 {
        return c as c_int;
    }
    EOF
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadfd(
    input: c_int,
    flags: usize,
    error: *mut JsonErrorT,
) -> *mut JsonT {
    let mut lex = new_lex(fd_get_func, core::ptr::null_mut());
    let source: *const c_char;
    let result: *mut JsonT;
    let mut input = input;

    if input == STDIN_FILENO {
        source = b"<stdin>\0".as_ptr() as *const c_char;
    } else {
        source = b"<stream>\0".as_ptr() as *const c_char;
    }

    jsonp_error_init(error, source);

    if input < 0 {
        error_set_no_lex(error, JSON_ERROR_INVALID_ARGUMENT, b"wrong arguments");
        return core::ptr::null_mut();
    }

    if lex_init(
        &mut lex,
        fd_get_func,
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
    error: *mut JsonErrorT,
) -> *mut JsonT {
    let result: *mut JsonT;
    let fp: *mut c_void;

    jsonp_error_init(error, path);

    if path.is_null() {
        error_set_no_lex(error, JSON_ERROR_INVALID_ARGUMENT, b"wrong arguments");
        return core::ptr::null_mut();
    }

    fp = fopen(path, b"rb\0".as_ptr() as *const c_char);
    if fp.is_null() {
        let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
        b.push(b"unable to open ");
        b.push_cstr(path);
        b.push(b": ");
        b.push_cstr(strerror(errno()));
        error_set_no_lex(error, JSON_ERROR_CANNOT_OPEN_FILE, b.as_bytes());
        return core::ptr::null_mut();
    }

    result = json_loadf(fp, flags, error);

    fclose(fp);
    result
}

const MAX_BUF_LEN: usize = 1024;

#[repr(C)]
struct CallbackDataT {
    data: [c_char; MAX_BUF_LEN],
    len: usize,
    pos: usize,
    callback: Option<JsonLoadCallbackT>,
    arg: *mut c_void,
}

unsafe extern "C" fn callback_get(data: *mut c_void) -> c_int {
    let c: c_char;
    let stream = data as *mut CallbackDataT;

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
    (c as u8) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_load_callback(
    callback: Option<JsonLoadCallbackT>,
    arg: *mut c_void,
    flags: usize,
    error: *mut JsonErrorT,
) -> *mut JsonT {
    let mut lex = new_lex(callback_get, core::ptr::null_mut());
    let result: *mut JsonT;

    let mut stream_data: CallbackDataT = core::mem::zeroed();
    stream_data.callback = callback;
    stream_data.arg = arg;

    jsonp_error_init(error, b"<callback>\0".as_ptr() as *const c_char);

    if callback.is_none() {
        error_set_no_lex(error, JSON_ERROR_INVALID_ARGUMENT, b"wrong arguments");
        return core::ptr::null_mut();
    }

    if lex_init(
        &mut lex,
        callback_get,
        flags,
        &mut stream_data as *mut CallbackDataT as *mut c_void,
    ) != 0
    {
        return core::ptr::null_mut();
    }

    result = parse_json(&mut lex, flags, error);

    lex_close(&mut lex);
    result
}
