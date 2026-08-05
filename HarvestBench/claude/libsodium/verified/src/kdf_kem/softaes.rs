// Translation of crypto_core/softaes/softaes.c (reference / non-FAVOR_PERFORMANCE path).
// Exported under the _sodium_softaes_* linker names (see quirks.h). The aead package
// also calls these via extern.

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SoftAesBlock {
    pub w0: u32,
    pub w1: u32,
    pub w2: u32,
    pub w3: u32,
}

static SBOX: [u8; 256] = [
    0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7, 0xab, 0x76,
    0xca, 0x82, 0xc9, 0x7d, 0xfa, 0x59, 0x47, 0xf0, 0xad, 0xd4, 0xa2, 0xaf, 0x9c, 0xa4, 0x72, 0xc0,
    0xb7, 0xfd, 0x93, 0x26, 0x36, 0x3f, 0xf7, 0xcc, 0x34, 0xa5, 0xe5, 0xf1, 0x71, 0xd8, 0x31, 0x15,
    0x04, 0xc7, 0x23, 0xc3, 0x18, 0x96, 0x05, 0x9a, 0x07, 0x12, 0x80, 0xe2, 0xeb, 0x27, 0xb2, 0x75,
    0x09, 0x83, 0x2c, 0x1a, 0x1b, 0x6e, 0x5a, 0xa0, 0x52, 0x3b, 0xd6, 0xb3, 0x29, 0xe3, 0x2f, 0x84,
    0x53, 0xd1, 0x00, 0xed, 0x20, 0xfc, 0xb1, 0x5b, 0x6a, 0xcb, 0xbe, 0x39, 0x4a, 0x4c, 0x58, 0xcf,
    0xd0, 0xef, 0xaa, 0xfb, 0x43, 0x4d, 0x33, 0x85, 0x45, 0xf9, 0x02, 0x7f, 0x50, 0x3c, 0x9f, 0xa8,
    0x51, 0xa3, 0x40, 0x8f, 0x92, 0x9d, 0x38, 0xf5, 0xbc, 0xb6, 0xda, 0x21, 0x10, 0xff, 0xf3, 0xd2,
    0xcd, 0x0c, 0x13, 0xec, 0x5f, 0x97, 0x44, 0x17, 0xc4, 0xa7, 0x7e, 0x3d, 0x64, 0x5d, 0x19, 0x73,
    0x60, 0x81, 0x4f, 0xdc, 0x22, 0x2a, 0x90, 0x88, 0x46, 0xee, 0xb8, 0x14, 0xde, 0x5e, 0x0b, 0xdb,
    0xe0, 0x32, 0x3a, 0x0a, 0x49, 0x06, 0x24, 0x5c, 0xc2, 0xd3, 0xac, 0x62, 0x91, 0x95, 0xe4, 0x79,
    0xe7, 0xc8, 0x37, 0x6d, 0x8d, 0xd5, 0x4e, 0xa9, 0x6c, 0x56, 0xf4, 0xea, 0x65, 0x7a, 0xae, 0x08,
    0xba, 0x78, 0x25, 0x2e, 0x1c, 0xa6, 0xb4, 0xc6, 0xe8, 0xdd, 0x74, 0x1f, 0x4b, 0xbd, 0x8b, 0x8a,
    0x70, 0x3e, 0xb5, 0x66, 0x48, 0x03, 0xf6, 0x0e, 0x61, 0x35, 0x57, 0xb9, 0x86, 0xc1, 0x1d, 0x9e,
    0xe1, 0xf8, 0x98, 0x11, 0x69, 0xd9, 0x8e, 0x94, 0x9b, 0x1e, 0x87, 0xe9, 0xce, 0x55, 0x28, 0xdf,
    0x8c, 0xa1, 0x89, 0x0d, 0xbf, 0xe6, 0x42, 0x68, 0x41, 0x99, 0x2d, 0x0f, 0xb0, 0x54, 0xbb, 0x16,
];

static RCON: [u8; 11] = [0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36];

