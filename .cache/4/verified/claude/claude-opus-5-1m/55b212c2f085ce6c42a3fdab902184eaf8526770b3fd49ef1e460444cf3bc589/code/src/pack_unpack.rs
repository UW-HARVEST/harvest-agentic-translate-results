//! Translation of `src/pack_unpack.c`.
//!
//! `json_pack`, `json_pack_ex`, `json_unpack` and `json_unpack_ex` are the
//! variadic entry points and live in `vararg.rs`.

use crate::error::{jsonp_error_init, jsonp_error_set_msg, jsonp_error_set_raw, jsonp_error_set_source};
use crate::hashtable::*;
use crate::memory::jsonp_free;
use crate::strbuffer::*;
use crate::types::*;
use crate::utf::utf8_check_string;
use crate::value::*;
use core::ffi::{c_char, c_int, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
struct token_t {
    line: c_int,
    column: c_int,
    pos: usize,
    token: c_char,
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
unsafe fn type_name(x: *const json_t) -> &'static [u8] {
    let t = json_typeof(x);
    /* `type_names[json_typeof(x)]` — guard against a corrupt type tag so that
       a bogus `json_t` cannot turn an out-of-bounds read into a Rust panic. */
    let n = if t >= 0 && (t as usize) < TYPE_NAMES.len() {
        TYPE_NAMES[t as usize]
    } else {
        b"\0"
    };
    &n[..n.len() - 1]
}

static UNPACK_VALUE_STARTERS: &[u8; 12] = b"{[siIbfFOon\0";

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
    let zero = token_t {
        line: 0,
        column: 0,
        pos: 0,
        token: 0,
    };
    (*s).prev_token = zero;
    (*s).token = zero;
    (*s).next_token = zero;
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

unsafe fn set_error(s: *mut scanner_t, source: &[u8], code: c_int, msg: &[u8]) {
    jsonp_error_set_raw(
        (*s).error,
        (*s).token.line,
        (*s).token.column,
        (*s).token.pos,
        code,
        msg,
    );

    jsonp_error_set_source((*s).error, source.as_ptr() as *const c_char);
}

/// ours will be set to 1 if `jsonp_free()` must be called for the result
/// afterwards
unsafe fn read_string(
    s: *mut scanner_t,
    ap: *mut VaListTag,
    purpose: &[u8],
    out_len: *mut usize,
    ours: *mut c_int,
    optional: c_int,
) -> *mut c_char {
    let t: c_char;
    let mut strbuff = strbuffer_t {
        value: core::ptr::null_mut(),
        length: 0,
        size: 0,
    };
    let mut str_: *const c_char;
    let mut length: usize;

    next_token(s);
    t = token(s);
    prev_token(s);

    *ours = 0;
    if t != b'#' as c_char && t != b'%' as c_char && t != b'+' as c_char {
        /* Optimize the simple case */
        str_ = (*ap).arg_gp::<*const c_char>();

        if str_.is_null() {
            if optional == 0 {
                let mut m: Vec<u8> = Vec::new();
                m.extend_from_slice(b"NULL ");
                m.extend_from_slice(purpose);
                set_error(s, b"<args>\0", JSON_ERROR_NULL_VALUE, &m);
                (*s).has_error = 1;
            }
            return core::ptr::null_mut();
        }

        length = strlen(str_);

        if utf8_check_string(str_, length) == 0 {
            let mut m: Vec<u8> = Vec::new();
            m.extend_from_slice(b"Invalid UTF-8 ");
            m.extend_from_slice(purpose);
            set_error(s, b"<args>\0", JSON_ERROR_INVALID_UTF8, &m);
            (*s).has_error = 1;
            return core::ptr::null_mut();
        }

        *out_len = length;
        return str_ as *mut c_char;
    } else if optional != 0 {
        let mut m: Vec<u8> = Vec::new();
        m.extend_from_slice(b"Cannot use '");
        m.push(t as u8);
        m.extend_from_slice(b"' on optional strings");
        set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, &m);
        (*s).has_error = 1;

        return core::ptr::null_mut();
    }

    if strbuffer_init(&mut strbuff) != 0 {
        set_error(
            s,
            b"<internal>\0",
            JSON_ERROR_OUT_OF_MEMORY,
            b"Out of memory",
        );
        (*s).has_error = 1;
    }

    loop {
        str_ = (*ap).arg_gp::<*const c_char>();
        if str_.is_null() {
            let mut m: Vec<u8> = Vec::new();
            m.extend_from_slice(b"NULL ");
            m.extend_from_slice(purpose);
            set_error(s, b"<args>\0", JSON_ERROR_NULL_VALUE, &m);
            (*s).has_error = 1;
        }

        next_token(s);

        if token(s) == b'#' as c_char {
            length = (*ap).arg_gp::<c_int>() as usize;
        } else if token(s) == b'%' as c_char {
            length = (*ap).arg_gp::<usize>();
        } else {
            prev_token(s);
            length = if (*s).has_error != 0 { 0 } else { strlen(str_) };
        }

        if (*s).has_error == 0 && strbuffer_append_bytes(&mut strbuff, str_, length) == -1 {
            set_error(
                s,
                b"<internal>\0",
                JSON_ERROR_OUT_OF_MEMORY,
                b"Out of memory",
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
        return core::ptr::null_mut();
    }

    if utf8_check_string(strbuff.value, strbuff.length) == 0 {
        let mut m: Vec<u8> = Vec::new();
        m.extend_from_slice(b"Invalid UTF-8 ");
        m.extend_from_slice(purpose);
        set_error(s, b"<args>\0", JSON_ERROR_INVALID_UTF8, &m);
        strbuffer_close(&mut strbuff);
        (*s).has_error = 1;
        return core::ptr::null_mut();
    }

    *out_len = strbuff.length;
    *ours = 1;
    strbuffer_steal_value(&mut strbuff)
}

unsafe fn pack_object(s: *mut scanner_t, ap: *mut VaListTag) -> *mut json_t {
    let object = json_object();
    next_token(s);

    'error: {
        while token(s) != b'}' as c_char {
            let key: *mut c_char;
            let mut len: usize = 0;
            let mut ours: c_int = 0;
            let value: *mut json_t;
            let valueOptional: c_char;

            if token(s) == 0 {
                set_error(
                    s,
                    b"<format>\0",
                    JSON_ERROR_INVALID_FORMAT,
                    b"Unexpected end of format string",
                );
                break 'error;
            }

            if token(s) != b's' as c_char {
                let mut m: Vec<u8> = Vec::new();
                m.extend_from_slice(b"Expected format 's', got '");
                m.push(token(s) as u8);
                m.push(b'\'');
                set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, &m);
                break 'error;
            }

            key = read_string(s, ap, b"object key", &mut len, &mut ours, 0);

            next_token(s);

            next_token(s);
            valueOptional = token(s);
            prev_token(s);

            value = pack(s, ap);
            if value.is_null() {
                if ours != 0 {
                    jsonp_free(key as *mut c_void);
                }

                if valueOptional != b'*' as c_char {
                    set_error(
                        s,
                        b"<args>\0",
                        JSON_ERROR_NULL_VALUE,
                        b"NULL object value",
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
                let mut m: Vec<u8> = Vec::new();
                m.extend_from_slice(b"Unable to add key \"");
                m.extend_from_slice(cstr_slice(key));
                m.extend_from_slice(b"\"");
                set_error(s, b"<internal>\0", JSON_ERROR_OUT_OF_MEMORY, &m);
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

    /* error: */
    json_decref(object);
    core::ptr::null_mut()
}

unsafe fn cstr_slice<'a>(p: *const c_char) -> &'a [u8] {
    if p.is_null() {
        return &[];
    }
    core::slice::from_raw_parts(p as *const u8, strlen(p))
}

unsafe fn pack_array(s: *mut scanner_t, ap: *mut VaListTag) -> *mut json_t {
    let array = json_array();
    next_token(s);

    'error: {
        while token(s) != b']' as c_char {
            let value: *mut json_t;
            let valueOptional: c_char;

            if token(s) == 0 {
                set_error(
                    s,
                    b"<format>\0",
                    JSON_ERROR_INVALID_FORMAT,
                    b"Unexpected end of format string",
                );
                /* Format string errors are unrecoverable. */
                break 'error;
            }

            next_token(s);
            valueOptional = token(s);
            prev_token(s);

            value = pack(s, ap);
            if value.is_null() {
                if valueOptional != b'*' as c_char {
                    (*s).has_error = 1;
                }

                next_token(s);
                continue;
            }

            if (*s).has_error != 0 {
                json_decref(value);
            }

            if (*s).has_error == 0 && json_array_append_new(array, value) != 0 {
                set_error(
                    s,
                    b"<internal>\0",
                    JSON_ERROR_OUT_OF_MEMORY,
                    b"Unable to append to array",
                );
                (*s).has_error = 1;
            }

            next_token(s);
        }

        if (*s).has_error == 0 {
            return array;
        }
    }

    /* error: */
    json_decref(array);
    core::ptr::null_mut()
}

unsafe fn pack_string(s: *mut scanner_t, ap: *mut VaListTag) -> *mut json_t {
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

    str_ = read_string(s, ap, b"string", &mut len, &mut ours, optional);

    if str_.is_null() {
        return if t == b'?' as c_char && (*s).has_error == 0 {
            json_null()
        } else {
            core::ptr::null_mut()
        };
    }

    if (*s).has_error != 0 {
        /* It's impossible to reach this point if ours != 0, do not free str. */
        return core::ptr::null_mut();
    }

    if ours != 0 {
        return jsonp_stringn_nocheck_own(str_, len);
    }

    json_stringn_nocheck(str_, len)
}

unsafe fn pack_object_inter(
    s: *mut scanner_t,
    ap: *mut VaListTag,
    need_incref: c_int,
) -> *mut json_t {
    let json: *mut json_t;
    let ntoken: c_char;

    next_token(s);
    ntoken = token(s);

    if ntoken != b'?' as c_char && ntoken != b'*' as c_char {
        prev_token(s);
    }

    json = (*ap).arg_gp::<*mut json_t>();

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
        return core::ptr::null_mut();
    }

    set_error(s, b"<args>\0", JSON_ERROR_NULL_VALUE, b"NULL object");
    (*s).has_error = 1;
    core::ptr::null_mut()
}

unsafe fn pack_integer(s: *mut scanner_t, value: json_int_t) -> *mut json_t {
    let json = json_integer(value);

    if json.is_null() {
        set_error(
            s,
            b"<internal>\0",
            JSON_ERROR_OUT_OF_MEMORY,
            b"Out of memory",
        );
        (*s).has_error = 1;
    }

    json
}

unsafe fn pack_real(s: *mut scanner_t, value: f64) -> *mut json_t {
    /* Allocate without setting value so we can identify OOM error. */
    let json = json_real(0.0);

    if json.is_null() {
        set_error(
            s,
            b"<internal>\0",
            JSON_ERROR_OUT_OF_MEMORY,
            b"Out of memory",
        );
        (*s).has_error = 1;

        return core::ptr::null_mut();
    }

    if json_real_set(json, value) != 0 {
        json_decref(json);

        set_error(
            s,
            b"<args>\0",
            JSON_ERROR_NUMERIC_OVERFLOW,
            b"Invalid floating point value",
        );
        (*s).has_error = 1;

        return core::ptr::null_mut();
    }

    json
}

unsafe fn pack(s: *mut scanner_t, ap: *mut VaListTag) -> *mut json_t {
    match token(s) as u8 {
        b'{' => pack_object(s, ap),

        b'[' => pack_array(s, ap),

        b's' => pack_string(s, ap), /* string */

        b'n' => json_null(), /* null */

        b'b' => {
            /* boolean */
            if (*ap).arg_gp::<c_int>() != 0 {
                json_true()
            } else {
                json_false()
            }
        }

        b'i' => {
            /* integer from int */
            let v = (*ap).arg_gp::<c_int>();
            pack_integer(s, v as json_int_t)
        }

        b'I' => {
            /* integer from json_int_t */
            let v = (*ap).arg_gp::<json_int_t>();
            pack_integer(s, v)
        }

        b'f' => {
            /* real */
            let v = (*ap).arg_double();
            pack_real(s, v)
        }

        b'O' => pack_object_inter(s, ap, 1), /* increments refcount */

        b'o' => pack_object_inter(s, ap, 0), /* doesn't increment refcount */

        _ => {
            let mut m: Vec<u8> = Vec::new();
            m.extend_from_slice(b"Unexpected format character '");
            m.push(token(s) as u8);
            m.push(b'\'');
            set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, &m);
            (*s).has_error = 1;
            core::ptr::null_mut()
        }
    }
}

unsafe fn unpack_object(s: *mut scanner_t, root: *mut json_t, ap: *mut VaListTag) -> c_int {
    let mut ret: c_int = -1;
    let mut strict: c_int = 0;
    let mut gotopt: c_int = 0;

    /* Use a set (emulated by a hashtable) to check that all object
       keys are accessed. Checking that the correct number of keys
       were accessed is not enough, as the same key can be unpacked
       multiple times.
    */
    let mut key_set = core::mem::MaybeUninit::<hashtable_t>::uninit();

    if hashtable_init(key_set.as_mut_ptr()) != 0 {
        set_error(
            s,
            b"<internal>\0",
            JSON_ERROR_OUT_OF_MEMORY,
            b"Out of memory",
        );
        return -1;
    }
    let key_set = key_set.as_mut_ptr();

    'out: {
        if !root.is_null() && !json_is_object(root) {
            let mut m: Vec<u8> = Vec::new();
            m.extend_from_slice(b"Expected object, got ");
            m.extend_from_slice(type_name(root));
            set_error(s, b"<validation>\0", JSON_ERROR_WRONG_TYPE, &m);
            break 'out;
        }
        next_token(s);

        while token(s) != b'}' as c_char {
            let key: *const c_char;
            let key_len: usize;
            let value: *mut json_t;
            let mut opt: c_int = 0;

            if strict != 0 {
                let mut m: Vec<u8> = Vec::new();
                m.extend_from_slice(b"Expected '}' after '");
                m.push(if strict == 1 { b'!' } else { b'*' });
                m.extend_from_slice(b"', got '");
                m.push(token(s) as u8);
                m.push(b'\'');
                set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, &m);
                break 'out;
            }

            if token(s) == 0 {
                set_error(
                    s,
                    b"<format>\0",
                    JSON_ERROR_INVALID_FORMAT,
                    b"Unexpected end of format string",
                );
                break 'out;
            }

            if token(s) == b'!' as c_char || token(s) == b'*' as c_char {
                strict = if token(s) == b'!' as c_char { 1 } else { -1 };
                next_token(s);
                continue;
            }

            if token(s) != b's' as c_char {
                let mut m: Vec<u8> = Vec::new();
                m.extend_from_slice(b"Expected format 's', got '");
                m.push(token(s) as u8);
                m.push(b'\'');
                set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, &m);
                break 'out;
            }

            key = (*ap).arg_gp::<*const c_char>();
            if key.is_null() {
                set_error(
                    s,
                    b"<args>\0",
                    JSON_ERROR_NULL_VALUE,
                    b"NULL object key",
                );
                break 'out;
            }
            key_len = strlen(key);

            next_token(s);

            if token(s) == b'?' as c_char {
                opt = 1;
                gotopt = 1;
                next_token(s);
            }

            if root.is_null() {
                /* skipping */
                value = core::ptr::null_mut();
            } else {
                value = json_object_getn(root, key, key_len);
                if value.is_null() && opt == 0 {
                    let mut m: Vec<u8> = Vec::new();
                    m.extend_from_slice(b"Object item not found: ");
                    m.extend_from_slice(cstr_slice(key));
                    set_error(s, b"<validation>\0", JSON_ERROR_ITEM_NOT_FOUND, &m);
                    break 'out;
                }
            }

            if unpack(s, value, ap) != 0 {
                break 'out;
            }

            hashtable_set(key_set, key, key_len, json_null());
            next_token(s);
        }

        if strict == 0 && ((*s).flags & JSON_STRICT) != 0 {
            strict = 1;
        }

        if !root.is_null() && strict == 1 {
            /* We need to check that all non optional items have been parsed */
            /* keys_res is 1 for uninitialized, 0 for success, -1 for error. */
            let mut keys_res: c_int = 1;
            let mut unrecognized_keys = strbuffer_t {
                value: core::ptr::null_mut(),
                length: 0,
                size: 0,
            };
            let mut unpacked: i64 = 0;

            if gotopt != 0 || json_object_size(root) != (*key_set).size {
                let mut key = json_object_iter_key(json_object_iter(root));
                let mut key_len = json_object_iter_key_len(json_object_key_to_iter(key));
                loop {
                    if key.is_null() {
                        break;
                    }
                    let value = json_object_iter_value(json_object_key_to_iter(key));
                    if value.is_null() {
                        break;
                    }

                    if hashtable_get(key_set, key, key_len).is_null() {
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

                    key = json_object_iter_key(json_object_iter_next(
                        root,
                        json_object_key_to_iter(key),
                    ));
                    key_len = json_object_iter_key_len(json_object_key_to_iter(key));
                }
            }
            if unpacked != 0 {
                let mut m: Vec<u8> = Vec::new();
                m.extend_from_slice(format!("{}", unpacked).as_bytes());
                m.extend_from_slice(b" object item(s) left unpacked: ");
                if keys_res != 0 {
                    m.extend_from_slice(b"<unknown>");
                } else {
                    m.extend_from_slice(cstr_slice(strbuffer_value(&unrecognized_keys)));
                }
                set_error(
                    s,
                    b"<validation>\0",
                    JSON_ERROR_END_OF_INPUT_EXPECTED,
                    &m,
                );
                strbuffer_close(&mut unrecognized_keys);
                break 'out;
            }
        }

        ret = 0;
    }

    /* out: */
    hashtable_close(key_set);
    ret
}

