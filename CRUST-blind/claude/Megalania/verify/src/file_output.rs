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

/// In the C version, this initializes an `OutputInterface` with file write callbacks.
/// Here we provide a no-op stub for API compatibility; users should construct
/// `FileOutput` directly to write to a file.
pub fn file_output_new(_output: &mut dyn OutputInterface, _file: File) {
    // The C API mutates the OutputInterface in-place to attach a file pointer.
    // Since Rust uses trait objects via FileOutput, this function is a no-op.
}
