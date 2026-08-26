use std::ffi::c_char;
use std::ffi::c_int;

unsafe fn strlen(s: *const c_char) -> usize {
    let mut len: usize = 0;
    while unsafe { *s.add(len) } != 0 {
        len += 1;
    }
    len
}

unsafe fn strrchr(s: *const c_char, c: c_char) -> *const c_char {
    let mut last: *const c_char = std::ptr::null();
    let mut i: usize = 0;
    loop {
        let ch = unsafe { *s.add(i) };
        if ch == c {
            last = unsafe { s.add(i) };
        }
        if ch == 0 {
            break;
        }
        i += 1;
    }
    last
}

unsafe fn extract_filename(path: *const c_char, separator: c_char) -> *const c_char {
    let search = unsafe { strrchr(path, separator) };
    if search.is_null() {
        return path;
    }
    unsafe { search.add(1) }
}

#[cfg(any(windows, target_env = "msvc"))]
const SEPARATOR: c_char = b'\\' as c_char;

#[cfg(not(any(windows, target_env = "msvc")))]
const SEPARATOR: c_char = b'/' as c_char;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    out_dir_name: *const c_char,
    suffix_len: usize,
) -> *mut c_char {
    let separator: c_char = SEPARATOR;

    #[cfg(not(any(windows, target_env = "msvc")))]
    let filename_start = unsafe { extract_filename(path, separator) };

    #[cfg(any(windows, target_env = "msvc"))]
    let filename_start = {
        let fs = unsafe { extract_filename(path, separator) };
        unsafe { extract_filename(fs, b'/' as c_char) }
    };

    let out_dir_len = unsafe { strlen(out_dir_name) };
    let filename_len = unsafe { strlen(filename_start) };
    let total_size = out_dir_len + 1 + filename_len + suffix_len + 1;

    let result = unsafe { libc::calloc(1, total_size) } as *mut c_char;
    if result.is_null() {
        let errno_val = unsafe { *libc::__errno_location() };
        let err_msg = unsafe { libc::strerror(errno_val) };
        unsafe {
            libc::fprintf(
                stderr_ptr(),
                b"zstd: FIO_createFilename_fromOutDir: %s\0".as_ptr() as *const c_char,
                err_msg,
            );
        }
        unsafe { libc::exit(30) };
    }

    unsafe {
        libc::memcpy(
            result as *mut libc::c_void,
            out_dir_name as *const libc::c_void,
            out_dir_len,
        );
    }

    let last_char = unsafe { *out_dir_name.add(out_dir_len - 1) };
    if last_char == separator {
        unsafe {
            libc::memcpy(
                result.add(out_dir_len) as *mut libc::c_void,
                filename_start as *const libc::c_void,
                filename_len,
            );
        }
    } else {
        unsafe {
            libc::memcpy(
                result.add(out_dir_len) as *mut libc::c_void,
                &separator as *const c_char as *const libc::c_void,
                1,
            );
            libc::memcpy(
                result.add(out_dir_len + 1) as *mut libc::c_void,
                filename_start as *const libc::c_void,
                filename_len,
            );
        }
    }

    result
}

#[cfg(target_os = "linux")]
fn stderr_ptr() -> *mut libc::FILE {
    unsafe extern "C" {
        static mut stderr: *mut libc::FILE;
    }
    unsafe { stderr }
}

#[cfg(target_os = "macos")]
fn stderr_ptr() -> *mut libc::FILE {
    unsafe extern "C" {
        static mut __stderrp: *mut libc::FILE;
    }
    unsafe { __stderrp }
}

// Suppress unused warning
#[allow(dead_code)]
fn _unused(_: c_int) {}
