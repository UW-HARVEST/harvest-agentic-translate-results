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
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn fgets(
        __s: *mut libc::c_char,
        __n: libc::c_int,
        __stream: *mut FILE,
    ) -> *mut libc::c_char;
    fn atoi(__nptr: *const libc::c_char) -> libc::c_int;
    fn strlen(__s: *const libc::c_char) -> size_t;
    fn process_decisions(
        decision_string: *mut libc::c_char,
        length: size_t,
        operation: libc::c_int,
        param: libc::c_int,
    ) -> libc::c_int;
}
pub type size_t = usize;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: libc::c_int,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: libc::c_int,
    pub _flags2: libc::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut libc::c_void,
    pub __pad2: *mut libc::c_void,
    pub __pad3: *mut libc::c_void,
    pub __pad4: *mut libc::c_void,
    pub __pad5: size_t,
    pub _mode: libc::c_int,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: libc::c_int,
}
pub type FILE = _IO_FILE;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const MAX_INPUT_SIZE: libc::c_int = 1024 as libc::c_int;
unsafe fn main_0() -> libc::c_int {
    let mut input_buffer: [libc::c_char; 1024] = [0; 1024];
    let mut operation: libc::c_int = 0;
    let mut param: libc::c_int = 0;
    let mut result: libc::c_int = 0;
    if fgets(
        &raw mut input_buffer as *mut libc::c_char,
        MAX_INPUT_SIZE,
        stdin as *mut FILE,
    )
    .is_null()
    {
        fprintf(
            stderr as *mut FILE,
            b"Error reading operation\n\0" as *const u8 as *const libc::c_char,
        );
        return 1 as libc::c_int;
    }
    operation = atoi(&raw mut input_buffer as *mut libc::c_char);
    if fgets(
        &raw mut input_buffer as *mut libc::c_char,
        MAX_INPUT_SIZE,
        stdin as *mut FILE,
    )
    .is_null()
    {
        fprintf(
            stderr as *mut FILE,
            b"Error reading parameter\n\0" as *const u8 as *const libc::c_char,
        );
        return 1 as libc::c_int;
    }
    param = atoi(&raw mut input_buffer as *mut libc::c_char);
    if fgets(
        &raw mut input_buffer as *mut libc::c_char,
        MAX_INPUT_SIZE,
        stdin as *mut FILE,
    )
    .is_null()
    {
        fprintf(
            stderr as *mut FILE,
            b"Error reading decision string\n\0" as *const u8 as *const libc::c_char,
        );
        return 1 as libc::c_int;
    }
    let mut len: size_t = strlen(&raw mut input_buffer as *mut libc::c_char);
    if len > 0 as size_t
        && input_buffer[len.wrapping_sub(1 as size_t) as usize] as libc::c_int == '\n' as i32
    {
        input_buffer[len.wrapping_sub(1 as size_t) as usize] = '\0' as i32 as libc::c_char;
        len = len.wrapping_sub(1);
    }
    result = process_decisions(
        &raw mut input_buffer as *mut libc::c_char,
        len,
        operation,
        param,
    );
    printf(b"%d\n\0" as *const u8 as *const libc::c_char, result);
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
