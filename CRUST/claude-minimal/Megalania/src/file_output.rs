use crate::output_interface::OutputInterface;
use std::fs::File;
use std::io::Write;

pub struct FileOutput {
    file: File,
}

impl FileOutput {
    pub fn new(file: File) -> Self {
        Self { file }
    }
}

impl OutputInterface for FileOutput {
    fn write(&mut self, data: &[u8]) -> bool {
        self.file.write_all(data).is_ok()
    }
}

/// API-compatible shim for the C `file_output_new`. The C function set up a
/// vtable-style struct in place. In Rust the natural translation is to
/// construct a `FileOutput` value and use it as the `OutputInterface`.
pub fn file_output_new(_output: &mut dyn OutputInterface, _file: File) {
    // No direct equivalent in safe Rust: callers should construct a
    // `FileOutput::new(file)` and pass it where an `&mut dyn OutputInterface`
    // is required.
}
