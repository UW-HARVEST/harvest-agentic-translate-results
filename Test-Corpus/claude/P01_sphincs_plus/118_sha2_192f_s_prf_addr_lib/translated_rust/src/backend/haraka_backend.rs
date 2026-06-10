// Haraka backend - direct translation from c_src/lib/haraka.

#![allow(non_snake_case)]

use crate::address::addr_to_bytes;
use crate::context::SpxCtx;
use crate::params::*;
use crate::utils::bytes_to_ull_rs;

const HARAKAS_RATE: usize = 32;

// Haraka 512 round constants
const HARAKA512_RC64: [[u64; 8]; 10] = [
    [0x24cf0ab9086f628b, 0xbdd6eeecc83b8382, 0xd96fb0306cdad0a7, 0xaace082ac8f95f89, 0x449d8e8870d7041f, 0x49bb2f80b2b3e2f8, 0x0569ae98d93bb258, 0x23dc9691e7d6a4b1],
    [0xd8ba10ede0fe5b6e, 0x7ecf7dbe424c7b8e, 0x6ea9949c6df62a31, 0xbf3f3c97ec9c313e, 0x241d03a196a1861e, 0xead3a51116e5a2ea, 0x77d479fcad9574e3, 0x18657a1af894b7a0],
    [0x10671e1a7f595522, 0xd9a00ff675d28c7b, 0x2f1edf0d2b9ba661, 0xb8ff58b8e3de45f9, 0xee29261da9865c02, 0xd1532aa4b50bdf43, 0x8bf858159b231bb1, 0xdf17439d22d4f599],
    [0xdd4b2f0870b918c0, 0x757a81f3b39b1bb6, 0x7a5c556898952e3f, 0x7dd70a16d915d87a, 0x3ae61971982b8301, 0xc3ab319e030412be, 0x17c0033ac094a8cb, 0x5a0630fc1a8dc4ef],
    [0x17708988c1632f73, 0xf92ddae090b44f4f, 0x11ac0285c43aa314, 0x509059941936b8ba, 0xd03e152fa2ce9b69, 0x3fbcbcb63a32998b, 0x6204696d692254f7, 0x915542ed93ec59b4],
    [0xf4ed94aa8879236e, 0xff6cb41cd38e03c0, 0x069b38602368aeab, 0x669495b820f0ddba, 0xf42013b1b8bf9e3d, 0xcf935efe6439734d, 0xbc1dcf42ca29e3f8, 0x7e6d3ed29f78ad67],
    [0xf3b0f6837ffcddaa, 0x3a76faef934ddf41, 0xcec7ae583a9c8e35, 0xe4dd18c68f0260af, 0x2c0e5df1ad398eaa, 0x478df5236ae22e8c, 0xfb944c46fe865f39, 0xaa48f82f028132ba],
    [0x231b9ae2b76aca77, 0x292a76a712db0b40, 0x5850625dc8134491, 0x73137dd469810fb5, 0x8a12a6a202a474fd, 0xd36fd9daa78bdb80, 0xb34c5e733505706f, 0xbaf1cdca818d9d96],
    [0x2e99781335e8c641, 0xbddfe5cce47d560e, 0xf74e9bf32e5e040c, 0x1d7a709d65996be9, 0x670df36a9cf66cdd, 0xd05ef84a176a2875, 0x0f888e828cb1c44e, 0x1a79e9c9727b052c],
    [0x83497348628d84de, 0x2e9387d51f22a754, 0xb000068da2f852d6, 0x378c9e1190fd6fe5, 0x870027c316de7293, 0xe51a9d4462e047bb, 0x90ecf7f8c6251195, 0x655953bfbed90a9c],
];

// =========================
// AES bitslicing helpers
// =========================

fn br_dec32le(src: &[u8]) -> u32 {
    (src[0] as u32) | ((src[1] as u32) << 8) | ((src[2] as u32) << 16) | ((src[3] as u32) << 24)
}

fn br_range_dec32le(v: &mut [u32], num: usize, src: &[u8]) {
    for i in 0..num {
        v[i] = br_dec32le(&src[4 * i..]);
    }
}

fn br_enc32le(dst: &mut [u8], x: u32) {
    dst[0] = x as u8;
    dst[1] = (x >> 8) as u8;
    dst[2] = (x >> 16) as u8;
    dst[3] = (x >> 24) as u8;
}

fn br_range_enc32le(dst: &mut [u8], v: &[u32], num: usize) {
    for i in 0..num {
        br_enc32le(&mut dst[4 * i..], v[i]);
    }
}

