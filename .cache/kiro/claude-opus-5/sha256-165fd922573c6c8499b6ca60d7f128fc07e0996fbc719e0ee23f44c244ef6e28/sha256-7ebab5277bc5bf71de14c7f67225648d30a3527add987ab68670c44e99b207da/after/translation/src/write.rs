//! Translation of `c_src/src/write.c`.

use std::ffi::c_char;
use std::ffi::c_int;

use crate::cutil::{EINVAL, errno, fclose, fopen, fputs, stderr_write, strerror_bytes};

/// Renders `"<prefix>'<filename>': <strerror(err)>\n"` exactly like the
/// original `fprintf(stderr, ...)` calls do.
fn report(prefix: &[u8], filename: *const c_char, err: c_int) {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(prefix);
    buf.push(b'\'');
    unsafe {
        let mut p = filename;
        while *p != 0 {
            buf.push(*p as u8);
            p = p.add(1);
        }
    }
    buf.extend_from_slice(b"': ");
    buf.extend_from_slice(&strerror_bytes(err));
    buf.push(b'\n');
    stderr_write(&buf);
}

#[unsafe(no_mangle)]
pub extern "C" fn write_to_file(filename: *const c_char, content: *const c_char) -> c_int {
    unsafe {
        if content.is_null() {
            stderr_write(b"Error: Content is NULL.\n");
            return EINVAL;
        }

        let file = fopen(filename, c"w".as_ptr());
        if file.is_null() {
            report(b"Error opening file ", filename, errno());
            // The original evaluates `strerror(errno)` for the message and then
            // reads `errno` again for the return value.
            return errno();
        }

        // `fprintf(file, "%s", content)` is equivalent to fputs here; both
        // return a negative value on failure.
        if fputs(content, file) < 0 {
            report(b"Error writing to file ", filename, errno());
            fclose(file);
            // As in the original, this reads `errno` after the fclose above.
            return errno();
        }

        if fclose(file) != 0 {
            report(b"Error closing file ", filename, errno());
            return errno();
        }

        0
    }
}
