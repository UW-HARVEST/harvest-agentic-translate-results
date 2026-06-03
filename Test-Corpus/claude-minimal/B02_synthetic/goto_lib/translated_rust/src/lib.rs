/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

use std::ffi::{CStr, c_char, c_int};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

fn forward_goto_example(x: i32) -> i32 {
    if x < 0 {
        eprintln!("Error: negative input");
        return -1;
    }

    println!("Processing: {}", x);
    x * 2
}

/// Opens the file at `filename` and prints all of its lines to stdout.
/// Returns `Some(File)` on success, or `None` on any failure (and prints an
/// error message to stderr in the failure case).
fn open_with_cleanup(filename: &str) -> Option<File> {
    let fp = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: opening or processing file {}", filename);
            return None;
        }
    };

    // We need to read lines but also return the underlying File handle.
    // Use a BufReader on a clone of the file handle so the original remains
    // usable for the caller (mirrors C semantics where fp is returned).
    let reader_handle = match fp.try_clone() {
        Ok(h) => h,
        Err(_) => {
            eprintln!("Error: opening or processing file {}", filename);
            return None;
        }
    };

    let mut reader = BufReader::new(reader_handle);
    let mut buffer = String::new();
    loop {
        buffer.clear();
        match reader.read_line(&mut buffer) {
            Ok(0) => break, // EOF
            Ok(_) => {
                // Print without adding an extra newline (line already contains it,
                // matching C's printf("%s", buffer) behavior).
                print!("{}", buffer);
                let _ = std::io::stdout().flush();
            }
            Err(_) => {
                eprintln!("Error: opening or processing file {}", filename);
                return None;
            }
        }
    }

    Some(fp)
}

pub fn driver_rs(num: i32, filename: &str) -> i32 {
    let res = forward_goto_example(num);
    if res == -1 {
        return -1;
    } else {
        println!("Goto output: {}", res);
    }

    let out = open_with_cleanup(filename);
    if out.is_none() {
        return -2;
    }
    // Dropping `out` closes the file (equivalent to fclose).
    drop(out);

    0
}

/// C-compatible entry point: `int driver(int num, const char* filename)`.
#[no_mangle]
pub extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
    if filename.is_null() {
        return -2;
    }
    let c_str = unsafe { CStr::from_ptr(filename) };
    let filename_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };
    driver_rs(num as i32, filename_str) as c_int
}
