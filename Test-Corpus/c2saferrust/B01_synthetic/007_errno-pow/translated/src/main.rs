#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]

use std::env;

#[allow(unused_imports)]
use ::driver;
extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn strtod(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_double;
    fn pow(__x: ::core::ffi::c_double, __y: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn __errno_location() -> *mut ::core::ffi::c_int;
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
fn main_0(argc: ::core::ffi::c_int, argv: *mut *mut ::core::ffi::c_char) -> ::core::ffi::c_int {
    let args: Vec<String> = env::args().collect();

    if argc != 3 {
        let program_name = args.get(0).map(String::as_str).unwrap_or("program");
        eprintln!("Usage: {} base exponent", program_name);
        return 1;
    }

    let base_str = match args.get(1) {
        Some(s) => s,
        None => {
            let program_name = args.get(0).map(String::as_str).unwrap_or("program");
            eprintln!("Usage: {} base exponent", program_name);
            return 1;
        }
    };

    let exponent_str = match args.get(2) {
        Some(s) => s,
        None => {
            let program_name = args.get(0).map(String::as_str).unwrap_or("program");
            eprintln!("Usage: {} base exponent", program_name);
            return 1;
        }
    };

    let base: f64 = match base_str.parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Invalid numeric input for base: '{}'", base_str);
            return 1;
        }
    };

    let exponent: f64 = match exponent_str.parse() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("Invalid numeric input for exponent: '{}'", exponent_str);
            return 1;
        }
    };

    let result = base.powf(exponent);

    if result.is_nan() {
        eprintln!(
            "Domain error: pow({:.2}, {:.2}) is undefined in the real number domain.",
            base, exponent
        );
        return 1;
    } else if !result.is_finite() || (result == 0.0 && base != 0.0) {
        eprintln!(
            "Range error: pow({:.2}, {:.2}) caused overflow or underflow.",
            base, exponent
        );
        return 1;
    }

    println!("Result: {:.2}", result);
    0
}

pub const EDOM: ::core::ffi::c_int = 33 as ::core::ffi::c_int;
pub const ERANGE: ::core::ffi::c_int = 34 as ::core::ffi::c_int;
pub fn main() {
    let mut args = std::env::args_os()
        .map(|arg| {
            use std::os::unix::ffi::OsStrExt;
            let mut bytes = arg.as_os_str().as_bytes().to_vec();
            bytes.push(0);
            bytes
        })
        .collect::<Vec<_>>();

    let mut argv = args
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut ::core::ffi::c_char)
        .collect::<Vec<_>>();
    argv.push(::core::ptr::null_mut());

    let argc = (argv.len() - 1) as ::core::ffi::c_int;
    let exit_code = unsafe { main_0(argc, argv.as_mut_ptr()) };
    std::process::exit(exit_code as i32);
}

