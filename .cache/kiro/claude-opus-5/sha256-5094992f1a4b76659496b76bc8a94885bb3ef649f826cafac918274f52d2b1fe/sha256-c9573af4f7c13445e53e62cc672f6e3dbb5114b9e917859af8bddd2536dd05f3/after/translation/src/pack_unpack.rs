//! Translation of `src/pack_unpack.c`.

use crate::cffi;
use crate::error::{jsonp_error_init, jsonp_error_set_source, jsonp_error_set_str};
use crate::hashtable::{hashtable_close, hashtable_get, hashtable_init, hashtable_set, hashtable_t};
use crate::jtypes::*;
use crate::memory::jsonp_free;
use crate::strbuffer::{
    strbuffer_append_bytes, strbuffer_close, strbuffer_init, strbuffer_steal_value, strbuffer_t,
    strbuffer_value,
};
use crate::utf::utf8_check_string;
use crate::valist::*;
use crate::value::*;
use core::ffi::{c_char, c_int, c_long, c_ulong, c_void};
use core::ptr::{null_mut, addr_of_mut};

#[derive(Clone, Copy)]
#[repr(C)]
struct token_t {
    line: c_int,
    column: c_int,
    pos: usize,
    token: c_char,
}

impl token_t {
    const fn zero() -> Self {
        token_t {
            line: 0,
            column: 0,
            pos: 0,
            token: 0,
        }
    }
}

#[repr(C)]
struct scanner_t {
    start: *const c_char,
    fmt: *const c_char,
    prev_token: token_t,
    token: token_t,
    next_token: token_t,
    error: *mut json_error_t,
    flags: usize,
    line: c_int,
    column: c_int,
    pos: usize,
    has_error: c_int,
}

impl scanner_t {
    const fn zero() -> Self {
        scanner_t {
            start: core::ptr::null(),
            fmt: core::ptr::null(),
            prev_token: token_t::zero(),
            token: token_t::zero(),
            next_token: token_t::zero(),
            error: null_mut(),
            flags: 0,
            line: 0,
            column: 0,
            pos: 0,
            has_error: 0,
        }
    }
}

#[inline]
unsafe fn token(s: *const scanner_t) -> c_char {
    unsafe { (*s).token.token }
}

static TYPE_NAMES: [&[u8]; 8] = [
    b"object\0",
    b"array\0",
    b"string\0",
    b"integer\0",
    b"real\0",
    b"true\0",
    b"false\0",
    b"null\0",
];

#[inline]
unsafe fn type_name(x: *const json_t) -> *const c_char {
    unsafe { TYPE_NAMES[json_typeof(x) as usize].as_ptr() as *const c_char }
}

const UNPACK_VALUE_STARTERS: &[u8] = b"{[siIbfFOon\0";

unsafe fn scanner_init(
    s: *mut scanner_t,
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
) {
    unsafe {
        (*s).error = error;
        (*s).flags = flags;
        (*s).fmt = fmt;
        (*s).start = fmt;
        (*s).prev_token = token_t::zero();
        (*s).token = token_t::zero();
        (*s).next_token = token_t::zero();
        (*s).line = 1;
        (*s).column = 0;
        (*s).pos = 0;
        (*s).has_error = 0;
    }
}

unsafe fn next_token(s: *mut scanner_t) {
    unsafe {
        (*s).prev_token = (*s).token;

        if (*s).next_token.line != 0 {
            (*s).token = (*s).next_token;
            (*s).next_token.line = 0;
            return;
        }

        if token(s) == 0 && *(*s).fmt == 0 {
            return;
        }

        let mut t = (*s).fmt;
        (*s).column += 1;
        (*s).pos += 1;

        /* skip space and ignored chars */
        while *t == b' ' as c_char
            || *t == b'\t' as c_char
            || *t == b'\n' as c_char
            || *t == b',' as c_char
            || *t == b':' as c_char
        {
            if *t == b'\n' as c_char {
                (*s).line += 1;
                (*s).column = 1;
            } else {
                (*s).column += 1;
            }

            (*s).pos += 1;
            t = t.add(1);
        }

        (*s).token.token = *t;
        (*s).token.line = (*s).line;
        (*s).token.column = (*s).column;
        (*s).token.pos = (*s).pos;

        if *t != 0 {
            t = t.add(1);
        }
        (*s).fmt = t;
    }
}

