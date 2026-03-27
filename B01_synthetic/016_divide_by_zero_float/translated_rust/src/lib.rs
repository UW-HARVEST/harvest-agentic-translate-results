use std::ffi::CStr;
use std::io::{self, BufRead};
use std::os::raw::{c_char, c_int};

#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) };
        println!("{}", s.to_str().unwrap_or(""));
    }
}

#[no_mangle]
pub extern "C" fn printIntLine(n: c_int) {
    println!("{}", n);
}

fn double_to_int_c(v: f64) -> i32 {
    if v.is_nan() || v.is_infinite() || v > i32::MAX as f64 || v < i32::MIN as f64 {
        i32::MIN
    } else {
        v as i32
    }
}

const CHAR_ARRAY_SIZE: usize = 20;

pub fn read_input_float(reader: &mut impl BufRead) -> Option<f32> {
    let mut buf = String::new();
    match reader.read_line(&mut buf) {
        Ok(0) => None,
        Ok(_) => {
            if buf.len() > CHAR_ARRAY_SIZE - 1 {
                buf.truncate(CHAR_ARRAY_SIZE - 1);
            }
            let val = buf.trim_end_matches('\n').parse::<f64>().unwrap_or(0.0);
            Some(val as f32)
        }
        Err(_) => None,
    }
}

pub fn bad_impl(reader: &mut impl BufRead) {
    let data: f32;
    if let Some(val) = read_input_float(reader) {
        data = val;
    } else {
        printLine(b"fgets() failed.\0".as_ptr() as *const c_char);
        data = 0.0_f32;
    }
    let result = double_to_int_c(100.0_f64 / data as f64);
    printIntLine(result);
}

#[no_mangle]
pub extern "C" fn bad() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    bad_impl(&mut reader);
}

pub fn good_g2b() {
    let data: f32 = 2.0;
    let result = double_to_int_c(100.0_f64 / data as f64);
    printIntLine(result);
}

pub fn good_b2g(reader: &mut impl BufRead) {
    let data: f32;
    if let Some(val) = read_input_float(reader) {
        data = val;
    } else {
        printLine(b"fgets() failed.\0".as_ptr() as *const c_char);
        data = 0.0_f32;
    }
    if (data as f64).abs() > 0.000001 {
        let result = double_to_int_c(100.0_f64 / data as f64);
        printIntLine(result);
    } else {
        printLine(b"This would result in a divide by zero\0".as_ptr() as *const c_char);
    }
}

#[no_mangle]
pub extern "C" fn good() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    good_g2b();
    good_b2g(&mut reader);
}

// Only export main for cdylib, not when linked as rlib into the binary or tests
#[cfg(all(not(feature = "_bin"), not(test)))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    printLine(b"Calling good()...\0".as_ptr() as *const c_char);
    good_g2b();
    good_b2g(&mut reader);
    printLine(b"Finished good()\0".as_ptr() as *const c_char);
    printLine(b"Calling bad()...\0".as_ptr() as *const c_char);
    bad_impl(&mut reader);
    printLine(b"Finished bad()\0".as_ptr() as *const c_char);
    0
}
