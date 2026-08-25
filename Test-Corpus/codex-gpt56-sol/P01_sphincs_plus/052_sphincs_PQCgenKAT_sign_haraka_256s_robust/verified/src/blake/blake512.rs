//! Translation of `lib/blake/src/blake512.c`.
//!
//! ```text
//! // supercop-20140525/crypto_hash/blake512/sandy
//!
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
//! The C file's `typedef`s map as follows: `u64`/`crypto_uint64` -> `u64`,
//! `u32`/`crypto_uint32` -> `u32`, `u8`/`crypto_uint8` -> `u8`.

use crate::utils::SPX_u32_to_bytes;

/// `SPX_BLAKE512_OUTPUT_BYTES` from `lib/blake/include/blake.h`.
pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

/// ```c
/// typedef struct
/// {
///   unsigned long long h[8], s[4], t[2];
///   int buflen, nullt;
///   unsigned char buf[128];
/// } blakestate512;
/// ```
#[repr(C)]
#[derive(Clone, Copy)]
pub struct blakestate512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

impl blakestate512 {
    /// All-zero state; the C code declares `blakestate512 S;` uninitialised and
    /// then calls `blake512_init()`, which sets every field it relies on.
    pub const fn new() -> Self {
        blakestate512 {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 128],
        }
    }
}

impl Default for blakestate512 {
    fn default() -> Self {
        Self::new()
    }
}

/// ```c
/// #define U8TO32(p)
///   (((uint32_t)((p)[0]) << 24) | ((uint32_t)((p)[1]) << 16) |
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
/// #define U32TO8(p, v)
///   (p)[0] = (uint8_t)((v) >> 24);
///   (p)[1] = (uint8_t)((v) >> 16);
///   (p)[2] = (uint8_t)((v) >> 8);
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
/// #define U8TO64(p) (((uint64_t)U8TO32(p) << 32) | ((uint64_t)U8TO32(p + 4)))
/// ```
#[inline(always)]
unsafe fn U8TO64(p: *const u8) -> u64 {
    ((U8TO32(p) as u64) << 32) | (U8TO32(p.add(4)) as u64)
}

/// ```c
/// #define U64TO8(p, v)
///   U32TO8((p),     (uint32_t)((v) >> 32));
///   U32TO8((p) + 4, (uint32_t)((v)      ));
/// ```
#[inline(always)]
unsafe fn U64TO8(p: *mut u8, v: u64) {
    U32TO8(p, (v >> 32) as u32);
    U32TO8(p.add(4), v as u32);
}

/// ```c
/// const u64 cst[16] = { ... };
/// ```
///
/// Note: unlike `blake256.c`'s `static const u32 cst[16]`, the BLAKE-512 table
/// has *external* linkage in the C source (no `static`), so `libblake.so`
/// exports it as the data symbol `cst`.  Keep that ABI detail.
#[unsafe(no_mangle)]
pub static cst: [u64; 16] = [
    0x243F6A8885A308D3,
    0x13198A2E03707344,
    0xA4093822299F31D0,
    0x082EFA98EC4E6C89,
    0x452821E638D01377,
    0xBE5466CF34E90C6C,
    0xC0AC29B7C97C50DD,
    0x3F84D5B5B5470917,
    0x9216D5D98979FB1B,
    0xD1310BA698DFB5AC,
    0x2FFD72DBD01ADFB7,
    0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99,
    0x24A19947B3916CF7,
    0x0801F2E2858EFC16,
    0x636920D871574E69,
];

/// ```c
/// static const u8 padding[129] = {0x80,0,0,...,0};
/// ```
static padding: [u8; 129] = {
    let mut p = [0u8; 129];
    p[0] = 0x80;
    p
};

/// ```c
/// #define BLAKE512_ROT(x,n) (((x)<<(64-n))|( (x)>>(n)))
/// ```
#[inline(always)]
const fn BLAKE512_ROT(x: u64, n: u32) -> u64 {
    (x << (64 - n)) | (x >> n)
}

