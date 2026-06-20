use crate::output_interface::OutputInterface;
use std::fs::File;
use std::io::Write;
use std::sync::{Mutex, OnceLock};

fn output_key(output: &mut dyn OutputInterface) -> usize {
    output as *mut dyn OutputInterface as *mut () as usize
}

fn registry() -> &'static Mutex<std::collections::HashMap<usize, File>> {
    static REGISTRY: OnceLock<Mutex<std::collections::HashMap<usize, File>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

pub(crate) fn write_output(output: &mut dyn OutputInterface, data: &[u8]) -> bool {
    let key = output_key(output);
    if let Some(file) = registry().lock().expect("file output registry poisoned").get_mut(&key) {
        file.write_all(data).is_ok()
    } else {
        output.write(data)
    }
}

pub fn file_output_new(output: &mut dyn OutputInterface, file: File) {
    registry()
        .lock()
        .expect("file output registry poisoned")
        .insert(output_key(output), file);
}
