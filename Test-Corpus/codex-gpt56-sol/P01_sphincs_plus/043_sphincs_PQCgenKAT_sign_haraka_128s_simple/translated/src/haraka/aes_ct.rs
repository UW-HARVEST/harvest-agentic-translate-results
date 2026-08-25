//! Translation of `lib/haraka/src/haraka.c` lines 54-678: the bit-sliced,
//! constant-time AES helper routines (from BearSSL, written by Thomas Pornin).
//!
//! Every expression is transcribed verbatim from the C reference implementation
//! so that the behaviour is byte-identical. All arithmetic on `uint32_t` /
//! `uint64_t` in the original is bitwise (xor / and / or / shift), so no
//! wrapping arithmetic helpers are required here.

/// `static inline uint32_t br_dec32le(const unsigned char *src)`
pub(crate) unsafe fn br_dec32le(src: *const u8) -> u32 {
    (*src.add(0)) as u32
        | ((*src.add(1)) as u32) << 8
        | ((*src.add(2)) as u32) << 16
        | ((*src.add(3)) as u32) << 24
}

/// `static void br_range_dec32le(uint32_t *v, size_t num, const unsigned char *src)`
pub(crate) unsafe fn br_range_dec32le(v: *mut u32, num: usize, src: *const u8) {
    let mut v = v;
    let mut src = src;
    let mut num = num;
    while num > 0 {
        num -= 1;
        *v = br_dec32le(src);
        v = v.add(1);
        src = src.add(4);
    }
}

/// `static inline void br_enc32le(unsigned char *dst, uint32_t x)`
pub(crate) unsafe fn br_enc32le(dst: *mut u8, x: u32) {
    *dst.add(0) = x as u8;
    *dst.add(1) = (x >> 8) as u8;
    *dst.add(2) = (x >> 16) as u8;
    *dst.add(3) = (x >> 24) as u8;
}

/// `static void br_range_enc32le(unsigned char *dst, const uint32_t *v, size_t num)`
pub(crate) unsafe fn br_range_enc32le(dst: *mut u8, v: *const u32, num: usize) {
    let mut dst = dst;
    let mut v = v;
    let mut num = num;
    while num > 0 {
        num -= 1;
        br_enc32le(dst, *v);
        v = v.add(1);
        dst = dst.add(4);
    }
}

