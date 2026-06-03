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
    CsvField {
        data: String::with_capacity(STR_MEM_PAD),
        len: STR_MEM_PAD,
    }
}
pub fn reset(&mut self) {
    // In the C version, reset sets data to '\0' (effectively empties it).
    self.data.clear();
}
pub fn set(&mut self, buf: &str, buf_start_idx: usize, len: usize) {
    // buf_start_idx and len are byte offsets/lengths into buf
    if len + 1 > self.len {
        self.len = len + 1 + STR_MEM_PAD;
    }
    self.data.clear();
    let bytes = buf.as_bytes();
    let end = (buf_start_idx + len).min(bytes.len());
    let start = buf_start_idx.min(bytes.len());
    // safe: we treat as bytes but push_str expects a &str slice
    // Use the str slice if buf_start_idx and end are char boundaries
    if buf.is_char_boundary(start) && buf.is_char_boundary(end) {
        self.data.push_str(&buf[start..end]);
    } else {
        // fallback: copy bytes one by one as chars
        let slice = &bytes[start..end];
        for &b in slice {
            self.data.push(b as char);
        }
    }
}
pub fn append(&mut self, buf: &str, buf_start_idx: usize, buflen: usize) {
    let origflen = self.data.len();
    if origflen + buflen + 1 > self.len {
        self.len = self.len + buflen + STR_MEM_PAD;
    }
    let bytes = buf.as_bytes();
    let end = (buf_start_idx + buflen).min(bytes.len());
    let start = buf_start_idx.min(bytes.len());
    if buf.is_char_boundary(start) && buf.is_char_boundary(end) {
        self.data.push_str(&buf[start..end]);
    } else {
        let slice = &bytes[start..end];
        for &b in slice {
            self.data.push(b as char);
        }
    }
}
pub fn print_to_file(&self, fp: &mut File) {
    let _ = writeln!(fp, "[{}:{}]", self.data, self.len);
}
}
