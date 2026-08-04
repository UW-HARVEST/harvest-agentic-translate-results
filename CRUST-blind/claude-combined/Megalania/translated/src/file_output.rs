use crate::output_interface::OutputInterface;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;

pub struct FileOutput {
    pub file: RefCell<File>,
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
        self.file.borrow_mut().write_all(data).is_ok()
    }
}

pub fn file_output_new(_output: &mut dyn OutputInterface, _file: File) {
    // The Rust API uses the FileOutput struct directly via FileOutput::new(file).
}
