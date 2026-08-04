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
    // In Rust, dropping handles deallocation automatically.
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
    let mut copied = new_string(str.size);
    let end = str.size.min(str.bytes.len());
    copied.bytes[..end].copy_from_slice(&str.bytes[..end]);
    copied
}
pub fn string_concat(first: &StringT, second: &StringT) -> StringT {
    let total = first.size + second.size;
    let mut new_str = new_string(total);
    let f_end = first.size.min(first.bytes.len());
    let s_end = second.size.min(second.bytes.len());
    new_str.bytes[..f_end].copy_from_slice(&first.bytes[..f_end]);
    new_str.bytes[first.size..first.size + s_end].copy_from_slice(&second.bytes[..s_end]);
    new_str
}
pub fn string_substr(str: &StringT, pos: usize, len: usize) -> StringT {
    let mut sub = new_string(len);
    for idx in 0..len {
        let src_idx = pos + idx;
        if src_idx < str.bytes.len() {
            sub.bytes[idx] = str.bytes[src_idx];
        } else {
            sub.bytes[idx] = 0;
        }
    }
    sub
}
pub fn string_startswith(str: &StringT, prefix: &str) -> BoolT {
    let prefix_bytes = prefix.as_bytes();
    if str.size < prefix_bytes.len() {
        return false;
    }
    let exp_prefix = new_string_from_bytes(prefix);
    let str_prefix = string_substr(str, 0, exp_prefix.size);
    string_eq(&str_prefix, &exp_prefix)
}
pub fn string_endswith(str: &StringT, suffix: &str) -> BoolT {
    let suffix_bytes = suffix.as_bytes();
    if str.size < suffix_bytes.len() {
        return false;
    }
    let exp_suffix = new_string_from_bytes(suffix);
    let str_suffix = string_substr(str, str.size - exp_suffix.size, exp_suffix.size);
    string_eq(&str_suffix, &exp_suffix)
}
pub fn string_find(str: &StringT, chars: &str) -> Option<usize> {
    let chars_str = new_string_from_bytes(chars);
    if chars_str.size == 0 {
        return Some(0);
    }
    if chars_str.size > str.size {
        return None;
    }
    for pos in 0..=(str.size - chars_str.size) {
        let sub_str = string_substr(str, pos, chars_str.size);
        if string_eq(&sub_str, &chars_str) {
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
    let len = (end_pos - start_pos as i64 + 1) as usize;
    string_substr(str, start_pos, len)
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
            // Skip consecutive whitespace
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

    let mut str_arr: StringTArray = Vec::new();
    for (s, e) in &indexes {
        str_arr.push(string_substr(str, *s, e - s));
    }
    *arr_size = str_arr.len();
    str_arr
}
pub fn string_split_by(str: &StringT, arr_size: &mut usize, split_chars: &str) -> StringTArray {
    let split_str = new_string_from_bytes(split_chars);

    if str.size <= split_str.size {
        *arr_size = 1;
        return vec![string_copy(str)];
    }

    let mut indexes: Vec<(usize, usize)> = Vec::new();
    let mut start_pos: usize = 0;
    let mut pos: usize = 0;

    // Mirroring the C loop bound: `pos < str->size - split_str->size`
    let upper_bound = str.size - split_str.size;
    while pos < upper_bound {
        let sub_str = string_substr(str, pos, split_str.size);
        if string_eq(&sub_str, &split_str) {
            indexes.push((start_pos, pos));
            start_pos = pos + split_str.size;
            pos += split_str.size;
        } else {
            pos += 1;
        }
    }
    if pos != start_pos {
        // Note: matches C code which uses `pos + 1` for the trailing piece's end
        let end = pos + 1;
        let end = end.min(str.size);
        indexes.push((start_pos, end));
    }

    let mut str_arr: StringTArray = Vec::new();
    for (s, e) in &indexes {
        str_arr.push(string_substr(str, *s, e - s));
    }
    *arr_size = str_arr.len();
    str_arr
}
pub fn string_join_arr(str_arr: &StringTArray, arr_size: usize, space_chars: &str) -> StringT {
    let space_bytes = space_chars.as_bytes();
    let space_len = space_bytes.len();

    if arr_size == 0 {
        return new_string(0);
    }

    let mut str_size: usize = space_len * (arr_size - 1);
    for idx in 0..arr_size {
        str_size += str_arr[idx].size;
    }

    let mut join_str = new_string(str_size);
    let mut offset: usize = 0;
    for idx in 0..arr_size {
        let s = &str_arr[idx];
        let s_end = s.size.min(s.bytes.len());
        join_str.bytes[offset..offset + s_end].copy_from_slice(&s.bytes[..s_end]);
        offset += s.size;
        if idx != arr_size - 1 {
            join_str.bytes[offset..offset + space_len].copy_from_slice(space_bytes);
            offset += space_len;
        }
    }
    join_str
}
pub fn string_t_is_space_char(byte: u8) -> BoolT {
    STRING_T_SPACE_CHARS_ARR.as_bytes().contains(&byte)
}
