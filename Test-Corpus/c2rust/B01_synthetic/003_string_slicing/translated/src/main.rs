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
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn strtol(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
unsafe fn main_0(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if argc > 4 as ::core::ffi::c_int || argc == 1 as ::core::ffi::c_int {
        printf(
            b"Error: there should be one to three arguments passed:\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        printf(b"<string> [start] [stop]\n\0" as *const u8 as *const ::core::ffi::c_char);
        return 1 as ::core::ffi::c_int;
    }
    let mut len: size_t = strlen(*argv.offset(1 as ::core::ffi::c_int as isize));
    let mut start: ::core::ffi::c_int = 0;
    let mut stop: ::core::ffi::c_int = 0;
    let mut end: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    if argc >= 3 as ::core::ffi::c_int {
        start = strtol(
            *argv.offset(2 as ::core::ffi::c_int as isize),
            &raw mut end,
            10 as ::core::ffi::c_int,
        ) as ::core::ffi::c_int;
        if end == *argv.offset(2 as ::core::ffi::c_int as isize) {
            printf(
                b"Second argument must be an integer!\0" as *const u8 as *const ::core::ffi::c_char,
            );
            return 1 as ::core::ffi::c_int;
        }
        if start as size_t > len {
            printf(
                b"Error: start is off the end of the string!\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            return 1 as ::core::ffi::c_int;
        }
    } else {
        start = 0 as ::core::ffi::c_int;
    }
    if argc == 4 as ::core::ffi::c_int {
        stop = strtol(
            *argv.offset(3 as ::core::ffi::c_int as isize),
            ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
            10 as ::core::ffi::c_int,
        ) as ::core::ffi::c_int;
        if end == *argv.offset(3 as ::core::ffi::c_int as isize) {
            printf(
                b"Third argument must be an integer!\0" as *const u8 as *const ::core::ffi::c_char,
            );
            return 1 as ::core::ffi::c_int;
        }
        if stop as size_t > len {
            printf(
                b"Error: stop is off the end of the string!\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            return 1 as ::core::ffi::c_int;
        }
        if stop <= start {
            printf(
                b"Error: stop must come after start!\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            return 1 as ::core::ffi::c_int;
        }
    } else {
        stop = len as ::core::ffi::c_int;
    }
    printf(
        b"%.*s\n\0" as *const u8 as *const ::core::ffi::c_char,
        stop - start,
        (*argv.offset(1 as ::core::ffi::c_int as isize)).offset(start as isize),
    );
    return 0 as ::core::ffi::c_int;
}
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
