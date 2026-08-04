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
    let total = ustr.byte_len;
    let start = if byte_index > total { total } else { byte_index };
    let end_unclamped = start.saturating_add(byte_len);
    let end = if end_unclamped > total { total } else { end_unclamped };

    let bytes = ustr.str.as_bytes();

    // For safety, clamp to actual byte slice length too
    let start = start.min(bytes.len());
    let end = end.min(bytes.len());

    let start_ok = is_utf8_char_boundary(&bytes[start..]);
    let end_ok = is_utf8_char_boundary(&bytes[end..]);

    if start_ok && end_ok {
        let slice_bytes = &bytes[start..end];
        match std::str::from_utf8(slice_bytes) {
            Ok(s) => Utf8String {
                str: s.to_string(),
                byte_len: end - start,
            },
            Err(_) => Utf8String {
                str: String::new(),
                byte_len: 0,
            },
        }
    } else {
        Utf8String {
            str: String::new(),
            byte_len: 0,
        }
    }
}

pub fn unicode_code_point(uchar: Utf8Char) -> u32 {
    let bytes = uchar.str.as_bytes();
    match uchar.byte_len {
        1 => (bytes[0] & 0b0111_1111) as u32,
        2 => {
            ((bytes[0] & 0b0001_1111) as u32) << 6
                | (bytes[1] & 0b0011_1111) as u32
        }
        3 => {
            ((bytes[0] & 0b0000_1111) as u32) << 12
                | ((bytes[1] & 0b0011_1111) as u32) << 6
                | (bytes[2] & 0b0011_1111) as u32
        }
        4 => {
            ((bytes[0] & 0b0000_0111) as u32) << 18
                | ((bytes[1] & 0b0011_1111) as u32) << 12
                | ((bytes[2] & 0b0011_1111) as u32) << 6
                | (bytes[3] & 0b0011_1111) as u32
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
    if offset >= bytes.len() {
        return Utf8CharValidity {
            valid: false,
            next_offset: offset,
        };
    }

    let b0 = bytes[offset];

    // Single-byte UTF-8: 0xxxxxxx
    if b0 & 0b1000_0000 == 0b0000_0000 {
        return Utf8CharValidity {
            valid: true,
            next_offset: offset + 1,
        };
    }

    // Two-byte UTF-8: 110xxxxx 10xxxxxx
    if offset + 1 < bytes.len()
        && b0 & 0b1110_0000 == 0b1100_0000
        && bytes[offset + 1] & 0b1100_0000 == 0b1000_0000
    {
        // Overlong encoding rejection
        if b0 & 0b0001_1111 < 0b0000_0010 {
            return Utf8CharValidity {
                valid: false,
                next_offset: offset,
            };
        }
        return Utf8CharValidity {
            valid: true,
            next_offset: offset + 2,
        };
    }

    // Three-byte UTF-8: 1110xxxx 10xxxxxx 10xxxxxx
    if offset + 2 < bytes.len()
        && b0 & 0b1111_0000 == 0b1110_0000
        && bytes[offset + 1] & 0b1100_0000 == 0b1000_0000
        && bytes[offset + 2] & 0b1100_0000 == 0b1000_0000
    {
        // Overlong rejection
        if b0 & 0b0000_1111 == 0b0000_0000 && bytes[offset + 1] & 0b0011_1111 < 0b0010_0000 {
            return Utf8CharValidity {
                valid: false,
                next_offset: offset,
            };
        }
        // UTF-16 surrogate rejection: U+D800 to U+DFFF
        if b0 == 0b1110_1101
            && bytes[offset + 1] >= 0b1010_0000
            && bytes[offset + 1] <= 0b1011_1111
        {
            return Utf8CharValidity {
                valid: false,
                next_offset: offset,
            };
        }
        return Utf8CharValidity {
            valid: true,
            next_offset: offset + 3,
        };
    }

    // Four-byte UTF-8: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
    if offset + 3 < bytes.len()
        && b0 & 0b1111_1000 == 0b1111_0000
        && bytes[offset + 1] & 0b1100_0000 == 0b1000_0000
        && bytes[offset + 2] & 0b1100_0000 == 0b1000_0000
        && bytes[offset + 3] & 0b1100_0000 == 0b1000_0000
    {
        // Overlong rejection
        if b0 & 0b0000_0111 == 0b0000_0000 && bytes[offset + 1] & 0b0011_1111 < 0b0001_0000 {
            return Utf8CharValidity {
                valid: false,
                next_offset: offset,
            };
        }
        return Utf8CharValidity {
            valid: true,
            next_offset: offset + 4,
        };
    }

    Utf8CharValidity {
        valid: false,
        next_offset: offset,
    }
}

pub fn make_utf8_string(bytes: &[u8]) -> Utf8String {
    let validity = validate_utf8(bytes);
    if validity.valid {
        match std::str::from_utf8(&bytes[..validity.valid_upto]) {
            Ok(s) => Utf8String {
                str: s.to_string(),
                byte_len: validity.valid_upto,
            },
            Err(_) => Utf8String {
                str: String::new(),
                byte_len: 0,
            },
        }
    } else {
        Utf8String {
            str: String::new(),
            byte_len: 0,
        }
    }
}

pub fn validate_utf8(bytes: &[u8]) -> Utf8Validity {
    let mut offset: usize = 0;
    while offset < bytes.len() {
        let v = validate_utf8_char(bytes, offset);
        if v.valid {
            offset = v.next_offset;
        } else {
            return Utf8Validity {
                valid: false,
                valid_upto: offset,
            };
        }
    }
    Utf8Validity {
        valid: true,
        valid_upto: offset,
    }
}

pub fn is_utf8_char_boundary(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        // Matches C behavior where '\0' is considered a boundary
        return true;
    }
    bytes[0] <= 0b0111_1111 || bytes[0] >= 0b1100_0000
}

pub fn as_utf8_string(owned_str: &OwnedUtf8String) -> Utf8String {
    Utf8String {
        str: owned_str.str.clone(),
        byte_len: owned_str.byte_len,
    }
}

pub fn next_utf8_char(iter: &mut Utf8CharIter) -> Utf8Char {
    if iter.str.is_empty() {
        return Utf8Char {
            str: String::new(),
            byte_len: 0,
        };
    }

    let bytes = iter.str.as_bytes();
    let mut byte_len: usize = 1;

    // Find the next char boundary
    while byte_len < bytes.len() && !is_utf8_char_boundary(&bytes[byte_len..]) {
        byte_len += 1;
    }

    // Extract the current char's bytes and the remainder
    let char_str: String = match std::str::from_utf8(&bytes[..byte_len]) {
        Ok(s) => s.to_string(),
        Err(_) => String::new(),
    };
    let rest_str: String = match std::str::from_utf8(&bytes[byte_len..]) {
        Ok(s) => s.to_string(),
        Err(_) => String::new(),
    };

    iter.str = rest_str;

    Utf8Char {
        str: char_str,
        byte_len: byte_len as u8,
    }
}

pub fn make_utf8_string_lossy(bytes: &[u8]) -> OwnedUtf8String {
    let mut buffer: Vec<u8> = Vec::with_capacity(bytes.len() * 3 + 1);
    let mut offset: usize = 0;

    while offset < bytes.len() {
        let v = validate_utf8_char(bytes, offset);
        if v.valid {
            buffer.extend_from_slice(&bytes[offset..v.next_offset]);
            offset = v.next_offset;
        } else {
            // U+FFFD REPLACEMENT CHARACTER bytes: EF BF BD
            buffer.push(0xEF);
            buffer.push(0xBF);
            buffer.push(0xBD);
            offset += 1;
        }
    }

    let byte_len = buffer.len();
    let str = match String::from_utf8(buffer) {
        Ok(s) => s,
        Err(e) => {
            // Fallback: use lossy decoding from the original bytes if direct
            // construction fails (e.g. C-valid but non-standard UTF-8 encodings).
            let bytes_back = e.into_bytes();
            String::from_utf8_lossy(&bytes_back).into_owned()
        }
    };

    OwnedUtf8String { str, byte_len }
}

pub fn nth_utf8_char(ustr: Utf8String, char_index: usize) -> Utf8Char {
    let mut iter = make_utf8_char_iter(ustr);
    let mut remaining = char_index;
    loop {
        let ch = next_utf8_char(&mut iter);
        if ch.byte_len == 0 {
            return Utf8Char {
                str: String::new(),
                byte_len: 0,
            };
        }
        if remaining == 0 {
            return ch;
        }
        remaining -= 1;
    }
}
