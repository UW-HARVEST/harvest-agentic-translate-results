#![allow(non_snake_case)]

use std::ffi::{c_char, c_int};
use std::ptr;

#[cfg(target_os = "windows")]
const PATH_SEPARATOR: c_char = b'\\' as c_char;
#[cfg(not(target_os = "windows"))]
const PATH_SEPARATOR: c_char = b'/' as c_char;

unsafe fn extractFilename(path: *const c_char, separator: c_char) -> *const c_char {
    let search = unsafe { libc::strrchr(path, separator as c_int) };
    if search.is_null() {
        return path;
    }
    unsafe { search.add(1) }
}

#[cfg(any(target_os = "linux", target_os = "android", target_env = "musl"))]
unsafe fn errno_location() -> *mut c_int {
    unsafe { libc::__errno_location() }
}

#[cfg(target_os = "macos")]
unsafe fn errno_location() -> *mut c_int {
    unsafe { libc::__error() }
}

#[cfg(target_os = "freebsd")]
unsafe fn errno_location() -> *mut c_int {
    unsafe { libc::__error() }
}

#[cfg(any(target_os = "windows", target_env = "msvc"))]
unsafe fn errno_location() -> *mut c_int {
    unsafe { libc::_errno() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    outDirName: *const c_char,
    suffixLen: usize,
) -> *mut c_char {
    let separator = PATH_SEPARATOR;

    #[cfg(target_os = "windows")]
    let filenameStart = {
        let filenameStart = unsafe { extractFilename(path, separator) };
        unsafe { extractFilename(filenameStart, b'/' as c_char) }
    };

    #[cfg(not(target_os = "windows"))]
    let filenameStart = unsafe { extractFilename(path, separator) };

    let out_dir_len = unsafe { libc::strlen(outDirName) };
    let filename_len = unsafe { libc::strlen(filenameStart) };
    let allocation_size = out_dir_len + 1 + filename_len + suffixLen + 1;
    let result = unsafe { libc::calloc(1, allocation_size) as *mut c_char };

    if result.is_null() {
        static PREFIX: &[u8] = b"zstd: FIO_createFilename_fromOutDir: \0";
        let errno_value = unsafe { *errno_location() };
        let error_message = unsafe { libc::strerror(errno_value) };
        let prefix_len = PREFIX.len() - 1;
        let error_len = unsafe { libc::strlen(error_message) };

        unsafe {
            libc::write(libc::STDERR_FILENO, PREFIX.as_ptr().cast(), prefix_len);
            libc::write(libc::STDERR_FILENO, error_message.cast(), error_len);
            libc::exit(30);
        }
    }

    unsafe {
        libc::memcpy(
            result.cast(),
            outDirName.cast(),
            out_dir_len,
        );
    }

    let last_out_dir_char = unsafe { ptr::read(outDirName.add(out_dir_len.wrapping_sub(1))) };
    if last_out_dir_char == separator {
        unsafe {
            libc::memcpy(
                result.add(out_dir_len).cast(),
                filenameStart.cast(),
                filename_len,
            );
        }
    } else {
        unsafe {
            libc::memcpy(
                result.add(out_dir_len).cast(),
                (&separator as *const c_char).cast(),
                1,
            );
            libc::memcpy(
                result.add(out_dir_len + 1).cast(),
                filenameStart.cast(),
                filename_len,
            );
        }
    }

    result
}
