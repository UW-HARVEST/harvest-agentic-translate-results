extern "C" {
    fn __errno_location() -> *mut ::core::ffi::c_int;
    fn pow(__x: ::core::ffi::c_double, __y: ::core::ffi::c_double) -> ::core::ffi::c_double;
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
}
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
pub type size_t = usize;
pub type __off64_t = ::core::ffi::c_long;
pub type _IO_lock_t = ();
pub type __off_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: ::core::ffi::c_int,
}
pub type FILE = _IO_FILE;
pub const EDOM: ::core::ffi::c_int = 33 as ::core::ffi::c_int;
pub const ERANGE: ::core::ffi::c_int = 34 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn my_pow(
    mut base: ::core::ffi::c_double,
    mut exponent: ::core::ffi::c_double,
) -> ::core::ffi::c_double {
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
        return -(1 as ::core::ffi::c_int) as ::core::ffi::c_double;
    } else if *__errno_location() == ERANGE {
        fprintf(
            stderr as *mut FILE,
            b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0" as *const u8
                as *const ::core::ffi::c_char,
            base,
            exponent,
        );
        return -(1 as ::core::ffi::c_int) as ::core::ffi::c_double;
    }
    return result;
}
