use libc::{c_char, c_int, size_t};
use std::ffi::CStr;
use std::process;
use std::ptr;

extern "C" {
    fn calloc(nmemb: size_t, size: size_t) -> *mut libc::c_void;
    fn strerror(errnum: c_int) -> *mut c_char;
    fn __errno_location() -> *mut c_int;
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn errno() -> c_int {
    unsafe { *__errno_location() }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn errno() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// Extracts the filename portion of a path by finding the last occurrence of `separator`.
/// Returns a pointer to the character following the last separator, or `path` itself if
/// the separator is not found.
///
/// # Safety
/// `path` must be a valid pointer to a NUL-terminated C string.
unsafe fn extract_filename(path: *const c_char, separator: c_char) -> *const c_char {
    let search = libc::strrchr(path, separator as c_int);
    if search.is_null() {
        return path;
    }
    search.add(1)
}

/// FIO_createFilename_fromOutDir() :
/// Takes a source file name and specified output directory, and
/// allocates memory for and returns a pointer to final path.
/// This function never returns an error (it may abort() in case of pb)
///
/// # Safety
/// `path` and `out_dir_name` must be valid pointers to NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    out_dir_name: *const c_char,
    suffix_len: size_t,
) -> *mut c_char {
    let separator: c_char;

    #[cfg(any(target_os = "windows"))]
    {
        separator = b'\\' as c_char;
    }
    #[cfg(not(any(target_os = "windows")))]
    {
        separator = b'/' as c_char;
    }

    #[cfg(not(target_os = "windows"))]
    let filename_start = extract_filename(path, separator);

    #[cfg(target_os = "windows")]
    let filename_start = {
        let fs = extract_filename(path, separator);
        extract_filename(fs, b'/' as c_char)
    };

    let out_dir_len = libc::strlen(out_dir_name);
    let filename_len = libc::strlen(filename_start);

    let total = out_dir_len + 1 + filename_len + suffix_len + 1;
    let result = calloc(1, total) as *mut c_char;
    if result.is_null() {
        let msg = strerror(errno());
        let msg_str = if msg.is_null() {
            String::from("unknown error")
        } else {
            CStr::from_ptr(msg).to_string_lossy().into_owned()
        };
        eprint!("zstd: FIO_createFilename_fromOutDir: {}", msg_str);
        process::exit(30);
    }

    ptr::copy_nonoverlapping(out_dir_name as *const u8, result as *mut u8, out_dir_len);

    if out_dir_len > 0 && *out_dir_name.add(out_dir_len - 1) == separator {
        ptr::copy_nonoverlapping(
            filename_start as *const u8,
            result.add(out_dir_len) as *mut u8,
            filename_len,
        );
    } else {
        let sep_byte = separator as u8;
        ptr::copy_nonoverlapping(
            &sep_byte as *const u8,
            result.add(out_dir_len) as *mut u8,
            1,
        );
        ptr::copy_nonoverlapping(
            filename_start as *const u8,
            result.add(out_dir_len + 1) as *mut u8,
            filename_len,
        );
    }

    result
}
