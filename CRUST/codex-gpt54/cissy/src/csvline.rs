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
    Self {
        field: Vec::new(),
        fieldsize: 0,
        current_idx: 0,
        eol_str: "\n".to_string(),
    }
}
pub fn get_field_count(&self) -> usize {
    self.current_idx
}
pub fn get_field(&self, idx: usize) -> Option<&str> {
    if idx >= self.current_idx {
        Some("")
    } else {
        Some(self.field[idx].data.as_str())
    }
}
pub fn reset(&mut self) {
    for field in &mut self.field {
        field.reset();
    }
    self.current_idx = 0;
    self.eol_str.clear();
    self.eol_str.push('\n');
}
pub fn add_field(&mut self, txtfield: &str, fieldstartidx: usize, fieldlen: usize) {
    if self.fieldsize <= self.current_idx {
        self.field.extend((0..10).map(|_| CsvField::new()));
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
    for field in &self.field {
        let _ = write!(fp, "[{}:{}]", field.data, field.len);
    }
    let _ = writeln!(fp, "]fs({}):i({})]", self.fieldsize, self.current_idx);
}
}
