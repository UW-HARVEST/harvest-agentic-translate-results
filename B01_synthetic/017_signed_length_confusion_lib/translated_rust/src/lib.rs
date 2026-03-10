use std::ffi::c_int;
use std::io::{self, Write};

unsafe fn print_line(line: *const u8) {
    if !line.is_null() {
        let mut len = 0usize;
        while *line.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(line, len);
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(slice);
        let _ = handle.write_all(b"\n");
    }
}

/// # Safety
/// Reproduces exact C behavior including signed/unsigned length confusion bug.
#[unsafe(no_mangle)]
pub extern "C" fn driver(data: c_int) {
    unsafe {
        let mut source = [b'A'; 100];
        source[99] = 0u8;
        let mut dest = [0u8; 100];

        if data < 100 {
            let n = data as usize; // signed-to-unsigned conversion mirrors C strncpy size_t cast
            // Reproduce strncpy: copy up to n bytes, stop at NUL in source
            let mut i = 0usize;
            while i < n {
                let c = *source.as_ptr().add(i);
                *dest.as_mut_ptr().add(i) = c;
                if c == 0 {
                    break;
                }
                i += 1;
            }
            // Pad remaining with zeros (strncpy behavior)
            while i < n {
                *dest.as_mut_ptr().add(i) = 0;
                i += 1;
            }
            *dest.as_mut_ptr().add(data as usize) = 0;
        }

        print_line(dest.as_ptr());
    }
}
