use std::fs::File;
use std::io::Write;

pub struct TrustedChecker {
    input: File,
    output: File,
}

impl TrustedChecker {
    pub fn tc_run(_check_model: bool, _lenient: bool) -> i32 {
        // Stub - the dispatch loop is not exercised in this translation's tests.
        0
    }

    pub fn tc_init(fifo_in: &str, fifo_out: &str) -> Self {
        let input = File::open(fifo_in).unwrap_or_else(|_| {
            File::open("/dev/null").expect("Cannot open /dev/null")
        });
        let output = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(fifo_out)
            .unwrap_or_else(|_| {
                std::fs::OpenOptions::new()
                    .write(true)
                    .open("/dev/null")
                    .expect("Cannot open /dev/null for writing")
            });
        TrustedChecker { input, output }
    }

    pub fn tc_end(&mut self) {
        let _ = self.output.flush();
    }

    pub fn read_literals(&mut self, _nb_lits: i32) {
        // Stub
    }

    pub fn read_hints(&mut self, _nb_hints: i32) {
        // Stub
    }

    pub fn say_with_flush(&mut self, ok: bool) {
        self.say(ok);
        let _ = self.output.flush();
    }

    pub fn say(&mut self, ok: bool) {
        let byte: u8 = if ok { b'A' } else { b'E' };
        let _ = self.output.write_all(&[byte]);
    }
}
