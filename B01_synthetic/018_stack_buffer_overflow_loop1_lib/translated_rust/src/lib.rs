use std::ffi::{c_char, c_int, CStr};
use std::alloc::{alloc, Layout};

#[unsafe(no_mangle)]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            let s = CStr::from_ptr(line);
            println!("{}", s.to_str().unwrap_or(""));
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn printIntLine(int_number: c_int) {
    println!("{}", int_number);
}

/// Reproduces the C bug: allocates only 10 bytes (not 10*sizeof(int)),
/// then writes 10 ints through that pointer — a buffer overflow.
fn bad() {
    unsafe {
        // alloca(10) — only 10 bytes, not enough for 10 ints
        let layout = Layout::from_size_align(10, std::mem::align_of::<c_int>()).unwrap();
        let data = alloc(layout) as *mut c_int;

        let source: [c_int; 10] = [0; 10];
        for i in 0..10 {
            *data.add(i) = source[i];
        }
        printIntLine(*data);
    }
}

fn good() {
    unsafe {
        // alloca(10*sizeof(int)) — correct size
        let layout = Layout::from_size_align(
            10 * std::mem::size_of::<c_int>(),
            std::mem::align_of::<c_int>(),
        )
        .unwrap();
        let data = alloc(layout) as *mut c_int;

        let source: [c_int; 10] = [0; 10];
        for i in 0..10 {
            *data.add(i) = source[i];
        }
        printIntLine(*data);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(use_good: c_int) {
    if use_good != 0 {
        good();
    } else {
        bad();
    }
}
