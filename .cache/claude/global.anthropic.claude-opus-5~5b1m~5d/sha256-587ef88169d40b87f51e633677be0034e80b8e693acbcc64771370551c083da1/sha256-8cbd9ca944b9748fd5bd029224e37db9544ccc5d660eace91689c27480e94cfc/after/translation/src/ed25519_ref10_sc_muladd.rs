#[inline]
unsafe fn load_3(inp: *const u8) -> u64 {
    let mut result: u64;

    result = *inp.add(0) as u64;
    result |= (*inp.add(1) as u64) << 8;
    result |= (*inp.add(2) as u64) << 16;

    result
}

#[inline]
unsafe fn load_4(inp: *const u8) -> u64 {
    let mut result: u64;

    result = *inp.add(0) as u64;
    result |= (*inp.add(1) as u64) << 8;
    result |= (*inp.add(2) as u64) << 16;
    result |= (*inp.add(3) as u64) << 24;

    result
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_sc25519_muladd(
    s: *mut u8,
    a: *const u8,
    b: *const u8,
    c: *const u8,
) {
    let mut a0: i64 = (2097151i64) & (load_3(a) as i64);
    let mut a1: i64 = (2097151i64) & ((load_4(a.add(2)) >> 5) as i64);
    let mut a2: i64 = (2097151i64) & ((load_3(a.add(5)) >> 2) as i64);
    let mut a3: i64 = (2097151i64) & ((load_4(a.add(7)) >> 7) as i64);
    let mut a4: i64 = (2097151i64) & ((load_4(a.add(10)) >> 4) as i64);
    let mut a5: i64 = (2097151i64) & ((load_3(a.add(13)) >> 1) as i64);
    let mut a6: i64 = (2097151i64) & ((load_4(a.add(15)) >> 6) as i64);
    let mut a7: i64 = (2097151i64) & ((load_3(a.add(18)) >> 3) as i64);
    let mut a8: i64 = (2097151i64) & (load_3(a.add(21)) as i64);
    let mut a9: i64 = (2097151i64) & ((load_4(a.add(23)) >> 5) as i64);
    let mut a10: i64 = (2097151i64) & ((load_3(a.add(26)) >> 2) as i64);
    let mut a11: i64 = (load_4(a.add(28)) >> 7) as i64;

    let mut b0: i64 = (2097151i64) & (load_3(b) as i64);
    let mut b1: i64 = (2097151i64) & ((load_4(b.add(2)) >> 5) as i64);
    let mut b2: i64 = (2097151i64) & ((load_3(b.add(5)) >> 2) as i64);
    let mut b3: i64 = (2097151i64) & ((load_4(b.add(7)) >> 7) as i64);
    let mut b4: i64 = (2097151i64) & ((load_4(b.add(10)) >> 4) as i64);
    let mut b5: i64 = (2097151i64) & ((load_3(b.add(13)) >> 1) as i64);
    let mut b6: i64 = (2097151i64) & ((load_4(b.add(15)) >> 6) as i64);
    let mut b7: i64 = (2097151i64) & ((load_3(b.add(18)) >> 3) as i64);
    let mut b8: i64 = (2097151i64) & (load_3(b.add(21)) as i64);
    let mut b9: i64 = (2097151i64) & ((load_4(b.add(23)) >> 5) as i64);
    let mut b10: i64 = (2097151i64) & ((load_3(b.add(26)) >> 2) as i64);
    let mut b11: i64 = (load_4(b.add(28)) >> 7) as i64;

    let mut c0: i64 = (2097151i64) & (load_3(c) as i64);
    let mut c1: i64 = (2097151i64) & ((load_4(c.add(2)) >> 5) as i64);
    let mut c2: i64 = (2097151i64) & ((load_3(c.add(5)) >> 2) as i64);
    let mut c3: i64 = (2097151i64) & ((load_4(c.add(7)) >> 7) as i64);
    let mut c4: i64 = (2097151i64) & ((load_4(c.add(10)) >> 4) as i64);
    let mut c5: i64 = (2097151i64) & ((load_3(c.add(13)) >> 1) as i64);
    let mut c6: i64 = (2097151i64) & ((load_4(c.add(15)) >> 6) as i64);
    let mut c7: i64 = (2097151i64) & ((load_3(c.add(18)) >> 3) as i64);
    let mut c8: i64 = (2097151i64) & (load_3(c.add(21)) as i64);
    let mut c9: i64 = (2097151i64) & ((load_4(c.add(23)) >> 5) as i64);
    let mut c10: i64 = (2097151i64) & ((load_3(c.add(26)) >> 2) as i64);
    let mut c11: i64 = (load_4(c.add(28)) >> 7) as i64;

    let mut s0: i64;
    let mut s1: i64;
    let mut s2: i64;
    let mut s3: i64;
    let mut s4: i64;
    let mut s5: i64;
    let mut s6: i64;
    let mut s7: i64;
    let mut s8: i64;
    let mut s9: i64;
    let mut s10: i64;
    let mut s11: i64;
    let mut s12: i64;
    let mut s13: i64;
    let mut s14: i64;
    let mut s15: i64;
    let mut s16: i64;
    let mut s17: i64;
    let mut s18: i64;
    let mut s19: i64;
    let mut s20: i64;
    let mut s21: i64;
    let mut s22: i64;
    let mut s23: i64;

    let mut carry0: i64;
    let mut carry1: i64;
    let mut carry2: i64;
    let mut carry3: i64;
    let mut carry4: i64;
    let mut carry5: i64;
    let mut carry6: i64;
    let mut carry7: i64;
    let mut carry8: i64;
    let mut carry9: i64;
    let mut carry10: i64;
    let mut carry11: i64;
    let mut carry12: i64;
    let mut carry13: i64;
    let mut carry14: i64;
    let mut carry15: i64;
    let mut carry16: i64;
    let mut carry17: i64;
    let mut carry18: i64;
    let mut carry19: i64;
    let mut carry20: i64;
    let mut carry21: i64;
    let mut carry22: i64;

    s0 = c0.wrapping_add(a0.wrapping_mul(b0));
    s1 = c1
        .wrapping_add(a0.wrapping_mul(b1))
        .wrapping_add(a1.wrapping_mul(b0));
    s2 = c2
        .wrapping_add(a0.wrapping_mul(b2))
        .wrapping_add(a1.wrapping_mul(b1))
        .wrapping_add(a2.wrapping_mul(b0));
    s3 = c3
        .wrapping_add(a0.wrapping_mul(b3))
        .wrapping_add(a1.wrapping_mul(b2))
        .wrapping_add(a2.wrapping_mul(b1))
        .wrapping_add(a3.wrapping_mul(b0));
    s4 = c4
        .wrapping_add(a0.wrapping_mul(b4))
        .wrapping_add(a1.wrapping_mul(b3))
        .wrapping_add(a2.wrapping_mul(b2))
        .wrapping_add(a3.wrapping_mul(b1))
        .wrapping_add(a4.wrapping_mul(b0));
    s5 = c5
        .wrapping_add(a0.wrapping_mul(b5))
        .wrapping_add(a1.wrapping_mul(b4))
        .wrapping_add(a2.wrapping_mul(b3))
        .wrapping_add(a3.wrapping_mul(b2))
        .wrapping_add(a4.wrapping_mul(b1))
        .wrapping_add(a5.wrapping_mul(b0));
    s6 = c6
        .wrapping_add(a0.wrapping_mul(b6))
        .wrapping_add(a1.wrapping_mul(b5))
        .wrapping_add(a2.wrapping_mul(b4))
        .wrapping_add(a3.wrapping_mul(b3))
        .wrapping_add(a4.wrapping_mul(b2))
        .wrapping_add(a5.wrapping_mul(b1))
        .wrapping_add(a6.wrapping_mul(b0));
    s7 = c7
        .wrapping_add(a0.wrapping_mul(b7))
        .wrapping_add(a1.wrapping_mul(b6))
        .wrapping_add(a2.wrapping_mul(b5))
        .wrapping_add(a3.wrapping_mul(b4))
        .wrapping_add(a4.wrapping_mul(b3))
        .wrapping_add(a5.wrapping_mul(b2))
        .wrapping_add(a6.wrapping_mul(b1))
        .wrapping_add(a7.wrapping_mul(b0));
    s8 = c8
        .wrapping_add(a0.wrapping_mul(b8))
        .wrapping_add(a1.wrapping_mul(b7))
        .wrapping_add(a2.wrapping_mul(b6))
        .wrapping_add(a3.wrapping_mul(b5))
        .wrapping_add(a4.wrapping_mul(b4))
        .wrapping_add(a5.wrapping_mul(b3))
        .wrapping_add(a6.wrapping_mul(b2))
        .wrapping_add(a7.wrapping_mul(b1))
        .wrapping_add(a8.wrapping_mul(b0));
    s9 = c9
        .wrapping_add(a0.wrapping_mul(b9))
        .wrapping_add(a1.wrapping_mul(b8))
        .wrapping_add(a2.wrapping_mul(b7))
        .wrapping_add(a3.wrapping_mul(b6))
        .wrapping_add(a4.wrapping_mul(b5))
        .wrapping_add(a5.wrapping_mul(b4))
        .wrapping_add(a6.wrapping_mul(b3))
        .wrapping_add(a7.wrapping_mul(b2))
        .wrapping_add(a8.wrapping_mul(b1))
        .wrapping_add(a9.wrapping_mul(b0));
    s10 = c10
        .wrapping_add(a0.wrapping_mul(b10))
        .wrapping_add(a1.wrapping_mul(b9))
        .wrapping_add(a2.wrapping_mul(b8))
        .wrapping_add(a3.wrapping_mul(b7))
        .wrapping_add(a4.wrapping_mul(b6))
        .wrapping_add(a5.wrapping_mul(b5))
        .wrapping_add(a6.wrapping_mul(b4))
        .wrapping_add(a7.wrapping_mul(b3))
        .wrapping_add(a8.wrapping_mul(b2))
        .wrapping_add(a9.wrapping_mul(b1))
        .wrapping_add(a10.wrapping_mul(b0));
    s11 = c11
        .wrapping_add(a0.wrapping_mul(b11))
        .wrapping_add(a1.wrapping_mul(b10))
        .wrapping_add(a2.wrapping_mul(b9))
        .wrapping_add(a3.wrapping_mul(b8))
        .wrapping_add(a4.wrapping_mul(b7))
        .wrapping_add(a5.wrapping_mul(b6))
        .wrapping_add(a6.wrapping_mul(b5))
        .wrapping_add(a7.wrapping_mul(b4))
        .wrapping_add(a8.wrapping_mul(b3))
        .wrapping_add(a9.wrapping_mul(b2))
        .wrapping_add(a10.wrapping_mul(b1))
        .wrapping_add(a11.wrapping_mul(b0));
    s12 = a1
        .wrapping_mul(b11)
        .wrapping_add(a2.wrapping_mul(b10))
        .wrapping_add(a3.wrapping_mul(b9))
        .wrapping_add(a4.wrapping_mul(b8))
        .wrapping_add(a5.wrapping_mul(b7))
        .wrapping_add(a6.wrapping_mul(b6))
        .wrapping_add(a7.wrapping_mul(b5))
        .wrapping_add(a8.wrapping_mul(b4))
        .wrapping_add(a9.wrapping_mul(b3))
        .wrapping_add(a10.wrapping_mul(b2))
        .wrapping_add(a11.wrapping_mul(b1));
    s13 = a2
        .wrapping_mul(b11)
        .wrapping_add(a3.wrapping_mul(b10))
        .wrapping_add(a4.wrapping_mul(b9))
        .wrapping_add(a5.wrapping_mul(b8))
        .wrapping_add(a6.wrapping_mul(b7))
        .wrapping_add(a7.wrapping_mul(b6))
        .wrapping_add(a8.wrapping_mul(b5))
        .wrapping_add(a9.wrapping_mul(b4))
        .wrapping_add(a10.wrapping_mul(b3))
        .wrapping_add(a11.wrapping_mul(b2));
    s14 = a3
        .wrapping_mul(b11)
        .wrapping_add(a4.wrapping_mul(b10))
        .wrapping_add(a5.wrapping_mul(b9))
        .wrapping_add(a6.wrapping_mul(b8))
        .wrapping_add(a7.wrapping_mul(b7))
        .wrapping_add(a8.wrapping_mul(b6))
        .wrapping_add(a9.wrapping_mul(b5))
        .wrapping_add(a10.wrapping_mul(b4))
        .wrapping_add(a11.wrapping_mul(b3));
    s15 = a4
        .wrapping_mul(b11)
        .wrapping_add(a5.wrapping_mul(b10))
        .wrapping_add(a6.wrapping_mul(b9))
        .wrapping_add(a7.wrapping_mul(b8))
        .wrapping_add(a8.wrapping_mul(b7))
        .wrapping_add(a9.wrapping_mul(b6))
        .wrapping_add(a10.wrapping_mul(b5))
        .wrapping_add(a11.wrapping_mul(b4));
    s16 = a5
        .wrapping_mul(b11)
        .wrapping_add(a6.wrapping_mul(b10))
        .wrapping_add(a7.wrapping_mul(b9))
        .wrapping_add(a8.wrapping_mul(b8))
        .wrapping_add(a9.wrapping_mul(b7))
        .wrapping_add(a10.wrapping_mul(b6))
        .wrapping_add(a11.wrapping_mul(b5));
    s17 = a6
        .wrapping_mul(b11)
        .wrapping_add(a7.wrapping_mul(b10))
        .wrapping_add(a8.wrapping_mul(b9))
        .wrapping_add(a9.wrapping_mul(b8))
        .wrapping_add(a10.wrapping_mul(b7))
        .wrapping_add(a11.wrapping_mul(b6));
    s18 = a7
        .wrapping_mul(b11)
        .wrapping_add(a8.wrapping_mul(b10))
        .wrapping_add(a9.wrapping_mul(b9))
        .wrapping_add(a10.wrapping_mul(b8))
        .wrapping_add(a11.wrapping_mul(b7));
    s19 = a8
        .wrapping_mul(b11)
        .wrapping_add(a9.wrapping_mul(b10))
        .wrapping_add(a10.wrapping_mul(b9))
        .wrapping_add(a11.wrapping_mul(b8));
    s20 = a9
        .wrapping_mul(b11)
        .wrapping_add(a10.wrapping_mul(b10))
        .wrapping_add(a11.wrapping_mul(b9));
    s21 = a10.wrapping_mul(b11).wrapping_add(a11.wrapping_mul(b10));
    s22 = a11.wrapping_mul(b11);
    s23 = 0;

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
    carry12 = (s12.wrapping_add(1i64 << 20)) >> 21;
    s13 = s13.wrapping_add(carry12);
    s12 = s12.wrapping_sub(carry12.wrapping_mul(1i64 << 21));
    carry14 = (s14.wrapping_add(1i64 << 20)) >> 21;
    s15 = s15.wrapping_add(carry14);
    s14 = s14.wrapping_sub(carry14.wrapping_mul(1i64 << 21));
    carry16 = (s16.wrapping_add(1i64 << 20)) >> 21;
    s17 = s17.wrapping_add(carry16);
    s16 = s16.wrapping_sub(carry16.wrapping_mul(1i64 << 21));
    carry18 = (s18.wrapping_add(1i64 << 20)) >> 21;
    s19 = s19.wrapping_add(carry18);
    s18 = s18.wrapping_sub(carry18.wrapping_mul(1i64 << 21));
    carry20 = (s20.wrapping_add(1i64 << 20)) >> 21;
    s21 = s21.wrapping_add(carry20);
    s20 = s20.wrapping_sub(carry20.wrapping_mul(1i64 << 21));
    carry22 = (s22.wrapping_add(1i64 << 20)) >> 21;
    s23 = s23.wrapping_add(carry22);
    s22 = s22.wrapping_sub(carry22.wrapping_mul(1i64 << 21));

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
    carry13 = (s13.wrapping_add(1i64 << 20)) >> 21;
    s14 = s14.wrapping_add(carry13);
    s13 = s13.wrapping_sub(carry13.wrapping_mul(1i64 << 21));
    carry15 = (s15.wrapping_add(1i64 << 20)) >> 21;
    s16 = s16.wrapping_add(carry15);
    s15 = s15.wrapping_sub(carry15.wrapping_mul(1i64 << 21));
    carry17 = (s17.wrapping_add(1i64 << 20)) >> 21;
    s18 = s18.wrapping_add(carry17);
    s17 = s17.wrapping_sub(carry17.wrapping_mul(1i64 << 21));
    carry19 = (s19.wrapping_add(1i64 << 20)) >> 21;
    s20 = s20.wrapping_add(carry19);
    s19 = s19.wrapping_sub(carry19.wrapping_mul(1i64 << 21));
    carry21 = (s21.wrapping_add(1i64 << 20)) >> 21;
    s22 = s22.wrapping_add(carry21);
    s21 = s21.wrapping_sub(carry21.wrapping_mul(1i64 << 21));

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
