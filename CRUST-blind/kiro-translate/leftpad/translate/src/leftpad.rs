// Function Declarations
pub fn leftpad(str: &str, padding: &str, min_len: usize, dest: &mut [u8]) -> usize {
    let padding = if padding.is_empty() { " " } else { padding };
    let npad = if str.len() < min_len { min_len - str.len() } else { 0 };
    let dest_sz = dest.len();

    if dest_sz == 0 {
        return str.len() + npad;
    }

    let pad_bytes = padding.as_bytes();
    let mut dest_len = 0;

    for i in 0..npad {
        if dest_len >= dest_sz - 1 { break; }
        dest[dest_len] = pad_bytes[i % pad_bytes.len()];
        dest_len += 1;
    }

    for &b in str.as_bytes() {
        if dest_len >= dest_sz - 1 { break; }
        dest[dest_len] = b;
        dest_len += 1;
    }

    dest[dest_len] = 0;
    dest_len
}