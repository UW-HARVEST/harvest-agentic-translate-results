// Function Declarations
pub fn leftpad(str: &str, padding: &str, min_len: usize, dest: &mut [u8]) -> usize {
    let str_bytes = str.as_bytes();
    let str_len = str_bytes.len();

    // If padding is empty, default to a single space (matches C behavior of
    // treating NULL or empty padding as " ").
    let effective_padding = if padding.is_empty() { " " } else { padding };
    let padding_bytes = effective_padding.as_bytes();
    let padding_len = padding_bytes.len();

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
    let mut i: usize = 0;

    // Write padding characters, wrapping around the padding string as needed.
    while dest_len < npad && dest_len < dest_sz - 1 {
        // In Rust strings have no NUL terminator, so we simply wrap when we
        // reach the end of the padding string.
        if i >= padding_len {
            i = 0;
        }
        dest[dest_len] = padding_bytes[i];
        dest_len += 1;
        i += 1;
    }

    // Write the input string.
    let mut j: usize = 0;
    while j < str_len && dest_len < dest_sz - 1 {
        dest[dest_len] = str_bytes[j];
        dest_len += 1;
        j += 1;
    }

    // NUL-terminate, mirroring the C implementation.
    dest[dest_len] = 0;

    dest_len
}
