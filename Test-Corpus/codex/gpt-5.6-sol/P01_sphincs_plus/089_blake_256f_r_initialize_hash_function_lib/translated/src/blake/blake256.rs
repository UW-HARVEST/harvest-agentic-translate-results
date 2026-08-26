//! Translation of `lib/blake/src/blake256.c` — the BLAKE-256 reference
//! implementation (supercop-20140525/crypto_hash/blake256/sandy) together with
//! the MGF1 construction built on top of it.
//!
//! ```text
//! BLAKE reference C implementation
//!
//! Copyright (c) 2012 Jean-Philippe Aumasson <jeanphilippe.aumasson@gmail.com>
//!
//! To the extent possible under law, the author(s) have dedicated all copyright
//! and related and neighboring rights to this software to the public domain
//! worldwide. This software is distributed without any warranty.
//!
//! You should have received a copy of the CC0 Public Domain Dedication along
//! with this software. If not, see
//! <http://creativecommons.org/publicdomain/zero/1.0/>.
//! ```
//!
//! This is the fully unrolled variant of the reference code: it has no `sigma`
//! permutation table — the message-word/constant permutation of every one of
//! the 14 rounds is spelled out at the `ROUND(...)` call sites (C lines
//! 241-254) and is transcribed here verbatim.  Every constant, index and
//! rotation amount is taken over unchanged so that the behaviour is
//! byte-identical to the C code.

use crate::utils::SPX_u32_to_bytes;

/// `#define SPX_BLAKE256_OUTPUT_BYTES 32` (from `lib/blake/include/blake.h`);
/// this does not necessarily equal `SPX_N`.
pub const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;

/// ```c
/// typedef struct
/// {
///   unsigned int h[8], s[4], t[2];
///   int buflen, nullt;
///   unsigned char buf[64];
/// } blakestate256;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct blakestate256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

impl blakestate256 {
    /// Zeroed state; `blake256_init` overwrites every field it cares about,
    /// exactly like the uninitialised `blakestate256 S;` of the C code.
    pub const fn new() -> Self {
        blakestate256 {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 64],
        }
    }
}

impl Default for blakestate256 {
    fn default() -> Self {
        blakestate256::new()
    }
}

/// ```c
/// static const u32 cst[16]
/// ```
static cst: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344, 0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C, 0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

/// ```c
/// static const u8 padding[] =
///   {0x80,0,0,0,...,0};
/// ```
static padding: [u8; 64] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0,
];

/// ```c
/// #define U8TO32(p)                                             \
///   (((uint32_t)((p)[0]) << 24) | ((uint32_t)((p)[1]) << 16) |  \
///    ((uint32_t)((p)[2]) <<  8) | ((uint32_t)((p)[3])      ))
/// ```
#[inline(always)]
unsafe fn U8TO32(p: *const u8) -> u32 {
    ((*p.add(0) as u32) << 24)
        | ((*p.add(1) as u32) << 16)
        | ((*p.add(2) as u32) << 8)
        | (*p.add(3) as u32)
}

/// ```c
/// #define U32TO8(p, v)             \
///   (p)[0] = (uint8_t)((v) >> 24); \
///   (p)[1] = (uint8_t)((v) >> 16); \
///   (p)[2] = (uint8_t)((v) >> 8);  \
///   (p)[3] = (uint8_t)((v));
/// ```
#[inline(always)]
unsafe fn U32TO8(p: *mut u8, v: u32) {
    *p.add(0) = (v >> 24) as u8;
    *p.add(1) = (v >> 16) as u8;
    *p.add(2) = (v >> 8) as u8;
    *p.add(3) = v as u8;
}

/// ```c
/// #define BLAKE256_ROT(x,n) (((x)<<(32-n))|( (x)>>(n)))
/// ```
#[inline(always)]
fn BLAKE256_ROT(x: u32, n: u32) -> u32 {
    x.rotate_right(n)
}

