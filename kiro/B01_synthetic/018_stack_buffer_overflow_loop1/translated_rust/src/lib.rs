use std::ffi::CStr;
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

#[no_mangle]
pub extern "C" fn bad() {
    unsafe {
        let mut buf = [0u8; 10];
        let data = buf.as_mut_ptr() as *mut i32;

        let source: [i32; 10] = [0; 10];
        for i in 0..10 {
            data.add(i).write(source[i]);
        }
        printIntLine(data.read());
    }
}

#[no_mangle]
pub extern "C" fn good() {
    let mut data = [0i32; 10];
    let source = [0i32; 10];
    for i in 0..10 {
        data[i] = source[i];
    }
    printIntLine(data[0]);
}

/// Exported as `main` in the cdylib to match C .so.
/// Build with: RUSTFLAGS='--cfg export_main' cargo build --lib
#[cfg(export_main)]
#[export_name = "main"]
pub extern "C" fn driver_main() -> c_int {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap_or(0);
    let x: c_int = input.trim().parse().unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}