/// `static void br_aes_ct64_bitslice_Sbox(uint64_t *q)`
///
/// This S-box implementation is a straightforward translation of the circuit
/// described by Boyar and Peralta in "A new combinational logic minimization
/// technique with applications to cryptology"
/// (https://eprint.iacr.org/2009/191.pdf).
///
/// Note that variables x* (input) and s* (output) are numbered in "reverse"
/// order (x0 is the high bit, x7 is the low bit).
#[allow(non_snake_case)]
pub(crate) unsafe fn br_aes_ct64_bitslice_Sbox(q: *mut u64) {
    let x0: u64;
    let x1: u64;
    let x2: u64;
    let x3: u64;
    let x4: u64;
    let x5: u64;
    let x6: u64;
    let x7: u64;

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
    let y14: u64 = x3 ^ x5;
    let y13: u64 = x0 ^ x6;
    let y9: u64 = x0 ^ x3;
    let y8: u64 = x0 ^ x5;
    let t0: u64 = x1 ^ x2;
    let y1: u64 = t0 ^ x7;
    let y4: u64 = y1 ^ x3;
    let y12: u64 = y13 ^ y14;
    let y2: u64 = y1 ^ x0;
    let y5: u64 = y1 ^ x6;
    let y3: u64 = y5 ^ y8;
    let t1: u64 = x4 ^ y12;
    let y15: u64 = t1 ^ x5;
    let y20: u64 = t1 ^ x1;
    let y6: u64 = y15 ^ x7;
    let y10: u64 = y15 ^ t0;
    let y11: u64 = y20 ^ y9;
    let y7: u64 = x7 ^ y11;
    let y17: u64 = y10 ^ y11;
    let y19: u64 = y10 ^ y8;
    let y16: u64 = t0 ^ y11;
    let y21: u64 = y13 ^ y16;
    let y18: u64 = x0 ^ y16;

    /*
     * Non-linear section.
     */
    let t2: u64 = y12 & y15;
    let t3: u64 = y3 & y6;
    let t4: u64 = t3 ^ t2;
    let t5: u64 = y4 & x7;
    let t6: u64 = t5 ^ t2;
    let t7: u64 = y13 & y16;
    let t8: u64 = y5 & y1;
    let t9: u64 = t8 ^ t7;
    let t10: u64 = y2 & y7;
    let t11: u64 = t10 ^ t7;
    let t12: u64 = y9 & y11;
    let t13: u64 = y14 & y17;
    let t14: u64 = t13 ^ t12;
    let t15: u64 = y8 & y10;
    let t16: u64 = t15 ^ t12;
    let t17: u64 = t4 ^ t14;
    let t18: u64 = t6 ^ t16;
    let t19: u64 = t9 ^ t14;
    let t20: u64 = t11 ^ t16;
    let t21: u64 = t17 ^ y20;
    let t22: u64 = t18 ^ y19;
    let t23: u64 = t19 ^ y21;
    let t24: u64 = t20 ^ y18;

    let t25: u64 = t21 ^ t22;
    let t26: u64 = t21 & t23;
    let t27: u64 = t24 ^ t26;
    let t28: u64 = t25 & t27;
    let t29: u64 = t28 ^ t22;
    let t30: u64 = t23 ^ t24;
    let t31: u64 = t22 ^ t26;
    let t32: u64 = t31 & t30;
    let t33: u64 = t32 ^ t24;
    let t34: u64 = t23 ^ t33;
    let t35: u64 = t27 ^ t33;
    let t36: u64 = t24 & t35;
    let t37: u64 = t36 ^ t34;
    let t38: u64 = t27 ^ t36;
    let t39: u64 = t29 & t38;
    let t40: u64 = t25 ^ t39;

    let t41: u64 = t40 ^ t37;
    let t42: u64 = t29 ^ t33;
    let t43: u64 = t29 ^ t40;
    let t44: u64 = t33 ^ t37;
    let t45: u64 = t42 ^ t41;
    let z0: u64 = t44 & y15;
    let z1: u64 = t37 & y6;
    let z2: u64 = t33 & x7;
    let z3: u64 = t43 & y16;
    let z4: u64 = t40 & y1;
    let z5: u64 = t29 & y7;
    let z6: u64 = t42 & y11;
    let z7: u64 = t45 & y17;
    let z8: u64 = t41 & y10;
    let z9: u64 = t44 & y12;
    let z10: u64 = t37 & y3;
    let z11: u64 = t33 & y4;
    let z12: u64 = t43 & y13;
    let z13: u64 = t40 & y5;
    let z14: u64 = t29 & y2;
    let z15: u64 = t42 & y9;
    let z16: u64 = t45 & y14;
    let z17: u64 = t41 & y8;

    /*
     * Bottom linear transformation.
     */
    let t46: u64 = z15 ^ z16;
    let t47: u64 = z10 ^ z11;
    let t48: u64 = z5 ^ z13;
    let t49: u64 = z9 ^ z10;
    let t50: u64 = z2 ^ z12;
    let t51: u64 = z2 ^ z5;
    let t52: u64 = z7 ^ z8;
    let t53: u64 = z0 ^ z3;
    let t54: u64 = z6 ^ z7;
    let t55: u64 = z16 ^ z17;
    let t56: u64 = z12 ^ t48;
    let t57: u64 = t50 ^ t53;
    let t58: u64 = z4 ^ t46;
    let t59: u64 = z3 ^ t54;
    let t60: u64 = t46 ^ t57;
    let t61: u64 = z14 ^ t57;
    let t62: u64 = t52 ^ t58;
    let t63: u64 = t49 ^ t58;
    let t64: u64 = z4 ^ t59;
    let t65: u64 = t61 ^ t62;
    let t66: u64 = z1 ^ t63;
    let s0: u64 = t59 ^ t63;
    let s6: u64 = t56 ^ !t62;
    let s7: u64 = t48 ^ !t60;
    let t67: u64 = t64 ^ t65;
    let s3: u64 = t53 ^ t66;
    let s4: u64 = t51 ^ t66;
    let s5: u64 = t47 ^ t65;
    let s1: u64 = t64 ^ !s3;
    let s2: u64 = t55 ^ !t67;

    *q.add(7) = s0;
    *q.add(6) = s1;
    *q.add(5) = s2;
    *q.add(4) = s3;
    *q.add(3) = s4;
    *q.add(2) = s5;
    *q.add(1) = s6;
    *q.add(0) = s7;
}

