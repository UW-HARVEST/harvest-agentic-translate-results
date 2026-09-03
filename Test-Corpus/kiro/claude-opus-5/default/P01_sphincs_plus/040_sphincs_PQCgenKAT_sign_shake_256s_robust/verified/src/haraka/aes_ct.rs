/*
 * Constant time implementation of the Haraka hash function.
 *
 * The bit-sliced implementation of the AES round functions are
 * based on the AES implementation in BearSSL written
 * by Thomas Pornin <pornin@bolet.org>, licensed as follows:
 *
 * Copyright (c) 2016 Thomas Pornin <pornin@bolet.org>
 *
 * Permission is hereby granted, free of charge, to any person obtaining
 * a copy of this software and associated documentation files (the
 * "Software"), to deal in the Software without restriction, including
 * without limitation the rights to use, copy, modify, merge, publish,
 * distribute, sublicense, and/or sell copies of the Software, and to
 * permit persons to whom the Software is furnished to do so, subject to
 * the following conditions:
 *
 * The above copyright notice and this permission notice shall be
 * included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
 * MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
 * NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS
 * BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
 * ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
 * CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

#![allow(clippy::all)]
#![allow(non_upper_case_globals)]

pub(crate) const HARAKAS_RATE: usize = 32;

pub(crate) static HARAKA512_RC64: [[u64; 8]; 10] = [
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

pub(crate) unsafe fn br_dec32le(src: *const u8) -> u32 {
    (*src.add(0) as u32)
        | ((*src.add(1) as u32) << 8)
        | ((*src.add(2) as u32) << 16)
        | ((*src.add(3) as u32) << 24)
}

pub(crate) unsafe fn br_range_dec32le(mut v: *mut u32, mut num: usize, mut src: *const u8) {
    while num > 0 {
        num -= 1;
        *v = br_dec32le(src);
        v = v.add(1);
        src = src.add(4);
    }
}

pub(crate) unsafe fn br_enc32le(dst: *mut u8, x: u32) {
    *dst.add(0) = x as u8;
    *dst.add(1) = (x >> 8) as u8;
    *dst.add(2) = (x >> 16) as u8;
    *dst.add(3) = (x >> 24) as u8;
}

pub(crate) unsafe fn br_range_enc32le(mut dst: *mut u8, mut v: *const u32, mut num: usize) {
    while num > 0 {
        num -= 1;
        br_enc32le(dst, *v);
        v = v.add(1);
        dst = dst.add(4);
    }
}

pub(crate) unsafe fn br_aes_ct64_bitslice_Sbox(q: *mut u64) {
    /*
     * This S-box implementation is a straightforward translation of
     * the circuit described by Boyar and Peralta in "A new
     * combinational logic minimization technique with applications
     * to cryptology" (https://eprint.iacr.org/2009/191.pdf).
     *
     * Note that variables x* (input) and s* (output) are numbered
     * in "reverse" order (x0 is the high bit, x7 is the low bit).
     */

    let x0: u64;
    let x1: u64;
    let x2: u64;
    let x3: u64;
    let x4: u64;
    let x5: u64;
    let x6: u64;
    let x7: u64;
    let y1: u64;
    let y2: u64;
    let y3: u64;
    let y4: u64;
    let y5: u64;
    let y6: u64;
    let y7: u64;
    let y8: u64;
    let y9: u64;
    let y10: u64;
    let y11: u64;
    let y12: u64;
    let y13: u64;
    let y14: u64;
    let y15: u64;
    let y16: u64;
    let y17: u64;
    let y18: u64;
    let y19: u64;
    let y20: u64;
    let y21: u64;
    let z0: u64;
    let z1: u64;
    let z2: u64;
    let z3: u64;
    let z4: u64;
    let z5: u64;
    let z6: u64;
    let z7: u64;
    let z8: u64;
    let z9: u64;
    let z10: u64;
    let z11: u64;
    let z12: u64;
    let z13: u64;
    let z14: u64;
    let z15: u64;
    let z16: u64;
    let z17: u64;
    let t0: u64;
    let t1: u64;
    let t2: u64;
    let t3: u64;
    let t4: u64;
    let t5: u64;
    let t6: u64;
    let t7: u64;
    let t8: u64;
    let t9: u64;
    let t10: u64;
    let t11: u64;
    let t12: u64;
    let t13: u64;
    let t14: u64;
    let t15: u64;
    let t16: u64;
    let t17: u64;
    let t18: u64;
    let t19: u64;
    let t20: u64;
    let t21: u64;
    let t22: u64;
    let t23: u64;
    let t24: u64;
    let t25: u64;
    let t26: u64;
    let t27: u64;
    let t28: u64;
    let t29: u64;
    let t30: u64;
    let t31: u64;
    let t32: u64;
    let t33: u64;
    let t34: u64;
    let t35: u64;
    let t36: u64;
    let t37: u64;
    let t38: u64;
    let t39: u64;
    let t40: u64;
    let t41: u64;
    let t42: u64;
    let t43: u64;
    let t44: u64;
    let t45: u64;
    let t46: u64;
    let t47: u64;
    let t48: u64;
    let t49: u64;
    let t50: u64;
    let t51: u64;
    let t52: u64;
    let t53: u64;
    let t54: u64;
    let t55: u64;
    let t56: u64;
    let t57: u64;
    let t58: u64;
    let t59: u64;
    let t60: u64;
    let t61: u64;
    let t62: u64;
    let t63: u64;
    let t64: u64;
    let t65: u64;
    let t66: u64;
    let t67: u64;
    let s0: u64;
    let s1: u64;
    let s2: u64;
    let s3: u64;
    let s4: u64;
    let s5: u64;
    let s6: u64;
    let s7: u64;

    x0 = *q.add(7);
    x1 = *q.add(6);
    x2 = *q.add(5);
    x3 = *q.add(4);
    x4 = *q.add(3);
    x5 = *q.add(2);
    x6 = *q.add(1);
    x7 = *q.add(0);

    /*
     * Top linear transformation.
     */
    y14 = x3 ^ x5;
    y13 = x0 ^ x6;
    y9 = x0 ^ x3;
    y8 = x0 ^ x5;
    t0 = x1 ^ x2;
    y1 = t0 ^ x7;
    y4 = y1 ^ x3;
    y12 = y13 ^ y14;
    y2 = y1 ^ x0;
    y5 = y1 ^ x6;
    y3 = y5 ^ y8;
    t1 = x4 ^ y12;
    y15 = t1 ^ x5;
    y20 = t1 ^ x1;
    y6 = y15 ^ x7;
    y10 = y15 ^ t0;
    y11 = y20 ^ y9;
    y7 = x7 ^ y11;
    y17 = y10 ^ y11;
    y19 = y10 ^ y8;
    y16 = t0 ^ y11;
    y21 = y13 ^ y16;
    y18 = x0 ^ y16;

    /*
     * Non-linear section.
     */
    t2 = y12 & y15;
    t3 = y3 & y6;
    t4 = t3 ^ t2;
    t5 = y4 & x7;
    t6 = t5 ^ t2;
    t7 = y13 & y16;
    t8 = y5 & y1;
    t9 = t8 ^ t7;
    t10 = y2 & y7;
    t11 = t10 ^ t7;
    t12 = y9 & y11;
    t13 = y14 & y17;
    t14 = t13 ^ t12;
    t15 = y8 & y10;
    t16 = t15 ^ t12;
    t17 = t4 ^ t14;
    t18 = t6 ^ t16;
    t19 = t9 ^ t14;
    t20 = t11 ^ t16;
    t21 = t17 ^ y20;
    t22 = t18 ^ y19;
    t23 = t19 ^ y21;
    t24 = t20 ^ y18;

    t25 = t21 ^ t22;
    t26 = t21 & t23;
    t27 = t24 ^ t26;
    t28 = t25 & t27;
    t29 = t28 ^ t22;
    t30 = t23 ^ t24;
    t31 = t22 ^ t26;
    t32 = t31 & t30;
    t33 = t32 ^ t24;
    t34 = t23 ^ t33;
    t35 = t27 ^ t33;
    t36 = t24 & t35;
    t37 = t36 ^ t34;
    t38 = t27 ^ t36;
    t39 = t29 & t38;
    t40 = t25 ^ t39;

    t41 = t40 ^ t37;
    t42 = t29 ^ t33;
    t43 = t29 ^ t40;
    t44 = t33 ^ t37;
    t45 = t42 ^ t41;
    z0 = t44 & y15;
    z1 = t37 & y6;
    z2 = t33 & x7;
    z3 = t43 & y16;
    z4 = t40 & y1;
    z5 = t29 & y7;
    z6 = t42 & y11;
    z7 = t45 & y17;
    z8 = t41 & y10;
    z9 = t44 & y12;
    z10 = t37 & y3;
    z11 = t33 & y4;
    z12 = t43 & y13;
    z13 = t40 & y5;
    z14 = t29 & y2;
    z15 = t42 & y9;
    z16 = t45 & y14;
    z17 = t41 & y8;

    /*
     * Bottom linear transformation.
     */
    t46 = z15 ^ z16;
    t47 = z10 ^ z11;
    t48 = z5 ^ z13;
    t49 = z9 ^ z10;
    t50 = z2 ^ z12;
    t51 = z2 ^ z5;
    t52 = z7 ^ z8;
    t53 = z0 ^ z3;
    t54 = z6 ^ z7;
    t55 = z16 ^ z17;
    t56 = z12 ^ t48;
    t57 = t50 ^ t53;
    t58 = z4 ^ t46;
    t59 = z3 ^ t54;
    t60 = t46 ^ t57;
    t61 = z14 ^ t57;
    t62 = t52 ^ t58;
    t63 = t49 ^ t58;
    t64 = z4 ^ t59;
    t65 = t61 ^ t62;
    t66 = z1 ^ t63;
    s0 = t59 ^ t63;
    s6 = t56 ^ !t62;
    s7 = t48 ^ !t60;
    t67 = t64 ^ t65;
    s3 = t53 ^ t66;
    s4 = t51 ^ t66;
    s5 = t47 ^ t65;
    s1 = t64 ^ !s3;
    s2 = t55 ^ !t67;

    *q.add(7) = s0;
    *q.add(6) = s1;
    *q.add(5) = s2;
    *q.add(4) = s3;
    *q.add(3) = s4;
    *q.add(2) = s5;
    *q.add(1) = s6;
    *q.add(0) = s7;
}