/// Faithful transcription of the `ROUND(...)` macro of `blake512.c`; `$v` is the
/// 16-word working vector `v0..v15`, all additions are modular.
macro_rules! ROUND {
    ($v:ident,
     $m0:expr, $c0:expr, $m1:expr, $c1:expr, $m2:expr, $c2:expr, $m3:expr, $c3:expr,
     $m4:expr, $c4:expr, $m5:expr, $c5:expr, $m6:expr, $c6:expr, $m7:expr, $c7:expr,
     $m8:expr, $c8:expr, $m9:expr, $c9:expr, $m10:expr, $c10:expr, $m11:expr, $c11:expr,
     $m12:expr, $c12:expr, $m13:expr, $c13:expr, $m14:expr, $c14:expr, $m15:expr, $c15:expr
     ) => {{
        $v[0] = $v[0].wrapping_add($m0 ^ $c0);
        $v[0] = $v[0].wrapping_add($v[4]);
        $v[12] ^= $v[0];
        $v[12] = BLAKE512_ROT($v[12], 32);
        $v[8] = $v[8].wrapping_add($v[12]);
        $v[4] ^= $v[8];
        $v[4] = BLAKE512_ROT($v[4], 25);

        $v[1] = $v[1].wrapping_add($m2 ^ $c2);
        $v[1] = $v[1].wrapping_add($v[5]);
        $v[13] ^= $v[1];
        $v[13] = BLAKE512_ROT($v[13], 32);
        $v[9] = $v[9].wrapping_add($v[13]);
        $v[5] ^= $v[9];
        $v[5] = BLAKE512_ROT($v[5], 25);

        $v[2] = $v[2].wrapping_add($m4 ^ $c4);
        $v[2] = $v[2].wrapping_add($v[6]);
        $v[14] ^= $v[2];
        $v[14] = BLAKE512_ROT($v[14], 32);
        $v[10] = $v[10].wrapping_add($v[14]);
        $v[6] ^= $v[10];
        $v[6] = BLAKE512_ROT($v[6], 25);

        $v[3] = $v[3].wrapping_add($m6 ^ $c6);
        $v[3] = $v[3].wrapping_add($v[7]);
        $v[15] ^= $v[3];
        $v[15] = BLAKE512_ROT($v[15], 32);
        $v[11] = $v[11].wrapping_add($v[15]);
        $v[7] ^= $v[11];
        $v[7] = BLAKE512_ROT($v[7], 25);

        $v[2] = $v[2].wrapping_add($m5 ^ $c5);
        $v[2] = $v[2].wrapping_add($v[6]);
        $v[14] ^= $v[2];
        $v[14] = BLAKE512_ROT($v[14], 16);
        $v[10] = $v[10].wrapping_add($v[14]);
        $v[6] ^= $v[10];
        $v[6] = BLAKE512_ROT($v[6], 11);

        $v[3] = $v[3].wrapping_add($m7 ^ $c7);
        $v[3] = $v[3].wrapping_add($v[7]);
        $v[15] ^= $v[3];
        $v[15] = BLAKE512_ROT($v[15], 16);
        $v[11] = $v[11].wrapping_add($v[15]);
        $v[7] ^= $v[11];
        $v[7] = BLAKE512_ROT($v[7], 11);

        $v[1] = $v[1].wrapping_add($m3 ^ $c3);
        $v[1] = $v[1].wrapping_add($v[5]);
        $v[13] ^= $v[1];
        $v[13] = BLAKE512_ROT($v[13], 16);
        $v[9] = $v[9].wrapping_add($v[13]);
        $v[5] ^= $v[9];
        $v[5] = BLAKE512_ROT($v[5], 11);

        $v[0] = $v[0].wrapping_add($m1 ^ $c1);
        $v[0] = $v[0].wrapping_add($v[4]);
        $v[12] ^= $v[0];
        $v[12] = BLAKE512_ROT($v[12], 16);
        $v[8] = $v[8].wrapping_add($v[12]);
        $v[4] ^= $v[8];
        $v[4] = BLAKE512_ROT($v[4], 11);

        $v[0] = $v[0].wrapping_add($m8 ^ $c8);
        $v[0] = $v[0].wrapping_add($v[5]);
        $v[15] ^= $v[0];
        $v[15] = BLAKE512_ROT($v[15], 32);
        $v[10] = $v[10].wrapping_add($v[15]);
        $v[5] ^= $v[10];
        $v[5] = BLAKE512_ROT($v[5], 25);

        $v[1] = $v[1].wrapping_add($m10 ^ $c10);
        $v[1] = $v[1].wrapping_add($v[6]);
        $v[12] ^= $v[1];
        $v[12] = BLAKE512_ROT($v[12], 32);
        $v[11] = $v[11].wrapping_add($v[12]);
        $v[6] ^= $v[11];
        $v[6] = BLAKE512_ROT($v[6], 25);

        $v[2] = $v[2].wrapping_add($m12 ^ $c12);
        $v[2] = $v[2].wrapping_add($v[7]);
        $v[13] ^= $v[2];
        $v[13] = BLAKE512_ROT($v[13], 32);
        $v[8] = $v[8].wrapping_add($v[13]);
        $v[7] ^= $v[8];
        $v[7] = BLAKE512_ROT($v[7], 25);

        $v[3] = $v[3].wrapping_add($m14 ^ $c14);
        $v[3] = $v[3].wrapping_add($v[4]);
        $v[14] ^= $v[3];
        $v[14] = BLAKE512_ROT($v[14], 32);
        $v[9] = $v[9].wrapping_add($v[14]);
        $v[4] ^= $v[9];
        $v[4] = BLAKE512_ROT($v[4], 25);

        $v[2] = $v[2].wrapping_add($m13 ^ $c13);
        $v[2] = $v[2].wrapping_add($v[7]);
        $v[13] ^= $v[2];
        $v[13] = BLAKE512_ROT($v[13], 16);
        $v[8] = $v[8].wrapping_add($v[13]);
        $v[7] ^= $v[8];
        $v[7] = BLAKE512_ROT($v[7], 11);

        $v[3] = $v[3].wrapping_add($m15 ^ $c15);
        $v[3] = $v[3].wrapping_add($v[4]);
        $v[14] ^= $v[3];
        $v[14] = BLAKE512_ROT($v[14], 16);
        $v[9] = $v[9].wrapping_add($v[14]);
        $v[4] ^= $v[9];
        $v[4] = BLAKE512_ROT($v[4], 11);

        $v[1] = $v[1].wrapping_add($m11 ^ $c11);
        $v[1] = $v[1].wrapping_add($v[6]);
        $v[12] ^= $v[1];
        $v[12] = BLAKE512_ROT($v[12], 16);
        $v[11] = $v[11].wrapping_add($v[12]);
        $v[6] ^= $v[11];
        $v[6] = BLAKE512_ROT($v[6], 11);

        $v[0] = $v[0].wrapping_add($m9 ^ $c9);
        $v[0] = $v[0].wrapping_add($v[5]);
        $v[15] ^= $v[0];
        $v[15] = BLAKE512_ROT($v[15], 16);
        $v[10] = $v[10].wrapping_add($v[15]);
        $v[5] ^= $v[10];
        $v[5] = BLAKE512_ROT($v[5], 11);
    }};
}

