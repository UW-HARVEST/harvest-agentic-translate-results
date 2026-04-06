extern "C" {
    fn sqrtf(__x: ::core::ffi::c_float) -> ::core::ffi::c_float;
}
#[no_mangle]
pub unsafe extern "C" fn tfm(
    mut dest: *mut ::core::ffi::c_float,
    mut src: *const ::core::ffi::c_float,
    mut count: ::core::ffi::c_int,
) {
    let mut i: ::core::ffi::c_int = 0;
    i = 0 as ::core::ffi::c_int;
    while i < count {
        if *src.offset(0 as ::core::ffi::c_int as isize)
            < *src.offset(1 as ::core::ffi::c_int as isize)
        {
            let mut dx2: ::core::ffi::c_float = *src.offset(0 as ::core::ffi::c_int as isize);
            let mut dy2: ::core::ffi::c_float = *src.offset(1 as ::core::ffi::c_int as isize);
            let mut dxy: ::core::ffi::c_float = *src.offset(2 as ::core::ffi::c_int as isize);
            let mut sqd: ::core::ffi::c_float =
                dy2 * dy2 - 2.0f32 * dx2 * dy2 + dx2 * dx2 + 4.0f32 * dxy * dxy;
            let mut lambda: ::core::ffi::c_float = 0.5f32
                * (dy2
                    + dx2
                    + sqrtf(
                        (if 0 as ::core::ffi::c_int as ::core::ffi::c_float > sqd {
                            0 as ::core::ffi::c_int as ::core::ffi::c_float
                        } else {
                            sqd
                        }),
                    ));
            *dest.offset(0 as ::core::ffi::c_int as isize) = dx2 - lambda;
            *dest.offset(1 as ::core::ffi::c_int as isize) = dxy;
        } else {
            let mut dy2_0: ::core::ffi::c_float = *src.offset(0 as ::core::ffi::c_int as isize);
            let mut dx2_0: ::core::ffi::c_float = *src.offset(1 as ::core::ffi::c_int as isize);
            let mut dxy_0: ::core::ffi::c_float = *src.offset(2 as ::core::ffi::c_int as isize);
            let mut sqd_0: ::core::ffi::c_float =
                dy2_0 * dy2_0 - 2.0f32 * dx2_0 * dy2_0 + dx2_0 * dx2_0 + 4.0f32 * dxy_0 * dxy_0;
            let mut lambda_0: ::core::ffi::c_float = 0.5f32
                * (dy2_0
                    + dx2_0
                    + sqrtf(
                        (if 0 as ::core::ffi::c_int as ::core::ffi::c_float > sqd_0 {
                            0 as ::core::ffi::c_int as ::core::ffi::c_float
                        } else {
                            sqd_0
                        }),
                    ));
            *dest.offset(0 as ::core::ffi::c_int as isize) = dxy_0;
            *dest.offset(1 as ::core::ffi::c_int as isize) = dx2_0 - lambda_0;
        }
        src = src.offset(3 as ::core::ffi::c_int as isize);
        dest = dest.offset(2 as ::core::ffi::c_int as isize);
        i += 1;
    }
}
