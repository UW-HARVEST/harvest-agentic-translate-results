//! Translation of `include/shared.h`.
//!
//! The original C header defines the helper allocation/duplication
//! functions as ordinary functions (not `static` / not `inline`),
//! so they are emitted into every translation unit that includes the
//! header. The build only links one shared object, so the symbols
//! appear once in `libdriver.so`.

use std::ffi::c_void;
use std::os::raw::c_char;

pub const OS_MAXSTR: usize = 1024;

extern "C" {
    fn calloc(num: libc::size_t, size: libc::size_t) -> *mut c_void;
    fn realloc(ptr: *mut c_void, new_size: libc::size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strdup(s: *const c_char) -> *mut c_char;
    fn exit(status: libc::c_int) -> !;
    fn fputs(s: *const c_char, stream: *mut libc::FILE) -> libc::c_int;
    fn fdopen(fd: libc::c_int, mode: *const c_char) -> *mut libc::FILE;
}

/// Print a message to stderr, mimicking `fprintf(stderr, "%s", msg)`.
fn write_stderr(msg: &str) {
    // Build a NUL-terminated C string and use fputs to emit it on the
    // exact same FILE* the C version would use (stderr).
    let mut buf: Vec<u8> = Vec::with_capacity(msg.len() + 1);
    buf.extend_from_slice(msg.as_bytes());
    buf.push(0);
    unsafe {
        // 2 == STDERR_FILENO. Reopen as a FILE* to match `fprintf(stderr, ...)`
        // semantics. Using a fresh FILE* avoids relying on the libc::stderr
        // global which is not always reachable on every platform.
        let mode = b"w\0".as_ptr() as *const c_char;
        let fp = fdopen(2, mode);
        if !fp.is_null() {
            fputs(buf.as_ptr() as *const c_char, fp);
            // Don't fclose - we don't want to close stderr's underlying fd.
        }
    }
}

/// Equivalent of `os_calloc` from shared.h.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_calloc(num: libc::size_t, size: libc::size_t) -> *mut c_void {
    let out = calloc(num, size);
    if out.is_null() {
        write_stderr("Memory allocation failed in os_calloc");
        exit(1); // EXIT_FAILURE
    }
    out
}

/// Equivalent of `os_realloc` from shared.h.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_realloc(ptr: *mut c_void, new_size: libc::size_t) -> *mut c_void {
    let out = realloc(ptr, new_size);
    if out.is_null() {
        write_stderr("Memory allocation failed in os_realloc");
        exit(1);
    }
    out
}

/// Equivalent of `os_strdup` from shared.h.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_strdup(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        write_stderr("NULL string passed to os_strdup");
        exit(1);
    }
    let dup = strdup(s);
    if dup.is_null() {
        write_stderr("Memory allocation failed in os_strdup");
        exit(1);
    }
    dup
}

/// Equivalent of the `os_free(x)` macro: free if non-null, callers must
/// also clear their pointer back to NULL afterwards (the macro does this
/// in C; we replicate by accepting a `*mut *mut c_void`-style pattern in
/// the call sites).
#[inline]
pub unsafe fn os_free<T>(p: &mut *mut T) {
    if !p.is_null() {
        free(*p as *mut c_void);
        *p = std::ptr::null_mut();
    }
}
