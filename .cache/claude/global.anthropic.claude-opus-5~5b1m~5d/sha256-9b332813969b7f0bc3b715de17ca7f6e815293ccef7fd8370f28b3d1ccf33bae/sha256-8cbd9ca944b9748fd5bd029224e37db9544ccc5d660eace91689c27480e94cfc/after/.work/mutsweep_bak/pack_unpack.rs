//! Translation of `src/pack_unpack.c`.

use core::ffi::{c_char, c_double, c_int, c_long, c_void};

use crate::error::{error_vset_with, jsonp_error_init, jsonp_error_set_source};
use crate::ffi::{self, VaListTag};
use crate::hashtable::{hashtable_close, hashtable_get, hashtable_init, hashtable_set, hashtable_t};
use crate::jansson::*;
use crate::memory::jsonp_free;
use crate::strbuffer::*;
use crate::utf::utf8_check_string;
use crate::value::*;

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
unsafe fn token(scanner: *const scanner_t) -> c_char {
    (*scanner).token.token
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
    (*s).prev_token = core::mem::zeroed();
    (*s).token = core::mem::zeroed();
    (*s).next_token = core::mem::zeroed();
    (*s).line = 1;
    (*s).column = 0;
    (*s).pos = 0;
    (*s).has_error = 0;
}

