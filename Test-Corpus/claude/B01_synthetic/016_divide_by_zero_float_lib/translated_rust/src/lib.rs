#![allow(non_snake_case)]

use std::ffi::{c_char, c_float, c_int, CStr};

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let s = unsafe { CStr::from_ptr(line) };
        // Use lossy conversion to match C's printf %s behavior on byte sequences.
        // Since inputs in this file are pure ASCII, this is byte-identical.
        match s.to_str() {
            Ok(st) => println!("{}", st),
            Err(_) => {
                // Fallback: print raw bytes
                let bytes = s.to_bytes();
                use std::io::Write;
                let stdout = std::io::stdout();
                let mut handle = stdout.lock();
                let _ = handle.write_all(bytes);
                let _ = handle.write_all(b"\n");
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(intNumber: c_int) {
    println!("{}", intNumber);
}

// Match C's `(int)x` semantics for double->int. On x86_64 this compiles to
// `cvttsd2si`, which is what C compilers emit for `(int)x`.
#[inline]
fn c_double_to_int(x: f64) -> c_int {
    unsafe { x.to_int_unchecked::<c_int>() }
}

#[unsafe(no_mangle)]
pub extern "C" fn bad(data: c_float) {
    let result: c_int = c_double_to_int(100.0_f64 / data as f64);
    printIntLine(result);
}

fn good_g2b() {
    let data: c_float;
    data = 2.0_f32;
    {
        let result: c_int = c_double_to_int(100.0_f64 / data as f64);
        printIntLine(result);
    }
}

fn good_b2g(data: c_float) {
    if (data as f64).abs() > 0.000001 {
        let result: c_int = c_double_to_int(100.0_f64 / data as f64);
        printIntLine(result);
    } else {
        let msg = b"This would result in a divide by zero\0";
        printLine(msg.as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn good(data: c_float) {
    good_g2b();
    good_b2g(data);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(good_data: c_float, bad_data: c_float) {
    let s1 = b"Calling good()...\0";
    printLine(s1.as_ptr() as *const c_char);
    good(good_data);
    let s2 = b"Finished good()\0";
    printLine(s2.as_ptr() as *const c_char);
    let s3 = b"Calling bad()...\0";
    printLine(s3.as_ptr() as *const c_char);
    bad(bad_data);
    let s4 = b"Finished bad()\0";
    printLine(s4.as_ptr() as *const c_char);
}
