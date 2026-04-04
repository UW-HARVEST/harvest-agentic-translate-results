extern "C" {
    fn sqrtf(__x: ::core::ffi::c_float) -> ::core::ffi::c_float;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type size_t = usize;
#[no_mangle]
pub unsafe extern "C" fn normalize(
    mut dest: *mut ::core::ffi::c_float,
    mut src: *const ::core::ffi::c_float,
    mut size: ::core::ffi::c_int,
) {
    let mut sum: ::core::ffi::c_float = 0.0f32;
    let mut i: ::core::ffi::c_int = 0;
    i = 0 as ::core::ffi::c_int;
    while i < size {
        sum += *src.offset(i as isize) * *src.offset(i as isize);
        i += 1;
    }
    if sum > 0.0f32 {
        sum = 1.0f32 / sqrtf(sum);
        i = 0 as ::core::ffi::c_int;
        while i < size {
            *dest.offset(i as isize) = *src.offset(i as isize) * sum;
            i += 1;
        }
    } else if dest != src as *mut ::core::ffi::c_float {
        memset(
            dest as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            (size as size_t).wrapping_mul(::core::mem::size_of::<::core::ffi::c_float>() as size_t),
        );
    }
}
