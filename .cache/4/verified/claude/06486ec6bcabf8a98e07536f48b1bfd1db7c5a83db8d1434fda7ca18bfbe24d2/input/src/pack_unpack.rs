//! Translation of c_src/src/pack_unpack.c
#![allow(dead_code)]

use crate::error::{jsonp_error_init, jsonp_error_set_1s, jsonp_error_set_source};
use crate::hashtable::{hashtable_close, hashtable_get, hashtable_init, hashtable_set, hashtable_t};
use crate::jansson::*;
use crate::libc;
use crate::libc::{va_arg_fp, va_arg_gp, va_list, VaListTag};
use crate::memory::jsonp_free;
use crate::strbuffer::*;
use crate::utf::utf8_check_string;
use crate::value::*;
use std::ffi::{c_char, c_int, c_void};

#[repr(C)]
#[derive(Copy, Clone)]
struct token_t {
    line: c_int,
    column: c_int,
    pos: usize,
    token: c_char,
}

impl token_t {
    const fn zero() -> token_t {
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

#[inline]
unsafe fn token(s: *const scanner_t) -> c_char {
    (*s).token.token
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
    TYPE_NAMES[json_typeof(x) as usize].as_ptr() as *const c_char
}

static UNPACK_VALUE_STARTERS: &[u8; 12] = b"{[siIbfFOon\0";

/// Reproduces the json_object_keylen_foreach() macro from jansson.h.
macro_rules! json_object_keylen_foreach {
    ($object:expr, $key:ident, $key_len:ident, $value:ident, $body:block) => {{
        let __obj = $object;
        #[allow(unused_mut)]
        let mut $key = json_object_iter_key(json_object_iter(__obj));
        #[allow(unused_mut)]
        let mut $key_len = json_object_iter_key_len(json_object_key_to_iter($key));
        #[allow(unused_mut, unused_assignments)]
        let mut $value: *mut json_t = std::ptr::null_mut();
        while !$key.is_null()
            && {
                $value = json_object_iter_value(json_object_key_to_iter($key));
                !$value.is_null()
            }
        {
            $body
            $key = json_object_iter_key(json_object_iter_next(
                __obj,
                json_object_key_to_iter($key),
            ));
            $key_len = json_object_iter_key_len(json_object_key_to_iter($key));
        }
    }};
}

unsafe fn scanner_init(
    s: *mut scanner_t,
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
) {
    (*s).error = error;
    (*s).flags = flags;
    (*s).start = fmt;
    (*s).fmt = fmt;
    (*s).prev_token = token_t::zero();
    (*s).token = token_t::zero();
    (*s).next_token = token_t::zero();
    (*s).line = 1;
    (*s).column = 0;
    (*s).pos = 0;
    (*s).has_error = 0;
}

unsafe fn next_token(s: *mut scanner_t) {
    let mut t: *const c_char;
    (*s).prev_token = (*s).token;

    if (*s).next_token.line != 0 {
        (*s).token = (*s).next_token;
        (*s).next_token.line = 0;
        return;
    }

    if token(s) == 0 && *(*s).fmt == 0 {
        return;
    }

    t = (*s).fmt;
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

unsafe fn prev_token(s: *mut scanner_t) {
    (*s).next_token = (*s).token;
    (*s).token = (*s).prev_token;
}

/* set_error(): formats the message exactly like vsnprintf() into the error
   struct, then sets the source. */
macro_rules! set_error {
    ($s:expr, $source:expr, $code:expr, $fmt:expr $(, $arg:expr)*) => {{
        let __s = $s;
        let mut __buf: [c_char; 512] = [0; 512];
        libc::snprintf(
            __buf.as_mut_ptr(),
            512,
            $fmt.as_ptr() as *const c_char
            $(, $arg)*
        );
        jsonp_error_set_1s(
            (*__s).error,
            (*__s).token.line,
            (*__s).token.column,
            (*__s).token.pos,
            $code,
            __buf.as_ptr(),
        );
        jsonp_error_set_source((*__s).error, $source.as_ptr() as *const c_char);
    }};
}

/* ours will be set to 1 if jsonp_free() must be called for the result
afterwards */
unsafe fn read_string(
    s: *mut scanner_t,
    ap: va_list,
    purpose: *const c_char,
    out_len: *mut usize,
    ours: *mut c_int,
    optional: c_int,
) -> *mut c_char {
    let t: c_char;
    let mut strbuff = strbuffer_t::new();
    let mut str_: *const c_char;
    let mut length: usize;

    next_token(s);
    t = token(s);
    prev_token(s);

    *ours = 0;
    if t != b'#' as c_char && t != b'%' as c_char && t != b'+' as c_char {
        /* Optimize the simple case */
        str_ = va_arg_gp::<*const c_char>(ap);

        if str_.is_null() {
            if optional == 0 {
                set_error!(
                    s,
                    b"<args>\0",
                    json_error_null_value,
                    b"NULL %s\0",
                    purpose
                );
                (*s).has_error = 1;
            }
            return std::ptr::null_mut();
        }

        length = libc::strlen(str_);

        if utf8_check_string(str_, length) == 0 {
            set_error!(
                s,
                b"<args>\0",
                json_error_invalid_utf8,
                b"Invalid UTF-8 %s\0",
                purpose
            );
            (*s).has_error = 1;
            return std::ptr::null_mut();
        }

        *out_len = length;
        return str_ as *mut c_char;
    } else if optional != 0 {
        set_error!(
            s,
            b"<format>\0",
            json_error_invalid_format,
            b"Cannot use '%c' on optional strings\0",
            t as c_int
        );
        (*s).has_error = 1;

        return std::ptr::null_mut();
    }

    if strbuffer_init(&mut strbuff) != 0 {
        set_error!(
            s,
            b"<internal>\0",
            json_error_out_of_memory,
            b"Out of memory\0"
        );
        (*s).has_error = 1;
    }

    loop {
        str_ = va_arg_gp::<*const c_char>(ap);
        if str_.is_null() {
            set_error!(
                s,
                b"<args>\0",
                json_error_null_value,
                b"NULL %s\0",
                purpose
            );
            (*s).has_error = 1;
        }

        next_token(s);

        if token(s) == b'#' as c_char {
            length = va_arg_gp::<c_int>(ap) as usize;
        } else if token(s) == b'%' as c_char {
            length = va_arg_gp::<usize>(ap);
        } else {
            prev_token(s);
            length = if (*s).has_error != 0 {
                0
            } else {
                libc::strlen(str_)
            };
        }

        if (*s).has_error == 0 && strbuffer_append_bytes(&mut strbuff, str_, length) == -1 {
            set_error!(
                s,
                b"<internal>\0",
                json_error_out_of_memory,
                b"Out of memory\0"
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
        strbuffer_close(&mut strbuff);
        return std::ptr::null_mut();
    }

    if utf8_check_string(strbuff.value, strbuff.length) == 0 {
        set_error!(
            s,
            b"<args>\0",
            json_error_invalid_utf8,
            b"Invalid UTF-8 %s\0",
            purpose
        );
        strbuffer_close(&mut strbuff);
        (*s).has_error = 1;
        return std::ptr::null_mut();
    }

    *out_len = strbuff.length;
    *ours = 1;
    strbuffer_steal_value(&mut strbuff)
}

unsafe fn pack_object(s: *mut scanner_t, ap: va_list) -> *mut json_t {
    let object = json_object();
    next_token(s);

    while token(s) != b'}' as c_char {
        let key: *mut c_char;
        let mut len: usize = 0;
        let mut ours: c_int = 0;
        let value: *mut json_t;
        let value_optional: c_char;

        if token(s) == 0 {
            set_error!(
                s,
                b"<format>\0",
                json_error_invalid_format,
                b"Unexpected end of format string\0"
            );
            json_decref(object);
            return std::ptr::null_mut();
        }

        if token(s) != b's' as c_char {
            set_error!(
                s,
                b"<format>\0",
                json_error_invalid_format,
                b"Expected format 's', got '%c'\0",
                token(s) as c_int
            );
            json_decref(object);
            return std::ptr::null_mut();
        }

        key = read_string(
            s,
            ap,
            b"object key\0".as_ptr() as *const c_char,
            &mut len,
            &mut ours,
            0,
        );

        next_token(s);

        next_token(s);
        value_optional = token(s);
        prev_token(s);

        value = pack(s, ap);
        if value.is_null() {
            if ours != 0 {
                jsonp_free(key as *mut c_void);
            }

            if value_optional != b'*' as c_char {
                set_error!(
                    s,
                    b"<args>\0",
                    json_error_null_value,
                    b"NULL object value\0"
                );
                (*s).has_error = 1;
            }

            next_token(s);
            continue;
        }

        if (*s).has_error != 0 {
            json_decref(value);
        }

        if (*s).has_error == 0 && json_object_setn_new_nocheck(object, key, len, value) != 0 {
            set_error!(
                s,
                b"<internal>\0",
                json_error_out_of_memory,
                b"Unable to add key \"%s\"\0",
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

    json_decref(object);
    std::ptr::null_mut()
}

unsafe fn pack_array(s: *mut scanner_t, ap: va_list) -> *mut json_t {
    let array = json_array();
    next_token(s);

    while token(s) != b']' as c_char {
        let value: *mut json_t;
        let value_optional: c_char;

        if token(s) == 0 {
            set_error!(
                s,
                b"<format>\0",
                json_error_invalid_format,
                b"Unexpected end of format string\0"
            );
            /* Format string errors are unrecoverable. */
            json_decref(array);
            return std::ptr::null_mut();
        }

        next_token(s);
        value_optional = token(s);
        prev_token(s);

        value = pack(s, ap);
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
                b"<internal>\0",
                json_error_out_of_memory,
                b"Unable to append to array\0"
            );
            (*s).has_error = 1;
        }

        next_token(s);
    }

    if (*s).has_error == 0 {
        return array;
    }

    json_decref(array);
    std::ptr::null_mut()
}

unsafe fn pack_string(s: *mut scanner_t, ap: va_list) -> *mut json_t {
    let str_: *mut c_char;
    let t: c_char;
    let mut len: usize = 0;
    let mut ours: c_int = 0;
    let optional: c_int;

    next_token(s);
    t = token(s);
    optional = (t == b'?' as c_char || t == b'*' as c_char) as c_int;
    if optional == 0 {
        prev_token(s);
    }

    str_ = read_string(
        s,
        ap,
        b"string\0".as_ptr() as *const c_char,
        &mut len,
        &mut ours,
        optional,
    );

    if str_.is_null() {
        return if t == b'?' as c_char && (*s).has_error == 0 {
            json_null()
        } else {
            std::ptr::null_mut()
        };
    }

    if (*s).has_error != 0 {
        /* It's impossible to reach this point if ours != 0, do not free str. */
        return std::ptr::null_mut();
    }

    if ours != 0 {
        return jsonp_stringn_nocheck_own(str_, len);
    }

    json_stringn_nocheck(str_, len)
}

unsafe fn pack_object_inter(s: *mut scanner_t, ap: va_list, need_incref: c_int) -> *mut json_t {
    let json: *mut json_t;
    let ntoken: c_char;

    next_token(s);
    ntoken = token(s);

    if ntoken != b'?' as c_char && ntoken != b'*' as c_char {
        prev_token(s);
    }

    json = va_arg_gp::<*mut json_t>(ap);

    if !json.is_null() {
        return if need_incref != 0 {
            json_incref(json)
        } else {
            json
        };
    }

    match ntoken as u8 {
        b'?' => return json_null(),
        b'*' => return std::ptr::null_mut(),
        _ => (),
    }

    set_error!(s, b"<args>\0", json_error_null_value, b"NULL object\0");
    (*s).has_error = 1;
    std::ptr::null_mut()
}

unsafe fn pack_integer(s: *mut scanner_t, value: json_int_t) -> *mut json_t {
    let json = json_integer(value);

    if json.is_null() {
        set_error!(
            s,
            b"<internal>\0",
            json_error_out_of_memory,
            b"Out of memory\0"
        );
        (*s).has_error = 1;
    }

    json
}

unsafe fn pack_real(s: *mut scanner_t, value: f64) -> *mut json_t {
    /* Allocate without setting value so we can identify OOM error. */
    let json = json_real(0.0);

    if json.is_null() {
        set_error!(
            s,
            b"<internal>\0",
            json_error_out_of_memory,
            b"Out of memory\0"
        );
        (*s).has_error = 1;

        return std::ptr::null_mut();
    }

    if json_real_set(json, value) != 0 {
        json_decref(json);

        set_error!(
            s,
            b"<args>\0",
            json_error_numeric_overflow,
            b"Invalid floating point value\0"
        );
        (*s).has_error = 1;

        return std::ptr::null_mut();
    }

    json
}

unsafe fn pack(s: *mut scanner_t, ap: va_list) -> *mut json_t {
    match token(s) as u8 {
        b'{' => pack_object(s, ap),

        b'[' => pack_array(s, ap),

        b's' => pack_string(s, ap),

        b'n' => json_null(),

        b'b' => {
            if va_arg_gp::<c_int>(ap) != 0 {
                json_true()
            } else {
                json_false()
            }
        }

        b'i' => {
            let v = va_arg_gp::<c_int>(ap);
            pack_integer(s, v as json_int_t)
        }

        b'I' => {
            let v = va_arg_gp::<json_int_t>(ap);
            pack_integer(s, v)
        }

        b'f' => {
            let v = va_arg_fp(ap);
            pack_real(s, v)
        }

        b'O' => pack_object_inter(s, ap, 1),

        b'o' => pack_object_inter(s, ap, 0),

        _ => {
            set_error!(
                s,
                b"<format>\0",
                json_error_invalid_format,
                b"Unexpected format character '%c'\0",
                token(s) as c_int
            );
            (*s).has_error = 1;
            std::ptr::null_mut()
        }
    }
}

unsafe fn unpack_object(s: *mut scanner_t, root: *mut json_t, ap: va_list) -> c_int {
    let mut ret: c_int = -1;
    let mut strict: c_int = 0;
    let mut gotopt: c_int = 0;

    /* Use a set (emulated by a hashtable) to check that all object
    keys are accessed. Checking that the correct number of keys
    were accessed is not enough, as the same key can be unpacked
    multiple times.
    */
    let mut key_set = hashtable_t::new();

    if hashtable_init(&mut key_set) != 0 {
        set_error!(
            s,
            b"<internal>\0",
            json_error_out_of_memory,
            b"Out of memory\0"
        );
        return -1;
    }

    'out: {
        if !root.is_null() && !json_is_object(root) {
            set_error!(
                s,
                b"<validation>\0",
                json_error_wrong_type,
                b"Expected object, got %s\0",
                type_name(root)
            );
            break 'out;
        }
        next_token(s);

        while token(s) != b'}' as c_char {
            let key: *const c_char;
            let key_len: usize;
            let value: *mut json_t;
            let mut opt: c_int = 0;

            if strict != 0 {
                set_error!(
                    s,
                    b"<format>\0",
                    json_error_invalid_format,
                    b"Expected '}' after '%c', got '%c'\0",
                    (if strict == 1 { b'!' } else { b'*' }) as c_int,
                    token(s) as c_int
                );
                break 'out;
            }

            if token(s) == 0 {
                set_error!(
                    s,
                    b"<format>\0",
                    json_error_invalid_format,
                    b"Unexpected end of format string\0"
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
                    b"<format>\0",
                    json_error_invalid_format,
                    b"Expected format 's', got '%c'\0",
                    token(s) as c_int
                );
                break 'out;
            }

            key = va_arg_gp::<*const c_char>(ap);
            if key.is_null() {
                set_error!(
                    s,
                    b"<args>\0",
                    json_error_null_value,
                    b"NULL object key\0"
                );
                break 'out;
            }
            key_len = libc::strlen(key);

            next_token(s);

            if token(s) == b'?' as c_char {
                opt = 1;
                gotopt = 1;
                next_token(s);
            }

            if root.is_null() {
                /* skipping */
                value = std::ptr::null_mut();
            } else {
                value = json_object_getn(root, key, key_len);
                if value.is_null() && opt == 0 {
                    set_error!(
                        s,
                        b"<validation>\0",
                        json_error_item_not_found,
                        b"Object item not found: %s\0",
                        key
                    );
                    break 'out;
                }
            }

            if unpack(s, value, ap) != 0 {
                break 'out;
            }

            hashtable_set(&mut key_set, key, key_len, json_null());
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
            let mut unpacked: i64 = 0;

            if gotopt != 0 || json_object_size(root) != key_set.size {
                json_object_keylen_foreach!(root, key, key_len, value, {
                    if hashtable_get(&mut key_set, key, key_len).is_null() {
                        unpacked += 1;

                        /* Save unrecognized keys for the error message */
                        if keys_res == 1 {
                            keys_res = strbuffer_init(&mut unrecognized_keys);
                        } else if keys_res == 0 {
                            keys_res = strbuffer_append_bytes(
                                &mut unrecognized_keys,
                                b", \0".as_ptr() as *const c_char,
                                2,
                            );
                        }

                        if keys_res == 0 {
                            keys_res =
                                strbuffer_append_bytes(&mut unrecognized_keys, key, key_len);
                        }
                    }
                });
            }
            if unpacked != 0 {
                set_error!(
                    s,
                    b"<validation>\0",
                    json_error_end_of_input_expected,
                    b"%li object item(s) left unpacked: %s\0",
                    unpacked,
                    if keys_res != 0 {
                        b"<unknown>\0".as_ptr() as *const c_char
                    } else {
                        strbuffer_value(&unrecognized_keys)
                    }
                );
                strbuffer_close(&mut unrecognized_keys);
                break 'out;
            }
        }

        ret = 0;
    }

    hashtable_close(&mut key_set);
    ret
}

unsafe fn unpack_array(s: *mut scanner_t, root: *mut json_t, ap: va_list) -> c_int {
    let mut i: usize = 0;
    let mut strict: c_int = 0;

    if !root.is_null() && !json_is_array(root) {
        set_error!(
            s,
            b"<validation>\0",
            json_error_wrong_type,
            b"Expected array, got %s\0",
            type_name(root)
        );
        return -1;
    }
    next_token(s);

    while token(s) != b']' as c_char {
        let value: *mut json_t;

        if strict != 0 {
            set_error!(
                s,
                b"<format>\0",
                json_error_invalid_format,
                b"Expected ']' after '%c', got '%c'\0",
                (if strict == 1 { b'!' } else { b'*' }) as c_int,
                token(s) as c_int
            );
            return -1;
        }

        if token(s) == 0 {
            set_error!(
                s,
                b"<format>\0",
                json_error_invalid_format,
                b"Unexpected end of format string\0"
            );
            return -1;
        }

        if token(s) == b'!' as c_char || token(s) == b'*' as c_char {
            strict = if token(s) == b'!' as c_char { 1 } else { -1 };
            next_token(s);
            continue;
        }

        if libc::strchr(
            UNPACK_VALUE_STARTERS.as_ptr() as *const c_char,
            token(s) as c_int,
        )
        .is_null()
        {
            set_error!(
                s,
                b"<format>\0",
                json_error_invalid_format,
                b"Unexpected format character '%c'\0",
                token(s) as c_int
            );
            return -1;
        }

        if root.is_null() {
            /* skipping */
            value = std::ptr::null_mut();
        } else {
            value = json_array_get(root, i);
            if value.is_null() {
                set_error!(
                    s,
                    b"<validation>\0",
                    json_error_index_out_of_range,
                    b"Array index %lu out of range\0",
                    i as u64
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
        let diff: i64 = (json_array_size(root) as i64) - (i as i64);
        set_error!(
            s,
            b"<validation>\0",
            json_error_end_of_input_expected,
            b"%li array item(s) left unpacked\0",
            diff
        );
        return -1;
    }

    0
}

unsafe fn unpack(s: *mut scanner_t, root: *mut json_t, ap: va_list) -> c_int {
    match token(s) as u8 {
        b'{' => unpack_object(s, root, ap),

        b'[' => unpack_array(s, root, ap),

        b's' => {
            if !root.is_null() && !json_is_string(root) {
                set_error!(
                    s,
                    b"<validation>\0",
                    json_error_wrong_type,
                    b"Expected string, got %s\0",
                    type_name(root)
                );
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let str_target: *mut *const c_char;
                let mut len_target: *mut usize = std::ptr::null_mut();

                str_target = va_arg_gp::<*mut *const c_char>(ap);
                if str_target.is_null() {
                    set_error!(
                        s,
                        b"<args>\0",
                        json_error_null_value,
                        b"NULL string argument\0"
                    );
                    return -1;
                }

                next_token(s);

                if token(s) == b'%' as c_char {
                    len_target = va_arg_gp::<*mut usize>(ap);
                    if len_target.is_null() {
                        set_error!(
                            s,
                            b"<args>\0",
                            json_error_null_value,
                            b"NULL string length argument\0"
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
                    b"<validation>\0",
                    json_error_wrong_type,
                    b"Expected integer, got %s\0",
                    type_name(root)
                );
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = va_arg_gp::<*mut c_int>(ap);
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
                    b"<validation>\0",
                    json_error_wrong_type,
                    b"Expected integer, got %s\0",
                    type_name(root)
                );
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = va_arg_gp::<*mut json_int_t>(ap);
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
                    b"<validation>\0",
                    json_error_wrong_type,
                    b"Expected true or false, got %s\0",
                    type_name(root)
                );
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = va_arg_gp::<*mut c_int>(ap);
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
                    b"<validation>\0",
                    json_error_wrong_type,
                    b"Expected real, got %s\0",
                    type_name(root)
                );
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = va_arg_gp::<*mut f64>(ap);
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
                    b"<validation>\0",
                    json_error_wrong_type,
                    b"Expected real or integer, got %s\0",
                    type_name(root)
                );
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = va_arg_gp::<*mut f64>(ap);
                if !root.is_null() {
                    *target = json_number_value(root);
                }
            }

            0
        }

        b'O' | b'o' => {
            if token(s) == b'O' as c_char
                && !root.is_null()
                && ((*s).flags & JSON_VALIDATE_ONLY) == 0
            {
                json_incref(root);
            }
            /* Fall through */

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = va_arg_gp::<*mut *mut json_t>(ap);
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
                    b"<validation>\0",
                    json_error_wrong_type,
                    b"Expected null, got %s\0",
                    type_name(root)
                );
                return -1;
            }
            0
        }

        _ => {
            set_error!(
                s,
                b"<format>\0",
                json_error_invalid_format,
                b"Unexpected format character '%c'\0",
                token(s) as c_int
            );
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_vpack_ex(
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
    ap: va_list,
) -> *mut json_t {
    let mut s = scanner_t {
        start: std::ptr::null(),
        fmt: std::ptr::null(),
        prev_token: token_t::zero(),
        token: token_t::zero(),
        next_token: token_t::zero(),
        error: std::ptr::null_mut(),
        flags: 0,
        line: 0,
        column: 0,
        pos: 0,
        has_error: 0,
    };
    let mut ap_copy_store = VaListTag {
        gp_offset: 0,
        fp_offset: 0,
        overflow_arg_area: std::ptr::null_mut(),
        reg_save_area: std::ptr::null_mut(),
    };
    let value: *mut json_t;

    if fmt.is_null() || *fmt == 0 {
        jsonp_error_init(error, b"<format>\0".as_ptr() as *const c_char);
        jsonp_error_set_1s(
            error,
            -1,
            -1,
            0,
            json_error_invalid_argument,
            b"NULL or empty format string\0".as_ptr() as *const c_char,
        );
        return std::ptr::null_mut();
    }
    jsonp_error_init(error, std::ptr::null());

    scanner_init(&mut s, error, flags, fmt);
    next_token(&mut s);

    let ap_copy = crate::libc::va_copy(ap, &mut ap_copy_store);
    value = pack(&mut s, ap_copy);

    /* This will cover all situations where s.has_error is true */
    if value.is_null() {
        return std::ptr::null_mut();
    }

    next_token(&mut s);
    if token(&s) != 0 {
        json_decref(value);
        set_error!(
            &mut s as *mut scanner_t,
            b"<format>\0",
            json_error_invalid_format,
            b"Garbage after format string\0"
        );
        return std::ptr::null_mut();
    }

    value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_vunpack_ex(
    root: *mut json_t,
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
    ap: va_list,
) -> c_int {
    let mut s = scanner_t {
        start: std::ptr::null(),
        fmt: std::ptr::null(),
        prev_token: token_t::zero(),
        token: token_t::zero(),
        next_token: token_t::zero(),
        error: std::ptr::null_mut(),
        flags: 0,
        line: 0,
        column: 0,
        pos: 0,
        has_error: 0,
    };
    let mut ap_copy_store = VaListTag {
        gp_offset: 0,
        fp_offset: 0,
        overflow_arg_area: std::ptr::null_mut(),
        reg_save_area: std::ptr::null_mut(),
    };

    if root.is_null() {
        jsonp_error_init(error, b"<root>\0".as_ptr() as *const c_char);
        jsonp_error_set_1s(
            error,
            -1,
            -1,
            0,
            json_error_null_value,
            b"NULL root value\0".as_ptr() as *const c_char,
        );
        return -1;
    }

    if fmt.is_null() || *fmt == 0 {
        jsonp_error_init(error, b"<format>\0".as_ptr() as *const c_char);
        jsonp_error_set_1s(
            error,
            -1,
            -1,
            0,
            json_error_invalid_argument,
            b"NULL or empty format string\0".as_ptr() as *const c_char,
        );
        return -1;
    }
    jsonp_error_init(error, std::ptr::null());

    scanner_init(&mut s, error, flags, fmt);
    next_token(&mut s);

    let ap_copy = crate::libc::va_copy(ap, &mut ap_copy_store);
    if unpack(&mut s, root, ap_copy) != 0 {
        return -1;
    }

    next_token(&mut s);
    if token(&s) != 0 {
        set_error!(
            &mut s as *mut scanner_t,
            b"<format>\0",
            json_error_invalid_format,
            b"Garbage after format string\0"
        );
        return -1;
    }

    0
}
