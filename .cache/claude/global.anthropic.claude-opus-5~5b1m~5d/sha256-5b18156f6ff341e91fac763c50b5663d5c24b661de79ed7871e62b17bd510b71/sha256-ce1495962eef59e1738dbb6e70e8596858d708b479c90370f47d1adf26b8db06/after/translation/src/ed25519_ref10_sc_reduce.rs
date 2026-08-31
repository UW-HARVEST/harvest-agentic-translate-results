//! Translation of `sc25519_reduce` from
//! `crypto_core/ed25519/ref10/ed25519_ref10.c` (lines 2248-2572), together
//! with private copies of the `load_3`/`load_4` helpers (lines 10-35).
//!
//! Linker symbol renamed by `private/quirks.h`:
//! `sc25519_reduce` -> `_sodium_sc25519_reduce`.

#[inline]
unsafe fn load_3(input: *const u8) -> u64 {
    let mut result: u64;

    result = *input.add(0) as u64;
    result |= (*input.add(1) as u64) << 8;
    result |= (*input.add(2) as u64) << 16;

    result
}

#[inline]
unsafe fn load_4(input: *const u8) -> u64 {
    let mut result: u64;

    result = *input.add(0) as u64;
    result |= (*input.add(1) as u64) << 8;
    result |= (*input.add(2) as u64) << 16;
    result |= (*input.add(3) as u64) << 24;

    result
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

#[no_mangle]
pub unsafe extern "C" fn _sodium_sc25519_reduce(s: *mut u8) {
    let mut s0: i64 = (2097151i64) & (load_3(s) as i64);
    let mut s1: i64 = (2097151i64) & ((load_4(s.add(2)) as i64) >> 5);
    let mut s2: i64 = (2097151i64) & ((load_3(s.add(5)) as i64) >> 2);
    let mut s3: i64 = (2097151i64) & ((load_4(s.add(7)) as i64) >> 7);
    let mut s4: i64 = (2097151i64) & ((load_4(s.add(10)) as i64) >> 4);
    let mut s5: i64 = (2097151i64) & ((load_3(s.add(13)) as i64) >> 1);
    let mut s6: i64 = (2097151i64) & ((load_4(s.add(15)) as i64) >> 6);
    let mut s7: i64 = (2097151i64) & ((load_3(s.add(18)) as i64) >> 3);
    let mut s8: i64 = (2097151i64) & (load_3(s.add(21)) as i64);
    let mut s9: i64 = (2097151i64) & ((load_4(s.add(23)) as i64) >> 5);
    let mut s10: i64 = (2097151i64) & ((load_3(s.add(26)) as i64) >> 2);
    let mut s11: i64 = (2097151i64) & ((load_4(s.add(28)) as i64) >> 7);
    let mut s12: i64 = (2097151i64) & ((load_4(s.add(31)) as i64) >> 4);
    let mut s13: i64 = (2097151i64) & ((load_3(s.add(34)) as i64) >> 1);
    let mut s14: i64 = (2097151i64) & ((load_4(s.add(36)) as i64) >> 6);
    let mut s15: i64 = (2097151i64) & ((load_3(s.add(39)) as i64) >> 3);
    let mut s16: i64 = (2097151i64) & (load_3(s.add(42)) as i64);
    let mut s17: i64 = (2097151i64) & ((load_4(s.add(44)) as i64) >> 5);
    let mut s18: i64 = (2097151i64) & ((load_3(s.add(47)) as i64) >> 2);
    let mut s19: i64 = (2097151i64) & ((load_4(s.add(49)) as i64) >> 7);
    let mut s20: i64 = (2097151i64) & ((load_4(s.add(52)) as i64) >> 4);
    let mut s21: i64 = (2097151i64) & ((load_3(s.add(55)) as i64) >> 1);
    let mut s22: i64 = (2097151i64) & ((load_4(s.add(57)) as i64) >> 6);
    let mut s23: i64 = (load_4(s.add(60)) as i64) >> 3;

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
    *s.add(2) = ((s0 >> 16) | s1.wrapping_mul(1i64 << 5)) as u8;
    *s.add(3) = (s1 >> 3) as u8;
    *s.add(4) = (s1 >> 11) as u8;
    *s.add(5) = ((s1 >> 19) | s2.wrapping_mul(1i64 << 2)) as u8;
    *s.add(6) = (s2 >> 6) as u8;
    *s.add(7) = ((s2 >> 14) | s3.wrapping_mul(1i64 << 7)) as u8;
    *s.add(8) = (s3 >> 1) as u8;
    *s.add(9) = (s3 >> 9) as u8;
    *s.add(10) = ((s3 >> 17) | s4.wrapping_mul(1i64 << 4)) as u8;
    *s.add(11) = (s4 >> 4) as u8;
    *s.add(12) = (s4 >> 12) as u8;
    *s.add(13) = ((s4 >> 20) | s5.wrapping_mul(1i64 << 1)) as u8;
    *s.add(14) = (s5 >> 7) as u8;
    *s.add(15) = ((s5 >> 15) | s6.wrapping_mul(1i64 << 6)) as u8;
    *s.add(16) = (s6 >> 2) as u8;
    *s.add(17) = (s6 >> 10) as u8;
    *s.add(18) = ((s6 >> 18) | s7.wrapping_mul(1i64 << 3)) as u8;
    *s.add(19) = (s7 >> 5) as u8;
    *s.add(20) = (s7 >> 13) as u8;
    *s.add(21) = (s8 >> 0) as u8;
    *s.add(22) = (s8 >> 8) as u8;
    *s.add(23) = ((s8 >> 16) | s9.wrapping_mul(1i64 << 5)) as u8;
    *s.add(24) = (s9 >> 3) as u8;
    *s.add(25) = (s9 >> 11) as u8;
    *s.add(26) = ((s9 >> 19) | s10.wrapping_mul(1i64 << 2)) as u8;
    *s.add(27) = (s10 >> 6) as u8;
    *s.add(28) = ((s10 >> 14) | s11.wrapping_mul(1i64 << 7)) as u8;
    *s.add(29) = (s11 >> 1) as u8;
    *s.add(30) = (s11 >> 9) as u8;
    *s.add(31) = (s11 >> 17) as u8;
}
