extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn sieve(mut val: ::core::ffi::c_int) {
    loop {
        printf(b"%d\n\0" as *const u8 as *const ::core::ffi::c_char, val);
        if val % 10 as ::core::ffi::c_int == 9 as ::core::ffi::c_int {
            break;
        }
        val += 1;
    }
}