/// `static void br_aes_ct_bitslice_Sbox(uint32_t *q)`
///
/// Same Boyar/Peralta circuit as `br_aes_ct64_bitslice_Sbox`, on 32-bit words.
#[allow(non_snake_case)]
pub(crate) unsafe fn br_aes_ct_bitslice_Sbox(q: *mut u32) {
    let x0: u32;
    let x1: u32;
    let x2: u32;
    let x3: u32;
    let x4: u32;
    let x5: u32;
    let x6: u32;
    let x7: u32;

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
    let y14: u32 = x3 ^ x5;
    let y13: u32 = x0 ^ x6;
    let y9: u32 = x0 ^ x3;
    let y8: u32 = x0 ^ x5;
    let t0: u32 = x1 ^ x2;
    let y1: u32 = t0 ^ x7;
    let y4: u32 = y1 ^ x3;
    let y12: u32 = y13 ^ y14;
    let y2: u32 = y1 ^ x0;
    let y5: u32 = y1 ^ x6;
    let y3: u32 = y5 ^ y8;
    let t1: u32 = x4 ^ y12;
    let y15: u32 = t1 ^ x5;
    let y20: u32 = t1 ^ x1;
    let y6: u32 = y15 ^ x7;
    let y10: u32 = y15 ^ t0;
    let y11: u32 = y20 ^ y9;
    let y7: u32 = x7 ^ y11;
    let y17: u32 = y10 ^ y11;
    let y19: u32 = y10 ^ y8;
    let y16: u32 = t0 ^ y11;
    let y21: u32 = y13 ^ y16;
    let y18: u32 = x0 ^ y16;

    /*
     * Non-linear section.
     */
    let t2: u32 = y12 & y15;
    let t3: u32 = y3 & y6;
    let t4: u32 = t3 ^ t2;
    let t5: u32 = y4 & x7;
    let t6: u32 = t5 ^ t2;
    let t7: u32 = y13 & y16;
    let t8: u32 = y5 & y1;
    let t9: u32 = t8 ^ t7;
    let t10: u32 = y2 & y7;
    let t11: u32 = t10 ^ t7;
    let t12: u32 = y9 & y11;
    let t13: u32 = y14 & y17;
    let t14: u32 = t13 ^ t12;
    let t15: u32 = y8 & y10;
    let t16: u32 = t15 ^ t12;
    let t17: u32 = t4 ^ t14;
    let t18: u32 = t6 ^ t16;
    let t19: u32 = t9 ^ t14;
    let t20: u32 = t11 ^ t16;
    let t21: u32 = t17 ^ y20;
    let t22: u32 = t18 ^ y19;
    let t23: u32 = t19 ^ y21;
    let t24: u32 = t20 ^ y18;

    let t25: u32 = t21 ^ t22;
    let t26: u32 = t21 & t23;
    let t27: u32 = t24 ^ t26;
    let t28: u32 = t25 & t27;
    let t29: u32 = t28 ^ t22;
    let t30: u32 = t23 ^ t24;
    let t31: u32 = t22 ^ t26;
    let t32: u32 = t31 & t30;
    let t33: u32 = t32 ^ t24;
    let t34: u32 = t23 ^ t33;
    let t35: u32 = t27 ^ t33;
    let t36: u32 = t24 & t35;
    let t37: u32 = t36 ^ t34;
    let t38: u32 = t27 ^ t36;
    let t39: u32 = t29 & t38;
    let t40: u32 = t25 ^ t39;

    let t41: u32 = t40 ^ t37;
    let t42: u32 = t29 ^ t33;
    let t43: u32 = t29 ^ t40;
    let t44: u32 = t33 ^ t37;
    let t45: u32 = t42 ^ t41;
    let z0: u32 = t44 & y15;
    let z1: u32 = t37 & y6;
    let z2: u32 = t33 & x7;
    let z3: u32 = t43 & y16;
    let z4: u32 = t40 & y1;
    let z5: u32 = t29 & y7;
    let z6: u32 = t42 & y11;
    let z7: u32 = t45 & y17;
    let z8: u32 = t41 & y10;
    let z9: u32 = t44 & y12;
    let z10: u32 = t37 & y3;
    let z11: u32 = t33 & y4;
    let z12: u32 = t43 & y13;
    let z13: u32 = t40 & y5;
    let z14: u32 = t29 & y2;
    let z15: u32 = t42 & y9;
    let z16: u32 = t45 & y14;
    let z17: u32 = t41 & y8;

    /*
     * Bottom linear transformation.
     */
    let t46: u32 = z15 ^ z16;
    let t47: u32 = z10 ^ z11;
    let t48: u32 = z5 ^ z13;
    let t49: u32 = z9 ^ z10;
    let t50: u32 = z2 ^ z12;
    let t51: u32 = z2 ^ z5;
    let t52: u32 = z7 ^ z8;
    let t53: u32 = z0 ^ z3;
    let t54: u32 = z6 ^ z7;
    let t55: u32 = z16 ^ z17;
    let t56: u32 = z12 ^ t48;
    let t57: u32 = t50 ^ t53;
    let t58: u32 = z4 ^ t46;
    let t59: u32 = z3 ^ t54;
    let t60: u32 = t46 ^ t57;
    let t61: u32 = z14 ^ t57;
    let t62: u32 = t52 ^ t58;
    let t63: u32 = t49 ^ t58;
    let t64: u32 = z4 ^ t59;
    let t65: u32 = t61 ^ t62;
    let t66: u32 = z1 ^ t63;
    let s0: u32 = t59 ^ t63;
    let s6: u32 = t56 ^ !t62;
    let s7: u32 = t48 ^ !t60;
    let t67: u32 = t64 ^ t65;
    let s3: u32 = t53 ^ t66;
    let s4: u32 = t51 ^ t66;
    let s5: u32 = t47 ^ t65;
    let s1: u32 = t64 ^ !s3;
    let s2: u32 = t55 ^ !t67;

    *q.add(7) = s0;
    *q.add(6) = s1;
    *q.add(5) = s2;
    *q.add(4) = s3;
    *q.add(3) = s4;
    *q.add(2) = s5;
    *q.add(1) = s6;
    *q.add(0) = s7;
}