/// The `ROUND(...)` macro of `blake256.c` (C lines 127-239).
///
/// `macro_rules!` cannot touch the caller's locals, so the working state is
/// passed in as a 16-word array (`v[i]` == the C `vi`); the 32 message-word /
/// constant arguments keep the exact names and order of the C macro, which
/// lets the 14 call sites below be copied over verbatim.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn ROUND(
    v: &mut [u32; 16],
    m0: u32,
    c0: u32,
    m1: u32,
    c1: u32,
    m2: u32,
    c2: u32,
    m3: u32,
    c3: u32,
    m4: u32,
    c4: u32,
    m5: u32,
    c5: u32,
    m6: u32,
    c6: u32,
    m7: u32,
    c7: u32,
    m8: u32,
    c8: u32,
    m9: u32,
    c9: u32,
    m10: u32,
    c10: u32,
    m11: u32,
    c11: u32,
    m12: u32,
    c12: u32,
    m13: u32,
    c13: u32,
    m14: u32,
    c14: u32,
    m15: u32,
    c15: u32,
) {
    v[0] = v[0].wrapping_add(m0 ^ c0);
    v[0] = v[0].wrapping_add(v[4]);
    v[12] ^= v[0];
    v[12] = BLAKE256_ROT(v[12], 16);
    v[8] = v[8].wrapping_add(v[12]);
    v[4] ^= v[8];
    v[4] = BLAKE256_ROT(v[4], 12);

    v[1] = v[1].wrapping_add(m2 ^ c2);
    v[1] = v[1].wrapping_add(v[5]);
    v[13] ^= v[1];
    v[13] = BLAKE256_ROT(v[13], 16);
    v[9] = v[9].wrapping_add(v[13]);
    v[5] ^= v[9];
    v[5] = BLAKE256_ROT(v[5], 12);

    v[2] = v[2].wrapping_add(m4 ^ c4);
    v[2] = v[2].wrapping_add(v[6]);
    v[14] ^= v[2];
    v[14] = BLAKE256_ROT(v[14], 16);
    v[10] = v[10].wrapping_add(v[14]);
    v[6] ^= v[10];
    v[6] = BLAKE256_ROT(v[6], 12);

    v[3] = v[3].wrapping_add(m6 ^ c6);
    v[3] = v[3].wrapping_add(v[7]);
    v[15] ^= v[3];
    v[15] = BLAKE256_ROT(v[15], 16);
    v[11] = v[11].wrapping_add(v[15]);
    v[7] ^= v[11];
    v[7] = BLAKE256_ROT(v[7], 12);

    v[2] = v[2].wrapping_add(m5 ^ c5);
    v[2] = v[2].wrapping_add(v[6]);
    v[14] ^= v[2];
    v[14] = BLAKE256_ROT(v[14], 8);
    v[10] = v[10].wrapping_add(v[14]);
    v[6] ^= v[10];
    v[6] = BLAKE256_ROT(v[6], 7);

    v[3] = v[3].wrapping_add(m7 ^ c7);
    v[3] = v[3].wrapping_add(v[7]);
    v[15] ^= v[3];
    v[15] = BLAKE256_ROT(v[15], 8);
    v[11] = v[11].wrapping_add(v[15]);
    v[7] ^= v[11];
    v[7] = BLAKE256_ROT(v[7], 7);

    v[1] = v[1].wrapping_add(m3 ^ c3);
    v[1] = v[1].wrapping_add(v[5]);
    v[13] ^= v[1];
    v[13] = BLAKE256_ROT(v[13], 8);
    v[9] = v[9].wrapping_add(v[13]);
    v[5] ^= v[9];
    v[5] = BLAKE256_ROT(v[5], 7);

    v[0] = v[0].wrapping_add(m1 ^ c1);
    v[0] = v[0].wrapping_add(v[4]);
    v[12] ^= v[0];
    v[12] = BLAKE256_ROT(v[12], 8);
    v[8] = v[8].wrapping_add(v[12]);
    v[4] ^= v[8];
    v[4] = BLAKE256_ROT(v[4], 7);

    v[0] = v[0].wrapping_add(m8 ^ c8);
    v[0] = v[0].wrapping_add(v[5]);
    v[15] ^= v[0];
    v[15] = BLAKE256_ROT(v[15], 16);
    v[10] = v[10].wrapping_add(v[15]);
    v[5] ^= v[10];
    v[5] = BLAKE256_ROT(v[5], 12);

    v[1] = v[1].wrapping_add(m10 ^ c10);
    v[1] = v[1].wrapping_add(v[6]);
    v[12] ^= v[1];
    v[12] = BLAKE256_ROT(v[12], 16);
    v[11] = v[11].wrapping_add(v[12]);
    v[6] ^= v[11];
    v[6] = BLAKE256_ROT(v[6], 12);

    v[2] = v[2].wrapping_add(m12 ^ c12);
    v[2] = v[2].wrapping_add(v[7]);
    v[13] ^= v[2];
    v[13] = BLAKE256_ROT(v[13], 16);
    v[8] = v[8].wrapping_add(v[13]);
    v[7] ^= v[8];
    v[7] = BLAKE256_ROT(v[7], 12);

    v[3] = v[3].wrapping_add(m14 ^ c14);
    v[3] = v[3].wrapping_add(v[4]);
    v[14] ^= v[3];
    v[14] = BLAKE256_ROT(v[14], 16);
    v[9] = v[9].wrapping_add(v[14]);
    v[4] ^= v[9];
    v[4] = BLAKE256_ROT(v[4], 12);

    v[2] = v[2].wrapping_add(m13 ^ c13);
    v[2] = v[2].wrapping_add(v[7]);
    v[13] ^= v[2];
    v[13] = BLAKE256_ROT(v[13], 8);
    v[8] = v[8].wrapping_add(v[13]);
    v[7] ^= v[8];
    v[7] = BLAKE256_ROT(v[7], 7);

    v[3] = v[3].wrapping_add(m15 ^ c15);
    v[3] = v[3].wrapping_add(v[4]);
    v[14] ^= v[3];
    v[14] = BLAKE256_ROT(v[14], 8);
    v[9] = v[9].wrapping_add(v[14]);
    v[4] ^= v[9];
    v[4] = BLAKE256_ROT(v[4], 7);

    v[1] = v[1].wrapping_add(m11 ^ c11);
    v[1] = v[1].wrapping_add(v[6]);
    v[12] ^= v[1];
    v[12] = BLAKE256_ROT(v[12], 8);
    v[11] = v[11].wrapping_add(v[12]);
    v[6] ^= v[11];
    v[6] = BLAKE256_ROT(v[6], 7);

    v[0] = v[0].wrapping_add(m9 ^ c9);
    v[0] = v[0].wrapping_add(v[5]);
    v[15] ^= v[0];
    v[15] = BLAKE256_ROT(v[15], 8);
    v[10] = v[10].wrapping_add(v[15]);
    v[5] ^= v[10];
    v[5] = BLAKE256_ROT(v[5], 7);
}

