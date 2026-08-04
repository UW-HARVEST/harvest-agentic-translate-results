use crate::output_interface::OutputInterface;
use std::fs::File;
use std::io::Write;

/// Wraps a `File` so it can be used as an `OutputInterface`.
pub struct FileOutput {
    pub file: File,
}

impl OutputInterface for FileOutput {
    fn write(&mut self, data: &[u8]) -> bool {
        self.file.write_all(data).is_ok()
    }
}

/// Compatibility shim that mirrors the C API.
///
/// The C-style "constructor" cannot rewire a `&mut dyn OutputInterface`
/// since Rust trait objects don't expose a way to swap their vtable.
/// Use `FileOutput { file }` directly to construct a writable interface.
pub fn file_output_new(_output: &mut dyn OutputInterface, _file: File) {
    // No-op: a `&mut dyn OutputInterface` cannot be reassigned to a different
    // implementation through a reference; consumers should construct
    // `FileOutput` directly when they need a file-backed output.
}
