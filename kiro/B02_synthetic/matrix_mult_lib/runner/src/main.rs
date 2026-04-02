#![cfg_attr(fuzzing, no_main)]

use std::{fs, io};

use cando2::*;

const FILE: &str = "matrix.txt";

harness! {
    state: {
        width_a: c_int,
        height_a: c_int,
        matrix_a: Option<CString>,
        width_b: c_int,
        height_b: c_int,
        matrix_b: Option<CString>,
        returns: c_int,
        file_content: Option<String>,
    },

    library: "driver",
    symbol: "driver",

    signature: unsafe extern "C" fn(c_int, c_int, *const c_char, c_int, c_int, *const c_char) -> c_int,

    fn run(&mut self) {
        // Just in case the file is somehow still there
        let _ = fs::remove_file(FILE);

        let mat_a = match &self.matrix_a {
            Some(a) => a.as_ptr(),
            None => std::ptr::null(),
        };
        let mat_b = match &self.matrix_b {
            Some(b) => b.as_ptr(),
            None => std::ptr::null(),
        };

        self.returns = unsafe {
            (*SYMBOL)(
                self.width_a,
                self.height_a,
                mat_a,
                self.width_b,
                self.height_b,
                mat_b,
            )
        };

        self.file_content = match fs::read_to_string(FILE) {
            Ok(c) => Some(c),
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => None,
                _ => panic!("Rust Runner: Error reading FILE"),
            }
        };

        // Cleanup file
        let _ = fs::remove_file(FILE);
    }
}
