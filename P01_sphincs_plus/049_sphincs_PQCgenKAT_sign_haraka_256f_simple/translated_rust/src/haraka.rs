use crate::context::SpxCtx;
use crate::params::SPX_N;

const HARAKAS_RATE: usize = 32;

pub static HARAKA512_RC64: [[u64; 8]; 10] = [
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

fn br_dec32le(src: &[u8]) -> u32 {
    src[0] as u32 | (src[1] as u32) << 8 | (src[2] as u32) << 16 | (src[3] as u32) << 24
}

fn br_range_dec32le(v: &mut [u32], src: &[u8]) {
    for i in 0..v.len() {
        v[i] = br_dec32le(&src[4 * i..]);
    }
}

fn br_enc32le(dst: &mut [u8], x: u32) {
    dst[0] = x as u8;
    dst[1] = (x >> 8) as u8;
    dst[2] = (x >> 16) as u8;
    dst[3] = (x >> 24) as u8;
}

fn br_range_enc32le(dst: &mut [u8], v: &[u32]) {
    for i in 0..v.len() {
        br_enc32le(&mut dst[4 * i..], v[i]);
    }
}

fn br_aes_ct64_bitslice_sbox(q: &mut [u64; 8]) {
    let x0 = q[7]; let x1 = q[6]; let x2 = q[5]; let x3 = q[4];
    let x4 = q[3]; let x5 = q[2]; let x6 = q[1]; let x7 = q[0];
    let y14 = x3 ^ x5; let y13 = x0 ^ x6; let y9 = x0 ^ x3; let y8 = x0 ^ x5;
    let t0 = x1 ^ x2; let y1 = t0 ^ x7; let y4 = y1 ^ x3; let y12 = y13 ^ y14;
    let y2 = y1 ^ x0; let y5 = y1 ^ x6; let y3 = y5 ^ y8;
    let t1 = x4 ^ y12; let y15 = t1 ^ x5; let y20 = t1 ^ x1;
    let y6 = y15 ^ x7; let y10 = y15 ^ t0; let y11 = y20 ^ y9;
    let y7 = x7 ^ y11; let y17 = y10 ^ y11; let y19 = y10 ^ y8;
    let y16 = t0 ^ y11; let y21 = y13 ^ y16; let y18 = x0 ^ y16;
    let t2 = y12 & y15; let t3 = y3 & y6; let t4 = t3 ^ t2;
    let t5 = y4 & x7; let t6 = t5 ^ t2; let t7 = y13 & y16;
    let t8 = y5 & y1; let t9 = t8 ^ t7; let t10 = y2 & y7;
    let t11 = t10 ^ t7; let t12 = y9 & y11; let t13 = y14 & y17;
    let t14 = t13 ^ t12; let t15 = y8 & y10; let t16 = t15 ^ t12;
    let t17 = t4 ^ t14; let t18 = t6 ^ t16; let t19 = t9 ^ t14; let t20 = t11 ^ t16;
    let t21 = t17 ^ y20; let t22 = t18 ^ y19; let t23 = t19 ^ y21; let t24 = t20 ^ y18;
    let t25 = t21 ^ t22; let t26 = t21 & t23; let t27 = t24 ^ t26;
    let t28 = t25 & t27; let t29 = t28 ^ t22; let t30 = t23 ^ t24;
    let t31 = t22 ^ t26; let t32 = t31 & t30; let t33 = t32 ^ t24;
    let t34 = t23 ^ t33; let t35 = t27 ^ t33; let t36 = t24 & t35;
    let t37 = t36 ^ t34; let t38 = t27 ^ t36; let t39 = t29 & t38;
    let t40 = t25 ^ t39; let t41 = t40 ^ t37; let t42 = t29 ^ t33;
    let t43 = t29 ^ t40; let t44 = t33 ^ t37; let t45 = t42 ^ t41;
    let z0 = t44 & y15; let z1 = t37 & y6; let z2 = t33 & x7;
    let z3 = t43 & y16; let z4 = t40 & y1; let z5 = t29 & y7;
    let z6 = t42 & y11; let z7 = t45 & y17; let z8 = t41 & y10;
    let z9 = t44 & y12; let z10 = t37 & y3; let z11 = t33 & y4;
    let z12 = t43 & y13; let z13 = t40 & y5; let z14 = t29 & y2;
    let z15 = t42 & y9; let z16 = t45 & y14; let z17 = t41 & y8;
    let t46 = z15 ^ z16; let t47 = z10 ^ z11; let t48 = z5 ^ z13;
    let t49 = z9 ^ z10; let t50 = z2 ^ z12; let t51 = z2 ^ z5;
    let t52 = z7 ^ z8; let t53 = z0 ^ z3; let t54 = z6 ^ z7;
    let t55 = z16 ^ z17; let t56 = z12 ^ t48; let t57 = t50 ^ t53;
    let t58 = z4 ^ t46; let t59 = z3 ^ t54; let t60 = t46 ^ t57;
    let t61 = z14 ^ t57; let t62 = t52 ^ t58; let t63 = t49 ^ t58;
    let t64 = z4 ^ t59; let t65 = t61 ^ t62; let t66 = z1 ^ t63;
    let s0 = t59 ^ t63; let s6 = t56 ^ !t62; let s7 = t48 ^ !t60;
    let t67 = t64 ^ t65; let s3 = t53 ^ t66; let s4 = t51 ^ t66;
    let s5 = t47 ^ t65; let s1 = t64 ^ !s3; let s2 = t55 ^ !t67;
    q[7] = s0; q[6] = s1; q[5] = s2; q[4] = s3;
    q[3] = s4; q[2] = s5; q[1] = s6; q[0] = s7;
}

fn br_aes_ct_bitslice_sbox(q: &mut [u32; 8]) {
    let x0 = q[7]; let x1 = q[6]; let x2 = q[5]; let x3 = q[4];
    let x4 = q[3]; let x5 = q[2]; let x6 = q[1]; let x7 = q[0];
    let y14 = x3 ^ x5; let y13 = x0 ^ x6; let y9 = x0 ^ x3; let y8 = x0 ^ x5;
    let t0 = x1 ^ x2; let y1 = t0 ^ x7; let y4 = y1 ^ x3; let y12 = y13 ^ y14;
    let y2 = y1 ^ x0; let y5 = y1 ^ x6; let y3 = y5 ^ y8;
    let t1 = x4 ^ y12; let y15 = t1 ^ x5; let y20 = t1 ^ x1;
    let y6 = y15 ^ x7; let y10 = y15 ^ t0; let y11 = y20 ^ y9;
    let y7 = x7 ^ y11; let y17 = y10 ^ y11; let y19 = y10 ^ y8;
    let y16 = t0 ^ y11; let y21 = y13 ^ y16; let y18 = x0 ^ y16;
    let t2 = y12 & y15; let t3 = y3 & y6; let t4 = t3 ^ t2;
    let t5 = y4 & x7; let t6 = t5 ^ t2; let t7 = y13 & y16;
    let t8 = y5 & y1; let t9 = t8 ^ t7; let t10 = y2 & y7;
    let t11 = t10 ^ t7; let t12 = y9 & y11; let t13 = y14 & y17;
    let t14 = t13 ^ t12; let t15 = y8 & y10; let t16 = t15 ^ t12;
    let t17 = t4 ^ t14; let t18 = t6 ^ t16; let t19 = t9 ^ t14; let t20 = t11 ^ t16;
    let t21 = t17 ^ y20; let t22 = t18 ^ y19; let t23 = t19 ^ y21; let t24 = t20 ^ y18;
    let t25 = t21 ^ t22; let t26 = t21 & t23; let t27 = t24 ^ t26;
    let t28 = t25 & t27; let t29 = t28 ^ t22; let t30 = t23 ^ t24;
    let t31 = t22 ^ t26; let t32 = t31 & t30; let t33 = t32 ^ t24;
    let t34 = t23 ^ t33; let t35 = t27 ^ t33; let t36 = t24 & t35;
    let t37 = t36 ^ t34; let t38 = t27 ^ t36; let t39 = t29 & t38;
    let t40 = t25 ^ t39; let t41 = t40 ^ t37; let t42 = t29 ^ t33;
    let t43 = t29 ^ t40; let t44 = t33 ^ t37; let t45 = t42 ^ t41;
    let z0 = t44 & y15; let z1 = t37 & y6; let z2 = t33 & x7;
    let z3 = t43 & y16; let z4 = t40 & y1; let z5 = t29 & y7;
    let z6 = t42 & y11; let z7 = t45 & y17; let z8 = t41 & y10;
    let z9 = t44 & y12; let z10 = t37 & y3; let z11 = t33 & y4;
    let z12 = t43 & y13; let z13 = t40 & y5; let z14 = t29 & y2;
    let z15 = t42 & y9; let z16 = t45 & y14; let z17 = t41 & y8;
    let t46 = z15 ^ z16; let t47 = z10 ^ z11; let t48 = z5 ^ z13;
    let t49 = z9 ^ z10; let t50 = z2 ^ z12; let t51 = z2 ^ z5;
    let t52 = z7 ^ z8; let t53 = z0 ^ z3; let t54 = z6 ^ z7;
    let t55 = z16 ^ z17; let t56 = z12 ^ t48; let t57 = t50 ^ t53;
    let t58 = z4 ^ t46; let t59 = z3 ^ t54; let t60 = t46 ^ t57;
    let t61 = z14 ^ t57; let t62 = t52 ^ t58; let t63 = t49 ^ t58;
    let t64 = z4 ^ t59; let t65 = t61 ^ t62; let t66 = z1 ^ t63;
    let s0 = t59 ^ t63; let s6 = t56 ^ !t62; let s7 = t48 ^ !t60;
    let t67 = t64 ^ t65; let s3 = t53 ^ t66; let s4 = t51 ^ t66;
    let s5 = t47 ^ t65; let s1 = t64 ^ !s3; let s2 = t55 ^ !t67;
    q[7] = s0; q[6] = s1; q[5] = s2; q[4] = s3;
    q[3] = s4; q[2] = s5; q[1] = s6; q[0] = s7;
}

macro_rules! swapn_32 {
    ($cl:expr, $ch:expr, $s:expr, $x:expr, $y:expr) => {{
        let a = $x; let b = $y;
        $x = (a & ($cl as u32)) | ((b & ($cl as u32)) << $s);
        $y = ((a & ($ch as u32)) >> $s) | (b & ($ch as u32));
    }};
}

fn br_aes_ct_ortho(q: &mut [u32; 8]) {
    swapn_32!(0x55555555u32, 0xAAAAAAAAu32, 1, q[0], q[1]);
    swapn_32!(0x55555555u32, 0xAAAAAAAAu32, 1, q[2], q[3]);
    swapn_32!(0x55555555u32, 0xAAAAAAAAu32, 1, q[4], q[5]);
    swapn_32!(0x55555555u32, 0xAAAAAAAAu32, 1, q[6], q[7]);
    swapn_32!(0x33333333u32, 0xCCCCCCCCu32, 2, q[0], q[2]);
    swapn_32!(0x33333333u32, 0xCCCCCCCCu32, 2, q[1], q[3]);
    swapn_32!(0x33333333u32, 0xCCCCCCCCu32, 2, q[4], q[6]);
    swapn_32!(0x33333333u32, 0xCCCCCCCCu32, 2, q[5], q[7]);
    swapn_32!(0x0F0F0F0Fu32, 0xF0F0F0F0u32, 4, q[0], q[4]);
    swapn_32!(0x0F0F0F0Fu32, 0xF0F0F0F0u32, 4, q[1], q[5]);
    swapn_32!(0x0F0F0F0Fu32, 0xF0F0F0F0u32, 4, q[2], q[6]);
    swapn_32!(0x0F0F0F0Fu32, 0xF0F0F0F0u32, 4, q[3], q[7]);
}

fn add_round_key32(q: &mut [u32; 8], sk: &[u32; 8]) {
    for i in 0..8 { q[i] ^= sk[i]; }
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

fn rotr16(x: u32) -> u32 { (x << 16) | (x >> 16) }

fn mix_columns32(q: &mut [u32; 8]) {
    let (q0,q1,q2,q3,q4,q5,q6,q7) = (q[0],q[1],q[2],q[3],q[4],q[5],q[6],q[7]);
    let r0 = q0.rotate_right(8); let r1 = q1.rotate_right(8);
    let r2 = q2.rotate_right(8); let r3 = q3.rotate_right(8);
    let r4 = q4.rotate_right(8); let r5 = q5.rotate_right(8);
    let r6 = q6.rotate_right(8); let r7 = q7.rotate_right(8);
    q[0] = q7 ^ r7 ^ r0 ^ rotr16(q0 ^ r0);
    q[1] = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ rotr16(q1 ^ r1);
    q[2] = q1 ^ r1 ^ r2 ^ rotr16(q2 ^ r2);
    q[3] = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ rotr16(q3 ^ r3);
    q[4] = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ rotr16(q4 ^ r4);
    q[5] = q4 ^ r4 ^ r5 ^ rotr16(q5 ^ r5);
    q[6] = q5 ^ r5 ^ r6 ^ rotr16(q6 ^ r6);
    q[7] = q6 ^ r6 ^ r7 ^ rotr16(q7 ^ r7);
}

macro_rules! swapn_64 {
    ($cl:expr, $ch:expr, $s:expr, $x:expr, $y:expr) => {{
        let a = $x; let b = $y;
        $x = (a & ($cl as u64)) | ((b & ($cl as u64)) << $s);
        $y = ((a & ($ch as u64)) >> $s) | (b & ($ch as u64));
    }};
}

fn br_aes_ct64_ortho(q: &mut [u64; 8]) {
    swapn_64!(0x5555555555555555u64, 0xAAAAAAAAAAAAAAAAu64, 1, q[0], q[1]);
    swapn_64!(0x5555555555555555u64, 0xAAAAAAAAAAAAAAAAu64, 1, q[2], q[3]);
    swapn_64!(0x5555555555555555u64, 0xAAAAAAAAAAAAAAAAu64, 1, q[4], q[5]);
    swapn_64!(0x5555555555555555u64, 0xAAAAAAAAAAAAAAAAu64, 1, q[6], q[7]);
    swapn_64!(0x3333333333333333u64, 0xCCCCCCCCCCCCCCCCu64, 2, q[0], q[2]);
    swapn_64!(0x3333333333333333u64, 0xCCCCCCCCCCCCCCCCu64, 2, q[1], q[3]);
    swapn_64!(0x3333333333333333u64, 0xCCCCCCCCCCCCCCCCu64, 2, q[4], q[6]);
    swapn_64!(0x3333333333333333u64, 0xCCCCCCCCCCCCCCCCu64, 2, q[5], q[7]);
    swapn_64!(0x0F0F0F0F0F0F0F0Fu64, 0xF0F0F0F0F0F0F0F0u64, 4, q[0], q[4]);
    swapn_64!(0x0F0F0F0F0F0F0F0Fu64, 0xF0F0F0F0F0F0F0F0u64, 4, q[1], q[5]);
    swapn_64!(0x0F0F0F0F0F0F0F0Fu64, 0xF0F0F0F0F0F0F0F0u64, 4, q[2], q[6]);
    swapn_64!(0x0F0F0F0F0F0F0F0Fu64, 0xF0F0F0F0F0F0F0F0u64, 4, q[3], q[7]);
}

fn br_aes_ct64_interleave_in(q0: &mut u64, q1: &mut u64, w: &[u32]) {
    let mut x0 = w[0] as u64; let mut x1 = w[1] as u64;
    let mut x2 = w[2] as u64; let mut x3 = w[3] as u64;
    x0 |= x0 << 16; x1 |= x1 << 16; x2 |= x2 << 16; x3 |= x3 << 16;
    x0 &= 0x0000FFFF0000FFFF; x1 &= 0x0000FFFF0000FFFF;
    x2 &= 0x0000FFFF0000FFFF; x3 &= 0x0000FFFF0000FFFF;
    x0 |= x0 << 8; x1 |= x1 << 8; x2 |= x2 << 8; x3 |= x3 << 8;
    x0 &= 0x00FF00FF00FF00FF; x1 &= 0x00FF00FF00FF00FF;
    x2 &= 0x00FF00FF00FF00FF; x3 &= 0x00FF00FF00FF00FF;
    *q0 = x0 | (x2 << 8); *q1 = x1 | (x3 << 8);
}

fn br_aes_ct64_interleave_out(w: &mut [u32], q0: u64, q1: u64) {
    let mut x0 = q0 & 0x00FF00FF00FF00FF; let mut x1 = q1 & 0x00FF00FF00FF00FF;
    let mut x2 = (q0 >> 8) & 0x00FF00FF00FF00FF; let mut x3 = (q1 >> 8) & 0x00FF00FF00FF00FF;
    x0 |= x0 >> 8; x1 |= x1 >> 8; x2 |= x2 >> 8; x3 |= x3 >> 8;
    x0 &= 0x0000FFFF0000FFFF; x1 &= 0x0000FFFF0000FFFF;
    x2 &= 0x0000FFFF0000FFFF; x3 &= 0x0000FFFF0000FFFF;
    w[0] = (x0 as u32) | ((x0 >> 16) as u32);
    w[1] = (x1 as u32) | ((x1 >> 16) as u32);
    w[2] = (x2 as u32) | ((x2 >> 16) as u32);
    w[3] = (x3 as u32) | ((x3 >> 16) as u32);
}

fn add_round_key(q: &mut [u64; 8], sk: &[u64; 8]) {
    for i in 0..8 { q[i] ^= sk[i]; }
}

fn shift_rows(q: &mut [u64; 8]) {
    for i in 0..8 {
        let x = q[i];
        q[i] = (x & 0x000000000000FFFF)
            | ((x & 0x00000000FFF00000) >> 4) | ((x & 0x00000000000F0000) << 12)
            | ((x & 0x0000FF0000000000) >> 8) | ((x & 0x000000FF00000000) << 8)
            | ((x & 0xF000000000000000) >> 12) | ((x & 0x0FFF000000000000) << 4);
    }
}

fn rotr32_64(x: u64) -> u64 { (x << 32) | (x >> 32) }

fn mix_columns(q: &mut [u64; 8]) {
    let (q0,q1,q2,q3,q4,q5,q6,q7) = (q[0],q[1],q[2],q[3],q[4],q[5],q[6],q[7]);
    let r0 = q0.rotate_right(16); let r1 = q1.rotate_right(16);
    let r2 = q2.rotate_right(16); let r3 = q3.rotate_right(16);
    let r4 = q4.rotate_right(16); let r5 = q5.rotate_right(16);
    let r6 = q6.rotate_right(16); let r7 = q7.rotate_right(16);
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
    br_range_dec32le(&mut tmp, inp);
    for i in 0..4 {
        let mut q0 = 0u64;
        let mut q1 = 0u64;
        br_aes_ct64_interleave_in(&mut q0, &mut q1, &tmp[i * 4..]);
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
    // Copy standard constants
    ctx.tweaked512_rc64 = HARAKA512_RC64;

    // Generate tweaked constants from pub_seed
    let mut buf = [0u8; 40 * 16];
    haraka_s(&mut buf, &ctx.pub_seed[..SPX_N], ctx);
    for i in 0..10 {
        interleave_constant32(&mut ctx.tweaked256_rc32[i], &buf[32 * i..]);
        interleave_constant(&mut ctx.tweaked512_rc64[i], &buf[64 * i..]);
    }
}

pub fn haraka512_perm(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let mut w = [0u32; 16];
    let mut q = [0u64; 8];

    br_range_dec32le(&mut w, &inp[..64]);
    for i in 0..4 {
        let mut q0 = 0u64;
        let mut q1 = 0u64;
        br_aes_ct64_interleave_in(&mut q0, &mut q1, &w[i * 4..]);
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
        br_aes_ct64_interleave_out(&mut w[i * 4..], q[i], q[i + 4]);
    }
    br_range_enc32le(&mut out[..64], &w);
}

pub fn haraka512(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let mut buf = [0u8; 64];
    haraka512_perm(&mut buf, inp, ctx);
    for i in 0..64 { buf[i] ^= inp[i]; }
    out[..8].copy_from_slice(&buf[8..16]);
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
    for i in 0..32 { out[i] ^= inp[i]; }
}

fn haraka_s_absorb(s: &mut [u8; 64], r: usize, m: &[u8], p: u8, ctx: &SpxCtx) {
    let mut mlen = m.len();
    let mut moff = 0;
    while mlen >= r {
        for i in 0..r { s[i] ^= m[moff + i]; }
        let tmp: [u8; 64] = *s;
        haraka512_perm(s, &tmp, ctx);
        mlen -= r;
        moff += r;
    }
    let mut t = vec![0u8; r];
    for i in 0..mlen { t[i] = m[moff + i]; }
    t[mlen] = p;
    t[r - 1] |= 128;
    for i in 0..r { s[i] ^= t[i]; }
}

fn haraka_s_squeezeblocks(h: &mut [u8], nblocks: usize, s: &mut [u8; 64], r: usize, ctx: &SpxCtx) {
    for i in 0..nblocks {
        let tmp: [u8; 64] = *s;
        haraka512_perm(s, &tmp, ctx);
        h[i * r..i * r + HARAKAS_RATE].copy_from_slice(&s[..HARAKAS_RATE]);
    }
}

pub fn haraka_s(out: &mut [u8], inp: &[u8], ctx: &SpxCtx) {
    let outlen = out.len();
    let mut s = [0u8; 64];
    haraka_s_absorb(&mut s, 32, inp, 0x1F, ctx);

    let full_blocks = outlen / 32;
    if full_blocks > 0 {
        haraka_s_squeezeblocks(&mut out[..full_blocks * 32], full_blocks, &mut s, 32, ctx);
    }

    if outlen % 32 != 0 {
        let mut d = [0u8; 32];
        haraka_s_squeezeblocks(&mut d, 1, &mut s, 32, ctx);
        let off = full_blocks * 32;
        out[off..].copy_from_slice(&d[..outlen % 32]);
    }
}

pub fn haraka_s_inc_init(s_inc: &mut [u8; 65]) {
    for i in 0..65 { s_inc[i] = 0; }
}

pub fn haraka_s_inc_absorb(s_inc: &mut [u8; 65], m: &[u8], ctx: &SpxCtx) {
    let mut mlen = m.len();
    let mut moff = 0;
    while mlen + s_inc[64] as usize >= HARAKAS_RATE {
        let avail = HARAKAS_RATE - s_inc[64] as usize;
        for i in 0..avail {
            s_inc[s_inc[64] as usize + i] ^= m[moff + i];
        }
        mlen -= avail;
        moff += avail;
        s_inc[64] = 0;
        let mut tmp = [0u8; 64];
        tmp.copy_from_slice(&s_inc[..64]);
        haraka512_perm(&mut s_inc[..64], &tmp, ctx);
    }
    for i in 0..mlen {
        s_inc[s_inc[64] as usize + i] ^= m[moff + i];
    }
    s_inc[64] += mlen as u8;
}

pub fn haraka_s_inc_finalize(s_inc: &mut [u8; 65]) {
    s_inc[s_inc[64] as usize] ^= 0x1F;
    s_inc[HARAKAS_RATE - 1] ^= 128;
    s_inc[64] = 0;
}

pub fn haraka_s_inc_squeeze(out: &mut [u8], s_inc: &mut [u8; 65], ctx: &SpxCtx) {
    let mut outlen = out.len();
    let mut outoff = 0;
    // First consume remaining bytes
    let mut i = 0;
    while i < outlen && i < s_inc[64] as usize {
        out[outoff] = s_inc[HARAKAS_RATE - s_inc[64] as usize + i];
        outoff += 1;
        i += 1;
    }
    outlen -= i;
    s_inc[64] -= i as u8;

    while outlen > 0 {
        let mut tmp = [0u8; 64];
        tmp.copy_from_slice(&s_inc[..64]);
        haraka512_perm(&mut s_inc[..64], &tmp, ctx);
        let mut i = 0;
        while i < outlen && i < HARAKAS_RATE {
            out[outoff] = s_inc[i];
            outoff += 1;
            i += 1;
        }
        outlen -= i;
        s_inc[64] = (HARAKAS_RATE - i) as u8;
    }
}