unsafe fn prev_token(s: *mut scanner_t) {
    unsafe {
        (*s).next_token = (*s).token;
        (*s).token = (*s).prev_token;
    }
}

/// `set_error()` with the message already rendered by the caller.
/// `static void set_error(scanner_t *s, const char *source,
///                        enum json_error_code code, const char *fmt, ...)`
///
/// The C hands `fmt` and its arguments straight to `jsonp_error_vset()`, which
/// `vsnprintf()`s them into `error->text`. Rendering into an intermediate
/// buffer and then passing that through a `"%s"` conversion is NOT equivalent:
/// a `%c` conversion of the value `0` writes a NUL byte into the middle of the
/// message and the C keeps writing the rest of the format after it (this
/// happens for real, e.g. `"Unexpected format character '%c'"` when the token
/// is the end-of-format NUL). So format directly into `error->text` here.
macro_rules! set_error {
    ($s:expr, $source:expr, $code:expr, $fmt:expr $(, $arg:expr)* $(,)?) => {{
        let s_ = $s;
        let error_ = (*s_).error;
        /* jsonp_error_vset() */
        if !error_.is_null() && (*error_).text[0] == 0 {
            (*error_).line = (*s_).token.line;
            (*error_).column = (*s_).token.column;
            (*error_).position = (*s_).token.pos as c_int;
            cffi::snprintf(
                (*error_).text.as_mut_ptr(),
                JSON_ERROR_TEXT_LENGTH - 1,
                $fmt
                $(, $arg)*
            );
            (*error_).text[JSON_ERROR_TEXT_LENGTH - 2] = 0;
            (*error_).text[JSON_ERROR_TEXT_LENGTH - 1] = $code as c_char;
        }
        jsonp_error_set_source(error_, $source);
    }};
}

/* ours will be set to 1 if jsonp_free() must be called for the result
   afterwards */
unsafe fn read_string(
    s: *mut scanner_t,
    ap: VaList,
    purpose: *const c_char,
    out_len: *mut usize,
    ours: *mut c_int,
    optional: c_int,
) -> *mut c_char {
    unsafe {
        let mut strbuff = strbuffer_t::new();
        let mut str_: *const c_char;
        let mut length: usize;

        next_token(s);
        let t = token(s);
        prev_token(s);

        *ours = 0;
        if t != b'#' as c_char && t != b'%' as c_char && t != b'+' as c_char {
            /* Optimize the simple case */
            str_ = va_str(ap);

            if str_.is_null() {
                if optional == 0 {
                    set_error!(
                        s,
                        c"<args>".as_ptr(),
                        json_error_null_value,
                        c"NULL %s".as_ptr(),
                        purpose
                    );
                    (*s).has_error = 1;
                }
                return null_mut();
            }

            length = cffi::c_strlen(str_);

            if utf8_check_string(str_, length) == 0 {
                set_error!(
                    s,
                    c"<args>".as_ptr(),
                    json_error_invalid_utf8,
                    c"Invalid UTF-8 %s".as_ptr(),
                    purpose
                );
                (*s).has_error = 1;
                return null_mut();
            }

            *out_len = length;
            return str_ as *mut c_char;
        } else if optional != 0 {
            set_error!(
                s,
                c"<format>".as_ptr(),
                json_error_invalid_format,
                c"Cannot use '%c' on optional strings".as_ptr(),
                t as c_int
            );
            (*s).has_error = 1;

            return null_mut();
        }

        if strbuffer_init(addr_of_mut!(strbuff)) != 0 {
            set_error!(
                s,
                c"<internal>".as_ptr(),
                json_error_out_of_memory,
                c"Out of memory".as_ptr()
            );
            (*s).has_error = 1;
        }

        loop {
            str_ = va_str(ap);
            if str_.is_null() {
                set_error!(
                    s,
                    c"<args>".as_ptr(),
                    json_error_null_value,
                    c"NULL %s".as_ptr(),
                    purpose
                );
                (*s).has_error = 1;
            }

            next_token(s);

            if token(s) == b'#' as c_char {
                length = va_int(ap) as usize;
            } else if token(s) == b'%' as c_char {
                length = va_size(ap);
            } else {
                prev_token(s);
                length = if (*s).has_error != 0 {
                    0
                } else {
                    cffi::c_strlen(str_)
                };
            }

            if (*s).has_error == 0
                && strbuffer_append_bytes(addr_of_mut!(strbuff), str_, length) == -1
            {
                set_error!(
                    s,
                    c"<internal>".as_ptr(),
                    json_error_out_of_memory,
                    c"Out of memory".as_ptr()
                );
                (*s).has_error = 1;
            }

            next_token(s);
            if token(s) != b'+' as c_char {
                prev_token(s);
                break;
            }
        }

        if (*s).has_error != 0 {
            strbuffer_close(addr_of_mut!(strbuff));
            return null_mut();
        }

        if utf8_check_string(strbuff.value, strbuff.length) == 0 {
            set_error!(
                s,
                c"<args>".as_ptr(),
                json_error_invalid_utf8,
                c"Invalid UTF-8 %s".as_ptr(),
                purpose
            );
            strbuffer_close(addr_of_mut!(strbuff));
            (*s).has_error = 1;
            return null_mut();
        }

        *out_len = strbuff.length;
        *ours = 1;
        strbuffer_steal_value(addr_of_mut!(strbuff))
    }
}

