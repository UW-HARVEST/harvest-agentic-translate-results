use std::fs::File;
use std::io::Write;
#[derive(Debug, Clone)]
pub struct CsvField {
pub data: String,
pub len: usize,
}
impl CsvField {
pub fn new() -> Self {
    Self {
        data: String::new(),
        len: 10,
    }
}
pub fn reset(&mut self) {
    self.data.clear();
}
pub fn set(&mut self, buf: &str, buf_start_idx: usize, len: usize) {
    if len + 1 > self.len {
        self.len = len + 11;
    }
    self.data.clear();
    self.data.push_str(&buf[buf_start_idx..buf_start_idx + len]);
}
pub fn append(&mut self, buf: &str, buf_start_idx: usize, buflen: usize) {
    let origflen = self.data.len();
    if origflen + buflen + 1 > self.len {
        self.len = self.len + buflen + 10;
    }
    self.data.push_str(&buf[buf_start_idx..buf_start_idx + buflen]);
}
pub fn print_to_file(&self, fp: &mut File) {
let _ = writeln!(fp, "[{}:{}]", self.data, self.len);
}
}
