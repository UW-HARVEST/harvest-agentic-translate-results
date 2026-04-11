extern "C" {
    fn sqrtf(__x: libc::c_float) -> libc::c_float;
    fn memset(
        __s: *mut libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
}
pub type size_t = usize;
#[no_mangle]
pub unsafe extern "C" fn normalize(
    mut dest: *mut libc::c_float,
    mut src: *const libc::c_float,
    mut size: libc::c_int,
) {
    let mut sum: libc::c_float = 0.0f32;
    let mut i: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < size {
        sum += *src.offset(i as isize) * *src.offset(i as isize);
        i += 1;
    }
    if sum > 0.0f32 {
        sum = 1.0f32 / sqrtf(sum);
        i = 0 as libc::c_int;
        while i < size {
            *dest.offset(i as isize) = *src.offset(i as isize) * sum;
            i += 1;
        }
    } else if dest != src as *mut libc::c_float {
        memset(
            dest as *mut libc::c_void,
            0 as libc::c_int,
            (size as size_t).wrapping_mul(std::mem::size_of::<libc::c_float>() as size_t),
        );
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