static INV_SBOX: [u8; 256] = [
    0x52, 0x09, 0x6a, 0xd5, 0x30, 0x36, 0xa5, 0x38, 0xbf, 0x40, 0xa3, 0x9e, 0x81, 0xf3, 0xd7, 0xfb,
    0x7c, 0xe3, 0x39, 0x82, 0x9b, 0x2f, 0xff, 0x87, 0x34, 0x8e, 0x43, 0x44, 0xc4, 0xde, 0xe9, 0xcb,
    0x54, 0x7b, 0x94, 0x32, 0xa6, 0xc2, 0x23, 0x3d, 0xee, 0x4c, 0x95, 0x0b, 0x42, 0xfa, 0xc3, 0x4e,
    0x08, 0x2e, 0xa1, 0x66, 0x28, 0xd9, 0x24, 0xb2, 0x76, 0x5b, 0xa2, 0x49, 0x6d, 0x8b, 0xd1, 0x25,
    0x72, 0xf8, 0xf6, 0x64, 0x86, 0x68, 0x98, 0x16, 0xd4, 0xa4, 0x5c, 0xcc, 0x5d, 0x65, 0xb6, 0x92,
    0x6c, 0x70, 0x48, 0x50, 0xfd, 0xed, 0xb9, 0xda, 0x5e, 0x15, 0x46, 0x57, 0xa7, 0x8d, 0x9d, 0x84,
    0x90, 0xd8, 0xab, 0x00, 0x8c, 0xbc, 0xd3, 0x0a, 0xf7, 0xe4, 0x58, 0x05, 0xb8, 0xb3, 0x45, 0x06,
    0xd0, 0x2c, 0x1e, 0x8f, 0xca, 0x3f, 0x0f, 0x02, 0xc1, 0xaf, 0xbd, 0x03, 0x01, 0x13, 0x8a, 0x6b,
    0x3a, 0x91, 0x11, 0x41, 0x4f, 0x67, 0xdc, 0xea, 0x97, 0xf2, 0xcf, 0xce, 0xf0, 0xb4, 0xe6, 0x73,
    0x96, 0xac, 0x74, 0x22, 0xe7, 0xad, 0x35, 0x85, 0xe2, 0xf9, 0x37, 0xe8, 0x1c, 0x75, 0xdf, 0x6e,
    0x47, 0xf1, 0x1a, 0x71, 0x1d, 0x29, 0xc5, 0x89, 0x6f, 0xb7, 0x62, 0x0e, 0xaa, 0x18, 0xbe, 0x1b,
    0xfc, 0x56, 0x3e, 0x4b, 0xc6, 0xd2, 0x79, 0x20, 0x9a, 0xdb, 0xc0, 0xfe, 0x78, 0xcd, 0x5a, 0xf4,
    0x1f, 0xdd, 0xa8, 0x33, 0x88, 0x07, 0xc7, 0x31, 0xb1, 0x12, 0x10, 0x59, 0x27, 0x80, 0xec, 0x5f,
    0x60, 0x51, 0x7f, 0xa9, 0x19, 0xb5, 0x4a, 0x0d, 0x2d, 0xe5, 0x7a, 0x9f, 0x93, 0xc9, 0x9c, 0xef,
    0xa0, 0xe0, 0x3b, 0x4d, 0xae, 0x2a, 0xf5, 0xb0, 0xc8, 0xeb, 0xbb, 0x3c, 0x83, 0x53, 0x99, 0x61,
    0x17, 0x2b, 0x04, 0x7e, 0xba, 0x77, 0xd6, 0x26, 0xe1, 0x69, 0x14, 0x63, 0x55, 0x21, 0x0c, 0x7d,
];

#[inline]
fn sub_word(w: u32) -> u32 {
    ((SBOX[((w >> 0) & 0xff) as usize] as u32) << 0)
        | ((SBOX[((w >> 8) & 0xff) as usize] as u32) << 8)
        | ((SBOX[((w >> 16) & 0xff) as usize] as u32) << 16)
        | ((SBOX[((w >> 24) & 0xff) as usize] as u32) << 24)
}

#[inline]
fn rot_word(w: u32) -> u32 {
    (w >> 8) | (w << 24)
}

#[inline]
fn xtime(a: u8) -> u8 {
    (((a as u32) << 1) ^ (((a as u32 >> 7) & 1) * 0x1b)) as u8
}

