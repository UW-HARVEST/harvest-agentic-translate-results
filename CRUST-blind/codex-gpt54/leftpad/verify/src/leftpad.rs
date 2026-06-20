// Function Declarations
pub fn leftpad(str: &str, padding: &str, min_len: usize, dest: &mut [u8]) -> usize {
    let str_bytes = str.as_bytes();
    let padding_bytes = if padding.is_empty() {
        b" ".as_slice()
    } else {
        padding.as_bytes()
    };

    let str_len = str_bytes.len();
    let npad = min_len.saturating_sub(str_len);

    if dest.is_empty() {
        return str_len + npad;
    }

    let max_write = dest.len() - 1;
    let mut written = 0usize;
    let mut pad_idx = 0usize;

    while written < npad && written < max_write {
        let byte = padding_bytes[pad_idx];
        dest[written] = byte;
        written += 1;

        pad_idx += 1;
        if pad_idx == padding_bytes.len() {
            pad_idx = 0;
        }
    }

    let mut str_idx = 0usize;
    while str_idx < str_len && written < max_write {
        dest[written] = str_bytes[str_idx];
        written += 1;
        str_idx += 1;
    }

    dest[written] = 0;
    written
}
