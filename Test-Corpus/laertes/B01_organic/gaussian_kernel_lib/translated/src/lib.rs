extern "C" {
    fn expf(__x: libc::c_float) -> libc::c_float;
}
#[no_mangle]
pub unsafe extern "C" fn gaussian_kernel(
    mut dest: *mut libc::c_float,
    mut size: libc::c_int,
    mut radius: libc::c_float,
) {
    let mut k: *mut libc::c_float = std::ptr::null_mut::<libc::c_float>();
    let mut rs: libc::c_float = 0.;
    let mut s2: libc::c_float = 0.;
    let mut sum: libc::c_float = 0.;
    let mut sigma: libc::c_float = 1.6f32;
    let mut tetha: libc::c_float = 2.25f32;
    let mut r: libc::c_int = 0;
    let mut hsize: libc::c_int = size / 2 as libc::c_int;
    s2 = 1.0f32 / expf(sigma * sigma * tetha);
    rs = sigma / radius;
    k = dest;
    sum = 0.0f32;
    r = -hsize;
    while r <= hsize {
        let mut x: libc::c_float = r as libc::c_float * rs;
        let mut v: libc::c_float = 1.0f32 / expf(x * x) - s2;
        v = if v > 0 as libc::c_int as libc::c_float {
            v
        } else {
            0 as libc::c_int as libc::c_float
        };
        *k = v;
        sum += v;
        k = k.offset(1);
        r += 1;
    }
    if sum > 0.0f32 {
        let mut isum: libc::c_float = 1.0f32 / sum;
        r = 0 as libc::c_int;
        while r < size {
            *dest.offset(r as isize) *= isum;
            r += 1;
        }
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

