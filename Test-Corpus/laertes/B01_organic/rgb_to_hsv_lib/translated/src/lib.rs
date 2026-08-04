#[no_mangle]
pub unsafe extern "C" fn rgb_to_hsv(
    mut dest: *mut libc::c_float,
    mut src: *const libc::c_float,
) {
    let mut r: libc::c_float = *src.offset(0 as libc::c_int as isize);
    let mut g: libc::c_float = *src.offset(1 as libc::c_int as isize);
    let mut b: libc::c_float = *src.offset(2 as libc::c_int as isize);
    let mut h: libc::c_float = 0 as libc::c_int as libc::c_float;
    let mut s: libc::c_float = 0 as libc::c_int as libc::c_float;
    let mut v: libc::c_float = 0 as libc::c_int as libc::c_float;
    let mut min: libc::c_float = r;
    let mut max: libc::c_float = r;
    let mut delta: libc::c_float = 0.;
    min = if min < g { min } else { g };
    min = if min < b { min } else { b };
    max = if max > g { max } else { g };
    max = if max > b { max } else { b };
    delta = max - min;
    v = max;
    if delta == 0 as libc::c_int as libc::c_float
        || max == 0 as libc::c_int as libc::c_float
    {
        *dest.offset(0 as libc::c_int as isize) = h;
        *dest.offset(1 as libc::c_int as isize) = s;
        *dest.offset(2 as libc::c_int as isize) = v;
        return;
    }
    s = delta / max;
    if r == max {
        h = (g - b) / delta;
    } else if g == max {
        h = 2 as libc::c_int as libc::c_float + (b - r) / delta;
    } else {
        h = 4 as libc::c_int as libc::c_float + (r - g) / delta;
    }
    h *= 60 as libc::c_int as libc::c_float;
    if h < 0 as libc::c_int as libc::c_float {
        h += 360 as libc::c_int as libc::c_float;
    }
    *dest.offset(0 as libc::c_int as isize) = h;
    *dest.offset(1 as libc::c_int as isize) = s;
    *dest.offset(2 as libc::c_int as isize) = v;
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

