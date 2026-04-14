use libc::{calloc, exit, memcpy};
use std::ffi::{CStr, c_char};
use std::ptr;

fn extract_filename(path: *const c_char, separator: u8) -> *const c_char {
    if path.is_null() {
        return path;
    }
    let bytes = unsafe { CStr::from_ptr(path) }.to_bytes();
    if let Some(pos) = bytes.iter().rposition(|&b| b == separator) {
        unsafe { path.add(pos + 1) }
    } else {
        path
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    outDirName: *const c_char,
    suffixLen: usize,
) -> *mut c_char {
    let separator = if cfg!(any(windows, target_env = "msvc")) {
        b'\\'
    } else {
        b'/'
    };

    let mut filename_start = extract_filename(path, separator);
    if cfg!(any(windows, target_env = "msvc")) {
        filename_start = extract_filename(filename_start, b'/');
    }

    let out_dir = unsafe { CStr::from_ptr(outDirName) };
    let filename = unsafe { CStr::from_ptr(filename_start) };
    let out_dir_bytes = out_dir.to_bytes();
    let filename_bytes = filename.to_bytes();

    let total_len = out_dir_bytes.len() + 1 + filename_bytes.len() + suffixLen + 1;
    let result = unsafe { calloc(1, total_len) as *mut c_char };
    if result.is_null() {
        unsafe { exit(30) };
    }

    unsafe {
        memcpy(
            result.cast(),
            out_dir_bytes.as_ptr().cast(),
            out_dir_bytes.len(),
        );

        if !out_dir_bytes.is_empty() && out_dir_bytes[out_dir_bytes.len() - 1] == separator {
            memcpy(
                result.add(out_dir_bytes.len()).cast(),
                filename_bytes.as_ptr().cast(),
                filename_bytes.len(),
            );
        } else {
            let sep = separator as c_char;
            ptr::copy_nonoverlapping(&sep, result.add(out_dir_bytes.len()), 1);
            memcpy(
                result.add(out_dir_bytes.len() + 1).cast(),
                filename_bytes.as_ptr().cast(),
                filename_bytes.len(),
            );
        }
    }

    result
}
