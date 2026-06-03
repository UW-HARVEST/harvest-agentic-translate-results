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
    self.data.clear();
}
pub fn set(&mut self, buf: &str, buf_start_idx: usize, len: usize) {
    if len + 1 > self.len {
        self.len = len + 1 + STR_MEM_PAD;
    }
    self.data.clear();
    let end = buf_start_idx + len;
    let slice = &buf[buf_start_idx..end];
    self.data.push_str(slice);
}
pub fn append(&mut self, buf: &str, buf_start_idx: usize, buflen: usize) {
    let origflen = self.data.len();
    if origflen + buflen + 1 > self.len {
        self.len = self.len + buflen + STR_MEM_PAD;
    }
    let end = buf_start_idx + buflen;
    let slice = &buf[buf_start_idx..end];
    self.data.push_str(slice);
}
pub fn print_to_file(&self, fp: &mut File) {
    let _ = write!(fp, "[{}:{}]\n", self.data, self.len);
}
}
