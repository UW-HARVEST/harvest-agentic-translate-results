extern "C" {
    fn expf(__x: ::core::ffi::c_float) -> ::core::ffi::c_float;
}
#[no_mangle]
pub unsafe extern "C" fn gaussian_kernel(
    mut dest: *mut ::core::ffi::c_float,
    mut size: ::core::ffi::c_int,
    mut radius: ::core::ffi::c_float,
) {
    let mut k: *mut ::core::ffi::c_float = ::core::ptr::null_mut::<::core::ffi::c_float>();
    let mut rs: ::core::ffi::c_float = 0.;
    let mut s2: ::core::ffi::c_float = 0.;
    let mut sum: ::core::ffi::c_float = 0.;
    let mut sigma: ::core::ffi::c_float = 1.6f32;
    let mut tetha: ::core::ffi::c_float = 2.25f32;
    let mut r: ::core::ffi::c_int = 0;
    let mut hsize: ::core::ffi::c_int = size / 2 as ::core::ffi::c_int;
    s2 = 1.0f32 / expf(sigma * sigma * tetha);
    rs = sigma / radius;
    k = dest;
    sum = 0.0f32;
    r = -hsize;
    while r <= hsize {
        let mut x: ::core::ffi::c_float = r as ::core::ffi::c_float * rs;
        let mut v: ::core::ffi::c_float = 1.0f32 / expf(x * x) - s2;
        v = if v > 0 as ::core::ffi::c_int as ::core::ffi::c_float {
            v
        } else {
            0 as ::core::ffi::c_int as ::core::ffi::c_float
        };
        *k = v;
        sum += v;
        k = k.offset(1);
        r += 1;
    }
    if sum > 0.0f32 {
        let mut isum: ::core::ffi::c_float = 1.0f32 / sum;
        r = 0 as ::core::ffi::c_int;
        while r < size {
            *dest.offset(r as isize) *= isum;
            r += 1;
        }
    }
}
