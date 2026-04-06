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
    // no-op in Rust; drop handles deallocation
}
pub fn string_len(str: &StringT) -> usize {
    str.size
}
pub fn string_bytes(str: &StringT) -> &str {
    std::str::from_utf8(&str.bytes).unwrap()
}
pub fn string_eq(left: &StringT, right: &StringT) -> BoolT {
    left.size == right.size && left.bytes == right.bytes
}
pub fn string_copy(str: &StringT) -> StringT {
    str.clone()
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
    StringT {
        bytes: str.bytes[pos..pos + len].to_vec(),
        size: len,
    }
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
    if chars.is_empty() {
        return Some(0);
    }
    let chars_bytes = chars.as_bytes();
    let clen = chars_bytes.len();
    if clen > str.size {
        return None;
    }
    for pos in 0..=str.size - clen {
        if &str.bytes[pos..pos + clen] == chars_bytes {
            return Some(pos);
        }
    }
    None
}
pub fn string_strip(str: &StringT) -> StringT {
    let start = str.bytes.iter().position(|b| !string_t_is_space_char(*b));
    let end = str.bytes.iter().rposition(|b| !string_t_is_space_char(*b));
    match (start, end) {
        (Some(s), Some(e)) if s <= e => string_substr(str, s, e - s + 1),
        _ => string_copy(str),
    }
}
pub fn string_split(str: &StringT, arr_size: &mut usize) -> StringTArray {
    if str.size == 0 {
        *arr_size = 1;
        return vec![string_copy(str)];
    }

    let mut result = Vec::new();
    let mut indexes: Vec<(usize, usize)> = Vec::new();

    let mut start_pos = 0usize;
    let mut pos = 0usize;
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

    for (s, e) in &indexes {
        result.push(string_substr(str, *s, e - s));
    }
    *arr_size = result.len();
    result
}
pub fn string_split_by(str: &StringT, arr_size: &mut usize, split_chars: &str) -> StringTArray {
    let split_bytes = split_chars.as_bytes();
    let split_len = split_bytes.len();

    if str.size <= split_len {
        *arr_size = 1;
        return vec![string_copy(str)];
    }

    let mut indexes: Vec<(usize, usize)> = Vec::new();
    let mut start_pos = 0usize;
    let mut pos = 0usize;

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

    let mut result = Vec::new();
    for (s, e) in &indexes {
        result.push(string_substr(str, *s, e - s));
    }
    *arr_size = result.len();
    result
}
pub fn string_join_arr(str_arr: &StringTArray, arr_size: usize, space_chars: &str) -> StringT {
    let sep = space_chars.as_bytes();
    let mut total_size = sep.len() * (arr_size - 1);
    for i in 0..arr_size {
        total_size += str_arr[i].size;
    }

    let mut bytes = Vec::with_capacity(total_size);
    for i in 0..arr_size {
        bytes.extend_from_slice(&str_arr[i].bytes);
        if i != arr_size - 1 {
            bytes.extend_from_slice(sep);
        }
    }

    StringT {
        bytes,
        size: total_size,
    }
}
pub fn string_t_is_space_char(byte: u8) -> BoolT {
    STRING_T_SPACE_CHARS_ARR.as_bytes().contains(&byte)
}