pub(crate) unsafe fn br_aes_ct_bitslice_Sbox(q: *mut u32) {
    /*
     * This S-box implementation is a straightforward translation of
     * the circuit described by Boyar and Peralta in "A new
     * combinational logic minimization technique with applications
     * to cryptology" (https://eprint.iacr.org/2009/191.pdf).
     *
     * Note that variables x* (input) and s* (output) are numbered
     * in "reverse" order (x0 is the high bit, x7 is the low bit).
     */

    let x0: u32;
    let x1: u32;
    let x2: u32;
    let x3: u32;
    let x4: u32;
    let x5: u32;
    let x6: u32;
    let x7: u32;
    let y1: u32;
    let y2: u32;
    let y3: u32;
    let y4: u32;
    let y5: u32;
    let y6: u32;
    let y7: u32;
    let y8: u32;
    let y9: u32;
    let y10: u32;
    let y11: u32;
    let y12: u32;
    let y13: u32;
    let y14: u32;
    let y15: u32;
    let y16: u32;
    let y17: u32;
    let y18: u32;
    let y19: u32;
    let y20: u32;
    let y21: u32;
    let z0: u32;
    let z1: u32;
    let z2: u32;
    let z3: u32;
    let z4: u32;
    let z5: u32;
    let z6: u32;
    let z7: u32;
    let z8: u32;
    let z9: u32;
    let z10: u32;
    let z11: u32;
    let z12: u32;
    let z13: u32;
    let z14: u32;
    let z15: u32;
    let z16: u32;
    let z17: u32;
    let t0: u32;
    let t1: u32;
    let t2: u32;
    let t3: u32;
    let t4: u32;
    let t5: u32;
    let t6: u32;
    let t7: u32;
    let t8: u32;
    let t9: u32;
    let t10: u32;
    let t11: u32;
    let t12: u32;
    let t13: u32;
    let t14: u32;
    let t15: u32;
    let t16: u32;
    let t17: u32;
    let t18: u32;
    let t19: u32;
    let t20: u32;
    let t21: u32;
    let t22: u32;
    let t23: u32;
    let t24: u32;
    let t25: u32;
    let t26: u32;
    let t27: u32;
    let t28: u32;
    let t29: u32;
    let t30: u32;
    let t31: u32;
    let t32: u32;
    let t33: u32;
    let t34: u32;
    let t35: u32;
    let t36: u32;
    let t37: u32;
    let t38: u32;
    let t39: u32;
    let t40: u32;
    let t41: u32;
    let t42: u32;
    let t43: u32;
    let t44: u32;
    let t45: u32;
    let t46: u32;
    let t47: u32;
    let t48: u32;
    let t49: u32;
    let t50: u32;
    let t51: u32;
    let t52: u32;
    let t53: u32;
    let t54: u32;
    let t55: u32;
    let t56: u32;
    let t57: u32;
    let t58: u32;
    let t59: u32;
    let t60: u32;
    let t61: u32;
    let t62: u32;
    let t63: u32;
    let t64: u32;
    let t65: u32;
    let t66: u32;
    let t67: u32;
    let s0: u32;
    let s1: u32;
    let s2: u32;
    let s3: u32;
    let s4: u32;
    let s5: u32;
    let s6: u32;
    let s7: u32;

    x0 = *q.add(7);
    x1 = *q.add(6);
    x2 = *q.add(5);
    x3 = *q.add(4);
    x4 = *q.add(3);
    x5 = *q.add(2);
    x6 = *q.add(1);
    x7 = *q.add(0);

    /*
     * Top linear transformation.
     */
    y14 = x3 ^ x5;
    y13 = x0 ^ x6;
    y9 = x0 ^ x3;
    y8 = x0 ^ x5;
    t0 = x1 ^ x2;
    y1 = t0 ^ x7;
    y4 = y1 ^ x3;
    y12 = y13 ^ y14;
    y2 = y1 ^ x0;
    y5 = y1 ^ x6;
    y3 = y5 ^ y8;
    t1 = x4 ^ y12;
    y15 = t1 ^ x5;
    y20 = t1 ^ x1;
    y6 = y15 ^ x7;
    y10 = y15 ^ t0;
    y11 = y20 ^ y9;
    y7 = x7 ^ y11;
    y17 = y10 ^ y11;
    y19 = y10 ^ y8;
    y16 = t0 ^ y11;
    y21 = y13 ^ y16;
    y18 = x0 ^ y16;

    /*
     * Non-linear section.
     */
    t2 = y12 & y15;
    t3 = y3 & y6;
    t4 = t3 ^ t2;
    t5 = y4 & x7;
    t6 = t5 ^ t2;
    t7 = y13 & y16;
    t8 = y5 & y1;
    t9 = t8 ^ t7;
    t10 = y2 & y7;
    t11 = t10 ^ t7;
    t12 = y9 & y11;
    t13 = y14 & y17;
    t14 = t13 ^ t12;
    t15 = y8 & y10;
    t16 = t15 ^ t12;
    t17 = t4 ^ t14;
    t18 = t6 ^ t16;
    t19 = t9 ^ t14;
    t20 = t11 ^ t16;
    t21 = t17 ^ y20;
    t22 = t18 ^ y19;
    t23 = t19 ^ y21;
    t24 = t20 ^ y18;

    t25 = t21 ^ t22;
    t26 = t21 & t23;
    t27 = t24 ^ t26;
    t28 = t25 & t27;
    t29 = t28 ^ t22;
    t30 = t23 ^ t24;
    t31 = t22 ^ t26;
    t32 = t31 & t30;
    t33 = t32 ^ t24;
    t34 = t23 ^ t33;
    t35 = t27 ^ t33;
    t36 = t24 & t35;
    t37 = t36 ^ t34;
    t38 = t27 ^ t36;
    t39 = t29 & t38;
    t40 = t25 ^ t39;

    t41 = t40 ^ t37;
    t42 = t29 ^ t33;
    t43 = t29 ^ t40;
    t44 = t33 ^ t37;
    t45 = t42 ^ t41;
    z0 = t44 & y15;
    z1 = t37 & y6;
    z2 = t33 & x7;
    z3 = t43 & y16;
    z4 = t40 & y1;
    z5 = t29 & y7;
    z6 = t42 & y11;
    z7 = t45 & y17;
    z8 = t41 & y10;
    z9 = t44 & y12;
    z10 = t37 & y3;
    z11 = t33 & y4;
    z12 = t43 & y13;
    z13 = t40 & y5;
    z14 = t29 & y2;
    z15 = t42 & y9;
    z16 = t45 & y14;
    z17 = t41 & y8;

    /*
     * Bottom linear transformation.
     */
    t46 = z15 ^ z16;
    t47 = z10 ^ z11;
    t48 = z5 ^ z13;
    t49 = z9 ^ z10;
    t50 = z2 ^ z12;
    t51 = z2 ^ z5;
    t52 = z7 ^ z8;
    t53 = z0 ^ z3;
    t54 = z6 ^ z7;
    t55 = z16 ^ z17;
    t56 = z12 ^ t48;
    t57 = t50 ^ t53;
    t58 = z4 ^ t46;
    t59 = z3 ^ t54;
    t60 = t46 ^ t57;
    t61 = z14 ^ t57;
    t62 = t52 ^ t58;
    t63 = t49 ^ t58;
    t64 = z4 ^ t59;
    t65 = t61 ^ t62;
    t66 = z1 ^ t63;
    s0 = t59 ^ t63;
    s6 = t56 ^ !t62;
    s7 = t48 ^ !t60;
    t67 = t64 ^ t65;
    s3 = t53 ^ t66;
    s4 = t51 ^ t66;
    s5 = t47 ^ t65;
    s1 = t64 ^ !s3;
    s2 = t55 ^ !t67;

    *q.add(7) = s0;
    *q.add(6) = s1;
    *q.add(5) = s2;
    *q.add(4) = s3;
    *q.add(3) = s4;
    *q.add(2) = s5;
    *q.add(1) = s6;
    *q.add(0) = s7;
}

