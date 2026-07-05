#[no_mangle]
pub unsafe extern "C" fn target(mut code: ::core::ffi::c_int) -> ::core::ffi::c_int {
    if code < 0 as ::core::ffi::c_int {
        return 7 as ::core::ffi::c_int;
    }
    let mut m: ::core::ffi::c_int = code % 10 as ::core::ffi::c_int;
    if m == 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    if m <= 3 as ::core::ffi::c_int {
        return 1 as ::core::ffi::c_int;
    }
    if m <= 6 as ::core::ffi::c_int {
        return 2 as ::core::ffi::c_int;
    }
    if m == 7 as ::core::ffi::c_int {
        return 3 as ::core::ffi::c_int;
    }
    return 4 as ::core::ffi::c_int;
}
