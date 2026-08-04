use crate::output_interface::OutputInterface;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;

/// A FileOutput wraps a `File` so that it can be used as an `OutputInterface`.
///
/// Note: The C version's `file_output_new` initializes a struct that points to
/// a `FILE*`. In Rust we can't easily attach an owned `File` to a `&mut dyn
/// OutputInterface` after the fact, so callers should construct a `FileOutput`
/// directly and use it as the interface. We still provide `file_output_new` as
/// a no-op-ish helper for API compatibility (it consumes the file but cannot
/// actually attach it through the trait object).
pub struct FileOutput {
    file: RefCell<File>,
}

impl FileOutput {
    pub fn new(file: File) -> Self {
        FileOutput {
            file: RefCell::new(file),
        }
    }
}

impl OutputInterface for FileOutput {
    fn write(&mut self, data: &[u8]) -> bool {
        self.file.borrow_mut().write_all(data).is_ok()
    }
}

pub fn file_output_new(_output: &mut dyn OutputInterface, _file: File) {
    // The C API attaches a FILE* to an OutputInterface. Rust's trait object
    // doesn't allow that pattern cleanly. Callers should construct a
    // `FileOutput` directly via `FileOutput::new(file)` and pass that as the
    // OutputInterface. This function is kept for API parity.
}