/// `static void br_aes_ct_ortho(uint32_t *q)`
pub(crate) unsafe fn br_aes_ct_ortho(q: *mut u32) {
    // #define SWAPN_32(cl, ch, s, x, y)
    macro_rules! swapn_32 {
        ($cl:expr, $ch:expr, $s:expr, $x:expr, $y:expr) => {{
            let a: u32 = *q.add($x);
            let b: u32 = *q.add($y);
            *q.add($x) = (a & ($cl as u32)) | ((b & ($cl as u32)) << $s);
            *q.add($y) = ((a & ($ch as u32)) >> $s) | (b & ($ch as u32));
        }};
    }
    // #define SWAP2_32(x, y)   SWAPN_32(0x55555555, 0xAAAAAAAA, 1, x, y)
    macro_rules! swap2_32 {
        ($x:expr, $y:expr) => {
            swapn_32!(0x55555555u32, 0xAAAAAAAAu32, 1, $x, $y)
        };
    }
    // #define SWAP4_32(x, y)   SWAPN_32(0x33333333, 0xCCCCCCCC, 2, x, y)
    macro_rules! swap4_32 {
        ($x:expr, $y:expr) => {
            swapn_32!(0x33333333u32, 0xCCCCCCCCu32, 2, $x, $y)
        };
    }
    // #define SWAP8_32(x, y)   SWAPN_32(0x0F0F0F0F, 0xF0F0F0F0, 4, x, y)
    macro_rules! swap8_32 {
        ($x:expr, $y:expr) => {
            swapn_32!(0x0F0F0F0Fu32, 0xF0F0F0F0u32, 4, $x, $y)
        };
    }

    swap2_32!(0, 1);
    swap2_32!(2, 3);
    swap2_32!(4, 5);
    swap2_32!(6, 7);

    swap4_32!(0, 2);
    swap4_32!(1, 3);
    swap4_32!(4, 6);
    swap4_32!(5, 7);

    swap8_32!(0, 4);
    swap8_32!(1, 5);
    swap8_32!(2, 6);
    swap8_32!(3, 7);
}

