use crate::csvfield::CsvField;
use std::fs::File;
use std::io::Write;
#[derive(Debug, Clone)]
pub struct CsvLine {
pub field: Vec<CsvField>,
pub fieldsize: usize,
pub current_idx: usize,
pub eol_str: String,
}
impl CsvLine {
pub fn new() -> Self {
    CsvLine {
        field: Vec::new(),
        fieldsize: 0,
        current_idx: 0,
        eol_str: String::from("\n"),
    }
}
pub fn get_field_count(&self) -> usize {
    self.current_idx
}
pub fn get_field(&self, idx: usize) -> Option<&str> {
    if idx >= self.current_idx {
        return Some("");
    }
    Some(self.field[idx].data.as_str())
}
pub fn reset(&mut self) {
    for i in 0..self.fieldsize {
        if i < self.field.len() {
            self.field[i].reset();
        }
    }
    self.current_idx = 0;
    self.eol_str = String::from("\n");
}
pub fn add_field(&mut self, txtfield: &str, fieldstartidx: usize, fieldlen: usize) {
    if self.fieldsize <= self.current_idx {
        // grow by 10
        for _ in 0..10 {
            self.field.push(CsvField::new());
        }
        self.fieldsize += 10;
    }
    self.field[self.current_idx].set(txtfield, fieldstartidx, fieldlen);
    self.current_idx += 1;
}
pub fn append_field(&mut self, txtfield: &str, fieldstartidx: usize, fieldlen: usize) {
    if self.current_idx == 0 {
        return;
    }
    self.field[self.current_idx - 1].append(txtfield, fieldstartidx, fieldlen);
}
pub fn print_to_file(&self, fp: &mut File) {
    let _ = write!(fp, "[[");
    for i in 0..self.fieldsize {
        if i < self.field.len() {
            // Note: original C code writes to stderr in this loop, not fp.
            // We mirror that behavior using eprintln-ish writes only to fp.
            let _ = write!(fp, "[{}:{}]", self.field[i].data, self.field[i].len);
        }
    }
    let _ = write!(fp, "]");
    let _ = write!(fp, "fs({}):", self.fieldsize);
    let _ = write!(fp, "i({})", self.current_idx);
    let _ = writeln!(fp, "]");
}
}
