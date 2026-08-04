use std::ffi::{c_char, c_int, CStr};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::os::raw::c_void;
use std::ptr;

fn forward_goto_example(x: c_int) -> c_int {
    if x < 0 {
        eprintln!("Error: negative input");
        return -1;
    }

    println!("Processing: {}", x);
    x * 2
}

fn open_with_cleanup(filename: &str) -> Option<File> {
    let file = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Error: opening or processing file {}", filename);
            return None;
        }
    };

    let reader = BufReader::new(&file);
    for line in reader.lines() {
        match line {
            Ok(l) => println!("{}", l),
            Err(_) => {
                eprintln!("Error: opening or processing file {}", filename);
                return None;
            }
        }
    }

    Some(file)
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(num: c_int, filename: *const c_char) -> c_int {
    let res = forward_goto_example(num);
    if res == -1 {
        return -1;
    } else {
        println!("Goto output: {}", res);
    }

    let c_str = unsafe {
        if filename.is_null() {
            return -2;
        }
        CStr::from_ptr(filename)
    };

    let filename_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    match open_with_cleanup(filename_str) {
        None => -2,
        Some(_) => 0,
    }
}
