// Bit-sliced AES S-box and helper functions for Haraka

pub fn br_dec32le(src: &[u8]) -> u32 {
    src[0] as u32
        | ((src[1] as u32) << 8)
        | ((src[2] as u32) << 16)
        | ((src[3] as u32) << 24)
}

pub fn br_range_dec32le(v: &mut [u32], src: &[u8]) {
    for i in 0..v.len() {
        v[i] = br_dec32le(&src[4 * i..]);
    }
}

pub fn br_enc32le(dst: &mut [u8], x: u32) {
    dst[0] = x as u8;
    dst[1] = (x >> 8) as u8;
    dst[2] = (x >> 16) as u8;
    dst[3] = (x >> 24) as u8;
}

pub fn br_range_enc32le(dst: &mut [u8], v: &[u32]) {
    for i in 0..v.len() {
        br_enc32le(&mut dst[4 * i..], v[i]);
    }
}

pub fn br_aes_ct64_bitslice_sbox(q: &mut [u64; 8]) {
    let (x0, x1, x2, x3, x4, x5, x6, x7) = (q[7], q[6], q[5], q[4], q[3], q[2], q[1], q[0]);
    let y14 = x3 ^ x5; let y13 = x0 ^ x6; let y9 = x0 ^ x3; let y8 = x0 ^ x5;
    let t0 = x1 ^ x2; let y1 = t0 ^ x7; let y4 = y1 ^ x3; let y12 = y13 ^ y14;
    let y2 = y1 ^ x0; let y5 = y1 ^ x6; let y3 = y5 ^ y8;
    let t1 = x4 ^ y12; let y15 = t1 ^ x5; let y20 = t1 ^ x1;
    let y6 = y15 ^ x7; let y10 = y15 ^ t0; let y11 = y20 ^ y9;
    let y7 = x7 ^ y11; let y17 = y10 ^ y11; let y19 = y10 ^ y8;
    let y16 = t0 ^ y11; let y21 = y13 ^ y16; let y18 = x0 ^ y16;
    let t2 = y12 & y15; let t3 = y3 & y6; let t4 = t3 ^ t2; let t5 = y4 & x7; let t6 = t5 ^ t2;
    let t7 = y13 & y16; let t8 = y5 & y1; let t9 = t8 ^ t7; let t10 = y2 & y7; let t11 = t10 ^ t7;
    let t12 = y9 & y11; let t13 = y14 & y17; let t14 = t13 ^ t12; let t15 = y8 & y10; let t16 = t15 ^ t12;
    let t17 = t4 ^ t14; let t18 = t6 ^ t16; let t19 = t9 ^ t14; let t20 = t11 ^ t16;
    let t21 = t17 ^ y20; let t22 = t18 ^ y19; let t23 = t19 ^ y21; let t24 = t20 ^ y18;
    let t25 = t21 ^ t22; let t26 = t21 & t23; let t27 = t24 ^ t26;
    let t28 = t25 & t27; let t29 = t28 ^ t22;
    let t30 = t23 ^ t24; let t31 = t22 ^ t26; let t32 = t31 & t30; let t33 = t32 ^ t24;
    let t34 = t23 ^ t33; let t35 = t27 ^ t33; let t36 = t24 & t35; let t37 = t36 ^ t34;
    let t38 = t27 ^ t36; let t39 = t29 & t38; let t40 = t25 ^ t39;
    let t41 = t40 ^ t37; let t42 = t29 ^ t33; let t43 = t29 ^ t40;
    let t44 = t33 ^ t37; let t45 = t42 ^ t41;
    let z0 = t44 & y15; let z1 = t37 & y6; let z2 = t33 & x7; let z3 = t43 & y16;
    let z4 = t40 & y1; let z5 = t29 & y7; let z6 = t42 & y11; let z7 = t45 & y17;
    let z8 = t41 & y10; let z9 = t44 & y12; let z10 = t37 & y3; let z11 = t33 & y4;
    let z12 = t43 & y13; let z13 = t40 & y5; let z14 = t29 & y2; let z15 = t42 & y9;
    let z16 = t45 & y14; let z17 = t41 & y8;
    let t46 = z15 ^ z16; let t47 = z10 ^ z11; let t48 = z5 ^ z13; let t49 = z9 ^ z10;
    let t50 = z2 ^ z12; let t51 = z2 ^ z5; let t52 = z7 ^ z8; let t53 = z0 ^ z3;
    let t54 = z6 ^ z7; let t55 = z16 ^ z17; let t56 = z12 ^ t48; let t57 = t50 ^ t53;
    let t58 = z4 ^ t46; let t59 = z3 ^ t54; let t60 = t46 ^ t57; let t61 = z14 ^ t57;
    let t62 = t52 ^ t58; let t63 = t49 ^ t58; let t64 = z4 ^ t59; let t65 = t61 ^ t62;
    let t66 = z1 ^ t63; let t67 = t64 ^ t65;
    let s0 = t59 ^ t63; let s6 = t56 ^ !t62; let s7 = t48 ^ !t60;
    let s3 = t53 ^ t66; let s4 = t51 ^ t66; let s5 = t47 ^ t65;
    let s1 = t64 ^ !s3; let s2 = t55 ^ !t67;
    q[7] = s0; q[6] = s1; q[5] = s2; q[4] = s3; q[3] = s4; q[2] = s5; q[1] = s6; q[0] = s7;
}

