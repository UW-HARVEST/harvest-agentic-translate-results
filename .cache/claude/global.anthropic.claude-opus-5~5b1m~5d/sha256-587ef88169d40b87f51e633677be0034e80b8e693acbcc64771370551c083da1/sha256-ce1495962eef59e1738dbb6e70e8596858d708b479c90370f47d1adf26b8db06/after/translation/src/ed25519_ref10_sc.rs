//! Translation of `sc25519_sq`, `sc25519_sqmul`, `sc25519_invert`, and
//! `sc25519_is_canonical` from `crypto_core/ed25519/ref10/ed25519_ref10.c`
//! (lines 2161, 2178, 2189, 2574).
//!
//! Linker symbols renamed by `private/quirks.h`:
//! `sc25519_invert` -> `_sodium_sc25519_invert`,
//! `sc25519_is_canonical` -> `_sodium_sc25519_is_canonical`.

extern "C" {
    #[link_name = "_sodium_sc25519_mul"]
    fn sc25519_mul(s: *mut u8, a: *const u8, b: *const u8);
}

/*
 Input:
 a[0]+256*a[1]+...+256^31*a[31] = a
 *
 Output:
 s[0]+256*s[1]+...+256^31*s[31] = a^2 mod l
 where l = 2^252 + 27742317777372353535851937790883648493.
 */

#[inline]
unsafe fn sc25519_sq(s: *mut u8, a: *const u8) {
    sc25519_mul(s, a, a);
}

/*
 Input:
 s[0]+256*a[1]+...+256^31*a[31] = a
 n
 *
 Output:
 s[0]+256*s[1]+...+256^31*s[31] = x * s^(s^n) mod l
 where l = 2^252 + 27742317777372353535851937790883648493.
 Overwrites s in place.
 */

