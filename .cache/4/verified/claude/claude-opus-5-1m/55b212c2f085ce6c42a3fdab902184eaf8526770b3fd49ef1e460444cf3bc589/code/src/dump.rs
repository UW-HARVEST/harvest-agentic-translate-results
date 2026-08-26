//! Translation of `src/dump.c`.

use crate::hashtable::{hashtable_close, hashtable_del, hashtable_init};
use crate::memory::{jsonp_free, jsonp_malloc, jsonp_realloc};
use crate::strbuffer::*;
use crate::strconv::jsonp_dtostr;
use crate::types::*;
use crate::utf::utf8_iterate;
use crate::value::*;
use core::ffi::{c_char, c_int, c_void};

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
    strbuffer_append_bytes(data as *mut strbuffer_t, buffer_, size)
}

unsafe extern "C" fn dump_to_buffer(
    buffer_: *const c_char,
    size: usize,
    data: *mut c_void,
) -> c_int {
    let buf = data as *mut buffer;

    if (*buf).used + size <= (*buf).size {
        memcpy(
            (*buf).data.add((*buf).used) as *mut c_void,
            buffer_ as *const c_void,
            size,
        );
    }

    (*buf).used += size;
    0
}

unsafe extern "C" fn dump_to_file(
    buffer_: *const c_char,
    size: usize,
    data: *mut c_void,
) -> c_int {
    let dest = data as *mut FILE;
    if fwrite(buffer_ as *const c_void, size, 1, dest) != 1 {
        return -1;
    }
    0
}

unsafe extern "C" fn dump_to_fd(buffer_: *const c_char, size: usize, data: *mut c_void) -> c_int {
    let dest = data as *mut c_int;
    if write(*dest, buffer_ as *const c_void, size) == size as isize {
        return 0;
    }
    -1
}

/* 32 spaces (the maximum indentation size) */
static WHITESPACE: &[u8; 33] = b"                                \0";

/// Invoke the caller supplied dump callback.
///
/// The C code calls straight through the function pointer at each emission
/// point; it never inspects the pointer up-front.  Reproducing that laziness
/// matters because several paths (`json == NULL`, an out-of-range `json_type`
/// tag, `JSON_EMBED` on an empty container, a failed circular-reference check)
/// return without ever emitting a chunk, and C returns normally there even if
/// the callback is NULL.
#[inline(always)]
unsafe fn dump_call(
    dump: json_dump_callback_t,
    buffer: *const c_char,
    size: usize,
    data: *mut c_void,
) -> c_int {
    match dump {
        Some(f) => f(buffer, size, data),
        // C jumps to address 0 here; fault the same way instead of panicking.
        None => core::ptr::read_volatile(core::ptr::null::<c_int>()),
    }
}

unsafe fn dump_indent(
    flags: usize,
    depth: c_int,
    space: c_int,
    dump: json_dump_callback_t,
    data: *mut c_void,
) -> c_int {
    if flags_to_indent(flags) > 0 {
        let ws_count: u32 = flags_to_indent(flags) as u32;
        let mut n_spaces: u32 = (depth as u32).wrapping_mul(ws_count);

        if dump_call(dump, b"\n\0".as_ptr() as *const c_char, 1, data) != 0 {
            return -1;
        }

        while n_spaces > 0 {
            let cur_n: c_int = if (n_spaces as usize) < 32 {
                n_spaces as c_int
            } else {
                32
            };

            if dump_call(dump, WHITESPACE.as_ptr() as *const c_char, cur_n as usize, data) != 0 {
                return -1;
            }

            n_spaces -= cur_n as u32;
        }
    } else if space != 0 && (flags & JSON_COMPACT) == 0 {
        return dump_call(dump, b" \0".as_ptr() as *const c_char, 1, data);
    }
    0
}

