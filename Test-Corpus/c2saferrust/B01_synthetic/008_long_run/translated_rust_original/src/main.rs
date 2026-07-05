#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]


use std::ffi::CStr;

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
    fn strtoul(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_ulong;
    fn rand() -> ::core::ffi::c_int;
    fn srand(__seed: ::core::ffi::c_uint);
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
pub const ARRAY_SIZE: ::core::ffi::c_int = 256 as ::core::ffi::c_int * 1024 as ::core::ffi::c_int;
pub const ITERATIONS: ::core::ffi::c_int = 2000 as ::core::ffi::c_int;
#[no_mangle]
pub static mut array: [::core::ffi::c_int; 262144] = [0; 262144];
#[no_mangle]
pub fn perform_expensive_operations() {
    unsafe {
        let len = ARRAY_SIZE as usize;
        let slice = &mut array[..len];
        for value in slice.iter_mut() {
            let mut x = *value;
            for _ in 0..100 {
                x = x * 3 + 7;
                x ^= x >> 3;
                x -= x << 1;
                x = x / 2 + x % 7;
            }
            *value = x;
        }
    }
}

fn main_0(argc: i32, argv: &[*mut ::core::ffi::c_char]) -> i32 {
    if argc != 2 {
        let program_name = argv
            .get(0)
            .map(|&p| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
            .unwrap_or_else(|| String::from("<program>"));
        eprintln!("Usage: {} <seed>", program_name);
        return 1;
    }

    let seed_str = match argv.get(1) {
        Some(&p) => unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned(),
        None => {
            eprintln!("Invalid seed: ''");
            return 1;
        }
    };

    let temp_seed = match seed_str.parse::<u64>() {
        Ok(v) if v <= u32::MAX as u64 => v,
        _ => {
            eprintln!("Invalid seed: '{}'", seed_str);
            return 1;
        }
    };

    let seed = temp_seed as ::core::ffi::c_uint;

    unsafe {
        srand(seed);

        let mut i: usize = 0;
        while i < ARRAY_SIZE as usize {
            array[i] = rand();
            i += 1;
        }

        let mut i_0: ::core::ffi::c_int = 0;
        while i_0 < ITERATIONS {
            perform_expensive_operations();
            i_0 += 1;
        }

        let mut xor_result: ::core::ffi::c_int = 0;
        let mut i_1: usize = 0;
        while i_1 < ARRAY_SIZE as usize {
            xor_result ^= array[i_1];
            i_1 += 1;
        }

        println!("{}", xor_result);
    }

    0
}

pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
pub const UINT_MAX: ::core::ffi::c_uint = (__INT_MAX__ as ::core::ffi::c_uint)
    .wrapping_mul(2 as ::core::ffi::c_uint)
    .wrapping_add(1 as ::core::ffi::c_uint);
pub fn main() {
    let mut args: Vec<Vec<u8>> = std::env::args_os()
        .map(|arg| {
            let mut bytes = arg.to_string_lossy().into_owned().into_bytes();
            if bytes.contains(&0) {
                panic!("Failed to convert argument into CString.");
            }
            bytes.push(0);
            bytes
        })
        .collect();

    let mut argv: Vec<*mut ::core::ffi::c_char> = args
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut ::core::ffi::c_char)
        .collect();

    let argc = argv.len() as i32;
    let code = main_0(argc, &argv);
    std::process::exit(code);
}

