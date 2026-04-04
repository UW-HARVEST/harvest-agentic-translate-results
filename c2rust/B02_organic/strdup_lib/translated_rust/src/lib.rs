extern "C" {
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub unsafe extern "C" fn custom_strdup(
    mut str: *const ::core::ffi::c_char,
) -> *mut ::core::ffi::c_char {
    let mut len: size_t = 0;
    let mut newstr: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    if str.is_null() {
        return NULL as *mut ::core::ffi::c_char;
    }
    len = strlen(str).wrapping_add(1 as size_t);
    newstr = malloc(len) as *mut ::core::ffi::c_char;
    if newstr.is_null() {
        return NULL as *mut ::core::ffi::c_char;
    }
    memcpy(
        newstr as *mut ::core::ffi::c_void,
        str as *const ::core::ffi::c_void,
        len,
    );
    return newstr;
}
