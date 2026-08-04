use std::ffi::c_char;
use std::ffi::c_int;

fn print_line(line: *const c_char) {
    if !line.is_null() {
        unsafe {
            libc::printf(b"%s\n\0".as_ptr() as *const c_char, line);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_int) {
    let mut source: [c_char; 100] = [0; 100];
    let mut dest: [c_char; 100] = [0; 100];

    // memset(source, 'A', 100-1);
    for i in 0..(100 - 1) {
        source[i] = b'A' as c_char;
    }
    // source[100-1] = '\0';
    source[100 - 1] = 0;

    if data < 100 {
        // strncpy(dest, source, data);
        // dest[data] = '\0';
        unsafe {
            libc::strncpy(
                dest.as_mut_ptr(),
                source.as_ptr(),
                data as libc::size_t,
            );
            // dest[data] = '\0'; -- replicates C behavior, including UB if data < 0
            *dest.as_mut_ptr().offset(data as isize) = 0;
        }
    }

    print_line(dest.as_ptr());
}
