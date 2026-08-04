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
#[no_mangle]
pub unsafe extern "C" fn printIntLine(mut intNumber: libc::c_int) {
    printf(
        b"%d\n\0" as *const u8 as *const libc::c_char,
        intNumber,
    );
}
#[no_mangle]
pub unsafe extern "C" fn bad() {
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
    let mut i: libc::c_int = 0;
    let mut buffer: [libc::c_int; 10] = [0 as libc::c_int; 10];
    if data >= 0 as libc::c_int {
        buffer[data as usize] = 1 as libc::c_int;
        i = 0 as libc::c_int;
        while i < 10 as libc::c_int {
            printIntLine(buffer[i as usize]);
            i += 1;
        }
    } else {
        printLine(b"ERROR: Array index is negative.\0" as *const u8 as *const libc::c_char);
    };
}
unsafe extern "C" fn goodG2B() {
    let mut data: libc::c_int = 0;
    data = -(1 as libc::c_int);
    data = 7 as libc::c_int;
    let mut i: libc::c_int = 0;
    let mut buffer: [libc::c_int; 10] = [0 as libc::c_int; 10];
    if data >= 0 as libc::c_int {
        buffer[data as usize] = 1 as libc::c_int;
        i = 0 as libc::c_int;
        while i < 10 as libc::c_int {
            printIntLine(buffer[i as usize]);
            i += 1;
        }
    } else {
        printLine(b"ERROR: Array index is negative.\0" as *const u8 as *const libc::c_char);
    };
}
unsafe extern "C" fn goodB2G() {
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
    let mut i: libc::c_int = 0;
    let mut buffer: [libc::c_int; 10] = [0 as libc::c_int; 10];
    if data >= 0 as libc::c_int && data < 10 as libc::c_int {
        buffer[data as usize] = 1 as libc::c_int;
        i = 0 as libc::c_int;
        while i < 10 as libc::c_int {
            printIntLine(buffer[i as usize]);
            i += 1;
        }
    } else {
        printLine(
            b"ERROR: Array index is out-of-bounds\0" as *const u8 as *const libc::c_char,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    goodG2B();
    goodB2G();
}
unsafe fn main_0(
    mut argc: libc::c_int,
    mut argv: *mut *mut libc::c_char,
) -> libc::c_int {
    printLine(b"Calling good()...\0" as *const u8 as *const libc::c_char);
    good();
    printLine(b"Finished good()\0" as *const u8 as *const libc::c_char);
    printLine(b"Calling bad()...\0" as *const u8 as *const libc::c_char);
    bad();
    printLine(b"Finished bad()\0" as *const u8 as *const libc::c_char);
    return 0 as libc::c_int;
}
pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args()
        .map(|arg| {
            ::std::ffi::CString::new(arg)
                .expect("Failed to convert argument into CString.")
                .into_bytes_with_nul()
        })
        .collect();
    let mut args_ptrs: Vec<*mut libc::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut libc::c_char)
        .chain(::core::iter::once(std::ptr::null_mut()))
        .collect();
    unsafe {
        ::std::process::exit(main_0(
            (args_ptrs.len() - 1) as libc::c_int,
            args_ptrs.as_mut_ptr() as *mut *mut libc::c_char,
        ) as i32)
    }
}
