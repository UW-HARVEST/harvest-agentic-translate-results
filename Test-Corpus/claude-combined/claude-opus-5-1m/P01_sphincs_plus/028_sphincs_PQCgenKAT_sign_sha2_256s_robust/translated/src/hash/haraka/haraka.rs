// Translation of c_src/lib/haraka/src/haraka.c
//
// Constant-time bit-sliced Haraka. Pure Rust translation of the C code.

#![allow(clippy::needless_range_loop)]
#![allow(non_snake_case)]

use core::slice;

use crate::context::SpxCtx;
use crate::params::SPX_N;

const HARAKAS_RATE: usize = 32;

const HARAKA512_RC64: [[u64; 8]; 10] = [
    [0x24cf0ab9086f628b, 0xbdd6eeecc83b8382, 0xd96fb0306cdad0a7, 0xaace082ac8f95f89,
     0x449d8e8870d7041f, 0x49bb2f80b2b3e2f8, 0x0569ae98d93bb258, 0x23dc9691e7d6a4b1],
    [0xd8ba10ede0fe5b6e, 0x7ecf7dbe424c7b8e, 0x6ea9949c6df62a31, 0xbf3f3c97ec9c313e,
     0x241d03a196a1861e, 0xead3a51116e5a2ea, 0x77d479fcad9574e3, 0x18657a1af894b7a0],
    [0x10671e1a7f595522, 0xd9a00ff675d28c7b, 0x2f1edf0d2b9ba661, 0xb8ff58b8e3de45f9,
     0xee29261da9865c02, 0xd1532aa4b50bdf43, 0x8bf858159b231bb1, 0xdf17439d22d4f599],
    [0xdd4b2f0870b918c0, 0x757a81f3b39b1bb6, 0x7a5c556898952e3f, 0x7dd70a16d915d87a,
     0x3ae61971982b8301, 0xc3ab319e030412be, 0x17c0033ac094a8cb, 0x5a0630fc1a8dc4ef],
    [0x17708988c1632f73, 0xf92ddae090b44f4f, 0x11ac0285c43aa314, 0x509059941936b8ba,
     0xd03e152fa2ce9b69, 0x3fbcbcb63a32998b, 0x6204696d692254f7, 0x915542ed93ec59b4],
    [0xf4ed94aa8879236e, 0xff6cb41cd38e03c0, 0x069b38602368aeab, 0x669495b820f0ddba,
     0xf42013b1b8bf9e3d, 0xcf935efe6439734d, 0xbc1dcf42ca29e3f8, 0x7e6d3ed29f78ad67],
    [0xf3b0f6837ffcddaa, 0x3a76faef934ddf41, 0xcec7ae583a9c8e35, 0xe4dd18c68f0260af,
     0x2c0e5df1ad398eaa, 0x478df5236ae22e8c, 0xfb944c46fe865f39, 0xaa48f82f028132ba],
    [0x231b9ae2b76aca77, 0x292a76a712db0b40, 0x5850625dc8134491, 0x73137dd469810fb5,
     0x8a12a6a202a474fd, 0xd36fd9daa78bdb80, 0xb34c5e733505706f, 0xbaf1cdca818d9d96],
    [0x2e99781335e8c641, 0xbddfe5cce47d560e, 0xf74e9bf32e5e040c, 0x1d7a709d65996be9,
     0x670df36a9cf66cdd, 0xd05ef84a176a2875, 0x0f888e828cb1c44e, 0x1a79e9c9727b052c],
    [0x83497348628d84de, 0x2e9387d51f22a754, 0xb000068da2f852d6, 0x378c9e1190fd6fe5,
     0x870027c316de7293, 0xe51a9d4462e047bb, 0x90ecf7f8c6251195, 0x655953bfbed90a9c],
];

#[inline]
fn br_dec32le(src: &[u8]) -> u32 {
    (src[0] as u32) | ((src[1] as u32) << 8) | ((src[2] as u32) << 16) | ((src[3] as u32) << 24)
}

fn br_range_dec32le(v: &mut [u32], num: usize, src: &[u8]) {
    for i in 0..num {
        v[i] = br_dec32le(&src[4 * i..]);
    }
}

#[inline]
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

