use crate::output_interface::OutputInterface;
use std::fs::File;
use std::io::Write;

/// Provides an `OutputInterface` for writing to a file.
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

/// Equivalent to the C `file_output_new` initializer. Prefer using
/// `FileOutput::new` directly.
pub fn file_output_new(_output: &mut dyn OutputInterface, _file: File) {
    // No-op: the trait object's vtable cannot be mutated.
}