pub(crate) unsafe fn br_aes_ct_ortho(q: *mut u32) {
    // #define SWAPN_32(cl, ch, s, x, y) do {
    //     uint32_t a, b;
    //     a = (x); b = (y);
    //     (x) = (a & (uint32_t)cl) | ((b & (uint32_t)cl) << (s));
    //     (y) = ((a & (uint32_t)ch) >> (s)) | (b & (uint32_t)ch);
    // } while (0)
    macro_rules! swapn_32 {
        ($cl:expr, $ch:expr, $s:expr, $x:expr, $y:expr) => {{
            let a: u32 = $x;
            let b: u32 = $y;
            $x = (a & ($cl as u32)) | ((b & ($cl as u32)) << ($s));
            $y = ((a & ($ch as u32)) >> ($s)) | (b & ($ch as u32));
        }};
    }
    macro_rules! swap2_32 {
        ($x:expr, $y:expr) => {
            swapn_32!(0x55555555u32, 0xAAAAAAAAu32, 1, $x, $y)
        };
    }
    macro_rules! swap4_32 {
        ($x:expr, $y:expr) => {
            swapn_32!(0x33333333u32, 0xCCCCCCCCu32, 2, $x, $y)
        };
    }
    macro_rules! swap8_32 {
        ($x:expr, $y:expr) => {
            swapn_32!(0x0F0F0F0Fu32, 0xF0F0F0F0u32, 4, $x, $y)
        };
    }

    swap2_32!(*q.add(0), *q.add(1));
    swap2_32!(*q.add(2), *q.add(3));
    swap2_32!(*q.add(4), *q.add(5));
    swap2_32!(*q.add(6), *q.add(7));

    swap4_32!(*q.add(0), *q.add(2));
    swap4_32!(*q.add(1), *q.add(3));
    swap4_32!(*q.add(4), *q.add(6));
    swap4_32!(*q.add(5), *q.add(7));

    swap8_32!(*q.add(0), *q.add(4));
    swap8_32!(*q.add(1), *q.add(5));
    swap8_32!(*q.add(2), *q.add(6));
    swap8_32!(*q.add(3), *q.add(7));
}