/// ```c
/// void blake256_compress( blakestate256 *S, const unsigned char *block )
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_compress(S: *mut blakestate256, block: *const u8) {
    let m0: u32 = U8TO32(block.add(0));
    let m1: u32 = U8TO32(block.add(4));
    let m2: u32 = U8TO32(block.add(8));
    let m3: u32 = U8TO32(block.add(12));
    let m4: u32 = U8TO32(block.add(16));
    let m5: u32 = U8TO32(block.add(20));
    let m6: u32 = U8TO32(block.add(24));
    let m7: u32 = U8TO32(block.add(28));
    let m8: u32 = U8TO32(block.add(32));
    let m9: u32 = U8TO32(block.add(36));
    let m10: u32 = U8TO32(block.add(40));
    let m11: u32 = U8TO32(block.add(44));
    let m12: u32 = U8TO32(block.add(48));
    let m13: u32 = U8TO32(block.add(52));
    let m14: u32 = U8TO32(block.add(56));
    let m15: u32 = U8TO32(block.add(60));

    /* v[i] is the C `vi` */
    let v = &mut [0u32; 16];
    v[0] = (*S).h[0];
    v[1] = (*S).h[1];
    v[2] = (*S).h[2];
    v[3] = (*S).h[3];
    v[4] = (*S).h[4];
    v[5] = (*S).h[5];
    v[6] = (*S).h[6];
    v[7] = (*S).h[7];
    v[8] = (*S).s[0] ^ 0x243F6A88;
    v[9] = (*S).s[1] ^ 0x85A308D3;
    v[10] = (*S).s[2] ^ 0x13198A2E;
    v[11] = (*S).s[3] ^ 0x03707344;
    v[12] = 0xA4093822;
    v[13] = 0x299F31D0;
    v[14] = 0x082EFA98;
    v[15] = 0xEC4E6C89;
    if (*S).nullt == 0 {
        v[12] ^= (*S).t[0];
        v[13] ^= (*S).t[0];
        v[14] ^= (*S).t[1];
        v[15] ^= (*S).t[1];
    }

    #[rustfmt::skip]
    ROUND(v, m0,cst[1],m1,cst[0],m2,cst[3],m3,cst[2],m4,cst[5],m5,cst[4],m6,cst[7],m7,cst[6],m8,cst[9],m9,cst[8],m10,cst[11],m11,cst[10],m12,cst[13],m13,cst[12],m14,cst[15],m15,cst[14]);
    #[rustfmt::skip]
    ROUND(v, m14,cst[10],m10,cst[14],m4,cst[8],m8,cst[4],m9,cst[15],m15,cst[9],m13,cst[6],m6,cst[13],m1,cst[12],m12,cst[1],m0,cst[2],m2,cst[0],m11,cst[7],m7,cst[11],m5,cst[3],m3,cst[5]);
    #[rustfmt::skip]
    ROUND(v, m11,cst[8],m8,cst[11],m12,cst[0],m0,cst[12],m5,cst[2],m2,cst[5],m15,cst[13],m13,cst[15],m10,cst[14],m14,cst[10],m3,cst[6],m6,cst[3],m7,cst[1],m1,cst[7],m9,cst[4],m4,cst[9]);
    #[rustfmt::skip]
    ROUND(v, m7,cst[9],m9,cst[7],m3,cst[1],m1,cst[3],m13,cst[12],m12,cst[13],m11,cst[14],m14,cst[11],m2,cst[6],m6,cst[2],m5,cst[10],m10,cst[5],m4,cst[0],m0,cst[4],m15,cst[8],m8,cst[15]);
    #[rustfmt::skip]
    ROUND(v, m9,cst[0],m0,cst[9],m5,cst[7],m7,cst[5],m2,cst[4],m4,cst[2],m10,cst[15],m15,cst[10],m14,cst[1],m1,cst[14],m11,cst[12],m12,cst[11],m6,cst[8],m8,cst[6],m3,cst[13],m13,cst[3]);
    #[rustfmt::skip]
    ROUND(v, m2,cst[12],m12,cst[2],m6,cst[10],m10,cst[6],m0,cst[11],m11,cst[0],m8,cst[3],m3,cst[8],m4,cst[13],m13,cst[4],m7,cst[5],m5,cst[7],m15,cst[14],m14,cst[15],m1,cst[9],m9,cst[1]);
    #[rustfmt::skip]
    ROUND(v, m12,cst[5],m5,cst[12],m1,cst[15],m15,cst[1],m14,cst[13],m13,cst[14],m4,cst[10],m10,cst[4],m0,cst[7],m7,cst[0],m6,cst[3],m3,cst[6],m9,cst[2],m2,cst[9],m8,cst[11],m11,cst[8]);
    #[rustfmt::skip]
    ROUND(v, m13,cst[11],m11,cst[13],m7,cst[14],m14,cst[7],m12,cst[1],m1,cst[12],m3,cst[9],m9,cst[3],m5,cst[0],m0,cst[5],m15,cst[4],m4,cst[15],m8,cst[6],m6,cst[8],m2,cst[10],m10,cst[2]);
    #[rustfmt::skip]
    ROUND(v, m6,cst[15],m15,cst[6],m14,cst[9],m9,cst[14],m11,cst[3],m3,cst[11],m0,cst[8],m8,cst[0],m12,cst[2],m2,cst[12],m13,cst[7],m7,cst[13],m1,cst[4],m4,cst[1],m10,cst[5],m5,cst[10]);
    #[rustfmt::skip]
    ROUND(v, m10,cst[2],m2,cst[10],m8,cst[4],m4,cst[8],m7,cst[6],m6,cst[7],m1,cst[5],m5,cst[1],m15,cst[11],m11,cst[15],m9,cst[14],m14,cst[9],m3,cst[12],m12,cst[3],m13,cst[0],m0,cst[13]);
    #[rustfmt::skip]
    ROUND(v, m0,cst[1],m1,cst[0],m2,cst[3],m3,cst[2],m4,cst[5],m5,cst[4],m6,cst[7],m7,cst[6],m8,cst[9],m9,cst[8],m10,cst[11],m11,cst[10],m12,cst[13],m13,cst[12],m14,cst[15],m15,cst[14]);
    #[rustfmt::skip]
    ROUND(v, m14,cst[10],m10,cst[14],m4,cst[8],m8,cst[4],m9,cst[15],m15,cst[9],m13,cst[6],m6,cst[13],m1,cst[12],m12,cst[1],m0,cst[2],m2,cst[0],m11,cst[7],m7,cst[11],m5,cst[3],m3,cst[5]);
    #[rustfmt::skip]
    ROUND(v, m11,cst[8],m8,cst[11],m12,cst[0],m0,cst[12],m5,cst[2],m2,cst[5],m15,cst[13],m13,cst[15],m10,cst[14],m14,cst[10],m3,cst[6],m6,cst[3],m7,cst[1],m1,cst[7],m9,cst[4],m4,cst[9]);
    #[rustfmt::skip]
    ROUND(v, m7,cst[9],m9,cst[7],m3,cst[1],m1,cst[3],m13,cst[12],m12,cst[13],m11,cst[14],m14,cst[11],m2,cst[6],m6,cst[2],m5,cst[10],m10,cst[5],m4,cst[0],m0,cst[4],m15,cst[8],m8,cst[15]);

    v[0] ^= v[8];
    v[1] ^= v[9];
    v[2] ^= v[10];
    v[3] ^= v[11];
    v[4] ^= v[12];
    v[5] ^= v[13];
    v[6] ^= v[14];
    v[7] ^= v[15];

    v[0] ^= (*S).s[0];
    v[1] ^= (*S).s[1];
    v[2] ^= (*S).s[2];
    v[3] ^= (*S).s[3];
    v[4] ^= (*S).s[0];
    v[5] ^= (*S).s[1];
    v[6] ^= (*S).s[2];
    v[7] ^= (*S).s[3];

    (*S).h[0] ^= v[0];
    (*S).h[1] ^= v[1];
    (*S).h[2] ^= v[2];
    (*S).h[3] ^= v[3];
    (*S).h[4] ^= v[4];
    (*S).h[5] ^= v[5];
    (*S).h[6] ^= v[6];
    (*S).h[7] ^= v[7];
}