#[inline]
fn swapn_32(cl: u32, ch: u32, s: u32, x: &mut u32, y: &mut u32) {
    let a = *x; let b = *y;
    *x = (a & cl) | ((b & cl) << s);
    *y = ((a & ch) >> s) | (b & ch);
}
fn br_aes_ct_ortho(q: &mut [u32; 8]) {
    let mut q0 = q[0]; let mut q1 = q[1];
    let mut q2 = q[2]; let mut q3 = q[3];
    let mut q4 = q[4]; let mut q5 = q[5];
    let mut q6 = q[6]; let mut q7 = q[7];

    swapn_32(0x55555555, 0xAAAAAAAA, 1, &mut q0, &mut q1);
    swapn_32(0x55555555, 0xAAAAAAAA, 1, &mut q2, &mut q3);
    swapn_32(0x55555555, 0xAAAAAAAA, 1, &mut q4, &mut q5);
    swapn_32(0x55555555, 0xAAAAAAAA, 1, &mut q6, &mut q7);

    swapn_32(0x33333333, 0xCCCCCCCC, 2, &mut q0, &mut q2);
    swapn_32(0x33333333, 0xCCCCCCCC, 2, &mut q1, &mut q3);
    swapn_32(0x33333333, 0xCCCCCCCC, 2, &mut q4, &mut q6);
    swapn_32(0x33333333, 0xCCCCCCCC, 2, &mut q5, &mut q7);

    swapn_32(0x0F0F0F0F, 0xF0F0F0F0, 4, &mut q0, &mut q4);
    swapn_32(0x0F0F0F0F, 0xF0F0F0F0, 4, &mut q1, &mut q5);
    swapn_32(0x0F0F0F0F, 0xF0F0F0F0, 4, &mut q2, &mut q6);
    swapn_32(0x0F0F0F0F, 0xF0F0F0F0, 4, &mut q3, &mut q7);

    q[0] = q0; q[1] = q1; q[2] = q2; q[3] = q3;
    q[4] = q4; q[5] = q5; q[6] = q6; q[7] = q7;
}

#[inline]
fn swapn_64(cl: u64, ch: u64, s: u32, x: &mut u64, y: &mut u64) {
    let a = *x; let b = *y;
    *x = (a & cl) | ((b & cl) << s);
    *y = ((a & ch) >> s) | (b & ch);
}
fn br_aes_ct64_ortho(q: &mut [u64; 8]) {
    let mut q0 = q[0]; let mut q1 = q[1];
    let mut q2 = q[2]; let mut q3 = q[3];
    let mut q4 = q[4]; let mut q5 = q[5];
    let mut q6 = q[6]; let mut q7 = q[7];

    swapn_64(0x5555555555555555, 0xAAAAAAAAAAAAAAAA, 1, &mut q0, &mut q1);
    swapn_64(0x5555555555555555, 0xAAAAAAAAAAAAAAAA, 1, &mut q2, &mut q3);
    swapn_64(0x5555555555555555, 0xAAAAAAAAAAAAAAAA, 1, &mut q4, &mut q5);
    swapn_64(0x5555555555555555, 0xAAAAAAAAAAAAAAAA, 1, &mut q6, &mut q7);

    swapn_64(0x3333333333333333, 0xCCCCCCCCCCCCCCCC, 2, &mut q0, &mut q2);
    swapn_64(0x3333333333333333, 0xCCCCCCCCCCCCCCCC, 2, &mut q1, &mut q3);
    swapn_64(0x3333333333333333, 0xCCCCCCCCCCCCCCCC, 2, &mut q4, &mut q6);
    swapn_64(0x3333333333333333, 0xCCCCCCCCCCCCCCCC, 2, &mut q5, &mut q7);

    swapn_64(0x0F0F0F0F0F0F0F0F, 0xF0F0F0F0F0F0F0F0, 4, &mut q0, &mut q4);
    swapn_64(0x0F0F0F0F0F0F0F0F, 0xF0F0F0F0F0F0F0F0, 4, &mut q1, &mut q5);
    swapn_64(0x0F0F0F0F0F0F0F0F, 0xF0F0F0F0F0F0F0F0, 4, &mut q2, &mut q6);
    swapn_64(0x0F0F0F0F0F0F0F0F, 0xF0F0F0F0F0F0F0F0, 4, &mut q3, &mut q7);

    q[0] = q0; q[1] = q1; q[2] = q2; q[3] = q3;
    q[4] = q4; q[5] = q5; q[6] = q6; q[7] = q7;
}

#[inline]
fn add_round_key32(q: &mut [u32; 8], sk: &[u32]) {
    for i in 0..8 {
        q[i] ^= sk[i];
    }
}

#[inline]
fn shift_rows32(q: &mut [u32; 8]) {
    for i in 0..8 {
        let x = q[i];
        q[i] = (x & 0x000000FF)
            | ((x & 0x0000FC00) >> 2) | ((x & 0x00000300) << 6)
            | ((x & 0x00F00000) >> 4) | ((x & 0x000F0000) << 4)
            | ((x & 0xC0000000) >> 6) | ((x & 0x3F000000) << 2);
    }
}

