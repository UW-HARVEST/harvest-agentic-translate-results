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
    fn __errno_location() -> *mut ::core::ffi::c_int;
    static mut stdin: *mut _IO_FILE;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn fgets(
        __s: *mut ::core::ffi::c_char,
        __n: ::core::ffi::c_int,
        __stream: *mut FILE,
    ) -> *mut ::core::ffi::c_char;
    fn strtol(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct house_t {
    pub floors: ::core::ffi::c_int,
    pub bedrooms: ::core::ffi::c_int,
    pub bathrooms: ::core::ffi::c_double,
}
unsafe extern "C" fn add_floor(mut house: *mut house_t) {
    (*house).floors += 1;
}
unsafe extern "C" fn add_bedrooms(mut house: *mut house_t, mut extra_bedrooms: ::core::ffi::c_int) {
    (*house).bedrooms += extra_bedrooms;
}
unsafe extern "C" fn print_house(mut house: *mut house_t) {
    printf(
        b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0" as *const u8
            as *const ::core::ffi::c_char,
        (*house).floors,
        (*house).bedrooms,
        (*house).bathrooms,
    );
}
#[no_mangle]
pub unsafe extern "C" fn run(mut the_house: *mut house_t, mut extra_bedrooms: ::core::ffi::c_int) {
    print_house(the_house);
    add_floor(the_house);
    print_house(the_house);
    (*the_house).bathrooms += 1.0f64;
    print_house(the_house);
    add_bedrooms(the_house, extra_bedrooms);
    print_house(the_house);
}
unsafe extern "C" fn parse_val(
    mut str: *const ::core::ffi::c_char,
    mut val: *mut ::core::ffi::c_int,
) -> bool {
    *__errno_location() = 0 as ::core::ffi::c_int;
    let mut endp: *mut ::core::ffi::c_char = str as *mut ::core::ffi::c_char;
    let mut tmp: ::core::ffi::c_long = strtol(str, &raw mut endp, 10 as ::core::ffi::c_int);
    if endp != str as *mut ::core::ffi::c_char
        && *__errno_location() == 0 as ::core::ffi::c_int
        && tmp >= INT_MIN as ::core::ffi::c_long
        && tmp <= INT_MAX as ::core::ffi::c_long
    {
        *val = tmp as ::core::ffi::c_int;
        return true_0 != 0;
    } else {
        return false_0 != 0;
    };
}
unsafe fn main_0() -> ::core::ffi::c_int {
    let mut in_0: [::core::ffi::c_char; 100] = ::core::mem::transmute::<
        [u8; 100],
        [::core::ffi::c_char; 100],
    >(
        *b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
    );
    fgets(
        &raw mut in_0 as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 100]>() as ::core::ffi::c_int,
        stdin as *mut FILE,
    );
    let mut x: ::core::ffi::c_int = 0;
    if parse_val(&raw mut in_0 as *mut ::core::ffi::c_char, &raw mut x) {
        let mut the_house: house_t = house_t {
            floors: 2 as ::core::ffi::c_int,
            bedrooms: 5 as ::core::ffi::c_int,
            bathrooms: 2.5f64,
        };
        run(&raw mut the_house, x);
        run(&raw mut the_house, x);
    } else {
        printf(b"An error occurred\n\0" as *const u8 as *const ::core::ffi::c_char);
    }
    return 0 as ::core::ffi::c_int;
}
pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
pub const INT_MAX: ::core::ffi::c_int = __INT_MAX__;
pub const INT_MIN: ::core::ffi::c_int = -__INT_MAX__ - 1 as ::core::ffi::c_int;
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
