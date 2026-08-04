// Function Declarations
pub fn leftpad(str: &str, padding: &str, min_len: usize, dest: &mut [u8]) -> usize {
    let str_bytes = str.as_bytes();
    let str_len = str_bytes.len();

    // If padding is empty, default to a single space.
    let padding_bytes: &[u8] = if padding.is_empty() {
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
    if dest_sz == 0 {
        return str_len + npad;
    }

    let mut dest_len: usize = 0;
    let mut i: usize = 0;

    // Write the padding, repeating as necessary.
    while dest_len < npad && dest_len < dest_sz - 1 {
        let b = padding_bytes[i];
        i += 1;
        if b == 0 {
            // Reached end of padding (in C, the null terminator). Restart.
            dest[dest_len] = padding_bytes[0];
            dest_len += 1;
            i = 1;
        } else {
            dest[dest_len] = b;
            dest_len += 1;
            if i >= padding_bytes.len() {
                // Wrap around as if we hit the null terminator in C.
                i = 0;
            }
        }
    }

    // Copy the string itself.
    let mut j: usize = 0;
    while j < str_len && dest_len < dest_sz - 1 {
        dest[dest_len] = str_bytes[j];
        dest_len += 1;
        j += 1;
    }

    // Null-terminate, matching C semantics.
    dest[dest_len] = 0;

    dest_len
}
