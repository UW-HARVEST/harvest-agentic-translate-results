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
pub unsafe extern "C" fn printLine(mut line: *const ::core::ffi::c_char) {
    if !line.is_null() {
        printf(b"%s\n\0" as *const u8 as *const ::core::ffi::c_char, line);
    }
}
#[no_mangle]
pub unsafe extern "C" fn printIntLine(mut intNumber: ::core::ffi::c_int) {
    printf(
        b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        intNumber,
    );
}
pub const CHAR_ARRAY_SIZE: ::core::ffi::c_int = 20 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn bad() {
    let mut data: ::core::ffi::c_float = 0.;
    data = 0.0f32;
    let mut inputBuffer: [::core::ffi::c_char; 20] = [0; 20];
    if !fgets(
        &raw mut inputBuffer as *mut ::core::ffi::c_char,
        CHAR_ARRAY_SIZE,
        stdin as *mut FILE,
    )
    .is_null()
    {
        data = atof(&raw mut inputBuffer as *mut ::core::ffi::c_char) as ::core::ffi::c_float;
    } else {
        printLine(b"fgets() failed.\0" as *const u8 as *const ::core::ffi::c_char);
    }
    let mut result: ::core::ffi::c_int =
        (100.0f64 / data as ::core::ffi::c_double) as ::core::ffi::c_int;
    printIntLine(result);
}
unsafe extern "C" fn goodG2B() {
    let mut data: ::core::ffi::c_float = 0.;
    data = 2.0f32;
    let mut result: ::core::ffi::c_int =
        (100.0f64 / data as ::core::ffi::c_double) as ::core::ffi::c_int;
    printIntLine(result);
}
unsafe extern "C" fn goodB2G() {
    let mut data: ::core::ffi::c_float = 0.;
    data = 0.0f32;
    let mut inputBuffer: [::core::ffi::c_char; 20] = [0; 20];
    if !fgets(
        &raw mut inputBuffer as *mut ::core::ffi::c_char,
        CHAR_ARRAY_SIZE,
        stdin as *mut FILE,
    )
    .is_null()
    {
        data = atof(&raw mut inputBuffer as *mut ::core::ffi::c_char) as ::core::ffi::c_float;
    } else {
        printLine(b"fgets() failed.\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if fabs(data as ::core::ffi::c_double) > 0.000001f64 {
        let mut result: ::core::ffi::c_int =
            (100.0f64 / data as ::core::ffi::c_double) as ::core::ffi::c_int;
        printIntLine(result);
    } else {
        printLine(
            b"This would result in a divide by zero\0" as *const u8 as *const ::core::ffi::c_char,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn good() {
    goodG2B();
    goodB2G();
}
unsafe fn main_0(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    printLine(b"Calling good()...\0" as *const u8 as *const ::core::ffi::c_char);
    good();
    printLine(b"Finished good()\0" as *const u8 as *const ::core::ffi::c_char);
    printLine(b"Calling bad()...\0" as *const u8 as *const ::core::ffi::c_char);
    bad();
    printLine(b"Finished bad()\0" as *const u8 as *const ::core::ffi::c_char);
    return 0 as ::core::ffi::c_int;
}
pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args()
        .map(|arg| {
            ::std::ffi::CString::new(arg)
                .expect("Failed to convert argument into CString.")
                .into_bytes_with_nul()
        })
        .collect();
    let mut args_ptrs: Vec<*mut ::core::ffi::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut ::core::ffi::c_char)
        .chain(::core::iter::once(::core::ptr::null_mut()))
        .collect();
    unsafe {
        ::std::process::exit(main_0(
            (args_ptrs.len() - 1) as ::core::ffi::c_int,
            args_ptrs.as_mut_ptr() as *mut *mut ::core::ffi::c_char,
        ) as i32)
    }
}
