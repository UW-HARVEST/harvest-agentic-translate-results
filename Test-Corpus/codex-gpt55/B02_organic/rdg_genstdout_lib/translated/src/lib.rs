use std::ffi::{c_char, c_int};

unsafe extern "C" {
    static mut stderr: *mut libc::FILE;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn extractFilename(path: *const c_char, separator: c_char) -> *const c_char {
    let search = unsafe { libc::strrchr(path, separator as c_int) };
    if search.is_null() {
        path
    } else {
        unsafe { search.add(1) }
    }
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    outDirName: *const c_char,
    suffixLen: usize,
) -> *mut c_char {
    let separator = path_separator();

    let filename_start = unsafe { extractFilename(path, separator) };
    #[cfg(windows)]
    let filename_start = unsafe { extractFilename(filename_start, b'/' as c_char) };

    let out_dir_len = unsafe { libc::strlen(outDirName) };
    let filename_len = unsafe { libc::strlen(filename_start) };
    let allocation_size = out_dir_len
        .wrapping_add(1)
        .wrapping_add(filename_len)
        .wrapping_add(suffixLen)
        .wrapping_add(1);

    let result = unsafe { libc::calloc(1, allocation_size) as *mut c_char };
    if result.is_null() {
        unsafe {
            libc::fprintf(
                stderr,
                c"zstd: FIO_createFilename_fromOutDir: %s".as_ptr(),
                libc::strerror(errno_value()),
            );
            libc::exit(30);
        }
    }

    unsafe {
        libc::memcpy(
            result.cast(),
            outDirName.cast(),
            libc::strlen(outDirName),
        );
    }

    let out_dir_ends_with_separator = unsafe {
        *outDirName.wrapping_add(libc::strlen(outDirName).wrapping_sub(1)) == separator
    };

    if out_dir_ends_with_separator {
        unsafe {
            libc::memcpy(
                result.wrapping_add(libc::strlen(outDirName)).cast(),
                filename_start.cast(),
                libc::strlen(filename_start),
            );
        }
    } else {
        unsafe {
            libc::memcpy(
                result.wrapping_add(libc::strlen(outDirName)).cast(),
                (&separator as *const c_char).cast(),
                1,
            );
            libc::memcpy(
                result
                    .wrapping_add(libc::strlen(outDirName))
                    .wrapping_add(1)
                    .cast(),
                filename_start.cast(),
                libc::strlen(filename_start),
            );
        }
    }

    result
}

#[cfg(windows)]
fn path_separator() -> c_char {
    b'\\' as c_char
}

#[cfg(not(windows))]
fn path_separator() -> c_char {
    b'/' as c_char
}

fn errno_value() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}