/// ```c
/// void blake256_init( blakestate256 *S )
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_init(S: *mut blakestate256) {
    (*S).h[0] = 0x6A09E667;
    (*S).h[1] = 0xBB67AE85;
    (*S).h[2] = 0x3C6EF372;
    (*S).h[3] = 0xA54FF53A;
    (*S).h[4] = 0x510E527F;
    (*S).h[5] = 0x9B05688C;
    (*S).h[6] = 0x1F83D9AB;
    (*S).h[7] = 0x5BE0CD19;
    /* S->t[0]=S->t[1]=S->buflen=S->nullt=0; */
    (*S).nullt = 0;
    (*S).buflen = 0;
    (*S).t[1] = 0;
    (*S).t[0] = 0;
    /* S->s[0]=S->s[1]=S->s[2]=S->s[3]=0; */
    (*S).s[3] = 0;
    (*S).s[2] = 0;
    (*S).s[1] = 0;
    (*S).s[0] = 0;
}

/// ```c
/// void blake256_update( blakestate256 *S, const u8 *data, u64 datalen )
/// ```
///
/// Note that `datalen` counts **bits**, not bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_update(S: *mut blakestate256, in_: *const u8, inlen: u64) {
    let mut data: *const u8 = in_;
    let mut datalen: u64 = inlen;

    let mut left: i32 = (*S).buflen >> 3;
    let fill: i32 = 64 - left;

    if left != 0 && (((datalen >> 3) & 0x3F) >= fill as u64) {
        core::ptr::copy_nonoverlapping(
            data,
            (core::ptr::addr_of_mut!((*S).buf) as *mut u8).add(left as usize),
            fill as usize,
        );
        (*S).t[0] = (*S).t[0].wrapping_add(512);
        if (*S).t[0] == 0 {
            (*S).t[1] = (*S).t[1].wrapping_add(1);
        }
        blake256_compress(S, core::ptr::addr_of!((*S).buf) as *const u8);
        data = data.add(fill as usize);
        datalen = datalen.wrapping_sub((fill << 3) as u64);
        left = 0;
    }

    while datalen >= 512 {
        (*S).t[0] = (*S).t[0].wrapping_add(512);
        if (*S).t[0] == 0 {
            (*S).t[1] = (*S).t[1].wrapping_add(1);
        }
        blake256_compress(S, data);
        data = data.add(64);
        datalen = datalen.wrapping_sub(512);
    }

    if datalen > 0 {
        core::ptr::copy_nonoverlapping(
            data,
            (core::ptr::addr_of_mut!((*S).buf) as *mut u8).add(left as usize),
            (datalen >> 3) as usize,
        );
        (*S).buflen = ((left << 3) as u64).wrapping_add(datalen) as i32;
    } else {
        (*S).buflen = 0;
    }
}

