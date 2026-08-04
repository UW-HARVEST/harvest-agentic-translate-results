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

// Helper: Construct a String from bytes that passed our UTF-8 validation.
// Falls back to a lossy conversion if Rust's strict validator disagrees
// (e.g., for sequences in the F5..=F7 range that our validator accepts but
// Rust's stdlib rejects). This avoids using `unsafe`.
fn bytes_to_string(bytes: &[u8]) -> String {
    match String::from_utf8(bytes.to_vec()) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(&e.into_bytes()).into_owned(),
    }
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
    // Guard against ustr.byte_len being out of sync with the underlying buffer.
    let bytes_len = bytes.len();
    let safe_start = start.min(bytes_len);
    let safe_end = end.min(bytes_len);

    let start_is_boundary = is_utf8_char_boundary(&bytes[safe_start..]);
    let end_is_boundary = is_utf8_char_boundary(&bytes[safe_end..]);

    if start_is_boundary && end_is_boundary {
        let slice_bytes = &bytes[safe_start..safe_end];
        let s = bytes_to_string(slice_bytes);
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

    // Single-byte UTF-8: 0xxxxxxx
    if b0 & 0b1000_0000 == 0 {
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
        // Overlong encoding check
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
        // Overlong encoding check
        if b0 & 0b0000_1111 == 0 && bytes[offset + 1] & 0b0011_1111 < 0b0010_0000 {
            return Utf8CharValidity {
                valid: false,
                next_offset: offset,
            };
        }
        // Reject UTF-16 surrogates (U+D800 to U+DFFF)
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
        // Overlong encoding check
        if b0 & 0b0000_0111 == 0 && bytes[offset + 1] & 0b0011_1111 < 0b0001_0000 {
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
        let s = bytes_to_string(&bytes[..validity.valid_upto]);
        Utf8String {
            str: s,
            byte_len: validity.valid_upto,
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
    if bytes.is_empty() {
        // Equivalent to pointing at the terminating '\0' in C, which is a boundary.
        return true;
    }
    let b = bytes[0];
    b <= 0b0111_1111 || b >= 0b1100_0000
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
    while byte_len < bytes.len() && !is_utf8_char_boundary(&bytes[byte_len..]) {
        byte_len += 1;
    }

    let consumed_bytes = bytes[..byte_len].to_vec();
    let remaining_bytes = bytes[byte_len..].to_vec();

    let consumed = bytes_to_string(&consumed_bytes);
    iter.str = bytes_to_string(&remaining_bytes);

    Utf8Char {
        str: consumed,
        byte_len: byte_len as u8,
    }
}

pub fn make_utf8_string_lossy(bytes: &[u8]) -> OwnedUtf8String {
    let mut buffer: Vec<u8> = Vec::with_capacity(bytes.len() * 3 + 1);
    let mut offset: usize = 0;

    while offset < bytes.len() {
        let cv = validate_utf8_char(bytes, offset);
        if cv.valid {
            buffer.extend_from_slice(&bytes[offset..cv.next_offset]);
            offset = cv.next_offset;
        } else {
            // U+FFFD REPLACEMENT CHARACTER (EF BF BD)
            buffer.extend_from_slice(&[0xEF, 0xBF, 0xBD]);
            offset += 1;
        }
    }

    let byte_len = buffer.len();
    let s = bytes_to_string(&buffer);
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
