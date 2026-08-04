use std::io::{self, BufRead};
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

#[inline]
fn c_double_to_i32(v: f64) -> i32 {
    if v.is_nan() || v.is_infinite() || v > i32::MAX as f64 || v < i32::MIN as f64 {
        i32::MIN
    } else {
        v as i32
    }
}

const CHAR_ARRAY_SIZE: usize = 20;

fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(n: i32) {
    println!("{}", n);
}

fn c_atof(s: &str) -> f64 {
    let s = s.trim_start();
    let mut last_ok = 0.0_f64;
    let mut found = false;
    for i in 1..=s.len() {
        if let Ok(v) = s[..i].parse::<f64>() {
            last_ok = v;
            found = true;
        } else if found {
            break;
        }
    }
    last_ok
}

fn fgets_stdin() -> Option<String> {
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buf = Vec::new();
    let limit = CHAR_ARRAY_SIZE - 1;
    loop {
        let available = match handle.fill_buf() {
            Ok(b) if b.is_empty() => {
                if buf.is_empty() { return None; }
                break;
            }
            Ok(b) => b,
            Err(_) => {
                if buf.is_empty() { return None; }
                break;
            }
        };
        let remaining = limit - buf.len();
        let to_copy = available.len().min(remaining);
        let chunk = &available[..to_copy];
        if let Some(nl) = chunk.iter().position(|&b| b == b'\n') {
            buf.extend_from_slice(&chunk[..=nl]);
            handle.consume(nl + 1);
            break;
        }
        buf.extend_from_slice(chunk);
        handle.consume(to_copy);
        if buf.len() >= limit { break; }
    }
    Some(String::from_utf8_lossy(&buf).into_owned())
}

fn impl_bad() {
    let mut data: f32 = 0.0;
    if let Some(input) = fgets_stdin() {
        data = c_atof(&input) as f32;
    } else {
        print_line("fgets() failed.");
    }
    let result = c_double_to_i32(100.0_f64 / data as f64);
    print_int_line(result);
}

fn good_g2b() {
    let data: f32 = 2.0;
    let result = c_double_to_i32(100.0_f64 / data as f64);
    print_int_line(result);
}

fn good_b2g() {
    let mut data: f32 = 0.0;
    if let Some(input) = fgets_stdin() {
        data = c_atof(&input) as f32;
    } else {
        print_line("fgets() failed.");
    }
    if (data as f64).abs() > 0.000001 {
        let result = c_double_to_i32(100.0_f64 / data as f64);
        print_int_line(result);
    } else {
        print_line("This would result in a divide by zero");
    }
}

fn impl_good() {
    good_g2b();
    good_b2g();
}

pub fn run_main() {
    print_line("Calling good()...");
    impl_good();
    print_line("Finished good()");
    print_line("Calling bad()...");
    impl_bad();
    print_line("Finished bad()");
}

// --- #[no_mangle] exports matching C shared library symbols ---

#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) }.to_string_lossy();
        println!("{}", s);
    }
}

#[no_mangle]
pub extern "C" fn printIntLine(n: c_int) {
    println!("{}", n);
}

#[no_mangle]
pub extern "C" fn bad() {
    impl_bad();
}

#[no_mangle]
pub extern "C" fn good() {
    impl_good();
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    run_main();
    0
}

