#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]






use std::io::BufRead;

use std::io;

#[allow(unused_imports)]
use ::driver;
extern "C" {
    fn fabs(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    static mut stdin: *mut _IO_FILE;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn fgets(
        __s: *mut ::core::ffi::c_char,
        __n: ::core::ffi::c_int,
        __stream: *mut FILE,
    ) -> *mut ::core::ffi::c_char;
    fn atof(__nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_double;
}
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
pub type size_t = usize;
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
pub fn printLine(line: &str) {
    println!("{}", line);
}

#[no_mangle]
pub fn printIntLine(int_number: i32) {
    println!("{}", int_number);
}

pub const CHAR_ARRAY_SIZE: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
#[no_mangle]
pub fn bad() {
    let mut data: f32 = 0.0;
    let mut input_buffer = String::new();

    let input = io::stdin();
    let mut handle = input.lock();

    if handle.read_line(&mut input_buffer).is_ok() {
        if let Ok(parsed) = input_buffer.trim().parse::<f32>() {
            data = parsed;
        }
    } else {
        printLine("fgets() failed.");
    }

    let result: ::core::ffi::c_int = (100.0f64 / data as f64) as ::core::ffi::c_int;
    printIntLine(result);
}

fn goodG2B() {
    let data: f32 = 2.0;
    let result: i32 = (100.0f64 / data as f64) as i32;
    printIntLine(result);
}

fn goodB2G() {
    let mut data: f32 = 0.0;
    let mut input = String::new();

    if io::stdin().read_line(&mut input).is_ok() {
        if let Ok(parsed) = input.trim().parse::<f32>() {
            data = parsed;
        }
    } else {
        printLine("fgets() failed.");
    }

    if (data as f64).abs() > 0.000001f64 {
        let result = (100.0f64 / data as f64) as i32;
        printIntLine(result);
    } else {
        printLine("This would result in a divide by zero");
    }
}

#[no_mangle]
pub fn good() {
    goodG2B();
    goodB2G();
}

fn main_0() -> i32 {
    printLine("Calling good()...");
    good();
    printLine("Finished good()");
    printLine("Calling bad()...");
    bad();
    printLine("Finished bad()");
    0
}

pub fn main() {
    std::process::exit(main_0())
}

