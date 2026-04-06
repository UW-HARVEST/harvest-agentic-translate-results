extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
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
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut initial_value: ::core::ffi::c_int,
    mut iterations: ::core::ffi::c_int,
) {
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
}
