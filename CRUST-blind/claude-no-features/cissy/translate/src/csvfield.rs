use std::fs::File;
use std::io::Write;
#[derive(Debug, Clone)]
pub struct CsvField {
pub data: String,
pub len: usize,
}
impl CsvField {
pub fn new() -> Self {
    // Mirrors csvfield_create: data is empty, with a small allocation budget.
    CsvField {
        data: String::with_capacity(10),
        len: 10,
    }
}
pub fn reset(&mut self) {
    // Mirrors csvfield_reset: clear the string contents.
    self.data.clear();
}
pub fn set(&mut self, buf: &str, buf_start_idx: usize, len: usize) {
    // Replace contents with a slice of `buf`.
    let bytes = buf.as_bytes();
    let end = buf_start_idx.saturating_add(len).min(bytes.len());
    let start = buf_start_idx.min(bytes.len());
    let slice = &bytes[start..end];
    let s = String::from_utf8_lossy(slice).into_owned();
    self.data = s;
    if len + 1 > self.len {
        self.len = len + 1 + 10;
    }
}
pub fn append(&mut self, buf: &str, buf_start_idx: usize, buflen: usize) {
    let bytes = buf.as_bytes();
    let end = buf_start_idx.saturating_add(buflen).min(bytes.len());
    let start = buf_start_idx.min(bytes.len());
    let slice = &bytes[start..end];
    let appended = String::from_utf8_lossy(slice);
    let origflen = self.data.len();
    if origflen + buflen + 1 > self.len {
        self.len = self.len + buflen + 10;
    }
    self.data.push_str(&appended);
}
pub fn print_to_file(&self, fp: &mut File) {
    // Mirrors csvfield_printToFile: "[%s:%d]\n"
    let _ = writeln!(fp, "[{}:{}]", self.data, self.len);
}
}
