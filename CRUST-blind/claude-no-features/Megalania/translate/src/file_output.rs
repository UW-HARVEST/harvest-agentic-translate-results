use crate::output_interface::OutputInterface;
use std::fs::File;
use std::io::Write;

/// A file-backed implementation of `OutputInterface`. Use this directly
/// in idiomatic Rust code instead of `file_output_new`.
pub struct FileOutput {
    pub file: File,
}

impl FileOutput {
    pub fn new(file: File) -> Self {
        FileOutput { file }
    }
}

impl OutputInterface for FileOutput {
    fn write(&mut self, data: &[u8]) -> bool {
        self.file.write_all(data).is_ok()
    }
}

/// In the original C code, this populates an OutputInterface struct with
/// function pointers. In idiomatic Rust, prefer `FileOutput::new(file)`.
pub fn file_output_new(_output: &mut dyn OutputInterface, _file: File) {
    // No-op: prefer FileOutput::new(file).
}
