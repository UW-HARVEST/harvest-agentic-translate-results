//! Translation of pack_unpack.c
#![allow(non_upper_case_globals)]

use crate::error::{jsonp_error_init, jsonp_error_set, jsonp_error_set_source, jsonp_error_vset};
use crate::hashtable::*;
use crate::memory::jsonp_free;
use crate::strbuffer::*;
use crate::types::*;
use crate::utf::utf8_check_string;
use crate::value::*;
use core::ffi::{c_char, c_double, c_int, c_void, VaList};
use core::ptr;

extern "C" {
    fn strlen(s: *const c_char) -> usize;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct token_t {
    line: c_int,
    column: c_int,
    pos: usize,
    token: c_char,
}

impl token_t {
    fn zero() -> Self {
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
unsafe fn token(scanner: *mut scanner_t) -> c_char {
    (*scanner).token.token
}

static type_names: [&[u8]; 8] = [
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
    type_names[json_typeof(x) as usize].as_ptr() as *const c_char
}

const unpack_value_starters: &[u8; 12] = b"{[siIbfFOon\0";

unsafe fn scanner_init(
    s: *mut scanner_t,
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
) {
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
    // `wrapping_add`, not `+=`, for `line`/`column`: they are plain `int`s the C
    // bumps once per format-string character with no guard, and the format string
    // comes straight from the caller of json_pack()/json_unpack().  A >2 GB format
    // string wraps them in the C; Rust's `+=` would panic under overflow-checks.
    // `pos` is a `size_t` and would need 2^64 increments, so it stays a plain `+=`.
    (*s).column = (*s).column.wrapping_add(1);
    (*s).pos += 1;

    /* skip space and ignored chars */
    while *t == ' ' as c_char
        || *t == '\t' as c_char
        || *t == '\n' as c_char
        || *t == ',' as c_char
        || *t == ':' as c_char
    {
        if *t == '\n' as c_char {
            (*s).line = (*s).line.wrapping_add(1);
            (*s).column = 1;
        } else {
            (*s).column = (*s).column.wrapping_add(1);
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

unsafe extern "C" fn set_error(
    s: *mut scanner_t,
    source: *const c_char,
    code: c_int,
    fmt: *const c_char,
    ap: ...
) {
    jsonp_error_vset(
        (*s).error,
        (*s).token.line,
        (*s).token.column,
        (*s).token.pos,
        code,
        fmt,
        ap,
    );

    jsonp_error_set_source((*s).error, source);
}

/* ours will be set to 1 if jsonp_free() must be called for the result afterwards */
unsafe fn read_string(
    s: *mut scanner_t,
    ap: &mut VaList,
    purpose: *const c_char,
    out_len: *mut usize,
    ours: *mut c_int,
    optional: c_int,
) -> *mut c_char {
    let t: c_char;
    let mut strbuff: strbuffer_t = core::mem::zeroed();
    let mut str: *const c_char;
    let mut length: usize;

    next_token(s);
    t = token(s);
    prev_token(s);

    *ours = 0;
    if t != '#' as c_char && t != '%' as c_char && t != '+' as c_char {
        /* Optimize the simple case */
        str = ap.arg::<*const c_char>();

        if str.is_null() {
            if optional == 0 {
                set_error(
                    s,
                    b"<args>\0".as_ptr() as *const c_char,
                    json_error_null_value,
                    b"NULL %s\0".as_ptr() as *const c_char,
                    purpose,
                );
                (*s).has_error = 1;
            }
            return ptr::null_mut();
        }

        length = strlen(str);

        if utf8_check_string(str, length) == 0 {
            set_error(
                s,
                b"<args>\0".as_ptr() as *const c_char,
                json_error_invalid_utf8,
                b"Invalid UTF-8 %s\0".as_ptr() as *const c_char,
                purpose,
            );
            (*s).has_error = 1;
            return ptr::null_mut();
        }

        *out_len = length;
        return str as *mut c_char;
    } else if optional != 0 {
        set_error(
            s,
            b"<format>\0".as_ptr() as *const c_char,
            json_error_invalid_format,
            b"Cannot use '%c' on optional strings\0".as_ptr() as *const c_char,
            t as c_int,
        );
        (*s).has_error = 1;

        return ptr::null_mut();
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
        str = ap.arg::<*const c_char>();
        if str.is_null() {
            set_error(
                s,
                b"<args>\0".as_ptr() as *const c_char,
                json_error_null_value,
                b"NULL %s\0".as_ptr() as *const c_char,
                purpose,
            );
            (*s).has_error = 1;
        }

        next_token(s);

        if token(s) == '#' as c_char {
            length = ap.arg::<c_int>() as usize;
        } else if token(s) == '%' as c_char {
            length = ap.arg::<usize>();
        } else {
            prev_token(s);
            length = if (*s).has_error != 0 { 0 } else { strlen(str) };
        }

        if (*s).has_error == 0 && strbuffer_append_bytes(&mut strbuff, str, length) == -1 {
            set_error(
                s,
                b"<internal>\0".as_ptr() as *const c_char,
                json_error_out_of_memory,
                b"Out of memory\0".as_ptr() as *const c_char,
            );
            (*s).has_error = 1;
        }

        next_token(s);
        if token(s) != '+' as c_char {
            prev_token(s);
            break;
        }
    }

    if (*s).has_error != 0 {
        strbuffer_close(&mut strbuff);
        return ptr::null_mut();
    }

    if utf8_check_string(strbuff.value, strbuff.length) == 0 {
        set_error(
            s,
            b"<args>\0".as_ptr() as *const c_char,
            json_error_invalid_utf8,
            b"Invalid UTF-8 %s\0".as_ptr() as *const c_char,
            purpose,
        );
        strbuffer_close(&mut strbuff);
        (*s).has_error = 1;
        return ptr::null_mut();
    }

    *out_len = strbuff.length;
    *ours = 1;
    strbuffer_steal_value(&mut strbuff)
}

unsafe fn pack_object(s: *mut scanner_t, ap: &mut VaList) -> *mut json_t {
    let object = json_object();
    next_token(s);

    while token(s) != '}' as c_char {
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
            return ptr::null_mut();
        }

        if token(s) != 's' as c_char {
            set_error(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                b"Expected format 's', got '%c'\0".as_ptr() as *const c_char,
                token(s) as c_int,
            );
            json_decref(object);
            return ptr::null_mut();
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

            if value_optional != '*' as c_char {
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
            set_error(
                s,
                b"<internal>\0".as_ptr() as *const c_char,
                json_error_out_of_memory,
                b"Unable to add key \"%s\"\0".as_ptr() as *const c_char,
                key,
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
    ptr::null_mut()
}

unsafe fn pack_array(s: *mut scanner_t, ap: &mut VaList) -> *mut json_t {
    let array = json_array();
    next_token(s);

    while token(s) != ']' as c_char {
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
            return ptr::null_mut();
        }

        next_token(s);
        value_optional = token(s);
        prev_token(s);

        value = pack(s, ap);
        if value.is_null() {
            if value_optional != '*' as c_char {
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
    ptr::null_mut()
}

unsafe fn pack_string(s: *mut scanner_t, ap: &mut VaList) -> *mut json_t {
    let str: *mut c_char;
    let t: c_char;
    let mut len: usize = 0;
    let mut ours: c_int = 0;
    let optional: c_int;

    next_token(s);
    t = token(s);
    optional = (t == '?' as c_char || t == '*' as c_char) as c_int;
    if optional == 0 {
        prev_token(s);
    }

    str = read_string(
        s,
        ap,
        b"string\0".as_ptr() as *const c_char,
        &mut len,
        &mut ours,
        optional,
    );

    if str.is_null() {
        return if t == '?' as c_char && (*s).has_error == 0 {
            json_null()
        } else {
            ptr::null_mut()
        };
    }

    if (*s).has_error != 0 {
        /* It's impossible to reach this point if ours != 0, do not free str. */
        return ptr::null_mut();
    }

    if ours != 0 {
        return jsonp_stringn_nocheck_own(str, len);
    }

    json_stringn_nocheck(str, len)
}

unsafe fn pack_object_inter(s: *mut scanner_t, ap: &mut VaList, need_incref: c_int) -> *mut json_t {
    let json: *mut json_t;
    let ntoken: c_char;

    next_token(s);
    ntoken = token(s);

    if ntoken != '?' as c_char && ntoken != '*' as c_char {
        prev_token(s);
    }

    json = ap.arg::<*mut json_t>();

    if !json.is_null() {
        return if need_incref != 0 { json_incref(json) } else { json };
    }

    match ntoken as u8 as char {
        '?' => return json_null(),
        '*' => return ptr::null_mut(),
        _ => {}
    }

    set_error(
        s,
        b"<args>\0".as_ptr() as *const c_char,
        json_error_null_value,
        b"NULL object\0".as_ptr() as *const c_char,
    );
    (*s).has_error = 1;
    ptr::null_mut()
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

        return ptr::null_mut();
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

        return ptr::null_mut();
    }

    json
}

unsafe fn pack(s: *mut scanner_t, ap: &mut VaList) -> *mut json_t {
    match token(s) as u8 as char {
        '{' => pack_object(s, ap),
        '[' => pack_array(s, ap),
        's' => pack_string(s, ap),
        'n' => json_null(),
        'b' => {
            if ap.arg::<c_int>() != 0 {
                json_true()
            } else {
                json_false()
            }
        }
        'i' => pack_integer(s, ap.arg::<c_int>() as json_int_t),
        'I' => pack_integer(s, ap.arg::<json_int_t>()),
        'f' => pack_real(s, ap.arg::<c_double>()),
        'O' => pack_object_inter(s, ap, 1),
        'o' => pack_object_inter(s, ap, 0),
        _ => {
            set_error(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                b"Unexpected format character '%c'\0".as_ptr() as *const c_char,
                token(s) as c_int,
            );
            (*s).has_error = 1;
            ptr::null_mut()
        }
    }
}

unsafe fn unpack_object(s: *mut scanner_t, root: *mut json_t, ap: &mut VaList) -> c_int {
    let mut ret: c_int = -1;
    let mut strict: c_int = 0;
    let mut gotopt: c_int = 0;

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

    if !root.is_null() && !json_is_object(root) {
        set_error(
            s,
            b"<validation>\0".as_ptr() as *const c_char,
            json_error_wrong_type,
            b"Expected object, got %s\0".as_ptr() as *const c_char,
            type_name(root),
        );
        hashtable_close(&mut key_set);
        return ret;
    }
    next_token(s);

    while token(s) != '}' as c_char {
        let key: *const c_char;
        let key_len: usize;
        let value: *mut json_t;
        let mut opt: c_int = 0;

        if strict != 0 {
            set_error(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                b"Expected '}' after '%c', got '%c'\0".as_ptr() as *const c_char,
                (if strict == 1 { '!' } else { '*' }) as c_int,
                token(s) as c_int,
            );
            hashtable_close(&mut key_set);
            return ret;
        }

        if token(s) == 0 {
            set_error(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                b"Unexpected end of format string\0".as_ptr() as *const c_char,
            );
            hashtable_close(&mut key_set);
            return ret;
        }

        if token(s) == '!' as c_char || token(s) == '*' as c_char {
            strict = if token(s) == '!' as c_char { 1 } else { -1 };
            next_token(s);
            continue;
        }

        if token(s) != 's' as c_char {
            set_error(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                b"Expected format 's', got '%c'\0".as_ptr() as *const c_char,
                token(s) as c_int,
            );
            hashtable_close(&mut key_set);
            return ret;
        }

        key = ap.arg::<*const c_char>();
        if key.is_null() {
            set_error(
                s,
                b"<args>\0".as_ptr() as *const c_char,
                json_error_null_value,
                b"NULL object key\0".as_ptr() as *const c_char,
            );
            hashtable_close(&mut key_set);
            return ret;
        }
        key_len = strlen(key);

        next_token(s);

        if token(s) == '?' as c_char {
            opt = 1;
            gotopt = 1;
            next_token(s);
        }

        if root.is_null() {
            /* skipping */
            value = ptr::null_mut();
        } else {
            value = json_object_getn(root, key, key_len);
            if value.is_null() && opt == 0 {
                set_error(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_item_not_found,
                    b"Object item not found: %s\0".as_ptr() as *const c_char,
                    key,
                );
                hashtable_close(&mut key_set);
                return ret;
            }
        }

        if unpack(s, value, ap) != 0 {
            hashtable_close(&mut key_set);
            return ret;
        }

        hashtable_set(&mut key_set, key, key_len, json_null());
        next_token(s);
    }

    if strict == 0 && (*s).flags & JSON_STRICT != 0 {
        strict = 1;
    }

    if !root.is_null() && strict == 1 {
        let mut keys_res: c_int = 1;
        let mut unrecognized_keys: strbuffer_t = core::mem::zeroed();
        let mut unpacked: core::ffi::c_long = 0;

        if gotopt != 0 || json_object_size(root) != key_set.size {
            let mut iter = json_object_iter(root);
            while !iter.is_null() {
                let key = json_object_iter_key(iter);
                let key_len = json_object_iter_key_len(iter);

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
                        keys_res = strbuffer_append_bytes(&mut unrecognized_keys, key, key_len);
                    }
                }
                iter = json_object_iter_next(root, iter);
            }
        }
        if unpacked != 0 {
            set_error(
                s,
                b"<validation>\0".as_ptr() as *const c_char,
                json_error_end_of_input_expected,
                b"%li object item(s) left unpacked: %s\0".as_ptr() as *const c_char,
                unpacked,
                if keys_res != 0 {
                    b"<unknown>\0".as_ptr() as *const c_char
                } else {
                    strbuffer_value(&unrecognized_keys)
                },
            );
            strbuffer_close(&mut unrecognized_keys);
            hashtable_close(&mut key_set);
            return ret;
        }
    }

    ret = 0;

    hashtable_close(&mut key_set);
    ret
}

unsafe fn unpack_array(s: *mut scanner_t, root: *mut json_t, ap: &mut VaList) -> c_int {
    let mut i: usize = 0;
    let mut strict: c_int = 0;

    if !root.is_null() && !json_is_array(root) {
        set_error(
            s,
            b"<validation>\0".as_ptr() as *const c_char,
            json_error_wrong_type,
            b"Expected array, got %s\0".as_ptr() as *const c_char,
            type_name(root),
        );
        return -1;
    }
    next_token(s);

    while token(s) != ']' as c_char {
        let value: *mut json_t;

        if strict != 0 {
            set_error(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                b"Expected ']' after '%c', got '%c'\0".as_ptr() as *const c_char,
                (if strict == 1 { '!' } else { '*' }) as c_int,
                token(s) as c_int,
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

        if token(s) == '!' as c_char || token(s) == '*' as c_char {
            strict = if token(s) == '!' as c_char { 1 } else { -1 };
            next_token(s);
            continue;
        }

        if strchr(
            unpack_value_starters.as_ptr() as *const c_char,
            token(s) as c_int,
        )
        .is_null()
        {
            set_error(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                b"Unexpected format character '%c'\0".as_ptr() as *const c_char,
                token(s) as c_int,
            );
            return -1;
        }

        if root.is_null() {
            /* skipping */
            value = ptr::null_mut();
        } else {
            value = json_array_get(root, i);
            if value.is_null() {
                set_error(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_index_out_of_range,
                    b"Array index %lu out of range\0".as_ptr() as *const c_char,
                    i as core::ffi::c_ulong,
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

    if strict == 0 && (*s).flags & JSON_STRICT != 0 {
        strict = 1;
    }

    if !root.is_null() && strict == 1 && i != json_array_size(root) {
        let diff = json_array_size(root) as core::ffi::c_long - i as core::ffi::c_long;
        set_error(
            s,
            b"<validation>\0".as_ptr() as *const c_char,
            json_error_end_of_input_expected,
            b"%li array item(s) left unpacked\0".as_ptr() as *const c_char,
            diff,
        );
        return -1;
    }

    0
}

unsafe fn unpack(s: *mut scanner_t, root: *mut json_t, ap: &mut VaList) -> c_int {
    match token(s) as u8 as char {
        '{' => unpack_object(s, root, ap),
        '[' => unpack_array(s, root, ap),
        's' => {
            if !root.is_null() && !json_is_string(root) {
                set_error(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_wrong_type,
                    b"Expected string, got %s\0".as_ptr() as *const c_char,
                    type_name(root),
                );
                return -1;
            }

            if (*s).flags & JSON_VALIDATE_ONLY == 0 {
                let str_target: *mut *const c_char;
                let mut len_target: *mut usize = ptr::null_mut();

                str_target = ap.arg::<*mut *const c_char>();
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

                if token(s) == '%' as c_char {
                    len_target = ap.arg::<*mut usize>();
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

        'i' => {
            if !root.is_null() && !json_is_integer(root) {
                set_error(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_wrong_type,
                    b"Expected integer, got %s\0".as_ptr() as *const c_char,
                    type_name(root),
                );
                return -1;
            }

            if (*s).flags & JSON_VALIDATE_ONLY == 0 {
                let target = ap.arg::<*mut c_int>();
                if !root.is_null() {
                    *target = json_integer_value(root) as c_int;
                }
            }

            0
        }

        'I' => {
            if !root.is_null() && !json_is_integer(root) {
                set_error(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_wrong_type,
                    b"Expected integer, got %s\0".as_ptr() as *const c_char,
                    type_name(root),
                );
                return -1;
            }

            if (*s).flags & JSON_VALIDATE_ONLY == 0 {
                let target = ap.arg::<*mut json_int_t>();
                if !root.is_null() {
                    *target = json_integer_value(root);
                }
            }

            0
        }

        'b' => {
            if !root.is_null() && !json_is_boolean(root) {
                set_error(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_wrong_type,
                    b"Expected true or false, got %s\0".as_ptr() as *const c_char,
                    type_name(root),
                );
                return -1;
            }

            if (*s).flags & JSON_VALIDATE_ONLY == 0 {
                let target = ap.arg::<*mut c_int>();
                if !root.is_null() {
                    *target = json_is_true(root) as c_int;
                }
            }

            0
        }

        'f' => {
            if !root.is_null() && !json_is_real(root) {
                set_error(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_wrong_type,
                    b"Expected real, got %s\0".as_ptr() as *const c_char,
                    type_name(root),
                );
                return -1;
            }

            if (*s).flags & JSON_VALIDATE_ONLY == 0 {
                let target = ap.arg::<*mut c_double>();
                if !root.is_null() {
                    *target = json_real_value(root);
                }
            }

            0
        }

        'F' => {
            if !root.is_null() && !json_is_number(root) {
                set_error(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_wrong_type,
                    b"Expected real or integer, got %s\0".as_ptr() as *const c_char,
                    type_name(root),
                );
                return -1;
            }

            if (*s).flags & JSON_VALIDATE_ONLY == 0 {
                let target = ap.arg::<*mut c_double>();
                if !root.is_null() {
                    *target = json_number_value(root);
                }
            }

            0
        }

        'O' => {
            if !root.is_null() && (*s).flags & JSON_VALIDATE_ONLY == 0 {
                json_incref(root);
            }
            /* Fall through */
            if (*s).flags & JSON_VALIDATE_ONLY == 0 {
                let target = ap.arg::<*mut *mut json_t>();
                if !root.is_null() {
                    *target = root;
                }
            }

            0
        }

        'o' => {
            if (*s).flags & JSON_VALIDATE_ONLY == 0 {
                let target = ap.arg::<*mut *mut json_t>();
                if !root.is_null() {
                    *target = root;
                }
            }

            0
        }

        'n' => {
            /* Never assign, just validate */
            if !root.is_null() && !json_is_null(root) {
                set_error(
                    s,
                    b"<validation>\0".as_ptr() as *const c_char,
                    json_error_wrong_type,
                    b"Expected null, got %s\0".as_ptr() as *const c_char,
                    type_name(root),
                );
                return -1;
            }
            0
        }

        _ => {
            set_error(
                s,
                b"<format>\0".as_ptr() as *const c_char,
                json_error_invalid_format,
                b"Unexpected format character '%c'\0".as_ptr() as *const c_char,
                token(s) as c_int,
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
    ap: VaList,
) -> *mut json_t {
    let mut s: scanner_t = core::mem::zeroed();
    let value: *mut json_t;

    if fmt.is_null() || *fmt == 0 {
        jsonp_error_init(error, b"<format>\0".as_ptr() as *const c_char);
        jsonp_error_set(
            error,
            -1,
            -1,
            0,
            json_error_invalid_argument,
            b"NULL or empty format string\0".as_ptr() as *const c_char,
        );
        return ptr::null_mut();
    }
    jsonp_error_init(error, ptr::null());

    scanner_init(&mut s, error, flags, fmt);
    next_token(&mut s);

    let mut ap_copy = ap.clone();
    value = pack(&mut s, &mut ap_copy);
    drop(ap_copy);

    /* This will cover all situations where s.has_error is true */
    if value.is_null() {
        return ptr::null_mut();
    }

    next_token(&mut s);
    if token(&mut s) != 0 {
        json_decref(value);
        set_error(
            &mut s,
            b"<format>\0".as_ptr() as *const c_char,
            json_error_invalid_format,
            b"Garbage after format string\0".as_ptr() as *const c_char,
        );
        return ptr::null_mut();
    }

    value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_pack_ex(
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
    ap: ...
) -> *mut json_t {
    json_vpack_ex(error, flags, fmt, ap)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_pack(fmt: *const c_char, ap: ...) -> *mut json_t {
    json_vpack_ex(ptr::null_mut(), 0, fmt, ap)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_vunpack_ex(
    root: *mut json_t,
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
    ap: VaList,
) -> c_int {
    let mut s: scanner_t = core::mem::zeroed();

    if root.is_null() {
        jsonp_error_init(error, b"<root>\0".as_ptr() as *const c_char);
        jsonp_error_set(
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
        jsonp_error_set(
            error,
            -1,
            -1,
            0,
            json_error_invalid_argument,
            b"NULL or empty format string\0".as_ptr() as *const c_char,
        );
        return -1;
    }
    jsonp_error_init(error, ptr::null());

    scanner_init(&mut s, error, flags, fmt);
    next_token(&mut s);

    let mut ap_copy = ap.clone();
    if unpack(&mut s, root, &mut ap_copy) != 0 {
        drop(ap_copy);
        return -1;
    }
    drop(ap_copy);

    next_token(&mut s);
    if token(&mut s) != 0 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_unpack_ex(
    root: *mut json_t,
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
    ap: ...
) -> c_int {
    json_vunpack_ex(root, error, flags, fmt, ap)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_unpack(root: *mut json_t, fmt: *const c_char, ap: ...) -> c_int {
    json_vunpack_ex(root, ptr::null_mut(), 0, fmt, ap)
}
