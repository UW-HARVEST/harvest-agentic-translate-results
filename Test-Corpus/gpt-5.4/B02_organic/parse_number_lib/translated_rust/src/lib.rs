use libc::{c_int, size_t};
use std::ptr;
use std::slice;

pub type CJSONBool = c_int;
pub const TRUE: CJSONBool = 1;
pub const FALSE: CJSONBool = 0;
pub const CJSON_NUMBER: c_int = 1 << 3;

#[repr(C)]
pub struct parse_buffer {
    pub content: *const u8,
    pub length: size_t,
    pub offset: size_t,
    pub depth: size_t,
}

#[repr(C)]
pub struct cJSON {
    pub type_: c_int,
    pub valueint: c_int,
    pub valuedouble: f64,
}

fn can_access_at_index(buffer: *const parse_buffer, index: usize) -> bool {
    if buffer.is_null() {
        return false;
    }
    let buffer_ref = unsafe { &*buffer };
    buffer_ref.offset.saturating_add(index) < buffer_ref.length
}

fn parse_number_bytes(bytes: &[u8]) -> Option<(f64, usize)> {
    let mut len = 0usize;
    let mut has_decimal_point = false;

    while len < bytes.len() {
        match bytes[len] {
            b'0'..=b'9' | b'+' | b'-' | b'e' | b'E' => len += 1,
            b'.' => {
                has_decimal_point = true;
                len += 1;
            }
            _ => break,
        }
    }

    if len == 0 {
        return None;
    }

    let mut number_bytes = bytes[..len].to_vec();
    if has_decimal_point {
        for byte in &mut number_bytes {
            if *byte == b'.' {
                *byte = b'.';
            }
        }
    }

    let number_str = std::str::from_utf8(&number_bytes).ok()?;
    let number = number_str.parse::<f64>().ok()?;
    Some((number, len))
}

#[unsafe(no_mangle)]
pub extern "C" fn parse_number(item: *mut cJSON, input_buffer: *mut parse_buffer) -> CJSONBool {
    if item.is_null() || input_buffer.is_null() {
        return FALSE;
    }

    let input_buffer_ref = unsafe { &mut *input_buffer };
    if input_buffer_ref.content.is_null() {
        return FALSE;
    }

    let remaining = input_buffer_ref.length.saturating_sub(input_buffer_ref.offset);
    let bytes = unsafe {
        slice::from_raw_parts(input_buffer_ref.content.add(input_buffer_ref.offset), remaining)
    };

    let (number, consumed) = match parse_number_bytes(bytes) {
        Some(v) => v,
        None => return FALSE,
    };

    let item_ref = unsafe { &mut *item };
    item_ref.valuedouble = number;

    if number >= c_int::MAX as f64 {
        item_ref.valueint = c_int::MAX;
    } else if number <= c_int::MIN as f64 {
        item_ref.valueint = c_int::MIN;
    } else {
        item_ref.valueint = number as c_int;
    }

    item_ref.type_ = CJSON_NUMBER;
    input_buffer_ref.offset = input_buffer_ref.offset.saturating_add(consumed as size_t);
    TRUE
}
