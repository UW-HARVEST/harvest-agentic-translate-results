
extern "C" {
    fn strrchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
}
#[no_mangle]
pub fn tool_basename(path: &str) -> &str {
    match (path.rfind('/'), path.rfind('\\')) {
        (Some(s1), Some(s2)) => &path[usize::max(s1, s2) + 1..],
        (Some(s1), None) => &path[s1 + 1..],
        (None, Some(s2)) => &path[s2 + 1..],
        (None, None) => path,
    }
}

