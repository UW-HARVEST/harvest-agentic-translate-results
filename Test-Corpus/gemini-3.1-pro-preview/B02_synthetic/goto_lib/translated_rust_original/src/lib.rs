use std::ffi::CStr;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::raw::{c_char, c_int};

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

    let mut reader = BufReader::new(&file);
    let mut buffer = String::new();
    loop {
        buffer.clear();
        match reader.read_line(&mut buffer) {
            Ok(0) => break,
            Ok(_) => print!("{}", buffer),
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

    let filename_str = unsafe {
        if filename.is_null() {
            return -2;
        }
        CStr::from_ptr(filename).to_string_lossy()
    };

    let out = open_with_cleanup(&filename_str);
    if out.is_none() {
        return -2;
    }

    0
}
