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
    let mut s = new_string(b.len());
    for (i, &c) in b.iter().enumerate() {
        s.bytes[i] = c;
    }
    s
}
pub fn string_free(_str: StringT) {
    // In Rust, memory is freed automatically when the value is dropped.
    // The parameter is consumed by value, so it will be dropped at end of scope.
}
pub fn string_len(str: &StringT) -> usize {
    str.size
}
pub fn string_bytes(str: &StringT) -> &str {
    // Mirror the C `string_bytes` which copies bytes into a fresh buffer of `size`
    // bytes (without writing any null terminator). We return a string slice that
    // views the underlying bytes as UTF-8.
    std::str::from_utf8(&str.bytes[..str.size]).unwrap_or("")
}
pub fn string_eq(left: &StringT, right: &StringT) -> BoolT {
    if left.size != right.size {
        return false;
    }
    for i in 0..left.size {
        if left.bytes[i] != right.bytes[i] {
            return false;
        }
    }
    true
}
pub fn string_copy(str: &StringT) -> StringT {
    let mut s = new_string(str.size);
    for i in 0..str.size {
        s.bytes[i] = str.bytes[i];
    }
    s
}
pub fn string_concat(first: &StringT, second: &StringT) -> StringT {
    let total = first.size + second.size;
    let mut s = new_string(total);
    for i in 0..first.size {
        s.bytes[i] = first.bytes[i];
    }
    for i in 0..second.size {
        s.bytes[first.size + i] = second.bytes[i];
    }
    s
}
pub fn string_substr(str: &StringT, pos: usize, len: usize) -> StringT {
    let mut sub = new_string(len);
    for i in 0..len {
        sub.bytes[i] = str.bytes[pos + i];
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
    if str.size < chars_str.size {
        return None;
    }
    for pos in 0..str.size {
        if pos + chars_str.size > str.size {
            break;
        }
        let sub = string_substr(str, pos, chars_str.size);
        if string_eq(&sub, &chars_str) {
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
    let mut indexes: Vec<usize> = vec![0; STRING_T_INDEXES_BUFFER_SIZE];

    if str.size == 0 {
        let str_arr = vec![string_copy(str)];
        *arr_size = 1;
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
        let sub_start = indexes[idx * 2];
        let sub_end = indexes[idx * 2 + 1];
        str_arr.push(string_substr(str, sub_start, sub_end - sub_start));
    }
    *arr_size = str_count;
    str_arr
}
pub fn string_split_by(str: &StringT, arr_size: &mut usize, split_chars: &str) -> StringTArray {
    let mut str_count: usize = 0;
    let mut indexes: Vec<usize> = vec![0; STRING_T_INDEXES_BUFFER_SIZE];

    let split_str = new_string_from_bytes(split_chars);
    if str.size <= split_str.size {
        let str_arr = vec![string_copy(str)];
        *arr_size = 1;
        return str_arr;
    }

    let mut start_pos: usize = 0;
    let mut pos: usize = 0;
    // Mirror C: for (; pos < str->size - split_str->size;)
    while pos < str.size - split_str.size {
        let sub = string_substr(str, pos, split_str.size);
        if string_eq(&sub, &split_str) {
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
        let sub_start = indexes[idx * 2];
        let sub_end = indexes[idx * 2 + 1];
        str_arr.push(string_substr(str, sub_start, sub_end - sub_start));
    }
    *arr_size = str_count;
    str_arr
}
pub fn string_join_arr(str_arr: &StringTArray, arr_size: usize, space_chars: &str) -> StringT {
    let space_bytes = space_chars.as_bytes();
    // Mirror C: separator length * (arr_size - 1), then sum of element sizes.
    let mut str_size: usize = if arr_size == 0 {
        0
    } else {
        space_bytes.len() * (arr_size - 1)
    };
    for idx in 0..arr_size {
        str_size += str_arr[idx].size;
    }

    let mut join_str = new_string(str_size);
    let mut offset: usize = 0;
    for idx in 0..arr_size {
        let elem = &str_arr[idx];
        for i in 0..elem.size {
            join_str.bytes[offset + i] = elem.bytes[i];
        }
        offset += elem.size;
        if idx != arr_size - 1 {
            for i in 0..space_bytes.len() {
                join_str.bytes[offset + i] = space_bytes[i];
            }
            offset += space_bytes.len();
        }
    }

    join_str
}
pub fn string_t_is_space_char(byte: u8) -> BoolT {
    let space_chars = STRING_T_SPACE_CHARS_ARR.as_bytes();
    for &c in space_chars.iter() {
        if c == byte {
            return true;
        }
    }
    false
}
