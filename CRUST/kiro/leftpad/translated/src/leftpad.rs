// Function Declarations
pub fn leftpad(str: &str, padding: &str, min_len: usize, dest: &mut [u8]) -> usize {
    let str_len = str.len();
    let padding = if padding.is_empty() { " " } else { padding };
    let npad = if str_len < min_len { min_len - str_len } else { 0 };
    let dest_sz = dest.len();
    if dest_sz == 0 {
        return str_len + npad;
    }
    let mut dest_len = 0;
    let pad_bytes = padding.as_bytes();
    let mut pi = 0;
    while dest_len < npad && dest_len < dest_sz - 1 {
        dest[dest_len] = pad_bytes[pi];
        pi += 1;
        if pi >= pad_bytes.len() {
            pi = 0;
        }
        dest_len += 1;
    }
    let str_bytes = str.as_bytes();
    for i in 0..str_len {
        if dest_len >= dest_sz - 1 {
            break;
        }
        dest[dest_len] = str_bytes[i];
        dest_len += 1;
    }
    dest[dest_len] = 0;
    dest_len
}
