use std::fs::File;

pub struct IntVec;

pub struct TrustedParser {
    f_out: File,
    f: File,
}

impl TrustedParser {
    pub fn tp_end(&mut self) {
        // Stub - resources released on drop.
        let _ = &self.f;
        let _ = &self.f_out;
    }

    pub fn output_literal_buffer(&self) {
        // Stub.
    }

    pub fn append_integer(&self) {
        // Stub.
    }

    pub fn tp_init(filename: &str, out: File) -> Self {
        let f = File::open(filename).unwrap_or_else(|_| {
            crate::trusted_utils::trusted_utils_exit_eof();
            unreachable!()
        });
        TrustedParser { f_out: out, f }
    }

    pub fn tp_parse(&mut self, _sig: &mut Option<Vec<u8>>) -> bool {
        // Stub: full parser pipeline not implemented in this Rust port.
        true
    }

    pub fn process(&mut self, _c: char) -> bool {
        // Stub.
        false
    }
}
