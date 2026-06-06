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
        len: 10,
    }
}
pub fn reset(&mut self) {
    self.data.clear();
}
pub fn set(&mut self, buf: &str, buf_start_idx: usize, len: usize) {
    let bytes = buf.as_bytes();
    let end = buf_start_idx + len;
    let end = end.min(bytes.len());
    let start = buf_start_idx.min(bytes.len());
    let slice = &bytes[start..end];
    self.data = String::from_utf8_lossy(slice).into_owned();
    if (len + 1) > self.len {
        self.len = len + 1 + 10;
    }
}
pub fn append(&mut self, buf: &str, buf_start_idx: usize, buflen: usize) {
    let origflen = self.data.len();
    let bytes = buf.as_bytes();
    let end = buf_start_idx + buflen;
    let end = end.min(bytes.len());
    let start = buf_start_idx.min(bytes.len());
    let slice = &bytes[start..end];
    let appended = String::from_utf8_lossy(slice);
    if (origflen + buflen + 1) > self.len {
        self.len = self.len + buflen + 10;
    }
    self.data.push_str(&appended);
}
pub fn print_to_file(&self, fp: &mut File) {
    write!(fp, "[{}:{}]\n", self.data, self.len).unwrap();
}
}
