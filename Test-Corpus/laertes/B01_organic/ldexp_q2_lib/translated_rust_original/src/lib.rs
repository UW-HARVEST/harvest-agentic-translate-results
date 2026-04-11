#[no_mangle]
pub unsafe extern "C" fn ldexp_q2(
    mut y: libc::c_float,
    mut exp_q2: libc::c_int,
) -> libc::c_float {
    static mut g_expfrac: [libc::c_float; 4] = [
        9.31322575e-10f32,
        7.83145814e-10f32,
        6.58544508e-10f32,
        5.53767716e-10f32,
    ];
    let mut e: libc::c_int = 0;
    loop {
        e = if 30 as libc::c_int * 4 as libc::c_int > exp_q2 {
            exp_q2
        } else {
            30 as libc::c_int * 4 as libc::c_int
        };
        y *= g_expfrac[(e & 3 as libc::c_int) as usize]
            * ((1 as libc::c_int) << 30 as libc::c_int
                >> (e >> 2 as libc::c_int)) as libc::c_float;
        exp_q2 -= e;
        if !(exp_q2 > 0 as libc::c_int) {
            break;
        }
    }
    return y;
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

