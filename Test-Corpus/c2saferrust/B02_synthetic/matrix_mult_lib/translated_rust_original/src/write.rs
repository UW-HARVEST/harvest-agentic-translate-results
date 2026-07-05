extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fclose(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn fopen(
        __filename: *const ::core::ffi::c_char,
        __modes: *const ::core::ffi::c_char,
    ) -> *mut FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn __errno_location() -> *mut ::core::ffi::c_int;
    fn strerror(__errnum: ::core::ffi::c_int) -> *mut ::core::ffi::c_char;
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
pub const EINVAL: ::core::ffi::c_int = 22 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn write_to_file(
    mut filename: *const ::core::ffi::c_char,
    mut content: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if content.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: Content is NULL.\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return EINVAL;
    }
    let mut file: *mut FILE = fopen(filename, b"w\0" as *const u8 as *const ::core::ffi::c_char);
    if file.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error opening file '%s': %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            filename,
            strerror(*__errno_location()),
        );
        return *__errno_location();
    }
    if fprintf(
        file,
        b"%s\0" as *const u8 as *const ::core::ffi::c_char,
        content,
    ) < 0 as ::core::ffi::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error writing to file '%s': %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            filename,
            strerror(*__errno_location()),
        );
        fclose(file);
        return *__errno_location();
    }
    if fclose(file) != 0 as ::core::ffi::c_int {
        fprintf(
            stderr as *mut FILE,
            b"Error closing file '%s': %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            filename,
            strerror(*__errno_location()),
        );
        return *__errno_location();
    }
    return 0 as ::core::ffi::c_int;
}