/// ```c
/// void blake512_compress( blakestate512 *S, const u8 *block )
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_compress(S: *mut blakestate512, block: *const u8) {
    let m0: u64 = U8TO64(block.add(0));
    let m1: u64 = U8TO64(block.add(8));
    let m2: u64 = U8TO64(block.add(16));
    let m3: u64 = U8TO64(block.add(24));
    let m4: u64 = U8TO64(block.add(32));
    let m5: u64 = U8TO64(block.add(40));
    let m6: u64 = U8TO64(block.add(48));
    let m7: u64 = U8TO64(block.add(56));
    let m8: u64 = U8TO64(block.add(64));
    let m9: u64 = U8TO64(block.add(72));
    let m10: u64 = U8TO64(block.add(80));
    let m11: u64 = U8TO64(block.add(88));
    let m12: u64 = U8TO64(block.add(96));
    let m13: u64 = U8TO64(block.add(104));
    let m14: u64 = U8TO64(block.add(112));
    let m15: u64 = U8TO64(block.add(120));

    /* v0..v15 */
    let mut v: [u64; 16] = [0; 16];
    v[0] = (*S).h[0];
    v[1] = (*S).h[1];
    v[2] = (*S).h[2];
    v[3] = (*S).h[3];
    v[4] = (*S).h[4];
    v[5] = (*S).h[5];
    v[6] = (*S).h[6];
    v[7] = (*S).h[7];
    v[8] = (*S).s[0] ^ 0x243F6A8885A308D3;
    v[9] = (*S).s[1] ^ 0x13198A2E03707344;
    v[10] = (*S).s[2] ^ 0xA4093822299F31D0;
    v[11] = (*S).s[3] ^ 0x082EFA98EC4E6C89;
    v[12] = 0x452821E638D01377;
    v[13] = 0xBE5466CF34E90C6C;
    v[14] = 0xC0AC29B7C97C50DD;
    v[15] = 0x3F84D5B5B5470917;

    if (*S).nullt == 0 {
        v[12] ^= (*S).t[0];
        v[13] ^= (*S).t[0];
        v[14] ^= (*S).t[1];
        v[15] ^= (*S).t[1];
    }

    ROUND!(
        v, m0, cst[1], m1, cst[0], m2, cst[3], m3, cst[2], m4, cst[5], m5, cst[4], m6, cst[7], m7,
        cst[6], m8, cst[9], m9, cst[8], m10, cst[11], m11, cst[10], m12, cst[13], m13, cst[12],
        m14, cst[15], m15, cst[14]
    );
    ROUND!(
        v, m14, cst[10], m10, cst[14], m4, cst[8], m8, cst[4], m9, cst[15], m15, cst[9], m13,
        cst[6], m6, cst[13], m1, cst[12], m12, cst[1], m0, cst[2], m2, cst[0], m11, cst[7], m7,
        cst[11], m5, cst[3], m3, cst[5]
    );
    ROUND!(
        v, m11, cst[8], m8, cst[11], m12, cst[0], m0, cst[12], m5, cst[2], m2, cst[5], m15,
        cst[13], m13, cst[15], m10, cst[14], m14, cst[10], m3, cst[6], m6, cst[3], m7, cst[1], m1,
        cst[7], m9, cst[4], m4, cst[9]
    );
    ROUND!(
        v, m7, cst[9], m9, cst[7], m3, cst[1], m1, cst[3], m13, cst[12], m12, cst[13], m11,
        cst[14], m14, cst[11], m2, cst[6], m6, cst[2], m5, cst[10], m10, cst[5], m4, cst[0], m0,
        cst[4], m15, cst[8], m8, cst[15]
    );
    ROUND!(
        v, m9, cst[0], m0, cst[9], m5, cst[7], m7, cst[5], m2, cst[4], m4, cst[2], m10, cst[15],
        m15, cst[10], m14, cst[1], m1, cst[14], m11, cst[12], m12, cst[11], m6, cst[8], m8, cst[6],
        m3, cst[13], m13, cst[3]
    );
    ROUND!(
        v, m2, cst[12], m12, cst[2], m6, cst[10], m10, cst[6], m0, cst[11], m11, cst[0], m8,
        cst[3], m3, cst[8], m4, cst[13], m13, cst[4], m7, cst[5], m5, cst[7], m15, cst[14], m14,
        cst[15], m1, cst[9], m9, cst[1]
    );
    ROUND!(
        v, m12, cst[5], m5, cst[12], m1, cst[15], m15, cst[1], m14, cst[13], m13, cst[14], m4,
        cst[10], m10, cst[4], m0, cst[7], m7, cst[0], m6, cst[3], m3, cst[6], m9, cst[2], m2,
        cst[9], m8, cst[11], m11, cst[8]
    );
    ROUND!(
        v, m13, cst[11], m11, cst[13], m7, cst[14], m14, cst[7], m12, cst[1], m1, cst[12], m3,
        cst[9], m9, cst[3], m5, cst[0], m0, cst[5], m15, cst[4], m4, cst[15], m8, cst[6], m6,
        cst[8], m2, cst[10], m10, cst[2]
    );
    ROUND!(
        v, m6, cst[15], m15, cst[6], m14, cst[9], m9, cst[14], m11, cst[3], m3, cst[11], m0,
        cst[8], m8, cst[0], m12, cst[2], m2, cst[12], m13, cst[7], m7, cst[13], m1, cst[4], m4,
        cst[1], m10, cst[5], m5, cst[10]
    );
    ROUND!(
        v, m10, cst[2], m2, cst[10], m8, cst[4], m4, cst[8], m7, cst[6], m6, cst[7], m1, cst[5],
        m5, cst[1], m15, cst[11], m11, cst[15], m9, cst[14], m14, cst[9], m3, cst[12], m12, cst[3],
        m13, cst[0], m0, cst[13]
    );
    ROUND!(
        v, m0, cst[1], m1, cst[0], m2, cst[3], m3, cst[2], m4, cst[5], m5, cst[4], m6, cst[7], m7,
        cst[6], m8, cst[9], m9, cst[8], m10, cst[11], m11, cst[10], m12, cst[13], m13, cst[12],
        m14, cst[15], m15, cst[14]
    );
    ROUND!(
        v, m14, cst[10], m10, cst[14], m4, cst[8], m8, cst[4], m9, cst[15], m15, cst[9], m13,
        cst[6], m6, cst[13], m1, cst[12], m12, cst[1], m0, cst[2], m2, cst[0], m11, cst[7], m7,
        cst[11], m5, cst[3], m3, cst[5]
    );
    ROUND!(
        v, m11, cst[8], m8, cst[11], m12, cst[0], m0, cst[12], m5, cst[2], m2, cst[5], m15,
        cst[13], m13, cst[15], m10, cst[14], m14, cst[10], m3, cst[6], m6, cst[3], m7, cst[1], m1,
        cst[7], m9, cst[4], m4, cst[9]
    );
    ROUND!(
        v, m7, cst[9], m9, cst[7], m3, cst[1], m1, cst[3], m13, cst[12], m12, cst[13], m11,
        cst[14], m14, cst[11], m2, cst[6], m6, cst[2], m5, cst[10], m10, cst[5], m4, cst[0], m0,
        cst[4], m15, cst[8], m8, cst[15]
    );
    ROUND!(
        v, m9, cst[0], m0, cst[9], m5, cst[7], m7, cst[5], m2, cst[4], m4, cst[2], m10, cst[15],
        m15, cst[10], m14, cst[1], m1, cst[14], m11, cst[12], m12, cst[11], m6, cst[8], m8, cst[6],
        m3, cst[13], m13, cst[3]
    );
    ROUND!(
        v, m2, cst[12], m12, cst[2], m6, cst[10], m10, cst[6], m0, cst[11], m11, cst[0], m8,
        cst[3], m3, cst[8], m4, cst[13], m13, cst[4], m7, cst[5], m5, cst[7], m15, cst[14], m14,
        cst[15], m1, cst[9], m9, cst[1]
    );

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
/// void blake512_init( blakestate512 *S )
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_init(S: *mut blakestate512) {
    (*S).h[0] = 0x6A09E667F3BCC908;
    (*S).h[1] = 0xBB67AE8584CAA73B;
    (*S).h[2] = 0x3C6EF372FE94F82B;
    (*S).h[3] = 0xA54FF53A5F1D36F1;
    (*S).h[4] = 0x510E527FADE682D1;
    (*S).h[5] = 0x9B05688C2B3E6C1F;
    (*S).h[6] = 0x1F83D9ABFB41BD6B;
    (*S).h[7] = 0x5BE0CD19137E2179;
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
/// void blake512_update( blakestate512 * S, const u8 * data, u64 datalen )
/// ```
///
/// Note that `datalen` counts *bits*, not bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_update(S: *mut blakestate512, in_: *const u8, inlen: u64) {
    let mut data: *const u8 = in_;
    let mut datalen: u64 = inlen;

    let mut left: i32 = (*S).buflen >> 3;
    let fill: i32 = 128 - left;

    if left != 0 && (((datalen >> 3) & 0x7F) >= fill as i64 as u64) {
        core::ptr::copy_nonoverlapping(
            data,
            (core::ptr::addr_of_mut!((*S).buf) as *mut u8).add(left as usize),
            fill as usize,
        );
        (*S).t[0] = (*S).t[0].wrapping_add(1024);
        blake512_compress(S, core::ptr::addr_of!((*S).buf) as *const u8);
        data = data.add(fill as usize);
        datalen = datalen.wrapping_sub((fill << 3) as i64 as u64);
        left = 0;
    }

    while datalen >= 1024 {
        (*S).t[0] = (*S).t[0].wrapping_add(1024);
        blake512_compress(S, data);
        data = data.add(128);
        datalen = datalen.wrapping_sub(1024);
    }

    if datalen > 0 {
        core::ptr::copy_nonoverlapping(
            data,
            (core::ptr::addr_of_mut!((*S).buf) as *mut u8).add(left as usize),
            ((datalen >> 3) & 0x7F) as usize,
        );
        (*S).buflen = ((left << 3) as i64 as u64).wrapping_add(datalen) as i32;
    } else {
        (*S).buflen = 0;
    }
}

/// ```c
/// void blake512_final( blakestate512 * S, u8 * digest )
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_final(S: *mut blakestate512, out: *mut u8) {
    let mut msglen: [u8; 16] = [0; 16];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo: u64 = (*S).t[0].wrapping_add((*S).buflen as i64 as u64);
    let mut hi: u64 = (*S).t[1];
    if lo < (*S).buflen as i64 as u64 {
        hi = hi.wrapping_add(1);
    }
    U64TO8(msglen.as_mut_ptr().add(0), hi);
    U64TO8(msglen.as_mut_ptr().add(8), lo);

    if (*S).buflen == 888 {
        /* one padding byte */
        (*S).t[0] = (*S).t[0].wrapping_sub(8);
        blake512_update(S, &oo as *const u8, 8);
    } else {
        if (*S).buflen < 888 {
            /* enough space to fill the block */
            if (*S).buflen == 0 {
                (*S).nullt = 1;
            }
            (*S).t[0] = (*S).t[0].wrapping_sub(888i32.wrapping_sub((*S).buflen) as i64 as u64);
            blake512_update(
                S,
                padding.as_ptr(),
                888i32.wrapping_sub((*S).buflen) as i64 as u64,
            );
        } else {
            /* NOT enough space, need 2 compressions */
            (*S).t[0] = (*S).t[0].wrapping_sub(1024i32.wrapping_sub((*S).buflen) as i64 as u64);
            blake512_update(
                S,
                padding.as_ptr(),
                1024i32.wrapping_sub((*S).buflen) as i64 as u64,
            );
            (*S).t[0] = (*S).t[0].wrapping_sub(888);
            blake512_update(S, padding.as_ptr().add(1), 888);
            (*S).nullt = 1;
        }
        blake512_update(S, &zo as *const u8, 8);
        (*S).t[0] = (*S).t[0].wrapping_sub(8);
    }
    (*S).t[0] = (*S).t[0].wrapping_sub(128);
    blake512_update(S, msglen.as_ptr(), 128);

    U64TO8(out.add(0), (*S).h[0]);
    U64TO8(out.add(8), (*S).h[1]);
    U64TO8(out.add(16), (*S).h[2]);
    U64TO8(out.add(24), (*S).h[3]);
    U64TO8(out.add(32), (*S).h[4]);
    U64TO8(out.add(40), (*S).h[5]);
    U64TO8(out.add(48), (*S).h[6]);
    U64TO8(out.add(56), (*S).h[7]);
}

/// ```c
/// void blake512_mgf1(unsigned char *out, unsigned long outlen,
///                    const unsigned char *in, unsigned long inlen)
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_blake512_mgf1(out: *mut u8, outlen: u64, in_: *const u8, inlen: u64) {
    /* SPX_VLA(uint8_t, inbuf, inlen+4); */
    let mut inbuf = vec![0u8; (inlen as usize).wrapping_add(4)];
    let inbuf_ptr = inbuf.as_mut_ptr();
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];

    let mut out = out;

    core::ptr::copy_nonoverlapping(in_, inbuf_ptr, inlen as usize);

    /* While we can fit in at least another full block of BLAKE512 output.. */
    let mut i: u64 = 0;
    while i
        .wrapping_add(1)
        .wrapping_mul(SPX_BLAKE512_OUTPUT_BYTES as u64)
        <= outlen
    {
        SPX_u32_to_bytes(inbuf_ptr.add(inlen as usize), i as u32);
        blake512(out, inbuf_ptr as *const u8, inlen.wrapping_add(4));
        out = out.add(SPX_BLAKE512_OUTPUT_BYTES);
        i = i.wrapping_add(1);
    }
    /* Until we cannot anymore, and we fill the remainder. */
    if outlen > i.wrapping_mul(SPX_BLAKE512_OUTPUT_BYTES as u64) {
        SPX_u32_to_bytes(inbuf_ptr.add(inlen as usize), i as u32);
        blake512(
            outbuf.as_mut_ptr(),
            inbuf_ptr as *const u8,
            inlen.wrapping_add(4),
        );
        core::ptr::copy_nonoverlapping(
            outbuf.as_ptr(),
            out,
            outlen.wrapping_sub(i.wrapping_mul(SPX_BLAKE512_OUTPUT_BYTES as u64)) as usize,
        );
    }
}

