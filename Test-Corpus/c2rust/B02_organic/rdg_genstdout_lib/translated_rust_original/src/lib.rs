extern "C" {
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn exit(__status: ::core::ffi::c_int) -> !;
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strrchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn strerror(__errnum: ::core::ffi::c_int) -> *mut ::core::ffi::c_char;
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
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub unsafe extern "C" fn extractFilename(
    mut path: *const ::core::ffi::c_char,
    mut separator: ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    let mut search: *const ::core::ffi::c_char = strrchr(path, separator as ::core::ffi::c_int);
    if search.is_null() {
        return path;
    }
    return search.offset(1 as ::core::ffi::c_int as isize);
}
#[no_mangle]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    mut path: *const ::core::ffi::c_char,
    mut outDirName: *const ::core::ffi::c_char,
    suffixLen: size_t,
) -> *mut ::core::ffi::c_char {
    let mut filenameStart: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut separator: ::core::ffi::c_char = 0;
    let mut result: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    separator = '/' as i32 as ::core::ffi::c_char;
    filenameStart = extractFilename(path, separator);
    result = calloc(
        1 as size_t,
        strlen(outDirName)
            .wrapping_add(1 as size_t)
            .wrapping_add(strlen(filenameStart))
            .wrapping_add(suffixLen)
            .wrapping_add(1 as size_t),
    ) as *mut ::core::ffi::c_char;
    if result.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"zstd: FIO_createFilename_fromOutDir: %s\0" as *const u8 as *const ::core::ffi::c_char,
            strerror(*__errno_location()),
        );
        exit(30 as ::core::ffi::c_int);
    }
    memcpy(
        result as *mut ::core::ffi::c_void,
        outDirName as *const ::core::ffi::c_void,
        strlen(outDirName),
    );
    if *outDirName.offset(strlen(outDirName).wrapping_sub(1 as size_t) as isize)
        as ::core::ffi::c_int
        == separator as ::core::ffi::c_int
    {
        memcpy(
            result.offset(strlen(outDirName) as isize) as *mut ::core::ffi::c_void,
            filenameStart as *const ::core::ffi::c_void,
            strlen(filenameStart),
        );
    } else {
        memcpy(
            result.offset(strlen(outDirName) as isize) as *mut ::core::ffi::c_void,
            &raw mut separator as *const ::core::ffi::c_void,
            1 as size_t,
        );
        memcpy(
            result
                .offset(strlen(outDirName) as isize)
                .offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
            filenameStart as *const ::core::ffi::c_void,
            strlen(filenameStart),
        );
    }
    return result;
}
