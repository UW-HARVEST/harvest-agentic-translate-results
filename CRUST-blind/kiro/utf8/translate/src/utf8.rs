// Import necessary modules
use crate::{utf8};
// Struct Definitions
#[derive(Debug, Clone)]
pub struct Utf8Validity {
    pub valid: bool,
    pub valid_upto: usize,
}
#[derive(Debug, Clone)]
pub struct OwnedUtf8String {
    pub str: String,
    pub byte_len: usize,
}
#[derive(Debug, Clone)]
pub struct Utf8Char {
    pub str: String,
    pub byte_len: u8,
}
#[derive(Debug, Clone)]
pub struct Utf8CharValidity {
    pub valid: bool,
    pub next_offset: usize,
}
#[derive(Debug, Clone)]
pub struct Utf8String {
    pub str: String,
    pub byte_len: usize,
}
#[derive(Debug, Clone)]
pub struct Utf8CharIter {
    pub str: String,
}

// Helper: position tracker for the iterator
// The C code uses a pointer that advances. We track position in the string bytes.
// We store the full string and a current byte offset in `str` by slicing.
// Actually, Utf8CharIter.str in C is a pointer that advances. In Rust, we'll
// store the remaining string (from current position onward).

pub fn validate_utf8_char(bytes: &[u8], offset: usize) -> Utf8CharValidity {
    // Single-byte: 0xxxxxxx
    if (bytes[offset] & 0b10000000) == 0b00000000 {
        return Utf8CharValidity { valid: true, next_offset: offset + 1 };
    }

    // Two-byte: 110xxxxx 10xxxxxx
    if offset + 1 < bytes.len()
        && (bytes[offset] & 0b11100000) == 0b11000000
        && (bytes[offset + 1] & 0b11000000) == 0b10000000
    {
        // Overlong check
        if (bytes[offset] & 0b00011111) < 0b00000010 {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        return Utf8CharValidity { valid: true, next_offset: offset + 2 };
    }

    // Three-byte: 1110xxxx 10xxxxxx 10xxxxxx
    if offset + 2 < bytes.len()
        && (bytes[offset] & 0b11110000) == 0b11100000
        && (bytes[offset + 1] & 0b11000000) == 0b10000000
        && (bytes[offset + 2] & 0b11000000) == 0b10000000
    {
        // Overlong check
        if (bytes[offset] & 0b00001111) == 0b00000000
            && (bytes[offset + 1] & 0b00111111) < 0b00100000
        {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        // Surrogate rejection: U+D800..U+DFFF
        if bytes[offset] == 0b11101101
            && bytes[offset + 1] >= 0b10100000
            && bytes[offset + 1] <= 0b10111111
        {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        return Utf8CharValidity { valid: true, next_offset: offset + 3 };
    }

    // Four-byte: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
    if offset + 3 < bytes.len()
        && (bytes[offset] & 0b11111000) == 0b11110000
        && (bytes[offset + 1] & 0b11000000) == 0b10000000
        && (bytes[offset + 2] & 0b11000000) == 0b10000000
        && (bytes[offset + 3] & 0b11000000) == 0b10000000
    {
        // Overlong check
        if (bytes[offset] & 0b00000111) == 0b00000000
            && (bytes[offset + 1] & 0b00111111) < 0b00010000
        {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        return Utf8CharValidity { valid: true, next_offset: offset + 4 };
    }

    Utf8CharValidity { valid: false, next_offset: offset }
}

pub fn validate_utf8(bytes: &[u8]) -> Utf8Validity {
    let mut offset = 0;
    while offset < bytes.len() {
        let cv = validate_utf8_char(bytes, offset);
        if cv.valid {
            offset = cv.next_offset;
        } else {
            return Utf8Validity { valid: false, valid_upto: offset };
        }
    }
    Utf8Validity { valid: true, valid_upto: offset }
}

pub fn make_utf8_string(bytes: &[u8]) -> Utf8String {
    let validity = validate_utf8(bytes);
    if validity.valid {
        let s = String::from_utf8_lossy(bytes).into_owned();
        Utf8String { byte_len: validity.valid_upto, str: s }
    } else {
        Utf8String { str: String::new(), byte_len: 0 }
    }
}

pub fn make_utf8_string_lossy(bytes: &[u8]) -> OwnedUtf8String {
    let mut result = Vec::with_capacity(bytes.len());
    let mut offset = 0;
    while offset < bytes.len() {
        let cv = validate_utf8_char(bytes, offset);
        if cv.valid {
            result.extend_from_slice(&bytes[offset..cv.next_offset]);
            offset = cv.next_offset;
        } else {
            // U+FFFD = EF BF BD
            result.extend_from_slice(&[0xEF, 0xBF, 0xBD]);
            offset += 1;
        }
    }
    let byte_len = result.len();
    let s = String::from_utf8(result).unwrap();
    OwnedUtf8String { str: s, byte_len }
}

pub fn as_utf8_string(owned_str: &OwnedUtf8String) -> Utf8String {
    Utf8String { str: owned_str.str.clone(), byte_len: owned_str.byte_len }
}

pub fn free_owned_utf8_string(_owned_str: &mut OwnedUtf8String) {
    _owned_str.str = String::new();
    _owned_str.byte_len = 0;
}

pub fn is_utf8_char_boundary(bytes: &[u8]) -> bool {
    // Empty slice means we're at the end ('\0' in C), which is a boundary
    if bytes.is_empty() { return true; }
    bytes[0] <= 0b01111111 || bytes[0] >= 0b11000000
}

pub fn slice_utf8_string(ustr: Utf8String, byte_index: usize, byte_len: usize) -> Utf8String {
    let bytes = ustr.str.as_bytes();
    let total = ustr.byte_len;

    let start = if byte_index > total { total } else { byte_index };
    let mut end = start.saturating_add(byte_len);
    if end > total { end = total; }

    if is_utf8_char_boundary(&bytes[start..]) && is_utf8_char_boundary(&bytes[end..]) {
        let slice = &ustr.str[start..end];
        Utf8String { str: slice.to_string(), byte_len: end - start }
    } else {
        Utf8String { str: String::new(), byte_len: 0 }
    }
}

pub fn make_utf8_char_iter(ustr: Utf8String) -> Utf8CharIter {
    Utf8CharIter { str: ustr.str }
}

pub fn next_utf8_char(iter: &mut Utf8CharIter) -> Utf8Char {
    let bytes = iter.str.as_bytes();
    if bytes.is_empty() {
        return Utf8Char { str: String::new(), byte_len: 0 };
    }

    let mut len: u8 = 1;
    while (len as usize) < bytes.len() && !is_utf8_char_boundary(&bytes[len as usize..]) {
        len += 1;
    }

    let ch_str = iter.str[..len as usize].to_string();
    iter.str = iter.str[len as usize..].to_string();
    Utf8Char { str: ch_str, byte_len: len }
}

pub fn nth_utf8_char(ustr: Utf8String, char_index: usize) -> Utf8Char {
    let mut iter = make_utf8_char_iter(ustr);
    let mut remaining = char_index;
    loop {
        let ch = next_utf8_char(&mut iter);
        if ch.byte_len == 0 {
            return Utf8Char { str: String::new(), byte_len: 0 };
        }
        if remaining == 0 {
            return ch;
        }
        remaining -= 1;
    }
}

pub fn utf8_char_count(ustr: Utf8String) -> usize {
    let mut iter = make_utf8_char_iter(ustr);
    let mut count = 0;
    while next_utf8_char(&mut iter).byte_len > 0 {
        count += 1;
    }
    count
}

pub fn unicode_code_point(uchar: Utf8Char) -> u32 {
    let b = uchar.str.as_bytes();
    match uchar.byte_len {
        1 => (b[0] & 0b01111111) as u32,
        2 => ((b[0] & 0b00011111) as u32) << 6
           | (b[1] & 0b00111111) as u32,
        3 => ((b[0] & 0b00001111) as u32) << 12
           | ((b[1] & 0b00111111) as u32) << 6
           | (b[2] & 0b00111111) as u32,
        4 => ((b[0] & 0b00000111) as u32) << 18
           | ((b[1] & 0b00111111) as u32) << 12
           | ((b[2] & 0b00111111) as u32) << 6
           | (b[3] & 0b00111111) as u32,
        _ => 0,
    }
}