/// ```c
/// int blake512( unsigned char *out, const unsigned char *in,
///               unsigned long long inlen )
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512(out: *mut u8, in_: *const u8, inlen: u64) -> i32 {
    let mut S = blakestate512::new();
    blake512_init(&mut S as *mut blakestate512);
    blake512_update(&mut S as *mut blakestate512, in_, inlen.wrapping_mul(8));
    blake512_final(&mut S as *mut blakestate512, out);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(msg: &[u8]) -> [u8; 64] {
        let mut out = [0u8; 64];
        unsafe { blake512(out.as_mut_ptr(), msg.as_ptr(), msg.len() as u64) };
        out
    }

    fn hex(bytes: &[u8]) -> String {
        let mut s = String::new();
        for b in bytes {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    /// `blake512()` is exactly `init` + `update` + `final`.
    #[test]
    fn oneshot_matches_incremental_api() {
        for len in [0usize, 1, 55, 111, 112, 127, 128, 129, 200, 255, 256] {
            let msg: Vec<u8> = (0..len).map(|i| (i * 7 + 3) as u8).collect();

            let mut a = [0u8; 64];
            unsafe { blake512(a.as_mut_ptr(), msg.as_ptr(), msg.len() as u64) };

            let mut S = blakestate512::new();
            let mut b = [0u8; 64];
            unsafe {
                blake512_init(&mut S);
                blake512_update(&mut S, msg.as_ptr(), (msg.len() as u64) * 8);
                blake512_final(&mut S, b.as_mut_ptr());
            }
            assert_eq!(a, b, "len {}", len);
        }
    }

    fn split_hash(msg: &[u8], split: usize) -> [u8; 64] {
        let mut S = blakestate512::new();
        let mut out = [0u8; 64];
        unsafe {
            blake512_init(&mut S);
            blake512_update(&mut S, msg.as_ptr(), (split as u64) * 8);
            blake512_update(
                &mut S,
                msg.as_ptr().add(split),
                ((msg.len() - split) as u64) * 8,
            );
            blake512_final(&mut S, out.as_mut_ptr());
        }
        out
    }

    /// Feeding the message in two `blake512_update()` calls reproduces the
    /// one-shot digest for the split points where the reference `update()`
    /// actually takes its "fill the buffer first" branch.
    #[test]
    fn split_update_matches_oneshot() {
        let msg: Vec<u8> = (0..300usize).map(|i| (i * 7 + 3) as u8).collect();
        let full = hash(&msg);

        for split in [0usize, 64, 128, 192, 256] {
            assert_eq!(full, split_hash(&msg, split), "split {}", split);
        }
    }

    /// `blake512_update()` of the reference implementation is *not* a general
    /// streaming API: the buffered-data branch is only taken when
    /// `((datalen >> 3) & 0x7F) >= 128 - (buflen >> 3)`, and a zero-length
    /// update resets `buflen`. Both quirks are reproduced bit-for-bit; the
    /// expected values below come from running the C implementation in
    /// `c_src/lib/blake/src/blake512.c`.
    #[test]
    fn split_update_reproduces_reference_quirks() {
        let msg: Vec<u8> = (0..300usize).map(|i| (i * 7 + 3) as u8).collect();

        // 1 + 299 bytes: the 299-byte call does not flush the single buffered
        // byte, so the digest differs from the one-shot digest.
        assert_eq!(
            hex(&split_hash(&msg, 1)),
            "6bcd81078f3d1c5588bfd341d3b121039b90ba390a9f5e86b60bac0253d63a10\
             e1596bc9f6582739a883c69cfa6cfb6f3eeb2b8ca1c60017e362b75794771d84"
        );
        // 300 + 0 bytes: the trailing empty update clears `buflen`.
        assert_eq!(
            hex(&split_hash(&msg, 300)),
            "87d67e192348857ffe9b0aeb84ae6df1c41d18b42df1c57435ff2a758d9f56bb\
             3319366be9d7c7c212edc17728d8484809605176fe988db3754ec3f5a7f34fe2"
        );
    }

    /// Reference digests, taken byte-for-byte from the C implementation in
    /// `c_src/lib/blake/src/blake512.c` (compiled and run to produce them).
    /// The empty-message digest is also the published BLAKE-512 test vector.
    #[test]
    fn known_answer_tests() {
        // empty message
        assert_eq!(
            hex(&hash(&[])),
            "a8cfbbd73726062df0c6864dda65defe58ef0cc52a5625090fa17601e1eecd1b\
             628e94f396ae402a00acc9eab77b4d4c2e852aaaa25a636d80af3fc7913ef5b8"
        );
        // a single zero byte
        assert_eq!(
            hex(&hash(&[0x00])),
            "97961587f6d970faba6d2478045de6d1fabd09b61ae50932054d52bc29d31be4\
             ff9102b9f69e2bbdb83be13d4b9c06091e5fa0b48bd081b634058be0ec49beb3"
        );
        // "abc"
        assert_eq!(
            hex(&hash(b"abc")),
            "14266c7c704a3b58fb421ee69fd005fcc6eeff742136be67435df995b7c986e7\
             cbde4dbde135e7689c354d2bc5b8d260536c554b4f84c118e61efc576fed7cd3"
        );
        // 111 zero bytes: the largest message that still fits a single block
        // together with the 0x80..0x01 padding and the 16-byte length field.
        assert_eq!(
            hex(&hash(&[0u8; 111])),
            "125695c5cc01de48d8b107c101778fc447a55ad3440a17dc153c6c652faecdbf\
             017aed68f4f48826b9dfc413ef8f14ae7dfd8b74a0afcf47b61ce7dcb1058976"
        );
        // 112 zero bytes: needs the two-compression padding path.
        assert_eq!(
            hex(&hash(&[0u8; 112])),
            "aa42836448c9db34e0e45a49f916b54c25c9eefe3f9f65db0c13654bcbd9a938\
             c24251f3bedb7105fa4ea54292ce9ebf5adea15ce530fb71cdf409387a78c6ff"
        );
        // exactly one full block of zeros
        assert_eq!(
            hex(&hash(&[0u8; 128])),
            "0f6f3a3a91f752d37e3d37141d5459aca9a88ed2d5b88f71120fbe39387b635e\
             cf6402a5bcb7b18f216ea9a8137d28954098e586014c4d435c979d8860d3a977"
        );
    }

    /// mgf1 output is the concatenation of `blake512(seed || counter)` blocks.
    #[test]
    fn mgf1_matches_manual_construction() {
        let seed: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let outlen = 150usize;
        let mut out = vec![0u8; outlen];
        unsafe { SPX_blake512_mgf1(out.as_mut_ptr(), outlen as u64, seed.as_ptr(), 8) };

        let mut expected: Vec<u8> = Vec::new();
        let mut counter: u32 = 0;
        while expected.len() < outlen {
            let mut inbuf = seed.to_vec();
            inbuf.extend_from_slice(&counter.to_be_bytes());
            expected.extend_from_slice(&hash(&inbuf));
            counter += 1;
        }
        expected.truncate(outlen);
        assert_eq!(out, expected);
    }
}