unsafe fn unpack_array(s: *mut scanner_t, root: *mut json_t, ap: *mut VaListTag) -> c_int {
    let mut i: usize = 0;
    let mut strict: c_int = 0;

    if !root.is_null() && !json_is_array(root) {
        let mut m: Vec<u8> = Vec::new();
        m.extend_from_slice(b"Expected array, got ");
        m.extend_from_slice(type_name(root));
        set_error(s, b"<validation>\0", JSON_ERROR_WRONG_TYPE, &m);
        return -1;
    }
    next_token(s);

    while token(s) != b']' as c_char {
        let value: *mut json_t;

        if strict != 0 {
            let mut m: Vec<u8> = Vec::new();
            m.extend_from_slice(b"Expected ']' after '");
            m.push(if strict == 1 { b'!' } else { b'*' });
            m.extend_from_slice(b"', got '");
            m.push(token(s) as u8);
            m.push(b'\'');
            set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, &m);
            return -1;
        }

        if token(s) == 0 {
            set_error(
                s,
                b"<format>\0",
                JSON_ERROR_INVALID_FORMAT,
                b"Unexpected end of format string",
            );
            return -1;
        }

        if token(s) == b'!' as c_char || token(s) == b'*' as c_char {
            strict = if token(s) == b'!' as c_char { 1 } else { -1 };
            next_token(s);
            continue;
        }

        if strchr(
            UNPACK_VALUE_STARTERS.as_ptr() as *const c_char,
            token(s) as c_int,
        )
        .is_null()
        {
            let mut m: Vec<u8> = Vec::new();
            m.extend_from_slice(b"Unexpected format character '");
            m.push(token(s) as u8);
            m.push(b'\'');
            set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, &m);
            return -1;
        }

        if root.is_null() {
            /* skipping */
            value = core::ptr::null_mut();
        } else {
            value = json_array_get(root, i);
            if value.is_null() {
                let mut m: Vec<u8> = Vec::new();
                m.extend_from_slice(b"Array index ");
                m.extend_from_slice(format!("{}", i as u64).as_bytes());
                m.extend_from_slice(b" out of range");
                set_error(s, b"<validation>\0", JSON_ERROR_INDEX_OUT_OF_RANGE, &m);
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
        let diff: i64 = json_array_size(root) as i64 - i as i64;
        let mut m: Vec<u8> = Vec::new();
        m.extend_from_slice(format!("{}", diff).as_bytes());
        m.extend_from_slice(b" array item(s) left unpacked");
        set_error(s, b"<validation>\0", JSON_ERROR_END_OF_INPUT_EXPECTED, &m);
        return -1;
    }

    0
}

