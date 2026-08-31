//! Translation of `src/dump.c`.

use crate::cfmt::hex_upper_pad4;
use crate::hashtable::*;
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

pub type JsonDumpCallbackT =
    unsafe extern "C" fn(buffer: *const c_char, size: usize, data: *mut c_void) -> c_int;

#[repr(C)]
struct Buffer {
    size: usize,
    used: usize,
    data: *mut c_char,
}

unsafe extern "C" fn dump_to_strbuffer(
    buffer: *const c_char,
    size: usize,
    data: *mut c_void,
) -> c_int {
    strbuffer_append_bytes(data as *mut StrbufferT, buffer, size)
}

unsafe extern "C" fn dump_to_buffer(
    buffer: *const c_char,
    size: usize,
    data: *mut c_void,
) -> c_int {
    let buf = data as *mut Buffer;

    if (*buf).used + size <= (*buf).size {
        memcpy(
            (*buf).data.add((*buf).used) as *mut c_void,
            buffer as *const c_void,
            size,
        );
    }

    (*buf).used += size;
    0
}

unsafe extern "C" fn dump_to_file(
    buffer: *const c_char,
    size: usize,
    data: *mut c_void,
) -> c_int {
    let dest = data;
    if fwrite(buffer as *const c_void, size, 1, dest) != 1 {
        return -1;
    }
    0
}

unsafe extern "C" fn dump_to_fd(buffer: *const c_char, size: usize, data: *mut c_void) -> c_int {
    let dest = data as *mut c_int;
    if write(*dest, buffer as *const c_void, size) == size as isize {
        return 0;
    }
    -1
}

/* 32 spaces (the maximum indentation size) */
static WHITESPACE: &[u8; 33] = b"                                \0";

unsafe fn dump_indent(
    flags: usize,
    depth: c_int,
    space: c_int,
    dump: JsonDumpCallbackT,
    data: *mut c_void,
) -> c_int {
    if flags_to_indent(flags) > 0 {
        let ws_count = flags_to_indent(flags) as u32;
        let mut n_spaces = (depth as u32).wrapping_mul(ws_count);

        if dump(b"\n".as_ptr() as *const c_char, 1, data) != 0 {
            return -1;
        }

        while n_spaces > 0 {
            let cur_n = if (n_spaces as usize) < WHITESPACE.len() - 1 {
                n_spaces as usize
            } else {
                WHITESPACE.len() - 1
            };

            if dump(WHITESPACE.as_ptr() as *const c_char, cur_n, data) != 0 {
                return -1;
            }

            n_spaces -= cur_n as u32;
        }
    } else if space != 0 && (flags & JSON_COMPACT) == 0 {
        return dump(b" ".as_ptr() as *const c_char, 1, data);
    }
    0
}

unsafe fn dump_string(
    str_: *const c_char,
    len: usize,
    dump: JsonDumpCallbackT,
    data: *mut c_void,
    flags: usize,
) -> c_int {
    let mut str_ = str_;
    let mut pos: *const c_char;
    let mut end: *const c_char;
    let lim: *const c_char;
    let mut codepoint: i32 = 0;

    if dump(b"\"".as_ptr() as *const c_char, 1, data) != 0 {
        return -1;
    }

    pos = str_;
    end = str_;
    lim = str_.add(len);
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
            if codepoint == b'\\' as i32 || codepoint == b'"' as i32 || codepoint < 0x20 {
                break;
            }

            /* slash */
            if (flags & JSON_ESCAPE_SLASH) != 0 && codepoint == b'/' as i32 {
                break;
            }

            /* non-ASCII */
            if (flags & JSON_ENSURE_ASCII) != 0 && codepoint > 0x7F {
                break;
            }

            pos = end;
        }

        if pos != str_ && dump(str_, pos.offset_from(str_) as usize, data) != 0 {
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
                    /* snprintf(seq, sizeof(seq), "\\u%04X", codepoint) */
                    let mut hx = [0u8; 8];
                    let n = hex_upper_pad4(codepoint as u32, &mut hx);
                    seq[0] = b'\\' as c_char;
                    seq[1] = b'u' as c_char;
                    for i in 0..n {
                        seq[2 + i] = hx[i] as c_char;
                    }
                    seq[2 + n] = 0;
                    length = 6;
                } else {
                    /* not in BMP -> construct a UTF-16 surrogate pair */
                    let first: i32;
                    let last: i32;

                    let cp = codepoint - 0x10000;
                    first = 0xD800 | ((cp & 0xffc00) >> 10);
                    last = 0xDC00 | (cp & 0x003ff);

                    let mut hx1 = [0u8; 8];
                    let n1 = hex_upper_pad4(first as u32, &mut hx1);
                    let mut hx2 = [0u8; 8];
                    let n2 = hex_upper_pad4(last as u32, &mut hx2);
                    seq[0] = b'\\' as c_char;
                    seq[1] = b'u' as c_char;
                    let mut w = 2;
                    for i in 0..n1 {
                        seq[w] = hx1[i] as c_char;
                        w += 1;
                    }
                    seq[w] = b'\\' as c_char;
                    w += 1;
                    seq[w] = b'u' as c_char;
                    w += 1;
                    for i in 0..n2 {
                        seq[w] = hx2[i] as c_char;
                        w += 1;
                    }
                    seq[w] = 0;
                    length = 12;
                }

                text = seq.as_ptr();
            }
        }

        if dump(text, length, data) != 0 {
            return -1;
        }

        str_ = end;
        pos = end;
    }

    dump(b"\"".as_ptr() as *const c_char, 1, data)
}