/// ```c
/// void blake256_final( blakestate256 *S, u8 *digest )
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256_final(S: *mut blakestate256, out: *mut u8) {
    let mut msglen = [0u8; 8];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo: u32 = (*S).t[0].wrapping_add((*S).buflen as u32);
    let mut hi: u32 = (*S).t[1];
    if lo < (*S).buflen as u32 {
        hi = hi.wrapping_add(1);
    }
    U32TO8(msglen.as_mut_ptr().add(0), hi);
    U32TO8(msglen.as_mut_ptr().add(4), lo);

    if (*S).buflen == 440 {
        /* one padding byte */
        (*S).t[0] = (*S).t[0].wrapping_sub(8);
        blake256_update(S, &oo as *const u8, 8);
    } else {
        if (*S).buflen < 440 {
            /* enough space to fill the block */
            if (*S).buflen == 0 {
                (*S).nullt = 1;
            }
            (*S).t[0] = (*S).t[0].wrapping_sub(440i32.wrapping_sub((*S).buflen) as u32);
            blake256_update(
                S,
                padding.as_ptr(),
                440i32.wrapping_sub((*S).buflen) as i64 as u64,
            );
        } else {
            /* need 2 compressions */
            (*S).t[0] = (*S).t[0].wrapping_sub(512i32.wrapping_sub((*S).buflen) as u32);
            blake256_update(
                S,
                padding.as_ptr(),
                512i32.wrapping_sub((*S).buflen) as i64 as u64,
            );
            (*S).t[0] = (*S).t[0].wrapping_sub(440);
            blake256_update(S, padding.as_ptr().add(1), 440);
            (*S).nullt = 1;
        }
        blake256_update(S, &zo as *const u8, 8);
        (*S).t[0] = (*S).t[0].wrapping_sub(8);
    }
    (*S).t[0] = (*S).t[0].wrapping_sub(64);
    blake256_update(S, msglen.as_ptr(), 64);

    U32TO8(out.add(0), (*S).h[0]);
    U32TO8(out.add(4), (*S).h[1]);
    U32TO8(out.add(8), (*S).h[2]);
    U32TO8(out.add(12), (*S).h[3]);
    U32TO8(out.add(16), (*S).h[4]);
    U32TO8(out.add(20), (*S).h[5]);
    U32TO8(out.add(24), (*S).h[6]);
    U32TO8(out.add(28), (*S).h[7]);
}

