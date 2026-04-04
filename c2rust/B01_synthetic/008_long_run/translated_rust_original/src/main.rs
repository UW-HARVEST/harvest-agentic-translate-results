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
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn strtoul(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_ulong;
    fn rand() -> ::core::ffi::c_int;
    fn srand(__seed: ::core::ffi::c_uint);
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
pub const ARRAY_SIZE: ::core::ffi::c_int = 256 as ::core::ffi::c_int * 1024 as ::core::ffi::c_int;
pub const ITERATIONS: ::core::ffi::c_int = 2000 as ::core::ffi::c_int;
#[no_mangle]
pub static mut array: [::core::ffi::c_int; 262144] = [0; 262144];
#[no_mangle]
pub unsafe extern "C" fn perform_expensive_operations() {
    let mut i: size_t = 0 as size_t;
    while i < ARRAY_SIZE as size_t {
        let mut x: ::core::ffi::c_int = array[i as usize];
        let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while j < 100 as ::core::ffi::c_int {
            x = x * 3 as ::core::ffi::c_int + 7 as ::core::ffi::c_int;
            x = x ^ x >> 3 as ::core::ffi::c_int;
            x = x - (x << 1 as ::core::ffi::c_int);
            x = x / 2 as ::core::ffi::c_int + x % 7 as ::core::ffi::c_int;
            j += 1;
        }
        array[i as usize] = x;
        i = i.wrapping_add(1);
    }
}
unsafe fn main_0(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if argc != 2 as ::core::ffi::c_int {
        fprintf(
            stderr as *mut FILE,
            b"Usage: %s <seed>\n\0" as *const u8 as *const ::core::ffi::c_char,
            *argv.offset(0 as ::core::ffi::c_int as isize),
        );
        return 1 as ::core::ffi::c_int;
    }
    *__errno_location() = 0 as ::core::ffi::c_int;
    let mut endptr: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut temp_seed: ::core::ffi::c_ulong = strtoul(
        *argv.offset(1 as ::core::ffi::c_int as isize),
        &raw mut endptr,
        10 as ::core::ffi::c_int,
    );
    if *endptr as ::core::ffi::c_int != '\0' as i32
        || *__errno_location() != 0 as ::core::ffi::c_int
        || temp_seed > UINT_MAX as ::core::ffi::c_ulong
    {
        fprintf(
            stderr as *mut FILE,
            b"Invalid seed: '%s'\n\0" as *const u8 as *const ::core::ffi::c_char,
            *argv.offset(1 as ::core::ffi::c_int as isize),
        );
        return 1 as ::core::ffi::c_int;
    }
    let mut seed: ::core::ffi::c_uint = temp_seed as ::core::ffi::c_uint;
    srand(seed);
    let mut i: size_t = 0 as size_t;
    while i < ARRAY_SIZE as size_t {
        array[i as usize] = rand();
        i = i.wrapping_add(1);
    }
    let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_0 < ITERATIONS {
        perform_expensive_operations();
        i_0 += 1;
    }
    let mut xor_result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i_1: size_t = 0 as size_t;
    while i_1 < ARRAY_SIZE as size_t {
        xor_result ^= array[i_1 as usize];
        i_1 = i_1.wrapping_add(1);
    }
    printf(
        b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        xor_result,
    );
    return 0 as ::core::ffi::c_int;
}
pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
pub const UINT_MAX: ::core::ffi::c_uint = (__INT_MAX__ as ::core::ffi::c_uint)
    .wrapping_mul(2 as ::core::ffi::c_uint)
    .wrapping_add(1 as ::core::ffi::c_uint);
pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args()
        .map(|arg| {
            ::std::ffi::CString::new(arg)
                .expect("Failed to convert argument into CString.")
                .into_bytes_with_nul()
        })
        .collect();
    let mut args_ptrs: Vec<*mut ::core::ffi::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut ::core::ffi::c_char)
        .chain(::core::iter::once(::core::ptr::null_mut()))
        .collect();
    unsafe {
        ::std::process::exit(main_0(
            (args_ptrs.len() - 1) as ::core::ffi::c_int,
            args_ptrs.as_mut_ptr() as *mut *mut ::core::ffi::c_char,
        ) as i32)
    }
}
