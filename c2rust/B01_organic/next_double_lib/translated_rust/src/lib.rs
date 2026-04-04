pub type __uint64_t = u64;
pub type uint64_t = __uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cn_rnd_t {
    pub state: [uint64_t; 2],
}
unsafe extern "C" fn cn_rnd_next(mut rnd: *mut cn_rnd_t) -> uint64_t {
    let mut x: uint64_t = (*rnd).state[0 as ::core::ffi::c_int as usize];
    let mut y: uint64_t = (*rnd).state[1 as ::core::ffi::c_int as usize];
    (*rnd).state[0 as ::core::ffi::c_int as usize] = y;
    x = (x as ::core::ffi::c_ulong ^ (x << 23 as ::core::ffi::c_int) as ::core::ffi::c_ulong)
        as uint64_t;
    x = (x as ::core::ffi::c_ulong ^ (x >> 17 as ::core::ffi::c_int) as ::core::ffi::c_ulong)
        as uint64_t;
    x = (x as ::core::ffi::c_ulong ^ (y ^ y >> 26 as ::core::ffi::c_int) as ::core::ffi::c_ulong)
        as uint64_t;
    (*rnd).state[1 as ::core::ffi::c_int as usize] = x;
    return x.wrapping_add(y);
}
#[no_mangle]
pub unsafe extern "C" fn next_double(mut rnd: *mut cn_rnd_t) -> ::core::ffi::c_double {
    let mut value: uint64_t = cn_rnd_next(rnd);
    let mut exponent: uint64_t = 1023 as uint64_t;
    let mut mantissa: uint64_t = value >> 12 as ::core::ffi::c_int;
    let mut result: uint64_t = exponent << 52 as ::core::ffi::c_int | mantissa;
    return *(&raw mut result as *mut ::core::ffi::c_double) - 1.0f64;
}
