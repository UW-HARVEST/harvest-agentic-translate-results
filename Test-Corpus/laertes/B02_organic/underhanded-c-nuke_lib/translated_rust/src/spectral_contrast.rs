extern "C" {
    fn sqrt(__x: libc::c_double) -> libc::c_double;
}
pub use crate::src::match::float_t;
unsafe extern "C" fn dot_product(
    mut a: *mut float_t,
    mut b: *mut float_t,
    mut length: libc::c_int,
) -> libc::c_double {
    let mut sum: libc::c_double = 0 as libc::c_int as libc::c_double;
    let mut i: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < length {
        sum += (*a.offset(i as isize) * *b.offset(i as isize)) as libc::c_double;
        i += 1;
    }
    return sum;
}
unsafe extern "C" fn normalize(mut v: *mut float_t, mut length: libc::c_int) {
    let mut magnitude: libc::c_double = sqrt(dot_product(v, v, length));
    let mut i: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < length {
        let ref mut fresh0 = *v.offset(i as isize);
        *fresh0 = (*fresh0 as libc::c_double / magnitude) as float_t;
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn spectral_contrast(
    mut a: *mut float_t,
    mut b: *mut float_t,
    mut length: libc::c_int,
) -> libc::c_double {
    normalize(a, length);
    normalize(b, length);
    return dot_product(a, b, length);
}
