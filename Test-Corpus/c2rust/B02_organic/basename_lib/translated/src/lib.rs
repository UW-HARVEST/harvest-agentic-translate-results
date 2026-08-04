extern "C" {
    fn strrchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn tool_basename(
    mut path: *mut ::core::ffi::c_char,
) -> *mut ::core::ffi::c_char {
    let mut s1: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut s2: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    s1 = strrchr(path, '/' as i32);
    s2 = strrchr(path, '\\' as i32);
    if !s1.is_null() && !s2.is_null() {
        path = if s1 > s2 {
            s1.offset(1 as ::core::ffi::c_int as isize)
        } else {
            s2.offset(1 as ::core::ffi::c_int as isize)
        };
    } else if !s1.is_null() {
        path = s1.offset(1 as ::core::ffi::c_int as isize);
    } else if !s2.is_null() {
        path = s2.offset(1 as ::core::ffi::c_int as isize);
    }
    return path;
}
