//! Translated from sc25519_mul/muladd/reduce/invert/is_canonical in ed25519_ref10.c.
use crate::ed25519::fe25519::{load_3, load_4};

pub fn sc_mul(a: &[u8], b: &[u8]) -> [u8; 32] {
    sc_muladd_impl(a, b, None)
}

pub fn sc_muladd(a: &[u8], b: &[u8], c: &[u8]) -> [u8; 32] {
    sc_muladd_impl(a, b, Some(c))
}

fn sc_muladd_impl(a: &[u8], b: &[u8], c: Option<&[u8]>) -> [u8; 32] {
    let a0 = 2097151i64 & load_3(&a[0..]) as i64;
    let a1 = 2097151i64 & (load_4(&a[2..]) >> 5) as i64;
    let a2 = 2097151i64 & (load_3(&a[5..]) >> 2) as i64;
    let a3 = 2097151i64 & (load_4(&a[7..]) >> 7) as i64;
    let a4 = 2097151i64 & (load_4(&a[10..]) >> 4) as i64;
    let a5 = 2097151i64 & (load_3(&a[13..]) >> 1) as i64;
    let a6 = 2097151i64 & (load_4(&a[15..]) >> 6) as i64;
    let a7 = 2097151i64 & (load_3(&a[18..]) >> 3) as i64;
    let a8 = 2097151i64 & load_3(&a[21..]) as i64;
    let a9 = 2097151i64 & (load_4(&a[23..]) >> 5) as i64;
    let a10 = 2097151i64 & (load_3(&a[26..]) >> 2) as i64;
    let a11 = (load_4(&a[28..]) >> 7) as i64;

    let b0 = 2097151i64 & load_3(&b[0..]) as i64;
    let b1 = 2097151i64 & (load_4(&b[2..]) >> 5) as i64;
    let b2 = 2097151i64 & (load_3(&b[5..]) >> 2) as i64;
    let b3 = 2097151i64 & (load_4(&b[7..]) >> 7) as i64;
    let b4 = 2097151i64 & (load_4(&b[10..]) >> 4) as i64;
    let b5 = 2097151i64 & (load_3(&b[13..]) >> 1) as i64;
    let b6 = 2097151i64 & (load_4(&b[15..]) >> 6) as i64;
    let b7 = 2097151i64 & (load_3(&b[18..]) >> 3) as i64;
    let b8 = 2097151i64 & load_3(&b[21..]) as i64;
    let b9 = 2097151i64 & (load_4(&b[23..]) >> 5) as i64;
    let b10 = 2097151i64 & (load_3(&b[26..]) >> 2) as i64;
    let b11 = (load_4(&b[28..]) >> 7) as i64;

    let (c0, c1, c2, c3, c4, c5, c6, c7, c8, c9, c10, c11) = match c {
        Some(c) => (
            2097151i64 & load_3(&c[0..]) as i64,
            2097151i64 & (load_4(&c[2..]) >> 5) as i64,
            2097151i64 & (load_3(&c[5..]) >> 2) as i64,
            2097151i64 & (load_4(&c[7..]) >> 7) as i64,
            2097151i64 & (load_4(&c[10..]) >> 4) as i64,
            2097151i64 & (load_3(&c[13..]) >> 1) as i64,
            2097151i64 & (load_4(&c[15..]) >> 6) as i64,
            2097151i64 & (load_3(&c[18..]) >> 3) as i64,
            2097151i64 & load_3(&c[21..]) as i64,
            2097151i64 & (load_4(&c[23..]) >> 5) as i64,
            2097151i64 & (load_3(&c[26..]) >> 2) as i64,
            (load_4(&c[28..]) >> 7) as i64,
        ),
        None => (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0),
    };

    let mut s0 = c0 + a0 * b0;
    let mut s1 = c1 + a0 * b1 + a1 * b0;
    let mut s2 = c2 + a0 * b2 + a1 * b1 + a2 * b0;
    let mut s3 = c3 + a0 * b3 + a1 * b2 + a2 * b1 + a3 * b0;
    let mut s4 = c4 + a0 * b4 + a1 * b3 + a2 * b2 + a3 * b1 + a4 * b0;
    let mut s5 = c5 + a0 * b5 + a1 * b4 + a2 * b3 + a3 * b2 + a4 * b1 + a5 * b0;
    let mut s6 = c6 + a0 * b6 + a1 * b5 + a2 * b4 + a3 * b3 + a4 * b2 + a5 * b1 + a6 * b0;
    let mut s7 = c7 + a0 * b7 + a1 * b6 + a2 * b5 + a3 * b4 + a4 * b3 + a5 * b2 + a6 * b1 + a7 * b0;
    let mut s8 =
        c8 + a0 * b8 + a1 * b7 + a2 * b6 + a3 * b5 + a4 * b4 + a5 * b3 + a6 * b2 + a7 * b1 + a8 * b0;
    let mut s9 = c9
        + a0 * b9
        + a1 * b8
        + a2 * b7
        + a3 * b6
        + a4 * b5
        + a5 * b4
        + a6 * b3
        + a7 * b2
        + a8 * b1
        + a9 * b0;
    let mut s10 = c10
        + a0 * b10
        + a1 * b9
        + a2 * b8
        + a3 * b7
        + a4 * b6
        + a5 * b5
        + a6 * b4
        + a7 * b3
        + a8 * b2
        + a9 * b1
        + a10 * b0;
    let mut s11 = c11
        + a0 * b11
        + a1 * b10
        + a2 * b9
        + a3 * b8
        + a4 * b7
        + a5 * b6
        + a6 * b5
        + a7 * b4
        + a8 * b3
        + a9 * b2
        + a10 * b1
        + a11 * b0;
    let mut s12 = a1 * b11
        + a2 * b10
        + a3 * b9
        + a4 * b8
        + a5 * b7
        + a6 * b6
        + a7 * b5
        + a8 * b4
        + a9 * b3
        + a10 * b2
        + a11 * b1;
    let mut s13 = a2 * b11
        + a3 * b10
        + a4 * b9
        + a5 * b8
        + a6 * b7
        + a7 * b6
        + a8 * b5
        + a9 * b4
        + a10 * b3
        + a11 * b2;
    let mut s14 =
        a3 * b11 + a4 * b10 + a5 * b9 + a6 * b8 + a7 * b7 + a8 * b6 + a9 * b5 + a10 * b4 + a11 * b3;
    let mut s15 = a4 * b11 + a5 * b10 + a6 * b9 + a7 * b8 + a8 * b7 + a9 * b6 + a10 * b5 + a11 * b4;
    let mut s16 = a5 * b11 + a6 * b10 + a7 * b9 + a8 * b8 + a9 * b7 + a10 * b6 + a11 * b5;
    let mut s17 = a6 * b11 + a7 * b10 + a8 * b9 + a9 * b8 + a10 * b7 + a11 * b6;
    let mut s18 = a7 * b11 + a8 * b10 + a9 * b9 + a10 * b8 + a11 * b7;
    let mut s19 = a8 * b11 + a9 * b10 + a10 * b9 + a11 * b8;
    let mut s20 = a9 * b11 + a10 * b10 + a11 * b9;
    let mut s21 = a10 * b11 + a11 * b10;
    let mut s22 = a11 * b11;
    let mut s23 = 0i64;

    let mut carry = [0i64; 23];

    macro_rules! c0carry {
        ($idx:expr, $lo:ident, $hi:ident) => {{
            carry[$idx] = ($lo + (1i64 << 20)) >> 21;
            $hi += carry[$idx];
            $lo -= carry[$idx] * (1i64 << 21);
        }};
    }

    c0carry!(0, s0, s1);
    c0carry!(2, s2, s3);
    c0carry!(4, s4, s5);
    c0carry!(6, s6, s7);
    c0carry!(8, s8, s9);
    c0carry!(10, s10, s11);
    c0carry!(12, s12, s13);
    c0carry!(14, s14, s15);
    c0carry!(16, s16, s17);
    c0carry!(18, s18, s19);
    c0carry!(20, s20, s21);
    c0carry!(22, s22, s23);

    c0carry!(1, s1, s2);
    c0carry!(3, s3, s4);
    c0carry!(5, s5, s6);
    c0carry!(7, s7, s8);
    c0carry!(9, s9, s10);
    c0carry!(11, s11, s12);
    c0carry!(13, s13, s14);
    c0carry!(15, s15, s16);
    c0carry!(17, s17, s18);
    c0carry!(19, s19, s20);
    c0carry!(21, s21, s22);

    macro_rules! reduce_word {
        ($sw:ident, $s5:ident, $s4:ident, $s3:ident, $s2:ident, $s1:ident, $s0:ident) => {{
            $s5 += $sw * 666643;
            $s4 += $sw * 470296;
            $s3 += $sw * 654183;
            $s2 -= $sw * 997805;
            $s1 += $sw * 136657;
            $s0 -= $sw * 683901;
        }};
    }

    reduce_word!(s23, s11, s12, s13, s14, s15, s16);
    reduce_word!(s22, s10, s11, s12, s13, s14, s15);
    reduce_word!(s21, s9, s10, s11, s12, s13, s14);
    reduce_word!(s20, s8, s9, s10, s11, s12, s13);
    reduce_word!(s19, s7, s8, s9, s10, s11, s12);
    reduce_word!(s18, s6, s7, s8, s9, s10, s11);

    c0carry!(6, s6, s7);
    c0carry!(8, s8, s9);
    c0carry!(10, s10, s11);
    c0carry!(12, s12, s13);
    c0carry!(14, s14, s15);
    c0carry!(16, s16, s17);

    c0carry!(7, s7, s8);
    c0carry!(9, s9, s10);
    c0carry!(11, s11, s12);
    c0carry!(13, s13, s14);
    c0carry!(15, s15, s16);

    reduce_word!(s17, s5, s6, s7, s8, s9, s10);
    reduce_word!(s16, s4, s5, s6, s7, s8, s9);
    reduce_word!(s15, s3, s4, s5, s6, s7, s8);
    reduce_word!(s14, s2, s3, s4, s5, s6, s7);
    reduce_word!(s13, s1, s2, s3, s4, s5, s6);
    reduce_word!(s12, s0, s1, s2, s3, s4, s5);
    s12 = 0;

    c0carry!(0, s0, s1);
    c0carry!(2, s2, s3);
    c0carry!(4, s4, s5);
    c0carry!(6, s6, s7);
    c0carry!(8, s8, s9);
    c0carry!(10, s10, s11);

    c0carry!(1, s1, s2);
    c0carry!(3, s3, s4);
    c0carry!(5, s5, s6);
    c0carry!(7, s7, s8);
    c0carry!(9, s9, s10);
    c0carry!(11, s11, s12);

    reduce_word!(s12, s0, s1, s2, s3, s4, s5);
    s12 = 0;

    macro_rules! fcarry {
        ($idx:expr, $lo:ident, $hi:ident) => {{
            carry[$idx] = $lo >> 21;
            $hi += carry[$idx];
            $lo -= carry[$idx] * (1i64 << 21);
        }};
    }

    fcarry!(0, s0, s1);
    fcarry!(1, s1, s2);
    fcarry!(2, s2, s3);
    fcarry!(3, s3, s4);
    fcarry!(4, s4, s5);
    fcarry!(5, s5, s6);
    fcarry!(6, s6, s7);
    fcarry!(7, s7, s8);
    fcarry!(8, s8, s9);
    fcarry!(9, s9, s10);
    fcarry!(10, s10, s11);
    fcarry!(11, s11, s12);

    reduce_word!(s12, s0, s1, s2, s3, s4, s5);

    fcarry!(0, s0, s1);
    fcarry!(1, s1, s2);
    fcarry!(2, s2, s3);
    fcarry!(3, s3, s4);
    fcarry!(4, s4, s5);
    fcarry!(5, s5, s6);
    fcarry!(6, s6, s7);
    fcarry!(7, s7, s8);
    fcarry!(8, s8, s9);
    fcarry!(9, s9, s10);
    fcarry!(10, s10, s11);

    let _ = (s22, s23);
    pack(&[
        s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11,
    ])
}

