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
    let b = bytes.as_bytes();
    StringT {
        bytes: b.to_vec(),
        size: b.len(),
    }
}
pub fn string_free(_str: StringT) {
    // In Rust, dropping happens automatically when StringT goes out of scope.
    // Taking ownership here ensures the value is consumed and freed.
}
pub fn string_len(str: &StringT) -> usize {
    str.size
}
pub fn string_bytes(str: &StringT) -> &str {
    let end = str.size.min(str.bytes.len());
    std::str::from_utf8(&str.bytes[..end]).unwrap_or("")
}
pub fn string_eq(left: &StringT, right: &StringT) -> BoolT {
    if left.size != right.size {
        return false;
    }
    let l_end = left.size.min(left.bytes.len());
    let r_end = right.size.min(right.bytes.len());
    left.bytes[..l_end] == right.bytes[..r_end]
}
pub fn string_copy(str: &StringT) -> StringT {
    StringT {
        bytes: str.bytes.clone(),
        size: str.size,
    }
}
pub fn string_concat(first: &StringT, second: &StringT) -> StringT {
    let mut bytes: Vec<u8> = Vec::with_capacity(first.size + second.size);
    let f_end = first.size.min(first.bytes.len());
    let s_end = second.size.min(second.bytes.len());
    bytes.extend_from_slice(&first.bytes[..f_end]);
    bytes.extend_from_slice(&second.bytes[..s_end]);
    // Pad with zeros if needed to match expected size
    while bytes.len() < first.size + second.size {
        bytes.push(0);
    }
    StringT {
        size: first.size + second.size,
        bytes,
    }
}
pub fn string_substr(str: &StringT, pos: usize, len: usize) -> StringT {
    let mut bytes = vec![0u8; len];
    for idx in 0..len {
        let src = pos.saturating_add(idx);
        if src < str.bytes.len() {
            bytes[idx] = str.bytes[src];
        }
    }
    StringT { bytes, size: len }
}
pub fn string_startswith(str: &StringT, prefix: &str) -> BoolT {
    let p = prefix.as_bytes();
    if str.size < p.len() {
        return false;
    }
    let end = p.len().min(str.bytes.len());
    if end < p.len() {
        return false;
    }
    &str.bytes[..p.len()] == p
}
pub fn string_endswith(str: &StringT, suffix: &str) -> BoolT {
    let s = suffix.as_bytes();
    if str.size < s.len() {
        return false;
    }
    if s.is_empty() {
        return true;
    }
    let start = str.size - s.len();
    if start + s.len() > str.bytes.len() {
        return false;
    }
    &str.bytes[start..start + s.len()] == s
}
pub fn string_find(str: &StringT, chars: &str) -> Option<usize> {
    let needle = chars.as_bytes();
    if needle.is_empty() {
        return Some(0);
    }
    if str.size < needle.len() {
        return None;
    }
    let last = str.size - needle.len();
    for pos in 0..=last {
        if pos + needle.len() > str.bytes.len() {
            break;
        }
        if &str.bytes[pos..pos + needle.len()] == needle {
            return Some(pos);
        }
    }
    None
}
pub fn string_strip(str: &StringT) -> StringT {
    let mut start_pos: usize = 0;
    while start_pos < str.size
        && start_pos < str.bytes.len()
        && string_t_is_space_char(str.bytes[start_pos])
    {
        start_pos += 1;
    }
    let mut end_pos: i64 = str.size as i64 - 1;
    while end_pos >= 0
        && (end_pos as usize) < str.bytes.len()
        && string_t_is_space_char(str.bytes[end_pos as usize])
    {
        end_pos -= 1;
    }
    if (start_pos as i64) >= end_pos {
        return string_copy(str);
    }
    string_substr(str, start_pos, (end_pos as usize) - start_pos + 1)
}
pub fn string_split(str: &StringT, arr_size: &mut usize) -> StringTArray {
    if str.size == 0 {
        *arr_size = 1;
        return vec![string_copy(str)];
    }

    let mut indexes: Vec<(usize, usize)> = Vec::new();
    let mut start_pos: usize = 0;
    let mut pos: usize = 0;
    let limit = str.size.min(str.bytes.len());

    while pos < limit {
        if string_t_is_space_char(str.bytes[pos]) {
            indexes.push((start_pos, pos));
            while pos < limit && string_t_is_space_char(str.bytes[pos]) {
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

    let mut result = Vec::with_capacity(indexes.len());
    for (s, e) in &indexes {
        result.push(string_substr(str, *s, e - s));
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
    let upper = str.size - split_size;
    let buf_len = str.bytes.len();

    while pos < upper {
        if pos + split_size <= buf_len && &str.bytes[pos..pos + split_size] == split_bytes {
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

    let mut result = Vec::with_capacity(indexes.len());
    for (s, e) in &indexes {
        result.push(string_substr(str, *s, e - s));
    }
    *arr_size = result.len();
    result
}
pub fn string_join_arr(str_arr: &StringTArray, arr_size: usize, space_chars: &str) -> StringT {
    let sep = space_chars.as_bytes();
    let total_strs_size: usize = (0..arr_size).map(|i| str_arr[i].size).sum();
    let total_size = total_strs_size + sep.len() * arr_size.saturating_sub(1);
    let mut bytes: Vec<u8> = Vec::with_capacity(total_size);
    for idx in 0..arr_size {
        let s = &str_arr[idx];
        let end = s.size.min(s.bytes.len());
        bytes.extend_from_slice(&s.bytes[..end]);
        // Pad with zeros if logical size exceeds actual buffer length
        let target = bytes.len() + (s.size - end);
        bytes.resize(target, 0);
        if idx + 1 != arr_size {
            bytes.extend_from_slice(sep);
        }
    }
    StringT {
        size: total_size,
        bytes,
    }
}
pub fn string_t_is_space_char(byte: u8) -> BoolT {
    STRING_T_SPACE_CHARS_ARR.as_bytes().contains(&byte)
}