pub fn br_aes_ct_bitslice_sbox(q: &mut [u32; 8]) {
    let (x0, x1, x2, x3, x4, x5, x6, x7) = (q[7], q[6], q[5], q[4], q[3], q[2], q[1], q[0]);
    let y14 = x3 ^ x5; let y13 = x0 ^ x6; let y9 = x0 ^ x3; let y8 = x0 ^ x5;
    let t0 = x1 ^ x2; let y1 = t0 ^ x7; let y4 = y1 ^ x3; let y12 = y13 ^ y14;
    let y2 = y1 ^ x0; let y5 = y1 ^ x6; let y3 = y5 ^ y8;
    let t1 = x4 ^ y12; let y15 = t1 ^ x5; let y20 = t1 ^ x1;
    let y6 = y15 ^ x7; let y10 = y15 ^ t0; let y11 = y20 ^ y9;
    let y7 = x7 ^ y11; let y17 = y10 ^ y11; let y19 = y10 ^ y8;
    let y16 = t0 ^ y11; let y21 = y13 ^ y16; let y18 = x0 ^ y16;
    let t2 = y12 & y15; let t3 = y3 & y6; let t4 = t3 ^ t2; let t5 = y4 & x7; let t6 = t5 ^ t2;
    let t7 = y13 & y16; let t8 = y5 & y1; let t9 = t8 ^ t7; let t10 = y2 & y7; let t11 = t10 ^ t7;
    let t12 = y9 & y11; let t13 = y14 & y17; let t14 = t13 ^ t12; let t15 = y8 & y10; let t16 = t15 ^ t12;
    let t17 = t4 ^ t14; let t18 = t6 ^ t16; let t19 = t9 ^ t14; let t20 = t11 ^ t16;
    let t21 = t17 ^ y20; let t22 = t18 ^ y19; let t23 = t19 ^ y21; let t24 = t20 ^ y18;
    let t25 = t21 ^ t22; let t26 = t21 & t23; let t27 = t24 ^ t26;
    let t28 = t25 & t27; let t29 = t28 ^ t22;
    let t30 = t23 ^ t24; let t31 = t22 ^ t26; let t32 = t31 & t30; let t33 = t32 ^ t24;
    let t34 = t23 ^ t33; let t35 = t27 ^ t33; let t36 = t24 & t35; let t37 = t36 ^ t34;
    let t38 = t27 ^ t36; let t39 = t29 & t38; let t40 = t25 ^ t39;
    let t41 = t40 ^ t37; let t42 = t29 ^ t33; let t43 = t29 ^ t40;
    let t44 = t33 ^ t37; let t45 = t42 ^ t41;
    let z0 = t44 & y15; let z1 = t37 & y6; let z2 = t33 & x7; let z3 = t43 & y16;
    let z4 = t40 & y1; let z5 = t29 & y7; let z6 = t42 & y11; let z7 = t45 & y17;
    let z8 = t41 & y10; let z9 = t44 & y12; let z10 = t37 & y3; let z11 = t33 & y4;
    let z12 = t43 & y13; let z13 = t40 & y5; let z14 = t29 & y2; let z15 = t42 & y9;
    let z16 = t45 & y14; let z17 = t41 & y8;
    let t46 = z15 ^ z16; let t47 = z10 ^ z11; let t48 = z5 ^ z13; let t49 = z9 ^ z10;
    let t50 = z2 ^ z12; let t51 = z2 ^ z5; let t52 = z7 ^ z8; let t53 = z0 ^ z3;
    let t54 = z6 ^ z7; let t55 = z16 ^ z17; let t56 = z12 ^ t48; let t57 = t50 ^ t53;
    let t58 = z4 ^ t46; let t59 = z3 ^ t54; let t60 = t46 ^ t57; let t61 = z14 ^ t57;
    let t62 = t52 ^ t58; let t63 = t49 ^ t58; let t64 = z4 ^ t59; let t65 = t61 ^ t62;
    let t66 = z1 ^ t63; let t67 = t64 ^ t65;
    let s0 = t59 ^ t63; let s6 = t56 ^ !t62; let s7 = t48 ^ !t60;
    let s3 = t53 ^ t66; let s4 = t51 ^ t66; let s5 = t47 ^ t65;
    let s1 = t64 ^ !s3; let s2 = t55 ^ !t67;
    q[7] = s0; q[6] = s1; q[5] = s2; q[4] = s3; q[3] = s4; q[2] = s5; q[1] = s6; q[0] = s7;
}

