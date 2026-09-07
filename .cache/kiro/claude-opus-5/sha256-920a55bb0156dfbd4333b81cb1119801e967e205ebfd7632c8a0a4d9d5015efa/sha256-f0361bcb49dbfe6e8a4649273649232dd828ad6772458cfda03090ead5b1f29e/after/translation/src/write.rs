//! Translation of `c_src/src/write.c`.
//!
//! Note the C parameter is named `content` in the definition (`contents` in the
//! header); only the type signature matters across the ABI.

use core::ffi::{c_char, c_int};

use crate::cstd::{self, EINVAL, errno, fclose, fopen, stderr, strerror};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(filename: *const c_char, content: *const c_char) -> c_int {
    unsafe {
        if content.is_null() {
            cstd::fprintf(stderr, c"Error: Content is NULL.\n".as_ptr());
            return EINVAL;
        }

        let file = fopen(filename, c"w".as_ptr());
        if file.is_null() {
            cstd::fprintf(
                stderr,
                c"Error opening file '%s': %s\n".as_ptr(),
                filename,
                strerror(errno()),
            );
            return errno();
        }

        if cstd::fprintf(file, c"%s".as_ptr(), content) < 0 {
            cstd::fprintf(
                stderr,
                c"Error writing to file '%s': %s\n".as_ptr(),
                filename,
                strerror(errno()),
            );
            fclose(file);
            return errno();
        }

        if fclose(file) != 0 {
            cstd::fprintf(
                stderr,
                c"Error closing file '%s': %s\n".as_ptr(),
                filename,
                strerror(errno()),
            );
            return errno();
        }

        0
    }
}
