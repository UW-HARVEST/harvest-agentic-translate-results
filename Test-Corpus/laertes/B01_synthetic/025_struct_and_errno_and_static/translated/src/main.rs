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
    fn __errno_location() -> *mut libc::c_int;
    static mut stdin: *mut _IO_FILE;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn fgets(
        __s: *mut libc::c_char,
        __n: libc::c_int,
        __stream: *mut FILE,
    ) -> *mut libc::c_char;
    fn strtol(
        __nptr: *const libc::c_char,
        __endptr: *mut *mut libc::c_char,
        __base: libc::c_int,
    ) -> libc::c_long;
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
pub type FILE = _IO_FILE;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct house_t {
    pub floors: libc::c_int,
    pub bedrooms: libc::c_int,
    pub bathrooms: libc::c_double,
}
static mut the_house: house_t = house_t {
    floors: 2 as libc::c_int,
    bedrooms: 5 as libc::c_int,
    bathrooms: 2.5f64,
};
unsafe extern "C" fn add_floor(mut house: *mut house_t) {
    (*house).floors += 1;
}
unsafe extern "C" fn add_bedrooms(mut house: *mut house_t, mut extra_bedrooms: libc::c_int) {
    (*house).bedrooms += extra_bedrooms;
}
unsafe extern "C" fn add_floor_to_the_house() {
    add_floor(&raw mut the_house);
}
unsafe extern "C" fn print_the_house() {
    printf(
        b"The house has %d floors, %d bedrooms, and %.1f bathrooms\n\0" as *const u8
            as *const libc::c_char,
        the_house.floors,
        the_house.bedrooms,
        the_house.bathrooms,
    );
}
#[no_mangle]
pub unsafe extern "C" fn run(mut extra_bedrooms: libc::c_int) {
    print_the_house();
    add_floor_to_the_house();
    print_the_house();
    the_house.bathrooms += 1.0f64;
    print_the_house();
    add_bedrooms(&raw mut the_house, extra_bedrooms);
    print_the_house();
}
unsafe extern "C" fn parse_val(
    mut str: *const libc::c_char,
    mut val: *mut libc::c_int,
) -> bool {
    *__errno_location() = 0 as libc::c_int;
    let mut endp: *mut libc::c_char = str as *mut libc::c_char;
    let mut tmp: libc::c_long = strtol(str, &raw mut endp, 10 as libc::c_int);
    if endp != str as *mut libc::c_char
        && *__errno_location() == 0 as libc::c_int
        && tmp >= INT_MIN as libc::c_long
        && tmp <= INT_MAX as libc::c_long
    {
        *val = tmp as libc::c_int;
        return true_0 != 0;
    } else {
        return false_0 != 0;
    };
}
unsafe fn main_0() -> libc::c_int {
    let mut in_0: [libc::c_char; 100] = std::mem::transmute::<
        [u8; 100],
        [libc::c_char; 100],
    >(
        *b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
    );
    fgets(
        &raw mut in_0 as *mut libc::c_char,
        std::mem::size_of::<[libc::c_char; 100]>() as libc::c_int,
        stdin as *mut FILE,
    );
    let mut x: libc::c_int = 0;
    if parse_val(&raw mut in_0 as *mut libc::c_char, &raw mut x) {
        run(x);
        run(x);
    } else {
        printf(b"An error occurred\n\0" as *const u8 as *const libc::c_char);
    }
    return 0 as libc::c_int;
}
pub const __INT_MAX__: libc::c_int = 2147483647 as libc::c_int;
pub const INT_MAX: libc::c_int = __INT_MAX__;
pub const INT_MIN: libc::c_int = -__INT_MAX__ - 1 as libc::c_int;
pub const true_0: libc::c_int = 1 as libc::c_int;
pub const false_0: libc::c_int = 0 as libc::c_int;
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