unsafe fn next_token(s: *mut scanner_t) {
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

unsafe fn prev_token(s: *mut scanner_t) {
    (*s).next_token = (*s).token;
    (*s).token = (*s).prev_token;
}

unsafe fn set_error_with<F>(s: *mut scanner_t, source: *const c_char, code: c_int, fmt: F)
where
    F: FnOnce(*mut c_char, usize),
{
    error_vset_with(
        (*s).error,
        (*s).token.line,
        (*s).token.column,
        (*s).token.pos,
        code,
        fmt,
    );

    jsonp_error_set_source((*s).error, source);
}

unsafe fn set_error(s: *mut scanner_t, source: *const c_char, code: c_int, msg: *const c_char) {
    set_error_with(s, source, code, |buf, n| {
        ffi::snprintf(buf, n, b"%s\0".as_ptr() as *const c_char, msg);
    });
}

/* ours will be set to 1 if jsonp_free() must be called for the result
afterwards */
unsafe fn read_string(
    s: *mut scanner_t,
    ap: *mut VaListTag,
    purpose: *const c_char,
    out_len: *mut usize,
    ours: *mut c_int,
    optional: c_int,
) -> *mut c_char {
    let t: c_char;
    let mut strbuff: strbuffer_t = strbuffer_t {
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
        str_ = ffi::va_arg_ptr::<c_char>(ap);

        if str_.is_null() {
            if optional == 0 {
                set_error_with(s, b"<args>\0".as_ptr() as *const c_char, json_error_null_value, |buf, n| {
                    ffi::snprintf(buf, n, b"NULL %s\0".as_ptr() as *const c_char, purpose);
                });
                (*s).has_error = 1;
            }
            return core::ptr::null_mut();
        }

        length = ffi::strlen(str_);

        if utf8_check_string(str_, length) == 0 {
            set_error_with(s, b"<args>\0".as_ptr() as *const c_char, json_error_invalid_utf8, |buf, n| {
                ffi::snprintf(
                    buf,
                    n,
                    b"Invalid UTF-8 %s\0".as_ptr() as *const c_char,
                    purpose,
                );
            });
            (*s).has_error = 1;
            return core::ptr::null_mut();
        }

        *out_len = length;
        return str_ as *mut c_char;
    } else if optional != 0 {
        set_error_with(
            s,
            b"<format>\0".as_ptr() as *const c_char,
            json_error_invalid_format,
            |buf, n| {
                ffi::snprintf(
                    buf,
                    n,
                    b"Cannot use '%c' on optional strings\0".as_ptr() as *const c_char,
                    t as c_int,
                );
            },
        );
        (*s).has_error = 1;

        return core::ptr::null_mut();
    }

    if strbuffer_init(&mut strbuff) != 0 {
        set_error(
            s,
            b"<internal>\0".as_ptr() as *const c_char,
            json_error_out_of_memory,
            b"Out of memory\0".as_ptr() as *const c_char,
        );
        (*s).has_error = 1;
    }

    loop {
        str_ = ffi::va_arg_ptr::<c_char>(ap);
        if str_.is_null() {
            set_error_with(s, b"<args>\0".as_ptr() as *const c_char, json_error_null_value, |buf, n| {
                ffi::snprintf(buf, n, b"NULL %s\0".as_ptr() as *const c_char, purpose);
            });
            (*s).has_error = 1;
        }

        next_token(s);

        if token(s) == b'#' as c_char {
            length = ffi::va_arg_int(ap) as usize;
        } else if token(s) == b'%' as c_char {
            length = ffi::va_arg_u64(ap) as usize;
        } else {
            prev_token(s);
            length = if (*s).has_error != 0 {
                0
            } else {
                ffi::strlen(str_)
            };
        }

        if (*s).has_error == 0 && strbuffer_append_bytes(&mut strbuff, str_, length) == -1 {
            set_error(
                s,
                b"<internal>\0".as_ptr() as *const c_char,
                json_error_out_of_memory,
                b"Out of memory\0".as_ptr() as *const c_char,
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
        set_error_with(s, b"<args>\0".as_ptr() as *const c_char, json_error_invalid_utf8, |buf, n| {
            ffi::snprintf(
                buf,
                n,
                b"Invalid UTF-8 %s\0".as_ptr() as *const c_char,
                purpose,
            );
        });
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

    while token(s) != b'}' as c_char {
        let key: *mut c_char;
        let mut len: usize = 0;
        let mut ours: c_int = 0;
        let value: *mut json_t;
        let value_optional: c_char;

        if token(s) == 0 {
            set_error(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                b"Unexpected end of format string\0".as_ptr() as *const c_char,
            );
            json_decref(object);
            return core::ptr::null_mut();
        }

        if token(s) != b's' as c_char {
            let tk = token(s);
            set_error_with(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                |buf, n| {
                    ffi::snprintf(
                        buf,
                        n,
                        b"Expected format 's', got '%c'\0".as_ptr() as *const c_char,
                        tk as c_int,
                    );
                },
            );
            json_decref(object);
            return core::ptr::null_mut();
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
                set_error(
                    s,
                    b"<args>\0".as_ptr() as *const c_char,
                    json_error_null_value,
                    b"NULL object value\0".as_ptr() as *const c_char,
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
            set_error_with(
                s,
                b"<internal>\0".as_ptr() as *const c_char,
                json_error_out_of_memory,
                |buf, n| {
                    ffi::snprintf(
                        buf,
                        n,
                        b"Unable to add key \"%s\"\0".as_ptr() as *const c_char,
                        key,
                    );
                },
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
    core::ptr::null_mut()
}

unsafe fn pack_array(s: *mut scanner_t, ap: *mut VaListTag) -> *mut json_t {
    let array = json_array();
    next_token(s);

    while token(s) != b']' as c_char {
        let value: *mut json_t;
        let value_optional: c_char;

        if token(s) == 0 {
            set_error(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                b"Unexpected end of format string\0".as_ptr() as *const c_char,
            );
            /* Format string errors are unrecoverable. */
            json_decref(array);
            return core::ptr::null_mut();
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
            set_error(
                s,
                b"<internal>\0".as_ptr() as *const c_char,
                json_error_out_of_memory,
                b"Unable to append to array\0".as_ptr() as *const c_char,
            );
            (*s).has_error = 1;
        }

        next_token(s);
    }

    if (*s).has_error == 0 {
        return array;
    }

    json_decref(array);
    core::ptr::null_mut()
}

unsafe fn pack_string(s: *mut scanner_t, ap: *mut VaListTag) -> *mut json_t {
    let mut len: usize = 0;
    let mut ours: c_int = 0;

    next_token(s);
    let t = token(s);
    let optional = (t == b'?' as c_char || t == b'*' as c_char) as c_int;
    if optional == 0 {
        prev_token(s);
    }

    let str_ = read_string(
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
    next_token(s);
    let ntoken = token(s);

    if ntoken != b'?' as c_char && ntoken != b'*' as c_char {
        prev_token(s);
    }

    let json = ffi::va_arg_ptr::<json_t>(ap);

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

    set_error(
        s,
        b"<args>\0".as_ptr() as *const c_char,
        json_error_null_value,
        b"NULL object\0".as_ptr() as *const c_char,
    );
    (*s).has_error = 1;
    core::ptr::null_mut()
}

unsafe fn pack_integer(s: *mut scanner_t, value: json_int_t) -> *mut json_t {
    let json = json_integer(value);

    if json.is_null() {
        set_error(
            s,
            b"<internal>\0".as_ptr() as *const c_char,
            json_error_out_of_memory,
            b"Out of memory\0".as_ptr() as *const c_char,
        );
        (*s).has_error = 1;
    }

    json
}

unsafe fn pack_real(s: *mut scanner_t, value: c_double) -> *mut json_t {
    /* Allocate without setting value so we can identify OOM error. */
    let json = json_real(0.0);

    if json.is_null() {
        set_error(
            s,
            b"<internal>\0".as_ptr() as *const c_char,
            json_error_out_of_memory,
            b"Out of memory\0".as_ptr() as *const c_char,
        );
        (*s).has_error = 1;

        return core::ptr::null_mut();
    }

    if json_real_set(json, value) != 0 {
        json_decref(json);

        set_error(
            s,
            b"<args>\0".as_ptr() as *const c_char,
            json_error_numeric_overflow,
            b"Invalid floating point value\0".as_ptr() as *const c_char,
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
            if ffi::va_arg_int(ap) != 0 {
                json_true()
            } else {
                json_false()
            }
        }

        b'i' => {
            /* integer from int */
            let v = ffi::va_arg_int(ap);
            pack_integer(s, v as json_int_t)
        }

        b'I' => {
            /* integer from json_int_t */
            let v = ffi::va_arg_u64(ap) as json_int_t;
            pack_integer(s, v)
        }

        b'f' => {
            /* real */
            let v = ffi::va_arg_f64(ap);
            pack_real(s, v)
        }

        b'O' => pack_object_inter(s, ap, 1), /* a json_t object; increments refcount */

        b'o' => pack_object_inter(s, ap, 0), /* a json_t object; doesn't increment refcount */

        _ => {
            let tk = token(s);
            set_error_with(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                |buf, n| {
                    ffi::snprintf(
                        buf,
                        n,
                        b"Unexpected format character '%c'\0".as_ptr() as *const c_char,
                        tk as c_int,
                    );
                },
            );
            (*s).has_error = 1;
            core::ptr::null_mut()
        }
    }
}

unsafe fn unpack_object(s: *mut scanner_t, root: *mut json_t, ap: *mut VaListTag) -> c_int {
    let mut ret: c_int = -1;
    let mut strict: c_int = 0;
    let mut gotopt: c_int = 0;

    /* Use a set (emulated by a hashtable) to check that all object keys are
       accessed. Checking that the correct number of keys were accessed is not
       enough, as the same key can be unpacked multiple times.
    */
    let mut key_set: hashtable_t = core::mem::zeroed();

    if hashtable_init(&mut key_set) != 0 {
        set_error(
            s,
            b"<internal>\0".as_ptr() as *const c_char,
            json_error_out_of_memory,
            b"Out of memory\0".as_ptr() as *const c_char,
        );
        return -1;
    }

    'out: {
        if !root.is_null() && !json_is_object(root) {
            set_error_with(
                s,
                b"<validation>\0".as_ptr() as *const c_char,
                json_error_wrong_type,
                |buf, n| {
                    ffi::snprintf(
                        buf,
                        n,
                        b"Expected object, got %s\0".as_ptr() as *const c_char,
                        type_name(root),
                    );
                },
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
                let tk = token(s);
                set_error_with(
                    s,
                    b"<format>\0".as_ptr() as *const c_char,
                    json_error_invalid_format,
                    |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"Expected '}' after '%c', got '%c'\0".as_ptr() as *const c_char,
                            (if strict == 1 { b'!' } else { b'*' }) as c_int,
                            tk as c_int,
                        );
                    },
                );
                break 'out;
            }

            if token(s) == 0 {
                set_error(
                    s,
                    b"<format>\0".as_ptr() as *const c_char,
                    json_error_invalid_format,
                    b"Unexpected end of format string\0".as_ptr() as *const c_char,
                );
                break 'out;
            }

            if token(s) == b'!' as c_char || token(s) == b'*' as c_char {
                strict = if token(s) == b'!' as c_char { 1 } else { -1 };
                next_token(s);
                continue;
            }

            if token(s) != b's' as c_char {
                let tk = token(s);
                set_error_with(
                    s,
                    b"<format>\0".as_ptr() as *const c_char,
                    json_error_invalid_format,
                    |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"Expected format 's', got '%c'\0".as_ptr() as *const c_char,
                            tk as c_int,
                        );
                    },
                );
                break 'out;
            }

            key = ffi::va_arg_ptr::<c_char>(ap);
            if key.is_null() {
                set_error(
                    s,
                    b"<args>\0".as_ptr() as *const c_char,
                    json_error_null_value,
                    b"NULL object key\0".as_ptr() as *const c_char,
                );
                break 'out;
            }
            key_len = ffi::strlen(key);

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
                    set_error_with(
                        s,
                        b"<validation>\0".as_ptr() as *const c_char,
                        json_error_item_not_found,
                        |buf, n| {
                            ffi::snprintf(
                                buf,
                                n,
                                b"Object item not found: %s\0".as_ptr() as *const c_char,
                                key,
                            );
                        },
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
            let mut unrecognized_keys: strbuffer_t = strbuffer_t {
                value: core::ptr::null_mut(),
                length: 0,
                size: 0,
            };
            let mut unpacked: c_long = 0;

            if gotopt != 0 || json_object_size(root) != key_set.size {
                keylen_foreach(root, |key, key_len, _value| {
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
                    true
                });
            }
            if unpacked != 0 {
                set_error_with(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_end_of_input_expected,
                    |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"%li object item(s) left unpacked: %s\0".as_ptr() as *const c_char,
                            unpacked,
                            if keys_res != 0 {
                                b"<unknown>\0".as_ptr() as *const c_char
                            } else {
                                strbuffer_value(&unrecognized_keys)
                            },
                        );
                    },
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

unsafe fn unpack_array(s: *mut scanner_t, root: *mut json_t, ap: *mut VaListTag) -> c_int {
    let mut i: usize = 0;
    let mut strict: c_int = 0;

    if !root.is_null() && !json_is_array(root) {
        set_error_with(
            s,
            b"<validation>\0".as_ptr() as *const c_char,
            json_error_wrong_type,
            |buf, n| {
                ffi::snprintf(
                    buf,
                    n,
                    b"Expected array, got %s\0".as_ptr() as *const c_char,
                    type_name(root),
                );
            },
        );
        return -1;
    }
    next_token(s);

    while token(s) != b']' as c_char {
        let value: *mut json_t;

        if strict != 0 {
            let tk = token(s);
            set_error_with(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                |buf, n| {
                    ffi::snprintf(
                        buf,
                        n,
                        b"Expected ']' after '%c', got '%c'\0".as_ptr() as *const c_char,
                        (if strict == 1 { b'!' } else { b'*' }) as c_int,
                        tk as c_int,
                    );
                },
            );
            return -1;
        }

        if token(s) == 0 {
            set_error(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                b"Unexpected end of format string\0".as_ptr() as *const c_char,
            );
            return -1;
        }

        if token(s) == b'!' as c_char || token(s) == b'*' as c_char {
            strict = if token(s) == b'!' as c_char { 1 } else { -1 };
            next_token(s);
            continue;
        }

        if ffi::strchr(
            UNPACK_VALUE_STARTERS.as_ptr() as *const c_char,
            token(s) as c_int,
        )
        .is_null()
        {
            let tk = token(s);
            set_error_with(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                |buf, n| {
                    ffi::snprintf(
                        buf,
                        n,
                        b"Unexpected format character '%c'\0".as_ptr() as *const c_char,
                        tk as c_int,
                    );
                },
            );
            return -1;
        }

        if root.is_null() {
            /* skipping */
            value = core::ptr::null_mut();
        } else {
            value = json_array_get(root, i);
            if value.is_null() {
                set_error_with(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_index_out_of_range,
                    |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"Array index %lu out of range\0".as_ptr() as *const c_char,
                            i as core::ffi::c_ulong,
                        );
                    },
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
        set_error_with(
            s,
            b"<validation>\0".as_ptr() as *const c_char,
            json_error_end_of_input_expected,
            |buf, n| {
                ffi::snprintf(
                    buf,
                    n,
                    b"%li array item(s) left unpacked\0".as_ptr() as *const c_char,
                    diff,
                );
            },
        );
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
                set_error_with(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_wrong_type,
                    |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"Expected string, got %s\0".as_ptr() as *const c_char,
                            type_name(root),
                        );
                    },
                );
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let mut len_target: *mut usize = core::ptr::null_mut();

                let str_target = ffi::va_arg_ptr::<*const c_char>(ap);
                if str_target.is_null() {
                    set_error(
                        s,
                        b"<args>\0".as_ptr() as *const c_char,
                        json_error_null_value,
                        b"NULL string argument\0".as_ptr() as *const c_char,
                    );
                    return -1;
                }

                next_token(s);

                if token(s) == b'%' as c_char {
                    len_target = ffi::va_arg_ptr::<usize>(ap);
                    if len_target.is_null() {
                        set_error(
                            s,
                            b"<args>\0".as_ptr() as *const c_char,
                            json_error_null_value,
                            b"NULL string length argument\0".as_ptr() as *const c_char,
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
                set_error_with(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_wrong_type,
                    |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"Expected integer, got %s\0".as_ptr() as *const c_char,
                            type_name(root),
                        );
                    },
                );
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = ffi::va_arg_ptr::<c_int>(ap);
                if !root.is_null() {
                    *target = json_integer_value(root) as c_int;
                }
            }

            0
        }

        b'I' => {
            if !root.is_null() && !json_is_integer(root) {
                set_error_with(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_wrong_type,
                    |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"Expected integer, got %s\0".as_ptr() as *const c_char,
                            type_name(root),
                        );
                    },
                );
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = ffi::va_arg_ptr::<json_int_t>(ap);
                if !root.is_null() {
                    *target = json_integer_value(root);
                }
            }

            0
        }

        b'b' => {
            if !root.is_null() && !json_is_boolean(root) {
                set_error_with(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_wrong_type,
                    |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"Expected true or false, got %s\0".as_ptr() as *const c_char,
                            type_name(root),
                        );
                    },
                );
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = ffi::va_arg_ptr::<c_int>(ap);
                if !root.is_null() {
                    *target = json_is_true(root) as c_int;
                }
            }

            0
        }

        b'f' => {
            if !root.is_null() && !json_is_real(root) {
                set_error_with(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_wrong_type,
                    |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"Expected real, got %s\0".as_ptr() as *const c_char,
                            type_name(root),
                        );
                    },
                );
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = ffi::va_arg_ptr::<c_double>(ap);
                if !root.is_null() {
                    *target = json_real_value(root);
                }
            }

            0
        }

        b'F' => {
            if !root.is_null() && !json_is_number(root) {
                set_error_with(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_wrong_type,
                    |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"Expected real or integer, got %s\0".as_ptr() as *const c_char,
                            type_name(root),
                        );
                    },
                );
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = ffi::va_arg_ptr::<c_double>(ap);
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
                let target = ffi::va_arg_ptr::<*mut json_t>(ap);
                if !root.is_null() {
                    *target = root;
                }
            }

            0
        }

        b'n' => {
            /* Never assign, just validate */
            if !root.is_null() && !json_is_null(root) {
                set_error_with(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_wrong_type,
                    |buf, n| {
                        ffi::snprintf(
                            buf,
                            n,
                            b"Expected null, got %s\0".as_ptr() as *const c_char,
                            type_name(root),
                        );
                    },
                );
                return -1;
            }
            0
        }

        _ => {
            let tk = token(s);
            set_error_with(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                |buf, n| {
                    ffi::snprintf(
                        buf,
                        n,
                        b"Unexpected format character '%c'\0".as_ptr() as *const c_char,
                        tk as c_int,
                    );
                },
            );
            -1
        }
    }
}

