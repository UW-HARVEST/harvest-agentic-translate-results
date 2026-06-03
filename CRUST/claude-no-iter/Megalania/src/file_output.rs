use crate::output_interface::OutputInterface;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;

/// A concrete `OutputInterface` that writes to a file.
pub struct FileOutput {
    file: RefCell<File>,
}

impl FileOutput {
    pub fn new(file: File) -> Self {
        Self {
            file: RefCell::new(file),
        }
    }
}

impl OutputInterface for FileOutput {
    fn write(&mut self, data: &[u8]) -> bool {
        self.file.get_mut().write_all(data).is_ok()
    }
}

/// Configures the output interface to write to a file.
///
/// In C this initialized function pointers and a `private_data` field on the
/// `OutputInterface` struct. Rust traits cannot swap the dynamic type behind a
/// `&mut dyn OutputInterface`, so this is a no-op preserved for API parity.
/// Construct a [`FileOutput`] directly when you need a real implementation.
pub fn file_output_new(_output: &mut dyn OutputInterface, _file: File) {
    // Intentionally a no-op. Use `FileOutput::new` to construct a concrete
    // file-backed output interface.
}
