use std::fs::File;
use std::io::Write;

pub struct Writer {
    file: File,
}

impl Writer {
    pub fn write_char(&mut self, c_int: i32) {
        let byte = c_int as u8;
        let _ = self.file.write_all(&[byte]);
    }

    pub fn write_int(&mut self, i: i32) {
        let _ = self.file.write_all(&i.to_le_bytes());
    }

    pub fn write_ints(&mut self, data: &[i32], nb_ints: u64) {
        let n = nb_ints as usize;
        let mut buf = Vec::with_capacity(n * 4);
        for i in 0..n {
            buf.extend_from_slice(&data[i].to_le_bytes());
        }
        let _ = self.file.write_all(&buf);
    }

    pub fn write_sig(&mut self, sig: &[u8]) {
        let n = crate::trusted_utils::SIG_SIZE_BYTES.min(sig.len());
        let _ = self.file.write_all(&sig[..n]);
    }

    pub fn writer_init(output_path: &str) -> Self {
        let file = match File::create(output_path) {
            Ok(f) => f,
            Err(_) => {
                crate::trusted_utils::trusted_utils_exit_eof();
                unreachable!()
            }
        };
        Writer { file }
    }

    pub fn write_bool(&mut self, b: bool) {
        let byte = if b { 1u8 } else { 0u8 };
        let _ = self.file.write_all(&[byte]);
    }

    pub fn write_uls(&mut self, data: &[u64], nb_uls: u64) {
        let n = nb_uls as usize;
        let mut buf = Vec::with_capacity(n * 8);
        for i in 0..n {
            buf.extend_from_slice(&data[i].to_le_bytes());
        }
        let _ = self.file.write_all(&buf);
    }

    pub fn write_ul(&mut self, ul: u64) {
        let _ = self.file.write_all(&ul.to_le_bytes());
    }
}
