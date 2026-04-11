extern "C" {
    fn abort() -> !;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type uint8_t = u8;
#[no_mangle]
pub unsafe extern "C" fn bin2hex(
    mut hex: *mut libc::c_char,
    mut hex_maxlen: size_t,
    mut bin: *const uint8_t,
    mut bin_len: size_t,
) -> *mut libc::c_char {
    let mut i: size_t = 0 as libc::c_uint as size_t;
    let mut x: libc::c_uint = 0;
    let mut b: libc::c_int = 0;
    let mut c: libc::c_int = 0;
    if bin_len >= (18446744073709551615 as size_t).wrapping_div(2 as size_t)
        || hex_maxlen <= bin_len.wrapping_mul(2 as size_t)
    {
        abort();
    }
    while i < bin_len {
        c = *bin.offset(i as isize) as libc::c_int & 0xf as libc::c_int;
        b = *bin.offset(i as isize) as libc::c_int >> 4 as libc::c_int;
        x = (((87 as libc::c_uint)
            .wrapping_add(c as libc::c_uint)
            .wrapping_add(
                (c as libc::c_uint).wrapping_sub(10 as libc::c_uint)
                    >> 8 as libc::c_int
                    & !(38 as libc::c_uint),
            ) as libc::c_uchar as libc::c_int)
            << 8 as libc::c_int
            | (87 as libc::c_uint)
                .wrapping_add(b as libc::c_uint)
                .wrapping_add(
                    (b as libc::c_uint).wrapping_sub(10 as libc::c_uint)
                        >> 8 as libc::c_int
                        & !(38 as libc::c_uint),
                ) as libc::c_uchar as libc::c_int) as libc::c_uint;
        *hex.offset(i.wrapping_mul(2 as size_t) as isize) = x as libc::c_char;
        x >>= 8 as libc::c_int;
        *hex.offset(i.wrapping_mul(2 as size_t).wrapping_add(1 as size_t) as isize) =
            x as libc::c_char;
        i = i.wrapping_add(1);
    }
    *hex.offset(i.wrapping_mul(2 as size_t) as isize) = 0 as libc::c_char;
    return hex;
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

