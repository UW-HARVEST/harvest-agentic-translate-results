pub type __uint8_t = u8;
pub type uint8_t = u8;
unsafe extern "C" fn hdr_valid(mut h: *const uint8_t) -> libc::c_int {
    return (*h.offset(0 as libc::c_int as isize) as libc::c_int
        == 0xff as libc::c_int
        && (*h.offset(1 as libc::c_int as isize) as libc::c_int
            & 0xf0 as libc::c_int
            == 0xf0 as libc::c_int
            || *h.offset(1 as libc::c_int as isize) as libc::c_int
                & 0xfe as libc::c_int
                == 0xe2 as libc::c_int)
        && *h.offset(1 as libc::c_int as isize) as libc::c_int
            >> 1 as libc::c_int
            & 3 as libc::c_int
            != 0 as libc::c_int
        && *h.offset(2 as libc::c_int as isize) as libc::c_int
            >> 4 as libc::c_int
            != 15 as libc::c_int
        && *h.offset(2 as libc::c_int as isize) as libc::c_int
            >> 2 as libc::c_int
            & 3 as libc::c_int
            != 3 as libc::c_int) as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn hdr_compare(
    mut h1: *const uint8_t,
    mut h2: *const uint8_t,
) -> libc::c_int {
    return (hdr_valid(h2) != 0
        && (*h1.offset(1 as libc::c_int as isize) as libc::c_int
            ^ *h2.offset(1 as libc::c_int as isize) as libc::c_int)
            & 0xfe as libc::c_int
            == 0 as libc::c_int
        && (*h1.offset(2 as libc::c_int as isize) as libc::c_int
            ^ *h2.offset(2 as libc::c_int as isize) as libc::c_int)
            & 0xc as libc::c_int
            == 0 as libc::c_int
        && (*h1.offset(2 as libc::c_int as isize) as libc::c_int
            & 0xf0 as libc::c_int
            == 0 as libc::c_int) as libc::c_int
            ^ (*h2.offset(2 as libc::c_int as isize) as libc::c_int
                & 0xf0 as libc::c_int
                == 0 as libc::c_int) as libc::c_int
            == 0) as libc::c_int;
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