fn br_aes_ct64_bitslice_sbox(q: &mut [u64; 8]) {
    let x0 = q[7]; let x1 = q[6]; let x2 = q[5]; let x3 = q[4];
    let x4 = q[3]; let x5 = q[2]; let x6 = q[1]; let x7 = q[0];

    let y14 = x3 ^ x5;
    let y13 = x0 ^ x6;
    let y9 = x0 ^ x3;
    let y8 = x0 ^ x5;
    let t0 = x1 ^ x2;
    let y1 = t0 ^ x7;
    let y4 = y1 ^ x3;
    let y12 = y13 ^ y14;
    let y2 = y1 ^ x0;
    let y5 = y1 ^ x6;
    let y3 = y5 ^ y8;
    let t1 = x4 ^ y12;
    let y15 = t1 ^ x5;
    let y20 = t1 ^ x1;
    let y6 = y15 ^ x7;
    let y10 = y15 ^ t0;
    let y11 = y20 ^ y9;
    let y7 = x7 ^ y11;
    let y17 = y10 ^ y11;
    let y19 = y10 ^ y8;
    let y16 = t0 ^ y11;
    let y21 = y13 ^ y16;
    let y18 = x0 ^ y16;

    let t2 = y12 & y15;
    let t3 = y3 & y6;
    let t4 = t3 ^ t2;
    let t5 = y4 & x7;
    let t6 = t5 ^ t2;
    let t7 = y13 & y16;
    let t8 = y5 & y1;
    let t9 = t8 ^ t7;
    let t10 = y2 & y7;
    let t11 = t10 ^ t7;
    let t12 = y9 & y11;
    let t13 = y14 & y17;
    let t14 = t13 ^ t12;
    let t15 = y8 & y10;
    let t16 = t15 ^ t12;
    let t17 = t4 ^ t14;
    let t18 = t6 ^ t16;
    let t19 = t9 ^ t14;
    let t20 = t11 ^ t16;
    let t21 = t17 ^ y20;
    let t22 = t18 ^ y19;
    let t23 = t19 ^ y21;
    let t24 = t20 ^ y18;

    let t25 = t21 ^ t22;
    let t26 = t21 & t23;
    let t27 = t24 ^ t26;
    let t28 = t25 & t27;
    let t29 = t28 ^ t22;
    let t30 = t23 ^ t24;
    let t31 = t22 ^ t26;
    let t32 = t31 & t30;
    let t33 = t32 ^ t24;
    let t34 = t23 ^ t33;
    let t35 = t27 ^ t33;
    let t36 = t24 & t35;
    let t37 = t36 ^ t34;
    let t38 = t27 ^ t36;
    let t39 = t29 & t38;
    let t40 = t25 ^ t39;

    let t41 = t40 ^ t37;
    let t42 = t29 ^ t33;
    let t43 = t29 ^ t40;
    let t44 = t33 ^ t37;
    let t45 = t42 ^ t41;
    let z0 = t44 & y15;
    let z1 = t37 & y6;
    let z2 = t33 & x7;
    let z3 = t43 & y16;
    let z4 = t40 & y1;
    let z5 = t29 & y7;
    let z6 = t42 & y11;
    let z7 = t45 & y17;
    let z8 = t41 & y10;
    let z9 = t44 & y12;
    let z10 = t37 & y3;
    let z11 = t33 & y4;
    let z12 = t43 & y13;
    let z13 = t40 & y5;
    let z14 = t29 & y2;
    let z15 = t42 & y9;
    let z16 = t45 & y14;
    let z17 = t41 & y8;

    let t46 = z15 ^ z16;
    let t47 = z10 ^ z11;
    let t48 = z5 ^ z13;
    let t49 = z9 ^ z10;
    let t50 = z2 ^ z12;
    let t51 = z2 ^ z5;
    let t52 = z7 ^ z8;
    let t53 = z0 ^ z3;
    let t54 = z6 ^ z7;
    let t55 = z16 ^ z17;
    let t56 = z12 ^ t48;
    let t57 = t50 ^ t53;
    let t58 = z4 ^ t46;
    let t59 = z3 ^ t54;
    let t60 = t46 ^ t57;
    let t61 = z14 ^ t57;
    let t62 = t52 ^ t58;
    let t63 = t49 ^ t58;
    let t64 = z4 ^ t59;
    let t65 = t61 ^ t62;
    let t66 = z1 ^ t63;
    let s0 = t59 ^ t63;
    let s6 = t56 ^ !t62;
    let s7 = t48 ^ !t60;
    let t67 = t64 ^ t65;
    let s3 = t53 ^ t66;
    let s4 = t51 ^ t66;
    let s5 = t47 ^ t65;
    let s1 = t64 ^ !s3;
    let s2 = t55 ^ !t67;

    q[7] = s0; q[6] = s1; q[5] = s2; q[4] = s3;
    q[3] = s4; q[2] = s5; q[1] = s6; q[0] = s7;
}

