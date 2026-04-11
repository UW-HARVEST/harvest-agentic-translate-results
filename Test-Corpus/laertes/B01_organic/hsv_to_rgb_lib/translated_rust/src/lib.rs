extern "C" {
    fn floorf(__x: libc::c_float) -> libc::c_float;
}
#[no_mangle]
pub unsafe extern "C" fn hsv_to_rgb(
    mut dest: *mut libc::c_float,
    mut src: *const libc::c_float,
) {
    let mut r: libc::c_float = 0.;
    let mut g: libc::c_float = 0.;
    let mut b: libc::c_float = 0.;
    let mut f: libc::c_float = 0.;
    let mut p: libc::c_float = 0.;
    let mut q: libc::c_float = 0.;
    let mut t: libc::c_float = 0.;
    let mut h: libc::c_float = *src.offset(0 as libc::c_int as isize);
    let mut s: libc::c_float = *src.offset(1 as libc::c_int as isize);
    let mut v: libc::c_float = *src.offset(2 as libc::c_int as isize);
    let mut i: libc::c_int = 0;
    if s == 0 as libc::c_int as libc::c_float {
        *dest.offset(0 as libc::c_int as isize) = v;
        *dest.offset(1 as libc::c_int as isize) = v;
        *dest.offset(2 as libc::c_int as isize) = v;
        return;
    }
    h /= 60.0f32;
    i = floorf(h) as libc::c_int;
    f = h - i as libc::c_float;
    p = v * (1 as libc::c_int as libc::c_float - s);
    q = v * (1 as libc::c_int as libc::c_float - s * f);
    t = v
        * (1 as libc::c_int as libc::c_float
            - s * (1 as libc::c_int as libc::c_float - f));
    match i {
        0 => {
            r = v;
            g = t;
            b = p;
        }
        1 => {
            r = q;
            g = v;
            b = p;
        }
        2 => {
            r = p;
            g = v;
            b = t;
        }
        3 => {
            r = p;
            g = q;
            b = v;
        }
        4 => {
            r = t;
            g = p;
            b = v;
        }
        _ => {
            r = v;
            g = p;
            b = q;
        }
    }
    *dest.offset(0 as libc::c_int as isize) = r;
    *dest.offset(1 as libc::c_int as isize) = g;
    *dest.offset(2 as libc::c_int as isize) = b;
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

