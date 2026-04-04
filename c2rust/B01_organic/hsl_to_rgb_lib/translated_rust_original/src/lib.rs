extern "C" {
    fn fabsf(__x: ::core::ffi::c_float) -> ::core::ffi::c_float;
    fn fmodf(__x: ::core::ffi::c_float, __y: ::core::ffi::c_float) -> ::core::ffi::c_float;
}
#[no_mangle]
pub unsafe extern "C" fn hsl_to_rgb(
    mut dest: *mut ::core::ffi::c_float,
    mut src: *const ::core::ffi::c_float,
) {
    let mut h: ::core::ffi::c_float = *src.offset(0 as ::core::ffi::c_int as isize);
    let mut s: ::core::ffi::c_float = *src.offset(1 as ::core::ffi::c_int as isize);
    let mut l: ::core::ffi::c_float = *src.offset(2 as ::core::ffi::c_int as isize);
    let mut c: ::core::ffi::c_float = 0.;
    let mut m: ::core::ffi::c_float = 0.;
    let mut x: ::core::ffi::c_float = 0.;
    if s == 0 as ::core::ffi::c_int as ::core::ffi::c_float {
        *dest.offset(0 as ::core::ffi::c_int as isize) = l;
        *dest.offset(1 as ::core::ffi::c_int as isize) = l;
        *dest.offset(2 as ::core::ffi::c_int as isize) = l;
        return;
    }
    c = (1.0f32 - fabsf(2.0f32 * l - 1.0f32)) * s;
    m = 1.0f32 * (l - 0.5f32 * c);
    x = c
        * (1.0f32
            - fabsf(fmodf(h / 60.0f32, 2 as ::core::ffi::c_int as ::core::ffi::c_float) - 1.0f32));
    if h >= 0.0f32 && h < 60.0f32 {
        *dest.offset(0 as ::core::ffi::c_int as isize) = c + m;
        *dest.offset(1 as ::core::ffi::c_int as isize) = x + m;
        *dest.offset(2 as ::core::ffi::c_int as isize) = m;
    } else if h >= 60.0f32 && h < 120.0f32 {
        *dest.offset(0 as ::core::ffi::c_int as isize) = x + m;
        *dest.offset(1 as ::core::ffi::c_int as isize) = c + m;
        *dest.offset(2 as ::core::ffi::c_int as isize) = m;
    } else if h < 120.0f32 && h < 180.0f32 {
        *dest.offset(0 as ::core::ffi::c_int as isize) = m;
        *dest.offset(1 as ::core::ffi::c_int as isize) = c + m;
        *dest.offset(2 as ::core::ffi::c_int as isize) = x + m;
    } else if h >= 180.0f32 && h < 240.0f32 {
        *dest.offset(0 as ::core::ffi::c_int as isize) = m;
        *dest.offset(1 as ::core::ffi::c_int as isize) = x + m;
        *dest.offset(2 as ::core::ffi::c_int as isize) = c + m;
    } else if h >= 240.0f32 && h < 300.0f32 {
        *dest.offset(0 as ::core::ffi::c_int as isize) = x + m;
        *dest.offset(1 as ::core::ffi::c_int as isize) = m;
        *dest.offset(2 as ::core::ffi::c_int as isize) = c + m;
    } else if h >= 300.0f32 && h < 360.0f32 {
        *dest.offset(0 as ::core::ffi::c_int as isize) = c + m;
        *dest.offset(1 as ::core::ffi::c_int as isize) = m;
        *dest.offset(2 as ::core::ffi::c_int as isize) = x + m;
    } else {
        *dest.offset(0 as ::core::ffi::c_int as isize) = m;
        *dest.offset(1 as ::core::ffi::c_int as isize) = m;
        *dest.offset(2 as ::core::ffi::c_int as isize) = m;
    };
}