#[inline]
fn gf_mul_09(a: u8) -> u8 {
    xtime(xtime(xtime(a))) ^ a
}
#[inline]
fn gf_mul_0b(a: u8) -> u8 {
    xtime(xtime(xtime(a)) ^ a) ^ a
}
#[inline]
fn gf_mul_0d(a: u8) -> u8 {
    xtime(xtime(xtime(a) ^ a)) ^ a
}
#[inline]
fn gf_mul_0e(a: u8) -> u8 {
    xtime(xtime(xtime(a) ^ a) ^ a)
}

fn inv_mix_column(col: u32) -> u32 {
    let b0 = col as u8;
    let b1 = (col >> 8) as u8;
    let b2 = (col >> 16) as u8;
    let b3 = (col >> 24) as u8;

    let r0 = gf_mul_0e(b0) ^ gf_mul_0b(b1) ^ gf_mul_0d(b2) ^ gf_mul_09(b3);
    let r1 = gf_mul_09(b0) ^ gf_mul_0e(b1) ^ gf_mul_0b(b2) ^ gf_mul_0d(b3);
    let r2 = gf_mul_0d(b0) ^ gf_mul_09(b1) ^ gf_mul_0e(b2) ^ gf_mul_0b(b3);
    let r3 = gf_mul_0b(b0) ^ gf_mul_0d(b1) ^ gf_mul_09(b2) ^ gf_mul_0e(b3);

    (r0 as u32) | ((r1 as u32) << 8) | ((r2 as u32) << 16) | ((r3 as u32) << 24)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_softaes_expand_key128(rkeys: *mut SoftAesBlock, key: *const u8) {
    let key = core::slice::from_raw_parts(key, 16);
    let mut w = [0u32; 44];

    w[0] = (key[0] as u32) | ((key[1] as u32) << 8) | ((key[2] as u32) << 16) | ((key[3] as u32) << 24);
    w[1] = (key[4] as u32) | ((key[5] as u32) << 8) | ((key[6] as u32) << 16) | ((key[7] as u32) << 24);
    w[2] = (key[8] as u32) | ((key[9] as u32) << 8) | ((key[10] as u32) << 16) | ((key[11] as u32) << 24);
    w[3] = (key[12] as u32) | ((key[13] as u32) << 8) | ((key[14] as u32) << 16) | ((key[15] as u32) << 24);

    for i in 4..44 {
        let mut temp = w[i - 1];
        if i % 4 == 0 {
            temp = sub_word(rot_word(temp)) ^ (RCON[i / 4] as u32);
        }
        w[i] = w[i - 4] ^ temp;
    }

    let rkeys = core::slice::from_raw_parts_mut(rkeys, 11);
    for i in 0..11 {
        rkeys[i].w0 = w[i * 4 + 0];
        rkeys[i].w1 = w[i * 4 + 1];
        rkeys[i].w2 = w[i * 4 + 2];
        rkeys[i].w3 = w[i * 4 + 3];
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_softaes_expand_key256(rkeys: *mut SoftAesBlock, key: *const u8) {
    let key = core::slice::from_raw_parts(key, 32);
    let mut w = [0u32; 60];

    for k in 0..8 {
        w[k] = (key[4 * k] as u32)
            | ((key[4 * k + 1] as u32) << 8)
            | ((key[4 * k + 2] as u32) << 16)
            | ((key[4 * k + 3] as u32) << 24);
    }

    for i in 8..60 {
        let mut temp = w[i - 1];
        if i % 8 == 0 {
            temp = sub_word(rot_word(temp)) ^ (RCON[i / 8] as u32);
        } else if i % 8 == 4 {
            temp = sub_word(temp);
        }
        w[i] = w[i - 8] ^ temp;
    }

    let rkeys = core::slice::from_raw_parts_mut(rkeys, 15);
    for i in 0..15 {
        rkeys[i].w0 = w[i * 4 + 0];
        rkeys[i].w1 = w[i * 4 + 1];
        rkeys[i].w2 = w[i * 4 + 2];
        rkeys[i].w3 = w[i * 4 + 3];
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _sodium_softaes_inv_mix_columns(block: SoftAesBlock) -> SoftAesBlock {
    SoftAesBlock {
        w0: inv_mix_column(block.w0),
        w1: inv_mix_column(block.w1),
        w2: inv_mix_column(block.w2),
        w3: inv_mix_column(block.w3),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_softaes_invert_key_schedule128(rkeys: *mut SoftAesBlock) {
    let rkeys = core::slice::from_raw_parts_mut(rkeys, 11);
    for i in 1..10 {
        rkeys[i] = _sodium_softaes_inv_mix_columns(rkeys[i]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_softaes_invert_key_schedule256(rkeys: *mut SoftAesBlock) {
    let rkeys = core::slice::from_raw_parts_mut(rkeys, 15);
    for i in 1..14 {
        rkeys[i] = _sodium_softaes_inv_mix_columns(rkeys[i]);
    }
}

// --- SRM-1R bitsliced round (constant-time reference path) ---

#[inline]
fn srm1r_dup16(x: u32) -> u32 {
    let x = x & 0xffff;
    x | (x << 16)
}

#[inline]
fn srm1r_ror16(x: u32, n: u32) -> u32 {
    (x >> n) | (x << (32 - n))
}

#[inline]
fn srm1r_ror32(x: u32, n: u32) -> u32 {
    (x >> n) | (x << (32 - n))
}

#[inline]
fn srm1r_load_row_words(block: &SoftAesBlock, shift: u32) -> u32 {
    ((block.w0 >> shift) & 0xffu32)
        | (((block.w1 >> shift) & 0xffu32) << 8)
        | (((block.w2 >> shift) & 0xffu32) << 16)
        | (((block.w3 >> shift) & 0xffu32) << 24)
}

#[inline]
fn srm1r_store_column_word(row0: u32, row1: u32, row2: u32, row3: u32, shift: u32) -> u32 {
    ((row0 >> shift) & 0xffu32)
        | (((row1 >> shift) & 0xffu32) << 8)
        | (((row2 >> shift) & 0xffu32) << 16)
        | (((row3 >> shift) & 0xffu32) << 24)
}

#[inline]
fn srm1r_gather_row_bit(row_word: u32, bit: u32) -> u32 {
    (((row_word >> bit) & 0x01010101u32).wrapping_mul(0x01020408u32)) >> 24
}

#[inline]
fn srm1r_pack_rows_bit(row0: u32, row1: u32, row2: u32, row3: u32, bit: u32) -> u32 {
    srm1r_dup16(
        srm1r_gather_row_bit(row0, bit)
            | (srm1r_gather_row_bit(row1, bit) << 4)
            | (srm1r_gather_row_bit(row2, bit) << 8)
            | (srm1r_gather_row_bit(row3, bit) << 12),
    )
}

#[inline]
fn srm1r_spread_row_bits(nibble: u32, bit: u32) -> u32 {
    let nibble = nibble & 0x0fu32;
    ((nibble.wrapping_mul(0x00204081u32)) & 0x01010101u32) << bit
}

#[inline]
fn srm1r_unpack_row_word(planes: &[u32; 8], row: u32) -> u32 {
    let lane_shift = 4u32 * row;
    srm1r_spread_row_bits(planes[0] >> lane_shift, 7)
        | srm1r_spread_row_bits(planes[1] >> lane_shift, 6)
        | srm1r_spread_row_bits(planes[2] >> lane_shift, 5)
        | srm1r_spread_row_bits(planes[3] >> lane_shift, 4)
        | srm1r_spread_row_bits(planes[4] >> lane_shift, 3)
        | srm1r_spread_row_bits(planes[5] >> lane_shift, 2)
        | srm1r_spread_row_bits(planes[6] >> lane_shift, 1)
        | srm1r_spread_row_bits(planes[7] >> lane_shift, 0)
}

#[inline]
fn srm1r_pack_planes(planes: &mut [u32; 8], row0: u32, row1: u32, row2: u32, row3: u32) {
    planes[0] = srm1r_pack_rows_bit(row0, row1, row2, row3, 7);
    planes[1] = srm1r_pack_rows_bit(row0, row1, row2, row3, 6);
    planes[2] = srm1r_pack_rows_bit(row0, row1, row2, row3, 5);
    planes[3] = srm1r_pack_rows_bit(row0, row1, row2, row3, 4);
    planes[4] = srm1r_pack_rows_bit(row0, row1, row2, row3, 3);
    planes[5] = srm1r_pack_rows_bit(row0, row1, row2, row3, 2);
    planes[6] = srm1r_pack_rows_bit(row0, row1, row2, row3, 1);
    planes[7] = srm1r_pack_rows_bit(row0, row1, row2, row3, 0);
}

fn srm1r_subbytes(planes: &mut [u32; 8]) {
    let s0 = planes[1] ^ planes[4];
    let s1 = planes[5] ^ planes[7];
    let s2 = planes[3] ^ s0;
    let s3 = planes[0] ^ planes[2];
    let q0 = s1 ^ s2;
    let s4 = planes[0] ^ planes[6];
    let s5 = planes[2] ^ planes[6];
    let s6 = planes[3] ^ s1;
    let s7 = planes[5] ^ s3;
    let q1 = s1 ^ s5;
    let q2 = planes[2] ^ q0;
    let q3 = s4 ^ s2;
    let q4 = s3 ^ q0;
    let s8 = planes[4] ^ s3;
    let q5 = s6 ^ s8;
    let q6 = planes[2] ^ planes[3];
    let q7 = planes[6] ^ s2;
    let s9 = planes[6] ^ s0;
    let q8 = s3 ^ s9;
    let q9 = s4 ^ s6;
    let q10 = s0 ^ s5;
    let q12 = planes[7] ^ s2;
    let q13 = planes[1] ^ s7;
    let q14 = planes[7] ^ s3;
    let q15 = s2 ^ s7;
    let q16 = planes[1] ^ s1;
    let q17 = planes[1] ^ planes[7];
    let q11 = planes[5];

    let t20 = q6 & q12;
    let t21 = q3 & q14;
    let t22 = q1 & q16;
    let t23 = q2 & q17;
    let x0 = ((q3 | q14) ^ (q0 & q7)) ^ (t20 ^ t22);
    let x1 = ((q4 | q13) ^ (q10 & q11)) ^ (t21 ^ t20);
    let x2 = ((q2 | q17) ^ (q5 & q9)) ^ (t21 ^ t22);
    let x3 = ((q8 | q15) ^ t23) ^ (t21 ^ (q4 & q13));

    let a = x1 & !x3;
    let b = x0 & !x3;
    let c = x3 & !x1;
    let d = x2 & !x1;
    let e = x0 ^ a;
    let y0 = x3 ^ (x2 & !e);
    let f = x1 ^ b;
    let y1 = c ^ (x2 & f);
    let g = x2 ^ c;
    let y2 = x1 ^ (x0 & !g);
    let h = x3 ^ d;
    let y3 = a ^ (x0 & h);
    let y02 = y2 ^ y0;
    let y13 = y3 ^ y1;
    let y23 = y3 ^ y2;
    let y01 = y1 ^ y0;
    let y00 = y02 ^ y13;

    let a0 = y01 & q11;
    let a1 = y0 & q12;
    let a2 = y1 & q0;
    let a3 = y23 & q17;
    let a4 = y2 & q5;
    let a5 = y3 & q15;
    let a6 = y13 & q14;
    let a7 = y00 & q16;
    let a8 = y02 & q13;
    let a9 = y01 & q7;
    let a10 = y0 & q10;
    let a11 = y1 & q6;
    let a12 = y23 & q2;
    let a13 = y2 & q9;
    let a14 = y3 & q8;
    let a15 = y13 & q3;
    let a16 = y00 & q1;
    let a17 = y02 & q4;

    let r0 = a1 ^ a5;
    let r1 = a9 ^ a15;
    let r2 = a4 ^ r0;
    let r3 = a2 ^ a10;
    let r4 = a11 ^ a17;
    let r5 = a8 ^ r1;
    let r6 = a0 ^ a16;
    let r7 = a7 ^ a13;
    let r8 = a11 ^ a14;
    let r9 = r3 ^ r4;
    let r10 = r5 ^ r6;
    let r11 = r2 ^ r9;
    let r12 = a3 ^ r0;
    let r13 = r7 ^ r8;
    let r14 = r12 ^ r13;
    planes[0] = r10 ^ r14;
    let r15 = a6 ^ a10;
    let r16 = r15 ^ r2;
    planes[1] = !(r10 ^ r16);
    planes[2] = !(a2 ^ r2);
    let r17 = a12 ^ a13;
    let r18 = a15 ^ r17;
    planes[3] = r18 ^ r11;
    let r19 = a1 ^ a14;
    let r20 = a17 ^ r3;
    let r21 = r7 ^ r19;
    let r22 = r5 ^ r20;
    planes[4] = r21 ^ r22;
    let r23 = a9 ^ a12;
    planes[5] = r8 ^ r23;
    planes[6] = !(r1 ^ r4);
    planes[7] = !(a16 ^ r11);
}

fn srm1r_mix_columns(planes: &mut [u32; 8]) {
    let adj0 = srm1r_ror16(planes[0], 4);
    let adj1 = srm1r_ror16(planes[1], 4);
    let adj2 = srm1r_ror16(planes[2], 4);
    let adj3 = srm1r_ror16(planes[3], 4);
    let adj4 = srm1r_ror16(planes[4], 4);
    let adj5 = srm1r_ror16(planes[5], 4);
    let adj6 = srm1r_ror16(planes[6], 4);
    let adj7 = srm1r_ror16(planes[7], 4);
    let pair0 = planes[0] ^ adj0;
    let pair1 = planes[1] ^ adj1;
    let pair2 = planes[2] ^ adj2;
    let pair3 = planes[3] ^ adj3;
    let pair4 = planes[4] ^ adj4;
    let pair5 = planes[5] ^ adj5;
    let pair6 = planes[6] ^ adj6;
    let pair7 = planes[7] ^ adj7;
    let opp0 = srm1r_ror16(pair0, 8);
    let opp1 = srm1r_ror16(pair1, 8);
    let opp2 = srm1r_ror16(pair2, 8);
    let opp3 = srm1r_ror16(pair3, 8);
    let opp4 = srm1r_ror16(pair4, 8);
    let opp5 = srm1r_ror16(pair5, 8);
    let opp6 = srm1r_ror16(pair6, 8);
    let opp7 = srm1r_ror16(pair7, 8);

    planes[0] = pair1 ^ adj0 ^ opp0;
    planes[1] = pair2 ^ adj1 ^ opp1;
    planes[2] = pair3 ^ adj2 ^ opp2;
    planes[3] = pair4 ^ adj3 ^ opp3 ^ pair0;
    planes[4] = pair5 ^ adj4 ^ opp4 ^ pair0;
    planes[5] = pair6 ^ adj5 ^ opp5;
    planes[6] = pair7 ^ adj6 ^ opp6 ^ pair0;
    planes[7] = pair0 ^ adj7 ^ opp7;
}

#[unsafe(no_mangle)]
pub extern "C" fn _sodium_softaes_block_encrypt(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock {
    let mut planes = [0u32; 8];

    let row0 = srm1r_load_row_words(&block, 0);
    let row1 = srm1r_ror32(srm1r_load_row_words(&block, 8), 8);
    let row2 = srm1r_ror32(srm1r_load_row_words(&block, 16), 16);
    let row3 = srm1r_ror32(srm1r_load_row_words(&block, 24), 24);
    srm1r_pack_planes(&mut planes, row0, row1, row2, row3);

    srm1r_subbytes(&mut planes);
    srm1r_mix_columns(&mut planes);

    let row0 = srm1r_unpack_row_word(&planes, 0);
    let row1 = srm1r_unpack_row_word(&planes, 1);
    let row2 = srm1r_unpack_row_word(&planes, 2);
    let row3 = srm1r_unpack_row_word(&planes, 3);
    SoftAesBlock {
        w0: srm1r_store_column_word(row0, row1, row2, row3, 0) ^ rk.w0,
        w1: srm1r_store_column_word(row0, row1, row2, row3, 8) ^ rk.w1,
        w2: srm1r_store_column_word(row0, row1, row2, row3, 16) ^ rk.w2,
        w3: srm1r_store_column_word(row0, row1, row2, row3, 24) ^ rk.w3,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _sodium_softaes_block_decrypt(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock {
    let (s0, s1, s2, s3) = (block.w0, block.w1, block.w2, block.w3);

    let t0 = (s0 & 0x000000ff) | (s3 & 0x0000ff00) | (s2 & 0x00ff0000) | (s1 & 0xff000000);
    let t1 = (s1 & 0x000000ff) | (s0 & 0x0000ff00) | (s3 & 0x00ff0000) | (s2 & 0xff000000);
    let t2 = (s2 & 0x000000ff) | (s1 & 0x0000ff00) | (s0 & 0x00ff0000) | (s3 & 0xff000000);
    let t3 = (s3 & 0x000000ff) | (s2 & 0x0000ff00) | (s1 & 0x00ff0000) | (s0 & 0xff000000);

    let s0 = (INV_SBOX[(t0 & 0xff) as usize] as u32)
        | ((INV_SBOX[((t0 >> 8) & 0xff) as usize] as u32) << 8)
        | ((INV_SBOX[((t0 >> 16) & 0xff) as usize] as u32) << 16)
        | ((INV_SBOX[((t0 >> 24) & 0xff) as usize] as u32) << 24);
    let s1 = (INV_SBOX[(t1 & 0xff) as usize] as u32)
        | ((INV_SBOX[((t1 >> 8) & 0xff) as usize] as u32) << 8)
        | ((INV_SBOX[((t1 >> 16) & 0xff) as usize] as u32) << 16)
        | ((INV_SBOX[((t1 >> 24) & 0xff) as usize] as u32) << 24);
    let s2 = (INV_SBOX[(t2 & 0xff) as usize] as u32)
        | ((INV_SBOX[((t2 >> 8) & 0xff) as usize] as u32) << 8)
        | ((INV_SBOX[((t2 >> 16) & 0xff) as usize] as u32) << 16)
        | ((INV_SBOX[((t2 >> 24) & 0xff) as usize] as u32) << 24);
    let s3 = (INV_SBOX[(t3 & 0xff) as usize] as u32)
        | ((INV_SBOX[((t3 >> 8) & 0xff) as usize] as u32) << 8)
        | ((INV_SBOX[((t3 >> 16) & 0xff) as usize] as u32) << 16)
        | ((INV_SBOX[((t3 >> 24) & 0xff) as usize] as u32) << 24);

    SoftAesBlock {
        w0: inv_mix_column(s0) ^ rk.w0,
        w1: inv_mix_column(s1) ^ rk.w1,
        w2: inv_mix_column(s2) ^ rk.w2,
        w3: inv_mix_column(s3) ^ rk.w3,
    }
}

const SOFTAES_STRIDE: usize = 16;

#[unsafe(no_mangle)]
pub extern "C" fn _sodium_softaes_block_encryptlast(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock {
    let (s0, s1, s2, s3) = (block.w0, block.w1, block.w2, block.w3);
    let mut out = SoftAesBlock { w0: 0, w1: 0, w2: 0, w3: 0 };

    let mut ix = [[0u8; 4]; 4];
    ix[0][0] = s0 as u8;
    ix[0][1] = (s1 >> 8) as u8;
    ix[0][2] = (s2 >> 16) as u8;
    ix[0][3] = (s3 >> 24) as u8;

    ix[1][0] = s1 as u8;
    ix[1][1] = (s2 >> 8) as u8;
    ix[1][2] = (s3 >> 16) as u8;
    ix[1][3] = (s0 >> 24) as u8;

    ix[2][0] = s2 as u8;
    ix[2][1] = (s3 >> 8) as u8;
    ix[2][2] = (s0 >> 16) as u8;
    ix[2][3] = (s1 >> 24) as u8;

    ix[3][0] = s3 as u8;
    ix[3][1] = (s0 >> 8) as u8;
    ix[3][2] = (s1 >> 16) as u8;
    ix[3][3] = (s2 >> 24) as u8;

    let mut t = [[0u8; 256 / SOFTAES_STRIDE]; 4];

    let words = [&mut out.w0, &mut out.w1, &mut out.w2, &mut out.w3];
    for (row, w) in words.into_iter().enumerate() {
        for i in 0..(256 / SOFTAES_STRIDE) {
            for j in 0..4 {
                t[j][i] = SBOX[(i * SOFTAES_STRIDE) | (ix[row][j] as usize % SOFTAES_STRIDE)];
            }
        }
        *w = ((t[0][ix[row][0] as usize / SOFTAES_STRIDE] as u32) << 0)
            | ((t[1][ix[row][1] as usize / SOFTAES_STRIDE] as u32) << 8)
            | ((t[2][ix[row][2] as usize / SOFTAES_STRIDE] as u32) << 16)
            | ((t[3][ix[row][3] as usize / SOFTAES_STRIDE] as u32) << 24);
    }

    out.w0 ^= rk.w0;
    out.w1 ^= rk.w1;
    out.w2 ^= rk.w2;
    out.w3 ^= rk.w3;

    out
}

#[unsafe(no_mangle)]
pub extern "C" fn _sodium_softaes_block_decryptlast(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock {
    let (s0, s1, s2, s3) = (block.w0, block.w1, block.w2, block.w3);

    let t0 = (s0 & 0x000000ff) | (s3 & 0x0000ff00) | (s2 & 0x00ff0000) | (s1 & 0xff000000);
    let t1 = (s1 & 0x000000ff) | (s0 & 0x0000ff00) | (s3 & 0x00ff0000) | (s2 & 0xff000000);
    let t2 = (s2 & 0x000000ff) | (s1 & 0x0000ff00) | (s0 & 0x00ff0000) | (s3 & 0xff000000);
    let t3 = (s3 & 0x000000ff) | (s2 & 0x0000ff00) | (s1 & 0x00ff0000) | (s0 & 0xff000000);

    let w0 = ((INV_SBOX[(t0 & 0xff) as usize] as u32)
        | ((INV_SBOX[((t0 >> 8) & 0xff) as usize] as u32) << 8)
        | ((INV_SBOX[((t0 >> 16) & 0xff) as usize] as u32) << 16)
        | ((INV_SBOX[((t0 >> 24) & 0xff) as usize] as u32) << 24))
        ^ rk.w0;
    let w1 = ((INV_SBOX[(t1 & 0xff) as usize] as u32)
        | ((INV_SBOX[((t1 >> 8) & 0xff) as usize] as u32) << 8)
        | ((INV_SBOX[((t1 >> 16) & 0xff) as usize] as u32) << 16)
        | ((INV_SBOX[((t1 >> 24) & 0xff) as usize] as u32) << 24))
        ^ rk.w1;
    let w2 = ((INV_SBOX[(t2 & 0xff) as usize] as u32)
        | ((INV_SBOX[((t2 >> 8) & 0xff) as usize] as u32) << 8)
        | ((INV_SBOX[((t2 >> 16) & 0xff) as usize] as u32) << 16)
        | ((INV_SBOX[((t2 >> 24) & 0xff) as usize] as u32) << 24))
        ^ rk.w2;
    let w3 = ((INV_SBOX[(t3 & 0xff) as usize] as u32)
        | ((INV_SBOX[((t3 >> 8) & 0xff) as usize] as u32) << 8)
        | ((INV_SBOX[((t3 >> 16) & 0xff) as usize] as u32) << 16)
        | ((INV_SBOX[((t3 >> 24) & 0xff) as usize] as u32) << 24))
        ^ rk.w3;

    SoftAesBlock { w0, w1, w2, w3 }
}

// Helper block operations used by the ipcrypt module (from softaes.h inlines).
#[inline]
pub fn block_load(inp: &[u8]) -> SoftAesBlock {
    SoftAesBlock {
        w0: crate::common::load32_le(&inp[0..]),
        w1: crate::common::load32_le(&inp[4..]),
        w2: crate::common::load32_le(&inp[8..]),
        w3: crate::common::load32_le(&inp[12..]),
    }
}

#[inline]
pub fn block_store(out: &mut [u8], inb: SoftAesBlock) {
    crate::common::store32_le(&mut out[0..], inb.w0);
    crate::common::store32_le(&mut out[4..], inb.w1);
    crate::common::store32_le(&mut out[8..], inb.w2);
    crate::common::store32_le(&mut out[12..], inb.w3);
}

#[inline]
pub fn block_xor(a: SoftAesBlock, b: SoftAesBlock) -> SoftAesBlock {
    SoftAesBlock {
        w0: a.w0 ^ b.w0,
        w1: a.w1 ^ b.w1,
        w2: a.w2 ^ b.w2,
        w3: a.w3 ^ b.w3,
    }
}