pub(crate) unsafe fn add_round_key32(q: *mut u32, sk: *const u32) {
    *q.add(0) ^= *sk.add(0);
    *q.add(1) ^= *sk.add(1);
    *q.add(2) ^= *sk.add(2);
    *q.add(3) ^= *sk.add(3);
    *q.add(4) ^= *sk.add(4);
    *q.add(5) ^= *sk.add(5);
    *q.add(6) ^= *sk.add(6);
    *q.add(7) ^= *sk.add(7);
}

pub(crate) unsafe fn shift_rows32(q: *mut u32) {
    let mut i: i32 = 0;
    while i < 8 {
        let x: u32;

        x = *q.add(i as usize);
        *q.add(i as usize) = (x & 0x000000FF)
            | ((x & 0x0000FC00) >> 2) | ((x & 0x00000300) << 6)
            | ((x & 0x00F00000) >> 4) | ((x & 0x000F0000) << 4)
            | ((x & 0xC0000000) >> 6) | ((x & 0x3F000000) << 2);
        i += 1;
    }
}

pub(crate) fn rotr16(x: u32) -> u32 {
    (x << 16) | (x >> 16)
}

pub(crate) unsafe fn mix_columns32(q: *mut u32) {
    let q0: u32;
    let q1: u32;
    let q2: u32;
    let q3: u32;
    let q4: u32;
    let q5: u32;
    let q6: u32;
    let q7: u32;
    let r0: u32;
    let r1: u32;
    let r2: u32;
    let r3: u32;
    let r4: u32;
    let r5: u32;
    let r6: u32;
    let r7: u32;

    q0 = *q.add(0);
    q1 = *q.add(1);
    q2 = *q.add(2);
    q3 = *q.add(3);
    q4 = *q.add(4);
    q5 = *q.add(5);
    q6 = *q.add(6);
    q7 = *q.add(7);
    r0 = (q0 >> 8) | (q0 << 24);
    r1 = (q1 >> 8) | (q1 << 24);
    r2 = (q2 >> 8) | (q2 << 24);
    r3 = (q3 >> 8) | (q3 << 24);
    r4 = (q4 >> 8) | (q4 << 24);
    r5 = (q5 >> 8) | (q5 << 24);
    r6 = (q6 >> 8) | (q6 << 24);
    r7 = (q7 >> 8) | (q7 << 24);

    *q.add(0) = q7 ^ r7 ^ r0 ^ rotr16(q0 ^ r0);
    *q.add(1) = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ rotr16(q1 ^ r1);
    *q.add(2) = q1 ^ r1 ^ r2 ^ rotr16(q2 ^ r2);
    *q.add(3) = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ rotr16(q3 ^ r3);
    *q.add(4) = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ rotr16(q4 ^ r4);
    *q.add(5) = q4 ^ r4 ^ r5 ^ rotr16(q5 ^ r5);
    *q.add(6) = q5 ^ r5 ^ r6 ^ rotr16(q6 ^ r6);
    *q.add(7) = q6 ^ r6 ^ r7 ^ rotr16(q7 ^ r7);
}

