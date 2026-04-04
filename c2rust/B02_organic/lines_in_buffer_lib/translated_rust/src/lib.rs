extern "C" {
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub unsafe extern "C" fn UTIL_createLinePointers(
    mut buffer: *mut ::core::ffi::c_char,
    mut numLines: size_t,
    mut bufferSize: size_t,
) -> *mut *const ::core::ffi::c_char {
    let mut lineIndex: size_t = 0 as size_t;
    let mut pos: size_t = 0 as size_t;
    let bufferPtrs: *mut ::core::ffi::c_void = malloc(
        numLines.wrapping_mul(::core::mem::size_of::<*mut *const ::core::ffi::c_char>() as size_t),
    ) as *mut ::core::ffi::c_void;
    let linePointers: *mut *const ::core::ffi::c_char =
        bufferPtrs as *mut *const ::core::ffi::c_char;
    if bufferPtrs.is_null() {
        return ::core::ptr::null_mut::<*const ::core::ffi::c_char>();
    }
    while lineIndex < numLines && pos < bufferSize {
        let mut len: size_t = 0 as size_t;
        let fresh0 = lineIndex;
        lineIndex = lineIndex.wrapping_add(1);
        let ref mut fresh1 = *linePointers.offset(fresh0 as isize);
        *fresh1 = buffer.offset(pos as isize);
        while pos.wrapping_add(len) < bufferSize
            && *buffer.offset(pos.wrapping_add(len) as isize) as ::core::ffi::c_int != '\0' as i32
        {
            len = len.wrapping_add(1);
        }
        pos = (pos as ::core::ffi::c_ulong).wrapping_add(len as ::core::ffi::c_ulong) as size_t
            as size_t;
        if pos < bufferSize {
            pos = pos.wrapping_add(1);
        }
    }
    if lineIndex != numLines {
        free(bufferPtrs);
        return ::core::ptr::null_mut::<*const ::core::ffi::c_char>();
    }
    return linePointers;
}
