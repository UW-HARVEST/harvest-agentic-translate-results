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
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn fgets(
        __s: *mut libc::c_char,
        __n: libc::c_int,
        __stream: *mut FILE,
    ) -> *mut libc::c_char;
    fn atoi(__nptr: *const libc::c_char) -> libc::c_int;
    fn memset(
        __s: *mut libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn strncpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
        __n: size_t,
    ) -> *mut libc::c_char;
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
#[no_mangle]
pub unsafe extern "C" fn printLine(mut line: *const libc::c_char) {
    if !line.is_null() {
        printf(b"%s\n\0" as *const u8 as *const libc::c_char, line);
    }
}
unsafe fn main_0() -> libc::c_int {
    let mut data: libc::c_int = 0;
    data = -(1 as libc::c_int);
    let mut inputBuffer: [libc::c_char; 14] = std::mem::transmute::<
        [u8; 14],
        [libc::c_char; 14],
    >(*b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0");
    if !fgets(
        &raw mut inputBuffer as *mut libc::c_char,
        14 as libc::c_int,
        stdin as *mut FILE,
    )
    .is_null()
    {
        data = atoi(&raw mut inputBuffer as *mut libc::c_char);
    } else {
        printLine(b"fgets() failed.\0" as *const u8 as *const libc::c_char);
    }
    let mut source: [libc::c_char; 100] = [0; 100];
    let mut dest: [libc::c_char; 100] = std::mem::transmute::<
        [u8; 100],
        [libc::c_char; 100],
    >(
        *b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
    );
    memset(
        &raw mut source as *mut libc::c_char as *mut libc::c_void,
        'A' as i32,
        (100 as libc::c_int - 1 as libc::c_int) as size_t,
    );
    source[(100 as libc::c_int - 1 as libc::c_int) as usize] =
        '\0' as i32 as libc::c_char;
    if data < 100 as libc::c_int {
        strncpy(
            &raw mut dest as *mut libc::c_char,
            &raw mut source as *mut libc::c_char,
            data as size_t,
        );
        dest[data as usize] = '\0' as i32 as libc::c_char;
    }
    printLine(&raw mut dest as *mut libc::c_char);
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
