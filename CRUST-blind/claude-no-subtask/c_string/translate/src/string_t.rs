// Import necessary modules
use crate::{string_t};
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
        bytes: vec![0u8; size],
        size,
    }
}
pub fn new_string_from_bytes(bytes: &str) -> StringT {
    let raw = bytes.as_bytes();
    StringT {
        bytes: raw.to_vec(),
        size: raw.len(),
    }
}
pub fn string_free(_str: StringT) {
    // Memory is automatically freed when `_str` goes out of scope.
}
pub fn string_len(str: &StringT) -> usize {
    str.size
}
pub fn string_bytes(str: &StringT) -> &str {
    std::str::from_utf8(&str.bytes).unwrap_or("")
}
pub fn string_eq(left: &StringT, right: &StringT) -> BoolT {
    if left.size != right.size {
        return false;
    }
    left.bytes == right.bytes
}
pub fn string_copy(str: &StringT) -> StringT {
    StringT {
        bytes: str.bytes.clone(),
        size: str.size,
    }
}
pub fn string_concat(first: &StringT, second: &StringT) -> StringT {
    let mut bytes = Vec::with_capacity(first.size + second.size);
    bytes.extend_from_slice(&first.bytes);
    bytes.extend_from_slice(&second.bytes);
    StringT {
        bytes,
        size: first.size + second.size,
    }
}
pub fn string_substr(str: &StringT, pos: usize, len: usize) -> StringT {
    let mut bytes = vec![0u8; len];
    for idx in 0..len {
        let src_idx = pos + idx;
        if src_idx < str.bytes.len() {
            bytes[idx] = str.bytes[src_idx];
        }
    }
    StringT { bytes, size: len }
}
pub fn string_startswith(str: &StringT, prefix: &str) -> BoolT {
    let prefix_bytes = prefix.as_bytes();
    if str.size < prefix_bytes.len() {
        return false;
    }
    let str_prefix = string_substr(str, 0, prefix_bytes.len());
    str_prefix.bytes == prefix_bytes
}
pub fn string_endswith(str: &StringT, suffix: &str) -> BoolT {
    let suffix_bytes = suffix.as_bytes();
    if str.size < suffix_bytes.len() {
        return false;
    }
    let str_suffix = string_substr(str, str.size - suffix_bytes.len(), suffix_bytes.len());
    str_suffix.bytes == suffix_bytes
}
pub fn string_find(str: &StringT, chars: &str) -> Option<usize> {
    let needle = chars.as_bytes();
    let needle_len = needle.len();
    if needle_len == 0 {
        return Some(0);
    }
    if needle_len > str.size {
        return None;
    }
    for pos in 0..=(str.size - needle_len) {
        if &str.bytes[pos..pos + needle_len] == needle {
            return Some(pos);
        }
    }
    None
}
pub fn string_strip(str: &StringT) -> StringT {
    let mut start_pos: usize = 0;
    while start_pos < str.size && string_t_is_space_char(str.bytes[start_pos]) {
        start_pos += 1;
    }
    let mut end_pos: i64 = str.size as i64 - 1;
    while end_pos >= 0 && string_t_is_space_char(str.bytes[end_pos as usize]) {
        end_pos -= 1;
    }
    if (start_pos as i64) >= end_pos {
        return string_copy(str);
    }
    let end = end_pos as usize;
    string_substr(str, start_pos, end - start_pos + 1)
}
pub fn string_split(str: &StringT, arr_size: &mut usize) -> StringTArray {
    if str.size == 0 {
        *arr_size = 1;
        return vec![string_copy(str)];
    }

    let mut indexes: Vec<(usize, usize)> = Vec::new();
    let mut start_pos: usize = 0;
    let mut pos: usize = 0;
    while pos < str.size {
        if string_t_is_space_char(str.bytes[pos]) {
            indexes.push((start_pos, pos));
            while pos < str.size && string_t_is_space_char(str.bytes[pos]) {
                pos += 1;
            }
            start_pos = pos;
        } else {
            pos += 1;
        }
    }
    if pos != start_pos {
        indexes.push((start_pos, pos));
    }

    let mut result: StringTArray = Vec::with_capacity(indexes.len());
    for (s, e) in &indexes {
        result.push(string_substr(str, *s, *e - *s));
    }
    *arr_size = result.len();
    result
}
pub fn string_split_by(str: &StringT, arr_size: &mut usize, split_chars: &str) -> StringTArray {
    let split_bytes = split_chars.as_bytes();
    let split_size = split_bytes.len();

    if str.size <= split_size {
        *arr_size = 1;
        return vec![string_copy(str)];
    }

    let mut indexes: Vec<(usize, usize)> = Vec::new();
    let mut start_pos: usize = 0;
    let mut pos: usize = 0;

    while pos < str.size - split_size {
        let window = &str.bytes[pos..pos + split_size];
        if window == split_bytes {
            indexes.push((start_pos, pos));
            start_pos = pos + split_size;
            pos += split_size;
        } else {
            pos += 1;
        }
    }
    if pos != start_pos {
        indexes.push((start_pos, pos + 1));
    }

    let mut result: StringTArray = Vec::with_capacity(indexes.len());
    for (s, e) in &indexes {
        result.push(string_substr(str, *s, *e - *s));
    }
    *arr_size = result.len();
    result
}
pub fn string_join_arr(str_arr: &StringTArray, arr_size: usize, space_chars: &str) -> StringT {
    let space_bytes = space_chars.as_bytes();
    let space_len = space_bytes.len();

    let mut str_size: usize = if arr_size > 0 {
        space_len * (arr_size - 1)
    } else {
        0
    };
    for idx in 0..arr_size {
        str_size += str_arr[idx].size;
    }

    let mut bytes: Vec<u8> = Vec::with_capacity(str_size);
    for idx in 0..arr_size {
        bytes.extend_from_slice(&str_arr[idx].bytes);
        if idx + 1 != arr_size {
            bytes.extend_from_slice(space_bytes);
        }
    }

    StringT {
        bytes,
        size: str_size,
    }
}
pub fn string_t_is_space_char(byte: u8) -> BoolT {
    STRING_T_SPACE_CHARS_ARR.as_bytes().contains(&byte)
}
