//! Access to the C runtime's `stdout` / `stderr` streams.
//!
//! `printf`/`fprintf(stderr, ...)` in the original library append to glibc's
//! own stdio buffers, which are shared with whatever C code calls into this
//! library and are flushed at process exit.  Writing through Rust's
//! `std::io::stdout()` would use a *separate* buffer and therefore interleave
//! differently with a caller's `printf`, so the real streams are used instead.

use std::ffi::c_void;

extern "C" {
    static mut stdout: *mut c_void;
    static mut stderr: *mut c_void;
    fn fwrite(ptr: *const u8, size: usize, nitems: usize, stream: *mut c_void) -> usize;
}

/// One `printf("%s", bytes)`-equivalent append to the `stdout` buffer.
pub fn print_stdout(bytes: &[u8]) {
    unsafe {
        let stream = std::ptr::addr_of!(stdout).read();
        fwrite(bytes.as_ptr(), 1, bytes.len(), stream);
    }
}

/// One `fprintf(stderr, "%s", bytes)`-equivalent write (stderr is unbuffered).
pub fn print_stderr(bytes: &[u8]) {
    unsafe {
        let stream = std::ptr::addr_of!(stderr).read();
        fwrite(bytes.as_ptr(), 1, bytes.len(), stream);
    }
}
