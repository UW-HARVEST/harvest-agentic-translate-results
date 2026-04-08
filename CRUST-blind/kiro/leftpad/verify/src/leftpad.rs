// Function Declarations
pub fn leftpad(str: &str, padding: &str, min_len: usize, dest: &mut [u8]) -> usize {
    let str_bytes = str.as_bytes();
    let str_len = str_bytes.len();
    let padding = if padding.is_empty() { " " } else { padding };
    let pad_bytes = padding.as_bytes();
    let npad = if str_len < min_len { min_len - str_len } else { 0 };

    if dest.is_empty() {
        return str_len + npad;
    }

    let max = dest.len() - 1;
    let mut dest_len = 0;
    let mut pi = 0;

    while dest_len < npad && dest_len < max {
        dest[dest_len] = pad_bytes[pi];
        pi = (pi + 1) % pad_bytes.len();
        dest_len += 1;
    }

    for i in 0..str_len {
        if dest_len >= max {
            break;
        }
        dest[dest_len] = str_bytes[i];
        dest_len += 1;
    }

    dest[dest_len] = 0;
    dest_len
}