#[inline]
fn rotr16(x: u32) -> u32 {
    (x << 16) | (x >> 16)
}

#[inline]
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

    q[0] = q7 ^ r7 ^ r0 ^ rotr16(q0 ^ r0);
    q[1] = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ rotr16(q1 ^ r1);
    q[2] = q1 ^ r1 ^ r2 ^ rotr16(q2 ^ r2);
    q[3] = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ rotr16(q3 ^ r3);
    q[4] = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ rotr16(q4 ^ r4);
    q[5] = q4 ^ r4 ^ r5 ^ rotr16(q5 ^ r5);
    q[6] = q5 ^ r5 ^ r6 ^ rotr16(q6 ^ r6);
    q[7] = q6 ^ r6 ^ r7 ^ rotr16(q7 ^ r7);
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
    let mut x0 = q0 & 0x00FF00FF00FF00FF;
    let mut x1 = q1 & 0x00FF00FF00FF00FF;
    let mut x2 = (q0 >> 8) & 0x00FF00FF00FF00FF;
    let mut x3 = (q1 >> 8) & 0x00FF00FF00FF00FF;
    x0 |= x0 >> 8;
    x1 |= x1 >> 8;
    x2 |= x2 >> 8;
    x3 |= x3 >> 8;
    x0 &= 0x0000FFFF0000FFFF;
    x1 &= 0x0000FFFF0000FFFF;
    x2 &= 0x0000FFFF0000FFFF;
    x3 &= 0x0000FFFF0000FFFF;
    w[0] = (x0 as u32) | ((x0 >> 16) as u32);
    w[1] = (x1 as u32) | ((x1 >> 16) as u32);
    w[2] = (x2 as u32) | ((x2 >> 16) as u32);
    w[3] = (x3 as u32) | ((x3 >> 16) as u32);
}

#[inline]
fn add_round_key(q: &mut [u64; 8], sk: &[u64]) {
    for i in 0..8 {
        q[i] ^= sk[i];
    }
}

#[inline]
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

#[inline]
fn rotr32(x: u64) -> u64 {
    (x << 32) | (x >> 32)
}

#[inline]
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

    q[0] = q7 ^ r7 ^ r0 ^ rotr32(q0 ^ r0);
    q[1] = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ rotr32(q1 ^ r1);
    q[2] = q1 ^ r1 ^ r2 ^ rotr32(q2 ^ r2);
    q[3] = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ rotr32(q3 ^ r3);
    q[4] = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ rotr32(q4 ^ r4);
    q[5] = q4 ^ r4 ^ r5 ^ rotr32(q5 ^ r5);
    q[6] = q5 ^ r5 ^ r6 ^ rotr32(q6 ^ r6);
    q[7] = q6 ^ r6 ^ r7 ^ rotr32(q7 ^ r7);
}

fn interleave_constant(out: &mut [u64; 8], input: &[u8]) {
    let mut tmp = [0u32; 16];
    br_range_dec32le(&mut tmp, 16, input);
    for i in 0..4 {
        let (a, b) = out.split_at_mut(4);
        br_aes_ct64_interleave_in(&mut a[i], &mut b[i], &tmp[i * 4..]);
    }
    br_aes_ct64_ortho(out);
}

fn interleave_constant32(out: &mut [u32; 8], input: &[u8]) {
    for i in 0..4 {
        out[2 * i] = br_dec32le(&input[4 * i..]);
        out[2 * i + 1] = br_dec32le(&input[4 * i + 16..]);
    }
    br_aes_ct_ortho(out);
}

