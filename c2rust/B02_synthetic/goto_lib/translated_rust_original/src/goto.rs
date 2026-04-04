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
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn fgets(
        __s: *mut ::core::ffi::c_char,
        __n: ::core::ffi::c_int,
        __stream: *mut FILE,
    ) -> *mut ::core::ffi::c_char;
    fn ferror(__stream: *mut FILE) -> ::core::ffi::c_int;
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
pub unsafe extern "C" fn forward_goto_example(mut x: ::core::ffi::c_int) -> ::core::ffi::c_int {
    if x < 0 as ::core::ffi::c_int {
        fprintf(
            stderr as *mut FILE,
            b"Error: negative input\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return -(1 as ::core::ffi::c_int);
    } else {
        printf(
            b"Processing: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
            x,
        );
        return x * 2 as ::core::ffi::c_int;
    };
}
#[no_mangle]
pub unsafe extern "C" fn open_with_cleanup(mut filename: *const ::core::ffi::c_char) -> *mut FILE {
    let mut buffer: [::core::ffi::c_char; 100] = [0; 100];
    let mut fp: *mut FILE = fopen(filename, b"r\0" as *const u8 as *const ::core::ffi::c_char);
    if !fp.is_null() {
        buffer = [0; 100];
        while !fgets(
            &raw mut buffer as *mut ::core::ffi::c_char,
            ::core::mem::size_of::<[::core::ffi::c_char; 100]>() as ::core::ffi::c_int,
            fp,
        )
        .is_null()
        {
            printf(
                b"%s\0" as *const u8 as *const ::core::ffi::c_char,
                &raw mut buffer as *mut ::core::ffi::c_char,
            );
        }
        if !(ferror(fp) != 0) {
            return fp;
        }
    }
    fprintf(
        stderr as *mut FILE,
        b"Error: opening or processing file %s\n\0" as *const u8 as *const ::core::ffi::c_char,
        filename,
    );
    if !fp.is_null() {
        fclose(fp);
    }
    return ::core::ptr::null_mut::<FILE>();
}
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut num: ::core::ffi::c_int,
    mut filename: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut res: ::core::ffi::c_int = forward_goto_example(num);
    if res == -(1 as ::core::ffi::c_int) {
        return -(1 as ::core::ffi::c_int);
    } else {
        printf(
            b"Goto output: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
            res,
        );
    }
    let mut out: *mut FILE = open_with_cleanup(filename);
    if out.is_null() {
        return -(2 as ::core::ffi::c_int);
    } else {
        fclose(out);
    }
    return 0 as ::core::ffi::c_int;
}
