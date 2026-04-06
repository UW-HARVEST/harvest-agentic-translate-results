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
    fn getenv(__name: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
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
static mut log_file: *mut FILE = ::core::ptr::null::<FILE>() as *mut FILE;
#[no_mangle]
pub unsafe extern "C" fn initialize_logger() -> ::core::ffi::c_int {
    let mut log_file_env: *const ::core::ffi::c_char =
        getenv(b"LOG_FILE\0" as *const u8 as *const ::core::ffi::c_char);
    let mut log_file_path: *const ::core::ffi::c_char = if !log_file_env.is_null() {
        log_file_env
    } else {
        b"default.log\0" as *const u8 as *const ::core::ffi::c_char
    };
    log_file = fopen(
        log_file_path,
        b"a\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if log_file.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Failed to open log file: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            log_file_path,
        );
        return -(1 as ::core::ffi::c_int);
    }
    log_info(b"Logger initialized.\0" as *const u8 as *const ::core::ffi::c_char);
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn log_info(mut message: *const ::core::ffi::c_char) {
    if !log_file.is_null() {
        fprintf(
            log_file,
            b"[INFO] %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            message,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn log_warning(mut message: *const ::core::ffi::c_char) {
    if !log_file.is_null() {
        fprintf(
            log_file,
            b"[WARNING] %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            message,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn log_error(mut message: *const ::core::ffi::c_char) {
    if !log_file.is_null() {
        fprintf(
            log_file,
            b"[ERROR] %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            message,
        );
    }
}
#[no_mangle]
pub unsafe extern "C" fn finalize_logger() {
    if !log_file.is_null() {
        log_info(b"Logger finalized.\0" as *const u8 as *const ::core::ffi::c_char);
        fclose(log_file);
    }
}