pub fn sc_reduce(s: &mut [u8]) {
    let mut s0 = 2097151i64 & load_3(&s[0..]) as i64;
    let mut s1 = 2097151i64 & (load_4(&s[2..]) >> 5) as i64;
    let mut s2 = 2097151i64 & (load_3(&s[5..]) >> 2) as i64;
    let mut s3 = 2097151i64 & (load_4(&s[7..]) >> 7) as i64;
    let mut s4 = 2097151i64 & (load_4(&s[10..]) >> 4) as i64;
    let mut s5 = 2097151i64 & (load_3(&s[13..]) >> 1) as i64;
    let mut s6 = 2097151i64 & (load_4(&s[15..]) >> 6) as i64;
    let mut s7 = 2097151i64 & (load_3(&s[18..]) >> 3) as i64;
    let mut s8 = 2097151i64 & load_3(&s[21..]) as i64;
    let mut s9 = 2097151i64 & (load_4(&s[23..]) >> 5) as i64;
    let mut s10 = 2097151i64 & (load_3(&s[26..]) >> 2) as i64;
    let mut s11 = 2097151i64 & (load_4(&s[28..]) >> 7) as i64;
    let mut s12 = 2097151i64 & (load_4(&s[31..]) >> 4) as i64;
    let mut s13 = 2097151i64 & (load_3(&s[34..]) >> 1) as i64;
    let mut s14 = 2097151i64 & (load_4(&s[36..]) >> 6) as i64;
    let mut s15 = 2097151i64 & (load_3(&s[39..]) >> 3) as i64;
    let mut s16 = 2097151i64 & load_3(&s[42..]) as i64;
    let mut s17 = 2097151i64 & (load_4(&s[44..]) >> 5) as i64;
    let mut s18 = 2097151i64 & (load_3(&s[47..]) >> 2) as i64;
    let mut s19 = 2097151i64 & (load_4(&s[49..]) >> 7) as i64;
    let mut s20 = 2097151i64 & (load_4(&s[52..]) >> 4) as i64;
    let mut s21 = 2097151i64 & (load_3(&s[55..]) >> 1) as i64;
    let mut s22 = 2097151i64 & (load_4(&s[57..]) >> 6) as i64;
    let mut s23 = (load_4(&s[60..]) >> 3) as i64;

    let mut carry = [0i64; 17];

    macro_rules! reduce_word {
        ($sw:ident, $s5:ident, $s4:ident, $s3:ident, $s2:ident, $s1:ident, $s0:ident) => {{
            $s5 += $sw * 666643;
            $s4 += $sw * 470296;
            $s3 += $sw * 654183;
            $s2 -= $sw * 997805;
            $s1 += $sw * 136657;
            $s0 -= $sw * 683901;
        }};
    }

    reduce_word!(s23, s11, s12, s13, s14, s15, s16);
    reduce_word!(s22, s10, s11, s12, s13, s14, s15);
    reduce_word!(s21, s9, s10, s11, s12, s13, s14);
    reduce_word!(s20, s8, s9, s10, s11, s12, s13);
    reduce_word!(s19, s7, s8, s9, s10, s11, s12);
    reduce_word!(s18, s6, s7, s8, s9, s10, s11);

    macro_rules! c0carry {
        ($idx:expr, $lo:ident, $hi:ident) => {{
            carry[$idx] = ($lo + (1i64 << 20)) >> 21;
            $hi += carry[$idx];
            $lo -= carry[$idx] * (1i64 << 21);
        }};
    }

    c0carry!(6, s6, s7);
    c0carry!(8, s8, s9);
    c0carry!(10, s10, s11);
    c0carry!(12, s12, s13);
    c0carry!(14, s14, s15);
    c0carry!(16, s16, s17);

    c0carry!(7, s7, s8);
    c0carry!(9, s9, s10);
    c0carry!(11, s11, s12);
    c0carry!(13, s13, s14);
    c0carry!(15, s15, s16);

    reduce_word!(s17, s5, s6, s7, s8, s9, s10);
    reduce_word!(s16, s4, s5, s6, s7, s8, s9);
    reduce_word!(s15, s3, s4, s5, s6, s7, s8);
    reduce_word!(s14, s2, s3, s4, s5, s6, s7);
    reduce_word!(s13, s1, s2, s3, s4, s5, s6);
    reduce_word!(s12, s0, s1, s2, s3, s4, s5);
    s12 = 0;

    c0carry!(0, s0, s1);
    c0carry!(2, s2, s3);
    c0carry!(4, s4, s5);
    c0carry!(6, s6, s7);
    c0carry!(8, s8, s9);
    c0carry!(10, s10, s11);

    c0carry!(1, s1, s2);
    c0carry!(3, s3, s4);
    c0carry!(5, s5, s6);
    c0carry!(7, s7, s8);
    c0carry!(9, s9, s10);
    c0carry!(11, s11, s12);

    reduce_word!(s12, s0, s1, s2, s3, s4, s5);
    s12 = 0;

    macro_rules! fcarry {
        ($idx:expr, $lo:ident, $hi:ident) => {{
            carry[$idx] = $lo >> 21;
            $hi += carry[$idx];
            $lo -= carry[$idx] * (1i64 << 21);
        }};
    }

    fcarry!(0, s0, s1);
    fcarry!(1, s1, s2);
    fcarry!(2, s2, s3);
    fcarry!(3, s3, s4);
    fcarry!(4, s4, s5);
    fcarry!(5, s5, s6);
    fcarry!(6, s6, s7);
    fcarry!(7, s7, s8);
    fcarry!(8, s8, s9);
    fcarry!(9, s9, s10);
    fcarry!(10, s10, s11);
    fcarry!(11, s11, s12);

    reduce_word!(s12, s0, s1, s2, s3, s4, s5);

    fcarry!(0, s0, s1);
    fcarry!(1, s1, s2);
    fcarry!(2, s2, s3);
    fcarry!(3, s3, s4);
    fcarry!(4, s4, s5);
    fcarry!(5, s5, s6);
    fcarry!(6, s6, s7);
    fcarry!(7, s7, s8);
    fcarry!(8, s8, s9);
    fcarry!(9, s9, s10);
    fcarry!(10, s10, s11);

    let _ = (s17, s18, s19, s20, s21, s22, s23);
    let packed = pack(&[s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11]);
    s[0..32].copy_from_slice(&packed);
}

