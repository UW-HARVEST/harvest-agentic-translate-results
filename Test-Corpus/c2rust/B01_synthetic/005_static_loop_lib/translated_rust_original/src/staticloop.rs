extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn static_sum(mut update: ::core::ffi::c_int) -> ::core::ffi::c_int {
    static mut sum: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    sum += update;
    return sum;
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut stride: ::core::ffi::c_int) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 10 as ::core::ffi::c_int {
        printf(
            b"%d\n\0" as *const u8 as *const ::core::ffi::c_char,
            static_sum(i * stride),
        );
        i += 1;
    }
}