unsafe fn dump_string(
    str_: *const c_char,
    len: usize,
    dump: json_dump_callback_t,
    data: *mut c_void,
    flags: usize,
) -> c_int {
    let mut pos: *const c_char;
    let mut end: *const c_char;
    let lim: *const c_char;
    let mut codepoint: i32 = 0;
    let mut str_ = str_;

    if dump_call(dump, b"\"\0".as_ptr() as *const c_char, 1, data) != 0 {
        return -1;
    }

    pos = str_;
    end = str_;
    lim = str_.add(len);
    loop {
        let text: *const c_char;
        let mut seq = [0i8; 13];
        let mut length: c_int;

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

        if pos != str_ && dump_call(dump, str_, pos.offset_from(str_) as usize, data) != 0 {
            return -1;
        }

        if end == pos {
            break;
        }

        /* handle \, /, ", and control codes */
        length = 2;
        match codepoint {
            0x5c => text = b"\\\\\0".as_ptr() as *const c_char,
            0x22 => text = b"\\\"\0".as_ptr() as *const c_char,
            0x08 => text = b"\\b\0".as_ptr() as *const c_char,
            0x0c => text = b"\\f\0".as_ptr() as *const c_char,
            0x0a => text = b"\\n\0".as_ptr() as *const c_char,
            0x0d => text = b"\\r\0".as_ptr() as *const c_char,
            0x09 => text = b"\\t\0".as_ptr() as *const c_char,
            0x2f => text = b"\\/\0".as_ptr() as *const c_char,
            _ => {
                /* codepoint is in BMP */
                if codepoint < 0x10000 {
                    snprintf(
                        seq.as_mut_ptr(),
                        13,
                        b"\\u%04X\0".as_ptr() as *const c_char,
                        codepoint as u32,
                    );
                    length = 6;
                } else {
                    /* not in BMP -> construct a UTF-16 surrogate pair */
                    let first: i32;
                    let last: i32;

                    codepoint -= 0x10000;
                    first = 0xD800 | ((codepoint & 0xffc00) >> 10);
                    last = 0xDC00 | (codepoint & 0x003ff);

                    snprintf(
                        seq.as_mut_ptr(),
                        13,
                        b"\\u%04X\\u%04X\0".as_ptr() as *const c_char,
                        first as u32,
                        last as u32,
                    );
                    length = 12;
                }

                text = seq.as_ptr();
            }
        }

        if dump_call(dump, text, length as usize, data) != 0 {
            return -1;
        }

        str_ = end;
        pos = end;
    }

    dump_call(dump, b"\"\0".as_ptr() as *const c_char, 1, data)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct key_len {
    key: *const c_char,
    len: c_int,
}

unsafe extern "C" fn compare_keys(key1: *const c_void, key2: *const c_void) -> c_int {
    let k1 = key1 as *const key_len;
    let k2 = key2 as *const key_len;
    let min_size = if (*k1).len < (*k2).len {
        (*k1).len
    } else {
        (*k2).len
    } as usize;
    let res = memcmp(
        (*k1).key as *const c_void,
        (*k2).key as *const c_void,
        min_size,
    );

    if res != 0 {
        return res;
    }

    (*k1).len - (*k2).len
}

unsafe fn do_dump(
    json: *const json_t,
    flags: usize,
    depth: c_int,
    parents: *mut hashtable_t,
    dump: json_dump_callback_t,
    data: *mut c_void,
) -> c_int {
    let embed = flags & JSON_EMBED;
    let flags = flags & !JSON_EMBED;

    if json.is_null() {
        return -1;
    }

    match json_typeof(json) {
        JSON_NULL => dump_call(dump, b"null\0".as_ptr() as *const c_char, 4, data),

        JSON_TRUE => dump_call(dump, b"true\0".as_ptr() as *const c_char, 4, data),

        JSON_FALSE => dump_call(dump, b"false\0".as_ptr() as *const c_char, 5, data),

        JSON_INTEGER => {
            let mut buffer = [0i8; MAX_INTEGER_STR_LENGTH];
            let size: c_int;

            size = snprintf(
                buffer.as_mut_ptr(),
                MAX_INTEGER_STR_LENGTH,
                b"%lld\0".as_ptr() as *const c_char,
                json_integer_value(json),
            );
            if size < 0 || size >= MAX_INTEGER_STR_LENGTH as c_int {
                return -1;
            }

            dump_call(dump, buffer.as_ptr(), size as usize, data)
        }

        JSON_REAL => {
            let mut buffer = [0i8; MAX_REAL_STR_LENGTH];
            let size: c_int;
            let value = json_real_value(json);

            size = jsonp_dtostr(
                buffer.as_mut_ptr(),
                MAX_REAL_STR_LENGTH,
                value,
                flags_to_precision(flags),
            );
            if size < 0 {
                return -1;
            }

            dump_call(dump, buffer.as_ptr(), size as usize, data)
        }

        JSON_STRING => dump_string(
            json_string_value(json),
            json_string_length(json),
            dump,
            data,
            flags,
        ),

        JSON_ARRAY => {
            let n: usize;
            let mut i: usize;
            /* Space for "0x", double the sizeof a pointer for the hex and a
             * terminator. */
            let mut key = [0i8; 2 + (core::mem::size_of::<*const json_t>() * 2) + 1];
            let mut key_len: usize = 0;

            /* detect circular references */
            if jsonp_loop_check(parents, json, key.as_mut_ptr(), key.len(), &mut key_len) != 0 {
                return -1;
            }

            n = json_array_size(json);

            if embed == 0 && dump_call(dump, b"[\0".as_ptr() as *const c_char, 1, data) != 0 {
                return -1;
            }
            if n == 0 {
                hashtable_del(parents, key.as_ptr(), key_len);
                return if embed != 0 {
                    0
                } else {
                    dump_call(dump, b"]\0".as_ptr() as *const c_char, 1, data)
                };
            }
            if dump_indent(flags, depth + 1, 0, dump, data) != 0 {
                return -1;
            }

            i = 0;
            while i < n - 1 {
                if do_dump(json_array_get(json, i), flags, depth + 1, parents, dump, data) != 0 {
                    return -1;
                }

                if dump_call(dump, b",\0".as_ptr() as *const c_char, 1, data) != 0
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
            if embed != 0 {
                0
            } else {
                dump_call(dump, b"]\0".as_ptr() as *const c_char, 1, data)
            }
        }

        JSON_OBJECT => {
            let mut iter: *mut c_void;
            let separator: *const c_char;
            let separator_length: c_int;
            let mut loop_key = [0i8; LOOP_KEY_LEN];
            let mut loop_key_len: usize = 0;

            if (flags & JSON_COMPACT) != 0 {
                separator = b":\0".as_ptr() as *const c_char;
                separator_length = 1;
            } else {
                separator = b": \0".as_ptr() as *const c_char;
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

            if embed == 0 && dump_call(dump, b"{\0".as_ptr() as *const c_char, 1, data) != 0 {
                return -1;
            }

            iter = json_object_iter(json as *mut json_t);
            if iter.is_null() {
                hashtable_del(parents, loop_key.as_ptr(), loop_key_len);
                return if embed != 0 {
                    0
                } else {
                    dump_call(dump, b"}\0".as_ptr() as *const c_char, 1, data)
                };
            }
            if dump_indent(flags, depth + 1, 0, dump, data) != 0 {
                return -1;
            }

            if (flags & JSON_SORT_KEYS) != 0 {
                let keys: *mut key_len;
                let size: usize;
                let mut i: usize;

                size = json_object_size(json);
                keys = jsonp_malloc(size * core::mem::size_of::<key_len>()) as *mut key_len;
                if keys.is_null() {
                    return -1;
                }

                i = 0;
                while !iter.is_null() {
                    let keylen = keys.add(i);

                    (*keylen).key = json_object_iter_key(iter);
                    (*keylen).len = json_object_iter_key_len(iter) as c_int;

                    iter = json_object_iter_next(json as *mut json_t, iter);
                    i += 1;
                }
                debug_assert!(i == size);

                qsort(
                    keys as *mut c_void,
                    size,
                    core::mem::size_of::<key_len>(),
                    Some(compare_keys),
                );

                i = 0;
                while i < size {
                    let key: *const key_len;
                    let value: *mut json_t;

                    key = keys.add(i);
                    value = json_object_getn(json, (*key).key, (*key).len as usize);
                    debug_assert!(!value.is_null());

                    dump_string((*key).key, (*key).len as usize, dump, data, flags);
                    if dump_call(dump, separator, separator_length as usize, data) != 0
                        || do_dump(value, flags, depth + 1, parents, dump, data) != 0
                    {
                        jsonp_free(keys as *mut c_void);
                        return -1;
                    }

                    if i < size - 1 {
                        if dump_call(dump, b",\0".as_ptr() as *const c_char, 1, data) != 0
                            || dump_indent(flags, depth + 1, 1, dump, data) != 0
                        {
                            jsonp_free(keys as *mut c_void);
                            return -1;
                        }
                    } else if dump_indent(flags, depth, 0, dump, data) != 0 {
                        jsonp_free(keys as *mut c_void);
                        return -1;
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
                    if dump_call(dump, separator, separator_length as usize, data) != 0
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
                        if dump_call(dump, b",\0".as_ptr() as *const c_char, 1, data) != 0
                            || dump_indent(flags, depth + 1, 1, dump, data) != 0
                        {
                            return -1;
                        }
                    } else if dump_indent(flags, depth, 0, dump, data) != 0 {
                        return -1;
                    }

                    iter = next;
                }
            }

            hashtable_del(parents, loop_key.as_ptr(), loop_key_len);
            if embed != 0 {
                0
            } else {
                dump_call(dump, b"}\0".as_ptr() as *const c_char, 1, data)
            }
        }

        _ => -1,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumps(json: *const json_t, flags: usize) -> *mut c_char {
    let mut strbuff = strbuffer_t {
        value: core::ptr::null_mut(),
        length: 0,
        size: 0,
    };
    let mut result: *mut c_char;

    if strbuffer_init(&mut strbuff) != 0 {
        return core::ptr::null_mut();
    }

    if json_dump_callback(
        json,
        Some(dump_to_strbuffer),
        &mut strbuff as *mut strbuffer_t as *mut c_void,
        flags,
    ) != 0
    {
        result = core::ptr::null_mut();
    } else {
        let new_result: *mut c_char;
        result = strbuffer_steal_value(&mut strbuff);
        // technically the resizing is not needed.
        new_result = jsonp_realloc(result as *mut c_void, strbuff.size, strbuff.length + 1)
            as *mut c_char;
        if !new_result.is_null() {
            // when realloc fails we just use the original pointer
            result = new_result;
        }
    }

    strbuffer_close(&mut strbuff);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumpb(
    json: *const json_t,
    buffer_: *mut c_char,
    size: usize,
    flags: usize,
) -> usize {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumpf(
    json: *const json_t,
    output: *mut FILE,
    flags: usize,
) -> c_int {
    json_dump_callback(json, Some(dump_to_file), output as *mut c_void, flags)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumpfd(json: *const json_t, output: c_int, flags: usize) -> c_int {
    let mut output = output;
    json_dump_callback(
        json,
        Some(dump_to_fd),
        &mut output as *mut c_int as *mut c_void,
        flags,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dump_file(
    json: *const json_t,
    path: *const c_char,
    flags: usize,
) -> c_int {
    let result: c_int;

    let output = fopen(path, b"w\0".as_ptr() as *const c_char);
    if output.is_null() {
        return -1;
    }

    result = json_dumpf(json, output, flags);

    if fclose(output) != 0 {
        return -1;
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dump_callback(
    json: *const json_t,
    callback: json_dump_callback_t,
    data: *mut c_void,
    flags: usize,
) -> c_int {
    let res: c_int;
    let mut parents_set = core::mem::MaybeUninit::<hashtable_t>::uninit();

    if (flags & JSON_ENCODE_ANY) == 0 && !json_is_array(json) && !json_is_object(json) {
        return -1;
    }

    if hashtable_init(parents_set.as_mut_ptr()) != 0 {
        return -1;
    }
    res = do_dump(json, flags, 0, parents_set.as_mut_ptr(), callback, data);
    hashtable_close(parents_set.as_mut_ptr());

    res
}
