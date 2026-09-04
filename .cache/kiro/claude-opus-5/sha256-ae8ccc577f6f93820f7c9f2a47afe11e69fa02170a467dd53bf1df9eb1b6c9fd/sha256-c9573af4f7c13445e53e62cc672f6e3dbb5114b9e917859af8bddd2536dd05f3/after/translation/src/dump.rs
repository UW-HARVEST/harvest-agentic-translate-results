//! Translation of `src/dump.c`.

use crate::cffi;
use crate::hashtable::{hashtable_close, hashtable_del, hashtable_init, hashtable_t};
use crate::jtypes::*;
use crate::memory::{jsonp_free, jsonp_malloc, jsonp_realloc};
use crate::strbuffer::{
    strbuffer_append_bytes, strbuffer_close, strbuffer_init, strbuffer_steal_value, strbuffer_t,
};
use crate::strconv::jsonp_dtostr;
use crate::utf::utf8_iterate;
use crate::value::{
    json_array_get, json_array_size, json_integer_value, json_object_getn, json_object_iter,
    json_object_iter_key, json_object_iter_key_len, json_object_iter_next, json_object_iter_value,
    json_object_size, json_real_value, json_string_length, json_string_value, jsonp_loop_check,
};
use core::ffi::{c_char, c_int, c_void};
use core::ptr::{null_mut, addr_of_mut};

const MAX_INTEGER_STR_LENGTH: usize = 25;
const MAX_REAL_STR_LENGTH: usize = 25;

#[inline]
fn flags_to_indent(f: usize) -> usize {
    f & 0x1F
}

#[inline]
fn flags_to_precision(f: usize) -> c_int {
    ((f >> 11) & 0x1F) as c_int
}

#[repr(C)]
struct buffer {
    size: usize,
    used: usize,
    data: *mut c_char,
}

unsafe extern "C" fn dump_to_strbuffer(
    buffer_: *const c_char,
    size: usize,
    data: *mut c_void,
) -> c_int {
    unsafe { strbuffer_append_bytes(data as *mut strbuffer_t, buffer_, size) }
}

unsafe extern "C" fn dump_to_buffer(
    buffer_: *const c_char,
    size: usize,
    data: *mut c_void,
) -> c_int {
    unsafe {
        let buf = data as *mut buffer;

        if (*buf).used + size <= (*buf).size {
            core::ptr::copy_nonoverlapping(
                buffer_ as *const u8,
                ((*buf).data as *mut u8).add((*buf).used),
                size,
            );
        }

        (*buf).used += size;
        0
    }
}

unsafe extern "C" fn dump_to_file(
    buffer_: *const c_char,
    size: usize,
    data: *mut c_void,
) -> c_int {
    unsafe {
        let dest = data as *mut cffi::FILE;
        if cffi::fwrite(buffer_ as *const c_void, size, 1, dest) != 1 {
            return -1;
        }
        0
    }
}

unsafe extern "C" fn dump_to_fd(buffer_: *const c_char, size: usize, data: *mut c_void) -> c_int {
    unsafe {
        let dest = data as *mut c_int;
        if cffi::write(*dest, buffer_ as *const c_void, size) == size as isize {
            return 0;
        }
        -1
    }
}

/* 32 spaces (the maximum indentation size) */
const WHITESPACE: &[u8; 33] = b"                                \0";

unsafe fn dump_indent(
    flags: usize,
    depth: c_int,
    space: c_int,
    dump: json_dump_callback_t,
    data: *mut c_void,
) -> c_int {
    unsafe {
        let dumpf = dump.unwrap();
        if flags_to_indent(flags) > 0 {
            let ws_count = flags_to_indent(flags) as u32;
            let mut n_spaces = (depth as u32).wrapping_mul(ws_count);

            if dumpf(c"\n".as_ptr(), 1, data) != 0 {
                return -1;
            }

            while n_spaces > 0 {
                let cur_n = if (n_spaces as usize) < WHITESPACE.len() - 1 {
                    n_spaces as usize
                } else {
                    WHITESPACE.len() - 1
                };

                if dumpf(WHITESPACE.as_ptr() as *const c_char, cur_n, data) != 0 {
                    return -1;
                }

                n_spaces -= cur_n as u32;
            }
        } else if space != 0 && (flags & JSON_COMPACT) == 0 {
            return dumpf(c" ".as_ptr(), 1, data);
        }
        0
    }
}

