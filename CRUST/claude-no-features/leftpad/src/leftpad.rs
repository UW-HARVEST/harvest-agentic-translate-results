// Function Declarations
pub fn leftpad(str: &str, padding: &str, min_len: usize, dest: &mut [u8]) -> usize {
    let str_bytes = str.as_bytes();
    let str_len = str_bytes.len();

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
    let mut i: usize = 0;

    // Write padding, cycling through `padding` as needed
    while dest_len < npad && dest_len < dest_sz - 1 {
        if i >= padding_bytes.len() {
            i = 0;
        }
        dest[dest_len] = padding_bytes[i];
        dest_len += 1;
        i += 1;
    }

    // Append the original string
    let mut j: usize = 0;
    while j < str_len && dest_len < dest_sz - 1 {
        dest[dest_len] = str_bytes[j];
        dest_len += 1;
        j += 1;
    }

    // Null-terminate (mirrors the C implementation)
    dest[dest_len] = 0;

    dest_len
}
