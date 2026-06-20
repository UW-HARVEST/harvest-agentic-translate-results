// Constants
pub const STRING_T_INDEXES_BUFFER_SIZE: usize = 512;
pub const STRING_T_SPACE_CHARS_ARR: &str = " \t\n\r";
// Type Definitions
pub type BoolT = bool;
#[derive(Clone)]
pub struct StringT {
    pub bytes: Vec<u8>,
    pub size: usize,
}
pub type StringTArray = Vec<StringT>;
// Function Declarations
pub fn new_string(size: usize) -> StringT {
    StringT {
        bytes: vec![0; size],
        size,
    }
}
pub fn new_string_from_bytes(bytes: &str) -> StringT {
    let bytes = bytes.as_bytes().to_vec();
    let size = bytes.len();
    StringT { bytes, size }
}
pub fn string_free(_str: StringT) {
}
pub fn string_len(str: &StringT) -> usize {
    str.size
}
pub fn string_bytes(str: &StringT) -> &str {
    std::str::from_utf8(&str.bytes[..str.size]).unwrap_or_default()
}
pub fn string_eq(left: &StringT, right: &StringT) -> BoolT {
    left.size == right.size && left.bytes[..left.size] == right.bytes[..right.size]
}
pub fn string_copy(str: &StringT) -> StringT {
    str.clone()
}
pub fn string_concat(first: &StringT, second: &StringT) -> StringT {
    let mut bytes = Vec::with_capacity(first.size + second.size);
    bytes.extend_from_slice(&first.bytes[..first.size]);
    bytes.extend_from_slice(&second.bytes[..second.size]);
    StringT {
        size: bytes.len(),
        bytes,
    }
}
pub fn string_substr(str: &StringT, pos: usize, len: usize) -> StringT {
    let end = pos.saturating_add(len).min(str.size);
    let start = pos.min(end);
    let bytes = str.bytes[start..end].to_vec();
    StringT {
        size: bytes.len(),
        bytes,
    }
}
pub fn string_startswith(str: &StringT, prefix: &str) -> BoolT {
    let prefix = prefix.as_bytes();
    str.size >= prefix.len() && &str.bytes[..prefix.len()] == prefix
}
pub fn string_endswith(str: &StringT, suffix: &str) -> BoolT {
    let suffix = suffix.as_bytes();
    str.size >= suffix.len() && &str.bytes[str.size - suffix.len()..str.size] == suffix
}
pub fn string_find(str: &StringT, chars: &str) -> Option<usize> {
    if chars.is_empty() {
        return Some(0);
    }

    let needle = chars.as_bytes();
    if needle.len() > str.size {
        return None;
    }

    str.bytes[..str.size]
        .windows(needle.len())
        .position(|window| window == needle)
}
pub fn string_strip(str: &StringT) -> StringT {
    let mut start_pos = 0;
    while start_pos < str.size && string_t_is_space_char(str.bytes[start_pos]) {
        start_pos += 1;
    }

    let mut end_pos = str.size;
    while end_pos > 0 && string_t_is_space_char(str.bytes[end_pos - 1]) {
        end_pos -= 1;
    }

    if start_pos >= end_pos {
        return string_copy(str);
    }

    string_substr(str, start_pos, end_pos - start_pos)
}
pub fn string_split(str: &StringT, arr_size: &mut usize) -> StringTArray {
    if str.size == 0 {
        *arr_size = 1;
        return vec![string_copy(str)];
    }

    let mut parts = Vec::new();
    let mut start_pos = 0;
    let mut pos = 0;

    while pos < str.size {
        if string_t_is_space_char(str.bytes[pos]) {
            parts.push(string_substr(str, start_pos, pos - start_pos));
            while pos < str.size && string_t_is_space_char(str.bytes[pos]) {
                pos += 1;
            }
            start_pos = pos;
        } else {
            pos += 1;
        }
    }

    if pos != start_pos {
        parts.push(string_substr(str, start_pos, pos - start_pos));
    }

    *arr_size = parts.len();
    parts
}
pub fn string_split_by(str: &StringT, arr_size: &mut usize, split_chars: &str) -> StringTArray {
    let split_len = split_chars.len();
    if str.size <= split_len || split_len == 0 {
        *arr_size = 1;
        return vec![string_copy(str)];
    }

    let split_bytes = split_chars.as_bytes();
    let mut parts = Vec::new();
    let mut start_pos = 0;
    let mut pos = 0;

    while pos < str.size - split_len {
        if &str.bytes[pos..pos + split_len] == split_bytes {
            parts.push(string_substr(str, start_pos, pos - start_pos));
            start_pos = pos + split_len;
            pos += split_len;
        } else {
            pos += 1;
        }
    }

    if pos != start_pos {
        parts.push(string_substr(str, start_pos, pos + 1 - start_pos));
    }

    *arr_size = parts.len();
    parts
}
pub fn string_join_arr(str_arr: &StringTArray, arr_size: usize, space_chars: &str) -> StringT {
    let take_count = arr_size.min(str_arr.len());
    let separators = take_count.saturating_sub(1);
    let total_len = str_arr
        .iter()
        .take(take_count)
        .map(|s| s.size)
        .sum::<usize>()
        + space_chars.len() * separators;

    let mut bytes = Vec::with_capacity(total_len);
    for (idx, str) in str_arr.iter().take(take_count).enumerate() {
        bytes.extend_from_slice(&str.bytes[..str.size]);
        if idx + 1 != take_count {
            bytes.extend_from_slice(space_chars.as_bytes());
        }
    }

    StringT {
        size: bytes.len(),
        bytes,
    }
}
pub fn string_t_is_space_char(byte: u8) -> BoolT {
    STRING_T_SPACE_CHARS_ARR.as_bytes().contains(&byte)
}