fn br_aes_ct_bitslice_sbox(q: &mut [u32; 8]) {
    let x0 = q[7]; let x1 = q[6]; let x2 = q[5]; let x3 = q[4];
    let x4 = q[3]; let x5 = q[2]; let x6 = q[1]; let x7 = q[0];

    let y14 = x3 ^ x5;
    let y13 = x0 ^ x6;
    let y9 = x0 ^ x3;
    let y8 = x0 ^ x5;
    let t0 = x1 ^ x2;
    let y1 = t0 ^ x7;
    let y4 = y1 ^ x3;
    let y12 = y13 ^ y14;
    let y2 = y1 ^ x0;
    let y5 = y1 ^ x6;
    let y3 = y5 ^ y8;
    let t1 = x4 ^ y12;
    let y15 = t1 ^ x5;
    let y20 = t1 ^ x1;
    let y6 = y15 ^ x7;
    let y10 = y15 ^ t0;
    let y11 = y20 ^ y9;
    let y7 = x7 ^ y11;
    let y17 = y10 ^ y11;
    let y19 = y10 ^ y8;
    let y16 = t0 ^ y11;
    let y21 = y13 ^ y16;
    let y18 = x0 ^ y16;

    let t2 = y12 & y15;
    let t3 = y3 & y6;
    let t4 = t3 ^ t2;
    let t5 = y4 & x7;
    let t6 = t5 ^ t2;
    let t7 = y13 & y16;
    let t8 = y5 & y1;
    let t9 = t8 ^ t7;
    let t10 = y2 & y7;
    let t11 = t10 ^ t7;
    let t12 = y9 & y11;
    let t13 = y14 & y17;
    let t14 = t13 ^ t12;
    let t15 = y8 & y10;
    let t16 = t15 ^ t12;
    let t17 = t4 ^ t14;
    let t18 = t6 ^ t16;
    let t19 = t9 ^ t14;
    let t20 = t11 ^ t16;
    let t21 = t17 ^ y20;
    let t22 = t18 ^ y19;
    let t23 = t19 ^ y21;
    let t24 = t20 ^ y18;

    let t25 = t21 ^ t22;
    let t26 = t21 & t23;
    let t27 = t24 ^ t26;
    let t28 = t25 & t27;
    let t29 = t28 ^ t22;
    let t30 = t23 ^ t24;
    let t31 = t22 ^ t26;
    let t32 = t31 & t30;
    let t33 = t32 ^ t24;
    let t34 = t23 ^ t33;
    let t35 = t27 ^ t33;
    let t36 = t24 & t35;
    let t37 = t36 ^ t34;
    let t38 = t27 ^ t36;
    let t39 = t29 & t38;
    let t40 = t25 ^ t39;

    let t41 = t40 ^ t37;
    let t42 = t29 ^ t33;
    let t43 = t29 ^ t40;
    let t44 = t33 ^ t37;
    let t45 = t42 ^ t41;
    let z0 = t44 & y15;
    let z1 = t37 & y6;
    let z2 = t33 & x7;
    let z3 = t43 & y16;
    let z4 = t40 & y1;
    let z5 = t29 & y7;
    let z6 = t42 & y11;
    let z7 = t45 & y17;
    let z8 = t41 & y10;
    let z9 = t44 & y12;
    let z10 = t37 & y3;
    let z11 = t33 & y4;
    let z12 = t43 & y13;
    let z13 = t40 & y5;
    let z14 = t29 & y2;
    let z15 = t42 & y9;
    let z16 = t45 & y14;
    let z17 = t41 & y8;

    let t46 = z15 ^ z16;
    let t47 = z10 ^ z11;
    let t48 = z5 ^ z13;
    let t49 = z9 ^ z10;
    let t50 = z2 ^ z12;
    let t51 = z2 ^ z5;
    let t52 = z7 ^ z8;
    let t53 = z0 ^ z3;
    let t54 = z6 ^ z7;
    let t55 = z16 ^ z17;
    let t56 = z12 ^ t48;
    let t57 = t50 ^ t53;
    let t58 = z4 ^ t46;
    let t59 = z3 ^ t54;
    let t60 = t46 ^ t57;
    let t61 = z14 ^ t57;
    let t62 = t52 ^ t58;
    let t63 = t49 ^ t58;
    let t64 = z4 ^ t59;
    let t65 = t61 ^ t62;
    let t66 = z1 ^ t63;
    let s0 = t59 ^ t63;
    let s6 = t56 ^ !t62;
    let s7 = t48 ^ !t60;
    let t67 = t64 ^ t65;
    let s3 = t53 ^ t66;
    let s4 = t51 ^ t66;
    let s5 = t47 ^ t65;
    let s1 = t64 ^ !s3;
    let s2 = t55 ^ !t67;

    q[7] = s0; q[6] = s1; q[5] = s2; q[4] = s3;
    q[3] = s4; q[2] = s5; q[1] = s6; q[0] = s7;
}

fn br_aes_ct_ortho(q: &mut [u32; 8]) {
    fn swapn(cl: u32, ch: u32, s: u32, x: &mut u32, y: &mut u32) {
        let a = *x;
        let b = *y;
        *x = (a & cl) | ((b & cl) << s);
        *y = ((a & ch) >> s) | (b & ch);
    }
    let (h, t) = q.split_at_mut(1); swapn(0x55555555, 0xAAAAAAAA, 1, &mut h[0], &mut t[0]);
    let (h, t) = q.split_at_mut(3); swapn(0x55555555, 0xAAAAAAAA, 1, &mut h[2], &mut t[0]);
    let (h, t) = q.split_at_mut(5); swapn(0x55555555, 0xAAAAAAAA, 1, &mut h[4], &mut t[0]);
    let (h, t) = q.split_at_mut(7); swapn(0x55555555, 0xAAAAAAAA, 1, &mut h[6], &mut t[0]);

    let (h, t) = q.split_at_mut(2); swapn(0x33333333, 0xCCCCCCCC, 2, &mut h[0], &mut t[0]);
    let (h, t) = q.split_at_mut(3); swapn(0x33333333, 0xCCCCCCCC, 2, &mut h[1], &mut t[0]);
    let (h, t) = q.split_at_mut(6); swapn(0x33333333, 0xCCCCCCCC, 2, &mut h[4], &mut t[0]);
    let (h, t) = q.split_at_mut(7); swapn(0x33333333, 0xCCCCCCCC, 2, &mut h[5], &mut t[0]);

    let (h, t) = q.split_at_mut(4); swapn(0x0F0F0F0F, 0xF0F0F0F0, 4, &mut h[0], &mut t[0]);
    let (h, t) = q.split_at_mut(5); swapn(0x0F0F0F0F, 0xF0F0F0F0, 4, &mut h[1], &mut t[0]);
    let (h, t) = q.split_at_mut(6); swapn(0x0F0F0F0F, 0xF0F0F0F0, 4, &mut h[2], &mut t[0]);
    let (h, t) = q.split_at_mut(7); swapn(0x0F0F0F0F, 0xF0F0F0F0, 4, &mut h[3], &mut t[0]);
}

