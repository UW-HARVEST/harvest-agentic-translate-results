use std::fs::File;
use std::io::Write;
pub struct Writer {
    file: File,
}
impl Writer {
    pub fn write_char(&mut self, c_int: i32) {
        let _ = self.file.write_all(&[c_int as u8]);
    }
    pub fn write_int(&mut self, i: i32) {
        let _ = self.file.write_all(&i.to_ne_bytes());
    }
    pub fn write_ints(&mut self, data: &[i32], nb_ints: u64) {
        let nb = nb_ints as usize;
        for j in 0..nb {
            let _ = self.file.write_all(&data[j].to_ne_bytes());
        }
    }
    pub fn write_sig(&mut self, sig: &[u8]) {
        let _ = self.file.write_all(&sig[..crate::trusted_utils::SIG_SIZE_BYTES]);
    }
    pub fn writer_init(output_path: &str) -> Self {
        let file = File::create(output_path).expect("Failed to open writer output");
        Writer { file }
    }
    pub fn write_bool(&mut self, b: bool) {
        let byte = if b { 1u8 } else { 0u8 };
        let _ = self.file.write_all(&[byte]);
    }
    pub fn write_uls(&mut self, data: &[u64], nb_uls: u64) {
        let nb = nb_uls as usize;
        for j in 0..nb {
            let _ = self.file.write_all(&data[j].to_ne_bytes());
        }
    }
    pub fn write_ul(&mut self, ul: u64) {
        let _ = self.file.write_all(&ul.to_ne_bytes());
    }
}
