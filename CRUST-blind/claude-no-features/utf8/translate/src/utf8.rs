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
// Function Definitions
pub fn slice_utf8_string(ustr: Utf8String, byte_index: usize, byte_len: usize) -> Utf8String {
    let mut start = byte_index;
    if start > ustr.byte_len {
        start = ustr.byte_len;
    }
    let mut end = start.saturating_add(byte_len);
    if end > ustr.byte_len {
        end = ustr.byte_len;
    }

    let bytes = ustr.str.as_bytes();
    // The Utf8String already holds valid UTF-8 with byte_len <= bytes.len(),
    // but be defensive in case byte_len is inconsistent.
    let upper = bytes.len().min(ustr.byte_len);
    if start > upper {
        start = upper;
    }
    if end > upper {
        end = upper;
    }

    let start_ok = is_utf8_char_boundary(&bytes[start..]);
    let end_ok = is_utf8_char_boundary(&bytes[end..]);

    if start_ok && end_ok {
        let slice_bytes = &bytes[start..end];
        match std::str::from_utf8(slice_bytes) {
            Ok(s) => Utf8String { str: s.to_string(), byte_len: end - start },
            Err(_) => Utf8String { str: String::new(), byte_len: 0 },
        }
    } else {
        Utf8String { str: String::new(), byte_len: 0 }
    }
}

pub fn unicode_code_point(uchar: Utf8Char) -> u32 {
    let bytes = uchar.str.as_bytes();
    match uchar.byte_len {
        1 => {
            if bytes.is_empty() { 0 } else { (bytes[0] & 0b0111_1111) as u32 }
        }
        2 => {
            if bytes.len() < 2 { 0 } else {
                (((bytes[0] & 0b0001_1111) as u32) << 6)
                    | ((bytes[1] & 0b0011_1111) as u32)
            }
        }
        3 => {
            if bytes.len() < 3 { 0 } else {
                (((bytes[0] & 0b0000_1111) as u32) << 12)
                    | (((bytes[1] & 0b0011_1111) as u32) << 6)
                    | ((bytes[2] & 0b0011_1111) as u32)
            }
        }
        4 => {
            if bytes.len() < 4 { 0 } else {
                (((bytes[0] & 0b0000_0111) as u32) << 18)
                    | (((bytes[1] & 0b0011_1111) as u32) << 12)
                    | (((bytes[2] & 0b0011_1111) as u32) << 6)
                    | ((bytes[3] & 0b0011_1111) as u32)
            }
        }
        _ => 0,
    }
}

pub fn free_owned_utf8_string(_owned_str: &mut OwnedUtf8String) {
    _owned_str.str.clear();
    _owned_str.byte_len = 0;
}

pub fn utf8_char_count(ustr: Utf8String) -> usize {
    let mut iter = make_utf8_char_iter(ustr);
    let mut count: usize = 0;
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
    Utf8CharIter { str: ustr.str }
}

