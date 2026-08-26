use std::ffi::c_char;

extern "C" {
    fn malloc(size: usize) -> *mut core::ffi::c_void;
}

/// Replicates the C `custom_strdup` function: returns NULL when given NULL,
/// otherwise allocates `strlen(str) + 1` bytes via `malloc` and copies the
/// string (including the trailing NUL) into the new buffer. Returns NULL on
/// allocation failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn custom_strdup(str: *const c_char) -> *mut c_char {
    if str.is_null() {
        return core::ptr::null_mut();
    }

    // Compute strlen
    let mut len: usize = 0;
    while *str.add(len) != 0 {
        len += 1;
    }
    len += 1; // include the NUL terminator

    let newstr = malloc(len) as *mut c_char;
    if newstr.is_null() {
        return core::ptr::null_mut();
    }

    core::ptr::copy_nonoverlapping(str, newstr, len);
    newstr
}
