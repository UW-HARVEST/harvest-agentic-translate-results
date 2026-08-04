// Function Declarations
pub fn leftpad(str: &str, padding: &str, min_len: usize, dest: &mut [u8]) -> usize {
    let str_bytes = str.as_bytes();
    let str_len = str_bytes.len();

    // Default padding to a single space if empty
    let padding_bytes = if padding.is_empty() {
        b" " as &[u8]
    } else {
        padding.as_bytes()
    };

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

    // Write padding, repeating the padding string as needed.
    // Match the C behavior of reserving one byte (for the NUL terminator)
    // by capping at dest_sz - 1.
    while dest_len < npad && dest_len < dest_sz - 1 {
        if i >= padding_bytes.len() {
            // Wrap around: emit padding[0] and continue from index 1
            dest[dest_len] = padding_bytes[0];
            dest_len += 1;
            i = 1;
        } else {
            dest[dest_len] = padding_bytes[i];
            dest_len += 1;
            i += 1;
        }
    }

    // Write the original string
    let mut j: usize = 0;
    while j < str_len && dest_len < dest_sz - 1 {
        dest[dest_len] = str_bytes[j];
        dest_len += 1;
        j += 1;
    }

    dest_len
}
