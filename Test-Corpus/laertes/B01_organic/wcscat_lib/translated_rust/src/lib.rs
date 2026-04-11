pub type size_t = usize;
pub type wchar_t = libc::unix::linux_like::linux::gnu::b64::x86_64::wchar_t;
#[no_mangle]
pub unsafe extern "C" fn wcscat(
    mut dst: *mut wchar_t,
    mut numElem: size_t,
    mut src: *const wchar_t,
) -> libc::c_int {
    let mut ptr: *mut wchar_t = dst;
    if dst.is_null() || numElem == 0 as size_t {
        return 22 as libc::c_int;
    }
    if src.is_null() {
        *dst.offset(0 as libc::c_int as isize) = 0 as libc::c_int as wchar_t;
        return 22 as libc::c_int;
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
            return 0 as libc::c_int;
        }
    }
    *dst.offset(0 as libc::c_int as isize) = 0 as libc::c_int as wchar_t;
    return 34 as libc::c_int;
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

