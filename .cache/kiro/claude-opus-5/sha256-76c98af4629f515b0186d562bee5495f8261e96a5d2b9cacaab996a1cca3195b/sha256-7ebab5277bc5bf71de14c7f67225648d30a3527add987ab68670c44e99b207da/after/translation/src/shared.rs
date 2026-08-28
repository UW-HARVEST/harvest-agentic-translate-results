//! Translation of `c_src/include/shared.h`.
//!
//! The helpers in the original header are defined (not merely declared) in the
//! header itself, so they end up as ordinary external symbols of the shared
//! library. They are reproduced here with the same linkage and behaviour.

use core::ffi::{c_char, c_int, c_void};

extern "C" {
    #[link_name = "stderr"]
    static mut C_STDERR: *mut libc::FILE;

    #[link_name = "fprintf"]
    fn c_fprintf(stream: *mut libc::FILE, format: *const c_char, ...) -> c_int;
}

/// `fprintf(stderr, "%s", msg)` - no trailing newline, exactly like the C code.
pub(crate) fn stderr_str(msg: &core::ffi::CStr) {
    unsafe {
        c_fprintf(C_STDERR, c"%s".as_ptr(), msg.as_ptr());
    }
}

/// `fprintf(stderr, "%s\n", buffer)` as used by `merror`.
pub(crate) fn stderr_line(buf: *const c_char) {
    unsafe {
        c_fprintf(C_STDERR, c"%s\n".as_ptr(), buf);
    }
}

/// `#define os_free(x) if(x){free(x);x=NULL;};`
pub(crate) unsafe fn os_free<T>(slot: &mut *mut T) {
    if !slot.is_null() {
        libc::free(*slot as *mut c_void);
        *slot = core::ptr::null_mut();
    }
}

/// `#define os_clearnl(x,p) if((p = strrchr(x, '\n')))*p = '\0';`
pub(crate) unsafe fn os_clearnl(x: *mut c_char) {
    let p = libc::strrchr(x, '\n' as c_int);
    if !p.is_null() {
        *p = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_calloc(num: usize, size: usize) -> *mut c_void {
    let out = libc::calloc(num, size);
    if out.is_null() {
        stderr_str(c"Memory allocation failed in os_calloc");
        libc::exit(libc::EXIT_FAILURE);
    }
    out
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
    let out = libc::realloc(ptr, new_size);
    if out.is_null() {
        stderr_str(c"Memory allocation failed in os_realloc");
        libc::exit(libc::EXIT_FAILURE);
    }
    out
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn os_strdup(str_: *const c_char) -> *mut c_char {
    if str_.is_null() {
        stderr_str(c"NULL string passed to os_strdup");
        libc::exit(libc::EXIT_FAILURE);
    }
    let dup = libc::strdup(str_);
    if dup.is_null() {
        stderr_str(c"Memory allocation failed in os_strdup");
        libc::exit(libc::EXIT_FAILURE);
    }
    dup
}
