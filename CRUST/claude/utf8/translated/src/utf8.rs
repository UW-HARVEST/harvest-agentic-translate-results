// Import necessary modules
#[allow(unused_imports)]
use crate::utf8;
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
// Function Definitions
pub fn slice_utf8_string(ustr: Utf8String, byte_index: usize, byte_len: usize) -> Utf8String {
    let mut start = byte_index;
    if start > ustr.byte_len {
        start = ustr.byte_len;
    }

    let mut excl_end = start.saturating_add(byte_len);
    if excl_end > ustr.byte_len {
        excl_end = ustr.byte_len;
    }

    let bytes = ustr.str.as_bytes();

    // Check is_utf8_char_boundary at both ends.
    let start_ok = if start == bytes.len() {
        true
    } else {
        is_utf8_char_boundary(&bytes[start..])
    };
    let end_ok = if excl_end == bytes.len() {
        true
    } else {
        is_utf8_char_boundary(&bytes[excl_end..])
    };

    if start_ok && end_ok {
        // Build the substring from start..excl_end. This is guaranteed to be
        // valid UTF-8 because the original string is valid UTF-8 and the
        // bounds are at character boundaries.
        let slice_bytes = &bytes[start..excl_end];
        let s = match std::str::from_utf8(slice_bytes) {
            Ok(s) => s.to_string(),
            Err(_) => String::new(),
        };
        let len = excl_end - start;
        return Utf8String { str: s, byte_len: len };
    }

    Utf8String { str: String::new(), byte_len: 0 }
}

pub fn unicode_code_point(uchar: Utf8Char) -> u32 {
    let bytes = uchar.str.as_bytes();
    match uchar.byte_len {
        1 => (bytes[0] & 0b0111_1111) as u32,
        2 => {
            ((bytes[0] & 0b0001_1111) as u32) << 6
                | ((bytes[1] & 0b0011_1111) as u32)
        }
        3 => {
            ((bytes[0] & 0b0000_1111) as u32) << 12
                | ((bytes[1] & 0b0011_1111) as u32) << 6
                | ((bytes[2] & 0b0011_1111) as u32)
        }
        4 => {
            ((bytes[0] & 0b0000_0111) as u32) << 18
                | ((bytes[1] & 0b0011_1111) as u32) << 12
                | ((bytes[2] & 0b0011_1111) as u32) << 6
                | ((bytes[3] & 0b0011_1111) as u32)
        }
        _ => 0,
    }
}

pub fn free_owned_utf8_string(owned_str: &mut OwnedUtf8String) {
    owned_str.str = String::new();
    owned_str.byte_len = 0;
}

pub fn utf8_char_count(ustr: Utf8String) -> usize {
    let mut iter = make_utf8_char_iter(ustr);
    let mut count = 0;
    loop {
        let ch = next_utf8_char(&mut iter);
        if ch.byte_len == 0 {
            break;
        }
        count += 1;
    }
    count
}

pub fn make_utf8_char_iter(ustr: Utf8String) -> Utf8CharIter {
    // Use only the first `byte_len` bytes of ustr.str (the C version uses
    // ustr.str directly which is null-terminated; here we trust byte_len).
    let bytes = ustr.str.as_bytes();
    let len = ustr.byte_len.min(bytes.len());
    let s = std::str::from_utf8(&bytes[..len]).unwrap_or("").to_string();
    Utf8CharIter { str: s }
}