fn add_round_key32(q: &mut [u32; 8], sk: &[u32; 8]) {
    for i in 0..8 {
        q[i] ^= sk[i];
    }
}

fn shift_rows32(q: &mut [u32; 8]) {
    for i in 0..8 {
        let x = q[i];
        q[i] = (x & 0x000000FF)
            | ((x & 0x0000FC00) >> 2) | ((x & 0x00000300) << 6)
            | ((x & 0x00F00000) >> 4) | ((x & 0x000F0000) << 4)
            | ((x & 0xC0000000) >> 6) | ((x & 0x3F000000) << 2);
    }
}

fn rotr16_32(x: u32) -> u32 {
    (x << 16) | (x >> 16)
}

fn mix_columns32(q: &mut [u32; 8]) {
    let q0 = q[0]; let q1 = q[1]; let q2 = q[2]; let q3 = q[3];
    let q4 = q[4]; let q5 = q[5]; let q6 = q[6]; let q7 = q[7];
    let r0 = (q0 >> 8) | (q0 << 24);
    let r1 = (q1 >> 8) | (q1 << 24);
    let r2 = (q2 >> 8) | (q2 << 24);
    let r3 = (q3 >> 8) | (q3 << 24);
    let r4 = (q4 >> 8) | (q4 << 24);
    let r5 = (q5 >> 8) | (q5 << 24);
    let r6 = (q6 >> 8) | (q6 << 24);
    let r7 = (q7 >> 8) | (q7 << 24);

    q[0] = q7 ^ r7 ^ r0 ^ rotr16_32(q0 ^ r0);
    q[1] = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ rotr16_32(q1 ^ r1);
    q[2] = q1 ^ r1 ^ r2 ^ rotr16_32(q2 ^ r2);
    q[3] = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ rotr16_32(q3 ^ r3);
    q[4] = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ rotr16_32(q4 ^ r4);
    q[5] = q4 ^ r4 ^ r5 ^ rotr16_32(q5 ^ r5);
    q[6] = q5 ^ r5 ^ r6 ^ rotr16_32(q6 ^ r6);
    q[7] = q6 ^ r6 ^ r7 ^ rotr16_32(q7 ^ r7);
}

fn br_aes_ct64_ortho(q: &mut [u64; 8]) {
    fn swapn(cl: u64, ch: u64, s: u32, x: &mut u64, y: &mut u64) {
        let a = *x;
        let b = *y;
        *x = (a & cl) | ((b & cl) << s);
        *y = ((a & ch) >> s) | (b & ch);
    }
    let (h, t) = q.split_at_mut(1); swapn(0x5555555555555555, 0xAAAAAAAAAAAAAAAA, 1, &mut h[0], &mut t[0]);
    let (h, t) = q.split_at_mut(3); swapn(0x5555555555555555, 0xAAAAAAAAAAAAAAAA, 1, &mut h[2], &mut t[0]);
    let (h, t) = q.split_at_mut(5); swapn(0x5555555555555555, 0xAAAAAAAAAAAAAAAA, 1, &mut h[4], &mut t[0]);
    let (h, t) = q.split_at_mut(7); swapn(0x5555555555555555, 0xAAAAAAAAAAAAAAAA, 1, &mut h[6], &mut t[0]);

    let (h, t) = q.split_at_mut(2); swapn(0x3333333333333333, 0xCCCCCCCCCCCCCCCC, 2, &mut h[0], &mut t[0]);
    let (h, t) = q.split_at_mut(3); swapn(0x3333333333333333, 0xCCCCCCCCCCCCCCCC, 2, &mut h[1], &mut t[0]);
    let (h, t) = q.split_at_mut(6); swapn(0x3333333333333333, 0xCCCCCCCCCCCCCCCC, 2, &mut h[4], &mut t[0]);
    let (h, t) = q.split_at_mut(7); swapn(0x3333333333333333, 0xCCCCCCCCCCCCCCCC, 2, &mut h[5], &mut t[0]);

    let (h, t) = q.split_at_mut(4); swapn(0x0F0F0F0F0F0F0F0F, 0xF0F0F0F0F0F0F0F0, 4, &mut h[0], &mut t[0]);
    let (h, t) = q.split_at_mut(5); swapn(0x0F0F0F0F0F0F0F0F, 0xF0F0F0F0F0F0F0F0, 4, &mut h[1], &mut t[0]);
    let (h, t) = q.split_at_mut(6); swapn(0x0F0F0F0F0F0F0F0F, 0xF0F0F0F0F0F0F0F0, 4, &mut h[2], &mut t[0]);
    let (h, t) = q.split_at_mut(7); swapn(0x0F0F0F0F0F0F0F0F, 0xF0F0F0F0F0F0F0F0, 4, &mut h[3], &mut t[0]);
}

