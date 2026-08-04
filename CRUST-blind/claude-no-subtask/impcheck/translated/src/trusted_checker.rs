use std::fs::File;
use std::io::Write;

pub struct TrustedChecker {
    input: File,
    output: File,
}
impl TrustedChecker {
    pub fn tc_run(_check_model: bool, _lenient: bool) -> i32 {
        // Stub: trusted checker run loop.
        0
    }
    pub fn tc_init(fifo_in: &str, fifo_out: &str) -> Self {
        let input = File::open(fifo_in).expect("open input");
        let output = File::create(fifo_out).expect("create output");
        TrustedChecker { input, output }
    }
    pub fn tc_end(&mut self) {
        let _ = self.output.flush();
        let _ = &self.input;
    }
    pub fn read_literals(&mut self, _nb_lits: i32) {}
    pub fn read_hints(&mut self, _nb_hints: i32) {}
    pub fn say_with_flush(&mut self, ok: bool) {
        let byte: u8 = if ok { b'A' } else { b'E' };
        let _ = self.output.write_all(&[byte]);
        let _ = self.output.flush();
    }
    pub fn say(&mut self, ok: bool) {
        let byte: u8 = if ok { b'A' } else { b'E' };
        let _ = self.output.write_all(&[byte]);
    }
}
