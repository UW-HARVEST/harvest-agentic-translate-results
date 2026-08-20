//! C `<stdio.h>`-compatible output.
//!
//! The translation is loaded next to the original C library (and, as the
//! `driver` binary, replaces the original C program), so its output has to be
//! byte-identical *and* has to share the C runtime's buffering behaviour.  The
//! simplest way to guarantee both is to write through the very same
//! `FILE *stdout` / `FILE *stderr` streams that `printf`/`fprintf` use.

use core::ffi::c_void;

extern "C" {
    /// `extern FILE *stdout;`
    #[link_name = "stdout"]
    static mut c_stdout: *mut libc::FILE;
    /// `extern FILE *stderr;`
    #[link_name = "stderr"]
    static mut c_stderr: *mut libc::FILE;
}

/// `printf("%s", ..)` for arbitrary (possibly non-UTF-8) bytes.
pub fn out_bytes(bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }
    unsafe {
        libc::fwrite(
            bytes.as_ptr() as *const c_void,
            1,
            bytes.len(),
            c_stdout,
        );
    }
}

/// `printf("%s", ..)`
pub fn out_str(s: &str) {
    out_bytes(s.as_bytes());
}

/// `fprintf(stderr, "%s", ..)` -- `stderr` is unbuffered in C, and since we
/// write through the same stream that property is preserved automatically.
pub fn err_bytes(bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }
    unsafe {
        libc::fwrite(
            bytes.as_ptr() as *const c_void,
            1,
            bytes.len(),
            c_stderr,
        );
    }
}

/// `fprintf(stderr, ..)`
pub fn err_str(s: &str) {
    err_bytes(s.as_bytes());
}

/// `fflush(NULL)` -- flush every C stream, as `exit()` does at process teardown.
pub fn flush_all() {
    unsafe {
        libc::fflush(core::ptr::null_mut());
    }
}

/// `printf(...)`
#[macro_export]
macro_rules! cprintf {
    ($($arg:tt)*) => {
        $crate::cio::out_str(&format!($($arg)*))
    };
}

/// `fprintf(stderr, ...)`
#[macro_export]
macro_rules! ceprintf {
    ($($arg:tt)*) => {
        $crate::cio::err_str(&format!($($arg)*))
    };
}

/// Mirrors `assert()` from `<assert.h>`: report to stderr and `abort()`.
#[macro_export]
macro_rules! cassert {
    ($cond:expr) => {
        if !$cond {
            $crate::cio::err_str(&format!(
                "driver: {}:{}: Assertion `{}' failed.\n",
                file!(),
                line!(),
                stringify!($cond)
            ));
            $crate::cio::flush_all();
            unsafe { libc::abort() }
        }
    };
}

/// A NUL-terminated string literal as `const char *`.
#[macro_export]
macro_rules! c_lit {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const core::ffi::c_char
    };
}
