#[no_mangle]
pub extern "C" fn div_euclid(
    mut v1: libc::c_int,
    mut v2: libc::c_int,
) -> libc::c_int {
    if v2 == 0 as libc::c_int {
        return 0 as libc::c_int;
    }
    let mut q: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    if v1 >= 0 as libc::c_int {
        if v2 >= 0 as libc::c_int {
            return v1 / v2;
        } else if v2 != -(0x7fffffff as libc::c_int) - 1 as libc::c_int {
            q = -(v1 / -v2);
            r = v1 % -v2;
        } else {
            q = 0 as libc::c_int;
            r = v1;
        }
    } else if v1 != -(0x7fffffff as libc::c_int) - 1 as libc::c_int {
        if v2 >= 0 as libc::c_int {
            q = -(-v1 / v2);
            r = -(-v1 % v2);
        } else if v2 != -(0x7fffffff as libc::c_int) - 1 as libc::c_int {
            q = -v1 / -v2;
            r = -(-v1 % -v2);
        } else {
            q = 1 as libc::c_int;
            r = v1 - q * v2;
        }
    } else if v2 >= 0 as libc::c_int {
        q = -(-(v1 + v2) / v2) - 1 as libc::c_int;
        r = -(-(v1 + v2) % v2);
    } else if v2 != -(0x7fffffff as libc::c_int) - 1 as libc::c_int {
        q = -(v1 - v2) / -v2 + 1 as libc::c_int;
        r = -(-(v1 - v2) % -v2);
    } else {
        q = 1 as libc::c_int;
        r = 0 as libc::c_int;
    }
    if r >= 0 as libc::c_int {
        return q;
    } else {
        return q
            + (if v2 > 0 as libc::c_int {
                -(1 as libc::c_int)
            } else {
                1 as libc::c_int
            });
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

