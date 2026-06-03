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
    let mut end = start + byte_len;
    if end > ustr.byte_len {
        end = ustr.byte_len;
    }

    let bytes = ustr.str.as_bytes();
    let start_is_boundary = is_utf8_char_boundary(&bytes[start..]);
    let end_is_boundary = is_utf8_char_boundary(&bytes[end..]);

    if start_is_boundary && end_is_boundary {
        // Mirror the C behavior: return a pointer offset into the original string,
        // i.e. the substring from `start` to the end of the original string,
        // with byte_len set to (end - start).
        let new_str = match std::str::from_utf8(&bytes[start..]) {
            Ok(s) => s.to_string(),
            Err(_) => String::new(),
        };
        Utf8String {
            str: new_str,
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
        _ => 0, // unreachable
    }
}
pub fn free_owned_utf8_string(owned_str: &mut OwnedUtf8String) {
    owned_str.str = String::new();
    owned_str.byte_len = 0;
}
pub fn utf8_char_count(ustr: Utf8String) -> usize {
    let mut iter = make_utf8_char_iter(ustr);
    let mut count = 0usize;
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
    if b0 & 0b1000_0000 == 0b0000_0000 {
        return Utf8CharValidity {
            valid: true,
            next_offset: offset + 1,
        };
    }

    // Two-byte: 110xxxxx 10xxxxxx
    if offset + 1 < bytes.len()
        && (bytes[offset] & 0b1110_0000) == 0b1100_0000
        && (bytes[offset + 1] & 0b1100_0000) == 0b1000_0000
    {
        // Overlong check
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

    // Three-byte: 1110xxxx 10xxxxxx 10xxxxxx
    if offset + 2 < bytes.len()
        && (bytes[offset] & 0b1111_0000) == 0b1110_0000
        && (bytes[offset + 1] & 0b1100_0000) == 0b1000_0000
        && (bytes[offset + 2] & 0b1100_0000) == 0b1000_0000
    {
        // Overlong check
        if (bytes[offset] & 0b0000_1111) == 0b0000_0000
            && (bytes[offset + 1] & 0b0011_1111) < 0b0010_0000
        {
            return Utf8CharValidity {
                valid: false,
                next_offset: offset,
            };
        }
        // Reject UTF-16 surrogates: U+D800 to U+DFFF (ED A0 80 to ED BF BF)
        if bytes[offset] == 0b1110_1101
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

    // Four-byte: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
    if offset + 3 < bytes.len()
        && (bytes[offset] & 0b1111_1000) == 0b1111_0000
        && (bytes[offset + 1] & 0b1100_0000) == 0b1000_0000
        && (bytes[offset + 2] & 0b1100_0000) == 0b1000_0000
        && (bytes[offset + 3] & 0b1100_0000) == 0b1000_0000
    {
        // Overlong check
        if (bytes[offset] & 0b0000_0111) == 0b0000_0000
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
        let s = std::str::from_utf8(&bytes[..validity.valid_upto])
            .unwrap_or("")
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
    let mut offset = 0usize;
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
        // Mirrors the C behavior where '\0' (end of string) is a valid boundary.
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

    let curr_str = iter.str.clone();
    let bytes = iter.str.as_bytes();
    let mut byte_len: u8 = 1;

    while (byte_len as usize) < bytes.len()
        && !is_utf8_char_boundary(&bytes[byte_len as usize..])
    {
        byte_len += 1;
    }

    // Advance the iterator past the consumed character.
    let remaining = match std::str::from_utf8(&bytes[byte_len as usize..]) {
        Ok(s) => s.to_string(),
        Err(_) => String::new(),
    };
    iter.str = remaining;

    Utf8Char {
        str: curr_str,
        byte_len,
    }
}
pub fn make_utf8_string_lossy(bytes: &[u8]) -> OwnedUtf8String {
    let mut buffer: Vec<u8> = Vec::with_capacity(bytes.len() * 3 + 1);
    let mut offset = 0usize;

    while offset < bytes.len() {
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

    OwnedUtf8String {
        str: s,
        byte_len,
    }
}
pub fn nth_utf8_char(ustr: Utf8String, char_index: usize) -> Utf8Char {
    let mut iter = make_utf8_char_iter(ustr);
    let mut idx = char_index;
    let mut ch;
    loop {
        ch = next_utf8_char(&mut iter);
        if ch.byte_len == 0 || idx == 0 {
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