pub(crate) unsafe fn br_aes_ct64_ortho(q: *mut u64) {
    // #define SWAPN(cl, ch, s, x, y) do {
    //     uint64_t a, b;
    //     a = (x); b = (y);
    //     (x) = (a & (uint64_t)(cl)) | ((b & (uint64_t)(cl)) << (s));
    //     (y) = ((a & (uint64_t)(ch)) >> (s)) | (b & (uint64_t)(ch));
    // } while (0)
    macro_rules! swapn {
        ($cl:expr, $ch:expr, $s:expr, $x:expr, $y:expr) => {{
            let a: u64 = $x;
            let b: u64 = $y;
            $x = (a & ($cl as u64)) | ((b & ($cl as u64)) << ($s));
            $y = ((a & ($ch as u64)) >> ($s)) | (b & ($ch as u64));
        }};
    }
    macro_rules! swap2 {
        ($x:expr, $y:expr) => {
            swapn!(0x5555555555555555u64, 0xAAAAAAAAAAAAAAAAu64, 1, $x, $y)
        };
    }
    macro_rules! swap4 {
        ($x:expr, $y:expr) => {
            swapn!(0x3333333333333333u64, 0xCCCCCCCCCCCCCCCCu64, 2, $x, $y)
        };
    }
    macro_rules! swap8 {
        ($x:expr, $y:expr) => {
            swapn!(0x0F0F0F0F0F0F0F0Fu64, 0xF0F0F0F0F0F0F0F0u64, 4, $x, $y)
        };
    }

    swap2!(*q.add(0), *q.add(1));
    swap2!(*q.add(2), *q.add(3));
    swap2!(*q.add(4), *q.add(5));
    swap2!(*q.add(6), *q.add(7));

    swap4!(*q.add(0), *q.add(2));
    swap4!(*q.add(1), *q.add(3));
    swap4!(*q.add(4), *q.add(6));
    swap4!(*q.add(5), *q.add(7));

    swap8!(*q.add(0), *q.add(4));
    swap8!(*q.add(1), *q.add(5));
    swap8!(*q.add(2), *q.add(6));
    swap8!(*q.add(3), *q.add(7));
}