unsafe fn new_scanner() -> scanner_t {
    core::mem::zeroed()
}

/// The variadic `json_pack_ex` / `json_pack` symbols are produced by the
/// assembly shims in `varargs.rs`, which forward here.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_vpack_ex(
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
    ap: *mut VaListTag,
) -> *mut json_t {
    let mut s = new_scanner();

    if fmt.is_null() || *fmt == 0 {
        jsonp_error_init(error, b"<format>\0".as_ptr() as *const c_char);
        error_vset_with(
            error,
            -1,
            -1,
            0,
            json_error_invalid_argument,
            |buf, n| {
                ffi::snprintf(
                    buf,
                    n,
                    b"%s\0".as_ptr() as *const c_char,
                    b"NULL or empty format string\0".as_ptr() as *const c_char,
                );
            },
        );
        return core::ptr::null_mut();
    }
    jsonp_error_init(error, core::ptr::null());

    scanner_init(&mut s, error, flags, fmt);
    next_token(&mut s);

    let mut ap_copy: VaListTag = *ap;
    let value = pack(&mut s, &mut ap_copy);

    /* This will cover all situations where s.has_error is true */
    if value.is_null() {
        return core::ptr::null_mut();
    }

    next_token(&mut s);
    if token(&s) != 0 {
        json_decref(value);
        set_error(
            &mut s,
            b"<format>\0".as_ptr() as *const c_char,
            json_error_invalid_format,
            b"Garbage after format string\0".as_ptr() as *const c_char,
        );
        return core::ptr::null_mut();
    }

    value
}

