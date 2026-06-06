use std::ffi::c_char;
use std::os::raw::c_int;

/// Equivalent of C's strlen for a NUL-terminated string.
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut len: usize = 0;
    while *s.add(len) != 0 {
        len += 1;
    }
    len
}

/// Equivalent of C's strrchr for a single byte. Returns pointer to last
/// occurrence of `ch` in `s`, or null if not found.
unsafe fn c_strrchr(s: *const c_char, ch: c_char) -> *const c_char {
    let mut last: *const c_char = std::ptr::null();
    let mut p = s;
    loop {
        let c = *p;
        if c == ch {
            last = p;
        }
        if c == 0 {
            break;
        }
        p = p.add(1);
    }
    last
}

/// Mirror of C `extractFilename`: returns pointer just past the last
/// occurrence of `separator` in `path`, or `path` itself if not found.
unsafe fn extract_filename(path: *const c_char, separator: c_char) -> *const c_char {
    let search = c_strrchr(path, separator);
    if search.is_null() {
        return path;
    }
    search.add(1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    out_dir_name: *const c_char,
    suffix_len: usize,
) -> *mut c_char {
    let separator: c_char = b'/' as c_char;

    let filename_start = extract_filename(path, separator);

    let out_dir_len = c_strlen(out_dir_name);
    let filename_len = c_strlen(filename_start);

    let total = out_dir_len + 1 + filename_len + suffix_len + 1;
    let result = libc::calloc(1, total) as *mut c_char;
    if result.is_null() {
        let errno_val = *libc::__errno_location();
        let err_msg = libc::strerror(errno_val);
        // Match the exact format string used in the C source: no trailing newline.
        libc::fprintf(
            libc_stderr(),
            b"zstd: FIO_createFilename_fromOutDir: %s\0".as_ptr() as *const c_char,
            err_msg,
        );
        libc::exit(30 as c_int);
    }

    // memcpy outDirName -> result
    std::ptr::copy_nonoverlapping(
        out_dir_name as *const u8,
        result as *mut u8,
        out_dir_len,
    );

    if out_dir_len > 0 && *out_dir_name.add(out_dir_len - 1) == separator {
        std::ptr::copy_nonoverlapping(
            filename_start as *const u8,
            (result.add(out_dir_len)) as *mut u8,
            filename_len,
        );
    } else {
        // Copy single separator byte
        *result.add(out_dir_len) = separator;
        std::ptr::copy_nonoverlapping(
            filename_start as *const u8,
            (result.add(out_dir_len + 1)) as *mut u8,
            filename_len,
        );
    }

    result
}

/// Helper that returns the libc `stderr` FILE*. Using an extern declaration
/// avoids depending on a specific libc version that exposes `stderr` directly.
fn libc_stderr() -> *mut libc::FILE {
    extern "C" {
        static mut stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}
