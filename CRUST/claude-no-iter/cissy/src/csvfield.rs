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
        len: 0,
    }
}
pub fn reset(&mut self) {
    self.data.clear();
}
pub fn set(&mut self, buf: &str, buf_start_idx: usize, len: usize) {
    self.data.clear();
    let bytes = buf.as_bytes();
    let end = std::cmp::min(buf_start_idx + len, bytes.len());
    if buf_start_idx < bytes.len() {
        let slice = &bytes[buf_start_idx..end];
        // Use String::from_utf8_lossy to be safe with arbitrary bytes
        self.data.push_str(&String::from_utf8_lossy(slice));
    }
    self.len = self.data.len() + 1;
}
pub fn append(&mut self, buf: &str, buf_start_idx: usize, buflen: usize) {
    let bytes = buf.as_bytes();
    let end = std::cmp::min(buf_start_idx + buflen, bytes.len());
    if buf_start_idx < bytes.len() {
        let slice = &bytes[buf_start_idx..end];
        self.data.push_str(&String::from_utf8_lossy(slice));
    }
    self.len = self.data.len() + 1;
}
pub fn print_to_file(&self, fp: &mut File) {
    let _ = writeln!(fp, "[{}:{}]", self.data, self.len);
}
}
