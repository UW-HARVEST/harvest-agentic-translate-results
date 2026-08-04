use crate::output_interface::OutputInterface;
use std::fs::File;
pub fn file_output_new(output: &mut dyn OutputInterface, file: File) {
    let _ = output;
    let _ = file;
}
