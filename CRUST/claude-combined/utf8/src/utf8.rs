// Import necessary modules
#[allow(unused_imports)]
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
    let bytes = ustr.str.as_bytes();
    let mut start = byte_index;
    if start > ustr.byte_len {
        start = ustr.byte_len;
    }
    let mut end = start.saturating_add(byte_len);
    if end > ustr.byte_len {
        end = ustr.byte_len;
    }

    if is_utf8_char_boundary(&bytes[start..]) && is_utf8_char_boundary(&bytes[end..]) {
        let s = std::str::from_utf8(&bytes[start..end])
            .map(|s| s.to_string())
            .unwrap_or_default();
        return Utf8String {
            str: s,
            byte_len: end - start,
        };
    }
    Utf8String {
        str: String::new(),
        byte_len: 0,
    }
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

pub fn free_owned_utf8_string(_owned_str: &mut OwnedUtf8String) {
    _owned_str.str.clear();
    _owned_str.byte_len = 0;
}

pub fn utf8_char_count(ustr: Utf8String) -> usize {
    let mut iter = make_utf8_char_iter(ustr);
    let mut count = 0;
    while next_utf8_char(&mut iter).byte_len > 0 {
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

    // Single-byte UTF-8 characters have the form 0xxxxxxx
    if b0 & 0b1000_0000 == 0 {
        return Utf8CharValidity {
            valid: true,
            next_offset: offset + 1,
        };
    }

    let b1 = bytes.get(offset + 1).copied();
    let b2 = bytes.get(offset + 2).copied();
    let b3 = bytes.get(offset + 3).copied();

    // Two-byte UTF-8 characters: 110xxxxx 10xxxxxx
    if let Some(b1) = b1 {
        if b0 & 0b1110_0000 == 0b1100_0000 && b1 & 0b1100_0000 == 0b1000_0000 {
            // Reject overlong (must be >= C2)
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

        // Three-byte UTF-8 characters: 1110xxxx 10xxxxxx 10xxxxxx
        if let Some(b2) = b2 {
            if b0 & 0b1111_0000 == 0b1110_0000
                && b1 & 0b1100_0000 == 0b1000_0000
                && b2 & 0b1100_0000 == 0b1000_0000
            {
                // Reject overlong
                if b0 & 0b0000_1111 == 0 && b1 & 0b0011_1111 < 0b0010_0000 {
                    return Utf8CharValidity {
                        valid: false,
                        next_offset: offset,
                    };
                }
                // Reject UTF-16 surrogates U+D800..U+DFFF
                if b0 == 0b1110_1101 && (0b1010_0000..=0b1011_1111).contains(&b1) {
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

            // Four-byte UTF-8 characters: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
            if let Some(b3) = b3 {
                if b0 & 0b1111_1000 == 0b1111_0000
                    && b1 & 0b1100_0000 == 0b1000_0000
                    && b2 & 0b1100_0000 == 0b1000_0000
                    && b3 & 0b1100_0000 == 0b1000_0000
                {
                    // Reject overlong
                    if b0 & 0b0000_0111 == 0 && b1 & 0b0011_1111 < 0b0001_0000 {
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
            }
        }
    }

    Utf8CharValidity {
        valid: false,
        next_offset: offset,
    }
}

pub fn make_utf8_string(bytes: &[u8]) -> Utf8String {
    let v = validate_utf8(bytes);
    if v.valid {
        let s = std::str::from_utf8(&bytes[..v.valid_upto])
            .map(|s| s.to_string())
            .unwrap_or_default();
        Utf8String {
            str: s,
            byte_len: v.valid_upto,
        }
    } else {
        Utf8String {
            str: String::new(),
            byte_len: 0,
        }
    }
}

pub fn validate_utf8(bytes: &[u8]) -> Utf8Validity {
    let mut offset = 0;
    while offset < bytes.len() {
        let cv = validate_utf8_char(bytes, offset);
        if cv.valid {
            offset = cv.next_offset;
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
    match bytes.first() {
        // Empty input corresponds to past-the-end '\0' in C, which is a boundary.
        None => true,
        Some(&b) => b <= 0b0111_1111 || b >= 0b1100_0000,
    }
}

pub fn as_utf8_string(owned_str: &OwnedUtf8String) -> Utf8String {
    Utf8String {
        str: owned_str.str.clone(),
        byte_len: owned_str.byte_len,
    }
}

pub fn next_utf8_char(iter: &mut Utf8CharIter) -> Utf8Char {
    let bytes = iter.str.as_bytes();
    if bytes.is_empty() {
        return Utf8Char {
            str: String::new(),
            byte_len: 0,
        };
    }

    // The current position is at a char boundary. Find next char boundary.
    let mut byte_len: usize = 1;
    while byte_len < bytes.len() && !is_utf8_char_boundary(&bytes[byte_len..]) {
        byte_len += 1;
    }

    let ch_str = std::str::from_utf8(&bytes[..byte_len])
        .map(|s| s.to_string())
        .unwrap_or_default();
    let rest = std::str::from_utf8(&bytes[byte_len..])
        .map(|s| s.to_string())
        .unwrap_or_default();
    iter.str = rest;
    Utf8Char {
        str: ch_str,
        byte_len: byte_len as u8,
    }
}

pub fn make_utf8_string_lossy(bytes: &[u8]) -> OwnedUtf8String {
    // Worst case: every byte invalid -> replaced with 3-byte U+FFFD
    let mut buffer: Vec<u8> = Vec::with_capacity(bytes.len() * 3 + 1);
    let mut offset = 0;
    while offset < bytes.len() {
        let cv = validate_utf8_char(bytes, offset);
        if cv.valid {
            buffer.extend_from_slice(&bytes[offset..cv.next_offset]);
            offset = cv.next_offset;
        } else {
            // U+FFFD REPLACEMENT CHARACTER = EF BF BD
            buffer.extend_from_slice(&[0xEF, 0xBF, 0xBD]);
            offset += 1;
        }
    }
    let byte_len = buffer.len();
    // Buffer is guaranteed valid UTF-8 by construction.
    let s = String::from_utf8(buffer).unwrap_or_default();
    OwnedUtf8String { str: s, byte_len }
}

pub fn nth_utf8_char(ustr: Utf8String, char_index: usize) -> Utf8Char {
    let mut iter = make_utf8_char_iter(ustr);
    let mut idx = char_index;
    loop {
        let ch = next_utf8_char(&mut iter);
        if ch.byte_len == 0 {
            return Utf8Char {
                str: String::new(),
                byte_len: 0,
            };
        }
        if idx == 0 {
            return ch;
        }
        idx -= 1;
    }
}
