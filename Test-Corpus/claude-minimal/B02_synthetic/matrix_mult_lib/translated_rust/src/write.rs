// Translated from c_src/src/write.c
// Original Copyright 2025 MIT Lincoln Laboratory

use std::fs::File;
use std::io::Write;

pub fn write_to_file(filename: &str, content: &str) -> i32 {
    let mut file = match File::create(filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error opening file '{}': {}", filename, e);
            return e.raw_os_error().unwrap_or(libc_eio());
        }
    };

    if let Err(e) = file.write_all(content.as_bytes()) {
        eprintln!("Error writing to file '{}': {}", filename, e);
        return e.raw_os_error().unwrap_or(libc_eio());
    }

    if let Err(e) = file.sync_all() {
        eprintln!("Error closing file '{}': {}", filename, e);
        return e.raw_os_error().unwrap_or(libc_eio());
    }

    0
}

fn libc_eio() -> i32 {
    5 // EIO on Linux
}
