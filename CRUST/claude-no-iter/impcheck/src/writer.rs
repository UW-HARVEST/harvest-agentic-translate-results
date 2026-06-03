use std::fs::File;
use std::io::Write;
use crate::trusted_utils::{
    trusted_utils_write_bool, trusted_utils_write_char, trusted_utils_write_int,
    trusted_utils_write_ints, trusted_utils_write_sig, trusted_utils_write_ul,
    trusted_utils_write_uls,
};

pub struct Writer {
    file: File,
}
impl Writer {
    pub fn write_char(&mut self, c_int: i32) {
        let c = (c_int as u8) as char;
        trusted_utils_write_char(c, &mut self.file);
    }
    pub fn write_int(&mut self, i: i32) {
        trusted_utils_write_int(i, &mut self.file);
    }
    pub fn write_ints(&mut self, data: &[i32], nb_ints: u64) {
        trusted_utils_write_ints(data, nb_ints, &mut self.file);
    }
    pub fn write_sig(&mut self, sig: &[u8]) {
        trusted_utils_write_sig(sig, &mut self.file);
    }
    pub fn writer_init(output_path: &str) -> Self {
        let file = File::create(output_path).unwrap_or_else(|_| {
            crate::trusted_utils::trusted_utils_exit_eof();
            unreachable!()
        });
        Writer { file }
    }
    pub fn write_bool(&mut self, b: bool) {
        trusted_utils_write_bool(b, &mut self.file);
    }
    pub fn write_uls(&mut self, data: &[u64], nb_uls: u64) {
        trusted_utils_write_uls(data, nb_uls, &mut self.file);
    }
    pub fn write_ul(&mut self, ul: u64) {
        trusted_utils_write_ul(ul, &mut self.file);
    }
}

#[allow(dead_code)]
fn _ensure_write_used(w: &mut Writer) {
    let _ = w.file.flush();
}
