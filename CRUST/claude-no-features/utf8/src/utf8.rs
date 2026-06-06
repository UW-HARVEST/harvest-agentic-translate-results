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
    let start_ok = is_utf8_char_boundary(&bytes[start..]);
    let end_ok = is_utf8_char_boundary(&bytes[end..]);

    if start_ok && end_ok {
        // Since `ustr.str` is a valid UTF-8 string and both `start` and `end`
        // are at UTF-8 char boundaries, slicing is safe and produces valid UTF-8.
        let s = ustr.str[start..end].to_string();
        Utf8String {
            str: s,
            byte_len: end - start,
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
            (((bytes[0] & 0b0001_1111) as u32) << 6)
                | ((bytes[1] & 0b0011_1111) as u32)
        }
        3 => {
            (((bytes[0] & 0b0000_1111) as u32) << 12)
                | (((bytes[1] & 0b0011_1111) as u32) << 6)
                | ((bytes[2] & 0b0011_1111) as u32)
        }
        4 => {
            (((bytes[0] & 0b0000_0111) as u32) << 18)
                | (((bytes[1] & 0b0011_1111) as u32) << 12)
                | (((bytes[2] & 0b0011_1111) as u32) << 6)
                | ((bytes[3] & 0b0011_1111) as u32)
        }
        _ => 0,
    }
}

pub fn free_owned_utf8_string(owned_str: &mut OwnedUtf8String) {
    if !owned_str.str.is_empty() {
        owned_str.str = String::new();
        owned_str.byte_len = 0;
    }
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
    Utf8CharIter { str: ustr.str }
}

pub fn validate_utf8_char(bytes: &[u8], offset: usize) -> Utf8CharValidity {
    let b0 = match bytes.get(offset) {
        Some(&b) => b,
        None => {
            return Utf8CharValidity {
                valid: false,
                next_offset: offset,
            };
        }
    };

    // Single-byte UTF-8 characters have the form 0xxxxxxx
    if b0 & 0b1000_0000 == 0 {
        return Utf8CharValidity {
            valid: true,
            next_offset: offset + 1,
        };
    }

    let b1 = bytes.get(offset + 1).copied();

    // Two-byte UTF-8 characters have the form 110xxxxx 10xxxxxx
    if b0 & 0b1110_0000 == 0b1100_0000
        && matches!(b1, Some(b) if b & 0b1100_0000 == 0b1000_0000)
    {
        // Check for overlong encoding
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

    let b2 = bytes.get(offset + 2).copied();

    // Three-byte UTF-8 characters have the form 1110xxxx 10xxxxxx 10xxxxxx
    if b0 & 0b1111_0000 == 0b1110_0000
        && matches!(b1, Some(b) if b & 0b1100_0000 == 0b1000_0000)
        && matches!(b2, Some(b) if b & 0b1100_0000 == 0b1000_0000)
    {
        let b1u = b1.unwrap();
        // Check for overlong encoding
        if b0 & 0b0000_1111 == 0 && b1u & 0b0011_1111 < 0b0010_0000 {
            return Utf8CharValidity {
                valid: false,
                next_offset: offset,
            };
        }
        // Reject UTF-16 surrogates (U+D800 to U+DFFF)
        if b0 == 0b1110_1101 && b1u >= 0b1010_0000 && b1u <= 0b1011_1111 {
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

    let b3 = bytes.get(offset + 3).copied();

    // Four-byte UTF-8 characters have the form 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
    if b0 & 0b1111_1000 == 0b1111_0000
        && matches!(b1, Some(b) if b & 0b1100_0000 == 0b1000_0000)
        && matches!(b2, Some(b) if b & 0b1100_0000 == 0b1000_0000)
        && matches!(b3, Some(b) if b & 0b1100_0000 == 0b1000_0000)
    {
        let b1u = b1.unwrap();
        // Check for overlong encoding
        if b0 & 0b0000_0111 == 0 && b1u & 0b0011_1111 < 0b0001_0000 {
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
        // Bytes are valid UTF-8 so this conversion always succeeds.
        let s = std::str::from_utf8(bytes)
            .expect("validated UTF-8")
            .to_string();
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
    let mut offset = 0;
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
    // The C version dereferences the pointer; for an empty slice (i.e. one
    // past the end of the string in C, where the byte would be '\0'), we
    // treat it as a boundary.
    if bytes.is_empty() {
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
    let bytes = iter.str.as_bytes();
    if bytes.is_empty() {
        return Utf8Char {
            str: String::new(),
            byte_len: 0,
        };
    }

    // The current char starts at byte 0 of `iter.str`. Find the next char
    // boundary by scanning forward.
    let mut byte_len: usize = 1;
    while byte_len < bytes.len() && !is_utf8_char_boundary(&bytes[byte_len..]) {
        byte_len += 1;
    }

    let curr_str = iter.str.clone();
    // `iter.str` is valid UTF-8 (it originates from a Rust String), and
    // `byte_len` is positioned at a UTF-8 char boundary, so this slice is
    // safe.
    iter.str = iter.str[byte_len..].to_string();

    Utf8Char {
        str: curr_str,
        byte_len: byte_len as u8,
    }
}

pub fn make_utf8_string_lossy(bytes: &[u8]) -> OwnedUtf8String {
    // Worst case: every byte is invalid and is replaced with 3 bytes for U+FFFD.
    let mut buffer: Vec<u8> = Vec::with_capacity(bytes.len() * 3);
    let mut offset = 0;

    while offset < bytes.len() {
        let v = validate_utf8_char(bytes, offset);
        if v.valid {
            buffer.extend_from_slice(&bytes[offset..v.next_offset]);
            offset = v.next_offset;
        } else {
            // Insert the UTF-8 bytes for U+FFFD (REPLACEMENT CHARACTER).
            buffer.extend_from_slice(&[0xEF, 0xBF, 0xBD]);
            offset += 1;
        }
    }

    let len = buffer.len();
    let s = String::from_utf8(buffer).expect("constructed valid UTF-8");
    OwnedUtf8String {
        str: s,
        byte_len: len,
    }
}

pub fn nth_utf8_char(ustr: Utf8String, char_index: usize) -> Utf8Char {
    let mut iter = make_utf8_char_iter(ustr);
    let mut idx = char_index;
    let mut ch;
    loop {
        ch = next_utf8_char(&mut iter);
        if ch.byte_len == 0 {
            break;
        }
        if idx == 0 {
            break;
        }
        idx -= 1;
    }

    if ch.byte_len == 0 {
        Utf8Char {
            str: String::new(),
            byte_len: 0,
        }
    } else {
        ch
    }
}