unsafe fn pack_object(s: *mut scanner_t, ap: VaList) -> *mut json_t {
    unsafe {
        let object = json_object();
        next_token(s);

        'outer: {
            while token(s) != b'}' as c_char {
                let mut len: usize = 0;
                let mut ours: c_int = 0;

                if token(s) == 0 {
                    set_error!(
                        s,
                        c"<format>".as_ptr(),
                        json_error_invalid_format,
                        c"Unexpected end of format string".as_ptr()
                    );
                    break 'outer;
                }

                if token(s) != b's' as c_char {
                    set_error!(
                        s,
                        c"<format>".as_ptr(),
                        json_error_invalid_format,
                        c"Expected format 's', got '%c'".as_ptr(),
                        token(s) as c_int
                    );
                    break 'outer;
                }

                let key = read_string(s, ap, c"object key".as_ptr(), &mut len, &mut ours, 0);

                next_token(s);

                next_token(s);
                let value_optional = token(s);
                prev_token(s);

                let value = pack(s, ap);
                if value.is_null() {
                    if ours != 0 {
                        jsonp_free(key as *mut c_void);
                    }

                    if value_optional != b'*' as c_char {
                        set_error!(
                            s,
                            c"<args>".as_ptr(),
                            json_error_null_value,
                            c"NULL object value".as_ptr()
                        );
                        (*s).has_error = 1;
                    }

                    next_token(s);
                    continue;
                }

                if (*s).has_error != 0 {
                    json_decref(value);
                }

                if (*s).has_error == 0
                    && json_object_setn_new_nocheck(object, key, len, value) != 0
                {
                    set_error!(
                        s,
                        c"<internal>".as_ptr(),
                        json_error_out_of_memory,
                        c"Unable to add key \"%s\"".as_ptr(),
                        key
                    );
                    (*s).has_error = 1;
                }

                if ours != 0 {
                    jsonp_free(key as *mut c_void);
                }

                next_token(s);
            }

            if (*s).has_error == 0 {
                return object;
            }
        }

        json_decref(object);
        null_mut()
    }
}