#[inline]
unsafe fn sc25519_sqmul(s: *mut u8, n: core::ffi::c_int, a: *const u8) {
    let mut i: core::ffi::c_int;

    i = 0;
    while i < n {
        sc25519_sq(s, s);
        i += 1;
    }
    sc25519_mul(s, s, a);
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_sc25519_invert(recip: *mut u8, s: *const u8) {
    let mut _10: [u8; 32] = [0; 32];
    let mut _100: [u8; 32] = [0; 32];
    let mut _1000: [u8; 32] = [0; 32];
    let mut _10000: [u8; 32] = [0; 32];
    let mut _100000: [u8; 32] = [0; 32];
    let mut _1000000: [u8; 32] = [0; 32];
    let mut _10010011: [u8; 32] = [0; 32];
    let mut _10010111: [u8; 32] = [0; 32];
    let mut _100110: [u8; 32] = [0; 32];
    let mut _1010: [u8; 32] = [0; 32];
    let mut _1010000: [u8; 32] = [0; 32];
    let mut _1010011: [u8; 32] = [0; 32];
    let mut _1011: [u8; 32] = [0; 32];
    let mut _10110: [u8; 32] = [0; 32];
    let mut _10111101: [u8; 32] = [0; 32];
    let mut _11: [u8; 32] = [0; 32];
    let mut _1100011: [u8; 32] = [0; 32];
    let mut _1100111: [u8; 32] = [0; 32];
    let mut _11010011: [u8; 32] = [0; 32];
    let mut _1101011: [u8; 32] = [0; 32];
    let mut _11100111: [u8; 32] = [0; 32];
    let mut _11101011: [u8; 32] = [0; 32];
    let mut _11110101: [u8; 32] = [0; 32];

    sc25519_sq(_10.as_mut_ptr(), s);
    sc25519_mul(_11.as_mut_ptr(), s, _10.as_ptr());
    sc25519_mul(_100.as_mut_ptr(), s, _11.as_ptr());
    sc25519_sq(_1000.as_mut_ptr(), _100.as_ptr());
    sc25519_mul(_1010.as_mut_ptr(), _10.as_ptr(), _1000.as_ptr());
    sc25519_mul(_1011.as_mut_ptr(), s, _1010.as_ptr());
    sc25519_sq(_10000.as_mut_ptr(), _1000.as_ptr());
    sc25519_sq(_10110.as_mut_ptr(), _1011.as_ptr());
    sc25519_mul(_100000.as_mut_ptr(), _1010.as_ptr(), _10110.as_ptr());
    sc25519_mul(_100110.as_mut_ptr(), _10000.as_ptr(), _10110.as_ptr());
    sc25519_sq(_1000000.as_mut_ptr(), _100000.as_ptr());
    sc25519_mul(_1010000.as_mut_ptr(), _10000.as_ptr(), _1000000.as_ptr());
    sc25519_mul(_1010011.as_mut_ptr(), _11.as_ptr(), _1010000.as_ptr());
    sc25519_mul(_1100011.as_mut_ptr(), _10000.as_ptr(), _1010011.as_ptr());
    sc25519_mul(_1100111.as_mut_ptr(), _100.as_ptr(), _1100011.as_ptr());
    sc25519_mul(_1101011.as_mut_ptr(), _100.as_ptr(), _1100111.as_ptr());
    sc25519_mul(_10010011.as_mut_ptr(), _1000000.as_ptr(), _1010011.as_ptr());
    sc25519_mul(_10010111.as_mut_ptr(), _100.as_ptr(), _10010011.as_ptr());
    sc25519_mul(_10111101.as_mut_ptr(), _100110.as_ptr(), _10010111.as_ptr());
    sc25519_mul(_11010011.as_mut_ptr(), _10110.as_ptr(), _10111101.as_ptr());
    sc25519_mul(_11100111.as_mut_ptr(), _1010000.as_ptr(), _10010111.as_ptr());
    sc25519_mul(_11101011.as_mut_ptr(), _100.as_ptr(), _11100111.as_ptr());
    sc25519_mul(_11110101.as_mut_ptr(), _1010.as_ptr(), _11101011.as_ptr());

    sc25519_mul(recip, _1011.as_ptr(), _11110101.as_ptr());
    sc25519_sqmul(recip, 126, _1010011.as_ptr());
    sc25519_sqmul(recip, 9, _10.as_ptr());
    sc25519_mul(recip, recip, _11110101.as_ptr());
    sc25519_sqmul(recip, 7, _1100111.as_ptr());
    sc25519_sqmul(recip, 9, _11110101.as_ptr());
    sc25519_sqmul(recip, 11, _10111101.as_ptr());
    sc25519_sqmul(recip, 8, _11100111.as_ptr());
    sc25519_sqmul(recip, 9, _1101011.as_ptr());
    sc25519_sqmul(recip, 6, _1011.as_ptr());
    sc25519_sqmul(recip, 14, _10010011.as_ptr());
    sc25519_sqmul(recip, 10, _1100011.as_ptr());
    sc25519_sqmul(recip, 9, _10010111.as_ptr());
    sc25519_sqmul(recip, 10, _11110101.as_ptr());
    sc25519_sqmul(recip, 8, _11010011.as_ptr());
    sc25519_sqmul(recip, 8, _11101011.as_ptr());
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_sc25519_is_canonical(s: *const u8) -> core::ffi::c_int {
    /* 2^252+27742317777372353535851937790883648493 */
    static L: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde,
        0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x10,
    ];
    let mut c: u8 = 0;
    let mut n: u8 = 1;
    let mut i: u32 = 32;

    loop {
        i -= 1;
        let si: i32 = *s.add(i as usize) as i32;
        let li: i32 = L[i as usize] as i32;
        c = (c as i32 | ((si.wrapping_sub(li) >> 8) & (n as i32))) as u8;
        n = (n as i32 & ((si ^ li).wrapping_sub(1) >> 8)) as u8;
        if i == 0 {
            break;
        }
    }

    (c != 0) as core::ffi::c_int
}
