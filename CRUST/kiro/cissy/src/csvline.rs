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
    CsvLine { field: Vec::new(), fieldsize: 0, current_idx: 0, eol_str: "\n".to_string() }
}
pub fn get_field_count(&self) -> usize {
    self.current_idx
}
pub fn get_field(&self, idx: usize) -> Option<&str> {
    if idx >= self.current_idx { return Some(""); }
    Some(&self.field[idx].data)
}
pub fn reset(&mut self) {
    for f in self.field.iter_mut() { f.reset(); }
    self.current_idx = 0;
    self.eol_str = "\n".to_string();
}
pub fn add_field(&mut self, txtfield: &str, fieldstartidx: usize, fieldlen: usize) {
    if self.fieldsize <= self.current_idx {
        for _ in 0..10 { self.field.push(CsvField::new()); }
        self.fieldsize += 10;
    }
    self.field[self.current_idx].set(txtfield, fieldstartidx, fieldlen);
    self.current_idx += 1;
}
pub fn append_field(&mut self, txtfield: &str, fieldstartidx: usize, fieldlen: usize) {
    self.field[self.current_idx - 1].append(txtfield, fieldstartidx, fieldlen);
}
pub fn print_to_file(&self, fp: &mut File) {
    let mut s = String::from("[[");
    for f in &self.field {
        s.push_str(&format!("[{}:{}]", f.data, f.len));
    }
    s.push_str(&format!("]fs({}):i({})]\n", self.fieldsize, self.current_idx));
    write!(fp, "{}", s).unwrap();
}
}
