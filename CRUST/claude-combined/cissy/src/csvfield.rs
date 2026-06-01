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
            data: String::new(),
            len: STR_MEM_PAD,
        }
    }

    pub fn reset(&mut self) {
        self.data.clear();
    }

    pub fn set(&mut self, buf: &str, buf_start_idx: usize, len: usize) {
        let bytes = buf.as_bytes();
        let end = (buf_start_idx + len).min(bytes.len());
        let start = buf_start_idx.min(bytes.len());
        let slice = &bytes[start..end];
        self.data.clear();
        // Mimic C's strncpy behavior for byte ranges
        let s = String::from_utf8_lossy(slice).into_owned();
        self.data.push_str(&s);
        if len + 1 > self.len {
            self.len = len + 1 + STR_MEM_PAD;
        }
    }

    pub fn append(&mut self, buf: &str, buf_start_idx: usize, buflen: usize) {
        let origflen = self.data.len();
        let bytes = buf.as_bytes();
        let end = (buf_start_idx + buflen).min(bytes.len());
        let start = buf_start_idx.min(bytes.len());
        let slice = &bytes[start..end];
        let s = String::from_utf8_lossy(slice).into_owned();
        self.data.push_str(&s);
        if (origflen + buflen + 1) > self.len {
            self.len = self.len + buflen + STR_MEM_PAD;
        }
    }

    pub fn print_to_file(&self, fp: &mut File) {
        let _ = writeln!(fp, "[{}:{}]", self.data, self.len as i32);
    }
}