pub fn validate_utf8_char(bytes: &[u8], offset: usize) -> Utf8CharValidity {
    let len = bytes.len();

    let b0 = if offset < len { bytes[offset] } else { 0 };

    // Single-byte: 0xxxxxxx
    if b0 & 0b1000_0000 == 0 {
        // In C, str[offset] is dereferenced unconditionally. The terminating
        // '\0' is also a valid 1-byte char. Mirror that here only when offset
        // < len; if offset >= len treat as invalid.
        if offset < len {
            return Utf8CharValidity { valid: true, next_offset: offset + 1 };
        }
        return Utf8CharValidity { valid: false, next_offset: offset };
    }

    let b1 = if offset + 1 < len { bytes[offset + 1] } else { 0 };

    // Two-byte: 110xxxxx 10xxxxxx
    if b0 & 0b1110_0000 == 0b1100_0000 && b1 & 0b1100_0000 == 0b1000_0000 {
        // Overlong check
        if (b0 & 0b0001_1111) < 0b0000_0010 {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        return Utf8CharValidity { valid: true, next_offset: offset + 2 };
    }

    let b2 = if offset + 2 < len { bytes[offset + 2] } else { 0 };

    // Three-byte: 1110xxxx 10xxxxxx 10xxxxxx
    if b0 & 0b1111_0000 == 0b1110_0000
        && b1 & 0b1100_0000 == 0b1000_0000
        && b2 & 0b1100_0000 == 0b1000_0000
    {
        // Overlong
        if b0 & 0b0000_1111 == 0 && (b1 & 0b0011_1111) < 0b0010_0000 {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        // Reject UTF-16 surrogates (U+D800..U+DFFF)
        if b0 == 0b1110_1101 && b1 >= 0b1010_0000 && b1 <= 0b1011_1111 {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        return Utf8CharValidity { valid: true, next_offset: offset + 3 };
    }

    let b3 = if offset + 3 < len { bytes[offset + 3] } else { 0 };

    // Four-byte: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
    if b0 & 0b1111_1000 == 0b1111_0000
        && b1 & 0b1100_0000 == 0b1000_0000
        && b2 & 0b1100_0000 == 0b1000_0000
        && b3 & 0b1100_0000 == 0b1000_0000
    {
        // Overlong
        if b0 & 0b0000_0111 == 0 && (b1 & 0b0011_1111) < 0b0001_0000 {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        return Utf8CharValidity { valid: true, next_offset: offset + 4 };
    }

    Utf8CharValidity { valid: false, next_offset: offset }
}

pub fn make_utf8_string(bytes: &[u8]) -> Utf8String {
    let validity = validate_utf8(bytes);
    if validity.valid {
        // bytes is valid UTF-8, safe to convert to String
        let s = std::str::from_utf8(&bytes[..validity.valid_upto])
            .unwrap_or("")
            .to_string();
        Utf8String { str: s, byte_len: validity.valid_upto }
    } else {
        Utf8String { str: String::new(), byte_len: 0 }
    }
}

pub fn validate_utf8(bytes: &[u8]) -> Utf8Validity {
    let len = bytes.len();
    let mut offset = 0;
    while offset < len {
        let cv = validate_utf8_char(bytes, offset);
        if cv.valid {
            offset = cv.next_offset;
        } else {
            return Utf8Validity { valid: false, valid_upto: offset };
        }
    }
    Utf8Validity { valid: true, valid_upto: offset }
}

pub fn is_utf8_char_boundary(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        // Mirror C behavior: '\0' is a valid char boundary; an empty slice
        // represents end-of-string which is also a boundary.
        return true;
    }
    let b = bytes[0];
    b <= 0b0111_1111 || b >= 0b1100_0000
}

pub fn as_utf8_string(owned_str: &OwnedUtf8String) -> Utf8String {
    Utf8String { str: owned_str.str.clone(), byte_len: owned_str.byte_len }
}

pub fn next_utf8_char(iter: &mut Utf8CharIter) -> Utf8Char {
    let bytes = iter.str.as_bytes();
    if bytes.is_empty() {
        return Utf8Char { str: String::new(), byte_len: 0 };
    }

    // The current iter.str starts at a char boundary; this entire string
    // becomes the resulting Utf8Char's str. Determine byte_len by scanning
    // until the next char boundary (or end of string).
    let curr = iter.str.clone();

    let mut byte_len: u8 = 1;
    let mut idx = 1;
    while idx < bytes.len() && !is_utf8_char_boundary(&bytes[idx..]) {
        idx += 1;
        byte_len = byte_len.saturating_add(1);
    }

    // Advance the iterator: drop the bytes we consumed.
    let remaining = std::str::from_utf8(&bytes[idx..])
        .unwrap_or("")
        .to_string();
    iter.str = remaining;

    Utf8Char { str: curr, byte_len }
}

pub fn make_utf8_string_lossy(bytes: &[u8]) -> OwnedUtf8String {
    let len = bytes.len();
    // Worst case size: every byte invalid -> 3 replacement bytes
    let mut buffer: Vec<u8> = Vec::with_capacity(len * 3);
    let mut offset = 0;

    while offset < len {
        let cv = validate_utf8_char(bytes, offset);
        if cv.valid {
            let char_len = cv.next_offset - offset;
            buffer.extend_from_slice(&bytes[offset..offset + char_len]);
            offset = cv.next_offset;
        } else {
            // U+FFFD encoded as EF BF BD
            buffer.push(0xEF);
            buffer.push(0xBF);
            buffer.push(0xBD);
            offset += 1;
        }
    }

    let byte_len = buffer.len();
    let s = String::from_utf8(buffer).unwrap_or_else(|_| String::new());
    OwnedUtf8String { str: s, byte_len }
}

pub fn nth_utf8_char(ustr: Utf8String, char_index: usize) -> Utf8Char {
    let mut iter = make_utf8_char_iter(ustr);
    let mut remaining = char_index;
    let mut ch = next_utf8_char(&mut iter);
    while ch.byte_len != 0 && remaining != 0 {
        remaining -= 1;
        ch = next_utf8_char(&mut iter);
    }
    if ch.byte_len == 0 {
        return Utf8Char { str: String::new(), byte_len: 0 };
    }
    ch
}