pub(crate) unsafe fn br_aes_ct64_interleave_in(q0: *mut u64, q1: *mut u64, w: *const u32) {
    let mut x0: u64;
    let mut x1: u64;
    let mut x2: u64;
    let mut x3: u64;

    x0 = *w.add(0) as u64;
    x1 = *w.add(1) as u64;
    x2 = *w.add(2) as u64;
    x3 = *w.add(3) as u64;
    x0 |= x0 << 16;
    x1 |= x1 << 16;
    x2 |= x2 << 16;
    x3 |= x3 << 16;
    x0 &= 0x0000FFFF0000FFFFu64;
    x1 &= 0x0000FFFF0000FFFFu64;
    x2 &= 0x0000FFFF0000FFFFu64;
    x3 &= 0x0000FFFF0000FFFFu64;
    x0 |= x0 << 8;
    x1 |= x1 << 8;
    x2 |= x2 << 8;
    x3 |= x3 << 8;
    x0 &= 0x00FF00FF00FF00FFu64;
    x1 &= 0x00FF00FF00FF00FFu64;
    x2 &= 0x00FF00FF00FF00FFu64;
    x3 &= 0x00FF00FF00FF00FFu64;
    *q0 = x0 | (x2 << 8);
    *q1 = x1 | (x3 << 8);
}

