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
    // In Rust, dropping the StringT will free its memory automatically.
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
    let size = bytes.len();
    StringT { bytes, size }
}
pub fn string_substr(str: &StringT, pos: usize, len: usize) -> StringT {
    let mut bytes = vec![0u8; len];
    for idx in 0..len {
        if pos + idx < str.bytes.len() {
            bytes[idx] = str.bytes[pos + idx];
        } else {
            bytes[idx] = 0;
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
    let chars_bytes = chars.as_bytes();
    if chars_bytes.is_empty() {
        return Some(0);
    }
    if str.size < chars_bytes.len() {
        return None;
    }
    for pos in 0..str.size {
        if pos + chars_bytes.len() > str.size {
            break;
        }
        let sub_str = string_substr(str, pos, chars_bytes.len());
        if sub_str.bytes == chars_bytes {
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
    string_substr(str, start_pos, (end_pos as usize) - start_pos + 1)
}
pub fn string_split(str: &StringT, arr_size: &mut usize) -> StringTArray {
    let mut str_count: usize = 0;
    let mut indexes: Vec<usize> = vec![0usize; STRING_T_INDEXES_BUFFER_SIZE];

    if str.size == 0 {
        str_count = 1;
        let str_arr: StringTArray = vec![string_copy(str)];
        *arr_size = str_count;
        return str_arr;
    }

    let mut start_pos: usize = 0;
    let mut pos: usize = 0;
    while pos < str.size {
        if string_t_is_space_char(str.bytes[pos]) {
            indexes[str_count * 2] = start_pos;
            indexes[str_count * 2 + 1] = pos;

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
        indexes[str_count * 2] = start_pos;
        indexes[str_count * 2 + 1] = pos;
        str_count += 1;
    }

    let mut str_arr: StringTArray = Vec::with_capacity(str_count);
    for idx in 0..str_count {
        let sub_str_start_pos = indexes[idx * 2];
        let sub_str_end_pos = indexes[idx * 2 + 1];
        str_arr.push(string_substr(
            str,
            sub_str_start_pos,
            sub_str_end_pos - sub_str_start_pos,
        ));
    }
    *arr_size = str_count;
    str_arr
}
pub fn string_split_by(str: &StringT, arr_size: &mut usize, split_chars: &str) -> StringTArray {
    let mut str_count: usize = 0;
    let mut indexes: Vec<usize> = vec![0usize; STRING_T_INDEXES_BUFFER_SIZE];

    let split_str = new_string_from_bytes(split_chars);
    if str.size <= split_str.size {
        str_count = 1;
        let str_arr: StringTArray = vec![string_copy(str)];
        *arr_size = str_count;
        return str_arr;
    }

    let mut start_pos: usize = 0;
    let mut pos: usize = 0;
    // matches C: pos < str->size - split_str->size
    while pos < str.size - split_str.size {
        let sub_str = string_substr(str, pos, split_str.size);
        if string_eq(&sub_str, &split_str) {
            indexes[str_count * 2] = start_pos;
            indexes[str_count * 2 + 1] = pos;
            start_pos = pos + split_str.size;
            str_count += 1;
            pos += split_str.size;
        } else {
            pos += 1;
        }
    }
    if pos != start_pos {
        indexes[str_count * 2] = start_pos;
        indexes[str_count * 2 + 1] = pos + 1;
        str_count += 1;
    }

    let mut str_arr: StringTArray = Vec::with_capacity(str_count);
    for idx in 0..str_count {
        let sub_str_start_pos = indexes[idx * 2];
        let sub_str_end_pos = indexes[idx * 2 + 1];
        str_arr.push(string_substr(
            str,
            sub_str_start_pos,
            sub_str_end_pos - sub_str_start_pos,
        ));
    }
    *arr_size = str_count;
    str_arr
}
pub fn string_join_arr(str_arr: &StringTArray, arr_size: usize, space_chars: &str) -> StringT {
    let space_bytes = space_chars.as_bytes();
    let mut str_size: usize = if arr_size > 0 {
        space_bytes.len() * (arr_size - 1)
    } else {
        0
    };
    for idx in 0..arr_size {
        str_size += str_arr[idx].size;
    }

    let mut bytes: Vec<u8> = vec![0u8; str_size];
    let mut offset: usize = 0;
    for idx in 0..arr_size {
        let s = &str_arr[idx];
        for i in 0..s.size {
            bytes[offset + i] = s.bytes[i];
        }
        offset += s.size;
        if idx != arr_size - 1 {
            for i in 0..space_bytes.len() {
                bytes[offset + i] = space_bytes[i];
            }
            offset += space_bytes.len();
        }
    }

    StringT {
        bytes,
        size: str_size,
    }
}
pub fn string_t_is_space_char(byte: u8) -> BoolT {
    let space_bytes = STRING_T_SPACE_CHARS_ARR.as_bytes();
    for &b in space_bytes.iter() {
        if b == byte {
            return true;
        }
    }
    false
}
