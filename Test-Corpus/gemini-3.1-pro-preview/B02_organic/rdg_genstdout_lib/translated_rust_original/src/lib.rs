use std::ffi::CStr;
use std::os::raw::c_char;
use std::process;

fn extract_filename(path: &[u8], separator: u8) -> &[u8] {
    if let Some(pos) = path.iter().rposition(|&c| c == separator) {
        &path[pos + 1..]
    } else {
        path
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    out_dir_name: *const c_char,
    suffix_len: usize,
) -> *mut c_char {
    let path_bytes = unsafe { CStr::from_ptr(path) }.to_bytes();
    let out_dir_bytes = unsafe { CStr::from_ptr(out_dir_name) }.to_bytes();

    let separator = if cfg!(windows) { b'\\' } else { b'/' };

    let mut filename_start = extract_filename(path_bytes, separator);
    if cfg!(windows) {
        filename_start = extract_filename(filename_start, b'/');
    }

    let total_len = out_dir_bytes.len() + 1 + filename_start.len() + suffix_len + 1;

    let result = unsafe { libc::calloc(1, total_len) as *mut c_char };
    if result.is_null() {
        eprintln!(
            "zstd: FIO_createFilename_fromOutDir: {}",
            std::io::Error::last_os_error()
        );
        process::exit(30);
    }

    unsafe {
        let dst = result as *mut u8;
        std::ptr::copy_nonoverlapping(out_dir_bytes.as_ptr(), dst, out_dir_bytes.len());

        if !out_dir_bytes.is_empty() && out_dir_bytes[out_dir_bytes.len() - 1] == separator {
            std::ptr::copy_nonoverlapping(
                filename_start.as_ptr(),
                dst.add(out_dir_bytes.len()),
                filename_start.len(),
            );
        } else {
            *dst.add(out_dir_bytes.len()) = separator;
            std::ptr::copy_nonoverlapping(
                filename_start.as_ptr(),
                dst.add(out_dir_bytes.len() + 1),
                filename_start.len(),
            );
        }
    }

    result
}
