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
    self.data = buf[buf_start_idx..buf_start_idx + len].to_string();
    if len + 1 > self.len {
        self.len = len + 1 + 10;
    }
}
pub fn append(&mut self, buf: &str, buf_start_idx: usize, buflen: usize) {
    self.data.push_str(&buf[buf_start_idx..buf_start_idx + buflen]);
    let total = self.data.len() + 1;
    if total > self.len {
        self.len = self.len + buflen + 10;
    }
}
pub fn print_to_file(&self, fp: &mut File) {
    let _ = write!(fp, "[{}:{}]\n", self.data, self.data.len());
}
}
