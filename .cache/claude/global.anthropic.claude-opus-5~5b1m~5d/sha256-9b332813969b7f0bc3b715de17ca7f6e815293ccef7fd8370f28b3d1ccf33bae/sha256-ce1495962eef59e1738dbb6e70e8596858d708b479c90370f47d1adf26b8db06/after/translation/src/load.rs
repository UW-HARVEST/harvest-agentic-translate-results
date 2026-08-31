//! Translation of `src/load.c`.

use core::ffi::{c_char, c_double, c_int, c_uint, c_void};

use crate::error::{error_vset_with, jsonp_error_init};
use crate::ffi::{self, FILE};
use crate::jansson::*;
use crate::memory::{jsonp_free, jsonp_malloc};
use crate::strbuffer::*;
use crate::utf::{utf8_check_first, utf8_check_full, utf8_encode};
use crate::value::*;

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
struct lex_string {
    val: *mut c_char,
    len: usize,
}

#[repr(C)]
struct lex_t {
    stream: stream_t,
    saved_text: strbuffer_t,
    flags: usize,
    depth: usize,
    token: c_int,
    value_string: lex_string,
    value_integer: json_int_t,
    value_real: c_double,
}

#[inline]
unsafe fn stream_to_lex(stream: *mut stream_t) -> *mut lex_t {
    stream as *mut lex_t
}

/*** error reporting ***/

/// Mirror of the static `error_set()` helper. The caller formats the message
/// into a 160 byte buffer, exactly like the `vsnprintf()` call in the C code.
unsafe fn error_set_with<F>(
    error: *mut json_error_t,
    lex: *const lex_t,
    mut code: c_int,
    fmtmsg: F,
) where
    F: FnOnce(*mut c_char, usize),
{
    let mut msg_text = [0 as c_char; JSON_ERROR_TEXT_LENGTH];
    let mut msg_with_context = [0 as c_char; JSON_ERROR_TEXT_LENGTH];

    let mut line: c_int = -1;
    let mut col: c_int = -1;
    let mut pos: usize = 0;
    let mut result: *const c_char = msg_text.as_ptr();

    if error.is_null() {
        return;
    }

    fmtmsg(msg_text.as_mut_ptr(), JSON_ERROR_TEXT_LENGTH);
    msg_text[JSON_ERROR_TEXT_LENGTH - 1] = 0;

    if !lex.is_null() {
        let saved_text = strbuffer_value(core::ptr::addr_of!((*lex).saved_text));

        line = (*lex).stream.line;
        col = (*lex).stream.column;
        pos = (*lex).stream.position;

        if !saved_text.is_null() && *saved_text != 0 {
            if (*lex).saved_text.length <= 20 {
                ffi::snprintf(
                    msg_with_context.as_mut_ptr(),
                    JSON_ERROR_TEXT_LENGTH,
                    b"%s near '%s'\0".as_ptr() as *const c_char,
                    msg_text.as_ptr(),
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
                result = msg_text.as_ptr();
            } else {
                ffi::snprintf(
                    msg_with_context.as_mut_ptr(),
                    JSON_ERROR_TEXT_LENGTH,
                    b"%s near end of file\0".as_ptr() as *const c_char,
                    msg_text.as_ptr(),
                );
                msg_with_context[JSON_ERROR_TEXT_LENGTH - 1] = 0;
                result = msg_with_context.as_ptr();
            }
        }
    }

    error_vset_with(error, line, col, pos, code, |buf, n| {
        ffi::snprintf(buf, n, b"%s\0".as_ptr() as *const c_char, result);
    });
}

/// `error_set()` with a plain, already-NUL-terminated message.
unsafe fn error_set(
    error: *mut json_error_t,
    lex: *const lex_t,
    code: c_int,
    msg: *const c_char,
) {
    error_set_with(error, lex, code, |buf, n| {
        ffi::snprintf(buf, n, b"%s\0".as_ptr() as *const c_char, msg);
    });
}

/*** lexical analyzer ***/

unsafe fn stream_init(stream: *mut stream_t, get: get_func, data: *mut c_void) {
    (*stream).get = Some(get);
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
        if c == ffi::EOF {
            (*stream).state = STREAM_STATE_EOF;
            return STREAM_STATE_EOF;
        }

        (*stream).buffer[0] = c as c_char;
        (*stream).buffer_pos = 0;

        if (0x80..=0xFF).contains(&c) {
            /* multi-byte UTF-8 sequence */
            let count = utf8_check_first(c as c_char);
            if count == 0 {
                /* goto out */
                (*stream).state = STREAM_STATE_ERROR;
                error_set_with(
                    error,
                    stream_to_lex(stream),
                    json_error_invalid_utf8,
                    |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"unable to decode byte 0x%x\0".as_ptr() as *const c_char,
                            c,
                        );
                    },
                );
                return STREAM_STATE_ERROR;
            }

            for i in 1..count {
                (*stream).buffer[i] = ((*stream).get.unwrap())((*stream).data) as c_char;
            }

            if utf8_check_full((*stream).buffer.as_ptr(), count, core::ptr::null_mut()) == 0 {
                /* goto out */
                (*stream).state = STREAM_STATE_ERROR;
                error_set_with(
                    error,
                    stream_to_lex(stream),
                    json_error_invalid_utf8,
                    |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"unable to decode byte 0x%x\0".as_ptr() as *const c_char,
                            c,
                        );
                    },
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
    if c == b'\n' as c_int {
        (*stream).line += 1;
        (*stream).last_column = (*stream).column;
        (*stream).column = 0;
    } else if utf8_check_first(c as c_char) != 0 {
        /* track the Unicode character column, so increment only if this is the
        first character of a UTF-8 sequence */
        (*stream).column += 1;
    }

    c
}

