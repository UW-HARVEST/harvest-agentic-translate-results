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
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn scanf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn process_buffer(
        buffer: *mut uint8_t,
        length: size_t,
        flags: uint32_t,
        param1: libc::c_int,
        param2: libc::c_int,
    ) -> size_t;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint32_t = u32;
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
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
unsafe fn main_0() -> libc::c_int {
    let mut flags: uint32_t = 0;
    let mut param1: libc::c_int = 0;
    let mut param2: libc::c_int = 0;
    let mut length: size_t = 0;
    let mut buffer: [uint8_t; 256] = [0; 256];
    if scanf(
        b"%u\0" as *const u8 as *const libc::c_char,
        &raw mut flags,
    ) != 1 as libc::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error reading flags\n\0" as *const u8 as *const libc::c_char,
        );
        return 1 as libc::c_int;
    }
    if scanf(
        b"%d\0" as *const u8 as *const libc::c_char,
        &raw mut param1,
    ) != 1 as libc::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error reading param1\n\0" as *const u8 as *const libc::c_char,
        );
        return 1 as libc::c_int;
    }
    if scanf(
        b"%d\0" as *const u8 as *const libc::c_char,
        &raw mut param2,
    ) != 1 as libc::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error reading param2\n\0" as *const u8 as *const libc::c_char,
        );
        return 1 as libc::c_int;
    }
    if scanf(
        b"%zu\0" as *const u8 as *const libc::c_char,
        &raw mut length,
    ) != 1 as libc::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error reading length\n\0" as *const u8 as *const libc::c_char,
        );
        return 1 as libc::c_int;
    }
    if length > 256 as size_t {
        fprintf(
            stderr as *mut FILE,
            b"Error: length %zu exceeds maximum 256\n\0" as *const u8 as *const libc::c_char,
            length,
        );
        return 1 as libc::c_int;
    }
    let mut i: size_t = 0 as size_t;
    while i < length {
        let mut byte: libc::c_uint = 0;
        if scanf(
            b"%u\0" as *const u8 as *const libc::c_char,
            &raw mut byte,
        ) != 1 as libc::c_int
        {
            fprintf(
                stderr as *mut FILE,
                b"Error reading byte %zu\n\0" as *const u8 as *const libc::c_char,
                i,
            );
            return 1 as libc::c_int;
        }
        buffer[i as usize] = byte as uint8_t;
        i = i.wrapping_add(1);
    }
    let mut new_length: size_t = process_buffer(
        &raw mut buffer as *mut uint8_t,
        length,
        flags,
        param1,
        param2,
    );
    printf(
        b"%zu\0" as *const u8 as *const libc::c_char,
        new_length,
    );
    let mut i_0: size_t = 0 as size_t;
    while i_0 < new_length {
        printf(
            b" %u\0" as *const u8 as *const libc::c_char,
            buffer[i_0 as usize] as libc::c_int,
        );
        i_0 = i_0.wrapping_add(1);
    }
    printf(b"\n\0" as *const u8 as *const libc::c_char);
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
