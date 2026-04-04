pub type __uint32_t = u32;
pub type uint32_t = __uint32_t;
#[no_mangle]
pub unsafe extern "C" fn rev16(mut a: uint32_t) -> uint32_t {
    a = (a & 0xaaaa as uint32_t) >> 1 as ::core::ffi::c_int
        | (a & 0x5555 as uint32_t) << 1 as ::core::ffi::c_int;
    a = (a & 0xcccc as uint32_t) >> 2 as ::core::ffi::c_int
        | (a & 0x3333 as uint32_t) << 2 as ::core::ffi::c_int;
    a = (a & 0xf0f0 as uint32_t) >> 4 as ::core::ffi::c_int
        | (a & 0xf0f as uint32_t) << 4 as ::core::ffi::c_int;
    a = (a & 0xff00 as uint32_t) >> 8 as ::core::ffi::c_int
        | (a & 0xff as uint32_t) << 8 as ::core::ffi::c_int;
    return a;
}
