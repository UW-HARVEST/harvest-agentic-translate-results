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
    let start = byte_index.min(ustr.byte_len).min(ustr.str.len());
    let end = start.saturating_add(byte_len).min(ustr.byte_len).min(ustr.str.len());

    if is_utf8_char_boundary(&ustr.str.as_bytes()[start..]) && is_utf8_char_boundary(&ustr.str.as_bytes()[end..]) {
        return Utf8String {
            str: bytes_to_string(&ustr.str.as_bytes()[start..end]),
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
        1 if !bytes.is_empty() => u32::from(bytes[0] & 0b0111_1111),
        2 if bytes.len() >= 2 => {
            (u32::from(bytes[0] & 0b0001_1111) << 6) |
            u32::from(bytes[1] & 0b0011_1111)
        }
        3 if bytes.len() >= 3 => {
            (u32::from(bytes[0] & 0b0000_1111) << 12) |
            (u32::from(bytes[1] & 0b0011_1111) << 6) |
            u32::from(bytes[2] & 0b0011_1111)
        }
        4 if bytes.len() >= 4 => {
            (u32::from(bytes[0] & 0b0000_0111) << 18) |
            (u32::from(bytes[1] & 0b0011_1111) << 12) |
            (u32::from(bytes[2] & 0b0011_1111) << 6) |
            u32::from(bytes[3] & 0b0011_1111)
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

    // Single-byte UTF-8 characters have the form 0xxxxxxx
    if (bytes[offset] & 0b1000_0000) == 0b0000_0000 {
        return Utf8CharValidity {
            valid: true,
            next_offset: offset + 1,
        };
    }

    // Two-byte UTF-8 characters have the form 110xxxxx 10xxxxxx
    if offset + 1 < bytes.len()
        && (bytes[offset] & 0b1110_0000) == 0b1100_0000
        && (bytes[offset + 1] & 0b1100_0000) == 0b1000_0000
    {
        if (bytes[offset] & 0b0001_1111) < 0b0000_0010 {
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

    // Three-byte UTF-8 characters have the form 1110xxxx 10xxxxxx 10xxxxxx
    if offset + 2 < bytes.len()
        && (bytes[offset] & 0b1111_0000) == 0b1110_0000
        && (bytes[offset + 1] & 0b1100_0000) == 0b1000_0000
        && (bytes[offset + 2] & 0b1100_0000) == 0b1000_0000
    {
        if (bytes[offset] & 0b0000_1111) == 0
            && (bytes[offset + 1] & 0b0011_1111) < 0b0010_0000
        {
            return Utf8CharValidity {
                valid: false,
                next_offset: offset,
            };
        }

        if bytes[offset] == 0b1110_1101
            && (0b1010_0000..=0b1011_1111).contains(&bytes[offset + 1])
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

    // Four-byte UTF-8 characters have the form 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
    if offset + 3 < bytes.len()
        && (bytes[offset] & 0b1111_1000) == 0b1111_0000
        && (bytes[offset + 1] & 0b1100_0000) == 0b1000_0000
        && (bytes[offset + 2] & 0b1100_0000) == 0b1000_0000
        && (bytes[offset + 3] & 0b1100_0000) == 0b1000_0000
    {
        if (bytes[offset] & 0b0000_0111) == 0
            && (bytes[offset + 1] & 0b0011_1111) < 0b0001_0000
        {
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
        return Utf8String {
            str: bytes_to_string(bytes),
            byte_len: validity.valid_upto,
        };
    }

    Utf8String {
        str: String::new(),
        byte_len: 0,
    }
}
pub fn validate_utf8(bytes: &[u8]) -> Utf8Validity {
    let mut offset = 0;

    while offset < bytes.len() {
        let char_validity = validate_utf8_char(bytes, offset);
        if char_validity.valid {
            offset = char_validity.next_offset;
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
    bytes.first().is_none_or(|byte| *byte <= 0b0111_1111 || *byte >= 0b1100_0000)
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
    let mut byte_len = 1;

    while byte_len < bytes.len() && !is_utf8_char_boundary(&bytes[byte_len..]) {
        byte_len += 1;
    }

    let ch = Utf8Char {
        str: bytes_to_string(&bytes[..byte_len]),
        byte_len: byte_len as u8,
    };
    iter.str = bytes_to_string(&bytes[byte_len..]);
    ch
}
pub fn make_utf8_string_lossy(bytes: &[u8]) -> OwnedUtf8String {
    let mut output = Vec::with_capacity(bytes.len().saturating_mul(3));
    let mut offset = 0;

    while offset < bytes.len() {
        let char_validity = validate_utf8_char(bytes, offset);
        if char_validity.valid {
            output.extend_from_slice(&bytes[offset..char_validity.next_offset]);
            offset = char_validity.next_offset;
        } else {
            output.extend_from_slice(&[0xEF, 0xBF, 0xBD]);
            offset += 1;
        }
    }

    let str = String::from_utf8(output).unwrap_or_else(|_| String::new());
    let byte_len = str.len();
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

fn bytes_to_string(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| String::from_utf8_lossy(bytes).into_owned())
}
