//! `sc25519_*` scalar arithmetic from
//! `crypto_core/ed25519/ref10/ed25519_ref10.c`
//! (`sc25519_sq`, `sc25519_sqmul`, `sc25519_invert`, `sc25519_reduce`,
//! `sc25519_is_canonical`), plus the `extern "C"` entry points renamed by
//! `include/sodium/private/quirks.h`.
//!
//! `sc25519_mul` / `sc25519_muladd` themselves live in
//! `crate::ed25519_ref10::sc_mul`; this file owns their exported wrappers.

use core::ffi::c_int;

use crate::ed25519_ref10::sc_mul;
use crate::ed25519_ref10::{load_3, load_4};

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
    sc_mul::sc25519_mul(s, a, a);
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
unsafe fn sc25519_sqmul(s: *mut u8, n: c_int, a: *const u8) {
    let mut i: c_int;

    i = 0;
    while i < n {
        sc25519_sq(s, s);
        i += 1;
    }
    sc_mul::sc25519_mul(s, s, a);
}

pub unsafe fn sc25519_invert(recip: *mut u8, s: *const u8) {
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
    sc_mul::sc25519_mul(_11.as_mut_ptr(), s, _10.as_ptr());
    sc_mul::sc25519_mul(_100.as_mut_ptr(), s, _11.as_ptr());
    sc25519_sq(_1000.as_mut_ptr(), _100.as_ptr());
    sc_mul::sc25519_mul(_1010.as_mut_ptr(), _10.as_ptr(), _1000.as_ptr());
    sc_mul::sc25519_mul(_1011.as_mut_ptr(), s, _1010.as_ptr());
    sc25519_sq(_10000.as_mut_ptr(), _1000.as_ptr());
    sc25519_sq(_10110.as_mut_ptr(), _1011.as_ptr());
    sc_mul::sc25519_mul(_100000.as_mut_ptr(), _1010.as_ptr(), _10110.as_ptr());
    sc_mul::sc25519_mul(_100110.as_mut_ptr(), _10000.as_ptr(), _10110.as_ptr());
    sc25519_sq(_1000000.as_mut_ptr(), _100000.as_ptr());
    sc_mul::sc25519_mul(_1010000.as_mut_ptr(), _10000.as_ptr(), _1000000.as_ptr());
    sc_mul::sc25519_mul(_1010011.as_mut_ptr(), _11.as_ptr(), _1010000.as_ptr());
    sc_mul::sc25519_mul(_1100011.as_mut_ptr(), _10000.as_ptr(), _1010011.as_ptr());
    sc_mul::sc25519_mul(_1100111.as_mut_ptr(), _100.as_ptr(), _1100011.as_ptr());
    sc_mul::sc25519_mul(_1101011.as_mut_ptr(), _100.as_ptr(), _1100111.as_ptr());
    sc_mul::sc25519_mul(_10010011.as_mut_ptr(), _1000000.as_ptr(), _1010011.as_ptr());
    sc_mul::sc25519_mul(_10010111.as_mut_ptr(), _100.as_ptr(), _10010011.as_ptr());
    sc_mul::sc25519_mul(_10111101.as_mut_ptr(), _100110.as_ptr(), _10010111.as_ptr());
    sc_mul::sc25519_mul(_11010011.as_mut_ptr(), _10110.as_ptr(), _10111101.as_ptr());
    sc_mul::sc25519_mul(_11100111.as_mut_ptr(), _1010000.as_ptr(), _10010111.as_ptr());
    sc_mul::sc25519_mul(_11101011.as_mut_ptr(), _100.as_ptr(), _11100111.as_ptr());
    sc_mul::sc25519_mul(_11110101.as_mut_ptr(), _1010.as_ptr(), _11101011.as_ptr());

    sc_mul::sc25519_mul(recip, _1011.as_ptr(), _11110101.as_ptr());
    sc25519_sqmul(recip, 126, _1010011.as_ptr());
    sc25519_sqmul(recip, 9, _10.as_ptr());
    sc_mul::sc25519_mul(recip, recip, _11110101.as_ptr());
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

/*
 Input:
 s[0]+256*s[1]+...+256^63*s[63] = s
 *
 Output:
 s[0]+256*s[1]+...+256^31*s[31] = s mod l
 where l = 2^252 + 27742317777372353535851937790883648493.
 Overwrites s in place.
 */

pub unsafe fn sc25519_reduce(s: *mut u8) {
    let mut s0: i64 = (2097151 & load_3(s)) as i64;
    let mut s1: i64 = (2097151 & (load_4(s.add(2)) >> 5)) as i64;
    let mut s2: i64 = (2097151 & (load_3(s.add(5)) >> 2)) as i64;
    let mut s3: i64 = (2097151 & (load_4(s.add(7)) >> 7)) as i64;
    let mut s4: i64 = (2097151 & (load_4(s.add(10)) >> 4)) as i64;
    let mut s5: i64 = (2097151 & (load_3(s.add(13)) >> 1)) as i64;
    let mut s6: i64 = (2097151 & (load_4(s.add(15)) >> 6)) as i64;
    let mut s7: i64 = (2097151 & (load_3(s.add(18)) >> 3)) as i64;
    let mut s8: i64 = (2097151 & load_3(s.add(21))) as i64;
    let mut s9: i64 = (2097151 & (load_4(s.add(23)) >> 5)) as i64;
    let mut s10: i64 = (2097151 & (load_3(s.add(26)) >> 2)) as i64;
    let mut s11: i64 = (2097151 & (load_4(s.add(28)) >> 7)) as i64;
    let mut s12: i64 = (2097151 & (load_4(s.add(31)) >> 4)) as i64;
    let mut s13: i64 = (2097151 & (load_3(s.add(34)) >> 1)) as i64;
    let mut s14: i64 = (2097151 & (load_4(s.add(36)) >> 6)) as i64;
    let mut s15: i64 = (2097151 & (load_3(s.add(39)) >> 3)) as i64;
    let mut s16: i64 = (2097151 & load_3(s.add(42))) as i64;
    let mut s17: i64 = (2097151 & (load_4(s.add(44)) >> 5)) as i64;
    let mut s18: i64 = (2097151 & (load_3(s.add(47)) >> 2)) as i64;
    let mut s19: i64 = (2097151 & (load_4(s.add(49)) >> 7)) as i64;
    let mut s20: i64 = (2097151 & (load_4(s.add(52)) >> 4)) as i64;
    let mut s21: i64 = (2097151 & (load_3(s.add(55)) >> 1)) as i64;
    let mut s22: i64 = (2097151 & (load_4(s.add(57)) >> 6)) as i64;
    let mut s23: i64 = ((load_4(s.add(60)) >> 3)) as i64;

    let mut carry0: i64 = 0;
    let mut carry1: i64 = 0;
    let mut carry2: i64 = 0;
    let mut carry3: i64 = 0;
    let mut carry4: i64 = 0;
    let mut carry5: i64 = 0;
    let mut carry6: i64 = 0;
    let mut carry7: i64 = 0;
    let mut carry8: i64 = 0;
    let mut carry9: i64 = 0;
    let mut carry10: i64 = 0;
    let mut carry11: i64 = 0;
    let mut carry12: i64 = 0;
    let mut carry13: i64 = 0;
    let mut carry14: i64 = 0;
    let mut carry15: i64 = 0;
    let mut carry16: i64 = 0;

    s11 = s11.wrapping_add(s23.wrapping_mul(666643));
    s12 = s12.wrapping_add(s23.wrapping_mul(470296));
    s13 = s13.wrapping_add(s23.wrapping_mul(654183));
    s14 = s14.wrapping_sub(s23.wrapping_mul(997805));
    s15 = s15.wrapping_add(s23.wrapping_mul(136657));
    s16 = s16.wrapping_sub(s23.wrapping_mul(683901));

    s10 = s10.wrapping_add(s22.wrapping_mul(666643));
    s11 = s11.wrapping_add(s22.wrapping_mul(470296));
    s12 = s12.wrapping_add(s22.wrapping_mul(654183));
    s13 = s13.wrapping_sub(s22.wrapping_mul(997805));
    s14 = s14.wrapping_add(s22.wrapping_mul(136657));
    s15 = s15.wrapping_sub(s22.wrapping_mul(683901));

    s9 = s9.wrapping_add(s21.wrapping_mul(666643));
    s10 = s10.wrapping_add(s21.wrapping_mul(470296));
    s11 = s11.wrapping_add(s21.wrapping_mul(654183));
    s12 = s12.wrapping_sub(s21.wrapping_mul(997805));
    s13 = s13.wrapping_add(s21.wrapping_mul(136657));
    s14 = s14.wrapping_sub(s21.wrapping_mul(683901));

    s8 = s8.wrapping_add(s20.wrapping_mul(666643));
    s9 = s9.wrapping_add(s20.wrapping_mul(470296));
    s10 = s10.wrapping_add(s20.wrapping_mul(654183));
    s11 = s11.wrapping_sub(s20.wrapping_mul(997805));
    s12 = s12.wrapping_add(s20.wrapping_mul(136657));
    s13 = s13.wrapping_sub(s20.wrapping_mul(683901));

    s7 = s7.wrapping_add(s19.wrapping_mul(666643));
    s8 = s8.wrapping_add(s19.wrapping_mul(470296));
    s9 = s9.wrapping_add(s19.wrapping_mul(654183));
    s10 = s10.wrapping_sub(s19.wrapping_mul(997805));
    s11 = s11.wrapping_add(s19.wrapping_mul(136657));
    s12 = s12.wrapping_sub(s19.wrapping_mul(683901));

    s6 = s6.wrapping_add(s18.wrapping_mul(666643));
    s7 = s7.wrapping_add(s18.wrapping_mul(470296));
    s8 = s8.wrapping_add(s18.wrapping_mul(654183));
    s9 = s9.wrapping_sub(s18.wrapping_mul(997805));
    s10 = s10.wrapping_add(s18.wrapping_mul(136657));
    s11 = s11.wrapping_sub(s18.wrapping_mul(683901));

    carry6 = (s6.wrapping_add(1i64 << 20)) >> 21;
    s7 = s7.wrapping_add(carry6);
    s6 = s6.wrapping_sub(carry6.wrapping_mul(1i64 << 21));
    carry8 = (s8.wrapping_add(1i64 << 20)) >> 21;
    s9 = s9.wrapping_add(carry8);
    s8 = s8.wrapping_sub(carry8.wrapping_mul(1i64 << 21));
    carry10 = (s10.wrapping_add(1i64 << 20)) >> 21;
    s11 = s11.wrapping_add(carry10);
    s10 = s10.wrapping_sub(carry10.wrapping_mul(1i64 << 21));
    carry12 = (s12.wrapping_add(1i64 << 20)) >> 21;
    s13 = s13.wrapping_add(carry12);
    s12 = s12.wrapping_sub(carry12.wrapping_mul(1i64 << 21));
    carry14 = (s14.wrapping_add(1i64 << 20)) >> 21;
    s15 = s15.wrapping_add(carry14);
    s14 = s14.wrapping_sub(carry14.wrapping_mul(1i64 << 21));
    carry16 = (s16.wrapping_add(1i64 << 20)) >> 21;
    s17 = s17.wrapping_add(carry16);
    s16 = s16.wrapping_sub(carry16.wrapping_mul(1i64 << 21));

    carry7 = (s7.wrapping_add(1i64 << 20)) >> 21;
    s8 = s8.wrapping_add(carry7);
    s7 = s7.wrapping_sub(carry7.wrapping_mul(1i64 << 21));
    carry9 = (s9.wrapping_add(1i64 << 20)) >> 21;
    s10 = s10.wrapping_add(carry9);
    s9 = s9.wrapping_sub(carry9.wrapping_mul(1i64 << 21));
    carry11 = (s11.wrapping_add(1i64 << 20)) >> 21;
    s12 = s12.wrapping_add(carry11);
    s11 = s11.wrapping_sub(carry11.wrapping_mul(1i64 << 21));
    carry13 = (s13.wrapping_add(1i64 << 20)) >> 21;
    s14 = s14.wrapping_add(carry13);
    s13 = s13.wrapping_sub(carry13.wrapping_mul(1i64 << 21));
    carry15 = (s15.wrapping_add(1i64 << 20)) >> 21;
    s16 = s16.wrapping_add(carry15);
    s15 = s15.wrapping_sub(carry15.wrapping_mul(1i64 << 21));

    s5 = s5.wrapping_add(s17.wrapping_mul(666643));
    s6 = s6.wrapping_add(s17.wrapping_mul(470296));
    s7 = s7.wrapping_add(s17.wrapping_mul(654183));
    s8 = s8.wrapping_sub(s17.wrapping_mul(997805));
    s9 = s9.wrapping_add(s17.wrapping_mul(136657));
    s10 = s10.wrapping_sub(s17.wrapping_mul(683901));

    s4 = s4.wrapping_add(s16.wrapping_mul(666643));
    s5 = s5.wrapping_add(s16.wrapping_mul(470296));
    s6 = s6.wrapping_add(s16.wrapping_mul(654183));
    s7 = s7.wrapping_sub(s16.wrapping_mul(997805));
    s8 = s8.wrapping_add(s16.wrapping_mul(136657));
    s9 = s9.wrapping_sub(s16.wrapping_mul(683901));

    s3 = s3.wrapping_add(s15.wrapping_mul(666643));
    s4 = s4.wrapping_add(s15.wrapping_mul(470296));
    s5 = s5.wrapping_add(s15.wrapping_mul(654183));
    s6 = s6.wrapping_sub(s15.wrapping_mul(997805));
    s7 = s7.wrapping_add(s15.wrapping_mul(136657));
    s8 = s8.wrapping_sub(s15.wrapping_mul(683901));

    s2 = s2.wrapping_add(s14.wrapping_mul(666643));
    s3 = s3.wrapping_add(s14.wrapping_mul(470296));
    s4 = s4.wrapping_add(s14.wrapping_mul(654183));
    s5 = s5.wrapping_sub(s14.wrapping_mul(997805));
    s6 = s6.wrapping_add(s14.wrapping_mul(136657));
    s7 = s7.wrapping_sub(s14.wrapping_mul(683901));

    s1 = s1.wrapping_add(s13.wrapping_mul(666643));
    s2 = s2.wrapping_add(s13.wrapping_mul(470296));
    s3 = s3.wrapping_add(s13.wrapping_mul(654183));
    s4 = s4.wrapping_sub(s13.wrapping_mul(997805));
    s5 = s5.wrapping_add(s13.wrapping_mul(136657));
    s6 = s6.wrapping_sub(s13.wrapping_mul(683901));

    s0 = s0.wrapping_add(s12.wrapping_mul(666643));
    s1 = s1.wrapping_add(s12.wrapping_mul(470296));
    s2 = s2.wrapping_add(s12.wrapping_mul(654183));
    s3 = s3.wrapping_sub(s12.wrapping_mul(997805));
    s4 = s4.wrapping_add(s12.wrapping_mul(136657));
    s5 = s5.wrapping_sub(s12.wrapping_mul(683901));
    s12 = 0;

    carry0 = (s0.wrapping_add(1i64 << 20)) >> 21;
    s1 = s1.wrapping_add(carry0);
    s0 = s0.wrapping_sub(carry0.wrapping_mul(1i64 << 21));
    carry2 = (s2.wrapping_add(1i64 << 20)) >> 21;
    s3 = s3.wrapping_add(carry2);
    s2 = s2.wrapping_sub(carry2.wrapping_mul(1i64 << 21));
    carry4 = (s4.wrapping_add(1i64 << 20)) >> 21;
    s5 = s5.wrapping_add(carry4);
    s4 = s4.wrapping_sub(carry4.wrapping_mul(1i64 << 21));
    carry6 = (s6.wrapping_add(1i64 << 20)) >> 21;
    s7 = s7.wrapping_add(carry6);
    s6 = s6.wrapping_sub(carry6.wrapping_mul(1i64 << 21));
    carry8 = (s8.wrapping_add(1i64 << 20)) >> 21;
    s9 = s9.wrapping_add(carry8);
    s8 = s8.wrapping_sub(carry8.wrapping_mul(1i64 << 21));
    carry10 = (s10.wrapping_add(1i64 << 20)) >> 21;
    s11 = s11.wrapping_add(carry10);
    s10 = s10.wrapping_sub(carry10.wrapping_mul(1i64 << 21));

    carry1 = (s1.wrapping_add(1i64 << 20)) >> 21;
    s2 = s2.wrapping_add(carry1);
    s1 = s1.wrapping_sub(carry1.wrapping_mul(1i64 << 21));
    carry3 = (s3.wrapping_add(1i64 << 20)) >> 21;
    s4 = s4.wrapping_add(carry3);
    s3 = s3.wrapping_sub(carry3.wrapping_mul(1i64 << 21));
    carry5 = (s5.wrapping_add(1i64 << 20)) >> 21;
    s6 = s6.wrapping_add(carry5);
    s5 = s5.wrapping_sub(carry5.wrapping_mul(1i64 << 21));
    carry7 = (s7.wrapping_add(1i64 << 20)) >> 21;
    s8 = s8.wrapping_add(carry7);
    s7 = s7.wrapping_sub(carry7.wrapping_mul(1i64 << 21));
    carry9 = (s9.wrapping_add(1i64 << 20)) >> 21;
    s10 = s10.wrapping_add(carry9);
    s9 = s9.wrapping_sub(carry9.wrapping_mul(1i64 << 21));
    carry11 = (s11.wrapping_add(1i64 << 20)) >> 21;
    s12 = s12.wrapping_add(carry11);
    s11 = s11.wrapping_sub(carry11.wrapping_mul(1i64 << 21));

    s0 = s0.wrapping_add(s12.wrapping_mul(666643));
    s1 = s1.wrapping_add(s12.wrapping_mul(470296));
    s2 = s2.wrapping_add(s12.wrapping_mul(654183));
    s3 = s3.wrapping_sub(s12.wrapping_mul(997805));
    s4 = s4.wrapping_add(s12.wrapping_mul(136657));
    s5 = s5.wrapping_sub(s12.wrapping_mul(683901));
    s12 = 0;

    carry0 = s0 >> 21;
    s1 = s1.wrapping_add(carry0);
    s0 = s0.wrapping_sub(carry0.wrapping_mul(1i64 << 21));
    carry1 = s1 >> 21;
    s2 = s2.wrapping_add(carry1);
    s1 = s1.wrapping_sub(carry1.wrapping_mul(1i64 << 21));
    carry2 = s2 >> 21;
    s3 = s3.wrapping_add(carry2);
    s2 = s2.wrapping_sub(carry2.wrapping_mul(1i64 << 21));
    carry3 = s3 >> 21;
    s4 = s4.wrapping_add(carry3);
    s3 = s3.wrapping_sub(carry3.wrapping_mul(1i64 << 21));
    carry4 = s4 >> 21;
    s5 = s5.wrapping_add(carry4);
    s4 = s4.wrapping_sub(carry4.wrapping_mul(1i64 << 21));
    carry5 = s5 >> 21;
    s6 = s6.wrapping_add(carry5);
    s5 = s5.wrapping_sub(carry5.wrapping_mul(1i64 << 21));
    carry6 = s6 >> 21;
    s7 = s7.wrapping_add(carry6);
    s6 = s6.wrapping_sub(carry6.wrapping_mul(1i64 << 21));
    carry7 = s7 >> 21;
    s8 = s8.wrapping_add(carry7);
    s7 = s7.wrapping_sub(carry7.wrapping_mul(1i64 << 21));
    carry8 = s8 >> 21;
    s9 = s9.wrapping_add(carry8);
    s8 = s8.wrapping_sub(carry8.wrapping_mul(1i64 << 21));
    carry9 = s9 >> 21;
    s10 = s10.wrapping_add(carry9);
    s9 = s9.wrapping_sub(carry9.wrapping_mul(1i64 << 21));
    carry10 = s10 >> 21;
    s11 = s11.wrapping_add(carry10);
    s10 = s10.wrapping_sub(carry10.wrapping_mul(1i64 << 21));
    carry11 = s11 >> 21;
    s12 = s12.wrapping_add(carry11);
    s11 = s11.wrapping_sub(carry11.wrapping_mul(1i64 << 21));

    s0 = s0.wrapping_add(s12.wrapping_mul(666643));
    s1 = s1.wrapping_add(s12.wrapping_mul(470296));
    s2 = s2.wrapping_add(s12.wrapping_mul(654183));
    s3 = s3.wrapping_sub(s12.wrapping_mul(997805));
    s4 = s4.wrapping_add(s12.wrapping_mul(136657));
    s5 = s5.wrapping_sub(s12.wrapping_mul(683901));

    carry0 = s0 >> 21;
    s1 = s1.wrapping_add(carry0);
    s0 = s0.wrapping_sub(carry0.wrapping_mul(1i64 << 21));
    carry1 = s1 >> 21;
    s2 = s2.wrapping_add(carry1);
    s1 = s1.wrapping_sub(carry1.wrapping_mul(1i64 << 21));
    carry2 = s2 >> 21;
    s3 = s3.wrapping_add(carry2);
    s2 = s2.wrapping_sub(carry2.wrapping_mul(1i64 << 21));
    carry3 = s3 >> 21;
    s4 = s4.wrapping_add(carry3);
    s3 = s3.wrapping_sub(carry3.wrapping_mul(1i64 << 21));
    carry4 = s4 >> 21;
    s5 = s5.wrapping_add(carry4);
    s4 = s4.wrapping_sub(carry4.wrapping_mul(1i64 << 21));
    carry5 = s5 >> 21;
    s6 = s6.wrapping_add(carry5);
    s5 = s5.wrapping_sub(carry5.wrapping_mul(1i64 << 21));
    carry6 = s6 >> 21;
    s7 = s7.wrapping_add(carry6);
    s6 = s6.wrapping_sub(carry6.wrapping_mul(1i64 << 21));
    carry7 = s7 >> 21;
    s8 = s8.wrapping_add(carry7);
    s7 = s7.wrapping_sub(carry7.wrapping_mul(1i64 << 21));
    carry8 = s8 >> 21;
    s9 = s9.wrapping_add(carry8);
    s8 = s8.wrapping_sub(carry8.wrapping_mul(1i64 << 21));
    carry9 = s9 >> 21;
    s10 = s10.wrapping_add(carry9);
    s9 = s9.wrapping_sub(carry9.wrapping_mul(1i64 << 21));
    carry10 = s10 >> 21;
    s11 = s11.wrapping_add(carry10);
    s10 = s10.wrapping_sub(carry10.wrapping_mul(1i64 << 21));

    *s.add(0) = (s0 >> 0) as u8;
    *s.add(1) = (s0 >> 8) as u8;
    *s.add(2) = ((s0 >> 16) | (s1.wrapping_mul(1i64 << 5))) as u8;
    *s.add(3) = (s1 >> 3) as u8;
    *s.add(4) = (s1 >> 11) as u8;
    *s.add(5) = ((s1 >> 19) | (s2.wrapping_mul(1i64 << 2))) as u8;
    *s.add(6) = (s2 >> 6) as u8;
    *s.add(7) = ((s2 >> 14) | (s3.wrapping_mul(1i64 << 7))) as u8;
    *s.add(8) = (s3 >> 1) as u8;
    *s.add(9) = (s3 >> 9) as u8;
    *s.add(10) = ((s3 >> 17) | (s4.wrapping_mul(1i64 << 4))) as u8;
    *s.add(11) = (s4 >> 4) as u8;
    *s.add(12) = (s4 >> 12) as u8;
    *s.add(13) = ((s4 >> 20) | (s5.wrapping_mul(1i64 << 1))) as u8;
    *s.add(14) = (s5 >> 7) as u8;
    *s.add(15) = ((s5 >> 15) | (s6.wrapping_mul(1i64 << 6))) as u8;
    *s.add(16) = (s6 >> 2) as u8;
    *s.add(17) = (s6 >> 10) as u8;
    *s.add(18) = ((s6 >> 18) | (s7.wrapping_mul(1i64 << 3))) as u8;
    *s.add(19) = (s7 >> 5) as u8;
    *s.add(20) = (s7 >> 13) as u8;
    *s.add(21) = (s8 >> 0) as u8;
    *s.add(22) = (s8 >> 8) as u8;
    *s.add(23) = ((s8 >> 16) | (s9.wrapping_mul(1i64 << 5))) as u8;
    *s.add(24) = (s9 >> 3) as u8;
    *s.add(25) = (s9 >> 11) as u8;
    *s.add(26) = ((s9 >> 19) | (s10.wrapping_mul(1i64 << 2))) as u8;
    *s.add(27) = (s10 >> 6) as u8;
    *s.add(28) = ((s10 >> 14) | (s11.wrapping_mul(1i64 << 7))) as u8;
    *s.add(29) = (s11 >> 1) as u8;
    *s.add(30) = (s11 >> 9) as u8;
    *s.add(31) = (s11 >> 17) as u8;
}

pub unsafe fn sc25519_is_canonical(s: *const u8) -> c_int {
    /* 2^252+27742317777372353535851937790883648493 */
    static L: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58,
        0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
    ];
    let mut c: u8 = 0;
    let mut n: u8 = 1;
    let mut i: u32 = 32;

    loop {
        i -= 1;
        c |= ((((*s.add(i as usize) as c_int) - (L[i as usize] as c_int)) >> 8)
            & (n as c_int)) as u8;
        n &= ((((*s.add(i as usize) as c_int) ^ (L[i as usize] as c_int)) - 1) >> 8) as u8;
        if i == 0 {
            break;
        }
    }

    (c != 0) as c_int
}

/* ---------------------------------------------------------------------------
 * Exported entry points (names after `private/quirks.h` renaming).
 * ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_invert(recip: *mut u8, s: *const u8) {
    sc25519_invert(recip, s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_reduce(s: *mut u8) {
    sc25519_reduce(s)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_mul(s: *mut u8, a: *const u8, b: *const u8) {
    sc_mul::sc25519_mul(s, a, b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_muladd(
    s: *mut u8,
    a: *const u8,
    b: *const u8,
    c: *const u8,
) {
    sc_mul::sc25519_muladd(s, a, b, c)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_is_canonical(s: *const u8) -> c_int {
    sc25519_is_canonical(s)
}
