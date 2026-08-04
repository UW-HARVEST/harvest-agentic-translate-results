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
        let end = buf_start_idx.saturating_add(len).min(buf.len());
        self.data = String::from_utf8_lossy(&buf.as_bytes()[buf_start_idx.min(buf.len())..end])
            .into_owned();
    }
    pub fn append(&mut self, buf: &str, buf_start_idx: usize, buflen: usize) {
        let origflen = self.data.len();
        if (origflen + buflen + 1) > self.len {
            self.len += buflen + 10;
        }
        let start = buf_start_idx.min(buf.len());
        let end = start.saturating_add(buflen).min(buf.len());
        self.data
            .push_str(&String::from_utf8_lossy(&buf.as_bytes()[start..end]));
    }
    pub fn print_to_file(&self, fp: &mut File) {
        let _ = write!(fp, "[{}:{}]\n", self.data, self.len);
    }
}