pub fn br_aes_ct_ortho(q: &mut [u32; 8]) {
    macro_rules! swapn32 {
        ($cl:expr, $ch:expr, $s:expr, $x:expr, $y:expr) => {
            let a = $x; let b = $y;
            $x = (a & $cl) | ((b & $cl) << $s);
            $y = ((a & $ch) >> $s) | (b & $ch);
        }
    }
    swapn32!(0x55555555u32, 0xAAAAAAAAu32, 1, q[0], q[1]);
    swapn32!(0x55555555u32, 0xAAAAAAAAu32, 1, q[2], q[3]);
    swapn32!(0x55555555u32, 0xAAAAAAAAu32, 1, q[4], q[5]);
    swapn32!(0x55555555u32, 0xAAAAAAAAu32, 1, q[6], q[7]);
    swapn32!(0x33333333u32, 0xCCCCCCCCu32, 2, q[0], q[2]);
    swapn32!(0x33333333u32, 0xCCCCCCCCu32, 2, q[1], q[3]);
    swapn32!(0x33333333u32, 0xCCCCCCCCu32, 2, q[4], q[6]);
    swapn32!(0x33333333u32, 0xCCCCCCCCu32, 2, q[5], q[7]);
    swapn32!(0x0F0F0F0Fu32, 0xF0F0F0F0u32, 4, q[0], q[4]);
    swapn32!(0x0F0F0F0Fu32, 0xF0F0F0F0u32, 4, q[1], q[5]);
    swapn32!(0x0F0F0F0Fu32, 0xF0F0F0F0u32, 4, q[2], q[6]);
    swapn32!(0x0F0F0F0Fu32, 0xF0F0F0F0u32, 4, q[3], q[7]);
}