unsafe fn pack_array(s: *mut scanner_t, ap: VaList) -> *mut json_t {
    unsafe {
        let array = json_array();
        next_token(s);

        'outer: {
            while token(s) != b']' as c_char {
                if token(s) == 0 {
                    set_error!(
                        s,
                        c"<format>".as_ptr(),
                        json_error_invalid_format,
                        c"Unexpected end of format string".as_ptr()
                    );
                    /* Format string errors are unrecoverable. */
                    break 'outer;
                }

                next_token(s);
                let value_optional = token(s);
                prev_token(s);

                let value = pack(s, ap);
                if value.is_null() {
                    if value_optional != b'*' as c_char {
                        (*s).has_error = 1;
                    }

                    next_token(s);
                    continue;
                }

                if (*s).has_error != 0 {
                    json_decref(value);
                }

                if (*s).has_error == 0 && json_array_append_new(array, value) != 0 {
                    set_error!(
                        s,
                        c"<internal>".as_ptr(),
                        json_error_out_of_memory,
                        c"Unable to append to array".as_ptr()
                    );
                    (*s).has_error = 1;
                }

                next_token(s);
            }

            if (*s).has_error == 0 {
                return array;
            }
        }

        json_decref(array);
        null_mut()
    }
}

unsafe fn pack_string(s: *mut scanner_t, ap: VaList) -> *mut json_t {
    unsafe {
        let mut len: usize = 0;
        let mut ours: c_int = 0;

        next_token(s);
        let t = token(s);
        let optional = (t == b'?' as c_char || t == b'*' as c_char) as c_int;
        if optional == 0 {
            prev_token(s);
        }

        let str_ = read_string(s, ap, c"string".as_ptr(), &mut len, &mut ours, optional);

        if str_.is_null() {
            return if t == b'?' as c_char && (*s).has_error == 0 {
                json_null()
            } else {
                null_mut()
            };
        }

        if (*s).has_error != 0 {
            /* It's impossible to reach this point if ours != 0, do not free str. */
            return null_mut();
        }

        if ours != 0 {
            return jsonp_stringn_nocheck_own(str_, len);
        }

        json_stringn_nocheck(str_, len)
    }
}

unsafe fn pack_object_inter(s: *mut scanner_t, ap: VaList, need_incref: c_int) -> *mut json_t {
    unsafe {
        next_token(s);
        let ntoken = token(s);

        if ntoken != b'?' as c_char && ntoken != b'*' as c_char {
            prev_token(s);
        }

        let json: *mut json_t = va_ptr::<json_t>(ap);

        if !json.is_null() {
            return if need_incref != 0 {
                json_incref(json)
            } else {
                json
            };
        }

        if ntoken == b'?' as c_char {
            return json_null();
        }
        if ntoken == b'*' as c_char {
            return null_mut();
        }

        set_error!(
            s,
            c"<args>".as_ptr(),
            json_error_null_value,
            c"NULL object".as_ptr()
        );
        (*s).has_error = 1;
        null_mut()
    }
}

unsafe fn pack_integer(s: *mut scanner_t, value: json_int_t) -> *mut json_t {
    unsafe {
        let json = json_integer(value);

        if json.is_null() {
            set_error!(
                s,
                c"<internal>".as_ptr(),
                json_error_out_of_memory,
                c"Out of memory".as_ptr()
            );
            (*s).has_error = 1;
        }

        json
    }
}

unsafe fn pack_real(s: *mut scanner_t, value: f64) -> *mut json_t {
    unsafe {
        /* Allocate without setting value so we can identify OOM error. */
        let json = json_real(0.0);

        if json.is_null() {
            set_error!(
                s,
                c"<internal>".as_ptr(),
                json_error_out_of_memory,
                c"Out of memory".as_ptr()
            );
            (*s).has_error = 1;

            return null_mut();
        }

        if json_real_set(json, value) != 0 {
            json_decref(json);

            set_error!(
                s,
                c"<args>".as_ptr(),
                json_error_numeric_overflow,
                c"Invalid floating point value".as_ptr()
            );
            (*s).has_error = 1;

            return null_mut();
        }

        json
    }
}

unsafe fn pack(s: *mut scanner_t, ap: VaList) -> *mut json_t {
    unsafe {
        match token(s) as u8 {
            b'{' => pack_object(s, ap),

            b'[' => pack_array(s, ap),

            b's' => pack_string(s, ap), /* string */

            b'n' => json_null(), /* null */

            b'b' => {
                /* boolean */
                if va_int(ap) != 0 {
                    json_true()
                } else {
                    json_false()
                }
            }

            b'i' => pack_integer(s, va_int(ap) as json_int_t), /* integer from int */

            b'I' => pack_integer(s, va_longlong(ap)), /* integer from json_int_t */

            b'f' => pack_real(s, va_double(ap)), /* real */

            b'O' => pack_object_inter(s, ap, 1), /* increments refcount */

            b'o' => pack_object_inter(s, ap, 0), /* doesn't increment refcount */

            _ => {
                set_error!(
                    s,
                    c"<format>".as_ptr(),
                    json_error_invalid_format,
                    c"Unexpected format character '%c'".as_ptr(),
                    token(s) as c_int
                );
                (*s).has_error = 1;
                null_mut()
            }
        }
    }
}

