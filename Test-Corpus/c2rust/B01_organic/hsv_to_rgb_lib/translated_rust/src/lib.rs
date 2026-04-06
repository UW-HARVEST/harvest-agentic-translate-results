extern "C" {
    fn floorf(__x: ::core::ffi::c_float) -> ::core::ffi::c_float;
}
#[no_mangle]
pub unsafe extern "C" fn hsv_to_rgb(
    mut dest: *mut ::core::ffi::c_float,
    mut src: *const ::core::ffi::c_float,
) {
    let mut r: ::core::ffi::c_float = 0.;
    let mut g: ::core::ffi::c_float = 0.;
    let mut b: ::core::ffi::c_float = 0.;
    let mut f: ::core::ffi::c_float = 0.;
    let mut p: ::core::ffi::c_float = 0.;
    let mut q: ::core::ffi::c_float = 0.;
    let mut t: ::core::ffi::c_float = 0.;
    let mut h: ::core::ffi::c_float = *src.offset(0 as ::core::ffi::c_int as isize);
    let mut s: ::core::ffi::c_float = *src.offset(1 as ::core::ffi::c_int as isize);
    let mut v: ::core::ffi::c_float = *src.offset(2 as ::core::ffi::c_int as isize);
    let mut i: ::core::ffi::c_int = 0;
    if s == 0 as ::core::ffi::c_int as ::core::ffi::c_float {
        *dest.offset(0 as ::core::ffi::c_int as isize) = v;
        *dest.offset(1 as ::core::ffi::c_int as isize) = v;
        *dest.offset(2 as ::core::ffi::c_int as isize) = v;
        return;
    }
    h /= 60.0f32;
    i = floorf(h) as ::core::ffi::c_int;
    f = h - i as ::core::ffi::c_float;
    p = v * (1 as ::core::ffi::c_int as ::core::ffi::c_float - s);
    q = v * (1 as ::core::ffi::c_int as ::core::ffi::c_float - s * f);
    t = v
        * (1 as ::core::ffi::c_int as ::core::ffi::c_float
            - s * (1 as ::core::ffi::c_int as ::core::ffi::c_float - f));
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
    *dest.offset(0 as ::core::ffi::c_int as isize) = r;
    *dest.offset(1 as ::core::ffi::c_int as isize) = g;
    *dest.offset(2 as ::core::ffi::c_int as isize) = b;
}
