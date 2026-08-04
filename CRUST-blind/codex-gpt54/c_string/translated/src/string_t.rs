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
    StringT {
        bytes: bytes.as_bytes().to_vec(),
        size: bytes.len(),
    }
}
pub fn string_free(_str: StringT) {
    // Ownership drop is sufficient in Rust.
}
pub fn string_len(str: &StringT) -> usize {
    str.size
}
pub fn string_bytes(str: &StringT) -> &str {
    bytes_as_str(str)
}
pub fn string_eq(left: &StringT, right: &StringT) -> BoolT {
    left.size == right.size && string_slice(left) == string_slice(right)
}
pub fn string_copy(str: &StringT) -> StringT {
    StringT {
        bytes: string_slice(str).to_vec(),
        size: str.size,
    }
}
pub fn string_concat(first: &StringT, second: &StringT) -> StringT {
    let first_slice = string_slice(first);
    let second_slice = string_slice(second);
    let mut bytes = Vec::with_capacity(first.size + second.size);
    bytes.extend_from_slice(first_slice);
    bytes.extend_from_slice(second_slice);
    StringT {
        bytes,
        size: first.size + second.size,
    }
}
pub fn string_substr(str: &StringT, pos: usize, len: usize) -> StringT {
    let bytes = string_slice(str);
    if pos >= bytes.len() {
        return new_string(0);
    }

    let end = pos.saturating_add(len).min(bytes.len());
    StringT {
        bytes: bytes[pos..end].to_vec(),
        size: end - pos,
    }
}
pub fn string_startswith(str: &StringT, prefix: &str) -> BoolT {
    let bytes = string_slice(str);
    let prefix_bytes = prefix.as_bytes();
    bytes.len() >= prefix_bytes.len() && bytes[..prefix_bytes.len()] == *prefix_bytes
}
pub fn string_endswith(str: &StringT, suffix: &str) -> BoolT {
    let bytes = string_slice(str);
    let suffix_bytes = suffix.as_bytes();
    bytes.len() >= suffix_bytes.len()
        && bytes[bytes.len() - suffix_bytes.len()..] == *suffix_bytes
}
pub fn string_find(str: &StringT, chars: &str) -> Option<usize> {
    if chars.is_empty() {
        return Some(0);
    }

    let bytes = string_slice(str);
    let needle = chars.as_bytes();
    if needle.len() > bytes.len() {
        return None;
    }

    (0..=bytes.len() - needle.len()).find(|&pos| bytes[pos..pos + needle.len()] == *needle)
}
pub fn string_strip(str: &StringT) -> StringT {
    let bytes = string_slice(str);
    let mut start_pos = 0;
    while start_pos < bytes.len() && string_t_is_space_char(bytes[start_pos]) {
        start_pos += 1;
    }

    let mut end_pos = bytes.len() as isize - 1;
    while end_pos >= 0 && string_t_is_space_char(bytes[end_pos as usize]) {
        end_pos -= 1;
    }

    if start_pos as isize >= end_pos {
        return string_copy(str);
    }

    string_substr(str, start_pos, (end_pos as usize) - start_pos + 1)
}
pub fn string_split(str: &StringT, arr_size: &mut usize) -> StringTArray {
    let bytes = string_slice(str);
    if bytes.is_empty() {
        *arr_size = 1;
        return vec![string_copy(str)];
    }

    let mut ranges = Vec::new();
    let mut start_pos = 0;
    let mut pos = 0;
    while pos < bytes.len() {
        if string_t_is_space_char(bytes[pos]) {
            ranges.push((start_pos, pos));
            while pos < bytes.len() && string_t_is_space_char(bytes[pos]) {
                pos += 1;
            }
            start_pos = pos;
        } else {
            pos += 1;
        }
    }

    if pos != start_pos {
        ranges.push((start_pos, pos));
    }

    *arr_size = ranges.len();
    ranges
        .into_iter()
        .map(|(start, end)| string_substr(str, start, end - start))
        .collect()
}
pub fn string_split_by(str: &StringT, arr_size: &mut usize, split_chars: &str) -> StringTArray {
    let bytes = string_slice(str);
    let split_bytes = split_chars.as_bytes();
    if split_bytes.is_empty() || bytes.len() <= split_bytes.len() {
        *arr_size = 1;
        return vec![string_copy(str)];
    }

    let mut ranges = Vec::new();
    let mut start_pos = 0;
    let mut pos = 0;
    while pos < bytes.len() - split_bytes.len() {
        if bytes[pos..pos + split_bytes.len()] == *split_bytes {
            ranges.push((start_pos, pos));
            start_pos = pos + split_bytes.len();
            pos += split_bytes.len();
        } else {
            pos += 1;
        }
    }

    if pos != start_pos {
        ranges.push((start_pos, pos + 1));
    }

    *arr_size = ranges.len();
    ranges
        .into_iter()
        .map(|(start, end)| string_substr(str, start, end - start))
        .collect()
}
pub fn string_join_arr(str_arr: &StringTArray, arr_size: usize, space_chars: &str) -> StringT {
    let arr_size = arr_size.min(str_arr.len());
    if arr_size == 0 {
        return new_string(0);
    }

    let sep = space_chars.as_bytes();
    let total_size = str_arr
        .iter()
        .take(arr_size)
        .map(|s| s.size)
        .sum::<usize>()
        + sep.len() * (arr_size - 1);

    let mut bytes = Vec::with_capacity(total_size);
    for (idx, item) in str_arr.iter().take(arr_size).enumerate() {
        bytes.extend_from_slice(string_slice(item));
        if idx + 1 != arr_size {
            bytes.extend_from_slice(sep);
        }
    }

    StringT {
        bytes,
        size: total_size,
    }
}
pub fn string_t_is_space_char(byte: u8) -> BoolT {
    STRING_T_SPACE_CHARS_ARR
        .as_bytes()
        .contains(&byte)
}

fn string_slice(str: &StringT) -> &[u8] {
    let end = str.size.min(str.bytes.len());
    &str.bytes[..end]
}

fn bytes_as_str(str: &StringT) -> &str {
    std::str::from_utf8(string_slice(str)).unwrap_or_default()
}
