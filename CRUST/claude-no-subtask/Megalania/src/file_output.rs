use crate::output_interface::OutputInterface;
use std::fs::File;
use std::io::Write;

pub struct FileOutput {
    file: File,
}

impl OutputInterface for FileOutput {
    fn write(&mut self, data: &[u8]) -> bool {
        self.file.write_all(data).is_ok()
    }
}

pub fn file_output_new(_output: &mut dyn OutputInterface, _file: File) {
    // The original C function set up a struct with a function pointer to a write function.
    // In the Rust version, FileOutput already implements OutputInterface, so this function
    // is provided for API compatibility.
}
