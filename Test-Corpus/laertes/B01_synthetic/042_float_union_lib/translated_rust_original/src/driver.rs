extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
}
pub type uint64_t = u64;
pub type __uint64_t = u64;
#[derive(Copy, Clone)]
#[repr(C)]
pub union raw_double_t {
    pub x: uint64_t,
    pub f: libc::c_double,
}
#[no_mangle]
pub unsafe extern "C" fn driver(mut f: libc::c_double) {
    let mut u: raw_double_t = raw_double_t { f: f };
    printf(
        b"%llx %a %.4f\n\0" as *const u8 as *const libc::c_char,
        u.x,
        f,
        f,
    );
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

