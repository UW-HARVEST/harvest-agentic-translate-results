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
unsafe fn main_0(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if argc != 3 as ::core::ffi::c_int {
        fprintf(
            stderr as *mut FILE,
            b"Usage: %s base exponent\n\0" as *const u8 as *const ::core::ffi::c_char,
            *argv.offset(0 as ::core::ffi::c_int as isize),
        );
        return 1 as ::core::ffi::c_int;
    }
    let mut endptr1: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut endptr2: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    *__errno_location() = 0 as ::core::ffi::c_int;
    let mut base: ::core::ffi::c_double = strtod(
        *argv.offset(1 as ::core::ffi::c_int as isize),
        &raw mut endptr1,
    );
    if *__errno_location() == ERANGE {
        fprintf(
            stderr as *mut FILE,
            b"Range error while converting base '%s'\n\0" as *const u8
                as *const ::core::ffi::c_char,
            *argv.offset(1 as ::core::ffi::c_int as isize),
        );
        return 1 as ::core::ffi::c_int;
    } else if *endptr1 as ::core::ffi::c_int != '\0' as i32 {
        fprintf(
            stderr as *mut FILE,
            b"Invalid numeric input for base: '%s'\n\0" as *const u8 as *const ::core::ffi::c_char,
            *argv.offset(1 as ::core::ffi::c_int as isize),
        );
        return 1 as ::core::ffi::c_int;
    }
    *__errno_location() = 0 as ::core::ffi::c_int;
    let mut exponent: ::core::ffi::c_double = strtod(
        *argv.offset(2 as ::core::ffi::c_int as isize),
        &raw mut endptr2,
    );
    if *__errno_location() == ERANGE {
        fprintf(
            stderr as *mut FILE,
            b"Range error while converting exponent '%s'\n\0" as *const u8
                as *const ::core::ffi::c_char,
            *argv.offset(2 as ::core::ffi::c_int as isize),
        );
        return 1 as ::core::ffi::c_int;
    } else if *endptr2 as ::core::ffi::c_int != '\0' as i32 {
        fprintf(
            stderr as *mut FILE,
            b"Invalid numeric input for exponent: '%s'\n\0" as *const u8
                as *const ::core::ffi::c_char,
            *argv.offset(2 as ::core::ffi::c_int as isize),
        );
        return 1 as ::core::ffi::c_int;
    }
    *__errno_location() = 0 as ::core::ffi::c_int;
    let mut result: ::core::ffi::c_double = pow(base, exponent);
    if *__errno_location() == EDOM {
        fprintf(
            stderr as *mut FILE,
            b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0"
                as *const u8 as *const ::core::ffi::c_char,
            base,
            exponent,
        );
        return 1 as ::core::ffi::c_int;
    } else if *__errno_location() == ERANGE {
        fprintf(
            stderr as *mut FILE,
            b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0" as *const u8
                as *const ::core::ffi::c_char,
            base,
            exponent,
        );
        return 1 as ::core::ffi::c_int;
    }
    printf(
        b"Result: %.2f\n\0" as *const u8 as *const ::core::ffi::c_char,
        result,
    );
    return 0 as ::core::ffi::c_int;
}
pub const EDOM: ::core::ffi::c_int = 33 as ::core::ffi::c_int;
pub const ERANGE: ::core::ffi::c_int = 34 as ::core::ffi::c_int;
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
