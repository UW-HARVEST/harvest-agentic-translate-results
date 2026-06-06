// Function Declarations
pub fn leftpad(str: &str, padding: &str, min_len: usize, dest: &mut [u8]) -> usize {
    let str_bytes = str.as_bytes();
    let str_len = str_bytes.len();

    // Default padding to a single space if the provided padding is empty.
    let pad_bytes: &[u8] = if padding.is_empty() {
        b" "
    } else {
        padding.as_bytes()
    };

    // How many padding characters are needed before the string.
    let npad = if str_len < min_len {
        min_len - str_len
    } else {
        0
    };

    let dest_sz = dest.len();

    // If no destination buffer (or zero-sized), just return the would-be length.
    if dest_sz == 0 {
        return str_len + npad;
    }

    let mut dest_len: usize = 0;
    let mut i: usize = 0;

    // Write padding, repeating it as needed. Reserve one byte for the NUL terminator.
    while dest_len < npad && dest_len < dest_sz - 1 {
        dest[dest_len] = pad_bytes[i % pad_bytes.len()];
        dest_len += 1;
        i += 1;
    }

    // Append the string itself, leaving room for the NUL terminator.
    let mut j: usize = 0;
    while j < str_len && dest_len < dest_sz - 1 {
        dest[dest_len] = str_bytes[j];
        dest_len += 1;
        j += 1;
    }

    // Write NUL terminator (always within bounds since dest_sz > 0 and dest_len <= dest_sz - 1).
    dest[dest_len] = 0;

    dest_len
}