unsafe fn unpack(s: *mut scanner_t, root: *mut json_t, ap: *mut VaListTag) -> c_int {
    match token(s) as u8 {
        b'{' => unpack_object(s, root, ap),

        b'[' => unpack_array(s, root, ap),

        b's' => {
            if !root.is_null() && !json_is_string(root) {
                let mut m: Vec<u8> = Vec::new();
                m.extend_from_slice(b"Expected string, got ");
                m.extend_from_slice(type_name(root));
                set_error(s, b"<validation>\0", JSON_ERROR_WRONG_TYPE, &m);
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let str_target: *mut *const c_char;
                let mut len_target: *mut usize = core::ptr::null_mut();

                str_target = (*ap).arg_gp::<*mut *const c_char>();
                if str_target.is_null() {
                    set_error(
                        s,
                        b"<args>\0",
                        JSON_ERROR_NULL_VALUE,
                        b"NULL string argument",
                    );
                    return -1;
                }

                next_token(s);

                if token(s) == b'%' as c_char {
                    len_target = (*ap).arg_gp::<*mut usize>();
                    if len_target.is_null() {
                        set_error(
                            s,
                            b"<args>\0",
                            JSON_ERROR_NULL_VALUE,
                            b"NULL string length argument",
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
                let mut m: Vec<u8> = Vec::new();
                m.extend_from_slice(b"Expected integer, got ");
                m.extend_from_slice(type_name(root));
                set_error(s, b"<validation>\0", JSON_ERROR_WRONG_TYPE, &m);
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = (*ap).arg_gp::<*mut c_int>();
                if !root.is_null() {
                    *target = json_integer_value(root) as c_int;
                }
            }

            0
        }

        b'I' => {
            if !root.is_null() && !json_is_integer(root) {
                let mut m: Vec<u8> = Vec::new();
                m.extend_from_slice(b"Expected integer, got ");
                m.extend_from_slice(type_name(root));
                set_error(s, b"<validation>\0", JSON_ERROR_WRONG_TYPE, &m);
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = (*ap).arg_gp::<*mut json_int_t>();
                if !root.is_null() {
                    *target = json_integer_value(root);
                }
            }

            0
        }

        b'b' => {
            if !root.is_null() && !json_is_boolean(root) {
                let mut m: Vec<u8> = Vec::new();
                m.extend_from_slice(b"Expected true or false, got ");
                m.extend_from_slice(type_name(root));
                set_error(s, b"<validation>\0", JSON_ERROR_WRONG_TYPE, &m);
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = (*ap).arg_gp::<*mut c_int>();
                if !root.is_null() {
                    *target = json_is_true(root) as c_int;
                }
            }

            0
        }

        b'f' => {
            if !root.is_null() && !json_is_real(root) {
                let mut m: Vec<u8> = Vec::new();
                m.extend_from_slice(b"Expected real, got ");
                m.extend_from_slice(type_name(root));
                set_error(s, b"<validation>\0", JSON_ERROR_WRONG_TYPE, &m);
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = (*ap).arg_gp::<*mut f64>();
                if !root.is_null() {
                    *target = json_real_value(root);
                }
            }

            0
        }

        b'F' => {
            if !root.is_null() && !json_is_number(root) {
                let mut m: Vec<u8> = Vec::new();
                m.extend_from_slice(b"Expected real or integer, got ");
                m.extend_from_slice(type_name(root));
                set_error(s, b"<validation>\0", JSON_ERROR_WRONG_TYPE, &m);
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = (*ap).arg_gp::<*mut f64>();
                if !root.is_null() {
                    *target = json_number_value(root);
                }
            }

            0
        }

        b'O' | b'o' => {
            if token(s) as u8 == b'O'
                && !root.is_null()
                && ((*s).flags & JSON_VALIDATE_ONLY) == 0
            {
                json_incref(root);
            }
            /* Fall through */

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = (*ap).arg_gp::<*mut *mut json_t>();
                if !root.is_null() {
                    *target = root;
                }
            }

            0
        }

        b'n' => {
            /* Never assign, just validate */
            if !root.is_null() && !json_is_null(root) {
                let mut m: Vec<u8> = Vec::new();
                m.extend_from_slice(b"Expected null, got ");
                m.extend_from_slice(type_name(root));
                set_error(s, b"<validation>\0", JSON_ERROR_WRONG_TYPE, &m);
                return -1;
            }
            0
        }

        _ => {
            let mut m: Vec<u8> = Vec::new();
            m.extend_from_slice(b"Unexpected format character '");
            m.push(token(s) as u8);
            m.push(b'\'');
            set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, &m);
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
    let mut s = core::mem::MaybeUninit::<scanner_t>::uninit();
    let value: *mut json_t;

    if fmt.is_null() || *fmt == 0 {
        jsonp_error_init(error, b"<format>\0".as_ptr() as *const c_char);
        jsonp_error_set_msg(
            error,
            -1,
            -1,
            0,
            JSON_ERROR_INVALID_ARGUMENT,
            b"NULL or empty format string",
        );
        return core::ptr::null_mut();
    }
    jsonp_error_init(error, core::ptr::null());

    let s = s.as_mut_ptr();
    scanner_init(s, error, flags, fmt);
    next_token(s);

    /* va_copy(ap_copy, ap) */
    let mut ap_copy: VaListTag = *ap;
    value = pack(s, &mut ap_copy);

    /* This will cover all situations where s.has_error is true */
    if value.is_null() {
        return core::ptr::null_mut();
    }

    next_token(s);
    if token(s) != 0 {
        json_decref(value);
        set_error(
            s,
            b"<format>\0",
            JSON_ERROR_INVALID_FORMAT,
            b"Garbage after format string",
        );
        return core::ptr::null_mut();
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
    let mut s = core::mem::MaybeUninit::<scanner_t>::uninit();

    if root.is_null() {
        jsonp_error_init(error, b"<root>\0".as_ptr() as *const c_char);
        jsonp_error_set_msg(error, -1, -1, 0, JSON_ERROR_NULL_VALUE, b"NULL root value");
        return -1;
    }

    if fmt.is_null() || *fmt == 0 {
        jsonp_error_init(error, b"<format>\0".as_ptr() as *const c_char);
        jsonp_error_set_msg(
            error,
            -1,
            -1,
            0,
            JSON_ERROR_INVALID_ARGUMENT,
            b"NULL or empty format string",
        );
        return -1;
    }
    jsonp_error_init(error, core::ptr::null());

    let s = s.as_mut_ptr();
    scanner_init(s, error, flags, fmt);
    next_token(s);

    /* va_copy(ap_copy, ap) */
    let mut ap_copy: VaListTag = *ap;
    if unpack(s, root, &mut ap_copy) != 0 {
        return -1;
    }

    next_token(s);
    if token(s) != 0 {
        set_error(
            s,
            b"<format>\0",
            JSON_ERROR_INVALID_FORMAT,
            b"Garbage after format string",
        );
        return -1;
    }

    0
}