fn pack(w: &[i64; 12]) -> [u8; 32] {
    let s0 = w[0];
    let s1 = w[1];
    let s2 = w[2];
    let s3 = w[3];
    let s4 = w[4];
    let s5 = w[5];
    let s6 = w[6];
    let s7 = w[7];
    let s8 = w[8];
    let s9 = w[9];
    let s10 = w[10];
    let s11 = w[11];
    let mut s = [0u8; 32];
    s[0] = (s0 >> 0) as u8;
    s[1] = (s0 >> 8) as u8;
    s[2] = ((s0 >> 16) | (s1 * (1 << 5))) as u8;
    s[3] = (s1 >> 3) as u8;
    s[4] = (s1 >> 11) as u8;
    s[5] = ((s1 >> 19) | (s2 * (1 << 2))) as u8;
    s[6] = (s2 >> 6) as u8;
    s[7] = ((s2 >> 14) | (s3 * (1 << 7))) as u8;
    s[8] = (s3 >> 1) as u8;
    s[9] = (s3 >> 9) as u8;
    s[10] = ((s3 >> 17) | (s4 * (1 << 4))) as u8;
    s[11] = (s4 >> 4) as u8;
    s[12] = (s4 >> 12) as u8;
    s[13] = ((s4 >> 20) | (s5 * (1 << 1))) as u8;
    s[14] = (s5 >> 7) as u8;
    s[15] = ((s5 >> 15) | (s6 * (1 << 6))) as u8;
    s[16] = (s6 >> 2) as u8;
    s[17] = (s6 >> 10) as u8;
    s[18] = ((s6 >> 18) | (s7 * (1 << 3))) as u8;
    s[19] = (s7 >> 5) as u8;
    s[20] = (s7 >> 13) as u8;
    s[21] = (s8 >> 0) as u8;
    s[22] = (s8 >> 8) as u8;
    s[23] = ((s8 >> 16) | (s9 * (1 << 5))) as u8;
    s[24] = (s9 >> 3) as u8;
    s[25] = (s9 >> 11) as u8;
    s[26] = ((s9 >> 19) | (s10 * (1 << 2))) as u8;
    s[27] = (s10 >> 6) as u8;
    s[28] = ((s10 >> 14) | (s11 * (1 << 7))) as u8;
    s[29] = (s11 >> 1) as u8;
    s[30] = (s11 >> 9) as u8;
    s[31] = (s11 >> 17) as u8;
    s
}