pub fn br_aes_ct64_ortho(q: &mut [u64; 8]) {
    macro_rules! swapn {
        ($cl:expr, $ch:expr, $s:expr, $x:expr, $y:expr) => {
            let a = $x; let b = $y;
            $x = (a & $cl) | ((b & $cl) << $s);
            $y = ((a & $ch) >> $s) | (b & $ch);
        }
    }
    swapn!(0x5555555555555555u64, 0xAAAAAAAAAAAAAAAAu64, 1, q[0], q[1]);
    swapn!(0x5555555555555555u64, 0xAAAAAAAAAAAAAAAAu64, 1, q[2], q[3]);
    swapn!(0x5555555555555555u64, 0xAAAAAAAAAAAAAAAAu64, 1, q[4], q[5]);
    swapn!(0x5555555555555555u64, 0xAAAAAAAAAAAAAAAAu64, 1, q[6], q[7]);
    swapn!(0x3333333333333333u64, 0xCCCCCCCCCCCCCCCCu64, 2, q[0], q[2]);
    swapn!(0x3333333333333333u64, 0xCCCCCCCCCCCCCCCCu64, 2, q[1], q[3]);
    swapn!(0x3333333333333333u64, 0xCCCCCCCCCCCCCCCCu64, 2, q[4], q[6]);
    swapn!(0x3333333333333333u64, 0xCCCCCCCCCCCCCCCCu64, 2, q[5], q[7]);
    swapn!(0x0F0F0F0F0F0F0F0Fu64, 0xF0F0F0F0F0F0F0F0u64, 4, q[0], q[4]);
    swapn!(0x0F0F0F0F0F0F0F0Fu64, 0xF0F0F0F0F0F0F0F0u64, 4, q[1], q[5]);
    swapn!(0x0F0F0F0F0F0F0F0Fu64, 0xF0F0F0F0F0F0F0F0u64, 4, q[2], q[6]);
    swapn!(0x0F0F0F0F0F0F0F0Fu64, 0xF0F0F0F0F0F0F0F0u64, 4, q[3], q[7]);
}

pub fn br_aes_ct64_interleave_in(q0: &mut u64, q1: &mut u64, w: &[u32]) {
    let mut x0 = w[0] as u64; let mut x1 = w[1] as u64;
    let mut x2 = w[2] as u64; let mut x3 = w[3] as u64;
    x0 |= x0 << 16; x1 |= x1 << 16; x2 |= x2 << 16; x3 |= x3 << 16;
    x0 &= 0x0000FFFF0000FFFFu64; x1 &= 0x0000FFFF0000FFFFu64;
    x2 &= 0x0000FFFF0000FFFFu64; x3 &= 0x0000FFFF0000FFFFu64;
    x0 |= x0 << 8; x1 |= x1 << 8; x2 |= x2 << 8; x3 |= x3 << 8;
    x0 &= 0x00FF00FF00FF00FFu64; x1 &= 0x00FF00FF00FF00FFu64;
    x2 &= 0x00FF00FF00FF00FFu64; x3 &= 0x00FF00FF00FF00FFu64;
    *q0 = x0 | (x2 << 8); *q1 = x1 | (x3 << 8);
}

pub fn br_aes_ct64_interleave_out(w: &mut [u32], q0: u64, q1: u64) {
    let mut x0 = q0 & 0x00FF00FF00FF00FFu64; let mut x1 = q1 & 0x00FF00FF00FF00FFu64;
    let mut x2 = (q0 >> 8) & 0x00FF00FF00FF00FFu64; let mut x3 = (q1 >> 8) & 0x00FF00FF00FF00FFu64;
    x0 |= x0 >> 8; x1 |= x1 >> 8; x2 |= x2 >> 8; x3 |= x3 >> 8;
    x0 &= 0x0000FFFF0000FFFFu64; x1 &= 0x0000FFFF0000FFFFu64;
    x2 &= 0x0000FFFF0000FFFFu64; x3 &= 0x0000FFFF0000FFFFu64;
    w[0] = (x0 as u32) | ((x0 >> 16) as u32); w[1] = (x1 as u32) | ((x1 >> 16) as u32);
    w[2] = (x2 as u32) | ((x2 >> 16) as u32); w[3] = (x3 as u32) | ((x3 >> 16) as u32);
}

