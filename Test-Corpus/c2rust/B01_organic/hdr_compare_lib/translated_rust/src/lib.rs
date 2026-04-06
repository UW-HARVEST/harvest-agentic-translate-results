pub type __uint8_t = u8;
pub type uint8_t = __uint8_t;
unsafe extern "C" fn hdr_valid(mut h: *const uint8_t) -> ::core::ffi::c_int {
    return (*h.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
        == 0xff as ::core::ffi::c_int
        && (*h.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 0xf0 as ::core::ffi::c_int
            == 0xf0 as ::core::ffi::c_int
            || *h.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0xfe as ::core::ffi::c_int
                == 0xe2 as ::core::ffi::c_int)
        && *h.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            >> 1 as ::core::ffi::c_int
            & 3 as ::core::ffi::c_int
            != 0 as ::core::ffi::c_int
        && *h.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            >> 4 as ::core::ffi::c_int
            != 15 as ::core::ffi::c_int
        && *h.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            >> 2 as ::core::ffi::c_int
            & 3 as ::core::ffi::c_int
            != 3 as ::core::ffi::c_int) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn hdr_compare(
    mut h1: *const uint8_t,
    mut h2: *const uint8_t,
) -> ::core::ffi::c_int {
    return (hdr_valid(h2) != 0
        && (*h1.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            ^ *h2.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            & 0xfe as ::core::ffi::c_int
            == 0 as ::core::ffi::c_int
        && (*h1.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            ^ *h2.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            & 0xc as ::core::ffi::c_int
            == 0 as ::core::ffi::c_int
        && (*h1.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 0xf0 as ::core::ffi::c_int
            == 0 as ::core::ffi::c_int) as ::core::ffi::c_int
            ^ (*h2.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0xf0 as ::core::ffi::c_int
                == 0 as ::core::ffi::c_int) as ::core::ffi::c_int
            == 0) as ::core::ffi::c_int;
}
