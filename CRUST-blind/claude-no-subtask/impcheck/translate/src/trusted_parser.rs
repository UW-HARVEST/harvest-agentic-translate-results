use std::fs::File;
use std::io::Write;

pub struct IntVec;

pub struct TrustedParser {
    f_out: File,
    f: File,
}
impl TrustedParser {
    pub fn tp_end(&mut self) {
        let _ = self.f_out.flush();
        let _ = &self.f;
    }
    pub fn output_literal_buffer(&self) {}
    pub fn append_integer(&self) {}
    pub fn tp_init(filename: &str, out: File) -> Self {
        let f = File::open(filename).expect("open input");
        TrustedParser { f_out: out, f }
    }
    pub fn tp_parse(&mut self, sig: &mut Option<Vec<u8>>) -> bool {
        *sig = Some(vec![0u8; 16]);
        true
    }
    pub fn process(&mut self, _c: char) -> bool {
        true
    }
}
