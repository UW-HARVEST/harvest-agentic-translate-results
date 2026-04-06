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
    static mut stdin: *mut _IO_FILE;
    static mut stderr: *mut _IO_FILE;
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
    fn atoi(__nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn process_decisions(
        decision_string: *mut ::core::ffi::c_char,
        length: size_t,
        operation: ::core::ffi::c_int,
        param: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
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
pub const MAX_INPUT_SIZE: ::core::ffi::c_int = 1024 as ::core::ffi::c_int;
unsafe fn main_0() -> ::core::ffi::c_int {
    let mut input_buffer: [::core::ffi::c_char; 1024] = [0; 1024];
    let mut operation: ::core::ffi::c_int = 0;
    let mut param: ::core::ffi::c_int = 0;
    let mut result: ::core::ffi::c_int = 0;
    if fgets(
        &raw mut input_buffer as *mut ::core::ffi::c_char,
        MAX_INPUT_SIZE,
        stdin as *mut FILE,
    )
    .is_null()
    {
        fprintf(
            stderr as *mut FILE,
            b"Error reading operation\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return 1 as ::core::ffi::c_int;
    }
    operation = atoi(&raw mut input_buffer as *mut ::core::ffi::c_char);
    if fgets(
        &raw mut input_buffer as *mut ::core::ffi::c_char,
        MAX_INPUT_SIZE,
        stdin as *mut FILE,
    )
    .is_null()
    {
        fprintf(
            stderr as *mut FILE,
            b"Error reading parameter\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return 1 as ::core::ffi::c_int;
    }
    param = atoi(&raw mut input_buffer as *mut ::core::ffi::c_char);
    if fgets(
        &raw mut input_buffer as *mut ::core::ffi::c_char,
        MAX_INPUT_SIZE,
        stdin as *mut FILE,
    )
    .is_null()
    {
        fprintf(
            stderr as *mut FILE,
            b"Error reading decision string\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return 1 as ::core::ffi::c_int;
    }
    let mut len: size_t = strlen(&raw mut input_buffer as *mut ::core::ffi::c_char);
    if len > 0 as size_t
        && input_buffer[len.wrapping_sub(1 as size_t) as usize] as ::core::ffi::c_int == '\n' as i32
    {
        input_buffer[len.wrapping_sub(1 as size_t) as usize] = '\0' as i32 as ::core::ffi::c_char;
        len = len.wrapping_sub(1);
    }
    result = process_decisions(
        &raw mut input_buffer as *mut ::core::ffi::c_char,
        len,
        operation,
        param,
    );
    printf(b"%d\n\0" as *const u8 as *const ::core::ffi::c_char, result);
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