unsafe fn unpack_object(s: *mut scanner_t, root: *mut json_t, ap: VaList) -> c_int {
    unsafe {
        let mut ret: c_int = -1;
        let mut strict: c_int = 0;
        let mut gotopt: c_int = 0;

        /* Use a set (emulated by a hashtable) to check that all object
           keys are accessed. Checking that the correct number of keys
           were accessed is not enough, as the same key can be unpacked
           multiple times.
        */
        let mut key_set = hashtable_t::new();

        if hashtable_init(addr_of_mut!(key_set)) != 0 {
            set_error!(
                s,
                c"<internal>".as_ptr(),
                json_error_out_of_memory,
                c"Out of memory".as_ptr()
            );
            return -1;
        }

        'out: {
            if !root.is_null() && !json_is_object(root) {
                set_error!(
                    s,
                    c"<validation>".as_ptr(),
                    json_error_wrong_type,
                    c"Expected object, got %s".as_ptr(),
                    type_name(root)
                );
                break 'out;
            }
            next_token(s);

            while token(s) != b'}' as c_char {
                let mut opt: c_int = 0;

                if strict != 0 {
                    set_error!(
                        s,
                        c"<format>".as_ptr(),
                        json_error_invalid_format,
                        c"Expected '}' after '%c', got '%c'".as_ptr(),
                        (if strict == 1 { b'!' } else { b'*' }) as c_int,
                        token(s) as c_int
                    );
                    break 'out;
                }

                if token(s) == 0 {
                    set_error!(
                        s,
                        c"<format>".as_ptr(),
                        json_error_invalid_format,
                        c"Unexpected end of format string".as_ptr()
                    );
                    break 'out;
                }

                if token(s) == b'!' as c_char || token(s) == b'*' as c_char {
                    strict = if token(s) == b'!' as c_char { 1 } else { -1 };
                    next_token(s);
                    continue;
                }

                if token(s) != b's' as c_char {
                    set_error!(
                        s,
                        c"<format>".as_ptr(),
                        json_error_invalid_format,
                        c"Expected format 's', got '%c'".as_ptr(),
                        token(s) as c_int
                    );
                    break 'out;
                }

                let key = va_str(ap);
                if key.is_null() {
                    set_error!(
                        s,
                        c"<args>".as_ptr(),
                        json_error_null_value,
                        c"NULL object key".as_ptr()
                    );
                    break 'out;
                }
                let key_len = cffi::c_strlen(key);

                next_token(s);

                if token(s) == b'?' as c_char {
                    opt = 1;
                    gotopt = 1;
                    next_token(s);
                }

                let value: *mut json_t;
                if root.is_null() {
                    /* skipping */
                    value = null_mut();
                } else {
                    value = json_object_getn(root, key, key_len);
                    if value.is_null() && opt == 0 {
                        set_error!(
                            s,
                            c"<validation>".as_ptr(),
                            json_error_item_not_found,
                            c"Object item not found: %s".as_ptr(),
                            key
                        );
                        break 'out;
                    }
                }

                if unpack(s, value, ap) != 0 {
                    break 'out;
                }

                hashtable_set(addr_of_mut!(key_set), key, key_len, json_null());
                next_token(s);
            }

            if strict == 0 && ((*s).flags & JSON_STRICT) != 0 {
                strict = 1;
            }

            if !root.is_null() && strict == 1 {
                /* We need to check that all non optional items have been parsed */
                /* keys_res is 1 for uninitialized, 0 for success, -1 for error. */
                let mut keys_res: c_int = 1;
                let mut unrecognized_keys = strbuffer_t::new();
                let mut unpacked: c_long = 0;

                if gotopt != 0 || json_object_size(root) != key_set.size {
                    let mut key = json_object_iter_key(json_object_iter(root));
                    let mut key_len = json_object_iter_key_len(json_object_key_to_iter(key));
                    while !key.is_null() {
                        let value = json_object_iter_value(json_object_key_to_iter(key));
                        if value.is_null() {
                            break;
                        }

                        if hashtable_get(addr_of_mut!(key_set), key, key_len).is_null() {
                            unpacked += 1;

                            /* Save unrecognized keys for the error message */
                            if keys_res == 1 {
                                keys_res = strbuffer_init(addr_of_mut!(unrecognized_keys));
                            } else if keys_res == 0 {
                                keys_res = strbuffer_append_bytes(
                                    addr_of_mut!(unrecognized_keys),
                                    c", ".as_ptr(),
                                    2,
                                );
                            }

                            if keys_res == 0 {
                                keys_res = strbuffer_append_bytes(
                                    addr_of_mut!(unrecognized_keys),
                                    key,
                                    key_len,
                                );
                            }
                        }

                        key = json_object_iter_key(json_object_iter_next(
                            root,
                            json_object_key_to_iter(key),
                        ));
                        key_len = json_object_iter_key_len(json_object_key_to_iter(key));
                    }
                }
                if unpacked != 0 {
                    let listing = if keys_res != 0 {
                        c"<unknown>".as_ptr()
                    } else {
                        strbuffer_value(addr_of_mut!(unrecognized_keys))
                    };
                    set_error!(
                        s,
                        c"<validation>".as_ptr(),
                        json_error_end_of_input_expected,
                        c"%li object item(s) left unpacked: %s".as_ptr(),
                        unpacked,
                        listing
                    );
                    strbuffer_close(addr_of_mut!(unrecognized_keys));
                    break 'out;
                }
            }

            ret = 0;
        }

        hashtable_close(addr_of_mut!(key_set));
        ret
    }
}

