pub type __uint8_t = u8;
pub type uint8_t = u8;
#[no_mangle]
pub unsafe extern "C" fn hdr_bitrate(mut h: *const uint8_t) -> libc::c_uint {
    static mut halfrate: [[[uint8_t; 15]; 3]; 2] = [
        [
            [
                0 as libc::c_int as uint8_t,
                4 as libc::c_int as uint8_t,
                8 as libc::c_int as uint8_t,
                12 as libc::c_int as uint8_t,
                16 as libc::c_int as uint8_t,
                20 as libc::c_int as uint8_t,
                24 as libc::c_int as uint8_t,
                28 as libc::c_int as uint8_t,
                32 as libc::c_int as uint8_t,
                40 as libc::c_int as uint8_t,
                48 as libc::c_int as uint8_t,
                56 as libc::c_int as uint8_t,
                64 as libc::c_int as uint8_t,
                72 as libc::c_int as uint8_t,
                80 as libc::c_int as uint8_t,
            ],
            [
                0 as libc::c_int as uint8_t,
                4 as libc::c_int as uint8_t,
                8 as libc::c_int as uint8_t,
                12 as libc::c_int as uint8_t,
                16 as libc::c_int as uint8_t,
                20 as libc::c_int as uint8_t,
                24 as libc::c_int as uint8_t,
                28 as libc::c_int as uint8_t,
                32 as libc::c_int as uint8_t,
                40 as libc::c_int as uint8_t,
                48 as libc::c_int as uint8_t,
                56 as libc::c_int as uint8_t,
                64 as libc::c_int as uint8_t,
                72 as libc::c_int as uint8_t,
                80 as libc::c_int as uint8_t,
            ],
            [
                0 as libc::c_int as uint8_t,
                16 as libc::c_int as uint8_t,
                24 as libc::c_int as uint8_t,
                28 as libc::c_int as uint8_t,
                32 as libc::c_int as uint8_t,
                40 as libc::c_int as uint8_t,
                48 as libc::c_int as uint8_t,
                56 as libc::c_int as uint8_t,
                64 as libc::c_int as uint8_t,
                72 as libc::c_int as uint8_t,
                80 as libc::c_int as uint8_t,
                88 as libc::c_int as uint8_t,
                96 as libc::c_int as uint8_t,
                112 as libc::c_int as uint8_t,
                128 as libc::c_int as uint8_t,
            ],
        ],
        [
            [
                0 as libc::c_int as uint8_t,
                16 as libc::c_int as uint8_t,
                20 as libc::c_int as uint8_t,
                24 as libc::c_int as uint8_t,
                28 as libc::c_int as uint8_t,
                32 as libc::c_int as uint8_t,
                40 as libc::c_int as uint8_t,
                48 as libc::c_int as uint8_t,
                56 as libc::c_int as uint8_t,
                64 as libc::c_int as uint8_t,
                80 as libc::c_int as uint8_t,
                96 as libc::c_int as uint8_t,
                112 as libc::c_int as uint8_t,
                128 as libc::c_int as uint8_t,
                160 as libc::c_int as uint8_t,
            ],
            [
                0 as libc::c_int as uint8_t,
                16 as libc::c_int as uint8_t,
                24 as libc::c_int as uint8_t,
                28 as libc::c_int as uint8_t,
                32 as libc::c_int as uint8_t,
                40 as libc::c_int as uint8_t,
                48 as libc::c_int as uint8_t,
                56 as libc::c_int as uint8_t,
                64 as libc::c_int as uint8_t,
                80 as libc::c_int as uint8_t,
                96 as libc::c_int as uint8_t,
                112 as libc::c_int as uint8_t,
                128 as libc::c_int as uint8_t,
                160 as libc::c_int as uint8_t,
                192 as libc::c_int as uint8_t,
            ],
            [
                0 as libc::c_int as uint8_t,
                16 as libc::c_int as uint8_t,
                32 as libc::c_int as uint8_t,
                48 as libc::c_int as uint8_t,
                64 as libc::c_int as uint8_t,
                80 as libc::c_int as uint8_t,
                96 as libc::c_int as uint8_t,
                112 as libc::c_int as uint8_t,
                128 as libc::c_int as uint8_t,
                144 as libc::c_int as uint8_t,
                160 as libc::c_int as uint8_t,
                176 as libc::c_int as uint8_t,
                192 as libc::c_int as uint8_t,
                208 as libc::c_int as uint8_t,
                224 as libc::c_int as uint8_t,
            ],
        ],
    ];
    return (2 as libc::c_int
        * halfrate[(*h.offset(1 as libc::c_int as isize) as libc::c_int
            & 0x8 as libc::c_int
            != 0) as libc::c_int as usize][((*h
            .offset(1 as libc::c_int as isize)
            as libc::c_int
            >> 1 as libc::c_int
            & 3 as libc::c_int)
            - 1 as libc::c_int) as usize][(*h
            .offset(2 as libc::c_int as isize)
            as libc::c_int
            >> 4 as libc::c_int) as usize] as libc::c_int)
        as libc::c_uint;
}
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}

