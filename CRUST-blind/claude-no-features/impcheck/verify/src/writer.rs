use std::fs::File;
use std::io::Write;

use crate::trusted_utils;

pub struct Writer {
    file: File,
}

impl Writer {
    pub fn write_char(&mut self, c_int: i32) {
        let byte = [c_int as u8];
        if self.file.write_all(&byte).is_err() {
            trusted_utils::trusted_utils_exit_eof();
        }
    }

    pub fn write_int(&mut self, i: i32) {
        if self.file.write_all(&i.to_ne_bytes()).is_err() {
            trusted_utils::trusted_utils_exit_eof();
        }
    }

    pub fn write_ints(&mut self, data: &[i32], nb_ints: u64) {
        let n = nb_ints as usize;
        let mut buf = Vec::with_capacity(n * 4);
        for &val in data.iter().take(n) {
            buf.extend_from_slice(&val.to_ne_bytes());
        }
        if self.file.write_all(&buf).is_err() {
            trusted_utils::trusted_utils_exit_eof();
        }
    }

    pub fn write_sig(&mut self, sig: &[u8]) {
        let n = trusted_utils::SIG_SIZE_BYTES.min(sig.len());
        if self.file.write_all(&sig[..n]).is_err() {
            trusted_utils::trusted_utils_exit_eof();
        }
    }

    pub fn writer_init(output_path: &str) -> Self {
        match File::create(output_path) {
            Ok(f) => Writer { file: f },
            Err(_) => {
                trusted_utils::trusted_utils_exit_eof();
                unreachable!()
            }
        }
    }

    pub fn write_bool(&mut self, b: bool) {
        let byte = [if b { 1u8 } else { 0u8 }];
        if self.file.write_all(&byte).is_err() {
            trusted_utils::trusted_utils_exit_eof();
        }
    }

    pub fn write_uls(&mut self, data: &[u64], nb_uls: u64) {
        let n = nb_uls as usize;
        let mut buf = Vec::with_capacity(n * 8);
        for &val in data.iter().take(n) {
            buf.extend_from_slice(&val.to_ne_bytes());
        }
        if self.file.write_all(&buf).is_err() {
            trusted_utils::trusted_utils_exit_eof();
        }
    }

    pub fn write_ul(&mut self, ul: u64) {
        if self.file.write_all(&ul.to_ne_bytes()).is_err() {
            trusted_utils::trusted_utils_exit_eof();
        }
    }
}
