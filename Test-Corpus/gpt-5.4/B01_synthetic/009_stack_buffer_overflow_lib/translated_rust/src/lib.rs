use std::ffi::c_char;
use std::os::raw::c_int;

fn print_line(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { std::ffi::CStr::from_ptr(line) };
        println!("{}", s.to_string_lossy());
    }
}

fn print_int_line(int_number: c_int) {
    println!("{}", int_number);
}

#[unsafe(no_mangle)]
pub extern "C" fn bad(data: c_int) {
    let mut buffer = [0i32; 10];
    if data >= 0 {
        let idx = data as usize;
        if idx < buffer.len() {
            buffer[idx] = 1;
        } else {
            panic!("index out of bounds");
        }
        for value in buffer {
            print_int_line(value);
        }
    } else {
        println!("ERROR: Array index is negative.");
    }
}

fn good_g2b() {
    let data: c_int = 7;
    let mut buffer = [0i32; 10];
    if data >= 0 {
        let idx = data as usize;
        if idx < buffer.len() {
            buffer[idx] = 1;
        } else {
            panic!("index out of bounds");
        }
        for value in buffer {
            print_int_line(value);
        }
    } else {
        println!("ERROR: Array index is negative.");
    }
}

fn good_b2g(data: c_int) {
    let mut buffer = [0i32; 10];
    if data >= 0 && data < 10 {
        buffer[data as usize] = 1;
        for value in buffer {
            print_int_line(value);
        }
    } else {
        println!("ERROR: Array index is out-of-bounds");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good(data: c_int) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(goodData: c_int, badData: c_int) {
    println!("Calling good()...");
    good(goodData);
    println!("Finished good()");
    println!("Calling bad()...");
    bad(badData);
    println!("Finished bad()");
}