unsafe fn stream_unget(stream: *mut stream_t, c: c_int) {
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
        strbuffer_pop(core::ptr::addr_of_mut!((*lex).saved_text));
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
    jsonp_free((*lex).value_string.val as *mut c_void);
    (*lex).value_string.val = core::ptr::null_mut();
    (*lex).value_string.len = 0;
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

unsafe fn lex_scan_string(lex: *mut lex_t, error: *mut json_error_t) {
    let mut c: c_int;
    let mut p: *const c_char;
    let mut t: *mut c_char;

    (*lex).value_string.val = core::ptr::null_mut();
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
                json_error_premature_end_of_input,
                b"premature end of input\0".as_ptr() as *const c_char,
            );
            lex_free_string(lex);
            return;
        } else if (0..=0x1F).contains(&c) {
            /* control character */
            lex_unget_unsave(lex, c);
            if c == b'\n' as c_int {
                error_set(
                    error,
                    lex,
                    json_error_invalid_syntax,
                    b"unexpected newline\0".as_ptr() as *const c_char,
                );
            } else {
                error_set_with(error, lex, json_error_invalid_syntax, |buf, n| {
                    ffi::snprintf(
                        buf,
                        n,
                        b"control character 0x%x\0".as_ptr() as *const c_char,
                        c,
                    );
                });
            }
            lex_free_string(lex);
            return;
        } else if c == b'\\' as c_int {
            c = lex_get_save(lex, error);
            if c == b'u' as c_int {
                c = lex_get_save(lex, error);
                for _ in 0..4 {
                    if !l_isxdigit(c) {
                        error_set(
                            error,
                            lex,
                            json_error_invalid_syntax,
                            b"invalid escape\0".as_ptr() as *const c_char,
                        );
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
                error_set(
                    error,
                    lex,
                    json_error_invalid_syntax,
                    b"invalid escape\0".as_ptr() as *const c_char,
                );
                lex_free_string(lex);
                return;
            }
        } else {
            c = lex_get_save(lex, error);
        }
    }

    /* the actual value is at most of the same length as the source string,
       because:
         - shortcut escapes (e.g. "\t") (length 2) are converted to 1 byte
         - a single \uXXXX escape (length 6) is converted to at most 3 bytes
         - two \uXXXX escapes (length 12) forming an UTF-16 surrogate pair
           are converted to 4 bytes
    */
    t = jsonp_malloc((*lex).saved_text.length + 1) as *mut c_char;
    if t.is_null() {
        /* this is not very nice, since TOKEN_INVALID is returned */
        lex_free_string(lex);
        return;
    }
    (*lex).value_string.val = t;

    /* + 1 to skip the " */
    p = strbuffer_value(core::ptr::addr_of!((*lex).saved_text)).add(1);

    while *p != b'"' as c_char {
        if *p == b'\\' as c_char {
            p = p.add(1);
            if *p == b'u' as c_char {
                let mut length: usize = 0;
                let mut value: i32;

                value = decode_unicode_escape(p);
                if value < 0 {
                    error_set_with(error, lex, json_error_invalid_syntax, |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"invalid Unicode escape '%.6s'\0".as_ptr() as *const c_char,
                            p.offset(-1),
                        );
                    });
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
                            error_set_with(error, lex, json_error_invalid_syntax, |buf, n| {
                                ffi::snprintf(
                                    buf,
                                    n,
                                    b"invalid Unicode escape '%.6s'\0".as_ptr() as *const c_char,
                                    p.offset(-1),
                                );
                            });
                            lex_free_string(lex);
                            return;
                        }
                        p = p.add(5);

                        if (0xDC00..=0xDFFF).contains(&value2) {
                            /* valid second surrogate */
                            value = ((value - 0xD800) << 10) + (value2 - 0xDC00) + 0x10000;
                        } else {
                            /* invalid second surrogate */
                            error_set_with(error, lex, json_error_invalid_syntax, |buf, n| {
                                ffi::snprintf(
                                    buf,
                                    n,
                                    b"invalid Unicode '\\u%04X\\u%04X'\0".as_ptr()
                                        as *const c_char,
                                    value as c_uint,
                                    value2 as c_uint,
                                );
                            });
                            lex_free_string(lex);
                            return;
                        }
                    } else {
                        /* no second surrogate */
                        error_set_with(error, lex, json_error_invalid_syntax, |buf, n| {
                            ffi::snprintf(
                                buf,
                                n,
                                b"invalid Unicode '\\u%04X'\0".as_ptr() as *const c_char,
                                value as c_uint,
                            );
                        });
                        lex_free_string(lex);
                        return;
                    }
                } else if (0xDC00..=0xDFFF).contains(&value) {
                    error_set_with(error, lex, json_error_invalid_syntax, |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"invalid Unicode '\\u%04X'\0".as_ptr() as *const c_char,
                            value as c_uint,
                        );
                    });
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
    (*lex).value_string.len = t as usize - (*lex).value_string.val as usize;
    (*lex).token = TOKEN_STRING;
}