#[repr(C)]
#[derive(Copy, Clone)]
struct KeyLen {
    key: *const c_char,
    len: c_int,
}

fn compare_keys(k1: &KeyLen, k2: &KeyLen) -> core::cmp::Ordering {
    let min_size = if k1.len < k2.len { k1.len } else { k2.len } as usize;
    let res = unsafe {
        memcmp(
            k1.key as *const c_void,
            k2.key as *const c_void,
            min_size,
        )
    };

    if res != 0 {
        return res.cmp(&0);
    }

    (k1.len - k2.len).cmp(&0)
}

unsafe fn do_dump(
    json: *const JsonT,
    flags_in: usize,
    depth: c_int,
    parents: *mut HashtableT,
    dump: JsonDumpCallbackT,
    data: *mut c_void,
) -> c_int {
    let embed = flags_in & JSON_EMBED;
    let flags = flags_in & !JSON_EMBED;

    if json.is_null() {
        return -1;
    }

    match json_typeof(json) {
        JSON_NULL => dump(b"null".as_ptr() as *const c_char, 4, data),

        JSON_TRUE => dump(b"true".as_ptr() as *const c_char, 4, data),

        JSON_FALSE => dump(b"false".as_ptr() as *const c_char, 5, data),

        JSON_INTEGER => {
            let mut buffer = [0 as c_char; MAX_INTEGER_STR_LENGTH];
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

            dump(buffer.as_ptr(), size as usize, data)
        }

        JSON_REAL => {
            let mut buffer = [0 as c_char; MAX_REAL_STR_LENGTH];
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

            dump(buffer.as_ptr(), size as usize, data)
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
            let mut key = [0 as c_char; 2 + (core::mem::size_of::<*const JsonT>() * 2) + 1];
            let mut key_len: usize = 0;

            /* detect circular references */
            if jsonp_loop_check(
                parents,
                json,
                key.as_mut_ptr(),
                core::mem::size_of_val(&key),
                &mut key_len,
            ) != 0
            {
                return -1;
            }

            n = json_array_size(json);

            if embed == 0 && dump(b"[".as_ptr() as *const c_char, 1, data) != 0 {
                return -1;
            }
            if n == 0 {
                hashtable_del(parents, key.as_ptr(), key_len);
                return if embed != 0 {
                    0
                } else {
                    dump(b"]".as_ptr() as *const c_char, 1, data)
                };
            }
            if dump_indent(flags, depth + 1, 0, dump, data) != 0 {
                return -1;
            }

            i = 0;
            while i < n - 1 {
                if do_dump(
                    json_array_get(json, i),
                    flags,
                    depth + 1,
                    parents,
                    dump,
                    data,
                ) != 0
                {
                    return -1;
                }

                if dump(b",".as_ptr() as *const c_char, 1, data) != 0
                    || dump_indent(flags, depth + 1, 1, dump, data) != 0
                {
                    return -1;
                }
                i += 1;
            }

            if do_dump(
                json_array_get(json, i),
                flags,
                depth + 1,
                parents,
                dump,
                data,
            ) != 0
            {
                return -1;
            }
            if dump_indent(flags, depth, 0, dump, data) != 0 {
                return -1;
            }

            hashtable_del(parents, key.as_ptr(), key_len);
            if embed != 0 {
                0
            } else {
                dump(b"]".as_ptr() as *const c_char, 1, data)
            }
        }

        JSON_OBJECT => {
            let mut iter: *mut c_void;
            let separator: *const c_char;
            let separator_length: usize;
            let mut loop_key = [0 as c_char; LOOP_KEY_LEN];
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

            if embed == 0 && dump(b"{".as_ptr() as *const c_char, 1, data) != 0 {
                return -1;
            }

            iter = json_object_iter(json as *mut JsonT);
            if iter.is_null() {
                hashtable_del(parents, loop_key.as_ptr(), loop_key_len);
                return if embed != 0 {
                    0
                } else {
                    dump(b"}".as_ptr() as *const c_char, 1, data)
                };
            }
            if dump_indent(flags, depth + 1, 0, dump, data) != 0 {
                return -1;
            }

            if (flags & JSON_SORT_KEYS) != 0 {
                let keys: *mut KeyLen;
                let size: usize;
                let mut i: usize;

                size = json_object_size(json);
                keys = jsonp_malloc(size * core::mem::size_of::<KeyLen>()) as *mut KeyLen;
                if keys.is_null() {
                    return -1;
                }

                i = 0;
                while !iter.is_null() {
                    let keylen = keys.add(i);

                    (*keylen).key = json_object_iter_key(iter);
                    (*keylen).len = json_object_iter_key_len(iter) as c_int;

                    iter = json_object_iter_next(json as *mut JsonT, iter);
                    i += 1;
                }

                let slice = core::slice::from_raw_parts_mut(keys, size);
                slice.sort_by(compare_keys);

                i = 0;
                while i < size {
                    let key = keys.add(i);
                    let value = json_object_getn(json, (*key).key, (*key).len as usize);

                    dump_string((*key).key, (*key).len as usize, dump, data, flags);
                    if dump(separator, separator_length, data) != 0
                        || do_dump(value, flags, depth + 1, parents, dump, data) != 0
                    {
                        jsonp_free(keys as *mut c_void);
                        return -1;
                    }

                    if i < size - 1 {
                        if dump(b",".as_ptr() as *const c_char, 1, data) != 0
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
                    let next = json_object_iter_next(json as *mut JsonT, iter);
                    let key = json_object_iter_key(iter);
                    let key_len = json_object_iter_key_len(iter);

                    dump_string(key, key_len, dump, data, flags);
                    if dump(separator, separator_length, data) != 0
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
                        if dump(b",".as_ptr() as *const c_char, 1, data) != 0
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
                dump(b"}".as_ptr() as *const c_char, 1, data)
            }
        }

        _ => {
            /* not reached */
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumps(json: *const JsonT, flags: usize) -> *mut c_char {
    let mut strbuff: StrbufferT = core::mem::zeroed();
    let mut result: *mut c_char;

    if strbuffer_init(&mut strbuff) != 0 {
        return core::ptr::null_mut();
    }

    if json_dump_callback(
        json,
        dump_to_strbuffer,
        &mut strbuff as *mut StrbufferT as *mut c_void,
        flags,
    ) != 0
    {
        result = core::ptr::null_mut();
    } else {
        let new_result: *mut c_char;
        result = strbuffer_steal_value(&mut strbuff);
        // technically the resizing is not needed.
        new_result = jsonp_realloc(
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumpb(
    json: *const JsonT,
    buffer: *mut c_char,
    size: usize,
    flags: usize,
) -> usize {
    let mut buf = Buffer {
        size,
        used: 0,
        data: buffer,
    };

    if json_dump_callback(
        json,
        dump_to_buffer,
        &mut buf as *mut Buffer as *mut c_void,
        flags,
    ) != 0
    {
        return 0;
    }

    buf.used
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumpf(json: *const JsonT, output: *mut c_void, flags: usize) -> c_int {
    json_dump_callback(json, dump_to_file, output, flags)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumpfd(json: *const JsonT, output: c_int, flags: usize) -> c_int {
    let mut out = output;
    json_dump_callback(
        json,
        dump_to_fd,
        &mut out as *mut c_int as *mut c_void,
        flags,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dump_file(
    json: *const JsonT,
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
    json: *const JsonT,
    callback: JsonDumpCallbackT,
    data: *mut c_void,
    flags: usize,
) -> c_int {
    let res: c_int;
    let mut parents_set: HashtableT = core::mem::zeroed();

    if (flags & JSON_ENCODE_ANY) == 0 && !json_is_array(json) && !json_is_object(json) {
        return -1;
    }

    if hashtable_init(&mut parents_set) != 0 {
        return -1;
    }
    res = do_dump(json, flags, 0, &mut parents_set, callback, data);
    hashtable_close(&mut parents_set);

    res
}