fn br_aes_ct64_interleave_in(q0: &mut u64, q1: &mut u64, w: &[u32]) {
    let mut x0 = w[0] as u64;
    let mut x1 = w[1] as u64;
    let mut x2 = w[2] as u64;
    let mut x3 = w[3] as u64;
    x0 |= x0 << 16;
    x1 |= x1 << 16;
    x2 |= x2 << 16;
    x3 |= x3 << 16;
    x0 &= 0x0000FFFF0000FFFF;
    x1 &= 0x0000FFFF0000FFFF;
    x2 &= 0x0000FFFF0000FFFF;
    x3 &= 0x0000FFFF0000FFFF;
    x0 |= x0 << 8;
    x1 |= x1 << 8;
    x2 |= x2 << 8;
    x3 |= x3 << 8;
    x0 &= 0x00FF00FF00FF00FF;
    x1 &= 0x00FF00FF00FF00FF;
    x2 &= 0x00FF00FF00FF00FF;
    x3 &= 0x00FF00FF00FF00FF;
    *q0 = x0 | (x2 << 8);
    *q1 = x1 | (x3 << 8);
}

fn br_aes_ct64_interleave_out(w: &mut [u32], q0: u64, q1: u64) {
    let x0 = q0 & 0x00FF00FF00FF00FF;
    let x1 = q1 & 0x00FF00FF00FF00FF;
    let x2 = (q0 >> 8) & 0x00FF00FF00FF00FF;
    let x3 = (q1 >> 8) & 0x00FF00FF00FF00FF;
    let x0 = x0 | (x0 >> 8);
    let x1 = x1 | (x1 >> 8);
    let x2 = x2 | (x2 >> 8);
    let x3 = x3 | (x3 >> 8);
    let x0 = x0 & 0x0000FFFF0000FFFF;
    let x1 = x1 & 0x0000FFFF0000FFFF;
    let x2 = x2 & 0x0000FFFF0000FFFF;
    let x3 = x3 & 0x0000FFFF0000FFFF;
    w[0] = (x0 as u32) | ((x0 >> 16) as u32);
    w[1] = (x1 as u32) | ((x1 >> 16) as u32);
    w[2] = (x2 as u32) | ((x2 >> 16) as u32);
    w[3] = (x3 as u32) | ((x3 >> 16) as u32);
}

fn add_round_key(q: &mut [u64; 8], sk: &[u64; 8]) {
    for i in 0..8 {
        q[i] ^= sk[i];
    }
}

fn shift_rows(q: &mut [u64; 8]) {
    for i in 0..8 {
        let x = q[i];
        q[i] = (x & 0x000000000000FFFF)
            | ((x & 0x00000000FFF00000) >> 4)
            | ((x & 0x00000000000F0000) << 12)
            | ((x & 0x0000FF0000000000) >> 8)
            | ((x & 0x000000FF00000000) << 8)
            | ((x & 0xF000000000000000) >> 12)
            | ((x & 0x0FFF000000000000) << 4);
    }
}

fn rotr32_64(x: u64) -> u64 {
    (x << 32) | (x >> 32)
}

fn mix_columns(q: &mut [u64; 8]) {
    let q0 = q[0]; let q1 = q[1]; let q2 = q[2]; let q3 = q[3];
    let q4 = q[4]; let q5 = q[5]; let q6 = q[6]; let q7 = q[7];
    let r0 = (q0 >> 16) | (q0 << 48);
    let r1 = (q1 >> 16) | (q1 << 48);
    let r2 = (q2 >> 16) | (q2 << 48);
    let r3 = (q3 >> 16) | (q3 << 48);
    let r4 = (q4 >> 16) | (q4 << 48);
    let r5 = (q5 >> 16) | (q5 << 48);
    let r6 = (q6 >> 16) | (q6 << 48);
    let r7 = (q7 >> 16) | (q7 << 48);

    q[0] = q7 ^ r7 ^ r0 ^ rotr32_64(q0 ^ r0);
    q[1] = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ rotr32_64(q1 ^ r1);
    q[2] = q1 ^ r1 ^ r2 ^ rotr32_64(q2 ^ r2);
    q[3] = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ rotr32_64(q3 ^ r3);
    q[4] = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ rotr32_64(q4 ^ r4);
    q[5] = q4 ^ r4 ^ r5 ^ rotr32_64(q5 ^ r5);
    q[6] = q5 ^ r5 ^ r6 ^ rotr32_64(q6 ^ r6);
    q[7] = q6 ^ r6 ^ r7 ^ rotr32_64(q7 ^ r7);
}

fn interleave_constant(out: &mut [u64; 8], inp: &[u8]) {
    let mut tmp = [0u32; 16];
    br_range_dec32le(&mut tmp, 16, inp);
    for i in 0..4 {
        let mut q0: u64 = 0;
        let mut q1: u64 = 0;
        br_aes_ct64_interleave_in(&mut q0, &mut q1, &tmp[i * 4..(i + 1) * 4]);
        out[i] = q0;
        out[i + 4] = q1;
    }
    br_aes_ct64_ortho(out);
}

