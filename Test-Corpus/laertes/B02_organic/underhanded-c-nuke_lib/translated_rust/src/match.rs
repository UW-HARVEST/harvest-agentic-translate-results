extern "C" {
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    
}
pub use crate::src::spectral_contrast::spectral_contrast;
pub type size_t = usize;
pub type float_t = libc::c_double;
pub const N_SMOOTH: libc::c_int = 16 as libc::c_int;
unsafe extern "C" fn total(
    mut v: *mut float_t,
    mut length: libc::c_int,
) -> libc::c_double {
    let mut sum: libc::c_double = 0 as libc::c_int as libc::c_double;
    let mut i: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < length {
        sum += *v.offset(i as isize) as libc::c_double;
        i += 1;
    }
    return sum;
}
unsafe extern "C" fn smoothen(mut v: *mut float_t, mut length: libc::c_int) {
    let mut sum: libc::c_double = 0.;
    let mut i: libc::c_int = 0;
    let mut j: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < length {
        sum = 0 as libc::c_int as libc::c_double;
        j = 0 as libc::c_int;
        while j < N_SMOOTH && i + j < length {
            sum += *v.offset((i + j) as isize) as libc::c_double;
            j += 1;
        }
        *v.offset(i as isize) = (sum / N_SMOOTH as libc::c_double) as float_t;
        i += 1;
    }
}
unsafe extern "C" fn differentiate(mut v: *mut float_t, mut length: libc::c_int) {
    let mut i: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < length - 1 as libc::c_int {
        *v.offset(i as isize) =
            *v.offset((i + 1 as libc::c_int) as isize) - *v.offset(i as isize);
        i += 1;
    }
    *v.offset((length - 1 as libc::c_int) as isize) = 0 as libc::c_int as float_t;
}
unsafe extern "C" fn preprocess(
    mut v: *mut float_t,
    mut source: *mut float_t,
    mut length: libc::c_int,
) {
    memcpy(
        v as *mut libc::c_void,
        source as *const libc::c_void,
        (length as size_t).wrapping_mul(std::mem::size_of::<float_t>() as size_t),
    );
    smoothen(v, length);
    differentiate(v, length);
    smoothen(v, length);
}
#[export_name = "match"]
pub unsafe extern "C" fn match_0(
    mut test: *mut float_t,
    mut reference: *mut float_t,
    mut bins: libc::c_int,
    mut threshold: libc::c_double,
) -> libc::c_int {
    let vla = bins as usize;
    let mut t: Vec<float_t> = ::std::vec::from_elem(0., vla);
    let vla_0 = bins as usize;
    let mut r: Vec<float_t> = ::std::vec::from_elem(0., vla_0);
    if total(test, bins) < threshold * total(reference, bins) {
        return 0 as libc::c_int;
    }
    preprocess(t.as_mut_ptr(), test, bins);
    preprocess(r.as_mut_ptr(), reference, bins);
    return (spectral_contrast(t.as_mut_ptr(), r.as_mut_ptr(), bins) >= threshold)
        as libc::c_int;
}
