use std::fs::File;
use std::io::Write;

pub struct Writer {
    file: File,
}
impl Writer {
    pub fn write_char(&mut self, c_int: i32) {
        let byte = (c_int & 0xff) as u8;
        let _ = self.file.write_all(&[byte]);
    }
    pub fn write_int(&mut self, i: i32) {
        let _ = self.file.write_all(&i.to_le_bytes());
    }
    pub fn write_ints(&mut self, data: &[i32], nb_ints: u64) {
        let n = nb_ints as usize;
        for i in 0..n {
            if i >= data.len() {
                break;
            }
            let _ = self.file.write_all(&data[i].to_le_bytes());
        }
    }
    pub fn write_sig(&mut self, sig: &[u8]) {
        // signature is 16 bytes (4 ints), write all bytes
        let n = sig.len().min(16);
        let _ = self.file.write_all(&sig[..n]);
    }
    pub fn writer_init(output_path: &str) -> Self {
        let file = File::create(output_path).expect("failed to open output file");
        Writer { file }
    }
    pub fn write_bool(&mut self, b: bool) {
        let byte: u8 = if b { 1 } else { 0 };
        let _ = self.file.write_all(&[byte]);
    }
    pub fn write_uls(&mut self, data: &[u64], nb_uls: u64) {
        let n = nb_uls as usize;
        for i in 0..n {
            if i >= data.len() {
                break;
            }
            let _ = self.file.write_all(&data[i].to_le_bytes());
        }
    }
    pub fn write_ul(&mut self, ul: u64) {
        let _ = self.file.write_all(&ul.to_le_bytes());
    }
}
