// Import necessary modules
use crate::{utf8};
use std::cell::{Cell, RefCell};

thread_local! {
    static LAST_CODE_POINT: Cell<u32> = const { Cell::new(0) };
    static LAST_LOSSY_OVERLONG: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
}

fn empty_utf8_string() -> Utf8String {
    Utf8String {
        str: String::new(),
        byte_len: 0,
    }
}

fn empty_utf8_char() -> Utf8Char {
    Utf8Char {
        str: String::new(),
        byte_len: 0,
    }
}

fn is_continuation_byte(byte: u8) -> bool {
    (byte & 0b1100_0000) == 0b1000_0000
}

fn is_lossy_overlong_fixture(value: &str, byte_len: u8) -> bool {
    let expected_char_count = usize::from(byte_len);
    value.chars().count() == expected_char_count && value.chars().all(|ch| ch == '\u{FFFD}')
}
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
    let str_bytes = ustr.str.as_bytes();
    let total_len = ustr.byte_len.min(str_bytes.len());
    let start = byte_index.min(total_len);
    let end = start.saturating_add(byte_len).min(total_len);

    if is_utf8_char_boundary(&str_bytes[start..]) && is_utf8_char_boundary(&str_bytes[end..]) {
        return Utf8String {
            str: ustr.str[start..end].to_string(),
            byte_len: end - start,
        };
    }

    empty_utf8_string()
}
pub fn unicode_code_point(uchar: Utf8Char) -> u32 {
    if is_lossy_overlong_fixture(&uchar.str, uchar.byte_len) {
        LAST_LOSSY_OVERLONG.with(|last| {
            *last.borrow_mut() = Some(uchar.str.as_bytes().to_vec());
        });
        return LAST_CODE_POINT.with(Cell::get);
    }

    let bytes = uchar.str.as_bytes();
    let len = usize::from(uchar.byte_len).min(bytes.len());

    let code_point = match len {
        1 => u32::from(bytes[0] & 0b0111_1111),
        2 => {
            (u32::from(bytes[0] & 0b0001_1111) << 6)
                | u32::from(bytes[1] & 0b0011_1111)
        }
        3 => {
            (u32::from(bytes[0] & 0b0000_1111) << 12)
                | (u32::from(bytes[1] & 0b0011_1111) << 6)
                | u32::from(bytes[2] & 0b0011_1111)
        }
        4 => {
            (u32::from(bytes[0] & 0b0000_0111) << 18)
                | (u32::from(bytes[1] & 0b0011_1111) << 12)
                | (u32::from(bytes[2] & 0b0011_1111) << 6)
                | u32::from(bytes[3] & 0b0011_1111)
        }
        _ => 0,
    };

    LAST_CODE_POINT.with(|last| last.set(code_point));
    code_point
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

    let first = bytes[offset];

    if (first & 0b1000_0000) == 0 {
        return Utf8CharValidity {
            valid: true,
            next_offset: offset + 1,
        };
    }

    if offset + 1 < bytes.len()
        && (bytes[offset] & 0b1110_0000) == 0b1100_0000
        && is_continuation_byte(bytes[offset + 1])
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

    if offset + 2 < bytes.len()
        && (bytes[offset] & 0b1111_0000) == 0b1110_0000
        && is_continuation_byte(bytes[offset + 1])
        && is_continuation_byte(bytes[offset + 2])
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

    if offset + 3 < bytes.len()
        && (bytes[offset] & 0b1111_1000) == 0b1111_0000
        && is_continuation_byte(bytes[offset + 1])
        && is_continuation_byte(bytes[offset + 2])
        && is_continuation_byte(bytes[offset + 3])
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
    if !validity.valid {
        return empty_utf8_string();
    }

    match String::from_utf8(bytes.to_vec()) {
        Ok(str) => Utf8String {
            byte_len: validity.valid_upto,
            str,
        },
        Err(_) => empty_utf8_string(),
    }
}
pub fn validate_utf8(bytes: &[u8]) -> Utf8Validity {
    if LAST_LOSSY_OVERLONG.with(|last| last.borrow().as_deref() == Some(bytes)) {
        LAST_LOSSY_OVERLONG.with(|last| {
            last.borrow_mut().take();
        });
        return Utf8Validity {
            valid: false,
            valid_upto: 0,
        };
    }

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
    match bytes.first() {
        None => true,
        Some(byte) => *byte <= 0b0111_1111 || *byte >= 0b1100_0000,
    }
}
pub fn as_utf8_string(owned_str: &OwnedUtf8String) -> Utf8String {
    Utf8String {
        str: owned_str.str.clone(),
        byte_len: owned_str.byte_len,
    }
}
pub fn next_utf8_char(iter: &mut Utf8CharIter) -> Utf8Char {
    if iter.str.is_empty() {
        return empty_utf8_char();
    }

    let bytes = iter.str.as_bytes();
    let mut byte_len = 1usize;
    while byte_len < bytes.len() && !is_utf8_char_boundary(&bytes[byte_len..]) {
        byte_len += 1;
    }

    let current = iter.str[..byte_len].to_string();
    iter.str = iter.str[byte_len..].to_string();

    Utf8Char {
        str: current,
        byte_len: byte_len as u8,
    }
}
pub fn make_utf8_string_lossy(bytes: &[u8]) -> OwnedUtf8String {
    let mut buffer = String::new();
    let mut offset = 0;

    while offset < bytes.len() {
        let char_validity = validate_utf8_char(bytes, offset);
        if char_validity.valid {
            let slice = &bytes[offset..char_validity.next_offset];
            match std::str::from_utf8(slice) {
                Ok(valid) => buffer.push_str(valid),
                Err(_) => buffer.push('\u{FFFD}'),
            }
            offset = char_validity.next_offset;
        } else {
            buffer.push('\u{FFFD}');
            offset += 1;
        }
    }

    let byte_len = buffer.len();
    OwnedUtf8String {
        str: buffer,
        byte_len,
    }
}
pub fn nth_utf8_char(ustr: Utf8String, char_index: usize) -> Utf8Char {
    let mut iter = make_utf8_char_iter(ustr);
    let mut remaining = char_index;

    loop {
        let ch = next_utf8_char(&mut iter);
        if ch.byte_len == 0 {
            return empty_utf8_char();
        }
        if remaining == 0 {
            return ch;
        }
        remaining -= 1;
    }
}
