//! Translation of `c_src/src/write.c` (public surface in `c_src/include/write.h`).

use core::ffi::{c_char, c_int};

use crate::cffi::{errno, fclose, fopen, fprintf, stderr_stream, strerror, EINVAL};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(filename: *const c_char, content: *const c_char) -> c_int {
    if content.is_null() {
        fprintf(stderr_stream(), c"Error: Content is NULL.\n".as_ptr());
        return EINVAL;
    }

    let file = fopen(filename, c"w".as_ptr());
    if file.is_null() {
        // `strerror(errno)` is evaluated before the fprintf call, but the
        // `return errno;` that follows re-reads errno *after* it. Preserved.
        let saved = errno();
        fprintf(
            stderr_stream(),
            c"Error opening file '%s': %s\n".as_ptr(),
            filename,
            strerror(saved),
        );
        return errno();
    }

    if fprintf(file, c"%s".as_ptr(), content) < 0 {
        let saved = errno();
        fprintf(
            stderr_stream(),
            c"Error writing to file '%s': %s\n".as_ptr(),
            filename,
            strerror(saved),
        );
        fclose(file);
        return errno();
    }

    if fclose(file) != 0 {
        let saved = errno();
        fprintf(
            stderr_stream(),
            c"Error closing file '%s': %s\n".as_ptr(),
            filename,
            strerror(saved),
        );
        return errno();
    }

    0
}
