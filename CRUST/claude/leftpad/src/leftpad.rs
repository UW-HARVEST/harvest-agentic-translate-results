// Function Declarations
pub fn leftpad(str: &str, padding: &str, min_len: usize, dest: &mut [u8]) -> usize {
    let str_bytes = str.as_bytes();
    let str_len = str_bytes.len();

    // Default padding is a single space when padding is empty.
    let pad_bytes: &[u8] = if padding.is_empty() {
        b" "
    } else {
        padding.as_bytes()
    };

    let npad = if str_len < min_len {
        min_len - str_len
    } else {
        0
    };

    let dest_sz = dest.len();
    // If no destination buffer, just return the required length.
    if dest_sz == 0 {
        return str_len + npad;
    }

    let mut dest_len: usize = 0;

    // Write padding cyclically, leaving room for a trailing null terminator.
    let pad_limit = if npad < dest_sz - 1 { npad } else { dest_sz - 1 };
    while dest_len < pad_limit {
        dest[dest_len] = pad_bytes[dest_len % pad_bytes.len()];
        dest_len += 1;
    }

    // Copy the string contents, leaving room for a trailing null terminator.
    let mut i: usize = 0;
    while i < str_len && dest_len < dest_sz - 1 {
        dest[dest_len] = str_bytes[i];
        dest_len += 1;
        i += 1;
    }

    // Null terminator.
    dest[dest_len] = 0;

    dest_len
}
