use std::ffi::{c_char, c_void, CStr, CString};
use std::os::raw::c_int;
use std::ptr;

#[cfg(windows)]
const SEPARATOR: char = '\\';
#[cfg(not(windows))]
const SEPARATOR: char = '/';

fn extract_filename(path: &CStr, separator: char) -> &CStr {
    let bytes = path.to_bytes();
    let sep = separator as u8;
    
    match bytes.riter().position(|&b| b == sep) {
        Some(pos) => {
            let start = bytes.len() - pos;
            unsafe { CStr::from_ptr(path.as_ptr().add(start) as *const c_char) }
        }
        None => path,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn FIO_createFilename_fromOutDir(
    path: *const c_char,
    outDirName: *const c_char,
    suffixLen: usize,
) -> *mut c_char {
    let path_cstr = unsafe { CStr::from_ptr(path) };
    let out_dir_cstr = unsafe { CStr::from_ptr(outDirName) };
    
    let mut filename_start = extract_filename(path_cstr, SEPARATOR);
    
    #[cfg(windows)]
    {
        filename_start = extract_filename(filename_start, '/');
    }
    
    let out_dir_len = out_dir_cstr.to_bytes().len();
    let filename_len = filename_start.to_bytes().len();
    
    let needs_sep = out_dir_len > 0 && out_dir_cstr.to_bytes().last() != Some(&(SEPARATOR as u8));
    let sep_len = if needs_sep { 1 } else { 0 };
    
    let total_len = out_dir_len + sep_len + filename_len + suffixLen + 1;
    
    let result = unsafe { libc::calloc(1, total_len) as *mut c_char };
    
    if result.is_null() {
        eprintln!("zstd: FIO_createFilename_fromOutDir: {}", std::io::Error::last_os_error());
        std::process::exit(30);
    }
    
    unsafe {
        ptr::copy_nonoverlapping(
            out_dir_cstr.as_ptr() as *const u8,
            result as *mut u8,
            out_dir_len,
        );
        
        if needs_sep {
            *(result.add(out_dir_len) as *mut u8) = SEPARATOR as u8;
        }
        
        let filename_offset = out_dir_len + sep_len;
        ptr::copy_nonoverlapping(
            filename_start.as_ptr() as *const u8,
            result.add(filename_offset) as *mut u8,
            filename_len,
        );
    }
    
    result
}