fn sc_sq(a: &[u8]) -> [u8; 32] {
    sc_mul(a, a)
}

fn sc_sqmul(s: &mut [u8; 32], n: i32, a: &[u8; 32]) {
    for _ in 0..n {
        let sq = sc_sq(s);
        *s = sq;
    }
    let m = sc_mul(s, a);
    *s = m;
}

pub fn sc_invert(s: &[u8]) -> [u8; 32] {
    let s = {
        let mut t = [0u8; 32];
        t.copy_from_slice(&s[0..32]);
        t
    };
    let _10 = sc_sq(&s);
    let _11 = sc_mul(&s, &_10);
    let _100 = sc_mul(&s, &_11);
    let _1000 = sc_sq(&_100);
    let _1010 = sc_mul(&_10, &_1000);
    let _1011 = sc_mul(&s, &_1010);
    let _10000 = sc_sq(&_1000);
    let _10110 = sc_sq(&_1011);
    let _100000 = sc_mul(&_1010, &_10110);
    let _100110 = sc_mul(&_10000, &_10110);
    let _1000000 = sc_sq(&_100000);
    let _1010000 = sc_mul(&_10000, &_1000000);
    let _1010011 = sc_mul(&_11, &_1010000);
    let _1100011 = sc_mul(&_10000, &_1010011);
    let _1100111 = sc_mul(&_100, &_1100011);
    let _1101011 = sc_mul(&_100, &_1100111);
    let _10010011 = sc_mul(&_1000000, &_1010011);
    let _10010111 = sc_mul(&_100, &_10010011);
    let _10111101 = sc_mul(&_100110, &_10010111);
    let _11010011 = sc_mul(&_10110, &_10111101);
    let _11100111 = sc_mul(&_1010000, &_10010111);
    let _11101011 = sc_mul(&_100, &_11100111);
    let _11110101 = sc_mul(&_1010, &_11101011);

    let mut recip = sc_mul(&_1011, &_11110101);
    sc_sqmul(&mut recip, 126, &_1010011);
    sc_sqmul(&mut recip, 9, &_10);
    recip = sc_mul(&recip, &_11110101);
    sc_sqmul(&mut recip, 7, &_1100111);
    sc_sqmul(&mut recip, 9, &_11110101);
    sc_sqmul(&mut recip, 11, &_10111101);
    sc_sqmul(&mut recip, 8, &_11100111);
    sc_sqmul(&mut recip, 9, &_1101011);
    sc_sqmul(&mut recip, 6, &_1011);
    sc_sqmul(&mut recip, 14, &_10010011);
    sc_sqmul(&mut recip, 10, &_1100011);
    sc_sqmul(&mut recip, 9, &_10010111);
    sc_sqmul(&mut recip, 10, &_11110101);
    sc_sqmul(&mut recip, 8, &_11010011);
    sc_sqmul(&mut recip, 8, &_11101011);
    recip
}

