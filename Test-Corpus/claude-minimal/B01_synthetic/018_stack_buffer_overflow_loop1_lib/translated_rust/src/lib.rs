// Translation of c_src/src/driver.c to Rust.

use std::os::raw::{c_char, c_int};

/// Print a C-style string followed by a newline. If the pointer is null,
/// nothing is printed (mirroring the original C behavior).
///
/// # Safety
/// `line` must either be null or point to a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let cstr = std::ffi::CStr::from_ptr(line);
        match cstr.to_str() {
            Ok(s) => println!("{}", s),
            Err(_) => {
                // Fall back to lossy printing for non-UTF-8 sequences.
                println!("{}", cstr.to_string_lossy());
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn printIntLine(int_number: c_int) {
    println!("{}", int_number);
}

/// Translation of the C `bad()` function. The original uses
/// `alloca(10)` (10 bytes) and then writes 10 ints into it, which is a
/// stack buffer overflow. We mimic the structure but use a safe Rust
/// stack allocation big enough to avoid undefined behavior in Rust,
/// while preserving the observable output (printing the first int).
fn bad() {
    // The original C allocates only 10 bytes via alloca(10) and then writes
    // 10 ints (40 bytes), which is a buffer overflow in C. To preserve the
    // observable behavior of the original program (printing source[0] which
    // is 0), we use a properly sized buffer in Rust.
    let mut data: [c_int; 10] = [0; 10];
    let source: [c_int; 10] = [0; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    printIntLine(data[0]);
}

fn good() {
    let mut data: [c_int; 10] = [0; 10];
    let source: [c_int; 10] = [0; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    printIntLine(data[0]);
}

#[no_mangle]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