unsafe fn lex_scan_number(lex: *mut lex_t, mut c: c_int, error: *mut json_error_t) -> c_int {
    let saved_text: *const c_char;
    let mut end: *mut c_char = core::ptr::null_mut();
    let mut doubleval: c_double = 0.0;

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
        lex_unget_unsave(lex, c);

        saved_text = strbuffer_value(core::ptr::addr_of!((*lex).saved_text));

        ffi::set_errno(0);
        let intval = ffi::strtoll(saved_text, &mut end, 10);
        if ffi::errno() == ffi::ERANGE {
            if intval < 0 {
                error_set(
                    error,
                    lex,
                    json_error_numeric_overflow,
                    b"too big negative integer\0".as_ptr() as *const c_char,
                );
            } else {
                error_set(
                    error,
                    lex,
                    json_error_numeric_overflow,
                    b"too big integer\0".as_ptr() as *const c_char,
                );
            }
            return -1;
        }

        (*lex).token = TOKEN_INTEGER;
        (*lex).value_integer = intval;
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

    if crate::strconv::jsonp_strtod(core::ptr::addr_of_mut!((*lex).saved_text), &mut doubleval) != 0
    {
        error_set(
            error,
            lex,
            json_error_numeric_overflow,
            b"real number overflow\0".as_ptr() as *const c_char,
        );
        return -1;
    }

    (*lex).token = TOKEN_REAL;
    (*lex).value_real = doubleval;
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
        if !(c == b' ' as c_int
            || c == b'\t' as c_int
            || c == b'\n' as c_int
            || c == b'\r' as c_int)
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
        loop {
            c = lex_get_save(lex, error);
            if !l_isalpha(c) {
                break;
            }
        }
        lex_unget_unsave(lex, c);

        let saved_text = strbuffer_value(core::ptr::addr_of!((*lex).saved_text));

        if ffi::strcmp(saved_text, b"true\0".as_ptr() as *const c_char) == 0 {
            (*lex).token = TOKEN_TRUE;
        } else if ffi::strcmp(saved_text, b"false\0".as_ptr() as *const c_char) == 0 {
            (*lex).token = TOKEN_FALSE;
        } else if ffi::strcmp(saved_text, b"null\0".as_ptr() as *const c_char) == 0 {
            (*lex).token = TOKEN_NULL;
        } else {
            (*lex).token = TOKEN_INVALID;
        }
    } else {
        /* save the rest of the input UTF-8 sequence to get an error message of
        valid UTF-8 */
        lex_save_cached(lex);
        (*lex).token = TOKEN_INVALID;
    }

    (*lex).token
}

