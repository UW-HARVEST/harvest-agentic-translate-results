extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn strchr(__s: *const ::core::ffi::c_char, __c: ::core::ffi::c_int)
        -> *mut ::core::ffi::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn foo(
    mut in_0: *const ::core::ffi::c_char,
    mut c: ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut res: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut s: *const ::core::ffi::c_char = in_0;
    loop {
        s = strchr(s, c as ::core::ffi::c_int);
        if s.is_null() {
            break;
        }
        res += 1;
        s = s.offset(1);
    }
    return res;
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut in_0: *const ::core::ffi::c_char) {
    printf(
        b"A: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        foo(in_0, 'A' as i32 as ::core::ffi::c_char),
    );
    printf(
        b"x: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        foo(in_0, 'x' as i32 as ::core::ffi::c_char),
    );
}
