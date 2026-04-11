extern "C" {
    fn __errno_location() -> *mut libc::c_int;
    fn pow(__x: libc::c_double, __y: libc::c_double) -> libc::c_double;
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
}
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
pub type size_t = usize;
pub type __off64_t = libc::linux_like::linux::gnu::b64::x86_64::not_x32::c_long;
pub type _IO_lock_t = ();
pub type __off_t = libc::linux_like::linux::gnu::b64::x86_64::not_x32::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: libc::c_int,
}
pub type FILE = crate::src::pow::_IO_FILE;
pub const EDOM: libc::c_int = 33 as libc::c_int;
pub const ERANGE: libc::c_int = 34 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn my_pow(
    mut base: libc::c_double,
    mut exponent: libc::c_double,
) -> libc::c_double {
    *__errno_location() = 0 as libc::c_int;
    let mut result: libc::c_double = pow(base, exponent);
    if *__errno_location() == EDOM {
        fprintf(
            stderr as *mut FILE,
            b"Domain error: pow(%.2f, %.2f) is undefined in the real number domain.\n\0"
                as *const u8 as *const libc::c_char,
            base,
            exponent,
        );
        return -(1 as libc::c_int) as libc::c_double;
    } else if *__errno_location() == ERANGE {
        fprintf(
            stderr as *mut FILE,
            b"Range error: pow(%.2f, %.2f) caused overflow or underflow.\n\0" as *const u8
                as *const libc::c_char,
            base,
            exponent,
        );
        return -(1 as libc::c_int) as libc::c_double;
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

