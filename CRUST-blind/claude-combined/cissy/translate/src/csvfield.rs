use std::fs::File;
use std::io::Write;

const STR_MEM_PAD: usize = 10;

#[derive(Debug, Clone)]
pub struct CsvField {
pub data: String,
pub len: usize,
}
impl CsvField {
pub fn new() -> Self {
    // Mirrors csvfield_create(): allocates STR_MEM_PAD bytes, data is empty.
    CsvField {
        data: String::new(),
        len: STR_MEM_PAD,
    }
}
pub fn reset(&mut self) {
    // Mirrors csvfield_reset(pField->data = '\0';): in C, this sets the
    // first byte to nul, effectively making the data an empty string.
    // The allocation (len) is not freed; preserve len.
    self.data.clear();
}
pub fn set(&mut self, buf: &str, buf_start_idx: usize, len: usize) {
    // Match C csvfield_set semantics: ensure capacity for len+1, copy len bytes
    // from buf[buf_start_idx..buf_start_idx+len], nul-terminate. We model "nul-
    // terminated string" with a Rust String containing only the bytes.
    if len + 1 > self.len {
        self.len = len + 1 + STR_MEM_PAD;
    }
    let bytes = buf.as_bytes();
    let empty: &[u8] = &[];
    let to_copy = if buf_start_idx >= bytes.len() {
        empty
    } else {
        let avail = bytes.len() - buf_start_idx;
        let n = std::cmp::min(len, avail);
        &bytes[buf_start_idx..buf_start_idx + n]
    };
    // strncpy: copies up to len bytes; if source shorter than len, pads with
    // nul. Since we then nul-terminate at index buflen, the data we keep is
    // the prefix up through the first nul or the actual copied bytes.
    self.data.clear();
    // Find first nul in source (within len)
    let mut effective_end = to_copy.len();
    for (i, b) in to_copy.iter().enumerate() {
        if *b == 0 {
            effective_end = i;
            break;
        }
    }
    let final_bytes = &to_copy[..effective_end];
    if let Ok(s) = std::str::from_utf8(final_bytes) {
        self.data.push_str(s);
    } else {
        // Fall back: lossy conversion for non-UTF8 bytes
        self.data.push_str(&String::from_utf8_lossy(final_bytes));
    }
}
pub fn append(&mut self, buf: &str, buf_start_idx: usize, buflen: usize) {
    let origflen = self.data.len();
    if origflen + buflen + 1 > self.len {
        self.len = self.len + buflen + STR_MEM_PAD;
    }
    let bytes = buf.as_bytes();
    let empty: &[u8] = &[];
    let to_copy = if buf_start_idx >= bytes.len() {
        empty
    } else {
        let avail = bytes.len() - buf_start_idx;
        let n = std::cmp::min(buflen, avail);
        &bytes[buf_start_idx..buf_start_idx + n]
    };
    let mut effective_end = to_copy.len();
    for (i, b) in to_copy.iter().enumerate() {
        if *b == 0 {
            effective_end = i;
            break;
        }
    }
    let final_bytes = &to_copy[..effective_end];
    if let Ok(s) = std::str::from_utf8(final_bytes) {
        self.data.push_str(s);
    } else {
        self.data.push_str(&String::from_utf8_lossy(final_bytes));
    }
}
pub fn print_to_file(&self, fp: &mut File) {
    let _ = write!(fp, "[{}:{}]\n", self.data, self.len);
}
}