unsafe fn lex_steal_string(lex: *mut lex_t, out_len: *mut usize) -> *mut c_char {
    let mut result: *mut c_char = core::ptr::null_mut();
    if (*lex).token == TOKEN_STRING {
        result = (*lex).value_string.val;
        *out_len = (*lex).value_string.len;
        (*lex).value_string.val = core::ptr::null_mut();
        (*lex).value_string.len = 0;
    }
    result
}

unsafe fn lex_init(
    lex: *mut lex_t,
    get: get_func,
    flags: usize,
    data: *mut c_void,
) -> c_int {
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

/*** parser ***/

unsafe fn parse_object(lex: *mut lex_t, flags: usize, error: *mut json_error_t) -> *mut json_t {
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
        let value: *mut json_t;

        if (*lex).token != TOKEN_STRING {
            error_set(
                error,
                lex,
                json_error_invalid_syntax,
                b"string or '}' expected\0".as_ptr() as *const c_char,
            );
            json_decref(object);
            return core::ptr::null_mut();
        }

        key = lex_steal_string(lex, &mut len);
        if key.is_null() {
            return core::ptr::null_mut();
        }
        if !ffi::memchr(key as *const c_void, 0, len).is_null() {
            jsonp_free(key as *mut c_void);
            error_set(
                error,
                lex,
                json_error_null_byte_in_key,
                b"NUL byte in object key not supported\0".as_ptr() as *const c_char,
            );
            json_decref(object);
            return core::ptr::null_mut();
        }

        if (flags & JSON_REJECT_DUPLICATES) != 0
            && !json_object_getn(object, key, len).is_null()
        {
            jsonp_free(key as *mut c_void);
            error_set(
                error,
                lex,
                json_error_duplicate_key,
                b"duplicate object key\0".as_ptr() as *const c_char,
            );
            json_decref(object);
            return core::ptr::null_mut();
        }

        lex_scan(lex, error);
        if (*lex).token != b':' as c_int {
            jsonp_free(key as *mut c_void);
            error_set(
                error,
                lex,
                json_error_invalid_syntax,
                b"':' expected\0".as_ptr() as *const c_char,
            );
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
        error_set(
            error,
            lex,
            json_error_invalid_syntax,
            b"'}' expected\0".as_ptr() as *const c_char,
        );
        json_decref(object);
        return core::ptr::null_mut();
    }

    object
}

unsafe fn parse_array(lex: *mut lex_t, flags: usize, error: *mut json_error_t) -> *mut json_t {
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
        error_set(
            error,
            lex,
            json_error_invalid_syntax,
            b"']' expected\0".as_ptr() as *const c_char,
        );
        json_decref(array);
        return core::ptr::null_mut();
    }

    array
}

