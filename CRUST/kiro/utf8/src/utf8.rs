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

// Helper: offset into the iter's string
// We store the full string and track position via trimming from front.

pub fn validate_utf8_char(bytes: &[u8], offset: usize) -> Utf8CharValidity {
    let b = bytes;
    // 1-byte: 0xxxxxxx
    if (b[offset] & 0x80) == 0x00 {
        return Utf8CharValidity { valid: true, next_offset: offset + 1 };
    }
    // 2-byte: 110xxxxx 10xxxxxx
    if offset + 1 < b.len()
        && (b[offset] & 0xE0) == 0xC0
        && (b[offset + 1] & 0xC0) == 0x80
    {
        if (b[offset] & 0x1F) < 0x02 {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        return Utf8CharValidity { valid: true, next_offset: offset + 2 };
    }
    // 3-byte: 1110xxxx 10xxxxxx 10xxxxxx
    if offset + 2 < b.len()
        && (b[offset] & 0xF0) == 0xE0
        && (b[offset + 1] & 0xC0) == 0x80
        && (b[offset + 2] & 0xC0) == 0x80
    {
        // overlong
        if (b[offset] & 0x0F) == 0x00 && (b[offset + 1] & 0x3F) < 0x20 {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        // surrogates U+D800..U+DFFF
        if b[offset] == 0xED && b[offset + 1] >= 0xA0 && b[offset + 1] <= 0xBF {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        return Utf8CharValidity { valid: true, next_offset: offset + 3 };
    }
    // 4-byte: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
    if offset + 3 < b.len()
        && (b[offset] & 0xF8) == 0xF0
        && (b[offset + 1] & 0xC0) == 0x80
        && (b[offset + 2] & 0xC0) == 0x80
        && (b[offset + 3] & 0xC0) == 0x80
    {
        // overlong
        if (b[offset] & 0x07) == 0x00 && (b[offset + 1] & 0x3F) < 0x10 {
            return Utf8CharValidity { valid: false, next_offset: offset };
        }
        return Utf8CharValidity { valid: true, next_offset: offset + 4 };
    }
    Utf8CharValidity { valid: false, next_offset: offset }
}

pub fn validate_utf8(bytes: &[u8]) -> Utf8Validity {
    if bytes.is_empty() {
        return Utf8Validity { valid: true, valid_upto: 0 };
    }
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
    let v = validate_utf8(bytes);
    if v.valid {
        // Safety: we just validated it's valid UTF-8
        let s = std::str::from_utf8(bytes).unwrap_or("").to_string();
        Utf8String { byte_len: v.valid_upto, str: s }
    } else {
        Utf8String { str: String::new(), byte_len: 0 }
    }
}

pub fn make_utf8_string_lossy(bytes: &[u8]) -> OwnedUtf8String {
    if bytes.is_empty() {
        return OwnedUtf8String { str: String::new(), byte_len: 0 };
    }
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
    let s = String::from_utf8(result).unwrap_or_default();
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
    if bytes.is_empty() {
        return true;
    }
    bytes[0] <= 0x7F || bytes[0] >= 0xC0
}

pub fn slice_utf8_string(ustr: Utf8String, byte_index: usize, byte_len: usize) -> Utf8String {
    let s = ustr.str.as_bytes();
    let len = ustr.byte_len;

    let start = if byte_index > len { len } else { byte_index };
    let mut end = start.saturating_add(byte_len);
    if end > len { end = len; }

    if is_utf8_char_boundary(&s[start..]) && is_utf8_char_boundary(&s[end..]) {
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
    if iter.str.is_empty() {
        return Utf8Char { str: String::new(), byte_len: 0 };
    }
    let bytes = iter.str.as_bytes();
    let mut char_len: u8 = 1;
    while (char_len as usize) < bytes.len() && !is_utf8_char_boundary(&bytes[char_len as usize..]) {
        char_len += 1;
    }
    let result_str = iter.str.clone();
    iter.str = iter.str[char_len as usize..].to_string();
    Utf8Char { str: result_str, byte_len: char_len }
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
        1 => (b[0] & 0x7F) as u32,
        2 => ((b[0] & 0x1F) as u32) << 6 | (b[1] & 0x3F) as u32,
        3 => ((b[0] & 0x0F) as u32) << 12 | ((b[1] & 0x3F) as u32) << 6 | (b[2] & 0x3F) as u32,
        4 => ((b[0] & 0x07) as u32) << 18 | ((b[1] & 0x3F) as u32) << 12 | ((b[2] & 0x3F) as u32) << 6 | (b[3] & 0x3F) as u32,
        _ => 0,
    }
}
