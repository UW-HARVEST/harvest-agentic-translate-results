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
}
#[no_mangle]
pub unsafe extern "C" fn static_alias(
    mut outer: *mut ::core::ffi::c_int,
) -> *mut ::core::ffi::c_int {
    static mut inner: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    if *outer >= inner {
        inner += *outer;
        return &raw mut inner;
    } else {
        *outer += inner;
        return outer;
    };
}
unsafe fn main_0(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if argc != 3 as ::core::ffi::c_int {
        printf(
            b"Error: should only be two (integer) arguments!\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        return 1 as ::core::ffi::c_int;
    }
    let mut end: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut initial_value: ::core::ffi::c_int = strtol(
        *argv.offset(1 as ::core::ffi::c_int as isize),
        &raw mut end,
        10 as ::core::ffi::c_int,
    ) as ::core::ffi::c_int;
    if end == *argv.offset(1 as ::core::ffi::c_int as isize) {
        printf(
            b"Error: first argument must be an integer!\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        return 1 as ::core::ffi::c_int;
    }
    let mut iterations: ::core::ffi::c_int = strtol(
        *argv.offset(2 as ::core::ffi::c_int as isize),
        &raw mut end,
        10 as ::core::ffi::c_int,
    ) as ::core::ffi::c_int;
    if end == *argv.offset(2 as ::core::ffi::c_int as isize) {
        printf(
            b"Error: second argument must be an integer!\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        return 1 as ::core::ffi::c_int;
    }
    let mut running_sum: *mut ::core::ffi::c_int = &raw mut initial_value;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < iterations {
        running_sum = static_alias(running_sum);
        printf(
            b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
            *running_sum,
        );
        i += 1;
    }
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
