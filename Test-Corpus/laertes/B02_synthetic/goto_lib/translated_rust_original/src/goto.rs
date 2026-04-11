extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fclose(__stream: *mut FILE) -> libc::c_int;
    fn fopen(
        __filename: *const libc::c_char,
        __modes: *const libc::c_char,
    ) -> *mut FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn fgets(
        __s: *mut libc::c_char,
        __n: libc::c_int,
        __stream: *mut FILE,
    ) -> *mut libc::c_char;
    fn ferror(__stream: *mut FILE) -> libc::c_int;
}
pub type size_t = usize;
pub type __off_t = libc::linux_like::linux::gnu::b64::x86_64::not_x32::c_long;
pub type __off64_t = libc::linux_like::linux::gnu::b64::x86_64::not_x32::c_long;
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
pub type FILE = crate::src::goto::_IO_FILE;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn forward_goto_example(mut x: libc::c_int) -> libc::c_int {
    if x < 0 as libc::c_int {
        fprintf(
            stderr as *mut FILE,
            b"Error: negative input\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    } else {
        printf(
            b"Processing: %d\n\0" as *const u8 as *const libc::c_char,
            x,
        );
        return x * 2 as libc::c_int;
    };
}
#[no_mangle]
pub unsafe extern "C" fn open_with_cleanup(mut filename: *const libc::c_char) -> *mut FILE {
    let mut buffer: [libc::c_char; 100] = [0; 100];
    let mut fp: *mut FILE = fopen(filename, b"r\0" as *const u8 as *const libc::c_char);
    if !fp.is_null() {
        buffer = [0; 100];
        while !fgets(
            &raw mut buffer as *mut libc::c_char,
            std::mem::size_of::<[libc::c_char; 100]>() as libc::c_int,
            fp,
        )
        .is_null()
        {
            printf(
                b"%s\0" as *const u8 as *const libc::c_char,
                &raw mut buffer as *mut libc::c_char,
            );
        }
        if !(ferror(fp) != 0) {
            return fp;
        }
    }
    fprintf(
        stderr as *mut FILE,
        b"Error: opening or processing file %s\n\0" as *const u8 as *const libc::c_char,
        filename,
    );
    if !fp.is_null() {
        fclose(fp);
    }
    return std::ptr::null_mut::<FILE>();
}
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut num: libc::c_int,
    mut filename: *const libc::c_char,
) -> libc::c_int {
    let mut res: libc::c_int = forward_goto_example(num);
    if res == -(1 as libc::c_int) {
        return -(1 as libc::c_int);
    } else {
        printf(
            b"Goto output: %d\n\0" as *const u8 as *const libc::c_char,
            res,
        );
    }
    let mut out: *mut FILE = open_with_cleanup(filename);
    if out.is_null() {
        return -(2 as libc::c_int);
    } else {
        fclose(out);
    }
    return 0 as libc::c_int;
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

