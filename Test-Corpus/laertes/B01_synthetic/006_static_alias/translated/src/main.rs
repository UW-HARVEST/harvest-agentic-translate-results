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
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn strtol(
        __nptr: *const libc::c_char,
        __endptr: *mut *mut libc::c_char,
        __base: libc::c_int,
    ) -> libc::c_long;
}
#[no_mangle]
pub unsafe extern "C" fn static_alias(
    mut outer: *mut libc::c_int,
) -> *mut libc::c_int {
    static mut inner: libc::c_int = 1 as libc::c_int;
    if *outer >= inner {
        inner += *outer;
        return &raw mut inner;
    } else {
        *outer += inner;
        return outer;
    };
}
unsafe fn main_0(
    mut argc: libc::c_int,
    mut argv: *mut *mut libc::c_char,
) -> libc::c_int {
    if argc != 3 as libc::c_int {
        printf(
            b"Error: should only be two (integer) arguments!\n\0" as *const u8
                as *const libc::c_char,
        );
        return 1 as libc::c_int;
    }
    let mut end: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut initial_value: libc::c_int = strtol(
        *argv.offset(1 as libc::c_int as isize),
        &raw mut end,
        10 as libc::c_int,
    ) as libc::c_int;
    if end == *argv.offset(1 as libc::c_int as isize) {
        printf(
            b"Error: first argument must be an integer!\n\0" as *const u8
                as *const libc::c_char,
        );
        return 1 as libc::c_int;
    }
    let mut iterations: libc::c_int = strtol(
        *argv.offset(2 as libc::c_int as isize),
        &raw mut end,
        10 as libc::c_int,
    ) as libc::c_int;
    if end == *argv.offset(2 as libc::c_int as isize) {
        printf(
            b"Error: second argument must be an integer!\n\0" as *const u8
                as *const libc::c_char,
        );
        return 1 as libc::c_int;
    }
    let mut running_sum: *mut libc::c_int = &raw mut initial_value;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < iterations {
        running_sum = static_alias(running_sum);
        printf(
            b"%d\n\0" as *const u8 as *const libc::c_char,
            *running_sum,
        );
        i += 1;
    }
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
