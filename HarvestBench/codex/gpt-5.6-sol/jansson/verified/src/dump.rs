use crate::memory::{jsonp_free, jsonp_malloc, jsonp_realloc};
use crate::types::*;
use crate::value::*;
use std::collections::HashSet;
use std::ffi::{CStr, c_char, c_int};
use std::fmt::Write;
use std::ptr;

fn float_string(value: f64, precision: usize) -> Option<String> {
    let mut buffer = [0i8; 64];
    let length = unsafe {
        crate::dtoa::jsonp_dtostr(buffer.as_mut_ptr(), buffer.len(), value, precision as c_int)
    };
    (length >= 0).then(|| {
        unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    })
}

fn dump_string(output: &mut Vec<u8>, bytes: &[u8], flags: usize) -> Result<(), ()> {
    let text = std::str::from_utf8(bytes).map_err(|_| ())?;
    output.push(b'"');
    for character in text.chars() {
        match character {
            '"' => output.extend_from_slice(br#"\""#),
            '\\' => output.extend_from_slice(br#"\\"#),
            '/' if flags & JSON_ESCAPE_SLASH != 0 => output.extend_from_slice(br#"\/"#),
            '\u{8}' => output.extend_from_slice(br#"\b"#),
            '\u{c}' => output.extend_from_slice(br#"\f"#),
            '\n' => output.extend_from_slice(br#"\n"#),
            '\r' => output.extend_from_slice(br#"\r"#),
            '\t' => output.extend_from_slice(br#"\t"#),
            c if c < '\u{20}' => {
                write!(ByteWriter(output), "\\u{:04X}", c as u32).map_err(|_| ())?
            }
            c if flags & JSON_ENSURE_ASCII != 0 && c as u32 > 0x7f => {
                let codepoint = c as u32;
                if codepoint < 0x10000 {
                    write!(ByteWriter(output), "\\u{codepoint:04X}").map_err(|_| ())?;
                } else {
                    let adjusted = codepoint - 0x10000;
                    let first = 0xd800 | ((adjusted >> 10) & 0x3ff);
                    let second = 0xdc00 | (adjusted & 0x3ff);
                    write!(ByteWriter(output), "\\u{first:04X}\\u{second:04X}").map_err(|_| ())?;
                }
            }
            c => {
                let mut buffer = [0; 4];
                output.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
            }
        }
    }
    output.push(b'"');
    Ok(())
}

struct ByteWriter<'a>(&'a mut Vec<u8>);

impl std::fmt::Write for ByteWriter<'_> {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        self.0.extend_from_slice(value.as_bytes());
        Ok(())
    }
}

fn indent(output: &mut Vec<u8>, flags: usize, depth: usize, space: bool) {
    let width = flags & JSON_MAX_INDENT;
    if width != 0 {
        output.push(b'\n');
        output.resize(output.len() + width * depth, b' ');
    } else if space && flags & JSON_COMPACT == 0 {
        output.push(b' ');
    }
}

unsafe fn dump_value(
    json: *const json_t,
    flags: usize,
    depth: usize,
    seen: &mut HashSet<usize>,
    output: &mut Vec<u8>,
) -> Result<(), ()> {
    if json.is_null() {
        return Err(());
    }
    let embed = flags & JSON_EMBED != 0;
    let flags = flags & !JSON_EMBED;
    match type_of(json) {
        Some(JSON_NULL) => output.extend_from_slice(b"null"),
        Some(JSON_TRUE) => output.extend_from_slice(b"true"),
        Some(JSON_FALSE) => output.extend_from_slice(b"false"),
        Some(JSON_INTEGER) => {
            output.extend_from_slice(json_integer_value(json).to_string().as_bytes())
        }
        Some(JSON_REAL) => {
            let precision = (flags >> 11) & 0x1f;
            output.extend_from_slice(
                float_string(json_real_value(json), precision)
                    .ok_or(())?
                    .as_bytes(),
            );
        }
        Some(JSON_STRING) => {
            let string = string_ref(json);
            dump_string(output, &string.value[..string.value.len() - 1], flags)?;
        }
        Some(JSON_ARRAY) => {
            if !seen.insert(json as usize) {
                return Err(());
            }
            let loop_entry = jsonp_malloc(75);
            if loop_entry.is_null() {
                return Err(());
            }
            let values = &array_ref(json).values;
            if !embed {
                output.push(b'[');
            }
            if !values.is_empty() {
                indent(output, flags, depth + 1, false);
                for (index, &value) in values.iter().enumerate() {
                    dump_value(value, flags, depth + 1, seen, output)?;
                    if index + 1 != values.len() {
                        output.push(b',');
                        indent(output, flags, depth + 1, true);
                    }
                }
                indent(output, flags, depth, false);
            }
            if !embed {
                output.push(b']');
            }
            seen.remove(&(json as usize));
            jsonp_free(loop_entry);
        }
        Some(JSON_OBJECT) => {
            if !seen.insert(json as usize) {
                return Err(());
            }
            let loop_entry = jsonp_malloc(75);
            if loop_entry.is_null() {
                return Err(());
            }
            let object = object_ref(json);
            if !embed {
                output.push(b'{');
            }
            if !object.entries.is_empty() {
                indent(output, flags, depth + 1, false);
                let mut entries: Vec<_> = object.entries.iter().collect();
                if flags & JSON_SORT_KEYS != 0 {
                    entries.sort_by(|a, b| {
                        let a = &a.key[std::mem::size_of::<usize>()..][..a.key_len];
                        let b = &b.key[std::mem::size_of::<usize>()..][..b.key_len];
                        a.cmp(b)
                    });
                }
                for (index, entry) in entries.iter().enumerate() {
                    let key = &entry.key[std::mem::size_of::<usize>()..][..entry.key_len];
                    dump_string(output, key, flags)?;
                    output.push(b':');
                    if flags & JSON_COMPACT == 0 {
                        output.push(b' ');
                    }
                    dump_value(entry.value, flags, depth + 1, seen, output)?;
                    if index + 1 != entries.len() {
                        output.push(b',');
                        indent(output, flags, depth + 1, true);
                    }
                }
                indent(output, flags, depth, false);
            }
            if !embed {
                output.push(b'}');
            }
            seen.remove(&(json as usize));
            jsonp_free(loop_entry);
        }
        _ => return Err(()),
    }
    Ok(())
}

unsafe fn render(json: *const json_t, flags: usize) -> Option<Vec<u8>> {
    if flags & JSON_ENCODE_ANY == 0 && !matches!(type_of(json), Some(JSON_ARRAY | JSON_OBJECT)) {
        return None;
    }
    let parents = jsonp_malloc(8 * 2 * std::mem::size_of::<usize>());
    if parents.is_null() {
        return None;
    }
    let mut output = Vec::new();
    let result = dump_value(json, flags, 0, &mut HashSet::new(), &mut output);
    jsonp_free(parents);
    result.ok()?;
    Some(output)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dump_callback(
    json: *const json_t,
    callback: json_dump_callback_t,
    data: *mut std::ffi::c_void,
    flags: usize,
) -> c_int {
    let Some(callback) = callback else {
        return -1;
    };
    let Some(output) = render(json, flags) else {
        return -1;
    };
    callback(output.as_ptr().cast(), output.len(), data)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumps(json: *const json_t, flags: usize) -> *mut c_char {
    let initial = jsonp_malloc(16);
    if initial.is_null() {
        return ptr::null_mut();
    }
    let Some(output) = render(json, flags) else {
        jsonp_free(initial);
        return ptr::null_mut();
    };
    let result = jsonp_realloc(initial, 16, output.len() + 1).cast::<c_char>();
    if result.is_null() {
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(output.as_ptr(), result.cast(), output.len());
    *result.add(output.len()) = 0;
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumpb(
    json: *const json_t,
    buffer: *mut c_char,
    size: usize,
    flags: usize,
) -> usize {
    let Some(output) = render(json, flags) else {
        return 0;
    };
    if output.len() <= size && !buffer.is_null() {
        ptr::copy_nonoverlapping(output.as_ptr(), buffer.cast(), output.len());
    }
    output.len()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumpf(
    json: *const json_t,
    output: *mut libc::FILE,
    flags: usize,
) -> c_int {
    let Some(bytes) = render(json, flags) else {
        return -1;
    };
    if output.is_null() || libc::fwrite(bytes.as_ptr().cast(), bytes.len(), 1, output) != 1 {
        -1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dumpfd(json: *const json_t, output: c_int, flags: usize) -> c_int {
    let Some(bytes) = render(json, flags) else {
        return -1;
    };
    if libc::write(output, bytes.as_ptr().cast(), bytes.len()) == bytes.len() as isize {
        0
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_dump_file(
    json: *const json_t,
    path: *const c_char,
    flags: usize,
) -> c_int {
    if path.is_null() {
        return -1;
    }
    let output = libc::fopen(path, c"w".as_ptr());
    if output.is_null() {
        return -1;
    }
    let result = json_dumpf(json, output, flags);
    if libc::fclose(output) != 0 {
        -1
    } else {
        result
    }
}
