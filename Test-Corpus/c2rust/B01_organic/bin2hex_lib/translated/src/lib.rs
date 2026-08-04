extern "C" {
    fn abort() -> !;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type uint8_t = __uint8_t;
#[no_mangle]
pub unsafe extern "C" fn bin2hex(
    mut hex: *mut ::core::ffi::c_char,
    mut hex_maxlen: size_t,
    mut bin: *const uint8_t,
    mut bin_len: size_t,
) -> *mut ::core::ffi::c_char {
    let mut i: size_t = 0 as ::core::ffi::c_uint as size_t;
    let mut x: ::core::ffi::c_uint = 0;
    let mut b: ::core::ffi::c_int = 0;
    let mut c: ::core::ffi::c_int = 0;
    if bin_len >= (18446744073709551615 as size_t).wrapping_div(2 as size_t)
        || hex_maxlen <= bin_len.wrapping_mul(2 as size_t)
    {
        abort();
    }
    while i < bin_len {
        c = *bin.offset(i as isize) as ::core::ffi::c_int & 0xf as ::core::ffi::c_int;
        b = *bin.offset(i as isize) as ::core::ffi::c_int >> 4 as ::core::ffi::c_int;
        x = (((87 as ::core::ffi::c_uint)
            .wrapping_add(c as ::core::ffi::c_uint)
            .wrapping_add(
                (c as ::core::ffi::c_uint).wrapping_sub(10 as ::core::ffi::c_uint)
                    >> 8 as ::core::ffi::c_int
                    & !(38 as ::core::ffi::c_uint),
            ) as ::core::ffi::c_uchar as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
            | (87 as ::core::ffi::c_uint)
                .wrapping_add(b as ::core::ffi::c_uint)
                .wrapping_add(
                    (b as ::core::ffi::c_uint).wrapping_sub(10 as ::core::ffi::c_uint)
                        >> 8 as ::core::ffi::c_int
                        & !(38 as ::core::ffi::c_uint),
                ) as ::core::ffi::c_uchar as ::core::ffi::c_int) as ::core::ffi::c_uint;
        *hex.offset(i.wrapping_mul(2 as size_t) as isize) = x as ::core::ffi::c_char;
        x >>= 8 as ::core::ffi::c_int;
        *hex.offset(i.wrapping_mul(2 as size_t).wrapping_add(1 as size_t) as isize) =
            x as ::core::ffi::c_char;
        i = i.wrapping_add(1);
    }
    *hex.offset(i.wrapping_mul(2 as size_t) as isize) = 0 as ::core::ffi::c_char;
    return hex;
}
