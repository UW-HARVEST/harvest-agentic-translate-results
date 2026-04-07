use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe { printf(b"%s\n\0".as_ptr() as *const c_char, line) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_int) {
    let mut source = [0u8; 100];
    let mut dest = [0u8; 100];

    // memset(source, 'A', 99)
    source[..99].fill(b'A');
    // source[99] = '\0' — already 0

    if data < 100 {
        let n = data as usize;
        dest[..n].copy_from_slice(&source[..n]);
        dest[n] = 0;
    }

    unsafe { print_line(dest.as_ptr() as *const c_char) };
}
