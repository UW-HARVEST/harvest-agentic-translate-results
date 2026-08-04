extern "C" {
    fn sqrt(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
}
pub type float_t = ::core::ffi::c_float;
unsafe extern "C" fn dot_product(
    mut a: *mut float_t,
    mut b: *mut float_t,
    mut length: ::core::ffi::c_int,
) -> ::core::ffi::c_double {
    let mut sum: ::core::ffi::c_double = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    let mut i: ::core::ffi::c_int = 0;
    i = 0 as ::core::ffi::c_int;
    while i < length {
        sum += (*a.offset(i as isize) * *b.offset(i as isize)) as ::core::ffi::c_double;
        i += 1;
    }
    return sum;
}
unsafe extern "C" fn normalize(mut v: *mut float_t, mut length: ::core::ffi::c_int) {
    let mut magnitude: ::core::ffi::c_double = sqrt(dot_product(v, v, length));
    let mut i: ::core::ffi::c_int = 0;
    i = 0 as ::core::ffi::c_int;
    while i < length {
        let ref mut fresh0 = *v.offset(i as isize);
        *fresh0 = (*fresh0 as ::core::ffi::c_double / magnitude) as float_t;
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn spectral_contrast(
    mut a: *mut float_t,
    mut b: *mut float_t,
    mut length: ::core::ffi::c_int,
) -> ::core::ffi::c_double {
    normalize(a, length);
    normalize(b, length);
    return dot_product(a, b, length);
}
