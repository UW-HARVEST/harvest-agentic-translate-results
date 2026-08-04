extern "C" {
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn strlen(__s: *const libc::c_char) -> size_t;
}
pub type size_t = usize;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn custom_strdup(
    mut str: *const libc::c_char,
) -> *mut libc::c_char {
    let mut len: size_t = 0;
    let mut newstr: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    if str.is_null() {
        return NULL as *mut libc::c_char;
    }
    len = strlen(str).wrapping_add(1 as size_t);
    newstr = malloc(len) as *mut libc::c_char;
    if newstr.is_null() {
        return NULL as *mut libc::c_char;
    }
    memcpy(
        newstr as *mut libc::c_void,
        str as *const libc::c_void,
        len,
    );
    return newstr;
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