pub(crate) unsafe fn br_aes_ct64_interleave_out(w: *mut u32, q0: u64, q1: u64) {
    let mut x0: u64;
    let mut x1: u64;
    let mut x2: u64;
    let mut x3: u64;

    x0 = q0 & 0x00FF00FF00FF00FFu64;
    x1 = q1 & 0x00FF00FF00FF00FFu64;
    x2 = (q0 >> 8) & 0x00FF00FF00FF00FFu64;
    x3 = (q1 >> 8) & 0x00FF00FF00FF00FFu64;
    x0 |= x0 >> 8;
    x1 |= x1 >> 8;
    x2 |= x2 >> 8;
    x3 |= x3 >> 8;
    x0 &= 0x0000FFFF0000FFFFu64;
    x1 &= 0x0000FFFF0000FFFFu64;
    x2 &= 0x0000FFFF0000FFFFu64;
    x3 &= 0x0000FFFF0000FFFFu64;
    *w.add(0) = (x0 as u32) | ((x0 >> 16) as u32);
    *w.add(1) = (x1 as u32) | ((x1 >> 16) as u32);
    *w.add(2) = (x2 as u32) | ((x2 >> 16) as u32);
    *w.add(3) = (x3 as u32) | ((x3 >> 16) as u32);
}

pub(crate) unsafe fn add_round_key(q: *mut u64, sk: *const u64) {
    *q.add(0) ^= *sk.add(0);
    *q.add(1) ^= *sk.add(1);
    *q.add(2) ^= *sk.add(2);
    *q.add(3) ^= *sk.add(3);
    *q.add(4) ^= *sk.add(4);
    *q.add(5) ^= *sk.add(5);
    *q.add(6) ^= *sk.add(6);
    *q.add(7) ^= *sk.add(7);
}

