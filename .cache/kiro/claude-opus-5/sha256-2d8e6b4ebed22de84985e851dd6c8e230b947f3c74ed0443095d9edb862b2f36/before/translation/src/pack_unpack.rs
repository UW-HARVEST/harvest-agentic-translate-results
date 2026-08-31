//! Translation of `src/pack_unpack.c`.

use crate::cfmt::CBuf;
use crate::error::*;
use crate::hashtable::*;
use crate::memory::*;
use crate::strbuffer::*;
use crate::types::*;
use crate::utf::utf8_check_string;
use crate::value::*;
use crate::varargs::{arg_double, arg_i64, arg_int, arg_ptr, arg_size, va_copy, VaListTag};
use core::ffi::{c_char, c_int, c_void};

#[derive(Copy, Clone)]
struct TokenT {
    line: c_int,
    column: c_int,
    pos: usize,
    token: c_char,
}

impl TokenT {
    const fn zeroed() -> TokenT {
        TokenT {
            line: 0,
            column: 0,
            pos: 0,
            token: 0,
        }
    }
}

struct ScannerT {
    #[allow(dead_code)]
    start: *const c_char,
    fmt: *const c_char,
    prev_token: TokenT,
    token: TokenT,
    next_token: TokenT,
    error: *mut JsonErrorT,
    flags: usize,
    line: c_int,
    column: c_int,
    pos: usize,
    has_error: c_int,
}

#[inline]
unsafe fn token(s: *const ScannerT) -> c_char {
    (*s).token.token
}

static TYPE_NAMES: [&[u8]; 8] = [
    b"object", b"array", b"string", b"integer", b"real", b"true", b"false", b"null",
];

#[inline]
unsafe fn type_name(x: *const JsonT) -> &'static [u8] {
    TYPE_NAMES[json_typeof(x) as usize]
}

static UNPACK_VALUE_STARTERS: &[u8] = b"{[siIbfFOon";

unsafe fn scanner_init(
    s: *mut ScannerT,
    error: *mut JsonErrorT,
    flags: usize,
    fmt: *const c_char,
) {
    (*s).error = error;
    (*s).flags = flags;
    (*s).start = fmt;
    (*s).fmt = fmt;
    (*s).prev_token = TokenT::zeroed();
    (*s).token = TokenT::zeroed();
    (*s).next_token = TokenT::zeroed();
    (*s).line = 1;
    (*s).column = 0;
    (*s).pos = 0;
    (*s).has_error = 0;
}

