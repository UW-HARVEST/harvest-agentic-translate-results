// Function Declarations
pub fn leftpad(str: &str, padding: &str, min_len: usize, dest: &mut [u8]) -> usize {
    let str_bytes = str.as_bytes();
    let str_len = str_bytes.len();

    // Default padding to a single space when empty (mirrors the C check
    // for NULL or empty padding).
    let padding = if padding.is_empty() { " " } else { padding };
    let padding_bytes = padding.as_bytes();

    let npad = if str_len < min_len {
        min_len - str_len
    } else {
        0
    };

    let dest_sz = dest.len();
    if dest_sz == 0 {
        return str_len + npad;
    }

    let mut dest_len: usize = 0;

    // Write the padding, repeating it as needed. In the original C the
    // padding is a NUL-terminated string and the loop wraps when it hits
    // the terminator; that is equivalent to indexing modulo the padding
    // length when there is no embedded NUL (which Rust strings never have).
    let pad_n = padding_bytes.len();
    let mut pi: usize = 0;
    while dest_len < npad && dest_len < dest_sz - 1 {
        dest[dest_len] = padding_bytes[pi % pad_n];
        pi += 1;
        dest_len += 1;
    }

    // Copy the source string into the destination, stopping if we run out
    // of room (leaving space for the trailing NUL).
    let mut i: usize = 0;
    while i < str_len && dest_len < dest_sz - 1 {
        dest[dest_len] = str_bytes[i];
        i += 1;
        dest_len += 1;
    }

    // NUL terminate. dest_len is always < dest_sz here.
    dest[dest_len] = 0;

    dest_len
}
