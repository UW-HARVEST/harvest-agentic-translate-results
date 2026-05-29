use std::fs::File;
use std::io::Write;
#[derive(Debug, Clone)]
pub struct CsvField {
pub data: String,
pub len: usize,
}
impl CsvField {
pub fn new() -> Self {
    CsvField {
        data: String::new(),
        len: 10, // STR_MEM_PAD
    }
}
pub fn reset(&mut self) {
    self.data.clear();
}
pub fn set(&mut self, buf: &str, buf_start_idx: usize, len: usize) {
    if len + 1 > self.len {
        self.len = len + 1 + 10;
    }
    self.data.clear();
    let bytes = buf.as_bytes();
    let end = (buf_start_idx + len).min(bytes.len());
    let slice = &bytes[buf_start_idx..end];
    // Use lossy conversion to be safe with arbitrary bytes; assume valid UTF-8 for tests.
    self.data
        .push_str(std::str::from_utf8(slice).unwrap_or(""));
}
pub fn append(&mut self, buf: &str, buf_start_idx: usize, buflen: usize) {
    let origflen = self.data.len();
    if origflen + buflen + 1 > self.len {
        self.len = self.len + buflen + 10;
    }
    let bytes = buf.as_bytes();
    let end = (buf_start_idx + buflen).min(bytes.len());
    let slice = &bytes[buf_start_idx..end];
    self.data
        .push_str(std::str::from_utf8(slice).unwrap_or(""));
}
pub fn print_to_file(&self, fp: &mut File) {
    let _ = write!(fp, "[{}:{}]\n", self.data, self.len);
}
}
