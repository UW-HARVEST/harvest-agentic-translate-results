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
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn atof(__nptr: *const libc::c_char) -> libc::c_double;
    fn exit(__status: libc::c_int) -> !;
    fn Q_rsqrt(f: libc::c_float) -> libc::c_float;
}
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
pub type size_t = usize;
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
pub type FILE = _IO_FILE;
pub type vec_t = libc::c_float;
pub type vec3_t = [vec_t; 3];
#[inline]
unsafe extern "C" fn VectorNormalizeFast(mut v: *mut vec_t) {
    let mut ilength: libc::c_float = 0.;
    ilength = Q_rsqrt(
        *v.offset(0 as libc::c_int as isize) as libc::c_float
            * *v.offset(0 as libc::c_int as isize) as libc::c_float
            + *v.offset(1 as libc::c_int as isize) as libc::c_float
                * *v.offset(1 as libc::c_int as isize) as libc::c_float
            + *v.offset(2 as libc::c_int as isize) as libc::c_float
                * *v.offset(2 as libc::c_int as isize) as libc::c_float,
    );
    let ref mut fresh0 = *v.offset(0 as libc::c_int as isize);
    *fresh0 *= ilength;
    let ref mut fresh1 = *v.offset(1 as libc::c_int as isize);
    *fresh1 *= ilength;
    let ref mut fresh2 = *v.offset(2 as libc::c_int as isize);
    *fresh2 *= ilength;
}
unsafe fn main_0(
    mut argc: libc::c_int,
    mut argv: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut Inputs: vec3_t = [0.; 3];
    if argc != 4 as libc::c_int {
        fprintf(
            stderr as *mut FILE,
            b"%s requires 4 inputs\n\0" as *const u8 as *const libc::c_char,
            *argv.offset(0 as libc::c_int as isize),
        );
        exit(1 as libc::c_int);
    }
    Inputs[0 as libc::c_int as usize] =
        atof(*argv.offset(1 as libc::c_int as isize)) as vec_t;
    Inputs[1 as libc::c_int as usize] =
        atof(*argv.offset(2 as libc::c_int as isize)) as vec_t;
    Inputs[2 as libc::c_int as usize] =
        atof(*argv.offset(3 as libc::c_int as isize)) as vec_t;
    VectorNormalizeFast(&raw mut Inputs as *mut vec_t);
    printf(
        b"%f %f %f\n\0" as *const u8 as *const libc::c_char,
        Inputs[0 as libc::c_int as usize] as libc::c_double,
        Inputs[1 as libc::c_int as usize] as libc::c_double,
        Inputs[2 as libc::c_int as usize] as libc::c_double,
    );
    return 0 as libc::c_int;
}
pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args()
        .map(|arg| {
            ::std::ffi::CString::new(arg)
                .expect("Failed to convert argument into CString.")
                .into_bytes_with_nul()
        })
        .collect();
    let mut args_ptrs: Vec<*mut libc::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut libc::c_char)
        .chain(::core::iter::once(std::ptr::null_mut()))
        .collect();
    unsafe {
        ::std::process::exit(main_0(
            (args_ptrs.len() - 1) as libc::c_int,
            args_ptrs.as_mut_ptr() as *mut *mut libc::c_char,
        ) as i32)
    }
}