pub fn validate_utf8_char(bytes: &[u8], offset: usize) -> Utf8CharValidity {
    let get = |i: usize| -> u8 {
        if i < bytes.len() { bytes[i] } else { 0 }
    };

    let b0 = get(offset);

    // Single-byte UTF-8 characters: 0xxxxxxx
    if (b0 & 0b1000_0000) == 0b0000_0000 {
        return Utf8CharValidity { valid: true, next_offset: offset + 1 };
    }

    let b1 = get(offset + 1);

    // Two-byte UTF-8 characters: 110xxxxx 10xxxxxx
    if (b0 & 0b1110_0000) == 0b1100_0000 && (b1 & 0b1100_0000) == 0b1000_0000 {
        // Check for overlong encoding
        if (b0 & 0b0001_1111) < 0b0000_0010 {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        return Utf8CharValidity { valid: true, next_offset: offset + 2 };
    }

    let b2 = get(offset + 2);

    // Three-byte UTF-8 characters: 1110xxxx 10xxxxxx 10xxxxxx
    if (b0 & 0b1111_0000) == 0b1110_0000
        && (b1 & 0b1100_0000) == 0b1000_0000
        && (b2 & 0b1100_0000) == 0b1000_0000
    {
        // Check for overlong encoding
        if (b0 & 0b0000_1111) == 0b0000_0000 && (b1 & 0b0011_1111) < 0b0010_0000 {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        // Reject UTF-16 surrogates: U+D800 to U+DFFF
        if b0 == 0b1110_1101 && b1 >= 0b1010_0000 && b1 <= 0b1011_1111 {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        return Utf8CharValidity { valid: true, next_offset: offset + 3 };
    }

    let b3 = get(offset + 3);

    // Four-byte UTF-8 characters: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
    if (b0 & 0b1111_1000) == 0b1111_0000
        && (b1 & 0b1100_0000) == 0b1000_0000
        && (b2 & 0b1100_0000) == 0b1000_0000
        && (b3 & 0b1100_0000) == 0b1000_0000
    {
        // Check for overlong encoding
        if (b0 & 0b0000_0111) == 0b0000_0000 && (b1 & 0b0011_1111) < 0b0001_0000 {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        return Utf8CharValidity { valid: true, next_offset: offset + 4 };
    }

    Utf8CharValidity { valid: false, next_offset: offset }
}

pub fn make_utf8_string(bytes: &[u8]) -> Utf8String {
    let validity = validate_utf8(bytes);
    if validity.valid {
        match std::str::from_utf8(&bytes[..validity.valid_upto]) {
            Ok(s) => Utf8String { str: s.to_string(), byte_len: validity.valid_upto },
            Err(_) => Utf8String { str: String::new(), byte_len: 0 },
        }
    } else {
        Utf8String { str: String::new(), byte_len: 0 }
    }
}

pub fn validate_utf8(bytes: &[u8]) -> Utf8Validity {
    let mut offset: usize = 0;
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

pub fn is_utf8_char_boundary(bytes: &[u8]) -> bool {
    // In C, the function dereferences *str which can be '\0' at end of string.
    // We treat an empty slice as the equivalent of pointing at '\0'.
    if bytes.is_empty() {
        return true;
    }
    let b = bytes[0];
    b <= 0b0111_1111 || b >= 0b1100_0000
}

pub fn as_utf8_string(owned_str: &OwnedUtf8String) -> Utf8String {
    Utf8String { str: owned_str.str.clone(), byte_len: owned_str.byte_len }
}

pub fn next_utf8_char(iter: &mut Utf8CharIter) -> Utf8Char {
    if iter.str.is_empty() {
        return Utf8Char { str: String::new(), byte_len: 0 };
    }

    let bytes = iter.str.as_bytes();
    let total = bytes.len();
    let mut byte_len: usize = 1;

    // advance until next char boundary or end of string
    while byte_len < total && !is_utf8_char_boundary(&bytes[byte_len..]) {
        byte_len += 1;
    }

    let ch_str = match std::str::from_utf8(&bytes[..byte_len]) {
        Ok(s) => s.to_string(),
        Err(_) => String::new(),
    };
    let rest_str = match std::str::from_utf8(&bytes[byte_len..]) {
        Ok(s) => s.to_string(),
        Err(_) => String::new(),
    };

    iter.str = rest_str;

    Utf8Char { str: ch_str, byte_len: byte_len as u8 }
}

pub fn make_utf8_string_lossy(bytes: &[u8]) -> OwnedUtf8String {
    let mut buffer: Vec<u8> = Vec::with_capacity(bytes.len() * 3 + 1);
    let mut offset: usize = 0;

    while offset < bytes.len() {
        let cv = validate_utf8_char(bytes, offset);

        if cv.valid {
            // Copy the valid UTF-8 character sequence to the buffer.
            buffer.extend_from_slice(&bytes[offset..cv.next_offset]);
            offset = cv.next_offset;
        } else {
            // Insert U+FFFD (REPLACEMENT CHARACTER) bytes.
            buffer.push(0xEF);
            buffer.push(0xBF);
            buffer.push(0xBD);
            offset += 1;
        }
    }

    let s = match String::from_utf8(buffer) {
        Ok(s) => s,
        Err(_) => String::new(),
    };
    let byte_len = s.len();
    OwnedUtf8String { str: s, byte_len }
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
