extern "C" {
    fn strrchr(
        __s: *const libc::c_char,
        __c: libc::c_int,
    ) -> *mut libc::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn tool_basename(
    mut path: *mut libc::c_char,
) -> *mut libc::c_char {
    let mut s1: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut s2: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    s1 = strrchr(path, '/' as i32);
    s2 = strrchr(path, '\\' as i32);
    if !s1.is_null() && !s2.is_null() {
        path = if s1 > s2 {
            s1.offset(1 as libc::c_int as isize)
        } else {
            s2.offset(1 as libc::c_int as isize)
        };
    } else if !s1.is_null() {
        path = s1.offset(1 as libc::c_int as isize);
    } else if !s2.is_null() {
        path = s2.offset(1 as libc::c_int as isize);
    }
    return path;
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