#[cfg(feature = "haraka")]
pub fn tweak_constants_inner(ctx: &mut SpxCtx) {
    let mut buf = vec![0u8; 40 * 16];
    // Initialize tweaked512_rc64 to standard constants
    for i in 0..10 {
        ctx.tweaked512_rc64[i] = HARAKA512_RC64[i];
    }
    // Generate buffer using haraka_S with current state
    haraka_s_inner(&mut buf, 40 * 16, &ctx.pub_seed, SPX_N as u64, ctx);
    for i in 0..10 {
        let mut tmp32 = [0u32; 8];
        interleave_constant32(&mut tmp32, &buf[32 * i..]);
        ctx.tweaked256_rc32[i] = tmp32;
        let mut tmp64 = [0u64; 8];
        interleave_constant(&mut tmp64, &buf[64 * i..]);
        ctx.tweaked512_rc64[i] = tmp64;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_tweak_constants(ctx: *mut SpxCtx) {
    let ctx = unsafe { &mut *ctx };
    tweak_constants_inner(ctx);
}

fn haraka_s_absorb(s: &mut [u8], r: usize, mut m: &[u8], mut mlen: u64, p: u8, ctx: &SpxCtx) {
    let mut t = vec![0u8; r];
    while mlen >= r as u64 {
        for i in 0..r {
            s[i] ^= m[i];
        }
        let mut tmp = vec![0u8; 64];
        tmp.copy_from_slice(&s[..64]);
        haraka512_perm_inner(&mut tmp, &s[..64].to_vec(), ctx);
        s[..64].copy_from_slice(&tmp);
        mlen -= r as u64;
        m = &m[r..];
    }
    for i in 0..r { t[i] = 0; }
    let mlen_us = mlen as usize;
    for i in 0..mlen_us { t[i] = m[i]; }
    t[mlen_us] = p;
    t[r - 1] |= 128;
    for i in 0..r {
        s[i] ^= t[i];
    }
}

fn haraka_s_squeezeblocks(h: &mut [u8], mut nblocks: u64, s: &mut [u8], r: usize, ctx: &SpxCtx) {
    let mut h_off = 0usize;
    while nblocks > 0 {
        let mut tmp = vec![0u8; 64];
        tmp.copy_from_slice(&s[..64]);
        haraka512_perm_inner(&mut tmp, &s[..64].to_vec(), ctx);
        s[..64].copy_from_slice(&tmp);
        h[h_off..h_off + HARAKAS_RATE].copy_from_slice(&s[..HARAKAS_RATE]);
        h_off += r;
        nblocks -= 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_init(s_inc: *mut u8) {
    let s_inc = unsafe { slice::from_raw_parts_mut(s_inc, 65) };
    haraka_s_inc_init_inner(s_inc);
}

pub fn haraka_s_inc_init_inner(s_inc: &mut [u8]) {
    for i in 0..64 { s_inc[i] = 0; }
    s_inc[64] = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_absorb(
    s_inc: *mut u8,
    m: *const u8,
    mlen: usize,
    ctx: *const SpxCtx,
) {
    let s_inc = unsafe { slice::from_raw_parts_mut(s_inc, 65) };
    let m_slice = unsafe { slice::from_raw_parts(m, mlen) };
    let ctx = unsafe { &*ctx };
    haraka_s_inc_absorb_inner(s_inc, m_slice, ctx);
}

pub fn haraka_s_inc_absorb_inner(s_inc: &mut [u8], mut m: &[u8], ctx: &SpxCtx) {
    let mut mlen = m.len();
    while mlen + (s_inc[64] as usize) >= HARAKAS_RATE {
        let cap = HARAKAS_RATE - s_inc[64] as usize;
        for i in 0..cap {
            s_inc[(s_inc[64] as usize) + i] ^= m[i];
        }
        mlen -= cap;
        m = &m[cap..];
        s_inc[64] = 0;

        let mut tmp = [0u8; 64];
        tmp.copy_from_slice(&s_inc[..64]);
        let snap = tmp;
        haraka512_perm_inner(&mut tmp, &snap, ctx);
        s_inc[..64].copy_from_slice(&tmp);
    }

    for i in 0..mlen {
        s_inc[(s_inc[64] as usize) + i] ^= m[i];
    }
    s_inc[64] = s_inc[64].wrapping_add(mlen as u8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_finalize(s_inc: *mut u8) {
    let s_inc = unsafe { slice::from_raw_parts_mut(s_inc, 65) };
    haraka_s_inc_finalize_inner(s_inc);
}

pub fn haraka_s_inc_finalize_inner(s_inc: &mut [u8]) {
    let pos = s_inc[64] as usize;
    s_inc[pos] ^= 0x1F;
    s_inc[HARAKAS_RATE - 1] ^= 128;
    s_inc[64] = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S_inc_squeeze(
    out: *mut u8,
    outlen: usize,
    s_inc: *mut u8,
    ctx: *const SpxCtx,
) {
    let out_slice = unsafe { slice::from_raw_parts_mut(out, outlen) };
    let s_inc = unsafe { slice::from_raw_parts_mut(s_inc, 65) };
    let ctx = unsafe { &*ctx };
    haraka_s_inc_squeeze_inner(out_slice, s_inc, ctx);
}

pub fn haraka_s_inc_squeeze_inner(out: &mut [u8], s_inc: &mut [u8], ctx: &SpxCtx) {
    let mut outlen = out.len();
    let mut out_off = 0usize;
    let mut i: usize = 0;
    while i < outlen && i < s_inc[64] as usize {
        out[out_off + i] = s_inc[HARAKAS_RATE - s_inc[64] as usize + i];
        i += 1;
    }
    out_off += i;
    outlen -= i;
    s_inc[64] = s_inc[64].wrapping_sub(i as u8);

    while outlen > 0 {
        let mut tmp = [0u8; 64];
        tmp.copy_from_slice(&s_inc[..64]);
        let snap = tmp;
        haraka512_perm_inner(&mut tmp, &snap, ctx);
        s_inc[..64].copy_from_slice(&tmp);

        let mut j: usize = 0;
        while j < outlen && j < HARAKAS_RATE {
            out[out_off + j] = s_inc[j];
            j += 1;
        }
        out_off += j;
        outlen -= j;
        s_inc[64] = (HARAKAS_RATE - j) as u8;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka_S(
    out: *mut u8,
    outlen: u64,
    input: *const u8,
    inlen: u64,
    ctx: *const SpxCtx,
) {
    let out = unsafe { slice::from_raw_parts_mut(out, outlen as usize) };
    let input = unsafe { slice::from_raw_parts(input, inlen as usize) };
    let ctx = unsafe { &*ctx };
    haraka_s_inner(out, outlen, input, inlen, ctx);
}

pub fn haraka_s_inner(out: &mut [u8], outlen: u64, input: &[u8], inlen: u64, ctx: &SpxCtx) {
    let mut s = [0u8; 64];
    let mut d = [0u8; 32];

    haraka_s_absorb(&mut s, 32, input, inlen, 0x1F, ctx);

    let blocks = outlen / 32;
    haraka_s_squeezeblocks(out, blocks, &mut s, 32, ctx);
    let mut out_off = (blocks * 32) as usize;

    if outlen % 32 != 0 {
        let mut s2 = s;
        haraka_s_squeezeblocks(&mut d, 1, &mut s2, 32, ctx);
        let rem = (outlen % 32) as usize;
        for i in 0..rem {
            out[out_off + i] = d[i];
        }
        out_off += rem;
    }
    let _ = out_off;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka512_perm(
    out: *mut u8,
    input: *const u8,
    ctx: *const SpxCtx,
) {
    let out = unsafe { slice::from_raw_parts_mut(out, 64) };
    let input = unsafe { slice::from_raw_parts(input, 64) };
    let ctx = unsafe { &*ctx };
    haraka512_perm_inner(out, input, ctx);
}

pub fn haraka512_perm_inner(out: &mut [u8], input: &[u8], ctx: &SpxCtx) {
    let mut w = [0u32; 16];
    let mut q = [0u64; 8];

    br_range_dec32le(&mut w, 16, input);
    for i in 0..4 {
        let (a, b) = q.split_at_mut(4);
        br_aes_ct64_interleave_in(&mut a[i], &mut b[i], &w[i * 4..]);
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
        let qi = q[i];
        let qi4 = q[i + 4];
        br_aes_ct64_interleave_out(&mut w[i * 4..], qi, qi4);
    }
    br_range_enc32le(out, &w, 16);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka512(out: *mut u8, input: *const u8, ctx: *const SpxCtx) {
    let out = unsafe { slice::from_raw_parts_mut(out, 32) };
    let input = unsafe { slice::from_raw_parts(input, 64) };
    let ctx = unsafe { &*ctx };
    haraka512_inner(out, input, ctx);
}

pub fn haraka512_inner(out: &mut [u8], input: &[u8], ctx: &SpxCtx) {
    let mut buf = [0u8; 64];
    let in_arr: [u8; 64] = input[..64].try_into().unwrap();
    haraka512_perm_inner(&mut buf, &in_arr, ctx);
    for i in 0..64 {
        buf[i] ^= in_arr[i];
    }
    out[..8].copy_from_slice(&buf[8..16]);
    out[8..16].copy_from_slice(&buf[24..32]);
    out[16..24].copy_from_slice(&buf[32..40]);
    out[24..32].copy_from_slice(&buf[48..56]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_haraka256(out: *mut u8, input: *const u8, ctx: *const SpxCtx) {
    let out = unsafe { slice::from_raw_parts_mut(out, 32) };
    let input = unsafe { slice::from_raw_parts(input, 32) };
    let ctx = unsafe { &*ctx };
    haraka256_inner(out, input, ctx);
}

pub fn haraka256_inner(out: &mut [u8], input: &[u8], ctx: &SpxCtx) {
    let mut q = [0u32; 8];
    for i in 0..4 {
        q[2 * i] = br_dec32le(&input[4 * i..]);
        q[2 * i + 1] = br_dec32le(&input[4 * i + 16..]);
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
        out[i] ^= input[i];
    }
}