/// `static inline void add_round_key32(uint32_t *q, const uint32_t *sk)`
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

/// `static inline void shift_rows32(uint32_t *q)`
pub(crate) unsafe fn shift_rows32(q: *mut u32) {
    for i in 0..8usize {
        let x: u32 = *q.add(i);
        *q.add(i) = (x & 0x000000FF)
            | ((x & 0x0000FC00) >> 2)
            | ((x & 0x00000300) << 6)
            | ((x & 0x00F00000) >> 4)
            | ((x & 0x000F0000) << 4)
            | ((x & 0xC0000000) >> 6)
            | ((x & 0x3F000000) << 2);
    }
}

/// `static inline uint32_t rotr16(uint32_t x)`
pub(crate) fn rotr16(x: u32) -> u32 {
    (x << 16) | (x >> 16)
}

/// `static inline void mix_columns32(uint32_t *q)`
pub(crate) unsafe fn mix_columns32(q: *mut u32) {
    let q0: u32 = *q.add(0);
    let q1: u32 = *q.add(1);
    let q2: u32 = *q.add(2);
    let q3: u32 = *q.add(3);
    let q4: u32 = *q.add(4);
    let q5: u32 = *q.add(5);
    let q6: u32 = *q.add(6);
    let q7: u32 = *q.add(7);
    let r0: u32 = (q0 >> 8) | (q0 << 24);
    let r1: u32 = (q1 >> 8) | (q1 << 24);
    let r2: u32 = (q2 >> 8) | (q2 << 24);
    let r3: u32 = (q3 >> 8) | (q3 << 24);
    let r4: u32 = (q4 >> 8) | (q4 << 24);
    let r5: u32 = (q5 >> 8) | (q5 << 24);
    let r6: u32 = (q6 >> 8) | (q6 << 24);
    let r7: u32 = (q7 >> 8) | (q7 << 24);

    *q.add(0) = q7 ^ r7 ^ r0 ^ rotr16(q0 ^ r0);
    *q.add(1) = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ rotr16(q1 ^ r1);
    *q.add(2) = q1 ^ r1 ^ r2 ^ rotr16(q2 ^ r2);
    *q.add(3) = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ rotr16(q3 ^ r3);
    *q.add(4) = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ rotr16(q4 ^ r4);
    *q.add(5) = q4 ^ r4 ^ r5 ^ rotr16(q5 ^ r5);
    *q.add(6) = q5 ^ r5 ^ r6 ^ rotr16(q6 ^ r6);
    *q.add(7) = q6 ^ r6 ^ r7 ^ rotr16(q7 ^ r7);
}

/// `static void br_aes_ct64_ortho(uint64_t *q)`
pub(crate) unsafe fn br_aes_ct64_ortho(q: *mut u64) {
    // #define SWAPN(cl, ch, s, x, y)
    macro_rules! swapn {
        ($cl:expr, $ch:expr, $s:expr, $x:expr, $y:expr) => {{
            let a: u64 = *q.add($x);
            let b: u64 = *q.add($y);
            *q.add($x) = (a & ($cl as u64)) | ((b & ($cl as u64)) << $s);
            *q.add($y) = ((a & ($ch as u64)) >> $s) | (b & ($ch as u64));
        }};
    }
    // #define SWAP2(x, y) SWAPN(0x5555555555555555, 0xAAAAAAAAAAAAAAAA, 1, x, y)
    macro_rules! swap2 {
        ($x:expr, $y:expr) => {
            swapn!(0x5555555555555555u64, 0xAAAAAAAAAAAAAAAAu64, 1, $x, $y)
        };
    }
    // #define SWAP4(x, y) SWAPN(0x3333333333333333, 0xCCCCCCCCCCCCCCCC, 2, x, y)
    macro_rules! swap4 {
        ($x:expr, $y:expr) => {
            swapn!(0x3333333333333333u64, 0xCCCCCCCCCCCCCCCCu64, 2, $x, $y)
        };
    }
    // #define SWAP8(x, y) SWAPN(0x0F0F0F0F0F0F0F0F, 0xF0F0F0F0F0F0F0F0, 4, x, y)
    macro_rules! swap8 {
        ($x:expr, $y:expr) => {
            swapn!(0x0F0F0F0F0F0F0F0Fu64, 0xF0F0F0F0F0F0F0F0u64, 4, $x, $y)
        };
    }

    swap2!(0, 1);
    swap2!(2, 3);
    swap2!(4, 5);
    swap2!(6, 7);

    swap4!(0, 2);
    swap4!(1, 3);
    swap4!(4, 6);
    swap4!(5, 7);

    swap8!(0, 4);
    swap8!(1, 5);
    swap8!(2, 6);
    swap8!(3, 7);
}

