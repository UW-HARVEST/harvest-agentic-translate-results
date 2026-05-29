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
        len: 10, // mirror STR_MEM_PAD
    }
}
pub fn reset(&mut self) {
    self.data.clear();
}
pub fn set(&mut self, buf: &str, buf_start_idx: usize, len: usize) {
    let bytes = buf.as_bytes();
    let end = (buf_start_idx + len).min(bytes.len());
    let slice = &bytes[buf_start_idx.min(bytes.len())..end];
    self.data.clear();
    self.data.push_str(&String::from_utf8_lossy(slice));
    if len + 1 > self.len {
        self.len = len + 1 + 10;
    }
}
pub fn append(&mut self, buf: &str, buf_start_idx: usize, buflen: usize) {
    let bytes = buf.as_bytes();
    let end = (buf_start_idx + buflen).min(bytes.len());
    let slice = &bytes[buf_start_idx.min(bytes.len())..end];
    let origflen = self.data.len();
    if origflen + buflen + 1 > self.len {
        self.len = self.len + buflen + 10;
    }
    self.data.push_str(&String::from_utf8_lossy(slice));
}
pub fn print_to_file(&self, fp: &mut File) {
    let _ = writeln!(fp, "[{}:{}]", self.data, self.len);
}
}
