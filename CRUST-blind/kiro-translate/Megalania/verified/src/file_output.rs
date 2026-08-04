use crate::output_interface::OutputInterface;
use std::fs::File;
use std::io::Write;

pub struct FileOutput {
    file: File,
}

impl FileOutput {
    pub fn from_file(file: File) -> Self {
        FileOutput { file }
    }
}

impl OutputInterface for FileOutput {
    fn write(&mut self, data: &[u8]) -> bool {
        self.file.write_all(data).is_ok()
    }
}

pub fn file_output_new(_output: &mut dyn OutputInterface, _file: File) {
    // In Rust, use FileOutput::from_file() directly instead.
}