pub(crate) unsafe fn shift_rows(q: *mut u64) {
    let mut i: i32 = 0;
    while i < 8 {
        let x: u64;

        x = *q.add(i as usize);
        *q.add(i as usize) = (x & 0x000000000000FFFFu64)
            | ((x & 0x00000000FFF00000u64) >> 4)
            | ((x & 0x00000000000F0000u64) << 12)
            | ((x & 0x0000FF0000000000u64) >> 8)
            | ((x & 0x000000FF00000000u64) << 8)
            | ((x & 0xF000000000000000u64) >> 12)
            | ((x & 0x0FFF000000000000u64) << 4);
        i += 1;
    }
}

pub(crate) fn rotr32(x: u64) -> u64 {
    (x << 32) | (x >> 32)
}

pub(crate) unsafe fn mix_columns(q: *mut u64) {
    let q0: u64;
    let q1: u64;
    let q2: u64;
    let q3: u64;
    let q4: u64;
    let q5: u64;
    let q6: u64;
    let q7: u64;
    let r0: u64;
    let r1: u64;
    let r2: u64;
    let r3: u64;
    let r4: u64;
    let r5: u64;
    let r6: u64;
    let r7: u64;

    q0 = *q.add(0);
    q1 = *q.add(1);
    q2 = *q.add(2);
    q3 = *q.add(3);
    q4 = *q.add(4);
    q5 = *q.add(5);
    q6 = *q.add(6);
    q7 = *q.add(7);
    r0 = (q0 >> 16) | (q0 << 48);
    r1 = (q1 >> 16) | (q1 << 48);
    r2 = (q2 >> 16) | (q2 << 48);
    r3 = (q3 >> 16) | (q3 << 48);
    r4 = (q4 >> 16) | (q4 << 48);
    r5 = (q5 >> 16) | (q5 << 48);
    r6 = (q6 >> 16) | (q6 << 48);
    r7 = (q7 >> 16) | (q7 << 48);

    *q.add(0) = q7 ^ r7 ^ r0 ^ rotr32(q0 ^ r0);
    *q.add(1) = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ rotr32(q1 ^ r1);
    *q.add(2) = q1 ^ r1 ^ r2 ^ rotr32(q2 ^ r2);
    *q.add(3) = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ rotr32(q3 ^ r3);
    *q.add(4) = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ rotr32(q4 ^ r4);
    *q.add(5) = q4 ^ r4 ^ r5 ^ rotr32(q5 ^ r5);
    *q.add(6) = q5 ^ r5 ^ r6 ^ rotr32(q6 ^ r6);
    *q.add(7) = q6 ^ r6 ^ r7 ^ rotr32(q7 ^ r7);
}
