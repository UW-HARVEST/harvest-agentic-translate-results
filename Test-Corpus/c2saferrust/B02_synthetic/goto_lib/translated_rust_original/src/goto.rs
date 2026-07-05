
use std::ffi::CString;

use std::ffi::CStr;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fclose(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn fopen(
        __filename: *const ::core::ffi::c_char,
        __modes: *const ::core::ffi::c_char,
    ) -> *mut FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn fgets(
        __s: *mut ::core::ffi::c_char,
        __n: ::core::ffi::c_int,
        __stream: *mut FILE,
    ) -> *mut ::core::ffi::c_char;
    fn ferror(__stream: *mut FILE) -> ::core::ffi::c_int;
}
pub type size_t = usize;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: ::core::ffi::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut ::core::ffi::c_void,
    pub __pad2: *mut ::core::ffi::c_void,
    pub __pad3: *mut ::core::ffi::c_void,
    pub __pad4: *mut ::core::ffi::c_void,
    pub __pad5: size_t,
    pub _mode: ::core::ffi::c_int,
    pub _unused2: [::core::ffi::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: ::core::ffi::c_int,
}
pub type FILE = _IO_FILE;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub fn forward_goto_example(x: i32) -> i32 {
    if x < 0 {
        eprintln!("Error: negative input");
        -1
    } else {
        println!("Processing: {}", x);
        x * 2
    }
}

#[no_mangle]
pub fn open_with_cleanup(filename: *const ::core::ffi::c_char) -> *mut FILE {
    let filename_str = unsafe { CStr::from_ptr(filename) };
    let filename_owned = filename_str.to_string_lossy().into_owned();

    match File::open(&filename_owned) {
        Ok(file) => {
            let reader_file = match file.try_clone() {
                Ok(f) => f,
                Err(_) => {
                    eprintln!("Error: opening or processing file {}", filename_owned);
                    return ::core::ptr::null_mut();
                }
            };

            let mut reader = BufReader::new(reader_file);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => print!("{}", line),
                    Err(_) => {
                        eprintln!("Error: opening or processing file {}", filename_owned);
                        return ::core::ptr::null_mut();
                    }
                }
            }

            if std::io::stdout().flush().is_err() {
                eprintln!("Error: opening or processing file {}", filename_owned);
                return ::core::ptr::null_mut();
            }

            match File::open(&filename_owned) {
                Ok(_) => unsafe { fopen(filename, b"r\0" as *const u8 as *const ::core::ffi::c_char) },
                Err(_) => {
                    eprintln!("Error: opening or processing file {}", filename_owned);
                    ::core::ptr::null_mut()
                }
            }
        }
        Err(_) => {
            eprintln!("Error: opening or processing file {}", filename_owned);
            ::core::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub fn driver(num: i32, filename: &str) -> i32 {
    let res = forward_goto_example(num);
    if res == -1 {
        return -1;
    }

    println!("Goto output: {}", res);

    let c_filename = match CString::new(filename) {
        Ok(s) => s,
        Err(_) => return -2,
    };

    let out = open_with_cleanup(c_filename.as_ptr());
    if out.is_null() {
        return -2;
    }

    unsafe {
        fclose(out);
    }

    0
}