/// `static void br_aes_ct64_interleave_in(uint64_t *q0, uint64_t *q1, const uint32_t *w)`
pub(crate) unsafe fn br_aes_ct64_interleave_in(q0: *mut u64, q1: *mut u64, w: *const u32) {
    let mut x0: u64;
    let mut x1: u64;
    let mut x2: u64;
    let mut x3: u64;

    x0 = (*w.add(0)) as u64;
    x1 = (*w.add(1)) as u64;
    x2 = (*w.add(2)) as u64;
    x3 = (*w.add(3)) as u64;
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

/// `static void br_aes_ct64_interleave_out(uint32_t *w, uint64_t q0, uint64_t q1)`
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

/// `static inline void add_round_key(uint64_t *q, const uint64_t *sk)`
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

/// `static inline void shift_rows(uint64_t *q)`
pub(crate) unsafe fn shift_rows(q: *mut u64) {
    for i in 0..8usize {
        let x: u64 = *q.add(i);
        *q.add(i) = (x & 0x000000000000FFFFu64)
            | ((x & 0x00000000FFF00000u64) >> 4)
            | ((x & 0x00000000000F0000u64) << 12)
            | ((x & 0x0000FF0000000000u64) >> 8)
            | ((x & 0x000000FF00000000u64) << 8)
            | ((x & 0xF000000000000000u64) >> 12)
            | ((x & 0x0FFF000000000000u64) << 4);
    }
}

/// `static inline uint64_t rotr32(uint64_t x)`
pub(crate) fn rotr32(x: u64) -> u64 {
    (x << 32) | (x >> 32)
}

/// `static inline void mix_columns(uint64_t *q)`
pub(crate) unsafe fn mix_columns(q: *mut u64) {
    let q0: u64 = *q.add(0);
    let q1: u64 = *q.add(1);
    let q2: u64 = *q.add(2);
    let q3: u64 = *q.add(3);
    let q4: u64 = *q.add(4);
    let q5: u64 = *q.add(5);
    let q6: u64 = *q.add(6);
    let q7: u64 = *q.add(7);
    let r0: u64 = (q0 >> 16) | (q0 << 48);
    let r1: u64 = (q1 >> 16) | (q1 << 48);
    let r2: u64 = (q2 >> 16) | (q2 << 48);
    let r3: u64 = (q3 >> 16) | (q3 << 48);
    let r4: u64 = (q4 >> 16) | (q4 << 48);
    let r5: u64 = (q5 >> 16) | (q5 << 48);
    let r6: u64 = (q6 >> 16) | (q6 << 48);
    let r7: u64 = (q7 >> 16) | (q7 << 48);

    *q.add(0) = q7 ^ r7 ^ r0 ^ rotr32(q0 ^ r0);
    *q.add(1) = q0 ^ r0 ^ q7 ^ r7 ^ r1 ^ rotr32(q1 ^ r1);
    *q.add(2) = q1 ^ r1 ^ r2 ^ rotr32(q2 ^ r2);
    *q.add(3) = q2 ^ r2 ^ q7 ^ r7 ^ r3 ^ rotr32(q3 ^ r3);
    *q.add(4) = q3 ^ r3 ^ q7 ^ r7 ^ r4 ^ rotr32(q4 ^ r4);
    *q.add(5) = q4 ^ r4 ^ r5 ^ rotr32(q5 ^ r5);
    *q.add(6) = q5 ^ r5 ^ r6 ^ rotr32(q6 ^ r6);
    *q.add(7) = q6 ^ r6 ^ r7 ^ rotr32(q7 ^ r7);
}