unsafe fn dump_string(
    str_in: *const c_char,
    len: usize,
    dump: json_dump_callback_t,
    data: *mut c_void,
    flags: usize,
) -> c_int {
    unsafe {
        let dumpf = dump.unwrap();
        let mut str_ = str_in;
        let mut codepoint: i32 = 0;

        if dumpf(c"\"".as_ptr(), 1, data) != 0 {
            return -1;
        }

        let mut end = str_;
        let mut pos = str_;
        let lim = str_.add(len);
        loop {
            let text: *const c_char;
            let mut seq = [0 as c_char; 13];
            let mut length: usize;

            while end < lim {
                end = utf8_iterate(pos, lim.offset_from(pos) as usize, &mut codepoint);
                if end.is_null() {
                    return -1;
                }

                /* mandatory escape or control char */
                if codepoint == '\\' as i32 || codepoint == '"' as i32 || codepoint < 0x20 {
                    break;
                }

                /* slash */
                if (flags & JSON_ESCAPE_SLASH) != 0 && codepoint == '/' as i32 {
                    break;
                }

                /* non-ASCII */
                if (flags & JSON_ENSURE_ASCII) != 0 && codepoint > 0x7F {
                    break;
                }

                pos = end;
            }

            if pos != str_ {
                if dumpf(str_, pos.offset_from(str_) as usize, data) != 0 {
                    return -1;
                }
            }

            if end == pos {
                break;
            }

            /* handle \, /, ", and control codes */
            length = 2;
            match codepoint {
                0x5C => text = c"\\\\".as_ptr(),  /* '\\' */
                0x22 => text = c"\\\"".as_ptr(),  /* '"'  */
                0x08 => text = c"\\b".as_ptr(),   /* '\b' */
                0x0C => text = c"\\f".as_ptr(),   /* '\f' */
                0x0A => text = c"\\n".as_ptr(),   /* '\n' */
                0x0D => text = c"\\r".as_ptr(),   /* '\r' */
                0x09 => text = c"\\t".as_ptr(),   /* '\t' */
                0x2F => text = c"\\/".as_ptr(),   /* '/'  */
                _ => {
                    /* codepoint is in BMP */
                    if codepoint < 0x10000 {
                        cffi::snprintf(
                            seq.as_mut_ptr(),
                            seq.len(),
                            c"\\u%04X".as_ptr(),
                            codepoint as core::ffi::c_uint,
                        );
                        length = 6;
                    } else {
                        /* not in BMP -> construct a UTF-16 surrogate pair */
                        let cp = codepoint - 0x10000;
                        let first = 0xD800 | ((cp & 0xffc00) >> 10);
                        let last = 0xDC00 | (cp & 0x003ff);

                        cffi::snprintf(
                            seq.as_mut_ptr(),
                            seq.len(),
                            c"\\u%04X\\u%04X".as_ptr(),
                            first as core::ffi::c_uint,
                            last as core::ffi::c_uint,
                        );
                        length = 12;
                    }

                    text = seq.as_ptr();
                }
            }

            if dumpf(text, length, data) != 0 {
                return -1;
            }

            str_ = end;
            pos = end;
        }

        dumpf(c"\"".as_ptr(), 1, data)
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct key_len {
    key: *const c_char,
    len: c_int,
}

unsafe fn compare_keys(k1: &key_len, k2: &key_len) -> c_int {
    unsafe {
        let min_size = if k1.len < k2.len { k1.len } else { k2.len } as usize;
        let res = cffi::c_memcmp(k1.key, k2.key, min_size);

        if res != 0 {
            return res;
        }

        k1.len - k2.len
    }
}

unsafe fn do_dump(
    json: *const json_t,
    flags_in: usize,
    depth: c_int,
    parents: *mut hashtable_t,
    dump: json_dump_callback_t,
    data: *mut c_void,
) -> c_int {
    unsafe {
        let embed = (flags_in & JSON_EMBED) != 0;
        let flags = flags_in & !JSON_EMBED;

        if json.is_null() {
            return -1;
        }

        let ty = json_typeof(json);
        // The C `switch` has a `default: return -1` arm for type tags outside
        // JSON_OBJECT..JSON_NULL, and it never touches `dump` on that path.
        // Hoisting the check keeps a NULL callback from being dereferenced
        // exactly where the C would not dereference it either.
        if ty < JSON_OBJECT || ty > JSON_NULL {
            return -1;
        }
        let dumpf = dump.unwrap();

        match ty {
            JSON_NULL => dumpf(c"null".as_ptr(), 4, data),

            JSON_TRUE => dumpf(c"true".as_ptr(), 4, data),

            JSON_FALSE => dumpf(c"false".as_ptr(), 5, data),

            JSON_INTEGER => {
                let mut buf = [0 as c_char; MAX_INTEGER_STR_LENGTH];

                let size = cffi::snprintf(
                    buf.as_mut_ptr(),
                    MAX_INTEGER_STR_LENGTH,
                    c"%lld".as_ptr(),
                    json_integer_value(json),
                );
                if size < 0 || size >= MAX_INTEGER_STR_LENGTH as c_int {
                    return -1;
                }

                dumpf(buf.as_ptr(), size as usize, data)
            }

            JSON_REAL => {
                let mut buf = [0 as c_char; MAX_REAL_STR_LENGTH];
                let value = json_real_value(json);

                let size = jsonp_dtostr(
                    buf.as_mut_ptr(),
                    MAX_REAL_STR_LENGTH,
                    value,
                    flags_to_precision(flags),
                );
                if size < 0 {
                    return -1;
                }

                dumpf(buf.as_ptr(), size as usize, data)
            }

            JSON_STRING => dump_string(
                json_string_value(json),
                json_string_length(json),
                dump,
                data,
                flags,
            ),

            JSON_ARRAY => {
                /* Space for "0x", double the sizeof a pointer for the hex and a
                 * terminator. */
                let mut key = [0 as c_char; LOOP_KEY_LEN];
                let mut key_len: usize = 0;

                /* detect circular references */
                if jsonp_loop_check(parents, json, key.as_mut_ptr(), LOOP_KEY_LEN, &mut key_len)
                    != 0
                {
                    return -1;
                }

                let n = json_array_size(json);

                if !embed && dumpf(c"[".as_ptr(), 1, data) != 0 {
                    return -1;
                }
                if n == 0 {
                    hashtable_del(parents, key.as_ptr(), key_len);
                    return if embed { 0 } else { dumpf(c"]".as_ptr(), 1, data) };
                }
                if dump_indent(flags, depth + 1, 0, dump, data) != 0 {
                    return -1;
                }

                let mut i: usize = 0;
                while i < n - 1 {
                    if do_dump(json_array_get(json, i), flags, depth + 1, parents, dump, data) != 0
                    {
                        return -1;
                    }

                    if dumpf(c",".as_ptr(), 1, data) != 0
                        || dump_indent(flags, depth + 1, 1, dump, data) != 0
                    {
                        return -1;
                    }
                    i += 1;
                }

                if do_dump(json_array_get(json, i), flags, depth + 1, parents, dump, data) != 0 {
                    return -1;
                }
                if dump_indent(flags, depth, 0, dump, data) != 0 {
                    return -1;
                }

                hashtable_del(parents, key.as_ptr(), key_len);
                if embed { 0 } else { dumpf(c"]".as_ptr(), 1, data) }
            }

            JSON_OBJECT => {
                let separator: *const c_char;
                let separator_length: usize;
                let mut loop_key = [0 as c_char; LOOP_KEY_LEN];
                let mut loop_key_len: usize = 0;

                if (flags & JSON_COMPACT) != 0 {
                    separator = c":".as_ptr();
                    separator_length = 1;
                } else {
                    separator = c": ".as_ptr();
                    separator_length = 2;
                }

                /* detect circular references */
                if jsonp_loop_check(
                    parents,
                    json,
                    loop_key.as_mut_ptr(),
                    LOOP_KEY_LEN,
                    &mut loop_key_len,
                ) != 0
                {
                    return -1;
                }

                if !embed && dumpf(c"{".as_ptr(), 1, data) != 0 {
                    return -1;
                }

                let mut iter = json_object_iter(json as *mut json_t);
                if iter.is_null() {
                    hashtable_del(parents, loop_key.as_ptr(), loop_key_len);
                    return if embed { 0 } else { dumpf(c"}".as_ptr(), 1, data) };
                }
                if dump_indent(flags, depth + 1, 0, dump, data) != 0 {
                    return -1;
                }

                if (flags & JSON_SORT_KEYS) != 0 {
                    let size = json_object_size(json);
                    let keys =
                        jsonp_malloc(size * core::mem::size_of::<key_len>()) as *mut key_len;
                    if keys.is_null() {
                        return -1;
                    }

                    let mut i: usize = 0;
                    while !iter.is_null() {
                        let keylen = keys.add(i);

                        (*keylen).key = json_object_iter_key(iter);
                        (*keylen).len = json_object_iter_key_len(iter) as c_int;

                        iter = json_object_iter_next(json as *mut json_t, iter);
                        i += 1;
                    }

                    let slice = core::slice::from_raw_parts_mut(keys, size);
                    slice.sort_unstable_by(|a, b| {
                        let r = compare_keys(a, b);
                        if r < 0 {
                            core::cmp::Ordering::Less
                        } else if r > 0 {
                            core::cmp::Ordering::Greater
                        } else {
                            core::cmp::Ordering::Equal
                        }
                    });

                    let mut i: usize = 0;
                    while i < size {
                        let key = keys.add(i);
                        let value = json_object_getn(json, (*key).key, (*key).len as usize);

                        dump_string((*key).key, (*key).len as usize, dump, data, flags);
                        if dumpf(separator, separator_length, data) != 0
                            || do_dump(value, flags, depth + 1, parents, dump, data) != 0
                        {
                            jsonp_free(keys as *mut c_void);
                            return -1;
                        }

                        if i < size - 1 {
                            if dumpf(c",".as_ptr(), 1, data) != 0
                                || dump_indent(flags, depth + 1, 1, dump, data) != 0
                            {
                                jsonp_free(keys as *mut c_void);
                                return -1;
                            }
                        } else {
                            if dump_indent(flags, depth, 0, dump, data) != 0 {
                                jsonp_free(keys as *mut c_void);
                                return -1;
                            }
                        }
                        i += 1;
                    }

                    jsonp_free(keys as *mut c_void);
                } else {
                    /* Don't sort keys */

                    while !iter.is_null() {
                        let next = json_object_iter_next(json as *mut json_t, iter);
                        let key = json_object_iter_key(iter);
                        let key_len = json_object_iter_key_len(iter);

                        dump_string(key, key_len, dump, data, flags);
                        if dumpf(separator, separator_length, data) != 0
                            || do_dump(
                                json_object_iter_value(iter),
                                flags,
                                depth + 1,
                                parents,
                                dump,
                                data,
                            ) != 0
                        {
                            return -1;
                        }

                        if !next.is_null() {
                            if dumpf(c",".as_ptr(), 1, data) != 0
                                || dump_indent(flags, depth + 1, 1, dump, data) != 0
                            {
                                return -1;
                            }
                        } else {
                            if dump_indent(flags, depth, 0, dump, data) != 0 {
                                return -1;
                            }
                        }

                        iter = next;
                    }
                }

                hashtable_del(parents, loop_key.as_ptr(), loop_key_len);
                if embed { 0 } else { dumpf(c"}".as_ptr(), 1, data) }
            }

            _ => -1,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumps(json: *const json_t, flags: usize) -> *mut c_char {
    unsafe {
        let mut strbuff = strbuffer_t::new();
        let mut result: *mut c_char;

        if strbuffer_init(&mut strbuff) != 0 {
            return null_mut();
        }

        if json_dump_callback(
            json,
            Some(dump_to_strbuffer),
            &mut strbuff as *mut strbuffer_t as *mut c_void,
            flags,
        ) != 0
        {
            result = null_mut();
        } else {
            result = strbuffer_steal_value(&mut strbuff);
            // technically the resizing is not needed.
            let new_result = jsonp_realloc(
                result as *mut c_void,
                strbuff.size,
                strbuff.length + 1,
            ) as *mut c_char;
            if !new_result.is_null() {
                // when realloc fails we just use the original pointer
                result = new_result;
            }
        }

        strbuffer_close(&mut strbuff);
        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumpb(
    json: *const json_t,
    buffer_: *mut c_char,
    size: usize,
    flags: usize,
) -> usize {
    unsafe {
        let mut buf = buffer {
            size,
            used: 0,
            data: buffer_,
        };

        if json_dump_callback(
            json,
            Some(dump_to_buffer),
            &mut buf as *mut buffer as *mut c_void,
            flags,
        ) != 0
        {
            return 0;
        }

        buf.used
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumpf(
    json: *const json_t,
    output: *mut cffi::FILE,
    flags: usize,
) -> c_int {
    unsafe { json_dump_callback(json, Some(dump_to_file), output as *mut c_void, flags) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumpfd(json: *const json_t, output: c_int, flags: usize) -> c_int {
    unsafe {
        let mut out = output;
        json_dump_callback(
            json,
            Some(dump_to_fd),
            &mut out as *mut c_int as *mut c_void,
            flags,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dump_file(
    json: *const json_t,
    path: *const c_char,
    flags: usize,
) -> c_int {
    unsafe {
        let output = cffi::fopen(path, c"w".as_ptr());
        if output.is_null() {
            return -1;
        }

        let result = json_dumpf(json, output, flags);

        if cffi::fclose(output) != 0 {
            return -1;
        }

        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dump_callback(
    json: *const json_t,
    callback: json_dump_callback_t,
    data: *mut c_void,
    flags: usize,
) -> c_int {
    unsafe {
        let mut parents_set = hashtable_t::new();

        if (flags & JSON_ENCODE_ANY) == 0 {
            if !json_is_array(json) && !json_is_object(json) {
                return -1;
            }
        }

        if hashtable_init(addr_of_mut!(parents_set)) != 0 {
            return -1;
        }
        let res = do_dump(json, flags, 0, addr_of_mut!(parents_set), callback, data);
        hashtable_close(addr_of_mut!(parents_set));

        res
    }
}