unsafe fn unpack_array(s: *mut scanner_t, root: *mut json_t, ap: VaList) -> c_int {
    unsafe {
        let mut i: usize = 0;
        let mut strict: c_int = 0;

        if !root.is_null() && !json_is_array(root) {
            set_error!(
                s,
                c"<validation>".as_ptr(),
                json_error_wrong_type,
                c"Expected array, got %s".as_ptr(),
                type_name(root)
            );
            return -1;
        }
        next_token(s);

        while token(s) != b']' as c_char {
            if strict != 0 {
                set_error!(
                    s,
                    c"<format>".as_ptr(),
                    json_error_invalid_format,
                    c"Expected ']' after '%c', got '%c'".as_ptr(),
                    (if strict == 1 { b'!' } else { b'*' }) as c_int,
                    token(s) as c_int
                );
                return -1;
            }

            if token(s) == 0 {
                set_error!(
                    s,
                    c"<format>".as_ptr(),
                    json_error_invalid_format,
                    c"Unexpected end of format string".as_ptr()
                );
                return -1;
            }

            if token(s) == b'!' as c_char || token(s) == b'*' as c_char {
                strict = if token(s) == b'!' as c_char { 1 } else { -1 };
                next_token(s);
                continue;
            }

            if cffi::c_strchr(UNPACK_VALUE_STARTERS.as_ptr() as *const c_char, token(s) as u8)
                .is_null()
            {
                set_error!(
                    s,
                    c"<format>".as_ptr(),
                    json_error_invalid_format,
                    c"Unexpected format character '%c'".as_ptr(),
                    token(s) as c_int
                );
                return -1;
            }

            let value: *mut json_t;
            if root.is_null() {
                /* skipping */
                value = null_mut();
            } else {
                value = json_array_get(root, i);
                if value.is_null() {
                    set_error!(
                        s,
                        c"<validation>".as_ptr(),
                        json_error_index_out_of_range,
                        c"Array index %lu out of range".as_ptr(),
                        i as c_ulong
                    );
                    return -1;
                }
            }

            if unpack(s, value, ap) != 0 {
                return -1;
            }

            next_token(s);
            i += 1;
        }

        if strict == 0 && ((*s).flags & JSON_STRICT) != 0 {
            strict = 1;
        }

        if !root.is_null() && strict == 1 && i != json_array_size(root) {
            let diff = json_array_size(root) as c_long - i as c_long;
            set_error!(
                s,
                c"<validation>".as_ptr(),
                json_error_end_of_input_expected,
                c"%li array item(s) left unpacked".as_ptr(),
                diff
            );
            return -1;
        }

        0
    }
}

