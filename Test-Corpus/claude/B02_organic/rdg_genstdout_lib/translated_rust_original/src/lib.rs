use std::ffi::c_char;
use std::io::Write;

/// Returns a pointer just past the last occurrence of `separator` in `path`.
/// If `separator` is not found, returns `path`.
///
/// # Safety
/// `path` must be a valid pointer to a NUL-terminated C string.
unsafe fn extract_filename(path: *const c_char, separator: c_char) -> *const c_char {
    let search = libc::strrchr(path, separator as i32);
    if search.is_null() {
        return path;
    }
    search.add(1)
}

/// FIO_createFilename_fromOutDir() :
/// Takes a source file name and specified output directory, and
/// allocates memory for and returns a pointer to final path.
/// This function never returns an error (it may abort() in case of pb)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    out_dir_name: *const c_char,
    suffix_len: libc::size_t,
) -> *mut c_char {
    let separator: c_char = b'/' as c_char;

    let filename_start = extract_filename(path, separator);

    let out_dir_len = libc::strlen(out_dir_name);
    let filename_len = libc::strlen(filename_start);

    let total_len = out_dir_len + 1 + filename_len + suffix_len + 1;
    let result = libc::calloc(1, total_len) as *mut c_char;
    if result.is_null() {
        // Match: fprintf(stderr, "zstd: FIO_createFilename_fromOutDir: %s", strerror(errno));
        let errno_val = *libc::__errno_location();
        let err_str_ptr = libc::strerror(errno_val);
        let err_bytes = if err_str_ptr.is_null() {
            &[][..]
        } else {
            let err_len = libc::strlen(err_str_ptr);
            std::slice::from_raw_parts(err_str_ptr as *const u8, err_len)
        };
        let stderr = std::io::stderr();
        let mut handle = stderr.lock();
        let _ = handle.write_all(b"zstd: FIO_createFilename_fromOutDir: ");
        let _ = handle.write_all(err_bytes);
        libc::exit(30);
    }

    libc::memcpy(
        result as *mut libc::c_void,
        out_dir_name as *const libc::c_void,
        out_dir_len,
    );

    let last_char = *out_dir_name.add(out_dir_len - 1);
    if last_char == separator {
        libc::memcpy(
            result.add(out_dir_len) as *mut libc::c_void,
            filename_start as *const libc::c_void,
            filename_len,
        );
    } else {
        let sep_ptr: *const c_char = &separator;
        libc::memcpy(
            result.add(out_dir_len) as *mut libc::c_void,
            sep_ptr as *const libc::c_void,
            1,
        );
        libc::memcpy(
            result.add(out_dir_len + 1) as *mut libc::c_void,
            filename_start as *const libc::c_void,
            filename_len,
        );
    }

    result
}
