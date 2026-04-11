pub type __uint32_t = u32;
pub type uint32_t = u32;
#[no_mangle]
pub extern "C" fn rev16(mut a: uint32_t) -> uint32_t {
    a = (a & 0xaaaa as uint32_t) >> 1 as libc::c_int
        | (a & 0x5555 as uint32_t) << 1 as libc::c_int;
    a = (a & 0xcccc as uint32_t) >> 2 as libc::c_int
        | (a & 0x3333 as uint32_t) << 2 as libc::c_int;
    a = (a & 0xf0f0 as uint32_t) >> 4 as libc::c_int
        | (a & 0xf0f as uint32_t) << 4 as libc::c_int;
    a = (a & 0xff00 as uint32_t) >> 8 as libc::c_int
        | (a & 0xff as uint32_t) << 8 as libc::c_int;
    return a;
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