unsafe fn unpack(s: *mut scanner_t, root: *mut json_t, ap: VaList) -> c_int {
    unsafe {
        match token(s) as u8 {
            b'{' => unpack_object(s, root, ap),

            b'[' => unpack_array(s, root, ap),

            b's' => {
                if !root.is_null() && !json_is_string(root) {
                    set_error!(
                        s,
                        c"<validation>".as_ptr(),
                        json_error_wrong_type,
                        c"Expected string, got %s".as_ptr(),
                        type_name(root)
                    );
                    return -1;
                }

                if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                    let mut len_target: *mut usize = null_mut();

                    let str_target = va_ptr::<*const c_char>(ap);
                    if str_target.is_null() {
                        set_error!(
                            s,
                            c"<args>".as_ptr(),
                            json_error_null_value,
                            c"NULL string argument".as_ptr()
                        );
                        return -1;
                    }

                    next_token(s);

                    if token(s) == b'%' as c_char {
                        len_target = va_ptr::<usize>(ap);
                        if len_target.is_null() {
                            set_error!(
                                s,
                                c"<args>".as_ptr(),
                                json_error_null_value,
                                c"NULL string length argument".as_ptr()
                            );
                            return -1;
                        }
                    } else {
                        prev_token(s);
                    }

                    if !root.is_null() {
                        *str_target = json_string_value(root);
                        if !len_target.is_null() {
                            *len_target = json_string_length(root);
                        }
                    }
                }
                0
            }

            b'i' => {
                if !root.is_null() && !json_is_integer(root) {
                    set_error!(
                        s,
                        c"<validation>".as_ptr(),
                        json_error_wrong_type,
                        c"Expected integer, got %s".as_ptr(),
                        type_name(root)
                    );
                    return -1;
                }

                if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                    let target = va_ptr::<c_int>(ap);
                    if !root.is_null() {
                        *target = json_integer_value(root) as c_int;
                    }
                }

                0
            }

            b'I' => {
                if !root.is_null() && !json_is_integer(root) {
                    set_error!(
                        s,
                        c"<validation>".as_ptr(),
                        json_error_wrong_type,
                        c"Expected integer, got %s".as_ptr(),
                        type_name(root)
                    );
                    return -1;
                }

                if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                    let target = va_ptr::<json_int_t>(ap);
                    if !root.is_null() {
                        *target = json_integer_value(root);
                    }
                }

                0
            }

            b'b' => {
                if !root.is_null() && !json_is_boolean(root) {
                    set_error!(
                        s,
                        c"<validation>".as_ptr(),
                        json_error_wrong_type,
                        c"Expected true or false, got %s".as_ptr(),
                        type_name(root)
                    );
                    return -1;
                }

                if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                    let target = va_ptr::<c_int>(ap);
                    if !root.is_null() {
                        *target = json_is_true(root) as c_int;
                    }
                }

                0
            }

            b'f' => {
                if !root.is_null() && !json_is_real(root) {
                    set_error!(
                        s,
                        c"<validation>".as_ptr(),
                        json_error_wrong_type,
                        c"Expected real, got %s".as_ptr(),
                        type_name(root)
                    );
                    return -1;
                }

                if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                    let target = va_ptr::<f64>(ap);
                    if !root.is_null() {
                        *target = json_real_value(root);
                    }
                }

                0
            }

            b'F' => {
                if !root.is_null() && !json_is_number(root) {
                    set_error!(
                        s,
                        c"<validation>".as_ptr(),
                        json_error_wrong_type,
                        c"Expected real or integer, got %s".as_ptr(),
                        type_name(root)
                    );
                    return -1;
                }

                if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                    let target = va_ptr::<f64>(ap);
                    if !root.is_null() {
                        *target = json_number_value(root);
                    }
                }

                0
            }

            b'O' | b'o' => {
                if token(s) as u8 == b'O' {
                    if !root.is_null() && ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                        json_incref(root);
                    }
                    /* Fall through */
                }

                if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                    let target = va_ptr::<*mut json_t>(ap);
                    if !root.is_null() {
                        *target = root;
                    }
                }

                0
            }

            b'n' => {
                /* Never assign, just validate */
                if !root.is_null() && !json_is_null(root) {
                    set_error!(
                        s,
                        c"<validation>".as_ptr(),
                        json_error_wrong_type,
                        c"Expected null, got %s".as_ptr(),
                        type_name(root)
                    );
                    return -1;
                }
                0
            }

            _ => {
                set_error!(
                    s,
                    c"<format>".as_ptr(),
                    json_error_invalid_format,
                    c"Unexpected format character '%c'".as_ptr(),
                    token(s) as c_int
                );
                -1
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_vpack_ex(
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
    ap: VaList,
) -> *mut json_t {
    unsafe {
        let mut s = scanner_t::zero();

        if fmt.is_null() || *fmt == 0 {
            jsonp_error_init(error, c"<format>".as_ptr());
            jsonp_error_set_str(
                error,
                -1,
                -1,
                0,
                json_error_invalid_argument,
                c"NULL or empty format string".as_ptr(),
            );
            return null_mut();
        }
        jsonp_error_init(error, core::ptr::null());

        scanner_init(addr_of_mut!(s), error, flags, fmt);
        next_token(addr_of_mut!(s));

        let mut ap_copy = VaListTag {
            gp_offset: 0,
            fp_offset: 0,
            overflow_arg_area: null_mut(),
            reg_save_area: null_mut(),
        };
        va_copy(&mut ap_copy, ap);
        let value = pack(addr_of_mut!(s), &mut ap_copy);

        /* This will cover all situations where s.has_error is true */
        if value.is_null() {
            return null_mut();
        }

        next_token(addr_of_mut!(s));
        if token(addr_of_mut!(s)) != 0 {
            json_decref(value);
            set_error!(
                addr_of_mut!(s),
                c"<format>".as_ptr(),
                json_error_invalid_format,
                c"Garbage after format string".as_ptr()
            );
            return null_mut();
        }

        value
    }
}

/* json_pack_ex() and json_pack() are provided by the assembly trampolines. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_vunpack_ex(
    root: *mut json_t,
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
    ap: VaList,
) -> c_int {
    unsafe {
        let mut s = scanner_t::zero();

        if root.is_null() {
            jsonp_error_init(error, c"<root>".as_ptr());
            jsonp_error_set_str(
                error,
                -1,
                -1,
                0,
                json_error_null_value,
                c"NULL root value".as_ptr(),
            );
            return -1;
        }

        if fmt.is_null() || *fmt == 0 {
            jsonp_error_init(error, c"<format>".as_ptr());
            jsonp_error_set_str(
                error,
                -1,
                -1,
                0,
                json_error_invalid_argument,
                c"NULL or empty format string".as_ptr(),
            );
            return -1;
        }
        jsonp_error_init(error, core::ptr::null());

        scanner_init(addr_of_mut!(s), error, flags, fmt);
        next_token(addr_of_mut!(s));

        let mut ap_copy = VaListTag {
            gp_offset: 0,
            fp_offset: 0,
            overflow_arg_area: null_mut(),
            reg_save_area: null_mut(),
        };
        va_copy(&mut ap_copy, ap);
        if unpack(addr_of_mut!(s), root, &mut ap_copy) != 0 {
            return -1;
        }

        next_token(addr_of_mut!(s));
        if token(addr_of_mut!(s)) != 0 {
            set_error!(
                addr_of_mut!(s),
                c"<format>".as_ptr(),
                json_error_invalid_format,
                c"Garbage after format string".as_ptr()
            );
            return -1;
        }

        0
    }
}

/* json_unpack_ex() and json_unpack() are provided by the assembly trampolines. */
