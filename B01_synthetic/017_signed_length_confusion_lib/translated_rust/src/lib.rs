use std::ffi::c_int;
use std::os::raw::c_char;

unsafe fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe { libc::printf(b"%s\n\0".as_ptr() as *const c_char, line) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_int) {
    unsafe {
        let mut source: [c_char; 100] = [0; 100];
        let mut dest: [c_char; 100] = [0; 100];

        libc::memset(source.as_mut_ptr() as *mut libc::c_void, b'A' as c_int, 100 - 1);
        source[100 - 1] = 0;

        if data < 100 {
            libc::strncpy(
                dest.as_mut_ptr(),
                source.as_ptr(),
                data as libc::size_t,
            );
            *dest.as_mut_ptr().offset(data as isize) = 0;
        }

        print_line(dest.as_ptr());
    }
}
