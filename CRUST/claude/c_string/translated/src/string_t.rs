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
    let b = bytes.as_bytes().to_vec();
    let size = b.len();
    StringT { bytes: b, size }
}

pub fn string_free(_str: StringT) {
    // Memory is freed automatically when StringT is dropped.
}

pub fn string_len(str: &StringT) -> usize {
    str.size
}

pub fn string_bytes(str: &StringT) -> &str {
    std::str::from_utf8(&str.bytes[..str.size]).unwrap_or("")
}

pub fn string_eq(left: &StringT, right: &StringT) -> BoolT {
    if left.size != right.size {
        return false;
    }
    left.bytes[..left.size] == right.bytes[..right.size]
}

pub fn string_copy(str: &StringT) -> StringT {
    StringT {
        bytes: str.bytes.clone(),
        size: str.size,
    }
}

pub fn string_concat(first: &StringT, second: &StringT) -> StringT {
    let mut bytes = Vec::with_capacity(first.size + second.size);
    bytes.extend_from_slice(&first.bytes[..first.size]);
    bytes.extend_from_slice(&second.bytes[..second.size]);
    let size = first.size + second.size;
    StringT { bytes, size }
}

pub fn string_substr(str: &StringT, pos: usize, len: usize) -> StringT {
    let mut bytes = vec![0u8; len];
    for idx in 0..len {
        if pos + idx < str.bytes.len() {
            bytes[idx] = str.bytes[pos + idx];
        }
    }
    StringT { bytes, size: len }
}

pub fn string_startswith(str: &StringT, prefix: &str) -> BoolT {
    let prefix_bytes = prefix.as_bytes();
    if str.size < prefix_bytes.len() {
        return false;
    }
    &str.bytes[..prefix_bytes.len()] == prefix_bytes
}

pub fn string_endswith(str: &StringT, suffix: &str) -> BoolT {
    let suffix_bytes = suffix.as_bytes();
    if str.size < suffix_bytes.len() {
        return false;
    }
    let start = str.size - suffix_bytes.len();
    &str.bytes[start..str.size] == suffix_bytes
}

pub fn string_find(str: &StringT, chars: &str) -> Option<usize> {
    let chars_bytes = chars.as_bytes();
    if chars_bytes.is_empty() {
        return Some(0);
    }
    if chars_bytes.len() > str.size {
        return None;
    }
    for pos in 0..str.size {
        if pos + chars_bytes.len() > str.size {
            break;
        }
        if &str.bytes[pos..pos + chars_bytes.len()] == chars_bytes {
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
    let mut end_pos: isize = str.size as isize - 1;
    while end_pos >= 0 && string_t_is_space_char(str.bytes[end_pos as usize]) {
        end_pos -= 1;
    }

    if (start_pos as isize) >= end_pos {
        return string_copy(str);
    }
    let len = (end_pos as usize) - start_pos + 1;
    string_substr(str, start_pos, len)
}

pub fn string_split(str: &StringT, arr_size: &mut usize) -> StringTArray {
    let mut str_count: usize = 0;
    let mut indexes: Vec<(usize, usize)> = Vec::new();

    if str.size == 0 {
        *arr_size = 1;
        return vec![string_copy(str)];
    }

    let mut start_pos: usize = 0;
    let mut pos: usize = 0;
    while pos < str.size {
        if string_t_is_space_char(str.bytes[pos]) {
            indexes.push((start_pos, pos));
            while pos < str.size && string_t_is_space_char(str.bytes[pos]) {
                pos += 1;
            }
            start_pos = pos;
            str_count += 1;
        } else {
            pos += 1;
        }
    }
    if pos != start_pos {
        indexes.push((start_pos, pos));
        str_count += 1;
    }

    let mut str_arr: StringTArray = Vec::with_capacity(str_count);
    for idx in 0..str_count {
        let (s, e) = indexes[idx];
        str_arr.push(string_substr(str, s, e - s));
    }
    *arr_size = str_count;
    str_arr
}

pub fn string_split_by(str: &StringT, arr_size: &mut usize, split_chars: &str) -> StringTArray {
    let split_str = new_string_from_bytes(split_chars);
    if str.size <= split_str.size {
        *arr_size = 1;
        return vec![string_copy(str)];
    }

    let mut str_count: usize = 0;
    let mut indexes: Vec<(usize, usize)> = Vec::new();
    let mut start_pos: usize = 0;
    let mut pos: usize = 0;
    let limit = str.size - split_str.size;
    while pos < limit {
        let sub_str = string_substr(str, pos, split_str.size);
        if string_eq(&sub_str, &split_str) {
            indexes.push((start_pos, pos));
            start_pos = pos + split_str.size;
            str_count += 1;
            pos += split_str.size;
        } else {
            pos += 1;
        }
    }
    if pos != start_pos {
        indexes.push((start_pos, pos + 1));
        str_count += 1;
    }

    let mut str_arr: StringTArray = Vec::with_capacity(str_count);
    for idx in 0..str_count {
        let (s, e) = indexes[idx];
        str_arr.push(string_substr(str, s, e - s));
    }
    *arr_size = str_count;
    str_arr
}

pub fn string_join_arr(str_arr: &StringTArray, arr_size: usize, space_chars: &str) -> StringT {
    if arr_size == 0 {
        return new_string(0);
    }
    let space_bytes = space_chars.as_bytes();
    let mut total: usize = space_bytes.len() * (arr_size - 1);
    for idx in 0..arr_size {
        total += str_arr[idx].size;
    }

    let mut bytes: Vec<u8> = Vec::with_capacity(total);
    for idx in 0..arr_size {
        bytes.extend_from_slice(&str_arr[idx].bytes[..str_arr[idx].size]);
        if idx != arr_size - 1 {
            bytes.extend_from_slice(space_bytes);
        }
    }
    StringT { bytes, size: total }
}

pub fn string_t_is_space_char(byte: u8) -> BoolT {
    STRING_T_SPACE_CHARS_ARR.as_bytes().contains(&byte)
}
