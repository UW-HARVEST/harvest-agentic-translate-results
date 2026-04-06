pub type size_t = usize;
pub type wchar_t = ::libc::wchar_t;
#[no_mangle]
pub unsafe extern "C" fn wcscat(
    mut dst: *mut wchar_t,
    mut numElem: size_t,
    mut src: *const wchar_t,
) -> ::core::ffi::c_int {
    let mut ptr: *mut wchar_t = dst;
    if dst.is_null() || numElem == 0 as size_t {
        return 22 as ::core::ffi::c_int;
    }
    if src.is_null() {
        *dst.offset(0 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_int as wchar_t;
        return 22 as ::core::ffi::c_int;
    }
    while ptr < dst.offset(numElem as isize) && *ptr != 0 as wchar_t {
        ptr = ptr.offset(1);
    }
    while ptr < dst.offset(numElem as isize) {
        let fresh0 = src;
        src = src.offset(1);
        let fresh1 = ptr;
        ptr = ptr.offset(1);
        *fresh1 = *fresh0;
        if *fresh1 == 0 as wchar_t {
            return 0 as ::core::ffi::c_int;
        }
    }
    *dst.offset(0 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_int as wchar_t;
    return 34 as ::core::ffi::c_int;
}