pub fn sc_is_canonical(s: &[u8]) -> i32 {
    const L: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde,
        0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x10,
    ];
    let mut c: u8 = 0;
    let mut n: u8 = 1;
    let mut i: usize = 32;
    loop {
        i -= 1;
        c |= (((s[i] as i32 - L[i] as i32) >> 8) as u8) & n;
        n &= (((s[i] ^ L[i]) as i32 - 1) >> 8) as u8;
        if i == 0 {
            break;
        }
    }
    (c != 0) as i32
}

/* ---- exported C-ABI symbols ---- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_mul(s: *mut u8, a: *const u8, b: *const u8) {
    let a = core::slice::from_raw_parts(a, 32);
    let b = core::slice::from_raw_parts(b, 32);
    let r = sc_mul(a, b);
    core::ptr::copy_nonoverlapping(r.as_ptr(), s, 32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_muladd(
    s: *mut u8,
    a: *const u8,
    b: *const u8,
    c: *const u8,
) {
    let a = core::slice::from_raw_parts(a, 32);
    let b = core::slice::from_raw_parts(b, 32);
    let c = core::slice::from_raw_parts(c, 32);
    let r = sc_muladd(a, b, c);
    core::ptr::copy_nonoverlapping(r.as_ptr(), s, 32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_reduce(s: *mut u8) {
    let sl = core::slice::from_raw_parts_mut(s, 64);
    sc_reduce(sl);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_invert(recip: *mut u8, s: *const u8) {
    let s = core::slice::from_raw_parts(s, 32);
    let r = sc_invert(s);
    core::ptr::copy_nonoverlapping(r.as_ptr(), recip, 32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_sc25519_is_canonical(s: *const u8) -> i32 {
    let s = core::slice::from_raw_parts(s, 32);
    sc_is_canonical(s)
}