pub fn shift_rows(q: &mut [u64; 8]) {
    for i in 0..8 {
        let x = q[i];
        q[i] = (x & 0x000000000000FFFFu64)
            | ((x & 0x00000000FFF00000u64) >> 4)
            | ((x & 0x00000000000F0000u64) << 12)
            | ((x & 0x0000FF0000000000u64) >> 8)
            | ((x & 0x000000FF00000000u64) << 8)
            | ((x & 0xF000000000000000u64) >> 12)
            | ((x & 0x0FFF000000000000u64) << 4);
    }
}

pub fn mix_columns(q: &mut [u64; 8]) {
    let (q0, q1, q2, q3, q4, q5, q6, q7) = (q[0], q[1], q[2], q[3], q[4], q[5], q[6], q[7]);
    let r0 = q0.rotate_right(16); let r1 = q1.rotate_right(16);
    let r2 = q2.rotate_right(16); let r3 = q3.rotate_right(16);
    let r4 = q4.rotate_right(16); let r5 = q5.rotate_right(16);
    let r6 = q6.rotate_right(16); let r7 = q7.rotate_right(16);
    q[0] = q7 ^ r7 ^ r0 ^ (q0 ^ r0).rotate_right(32);
    q[1] = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ (q1 ^ r1).rotate_right(32);
    q[2] = q1 ^ r1 ^ r2 ^ (q2 ^ r2).rotate_right(32);
    q[3] = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ (q3 ^ r3).rotate_right(32);
    q[4] = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ (q4 ^ r4).rotate_right(32);
    q[5] = q4 ^ r4 ^ r5 ^ (q5 ^ r5).rotate_right(32);
    q[6] = q5 ^ r5 ^ r6 ^ (q6 ^ r6).rotate_right(32);
    q[7] = q6 ^ r6 ^ r7 ^ (q7 ^ r7).rotate_right(32);
}

pub fn shift_rows32(q: &mut [u32; 8]) {
    for i in 0..8 {
        let x = q[i];
        q[i] = (x & 0x000000FF)
            | ((x & 0x0000FC00) >> 2) | ((x & 0x00000300) << 6)
            | ((x & 0x00F00000) >> 4) | ((x & 0x000F0000) << 4)
            | ((x & 0xC0000000) >> 6) | ((x & 0x3F000000) << 2);
    }
}

pub fn mix_columns32(q: &mut [u32; 8]) {
    let (q0, q1, q2, q3, q4, q5, q6, q7) = (q[0], q[1], q[2], q[3], q[4], q[5], q[6], q[7]);
    let r0 = q0.rotate_right(8); let r1 = q1.rotate_right(8);
    let r2 = q2.rotate_right(8); let r3 = q3.rotate_right(8);
    let r4 = q4.rotate_right(8); let r5 = q5.rotate_right(8);
    let r6 = q6.rotate_right(8); let r7 = q7.rotate_right(8);
    q[0] = q7 ^ r7 ^ r0 ^ (q0 ^ r0).rotate_right(16);
    q[1] = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ (q1 ^ r1).rotate_right(16);
    q[2] = q1 ^ r1 ^ r2 ^ (q2 ^ r2).rotate_right(16);
    q[3] = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ (q3 ^ r3).rotate_right(16);
    q[4] = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ (q4 ^ r4).rotate_right(16);
    q[5] = q4 ^ r4 ^ r5 ^ (q5 ^ r5).rotate_right(16);
    q[6] = q5 ^ r5 ^ r6 ^ (q6 ^ r6).rotate_right(16);
    q[7] = q6 ^ r6 ^ r7 ^ (q7 ^ r7).rotate_right(16);
}