fn interleave_constant32(out: &mut [u32; 8], inp: &[u8]) {
    for i in 0..4 {
        out[2 * i] = br_dec32le(&inp[4 * i..]);
        out[2 * i + 1] = br_dec32le(&inp[4 * i + 16..]);
    }
    br_aes_ct_ortho(out);
}

pub fn tweak_constants(ctx: &mut SpxCtx) {
    let mut buf = vec![0u8; 40 * 16];
    // Use the standard constants to generate tweaked ones
    ctx.tweaked512_rc64 = HARAKA512_RC64;

    // Use seed via haraka_S
    let pub_seed = ctx.pub_seed;
    haraka_S(&mut buf, &pub_seed, ctx);
    for i in 0..10 {
        let mut a = [0u32; 8];
        interleave_constant32(&mut a, &buf[32 * i..]);
        ctx.tweaked256_rc32[i] = a;
        let mut b = [0u64; 8];
        interleave_constant(&mut b, &buf[64 * i..]);
        ctx.tweaked512_rc64[i] = b;
    }
}

pub fn haraka512_perm(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let mut w = [0u32; 16];
    let mut q = [0u64; 8];
    br_range_dec32le(&mut w, 16, inp);
    for i in 0..4 {
        let mut q0: u64 = 0;
        let mut q1: u64 = 0;
        br_aes_ct64_interleave_in(&mut q0, &mut q1, &w[i * 4..(i + 1) * 4]);
        q[i] = q0;
        q[i + 4] = q1;
    }
    br_aes_ct64_ortho(&mut q);

    for i in 0..5 {
        for j in 0..2 {
            br_aes_ct64_bitslice_sbox(&mut q);
            shift_rows(&mut q);
            mix_columns(&mut q);
            add_round_key(&mut q, &ctx.tweaked512_rc64[2 * i + j]);
        }
        for j in 0..8 {
            let tmp_q = q[j];
            q[j] = (tmp_q & 0x0001000100010001) << 5
                 | (tmp_q & 0x0002000200020002) << 12
                 | (tmp_q & 0x0004000400040004) >> 1
                 | (tmp_q & 0x0008000800080008) << 6
                 | (tmp_q & 0x0020002000200020) << 9
                 | (tmp_q & 0x0040004000400040) >> 4
                 | (tmp_q & 0x0080008000800080) << 3
                 | (tmp_q & 0x2100210021002100) >> 5
                 | (tmp_q & 0x0210021002100210) << 2
                 | (tmp_q & 0x0800080008000800) << 4
                 | (tmp_q & 0x1000100010001000) >> 12
                 | (tmp_q & 0x4000400040004000) >> 10
                 | (tmp_q & 0x8400840084008400) >> 3;
        }
    }

    br_aes_ct64_ortho(&mut q);
    for i in 0..4 {
        let q0 = q[i];
        let q1 = q[i + 4];
        let mut tmp = [0u32; 4];
        br_aes_ct64_interleave_out(&mut tmp, q0, q1);
        for k in 0..4 {
            w[i * 4 + k] = tmp[k];
        }
    }
    br_range_enc32le(out, &w, 16);
}

pub fn haraka512(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let mut buf = [0u8; 64];
    haraka512_perm(&mut buf, inp, ctx);
    for i in 0..64 {
        buf[i] ^= inp[i];
    }
    out[0..8].copy_from_slice(&buf[8..16]);
    out[8..16].copy_from_slice(&buf[24..32]);
    out[16..24].copy_from_slice(&buf[32..40]);
    out[24..32].copy_from_slice(&buf[48..56]);
}

pub fn haraka256(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let mut q = [0u32; 8];
    for i in 0..4 {
        q[2 * i] = br_dec32le(&inp[4 * i..]);
        q[2 * i + 1] = br_dec32le(&inp[4 * i + 16..]);
    }
    br_aes_ct_ortho(&mut q);

    for i in 0..5 {
        for j in 0..2 {
            br_aes_ct_bitslice_sbox(&mut q);
            shift_rows32(&mut q);
            mix_columns32(&mut q);
            add_round_key32(&mut q, &ctx.tweaked256_rc32[2 * i + j]);
        }
        for j in 0..8 {
            let tmp_q = q[j];
            q[j] = (tmp_q & 0x81818181)
                 | (tmp_q & 0x02020202) << 1
                 | (tmp_q & 0x04040404) << 2
                 | (tmp_q & 0x08080808) << 3
                 | (tmp_q & 0x10101010) >> 3
                 | (tmp_q & 0x20202020) >> 2
                 | (tmp_q & 0x40404040) >> 1;
        }
    }

    br_aes_ct_ortho(&mut q);
    for i in 0..4 {
        br_enc32le(&mut out[4 * i..], q[2 * i]);
        br_enc32le(&mut out[4 * i + 16..], q[2 * i + 1]);
    }
    for i in 0..32 {
        out[i] ^= inp[i];
    }
}

pub fn haraka_S_inc_init(s_inc: &mut [u8; 65]) {
    for v in s_inc.iter_mut() {
        *v = 0;
    }
}

