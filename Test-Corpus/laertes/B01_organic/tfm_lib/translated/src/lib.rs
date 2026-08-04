extern "C" {
    fn sqrtf(__x: libc::c_float) -> libc::c_float;
}
#[no_mangle]
pub unsafe extern "C" fn tfm(
    mut dest: *mut libc::c_float,
    mut src: *const libc::c_float,
    mut count: libc::c_int,
) {
    let mut i: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < count {
        if *src.offset(0 as libc::c_int as isize)
            < *src.offset(1 as libc::c_int as isize)
        {
            let mut dx2: libc::c_float = *src.offset(0 as libc::c_int as isize);
            let mut dy2: libc::c_float = *src.offset(1 as libc::c_int as isize);
            let mut dxy: libc::c_float = *src.offset(2 as libc::c_int as isize);
            let mut sqd: libc::c_float =
                dy2 * dy2 - 2.0f32 * dx2 * dy2 + dx2 * dx2 + 4.0f32 * dxy * dxy;
            let mut lambda: libc::c_float = 0.5f32
                * (dy2
                    + dx2
                    + sqrtf(
                        (if 0 as libc::c_int as libc::c_float > sqd {
                            0 as libc::c_int as libc::c_float
                        } else {
                            sqd
                        }),
                    ));
            *dest.offset(0 as libc::c_int as isize) = dx2 - lambda;
            *dest.offset(1 as libc::c_int as isize) = dxy;
        } else {
            let mut dy2_0: libc::c_float = *src.offset(0 as libc::c_int as isize);
            let mut dx2_0: libc::c_float = *src.offset(1 as libc::c_int as isize);
            let mut dxy_0: libc::c_float = *src.offset(2 as libc::c_int as isize);
            let mut sqd_0: libc::c_float =
                dy2_0 * dy2_0 - 2.0f32 * dx2_0 * dy2_0 + dx2_0 * dx2_0 + 4.0f32 * dxy_0 * dxy_0;
            let mut lambda_0: libc::c_float = 0.5f32
                * (dy2_0
                    + dx2_0
                    + sqrtf(
                        (if 0 as libc::c_int as libc::c_float > sqd_0 {
                            0 as libc::c_int as libc::c_float
                        } else {
                            sqd_0
                        }),
                    ));
            *dest.offset(0 as libc::c_int as isize) = dxy_0;
            *dest.offset(1 as libc::c_int as isize) = dx2_0 - lambda_0;
        }
        src = src.offset(3 as libc::c_int as isize);
        dest = dest.offset(2 as libc::c_int as isize);
        i += 1;
    }
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

