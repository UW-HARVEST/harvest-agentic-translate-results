extern "C" {
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut libc::c_void;
    fn exit(__status: libc::c_int) -> !;
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn strrchr(
        __s: *const libc::c_char,
        __c: libc::c_int,
    ) -> *mut libc::c_char;
    fn strlen(__s: *const libc::c_char) -> size_t;
    fn strerror(__errnum: libc::c_int) -> *mut libc::c_char;
    fn __errno_location() -> *mut libc::c_int;
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
pub type FILE = crate::src::lib::_IO_FILE;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn extractFilename(
    mut path: *const libc::c_char,
    mut separator: libc::c_char,
) -> *const libc::c_char {
    let mut search: *const libc::c_char = strrchr(path, separator as libc::c_int);
    if search.is_null() {
        return path;
    }
    return search.offset(1 as libc::c_int as isize);
}
#[no_mangle]
pub unsafe extern "C" fn FIO_createFilename_fromOutDir(
    mut path: *const libc::c_char,
    mut outDirName: *const libc::c_char,
    suffixLen: size_t,
) -> *mut libc::c_char {
    let mut filenameStart: *const libc::c_char = std::ptr::null::<libc::c_char>();
    let mut separator: libc::c_char = 0;
    let mut result: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    separator = '/' as i32 as libc::c_char;
    filenameStart = extractFilename(path, separator);
    result = calloc(
        1 as size_t,
        strlen(outDirName)
            .wrapping_add(1 as size_t)
            .wrapping_add(strlen(filenameStart))
            .wrapping_add(suffixLen)
            .wrapping_add(1 as size_t),
    ) as *mut libc::c_char;
    if result.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"zstd: FIO_createFilename_fromOutDir: %s\0" as *const u8 as *const libc::c_char,
            strerror(*__errno_location()),
        );
        exit(30 as libc::c_int);
    }
    memcpy(
        result as *mut libc::c_void,
        outDirName as *const libc::c_void,
        strlen(outDirName),
    );
    if *outDirName.offset(strlen(outDirName).wrapping_sub(1 as size_t) as isize)
        as libc::c_int
        == separator as libc::c_int
    {
        memcpy(
            result.offset(strlen(outDirName) as isize) as *mut libc::c_void,
            filenameStart as *const libc::c_void,
            strlen(filenameStart),
        );
    } else {
        memcpy(
            result.offset(strlen(outDirName) as isize) as *mut libc::c_void,
            &raw mut separator as *const libc::c_void,
            1 as size_t,
        );
        memcpy(
            result
                .offset(strlen(outDirName) as isize)
                .offset(1 as libc::c_int as isize) as *mut libc::c_void,
            filenameStart as *const libc::c_void,
            strlen(filenameStart),
        );
    }
    return result;
}
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}