pub fn haraka_S_inc_absorb(s_inc: &mut [u8; 65], m: &[u8], ctx: &SpxCtx) {
    let mut mlen = m.len();
    let mut idx = 0;
    while mlen + s_inc[64] as usize >= HARAKAS_RATE {
        let n = HARAKAS_RATE - s_inc[64] as usize;
        for i in 0..n {
            s_inc[s_inc[64] as usize + i] ^= m[idx + i];
        }
        mlen -= n;
        idx += n;
        s_inc[64] = 0;
        let s_copy: [u8; 64] = (s_inc[..64]).try_into().unwrap();
        let mut perm_out = [0u8; 64];
        haraka512_perm(&mut perm_out, &s_copy, ctx);
        s_inc[..64].copy_from_slice(&perm_out);
    }
    for i in 0..mlen {
        s_inc[s_inc[64] as usize + i] ^= m[idx + i];
    }
    s_inc[64] += mlen as u8;
}

pub fn haraka_S_inc_finalize(s_inc: &mut [u8; 65]) {
    let pos = s_inc[64] as usize;
    s_inc[pos] ^= 0x1F;
    s_inc[HARAKAS_RATE - 1] ^= 128;
    s_inc[64] = 0;
}

pub fn haraka_S_inc_squeeze(out: &mut [u8], s_inc: &mut [u8; 65], ctx: &SpxCtx) {
    let mut outlen = out.len();
    let mut written = 0;
    let mut i: usize = 0;
    while i < outlen && i < s_inc[64] as usize {
        out[written + i] = s_inc[HARAKAS_RATE - s_inc[64] as usize + i];
        i += 1;
    }
    written += i;
    outlen -= i;
    s_inc[64] -= i as u8;

    while outlen > 0 {
        let s_copy: [u8; 64] = (s_inc[..64]).try_into().unwrap();
        let mut perm_out = [0u8; 64];
        haraka512_perm(&mut perm_out, &s_copy, ctx);
        s_inc[..64].copy_from_slice(&perm_out);
        let mut k: usize = 0;
        while k < outlen && k < HARAKAS_RATE {
            out[written + k] = s_inc[k];
            k += 1;
        }
        written += k;
        outlen -= k;
        s_inc[64] = (HARAKAS_RATE - k) as u8;
    }
}

fn haraka_S_absorb(s: &mut [u8; 64], r: usize, m: &[u8], p: u8, ctx: &SpxCtx) {
    let mut mlen = m.len();
    let mut idx = 0;
    let mut t = vec![0u8; r];
    while mlen >= r {
        for i in 0..r {
            s[i] ^= m[idx + i];
        }
        let s_copy = *s;
        haraka512_perm(s, &s_copy, ctx);
        mlen -= r;
        idx += r;
    }
    for i in 0..r {
        t[i] = 0;
    }
    for i in 0..mlen {
        t[i] = m[idx + i];
    }
    t[mlen] = p;
    t[r - 1] |= 128;
    for i in 0..r {
        s[i] ^= t[i];
    }
}

fn haraka_S_squeezeblocks(h: &mut [u8], nblocks: usize, s: &mut [u8; 64], r: usize, ctx: &SpxCtx) {
    let mut written = 0;
    for _ in 0..nblocks {
        let s_copy = *s;
        haraka512_perm(s, &s_copy, ctx);
        h[written..written + HARAKAS_RATE].copy_from_slice(&s[..HARAKAS_RATE]);
        written += r;
    }
}

pub fn haraka_S(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let outlen = out.len();
    let mut s = [0u8; 64];
    haraka_S_absorb(&mut s, 32, inp, 0x1F, ctx);
    let nblocks = outlen / 32;
    haraka_S_squeezeblocks(&mut out[..nblocks * 32], nblocks, &mut s, 32, ctx);
    let written = nblocks * 32;
    if outlen % 32 != 0 {
        let mut d = [0u8; 32];
        haraka_S_squeezeblocks(&mut d, 1, &mut s, 32, ctx);
        let rem = outlen % 32;
        out[written..written + rem].copy_from_slice(&d[..rem]);
    }
}

// =========================
// Backend impl
// =========================

pub fn initialize_hash_function_impl(ctx: &mut SpxCtx) {
    tweak_constants(ctx);
}

pub fn prf_addr_impl(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; 32];
    let mut buf = [0u8; 64];
    let addr_bytes = addr_to_bytes(addr);
    buf[..SPX_ADDR_BYTES].copy_from_slice(&addr_bytes);
    buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);
    haraka512(&mut outbuf, &buf, ctx);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn gen_message_random_impl(
    R: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    ctx: &SpxCtx,
) {
    let mut s_inc = [0u8; 65];
    haraka_S_inc_init(&mut s_inc);
    haraka_S_inc_absorb(&mut s_inc, sk_prf, ctx);
    haraka_S_inc_absorb(&mut s_inc, optrand, ctx);
    haraka_S_inc_absorb(&mut s_inc, m, ctx);
    haraka_S_inc_finalize(&mut s_inc);
    let mut tmp = vec![0u8; SPX_N];
    haraka_S_inc_squeeze(&mut tmp, &mut s_inc, ctx);
    R.copy_from_slice(&tmp);
}

