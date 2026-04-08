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
    StringT {
        bytes: bytes.as_bytes().to_vec(),
        size: bytes.len(),
    }
}
pub fn string_free(_str: StringT) {
    // Drop happens automatically in Rust
}
pub fn string_len(str: &StringT) -> usize {
    str.size
}
pub fn string_bytes(str: &StringT) -> &str {
    std::str::from_utf8(&str.bytes).unwrap_or("")
}
pub fn string_eq(left: &StringT, right: &StringT) -> BoolT {
    left.size == right.size && left.bytes == right.bytes
}
pub fn string_copy(str: &StringT) -> StringT {
    StringT {
        bytes: str.bytes.clone(),
        size: str.size,
    }
}
pub fn string_concat(first: &StringT, second: &StringT) -> StringT {
    let mut bytes = first.bytes.clone();
    bytes.extend_from_slice(&second.bytes);
    StringT {
        size: first.size + second.size,
        bytes,
    }
}
pub fn string_substr(str: &StringT, pos: usize, len: usize) -> StringT {
    let bytes = str.bytes[pos..pos + len].to_vec();
    StringT { bytes, size: len }
}
pub fn string_startswith(str: &StringT, prefix: &str) -> BoolT {
    if str.size < prefix.len() {
        return false;
    }
    &str.bytes[..prefix.len()] == prefix.as_bytes()
}
pub fn string_endswith(str: &StringT, suffix: &str) -> BoolT {
    if str.size < suffix.len() {
        return false;
    }
    &str.bytes[str.size - suffix.len()..] == suffix.as_bytes()
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
        if pos + chars_bytes.len() <= str.size && &str.bytes[pos..pos + chars_bytes.len()] == chars_bytes {
            return Some(pos);
        }
    }
    None
}
pub fn string_strip(str: &StringT) -> StringT {
    let space = STRING_T_SPACE_CHARS_ARR.as_bytes();
    let mut start = 0;
    while start < str.size && space.contains(&str.bytes[start]) {
        start += 1;
    }
    let mut end = str.size as isize - 1;
    while end >= 0 && space.contains(&str.bytes[end as usize]) {
        end -= 1;
    }
    if start as isize >= end {
        return string_copy(str);
    }
    string_substr(str, start, (end as usize) - start + 1)
}
pub fn string_split(str: &StringT, arr_size: &mut usize) -> StringTArray {
    if str.size == 0 {
        *arr_size = 1;
        return vec![string_copy(str)];
    }

    let mut indexes: Vec<(usize, usize)> = Vec::new();
    let mut start_pos = 0;
    let mut pos = 0;
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

    *arr_size = indexes.len();
    indexes.iter().map(|&(s, e)| string_substr(str, s, e - s)).collect()
}
pub fn string_split_by(str: &StringT, arr_size: &mut usize, split_chars: &str) -> StringTArray {
    let split_bytes = split_chars.as_bytes();
    let split_len = split_bytes.len();

    if str.size <= split_len {
        *arr_size = 1;
        return vec![string_copy(str)];
    }

    let mut indexes: Vec<(usize, usize)> = Vec::new();
    let mut start_pos = 0;
    let mut pos = 0;
    while pos < str.size - split_len {
        if &str.bytes[pos..pos + split_len] == split_bytes {
            indexes.push((start_pos, pos));
            start_pos = pos + split_len;
            pos += split_len;
        } else {
            pos += 1;
        }
    }
    if pos != start_pos {
        indexes.push((start_pos, pos + 1));
    }

    *arr_size = indexes.len();
    indexes.iter().map(|&(s, e)| string_substr(str, s, e - s)).collect()
}
pub fn string_join_arr(str_arr: &StringTArray, arr_size: usize, space_chars: &str) -> StringT {
    let sep = space_chars.as_bytes();
    let total_size: usize = str_arr[..arr_size].iter().map(|s| s.size).sum::<usize>()
        + sep.len() * (arr_size - 1);
    let mut bytes = Vec::with_capacity(total_size);
    for (i, s) in str_arr[..arr_size].iter().enumerate() {
        bytes.extend_from_slice(&s.bytes);
        if i != arr_size - 1 {
            bytes.extend_from_slice(sep);
        }
    }
    StringT { bytes, size: total_size }
}
pub fn string_t_is_space_char(byte: u8) -> BoolT {
    STRING_T_SPACE_CHARS_ARR.as_bytes().contains(&byte)
}
