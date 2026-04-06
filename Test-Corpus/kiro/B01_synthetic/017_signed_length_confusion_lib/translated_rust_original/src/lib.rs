use std::ffi::c_int;

unsafe fn print_line(line: *const u8) {
    if !line.is_null() {
        unsafe {
            libc::printf(b"%s\n\0".as_ptr() as *const libc::c_char, line as *const libc::c_char);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_int) {
    unsafe {
        let mut source = [0u8; 100];
        let mut dest = [0u8; 100];

        libc::memset(source.as_mut_ptr() as *mut libc::c_void, b'A' as c_int, 99);
        source[99] = 0;

        if data < 100 {
            // Reproduce C behavior: negative int cast to size_t (usize)
            libc::strncpy(
                dest.as_mut_ptr() as *mut libc::c_char,
                source.as_ptr() as *const libc::c_char,
                data as usize,
            );
            dest[data as usize] = 0;
        }

        print_line(dest.as_ptr());
    }
}
