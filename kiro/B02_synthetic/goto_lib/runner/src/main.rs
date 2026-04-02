#![cfg_attr(fuzzing, no_main)]

use std::{fs, io::Write};

use cando2::*;

harness! {
    state: {
        num: c_int,
        filename: Option<CString>,
        file_content: Option<String>,
        returns: c_int,
    },
    library: "driver",
    symbol: "driver",
    signature: unsafe extern "C" fn(c_int, *const c_char) -> c_int,

    fn run(&mut self) {
        if let Some(f) = &self.filename {
            let filename = f.to_str().expect("Rust Runner: Error invalid UTF-8 in filename");
            let mut file = fs::File::create(filename).expect(&format!("Rust Runner: Error creating file {:?}", f));

            if let Some(c) = &self.file_content {
                file.write_all(c.as_bytes()).expect(&format!("Rust Runner: Error writing to file {:?}", f));
            }
        }

        let filename = match &self.filename {
            None => std::ptr::null(),
            Some(f) => f.as_ptr(),
        };

        self.returns = unsafe {
            (*SYMBOL)(
                self.num,
                filename
            )
        };

        if let Some(f) = &self.filename {
            let filename = f.to_str().expect("Rust Runner: Error invalid UTF-8 in filename");
            let _ = fs::remove_file(filename);
        }
    }
}
