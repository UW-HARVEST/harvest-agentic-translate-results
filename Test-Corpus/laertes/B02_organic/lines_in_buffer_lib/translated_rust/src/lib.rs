extern "C" {
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
}
pub type size_t = usize;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn UTIL_createLinePointers(
    mut buffer: *mut libc::c_char,
    mut numLines: size_t,
    mut bufferSize: size_t,
) -> *mut *const libc::c_char {
    let mut lineIndex: size_t = 0 as size_t;
    let mut pos: size_t = 0 as size_t;
    let bufferPtrs: *mut libc::c_void = malloc(
        numLines.wrapping_mul(std::mem::size_of::<*mut *const libc::c_char>() as size_t),
    ) as *mut libc::c_void;
    let linePointers: *mut *const libc::c_char =
        bufferPtrs as *mut *const libc::c_char;
    if bufferPtrs.is_null() {
        return std::ptr::null_mut::<*const libc::c_char>();
    }
    while lineIndex < numLines && pos < bufferSize {
        let mut len: size_t = 0 as size_t;
        let fresh0 = lineIndex;
        lineIndex = lineIndex.wrapping_add(1);
        let ref mut fresh1 = *linePointers.offset(fresh0 as isize);
        *fresh1 = buffer.offset(pos as isize);
        while pos.wrapping_add(len) < bufferSize
            && *buffer.offset(pos.wrapping_add(len) as isize) as libc::c_int != '\0' as i32
        {
            len = len.wrapping_add(1);
        }
        pos = (pos as libc::c_ulong).wrapping_add(len as libc::c_ulong) as size_t
            as size_t;
        if pos < bufferSize {
            pos = pos.wrapping_add(1);
        }
    }
    if lineIndex != numLines {
        free(bufferPtrs);
        return std::ptr::null_mut::<*const libc::c_char>();
    }
    return linePointers;
}
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}

