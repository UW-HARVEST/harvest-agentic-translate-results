#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]






use std::ffi::CStr;

use std::io;

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
static mut the_house: house_t = house_t {
    floors: 2 as ::core::ffi::c_int,
    bedrooms: 5 as ::core::ffi::c_int,
    bathrooms: 2.5f64,
};
fn add_floor(house: &mut house_t) {
    house.floors += 1;
}

fn add_bedrooms(house: &mut house_t, extra_bedrooms: i32) {
    house.bedrooms += extra_bedrooms;
}

fn add_floor_to_the_house() {
    unsafe {
        add_floor(&mut the_house);
    }
}

fn print_the_house() {
    println!(
        "The house has {} floors, {} bedrooms, and {:.1} bathrooms",
        unsafe { the_house.floors },
        unsafe { the_house.bedrooms },
        unsafe { the_house.bathrooms },
    );
}

#[no_mangle]
pub fn run(extra_bedrooms: i32) {
    unsafe {
        print_the_house();
        add_floor_to_the_house();
        print_the_house();
        the_house.bathrooms += 1.0;
        print_the_house();
        add_bedrooms(&mut the_house, extra_bedrooms);
        print_the_house();
    }
}

fn parse_val(s: *const ::core::ffi::c_char, val: &mut ::core::ffi::c_int) -> bool {
    if s.is_null() {
        return false;
    }

    let c_str = unsafe { CStr::from_ptr(s) };
    let s = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

    let parsed = match s.parse::<::core::ffi::c_int>() {
        Ok(n) => n,
        Err(_) => return false,
    };

    *val = parsed;
    true
}

fn main_0() -> i32 {
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);

    let mut x: ::core::ffi::c_int = 0;
    let mut bytes: Vec<::core::ffi::c_char> = input.bytes().map(|b| b as ::core::ffi::c_char).collect();
    bytes.push(0);

    let parsed = parse_val(bytes.as_ptr(), &mut x);
    if parsed {
        run(x);
        run(x);
    } else {
        println!("An error occurred");
    }

    0
}

pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
pub const INT_MAX: ::core::ffi::c_int = __INT_MAX__;
pub const INT_MIN: ::core::ffi::c_int = -__INT_MAX__ - 1 as ::core::ffi::c_int;
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub fn main() {
    std::process::exit(main_0())
}

