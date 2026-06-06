use crate::output_interface::OutputInterface;
use std::fs::File;
use std::io::Write;

pub struct FileOutput {
    pub file: File,
}

impl OutputInterface for FileOutput {
    fn write(&mut self, data: &[u8]) -> bool {
        self.file.write_all(data).is_ok()
    }
}

pub fn file_output_new(_output: &mut dyn OutputInterface, _file: File) {
    // The original C function setup the OutputInterface struct with a function pointer
    // and the file handle. In Rust, callers should construct a FileOutput value directly
    // and use it as a `&mut dyn OutputInterface`.
    // This function is kept for API parity; it does nothing.
}