unsafe fn parse_value(lex: *mut lex_t, flags: usize, error: *mut json_error_t) -> *mut json_t {
    let json: *mut json_t;

    (*lex).depth += 1;
    if (*lex).depth > JSON_PARSER_MAX_DEPTH {
        error_set(
            error,
            lex,
            json_error_stack_overflow,
            b"maximum parsing depth reached\0".as_ptr() as *const c_char,
        );
        return core::ptr::null_mut();
    }

    match (*lex).token {
        TOKEN_STRING => {
            let value = (*lex).value_string.val;
            let len = (*lex).value_string.len;

            if (flags & JSON_ALLOW_NUL) == 0
                && !ffi::memchr(value as *const c_void, 0, len).is_null()
            {
                error_set(
                    error,
                    lex,
                    json_error_null_character,
                    b"\\u0000 is not allowed without JSON_ALLOW_NUL\0".as_ptr() as *const c_char,
                );
                return core::ptr::null_mut();
            }

            json = jsonp_stringn_nocheck_own(value, len);
            (*lex).value_string.val = core::ptr::null_mut();
            (*lex).value_string.len = 0;
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

        0x7B /* '{' */ => json = parse_object(lex, flags, error),

        0x5B /* '[' */ => json = parse_array(lex, flags, error),

        TOKEN_INVALID => {
            error_set(
                error,
                lex,
                json_error_invalid_syntax,
                b"invalid token\0".as_ptr() as *const c_char,
            );
            return core::ptr::null_mut();
        }

        _ => {
            error_set(
                error,
                lex,
                json_error_invalid_syntax,
                b"unexpected token\0".as_ptr() as *const c_char,
            );
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
    (*lex).depth = 0;

    lex_scan(lex, error);
    if (flags & JSON_DECODE_ANY) == 0
        && (*lex).token != b'[' as c_int
        && (*lex).token != b'{' as c_int
    {
        error_set(
            error,
            lex,
            json_error_invalid_syntax,
            b"'[' or '{' expected\0".as_ptr() as *const c_char,
        );
        return core::ptr::null_mut();
    }

    let result = parse_value(lex, flags, error);
    if result.is_null() {
        return core::ptr::null_mut();
    }

    if (flags & JSON_DISABLE_EOF_CHECK) == 0 {
        lex_scan(lex, error);
        if (*lex).token != TOKEN_EOF {
            error_set(
                error,
                lex,
                json_error_end_of_input_expected,
                b"end of file expected\0".as_ptr() as *const c_char,
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

unsafe fn new_lex() -> lex_t {
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
        value_string: lex_string {
            val: core::ptr::null_mut(),
            len: 0,
        },
        value_integer: 0,
        value_real: 0.0,
    }
}

#[repr(C)]
struct string_data_t {
    data: *const c_char,
    pos: usize,
}

unsafe extern "C" fn string_get(data: *mut c_void) -> c_int {
    let stream = data as *mut string_data_t;
    let c = *(*stream).data.add((*stream).pos);
    if c == 0 {
        ffi::EOF
    } else {
        (*stream).pos += 1;
        (c as u8) as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loads(
    string: *const c_char,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    let mut lex = new_lex();
    let mut stream_data = string_data_t {
        data: core::ptr::null(),
        pos: 0,
    };

    jsonp_error_init(error, b"<string>\0".as_ptr() as *const c_char);

    if string.is_null() {
        error_set(
            error,
            core::ptr::null(),
            json_error_invalid_argument,
            b"wrong arguments\0".as_ptr() as *const c_char,
        );
        return core::ptr::null_mut();
    }

    stream_data.data = string;
    stream_data.pos = 0;

    if lex_init(
        &mut lex,
        string_get,
        flags,
        &mut stream_data as *mut string_data_t as *mut c_void,
    ) != 0
    {
        return core::ptr::null_mut();
    }

    let result = parse_json(&mut lex, flags, error);

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
    let stream = data as *mut buffer_data_t;
    if (*stream).pos >= (*stream).len {
        return ffi::EOF;
    }

    let c = *(*stream).data.add((*stream).pos);
    (*stream).pos += 1;
    (c as u8) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadb(
    buffer: *const c_char,
    buflen: usize,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    let mut lex = new_lex();
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
            json_error_invalid_argument,
            b"wrong arguments\0".as_ptr() as *const c_char,
        );
        return core::ptr::null_mut();
    }

    stream_data.data = buffer;
    stream_data.pos = 0;
    stream_data.len = buflen;

    if lex_init(
        &mut lex,
        buffer_get,
        flags,
        &mut stream_data as *mut buffer_data_t as *mut c_void,
    ) != 0
    {
        return core::ptr::null_mut();
    }

    let result = parse_json(&mut lex, flags, error);

    lex_close(&mut lex);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadf(
    input: *mut FILE,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    let mut lex = new_lex();
    let source: *const c_char;

    if input == ffi::c_stdin {
        source = b"<stdin>\0".as_ptr() as *const c_char;
    } else {
        source = b"<stream>\0".as_ptr() as *const c_char;
    }

    jsonp_error_init(error, source);

    if input.is_null() {
        error_set(
            error,
            core::ptr::null(),
            json_error_invalid_argument,
            b"wrong arguments\0".as_ptr() as *const c_char,
        );
        return core::ptr::null_mut();
    }

    let getf: get_func = core::mem::transmute(
        ffi::fgetc as unsafe extern "C" fn(*mut FILE) -> c_int,
    );
    if lex_init(&mut lex, getf, flags, input as *mut c_void) != 0 {
        return core::ptr::null_mut();
    }

    let result = parse_json(&mut lex, flags, error);

    lex_close(&mut lex);
    result
}

unsafe extern "C" fn fd_get_func(fd: *mut c_int) -> c_int {
    let mut c: u8 = 0;
    if ffi::read(*fd, &mut c as *mut u8 as *mut c_void, 1) == 1 {
        return c as c_int;
    }
    ffi::EOF
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadfd(
    input: c_int,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    let mut lex = new_lex();
    let source: *const c_char;
    let mut input = input;

    if input == ffi::STDIN_FILENO {
        source = b"<stdin>\0".as_ptr() as *const c_char;
    } else {
        source = b"<stream>\0".as_ptr() as *const c_char;
    }

    jsonp_error_init(error, source);

    if input < 0 {
        error_set(
            error,
            core::ptr::null(),
            json_error_invalid_argument,
            b"wrong arguments\0".as_ptr() as *const c_char,
        );
        return core::ptr::null_mut();
    }

    let getf: get_func = core::mem::transmute(
        fd_get_func as unsafe extern "C" fn(*mut c_int) -> c_int,
    );
    if lex_init(
        &mut lex,
        getf,
        flags,
        &mut input as *mut c_int as *mut c_void,
    ) != 0
    {
        return core::ptr::null_mut();
    }

    let result = parse_json(&mut lex, flags, error);

    lex_close(&mut lex);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_load_file(
    path: *const c_char,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    jsonp_error_init(error, path);

    if path.is_null() {
        error_set(
            error,
            core::ptr::null(),
            json_error_invalid_argument,
            b"wrong arguments\0".as_ptr() as *const c_char,
        );
        return core::ptr::null_mut();
    }

    let fp = ffi::fopen(path, b"rb\0".as_ptr() as *const c_char);
    if fp.is_null() {
        error_set_with(
            error,
            core::ptr::null(),
            json_error_cannot_open_file,
            |buf, n| {
                ffi::snprintf(
                    buf,
                    n,
                    b"unable to open %s: %s\0".as_ptr() as *const c_char,
                    path,
                    ffi::strerror(ffi::errno()),
                );
            },
        );
        return core::ptr::null_mut();
    }

    let result = json_loadf(fp, flags, error);

    ffi::fclose(fp);
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
    let stream = data as *mut callback_data_t;

    if (*stream).pos >= (*stream).len {
        (*stream).pos = 0;
        (*stream).len = ((*stream).callback.unwrap())(
            (*stream).data.as_mut_ptr() as *mut c_void,
            MAX_BUF_LEN,
            (*stream).arg,
        );
        if (*stream).len == 0 || (*stream).len == usize::MAX {
            return ffi::EOF;
        }
    }

    let c = (*stream).data[(*stream).pos];
    (*stream).pos += 1;
    (c as u8) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_load_callback(
    callback: json_load_callback_t,
    arg: *mut c_void,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    let mut lex = new_lex();

    let mut stream_data: callback_data_t = core::mem::zeroed();
    stream_data.callback = callback;
    stream_data.arg = arg;

    jsonp_error_init(error, b"<callback>\0".as_ptr() as *const c_char);

    if callback.is_none() {
        error_set(
            error,
            core::ptr::null(),
            json_error_invalid_argument,
            b"wrong arguments\0".as_ptr() as *const c_char,
        );
        return core::ptr::null_mut();
    }

    if lex_init(
        &mut lex,
        callback_get,
        flags,
        &mut stream_data as *mut callback_data_t as *mut c_void,
    ) != 0
    {
        return core::ptr::null_mut();
    }

    let result = parse_json(&mut lex, flags, error);

    lex_close(&mut lex);
    result
}