/// mgf1 function based on the BLAKE-256 hash function.
///
/// Note that `inlen` should be sufficiently small that it still allows for an
/// array to be allocated on the stack. Typically 'in' is merely a seed.
/// Outputs `outlen` number of bytes.
///
/// ```c
/// void blake256_mgf1(unsigned char *out, unsigned long outlen,
///                    const unsigned char *in, unsigned long inlen)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_blake256_mgf1(out: *mut u8, outlen: u64, in_: *const u8, inlen: u64) {
    /* SPX_VLA(uint8_t, inbuf, inlen+4); */
    let mut inbuf = vec![0u8; (inlen as usize).wrapping_add(4)];
    let inbuf_ptr = inbuf.as_mut_ptr();
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    let mut out = out;

    core::ptr::copy_nonoverlapping(in_, inbuf_ptr, inlen as usize);

    /* While we can fit in at least another full block of BLAKE256 output.. */
    let mut i: u64 = 0;
    while i
        .wrapping_add(1)
        .wrapping_mul(SPX_BLAKE256_OUTPUT_BYTES as u64)
        <= outlen
    {
        SPX_u32_to_bytes(inbuf_ptr.add(inlen as usize), i as u32);
        blake256(out, inbuf_ptr as *const u8, inlen.wrapping_add(4));
        out = out.add(SPX_BLAKE256_OUTPUT_BYTES);
        i = i.wrapping_add(1);
    }
    /* Until we cannot anymore, and we fill the remainder. */
    if outlen > i.wrapping_mul(SPX_BLAKE256_OUTPUT_BYTES as u64) {
        SPX_u32_to_bytes(inbuf_ptr.add(inlen as usize), i as u32);
        blake256(
            outbuf.as_mut_ptr(),
            inbuf_ptr as *const u8,
            inlen.wrapping_add(4),
        );
        core::ptr::copy_nonoverlapping(
            outbuf.as_ptr(),
            out,
            outlen.wrapping_sub(i.wrapping_mul(SPX_BLAKE256_OUTPUT_BYTES as u64)) as usize,
        );
    }
}

