extern "C" {
    fn fabsf(__x: libc::c_float) -> libc::c_float;
    fn fmodf(__x: libc::c_float, __y: libc::c_float) -> libc::c_float;
}
#[no_mangle]
pub unsafe extern "C" fn hsl_to_rgb(
    mut dest: *mut libc::c_float,
    mut src: *const libc::c_float,
) {
    let mut h: libc::c_float = *src.offset(0 as libc::c_int as isize);
    let mut s: libc::c_float = *src.offset(1 as libc::c_int as isize);
    let mut l: libc::c_float = *src.offset(2 as libc::c_int as isize);
    let mut c: libc::c_float = 0.;
    let mut m: libc::c_float = 0.;
    let mut x: libc::c_float = 0.;
    if s == 0 as libc::c_int as libc::c_float {
        *dest.offset(0 as libc::c_int as isize) = l;
        *dest.offset(1 as libc::c_int as isize) = l;
        *dest.offset(2 as libc::c_int as isize) = l;
        return;
    }
    c = (1.0f32 - fabsf(2.0f32 * l - 1.0f32)) * s;
    m = 1.0f32 * (l - 0.5f32 * c);
    x = c
        * (1.0f32
            - fabsf(fmodf(h / 60.0f32, 2 as libc::c_int as libc::c_float) - 1.0f32));
    if h >= 0.0f32 && h < 60.0f32 {
        *dest.offset(0 as libc::c_int as isize) = c + m;
        *dest.offset(1 as libc::c_int as isize) = x + m;
        *dest.offset(2 as libc::c_int as isize) = m;
    } else if h >= 60.0f32 && h < 120.0f32 {
        *dest.offset(0 as libc::c_int as isize) = x + m;
        *dest.offset(1 as libc::c_int as isize) = c + m;
        *dest.offset(2 as libc::c_int as isize) = m;
    } else if h < 120.0f32 && h < 180.0f32 {
        *dest.offset(0 as libc::c_int as isize) = m;
        *dest.offset(1 as libc::c_int as isize) = c + m;
        *dest.offset(2 as libc::c_int as isize) = x + m;
    } else if h >= 180.0f32 && h < 240.0f32 {
        *dest.offset(0 as libc::c_int as isize) = m;
        *dest.offset(1 as libc::c_int as isize) = x + m;
        *dest.offset(2 as libc::c_int as isize) = c + m;
    } else if h >= 240.0f32 && h < 300.0f32 {
        *dest.offset(0 as libc::c_int as isize) = x + m;
        *dest.offset(1 as libc::c_int as isize) = m;
        *dest.offset(2 as libc::c_int as isize) = c + m;
    } else if h >= 300.0f32 && h < 360.0f32 {
        *dest.offset(0 as libc::c_int as isize) = c + m;
        *dest.offset(1 as libc::c_int as isize) = m;
        *dest.offset(2 as libc::c_int as isize) = x + m;
    } else {
        *dest.offset(0 as libc::c_int as isize) = m;
        *dest.offset(1 as libc::c_int as isize) = m;
        *dest.offset(2 as libc::c_int as isize) = m;
    };
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