unsafe fn next_token(s: *mut ScannerT) {
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

unsafe fn prev_token(s: *mut ScannerT) {
    (*s).next_token = (*s).token;
    (*s).token = (*s).prev_token;
}

unsafe fn set_error(s: *mut ScannerT, source: &[u8], code: c_int, text: &[u8]) {
    jsonp_error_set_bytes(
        (*s).error,
        (*s).token.line,
        (*s).token.column,
        (*s).token.pos,
        code,
        text,
    );

    jsonp_error_set_source((*s).error, source.as_ptr() as *const c_char);
}

/* ours will be set to 1 if jsonp_free() must be called for the result
afterwards */
unsafe fn read_string(
    s: *mut ScannerT,
    ap: *mut VaListTag,
    purpose: &[u8],
    out_len: *mut usize,
    ours: *mut c_int,
    optional: c_int,
) -> *mut c_char {
    let t: c_char;
    let mut strbuff: StrbufferT = core::mem::zeroed();
    let mut str_: *const c_char;
    let mut length: usize;

    next_token(s);
    t = token(s);
    prev_token(s);

    *ours = 0;
    if t != b'#' as c_char && t != b'%' as c_char && t != b'+' as c_char {
        /* Optimize the simple case */
        str_ = arg_ptr::<c_char>(ap);

        if str_.is_null() {
            if optional == 0 {
                let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
                b.push(b"NULL ");
                b.push(purpose);
                set_error(s, b"<args>\0", JSON_ERROR_NULL_VALUE, b.as_bytes());
                (*s).has_error = 1;
            }
            return core::ptr::null_mut();
        }

        length = strlen(str_);

        if utf8_check_string(str_, length) == 0 {
            let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
            b.push(b"Invalid UTF-8 ");
            b.push(purpose);
            set_error(s, b"<args>\0", JSON_ERROR_INVALID_UTF8, b.as_bytes());
            (*s).has_error = 1;
            return core::ptr::null_mut();
        }

        *out_len = length;
        return str_ as *mut c_char;
    } else if optional != 0 {
        let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
        b.push(b"Cannot use '");
        b.push_char(t as u8);
        b.push(b"' on optional strings");
        set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, b.as_bytes());
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
        str_ = arg_ptr::<c_char>(ap);
        if str_.is_null() {
            let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
            b.push(b"NULL ");
            b.push(purpose);
            set_error(s, b"<args>\0", JSON_ERROR_NULL_VALUE, b.as_bytes());
            (*s).has_error = 1;
        }

        next_token(s);

        if token(s) == b'#' as c_char {
            length = arg_int(ap) as usize;
        } else if token(s) == b'%' as c_char {
            length = arg_size(ap);
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
        let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
        b.push(b"Invalid UTF-8 ");
        b.push(purpose);
        set_error(s, b"<args>\0", JSON_ERROR_INVALID_UTF8, b.as_bytes());
        strbuffer_close(&mut strbuff);
        (*s).has_error = 1;
        return core::ptr::null_mut();
    }

    *out_len = strbuff.length;
    *ours = 1;
    strbuffer_steal_value(&mut strbuff)
}

unsafe fn pack_object(s: *mut ScannerT, ap: *mut VaListTag) -> *mut JsonT {
    let object = json_object();
    next_token(s);

    while token(s) != b'}' as c_char {
        let key: *mut c_char;
        let mut len: usize = 0;
        let mut ours: c_int = 0;
        let value: *mut JsonT;
        let value_optional: c_char;

        if token(s) == 0 {
            set_error(
                s,
                b"<format>\0",
                JSON_ERROR_INVALID_FORMAT,
                b"Unexpected end of format string",
            );
            json_decref(object);
            return core::ptr::null_mut();
        }

        if token(s) != b's' as c_char {
            let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
            b.push(b"Expected format 's', got '");
            b.push_char(token(s) as u8);
            b.push(b"'");
            set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, b.as_bytes());
            json_decref(object);
            return core::ptr::null_mut();
        }

        key = read_string(s, ap, b"object key", &mut len, &mut ours, 0);

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
            let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
            b.push(b"Unable to add key \"");
            b.push_cstr(key);
            b.push(b"\"");
            set_error(s, b"<internal>\0", JSON_ERROR_OUT_OF_MEMORY, b.as_bytes());
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

unsafe fn pack_array(s: *mut ScannerT, ap: *mut VaListTag) -> *mut JsonT {
    let array = json_array();
    next_token(s);

    while token(s) != b']' as c_char {
        let value: *mut JsonT;
        let value_optional: c_char;

        if token(s) == 0 {
            set_error(
                s,
                b"<format>\0",
                JSON_ERROR_INVALID_FORMAT,
                b"Unexpected end of format string",
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

    json_decref(array);
    core::ptr::null_mut()
}

unsafe fn pack_string(s: *mut ScannerT, ap: *mut VaListTag) -> *mut JsonT {
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
    s: *mut ScannerT,
    ap: *mut VaListTag,
    need_incref: c_int,
) -> *mut JsonT {
    let json: *mut JsonT;
    let ntoken: c_char;

    next_token(s);
    ntoken = token(s);

    if ntoken != b'?' as c_char && ntoken != b'*' as c_char {
        prev_token(s);
    }

    json = arg_ptr::<JsonT>(ap);

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

unsafe fn pack_integer(s: *mut ScannerT, value: JsonIntT) -> *mut JsonT {
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

unsafe fn pack_real(s: *mut ScannerT, value: f64) -> *mut JsonT {
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

unsafe fn pack(s: *mut ScannerT, ap: *mut VaListTag) -> *mut JsonT {
    match token(s) as u8 {
        b'{' => pack_object(s, ap),

        b'[' => pack_array(s, ap),

        b's' => pack_string(s, ap), /* string */

        b'n' => json_null(), /* null */

        b'b' => {
            /* boolean */
            if arg_int(ap) != 0 {
                json_true()
            } else {
                json_false()
            }
        }

        b'i' => pack_integer(s, arg_int(ap) as JsonIntT), /* integer from int */

        b'I' => pack_integer(s, arg_i64(ap)), /* integer from json_int_t */

        b'f' => pack_real(s, arg_double(ap)), /* real */

        b'O' => pack_object_inter(s, ap, 1), /* increments refcount */

        b'o' => pack_object_inter(s, ap, 0), /* doesn't increment refcount */

        _ => {
            let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
            b.push(b"Unexpected format character '");
            b.push_char(token(s) as u8);
            b.push(b"'");
            set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, b.as_bytes());
            (*s).has_error = 1;
            core::ptr::null_mut()
        }
    }
}

unsafe fn unpack_object(s: *mut ScannerT, root: *mut JsonT, ap: *mut VaListTag) -> c_int {
    let mut ret: c_int = -1;
    let mut strict: c_int = 0;
    let mut gotopt: c_int = 0;

    /* Use a set (emulated by a hashtable) to check that all object
    keys are accessed. */
    let mut key_set: HashtableT = core::mem::zeroed();

    if hashtable_init(&mut key_set) != 0 {
        set_error(
            s,
            b"<internal>\0",
            JSON_ERROR_OUT_OF_MEMORY,
            b"Out of memory",
        );
        return -1;
    }

    'out: {
        if !root.is_null() && !json_is_object(root) {
            let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
            b.push(b"Expected object, got ");
            b.push(type_name(root));
            set_error(s, b"<validation>\0", JSON_ERROR_WRONG_TYPE, b.as_bytes());
            break 'out;
        }
        next_token(s);

        while token(s) != b'}' as c_char {
            let key: *const c_char;
            let key_len: usize;
            let value: *mut JsonT;
            let mut opt: c_int = 0;

            if strict != 0 {
                let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
                b.push(b"Expected '}' after '");
                b.push_char(if strict == 1 { b'!' } else { b'*' });
                b.push(b"', got '");
                b.push_char(token(s) as u8);
                b.push(b"'");
                set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, b.as_bytes());
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
                let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
                b.push(b"Expected format 's', got '");
                b.push_char(token(s) as u8);
                b.push(b"'");
                set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, b.as_bytes());
                break 'out;
            }

            key = arg_ptr::<c_char>(ap);
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
                    let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
                    b.push(b"Object item not found: ");
                    b.push_cstr(key);
                    set_error(
                        s,
                        b"<validation>\0",
                        JSON_ERROR_ITEM_NOT_FOUND,
                        b.as_bytes(),
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
            let mut unrecognized_keys: StrbufferT = core::mem::zeroed();
            let mut unpacked: i64 = 0;

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
                            keys_res =
                                strbuffer_append_bytes(&mut unrecognized_keys, key, key_len);
                        }
                    }
                    iter = json_object_iter_next(root, iter);
                }
            }
            if unpacked != 0 {
                let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
                b.push_dec_i64(unpacked);
                b.push(b" object item(s) left unpacked: ");
                if keys_res != 0 {
                    b.push(b"<unknown>");
                } else {
                    b.push_cstr(strbuffer_value(&unrecognized_keys));
                }
                set_error(
                    s,
                    b"<validation>\0",
                    JSON_ERROR_END_OF_INPUT_EXPECTED,
                    b.as_bytes(),
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

unsafe fn unpack_array(s: *mut ScannerT, root: *mut JsonT, ap: *mut VaListTag) -> c_int {
    let mut i: usize = 0;
    let mut strict: c_int = 0;

    if !root.is_null() && !json_is_array(root) {
        let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
        b.push(b"Expected array, got ");
        b.push(type_name(root));
        set_error(s, b"<validation>\0", JSON_ERROR_WRONG_TYPE, b.as_bytes());
        return -1;
    }
    next_token(s);

    while token(s) != b']' as c_char {
        let value: *mut JsonT;

        if strict != 0 {
            let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
            b.push(b"Expected ']' after '");
            b.push_char(if strict == 1 { b'!' } else { b'*' });
            b.push(b"', got '");
            b.push_char(token(s) as u8);
            b.push(b"'");
            set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, b.as_bytes());
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

        /* strchr() also matches the terminating NUL of unpack_value_starters */
        if !(UNPACK_VALUE_STARTERS.contains(&(token(s) as u8)) || token(s) == 0) {
            let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
            b.push(b"Unexpected format character '");
            b.push_char(token(s) as u8);
            b.push(b"'");
            set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, b.as_bytes());
            return -1;
        }

        if root.is_null() {
            /* skipping */
            value = core::ptr::null_mut();
        } else {
            value = json_array_get(root, i);
            if value.is_null() {
                let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
                b.push(b"Array index ");
                b.push_dec_u64(i as u64);
                b.push(b" out of range");
                set_error(
                    s,
                    b"<validation>\0",
                    JSON_ERROR_INDEX_OUT_OF_RANGE,
                    b.as_bytes(),
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
        let diff = json_array_size(root) as i64 - i as i64;
        let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
        b.push_dec_i64(diff);
        b.push(b" array item(s) left unpacked");
        set_error(
            s,
            b"<validation>\0",
            JSON_ERROR_END_OF_INPUT_EXPECTED,
            b.as_bytes(),
        );
        return -1;
    }

    0
}

unsafe fn wrong_type(s: *mut ScannerT, expected: &[u8], root: *const JsonT) {
    let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
    b.push(b"Expected ");
    b.push(expected);
    b.push(b", got ");
    b.push(type_name(root));
    set_error(s, b"<validation>\0", JSON_ERROR_WRONG_TYPE, b.as_bytes());
}

unsafe fn unpack(s: *mut ScannerT, root: *mut JsonT, ap: *mut VaListTag) -> c_int {
    match token(s) as u8 {
        b'{' => unpack_object(s, root, ap),

        b'[' => unpack_array(s, root, ap),

        b's' => {
            if !root.is_null() && !json_is_string(root) {
                wrong_type(s, b"string", root);
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let str_target: *mut *const c_char;
                let mut len_target: *mut usize = core::ptr::null_mut();

                str_target = arg_ptr::<*const c_char>(ap);
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
                    len_target = arg_ptr::<usize>(ap);
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
                wrong_type(s, b"integer", root);
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = arg_ptr::<c_int>(ap);
                if !root.is_null() {
                    *target = json_integer_value(root) as c_int;
                }
            }

            0
        }

        b'I' => {
            if !root.is_null() && !json_is_integer(root) {
                wrong_type(s, b"integer", root);
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = arg_ptr::<JsonIntT>(ap);
                if !root.is_null() {
                    *target = json_integer_value(root);
                }
            }

            0
        }

        b'b' => {
            if !root.is_null() && !json_is_boolean(root) {
                wrong_type(s, b"true or false", root);
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = arg_ptr::<c_int>(ap);
                if !root.is_null() {
                    *target = json_is_true(root) as c_int;
                }
            }

            0
        }

        b'f' => {
            if !root.is_null() && !json_is_real(root) {
                wrong_type(s, b"real", root);
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = arg_ptr::<f64>(ap);
                if !root.is_null() {
                    *target = json_real_value(root);
                }
            }

            0
        }

        b'F' => {
            if !root.is_null() && !json_is_number(root) {
                wrong_type(s, b"real or integer", root);
                return -1;
            }

            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = arg_ptr::<f64>(ap);
                if !root.is_null() {
                    *target = json_number_value(root);
                }
            }

            0
        }

        b'O' | b'o' => {
            if token(s) as u8 == b'O' && !root.is_null() && ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                json_incref(root);
            }
            /* Fall through */
            if ((*s).flags & JSON_VALIDATE_ONLY) == 0 {
                let target = arg_ptr::<*mut JsonT>(ap);
                if !root.is_null() {
                    *target = root;
                }
            }

            0
        }

        b'n' => {
            /* Never assign, just validate */
            if !root.is_null() && !json_is_null(root) {
                wrong_type(s, b"null", root);
                return -1;
            }
            0
        }

        _ => {
            let mut b: CBuf<{ JSON_ERROR_TEXT_LENGTH }> = CBuf::new();
            b.push(b"Unexpected format character '");
            b.push_char(token(s) as u8);
            b.push(b"'");
            set_error(s, b"<format>\0", JSON_ERROR_INVALID_FORMAT, b.as_bytes());
            -1
        }
    }
}

unsafe fn new_scanner() -> ScannerT {
    ScannerT {
        start: core::ptr::null(),
        fmt: core::ptr::null(),
        prev_token: TokenT::zeroed(),
        token: TokenT::zeroed(),
        next_token: TokenT::zeroed(),
        error: core::ptr::null_mut(),
        flags: 0,
        line: 0,
        column: 0,
        pos: 0,
        has_error: 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_vpack_ex(
    error: *mut JsonErrorT,
    flags: usize,
    fmt: *const c_char,
    ap: *mut VaListTag,
) -> *mut JsonT {
    let mut s = new_scanner();
    let value: *mut JsonT;

    if fmt.is_null() || *fmt == 0 {
        jsonp_error_init(error, b"<format>\0".as_ptr() as *const c_char);
        jsonp_error_set_bytes(
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

    scanner_init(&mut s, error, flags, fmt);
    next_token(&mut s);

    let mut ap_copy: VaListTag = va_copy(ap);
    value = pack(&mut s, &mut ap_copy);

    /* This will cover all situations where s.has_error is true */
    if value.is_null() {
        return core::ptr::null_mut();
    }

    next_token(&mut s);
    if token(&s) != 0 {
        json_decref(value);
        set_error(
            &mut s,
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
    root: *mut JsonT,
    error: *mut JsonErrorT,
    flags: usize,
    fmt: *const c_char,
    ap: *mut VaListTag,
) -> c_int {
    let mut s = new_scanner();

    if root.is_null() {
        jsonp_error_init(error, b"<root>\0".as_ptr() as *const c_char);
        jsonp_error_set_bytes(error, -1, -1, 0, JSON_ERROR_NULL_VALUE, b"NULL root value");
        return -1;
    }

    if fmt.is_null() || *fmt == 0 {
        jsonp_error_init(error, b"<format>\0".as_ptr() as *const c_char);
        jsonp_error_set_bytes(
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

    scanner_init(&mut s, error, flags, fmt);
    next_token(&mut s);

    let mut ap_copy: VaListTag = va_copy(ap);
    if unpack(&mut s, root, &mut ap_copy) != 0 {
        return -1;
    }

    next_token(&mut s);
    if token(&s) != 0 {
        set_error(
            &mut s,
            b"<format>\0",
            JSON_ERROR_INVALID_FORMAT,
            b"Garbage after format string",
        );
        return -1;
    }

    0
}
