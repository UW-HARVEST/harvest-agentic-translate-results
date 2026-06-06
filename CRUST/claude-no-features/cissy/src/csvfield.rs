use std::fs::File;
use std::io::Write;
#[derive(Debug, Clone)]
pub struct CsvField {
pub data: String,
pub len: usize,
}
impl CsvField {
pub fn new() -> Self {
    const STR_MEM_PAD: usize = 10;
    CsvField {
        data: String::new(),
        len: STR_MEM_PAD,
    }
}
pub fn reset(&mut self) {
    self.data.clear();
}
pub fn set(&mut self, buf: &str, buf_start_idx: usize, len: usize) {
    const STR_MEM_PAD: usize = 10;
    let bytes = buf.as_bytes();
    let end = buf_start_idx + len;
    let slice = &bytes[buf_start_idx..end];
    if len + 1 > self.len {
        self.len = len + 1 + STR_MEM_PAD;
    }
    self.data = String::from_utf8_lossy(slice).into_owned();
}
pub fn append(&mut self, buf: &str, buf_start_idx: usize, buflen: usize) {
    const STR_MEM_PAD: usize = 10;
    let origflen = self.data.len();
    if origflen + buflen + 1 > self.len {
        self.len = self.len + buflen + STR_MEM_PAD;
    }
    let bytes = buf.as_bytes();
    let end = buf_start_idx + buflen;
    let slice = &bytes[buf_start_idx..end];
    let appended = String::from_utf8_lossy(slice);
    self.data.push_str(&appended);
}
pub fn print_to_file(&self, fp: &mut File) {
    write!(fp, "[{}:{}]\n", self.data, self.len as i32).unwrap();
}
}
