// Function Declarations
pub fn leftpad(str: &str, padding: &str, min_len: usize, dest: &mut [u8]) -> usize {
    let str_bytes = str.as_bytes();
    let padding_bytes = if padding.is_empty() {
        b" ".as_slice()
    } else {
        padding.as_bytes()
    };

    let npad = min_len.saturating_sub(str_bytes.len());
    if dest.is_empty() {
        return str_bytes.len() + npad;
    }

    let mut dest_len = 0usize;
    let max_visible = dest.len() - 1;
    let mut pad_idx = 0usize;

    while dest_len < npad && dest_len < max_visible {
        let byte = padding_bytes[pad_idx];
        dest[dest_len] = byte;
        dest_len += 1;

        pad_idx += 1;
        if pad_idx == padding_bytes.len() {
            pad_idx = 0;
        }
    }

    for &byte in str_bytes {
        if dest_len >= max_visible {
            break;
        }
        dest[dest_len] = byte;
        dest_len += 1;
    }

    dest[dest_len] = 0;
    dest_len
}