/// The variadic `json_unpack_ex` / `json_unpack` symbols are produced by the
/// assembly shims in `varargs.rs`, which forward here.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_vunpack_ex(
    root: *mut json_t,
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
    ap: *mut VaListTag,
) -> c_int {
    let mut s = new_scanner();

    if root.is_null() {
        jsonp_error_init(error, b"<root>\0".as_ptr() as *const c_char);
        error_vset_with(error, -1, -1, 0, json_error_null_value, |buf, n| {
            ffi::snprintf(
                buf,
                n,
                b"%s\0".as_ptr() as *const c_char,
                b"NULL root value\0".as_ptr() as *const c_char,
            );
        });
        return -1;
    }

    if fmt.is_null() || *fmt == 0 {
        jsonp_error_init(error, b"<format>\0".as_ptr() as *const c_char);
        error_vset_with(
            error,
            -1,
            -1,
            0,
            json_error_invalid_argument,
            |buf, n| {
                ffi::snprintf(
                    buf,
                    n,
                    b"%s\0".as_ptr() as *const c_char,
                    b"NULL or empty format string\0".as_ptr() as *const c_char,
                );
            },
        );
        return -1;
    }
    jsonp_error_init(error, core::ptr::null());

    scanner_init(&mut s, error, flags, fmt);
    next_token(&mut s);

    let mut ap_copy: VaListTag = *ap;
    if unpack(&mut s, root, &mut ap_copy) != 0 {
        return -1;
    }

    next_token(&mut s);
    if token(&s) != 0 {
        set_error(
            &mut s,
            b"<format>\0".as_ptr() as *const c_char,
            json_error_invalid_format,
            b"Garbage after format string\0".as_ptr() as *const c_char,
        );
        return -1;
    }

    0
}
