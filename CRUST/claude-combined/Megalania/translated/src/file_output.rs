use crate::output_interface::OutputInterface;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;

pub struct FileOutput {
    file: Rc<RefCell<File>>,
}

impl OutputInterface for FileOutput {
    fn write(&mut self, data: &[u8]) -> bool {
        self.file.borrow_mut().write_all(data).is_ok()
    }
}

pub fn file_output_new(_output: &mut dyn OutputInterface, _file: File) {
    // Note: this function in C sets the function pointers and private data
    // for the OutputInterface. In Rust, the OutputInterface trait pattern
    // requires constructing a concrete type. This stub is kept for API
    // compatibility. Use `FileOutput::new` directly to construct an output.
}

impl FileOutput {
    pub fn new(file: File) -> Self {
        Self {
            file: Rc::new(RefCell::new(file)),
        }
    }
}
