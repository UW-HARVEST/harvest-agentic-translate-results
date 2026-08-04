use crate::trusted_utils::*;
use std::fs::File;
use std::io::Write;

pub struct Writer {
    file: File,
}

impl Writer {
    pub fn writer_init(output_path: &str) -> Self {
        let file = match File::create(output_path) {
            Ok(f) => f,
            Err(_) => {
                trusted_utils_exit_eof();
                unreachable!()
            }
        };
        Writer { file }
    }

    pub fn write_char(&mut self, c_int: i32) {
        trusted_utils_write_char(c_int as u8 as char, &mut self.file);
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
fn _flush(w: &mut Writer) {
    let _ = w.file.flush();
}
