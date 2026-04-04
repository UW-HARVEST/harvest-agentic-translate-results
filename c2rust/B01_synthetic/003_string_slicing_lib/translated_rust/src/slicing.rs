extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
#[no_mangle]
pub unsafe extern "C" fn slice(
    mut mystr: *mut ::core::ffi::c_char,
    mut start_ptr: *mut ::core::ffi::c_int,
    mut stop_ptr: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut len: size_t = strlen(mystr);
    let mut end: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut start: ::core::ffi::c_int = 0;
    let mut stop: ::core::ffi::c_int = 0;
    if !start_ptr.is_null() {
        start = *start_ptr;
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
    if !stop_ptr.is_null() {
        stop = *stop_ptr;
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
        mystr.offset(start as isize),
    );
    return 0 as ::core::ffi::c_int;
}
