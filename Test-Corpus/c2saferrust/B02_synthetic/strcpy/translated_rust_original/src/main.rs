#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]
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
    fn scanf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn process_strings(
        input: *mut ::core::ffi::c_char,
        input_len: size_t,
        reference: *const ::core::ffi::c_char,
        ref_len: size_t,
        operation: ::core::ffi::c_int,
        flags: uint32_t,
    ) -> ::core::ffi::c_int;
}
pub type size_t = usize;
pub type __uint32_t = u32;
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
pub type uint32_t = __uint32_t;
pub const MAX_BUFFER_SIZE: ::core::ffi::c_int = 1024 as ::core::ffi::c_int;
unsafe fn main_0() -> ::core::ffi::c_int {
    let mut operation: ::core::ffi::c_int = 0;
    let mut flags: uint32_t = 0;
    let mut input_len: size_t = 0;
    let mut ref_len: size_t = 0;
    let mut input_buffer: [::core::ffi::c_char; 1024] = [0; 1024];
    let mut ref_buffer: [::core::ffi::c_char; 1024] = [0; 1024];
    if scanf(
        b"%d\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut operation,
    ) != 1 as ::core::ffi::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error reading operation\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return 1 as ::core::ffi::c_int;
    }
    if scanf(
        b"%u\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut flags,
    ) != 1 as ::core::ffi::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error reading flags\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return 1 as ::core::ffi::c_int;
    }
    if scanf(
        b"%zu\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut input_len,
    ) != 1 as ::core::ffi::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error reading input length\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return 1 as ::core::ffi::c_int;
    }
    if input_len > MAX_BUFFER_SIZE as size_t {
        fprintf(
            stderr as *mut FILE,
            b"Error: input length %zu exceeds maximum %d\n\0" as *const u8
                as *const ::core::ffi::c_char,
            input_len,
            MAX_BUFFER_SIZE,
        );
        return 1 as ::core::ffi::c_int;
    }
    let mut i: size_t = 0 as size_t;
    while i < input_len {
        let mut byte: ::core::ffi::c_uint = 0;
        if scanf(
            b"%u\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut byte,
        ) != 1 as ::core::ffi::c_int
        {
            fprintf(
                stderr as *mut FILE,
                b"Error reading input byte %zu\n\0" as *const u8 as *const ::core::ffi::c_char,
                i,
            );
            return 1 as ::core::ffi::c_int;
        }
        input_buffer[i as usize] = byte as ::core::ffi::c_char;
        i = i.wrapping_add(1);
    }
    if scanf(
        b"%zu\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut ref_len,
    ) != 1 as ::core::ffi::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error reading reference length\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return 1 as ::core::ffi::c_int;
    }
    if ref_len > MAX_BUFFER_SIZE as size_t {
        fprintf(
            stderr as *mut FILE,
            b"Error: reference length %zu exceeds maximum %d\n\0" as *const u8
                as *const ::core::ffi::c_char,
            ref_len,
            MAX_BUFFER_SIZE,
        );
        return 1 as ::core::ffi::c_int;
    }
    let mut i_0: size_t = 0 as size_t;
    while i_0 < ref_len {
        let mut byte_0: ::core::ffi::c_uint = 0;
        if scanf(
            b"%u\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut byte_0,
        ) != 1 as ::core::ffi::c_int
        {
            fprintf(
                stderr as *mut FILE,
                b"Error reading reference byte %zu\n\0" as *const u8 as *const ::core::ffi::c_char,
                i_0,
            );
            return 1 as ::core::ffi::c_int;
        }
        ref_buffer[i_0 as usize] = byte_0 as ::core::ffi::c_char;
        i_0 = i_0.wrapping_add(1);
    }
    let mut result: ::core::ffi::c_int = process_strings(
        &raw mut input_buffer as *mut ::core::ffi::c_char,
        input_len,
        &raw mut ref_buffer as *mut ::core::ffi::c_char,
        ref_len,
        operation,
        flags,
    );
    printf(b"%d\n\0" as *const u8 as *const ::core::ffi::c_char, result);
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