/// ```c
/// int blake256( unsigned char *out, const unsigned char *in,
///               unsigned long long inlen )
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake256(out: *mut u8, in_: *const u8, inlen: u64) -> i32 {
    let mut S = blakestate256::new();
    blake256_init(&mut S as *mut blakestate256);
    blake256_update(&mut S as *mut blakestate256, in_, inlen.wrapping_mul(8));
    blake256_final(&mut S as *mut blakestate256, out);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(data: &[u8]) -> [u8; 32] {
        let mut out = [0u8; 32];
        let rc = unsafe { blake256(out.as_mut_ptr(), data.as_ptr(), data.len() as u64) };
        assert_eq!(rc, 0);
        out
    }

    fn hex(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    /// The two vectors printed by the `main()` of the original BLAKE
    /// reference implementation: one zero byte and 72 zero bytes.
    #[test]
    fn reference_test_vectors() {
        assert_eq!(
            hex(&hash(&[0x00])),
            "0ce8d4ef4dd7cd8d62dfded9d4edb0a774ae6a41929a74da23109e8f11139c87"
        );
        assert_eq!(
            hex(&hash(&[0x00; 72])),
            "d419bad32d504fb7d44d460c42c5593fe544fa4c135dec31e21bd9abdcc22d41"
        );
    }

    /// The empty message exercises the `nullt` path of `blake256_final`.
    #[test]
    fn empty_message() {
        assert_eq!(
            hex(&hash(&[])),
            "716f6e863f744b9ac22c97ec7b76ea5f5908bc5b2f67c61510bfc4751384ea7a"
        );
    }

    /// Multi-block message; the expected digest was produced by compiling and
    /// running the C reference (`lib/blake/src/blake256.c`) on the same input.
    #[test]
    fn multi_block_message() {
        let msg: Vec<u8> = (0..300u32).map(|i| (i * 7 + 3) as u8).collect();
        assert_eq!(
            hex(&hash(&msg)),
            "334b4ea228a3df893fab49bb571a2fa57039ac909dea8c7d1a10d07f583cd6fa"
        );
    }

    /// Two-call `blake256_update`. Note that the reference `update()` only
    /// flushes an already partially filled buffer when
    /// `((datalen >> 3) & 0x3F) >= fill`, so for some split points the
    /// buffered bytes are *not* prepended to the following blocks and the
    /// result differs from the one-shot digest. Both expected values below
    /// come from the C reference, so this test pins that exact behaviour.
    #[test]
    fn incremental_update_matches_c_reference() {
        let msg: Vec<u8> = (0..300u32).map(|i| (i * 7 + 3) as u8).collect();

        let split_digest = |split: usize| -> [u8; 32] {
            let mut S = blakestate256::new();
            let mut out = [0u8; 32];
            unsafe {
                blake256_init(&mut S);
                blake256_update(&mut S, msg.as_ptr(), (split as u64) * 8);
                blake256_update(
                    &mut S,
                    msg.as_ptr().add(split),
                    ((msg.len() - split) as u64) * 8,
                );
                blake256_final(&mut S, out.as_mut_ptr());
            }
            out
        };

        /* Splits for which the buffer is flushed / empty: same as one-shot. */
        for split in [0usize, 55, 56, 63, 64, 119, 127, 128, 255, 256] {
            assert_eq!(split_digest(split), hash(&msg), "split at {}", split);
        }
        /* Splits that hit the quirk above (values from the C reference). */
        assert_eq!(
            hex(&split_digest(1)),
            "72d544fa5f464100bb0a56ea995e4601fdee7c431e5ab3669a9fb9e9138b81a1"
        );
        assert_eq!(
            hex(&split_digest(31)),
            "1cae6f4ef03b88421ec8390e6e872a932f7f508ec6a0b4e1377d008ded96027a"
        );
        assert_eq!(
            hex(&split_digest(199)),
            "8c6292288d0d0b836858007c95f6591fea192c821ff100f6a026578457b812dc"
        );
    }

    /// `blake256_compress` with a non-zero salt, a non-zero counter and both
    /// values of `nullt` (expected chaining values taken from the C
    /// reference).
    #[test]
    fn compress_with_salt_and_counter() {
        let block: Vec<u8> = (0..128u32).map(|i| (i * 7 + 3) as u8).collect();
        let mut S = blakestate256::new();
        unsafe {
            blake256_init(&mut S);
            S.s = [0x01020304, 0x05060708, 0x090a0b0c, 0x0d0e0f10];
            S.t = [0xfffffe00, 0x12345678];
            S.nullt = 0;
            blake256_compress(&mut S, block.as_ptr());
            assert_eq!(
                S.h,
                [
                    0x3C5970BE, 0x2E14EC14, 0xCF97B57E, 0x58689A26, 0x81B19490, 0x9C475064,
                    0x0ECE43E7, 0x96E8ABD0
                ]
            );
            S.nullt = 1;
            blake256_compress(&mut S, block.as_ptr().add(64));
            assert_eq!(
                S.h,
                [
                    0xE1A43D43, 0xF71DC341, 0x639BF662, 0x766C8D29, 0x09213D5A, 0x84276102,
                    0x53B096BA, 0x2316E56D
                ]
            );
        }
    }

    /// mgf1 is the concatenation of BLAKE-256 over `seed || be32(counter)`,
    /// truncated to `outlen`.
    #[test]
    fn mgf1_matches_manual() {
        let seed: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut out = [0u8; 70];
        unsafe { SPX_blake256_mgf1(out.as_mut_ptr(), 70, seed.as_ptr(), 8) };

        let mut expected: Vec<u8> = Vec::new();
        for i in 0u32..3 {
            let mut block = seed.to_vec();
            block.extend_from_slice(&i.to_be_bytes());
            expected.extend_from_slice(&hash(&block));
        }
        assert_eq!(out[..], expected[..70]);
    }
}
