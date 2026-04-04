pub type __uint8_t = u8;
pub type uint8_t = __uint8_t;
#[no_mangle]
pub unsafe extern "C" fn hdr_bitrate(mut h: *const uint8_t) -> ::core::ffi::c_uint {
    static mut halfrate: [[[uint8_t; 15]; 3]; 2] = [
        [
            [
                0 as ::core::ffi::c_int as uint8_t,
                4 as ::core::ffi::c_int as uint8_t,
                8 as ::core::ffi::c_int as uint8_t,
                12 as ::core::ffi::c_int as uint8_t,
                16 as ::core::ffi::c_int as uint8_t,
                20 as ::core::ffi::c_int as uint8_t,
                24 as ::core::ffi::c_int as uint8_t,
                28 as ::core::ffi::c_int as uint8_t,
                32 as ::core::ffi::c_int as uint8_t,
                40 as ::core::ffi::c_int as uint8_t,
                48 as ::core::ffi::c_int as uint8_t,
                56 as ::core::ffi::c_int as uint8_t,
                64 as ::core::ffi::c_int as uint8_t,
                72 as ::core::ffi::c_int as uint8_t,
                80 as ::core::ffi::c_int as uint8_t,
            ],
            [
                0 as ::core::ffi::c_int as uint8_t,
                4 as ::core::ffi::c_int as uint8_t,
                8 as ::core::ffi::c_int as uint8_t,
                12 as ::core::ffi::c_int as uint8_t,
                16 as ::core::ffi::c_int as uint8_t,
                20 as ::core::ffi::c_int as uint8_t,
                24 as ::core::ffi::c_int as uint8_t,
                28 as ::core::ffi::c_int as uint8_t,
                32 as ::core::ffi::c_int as uint8_t,
                40 as ::core::ffi::c_int as uint8_t,
                48 as ::core::ffi::c_int as uint8_t,
                56 as ::core::ffi::c_int as uint8_t,
                64 as ::core::ffi::c_int as uint8_t,
                72 as ::core::ffi::c_int as uint8_t,
                80 as ::core::ffi::c_int as uint8_t,
            ],
            [
                0 as ::core::ffi::c_int as uint8_t,
                16 as ::core::ffi::c_int as uint8_t,
                24 as ::core::ffi::c_int as uint8_t,
                28 as ::core::ffi::c_int as uint8_t,
                32 as ::core::ffi::c_int as uint8_t,
                40 as ::core::ffi::c_int as uint8_t,
                48 as ::core::ffi::c_int as uint8_t,
                56 as ::core::ffi::c_int as uint8_t,
                64 as ::core::ffi::c_int as uint8_t,
                72 as ::core::ffi::c_int as uint8_t,
                80 as ::core::ffi::c_int as uint8_t,
                88 as ::core::ffi::c_int as uint8_t,
                96 as ::core::ffi::c_int as uint8_t,
                112 as ::core::ffi::c_int as uint8_t,
                128 as ::core::ffi::c_int as uint8_t,
            ],
        ],
        [
            [
                0 as ::core::ffi::c_int as uint8_t,
                16 as ::core::ffi::c_int as uint8_t,
                20 as ::core::ffi::c_int as uint8_t,
                24 as ::core::ffi::c_int as uint8_t,
                28 as ::core::ffi::c_int as uint8_t,
                32 as ::core::ffi::c_int as uint8_t,
                40 as ::core::ffi::c_int as uint8_t,
                48 as ::core::ffi::c_int as uint8_t,
                56 as ::core::ffi::c_int as uint8_t,
                64 as ::core::ffi::c_int as uint8_t,
                80 as ::core::ffi::c_int as uint8_t,
                96 as ::core::ffi::c_int as uint8_t,
                112 as ::core::ffi::c_int as uint8_t,
                128 as ::core::ffi::c_int as uint8_t,
                160 as ::core::ffi::c_int as uint8_t,
            ],
            [
                0 as ::core::ffi::c_int as uint8_t,
                16 as ::core::ffi::c_int as uint8_t,
                24 as ::core::ffi::c_int as uint8_t,
                28 as ::core::ffi::c_int as uint8_t,
                32 as ::core::ffi::c_int as uint8_t,
                40 as ::core::ffi::c_int as uint8_t,
                48 as ::core::ffi::c_int as uint8_t,
                56 as ::core::ffi::c_int as uint8_t,
                64 as ::core::ffi::c_int as uint8_t,
                80 as ::core::ffi::c_int as uint8_t,
                96 as ::core::ffi::c_int as uint8_t,
                112 as ::core::ffi::c_int as uint8_t,
                128 as ::core::ffi::c_int as uint8_t,
                160 as ::core::ffi::c_int as uint8_t,
                192 as ::core::ffi::c_int as uint8_t,
            ],
            [
                0 as ::core::ffi::c_int as uint8_t,
                16 as ::core::ffi::c_int as uint8_t,
                32 as ::core::ffi::c_int as uint8_t,
                48 as ::core::ffi::c_int as uint8_t,
                64 as ::core::ffi::c_int as uint8_t,
                80 as ::core::ffi::c_int as uint8_t,
                96 as ::core::ffi::c_int as uint8_t,
                112 as ::core::ffi::c_int as uint8_t,
                128 as ::core::ffi::c_int as uint8_t,
                144 as ::core::ffi::c_int as uint8_t,
                160 as ::core::ffi::c_int as uint8_t,
                176 as ::core::ffi::c_int as uint8_t,
                192 as ::core::ffi::c_int as uint8_t,
                208 as ::core::ffi::c_int as uint8_t,
                224 as ::core::ffi::c_int as uint8_t,
            ],
        ],
    ];
    return (2 as ::core::ffi::c_int
        * halfrate[(*h.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 0x8 as ::core::ffi::c_int
            != 0) as ::core::ffi::c_int as usize][((*h
            .offset(1 as ::core::ffi::c_int as isize)
            as ::core::ffi::c_int
            >> 1 as ::core::ffi::c_int
            & 3 as ::core::ffi::c_int)
            - 1 as ::core::ffi::c_int) as usize][(*h
            .offset(2 as ::core::ffi::c_int as isize)
            as ::core::ffi::c_int
            >> 4 as ::core::ffi::c_int) as usize] as ::core::ffi::c_int)
        as ::core::ffi::c_uint;
}