pub fn hash_message_impl(
    digest: &mut [u8],
    R: &[u8],
    pk: &[u8],
    m: &[u8],
    ctx: &SpxCtx,
) -> (u64, u32) {
    let spx_tree_bits = SPX_TREE_HEIGHT * (SPX_D - 1);
    let spx_tree_bytes = (spx_tree_bits + 7) / 8;
    let spx_leaf_bits = SPX_TREE_HEIGHT;
    let spx_leaf_bytes = (spx_leaf_bits + 7) / 8;
    let spx_dgst_bytes = SPX_FORS_MSG_BYTES + spx_tree_bytes + spx_leaf_bytes;

    let mut buf = vec![0u8; spx_dgst_bytes];
    let mut s_inc = [0u8; 65];
    haraka_S_inc_init(&mut s_inc);
    haraka_S_inc_absorb(&mut s_inc, R, ctx);
    haraka_S_inc_absorb(&mut s_inc, &pk[SPX_N..2 * SPX_N], ctx);
    haraka_S_inc_absorb(&mut s_inc, m, ctx);
    haraka_S_inc_finalize(&mut s_inc);
    haraka_S_inc_squeeze(&mut buf, &mut s_inc, ctx);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    let tree = if SPX_D == 1 {
        0u64
    } else {
        let mut t = bytes_to_ull_rs(&buf[bufp..bufp + spx_tree_bytes]);
        t &= !0u64 >> (64 - spx_tree_bits);
        t
    };
    bufp += spx_tree_bytes;
    let mut leaf_idx = bytes_to_ull_rs(&buf[bufp..bufp + spx_leaf_bytes]) as u32;
    leaf_idx &= !0u32 >> (32 - spx_leaf_bits);
    (tree, leaf_idx)
}

pub fn thash_impl(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    #[cfg(feature = "robust")]
    {
        thash_robust(out, inp, inblocks, ctx, addr);
    }
    #[cfg(feature = "simple")]
    {
        thash_simple(out, inp, inblocks, ctx, addr);
    }
}

#[cfg(feature = "robust")]
fn thash_robust(
    out: &mut [u8],
    inp: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let inblocks_us = inblocks as usize;
    let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks_us * SPX_N];
    let mut bitmask = vec![0u8; inblocks_us * SPX_N];
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];
    let addr_bytes = addr_to_bytes(addr);

    if inblocks == 1 {
        for v in buf_tmp.iter_mut() {
            *v = 0;
        }
        buf_tmp[..32].copy_from_slice(&addr_bytes);
        haraka256(&mut outbuf, &buf_tmp, ctx);
        for i in 0..inblocks_us * SPX_N {
            buf_tmp[SPX_ADDR_BYTES + i] = inp[i] ^ outbuf[i];
        }
        haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        buf[..32].copy_from_slice(&addr_bytes);
        haraka_S(&mut bitmask, &buf[..SPX_ADDR_BYTES], ctx);
        for i in 0..inblocks_us * SPX_N {
            buf[SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
        }
        let mut tmp_out = vec![0u8; SPX_N];
        haraka_S(&mut tmp_out, &buf, ctx);
        out[..SPX_N].copy_from_slice(&tmp_out);
    }
}

#[cfg(feature = "simple")]
fn thash_simple(
    out: &mut [u8],
    inp: &[u8],
    inblocks: u32,
    ctx: &SpxCtx,
    addr: &mut [u32; 8],
) {
    let inblocks_us = inblocks as usize;
    let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks_us * SPX_N];
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];
    let addr_bytes = addr_to_bytes(addr);

    if inblocks == 1 {
        for v in buf_tmp.iter_mut() {
            *v = 0;
        }
        buf_tmp[..32].copy_from_slice(&addr_bytes);
        buf_tmp[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&inp[..SPX_N]);
        haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        buf[..32].copy_from_slice(&addr_bytes);
        buf[SPX_ADDR_BYTES..].copy_from_slice(&inp[..inblocks_us * SPX_N]);
        let mut tmp_out = vec![0u8; SPX_N];
        haraka_S(&mut tmp_out, &buf, ctx);
        out[..SPX_N].copy_from_slice(&tmp_out);
    }
}

// FFI exports for haraka primitives used by the driver

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_tweak_constants(ctx: *mut SpxCtx) {
    tweak_constants(unsafe { &mut *ctx });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_init(s_inc: *mut u8) {
    let s = unsafe { &mut *(s_inc as *mut [u8; 65]) };
    haraka_S_inc_init(s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_absorb(
    s_inc: *mut u8,
    m: *const u8,
    mlen: usize,
    ctx: *const SpxCtx,
) {
    let s = unsafe { &mut *(s_inc as *mut [u8; 65]) };
    let m_slice = unsafe { core::slice::from_raw_parts(m, mlen) };
    let ctx_ref = unsafe { &*ctx };
    haraka_S_inc_absorb(s, m_slice, ctx_ref);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_finalize(s_inc: *mut u8) {
    let s = unsafe { &mut *(s_inc as *mut [u8; 65]) };
    haraka_S_inc_finalize(s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_squeeze(
    out: *mut u8,
    outlen: usize,
    s_inc: *mut u8,
    ctx: *const SpxCtx,
) {
    let s = unsafe { &mut *(s_inc as *mut [u8; 65]) };
    let out_slice = unsafe { core::slice::from_raw_parts_mut(out, outlen) };
    let ctx_ref = unsafe { &*ctx };
    haraka_S_inc_squeeze(out_slice, s, ctx_ref);
}
