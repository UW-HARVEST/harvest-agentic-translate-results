#[no_mangle]
pub unsafe extern "C" fn rgb_to_hsv(
    mut dest: *mut ::core::ffi::c_float,
    mut src: *const ::core::ffi::c_float,
) {
    let mut r: ::core::ffi::c_float = *src.offset(0 as ::core::ffi::c_int as isize);
    let mut g: ::core::ffi::c_float = *src.offset(1 as ::core::ffi::c_int as isize);
    let mut b: ::core::ffi::c_float = *src.offset(2 as ::core::ffi::c_int as isize);
    let mut h: ::core::ffi::c_float = 0 as ::core::ffi::c_int as ::core::ffi::c_float;
    let mut s: ::core::ffi::c_float = 0 as ::core::ffi::c_int as ::core::ffi::c_float;
    let mut v: ::core::ffi::c_float = 0 as ::core::ffi::c_int as ::core::ffi::c_float;
    let mut min: ::core::ffi::c_float = r;
    let mut max: ::core::ffi::c_float = r;
    let mut delta: ::core::ffi::c_float = 0.;
    min = if min < g { min } else { g };
    min = if min < b { min } else { b };
    max = if max > g { max } else { g };
    max = if max > b { max } else { b };
    delta = max - min;
    v = max;
    if delta == 0 as ::core::ffi::c_int as ::core::ffi::c_float
        || max == 0 as ::core::ffi::c_int as ::core::ffi::c_float
    {
        *dest.offset(0 as ::core::ffi::c_int as isize) = h;
        *dest.offset(1 as ::core::ffi::c_int as isize) = s;
        *dest.offset(2 as ::core::ffi::c_int as isize) = v;
        return;
    }
    s = delta / max;
    if r == max {
        h = (g - b) / delta;
    } else if g == max {
        h = 2 as ::core::ffi::c_int as ::core::ffi::c_float + (b - r) / delta;
    } else {
        h = 4 as ::core::ffi::c_int as ::core::ffi::c_float + (r - g) / delta;
    }
    h *= 60 as ::core::ffi::c_int as ::core::ffi::c_float;
    if h < 0 as ::core::ffi::c_int as ::core::ffi::c_float {
        h += 360 as ::core::ffi::c_int as ::core::ffi::c_float;
    }
    *dest.offset(0 as ::core::ffi::c_int as isize) = h;
    *dest.offset(1 as ::core::ffi::c_int as isize) = s;
    *dest.offset(2 as ::core::ffi::c_int as isize) = v;
}
