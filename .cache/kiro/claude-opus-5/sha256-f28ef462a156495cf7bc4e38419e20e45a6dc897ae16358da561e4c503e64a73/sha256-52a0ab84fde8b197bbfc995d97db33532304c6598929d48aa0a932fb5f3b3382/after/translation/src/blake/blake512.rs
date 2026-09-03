//! Translated from c_src/lib/blake/src/blake512.c
//!
//! BLAKE reference C implementation
//! Copyright (c) 2012 Jean-Philippe Aumasson
//! CC0 public domain.

use core::ffi::{c_int, c_ulonglong};

#[repr(C)]
pub struct blakestate512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: c_int,
    pub nullt: c_int,
    pub buf: [u8; 128],
}

impl blakestate512 {
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

// #define U8TO32(p)
#[inline(always)]
unsafe fn u8to32(p: *const u8) -> u32 {
    ((*p.add(0) as u32) << 24)
        | ((*p.add(1) as u32) << 16)
        | ((*p.add(2) as u32) << 8)
        | (*p.add(3) as u32)
}

// #define U32TO8(p, v)
#[inline(always)]
unsafe fn u32to8(p: *mut u8, v: u32) {
    *p.add(0) = (v >> 24) as u8;
    *p.add(1) = (v >> 16) as u8;
    *p.add(2) = (v >> 8) as u8;
    *p.add(3) = v as u8;
}

// #define U8TO64(p) (((uint64_t)U8TO32(p) << 32) | ((uint64_t)U8TO32(p + 4)))
#[inline(always)]
unsafe fn u8to64(p: *const u8) -> u64 {
    ((u8to32(p) as u64) << 32) | (u8to32(p.add(4)) as u64)
}

// #define U64TO8(p, v)
#[inline(always)]
unsafe fn u64to8(p: *mut u8, v: u64) {
    u32to8(p, (v >> 32) as u32);
    u32to8(p.add(4), v as u32);
}

/// `const u64 cst[16]` in `lib/blake/src/blake512.c`.  Note the C definition is
/// NOT `static`, so it is an exported (read-only) global of the `blake` shared
/// library; the BLAKE-256 `cst` in `blake256.c` *is* `static` and therefore has
/// no linker symbol.  Keep the `cst` linker name so `nm -D` matches.
#[unsafe(no_mangle)]
pub static cst: [u64; 16] = CST;

static CST: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
];

static PADDING: [u8; 129] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

// #define BLAKE512_ROT(x,n) (((x)<<(64-n))|( (x)>>(n)))
#[inline(always)]
fn blake512_rot(x: u64, n: u32) -> u64 {
    (x << (64 - n)) | (x >> n)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_compress(S: *mut blakestate512, block: *const u8) {
    let m0: u64;
    let m1: u64;
    let m2: u64;
    let m3: u64;
    let m4: u64;
    let m5: u64;
    let m6: u64;
    let m7: u64;
    let m8: u64;
    let m9: u64;
    let m10: u64;
    let m11: u64;
    let m12: u64;
    let m13: u64;
    let m14: u64;
    let m15: u64;
    let mut v0: u64;
    let mut v1: u64;
    let mut v2: u64;
    let mut v3: u64;
    let mut v4: u64;
    let mut v5: u64;
    let mut v6: u64;
    let mut v7: u64;
    let mut v8: u64;
    let mut v9: u64;
    let mut v10: u64;
    let mut v11: u64;
    let mut v12: u64;
    let mut v13: u64;
    let mut v14: u64;
    let mut v15: u64;

    m0 = u8to64(block.add(0));
    m1 = u8to64(block.add(8));
    m2 = u8to64(block.add(16));
    m3 = u8to64(block.add(24));
    m4 = u8to64(block.add(32));
    m5 = u8to64(block.add(40));
    m6 = u8to64(block.add(48));
    m7 = u8to64(block.add(56));
    m8 = u8to64(block.add(64));
    m9 = u8to64(block.add(72));
    m10 = u8to64(block.add(80));
    m11 = u8to64(block.add(88));
    m12 = u8to64(block.add(96));
    m13 = u8to64(block.add(104));
    m14 = u8to64(block.add(112));
    m15 = u8to64(block.add(120));
    v0 = (*S).h[0];
    v1 = (*S).h[1];
    v2 = (*S).h[2];
    v3 = (*S).h[3];
    v4 = (*S).h[4];
    v5 = (*S).h[5];
    v6 = (*S).h[6];
    v7 = (*S).h[7];
    v8 = (*S).s[0] ^ 0x243F6A8885A308D3;
    v9 = (*S).s[1] ^ 0x13198A2E03707344;
    v10 = (*S).s[2] ^ 0xA4093822299F31D0;
    v11 = (*S).s[3] ^ 0x082EFA98EC4E6C89;
    v12 = 0x452821E638D01377;
    v13 = 0xBE5466CF34E90C6C;
    v14 = 0xC0AC29B7C97C50DD;
    v15 = 0x3F84D5B5B5470917;

    if (*S).nullt == 0 {
        v12 ^= (*S).t[0];
        v13 ^= (*S).t[0];
        v14 ^= (*S).t[1];
        v15 ^= (*S).t[1];
    }

    macro_rules! round {
        ($m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
         $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
         $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
         $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
            v0 = v0.wrapping_add($m0 ^ $c0);
            v0 = v0.wrapping_add(v4);
            v12 ^= v0;
            v12 = blake512_rot(v12, 32);
            v8 = v8.wrapping_add(v12);
            v4 ^= v8;
            v4 = blake512_rot(v4, 25);
            v1 = v1.wrapping_add($m2 ^ $c2);
            v1 = v1.wrapping_add(v5);
            v13 ^= v1;
            v13 = blake512_rot(v13, 32);
            v9 = v9.wrapping_add(v13);
            v5 ^= v9;
            v5 = blake512_rot(v5, 25);
            v2 = v2.wrapping_add($m4 ^ $c4);
            v2 = v2.wrapping_add(v6);
            v14 ^= v2;
            v14 = blake512_rot(v14, 32);
            v10 = v10.wrapping_add(v14);
            v6 ^= v10;
            v6 = blake512_rot(v6, 25);
            v3 = v3.wrapping_add($m6 ^ $c6);
            v3 = v3.wrapping_add(v7);
            v15 ^= v3;
            v15 = blake512_rot(v15, 32);
            v11 = v11.wrapping_add(v15);
            v7 ^= v11;
            v7 = blake512_rot(v7, 25);
            v2 = v2.wrapping_add($m5 ^ $c5);
            v2 = v2.wrapping_add(v6);
            v14 ^= v2;
            v14 = blake512_rot(v14, 16);
            v10 = v10.wrapping_add(v14);
            v6 ^= v10;
            v6 = blake512_rot(v6, 11);
            v3 = v3.wrapping_add($m7 ^ $c7);
            v3 = v3.wrapping_add(v7);
            v15 ^= v3;
            v15 = blake512_rot(v15, 16);
            v11 = v11.wrapping_add(v15);
            v7 ^= v11;
            v7 = blake512_rot(v7, 11);
            v1 = v1.wrapping_add($m3 ^ $c3);
            v1 = v1.wrapping_add(v5);
            v13 ^= v1;
            v13 = blake512_rot(v13, 16);
            v9 = v9.wrapping_add(v13);
            v5 ^= v9;
            v5 = blake512_rot(v5, 11);
            v0 = v0.wrapping_add($m1 ^ $c1);
            v0 = v0.wrapping_add(v4);
            v12 ^= v0;
            v12 = blake512_rot(v12, 16);
            v8 = v8.wrapping_add(v12);
            v4 ^= v8;
            v4 = blake512_rot(v4, 11);
            v0 = v0.wrapping_add($m8 ^ $c8);
            v0 = v0.wrapping_add(v5);
            v15 ^= v0;
            v15 = blake512_rot(v15, 32);
            v10 = v10.wrapping_add(v15);
            v5 ^= v10;
            v5 = blake512_rot(v5, 25);
            v1 = v1.wrapping_add($m10 ^ $c10);
            v1 = v1.wrapping_add(v6);
            v12 ^= v1;
            v12 = blake512_rot(v12, 32);
            v11 = v11.wrapping_add(v12);
            v6 ^= v11;
            v6 = blake512_rot(v6, 25);
            v2 = v2.wrapping_add($m12 ^ $c12);
            v2 = v2.wrapping_add(v7);
            v13 ^= v2;
            v13 = blake512_rot(v13, 32);
            v8 = v8.wrapping_add(v13);
            v7 ^= v8;
            v7 = blake512_rot(v7, 25);
            v3 = v3.wrapping_add($m14 ^ $c14);
            v3 = v3.wrapping_add(v4);
            v14 ^= v3;
            v14 = blake512_rot(v14, 32);
            v9 = v9.wrapping_add(v14);
            v4 ^= v9;
            v4 = blake512_rot(v4, 25);
            v2 = v2.wrapping_add($m13 ^ $c13);
            v2 = v2.wrapping_add(v7);
            v13 ^= v2;
            v13 = blake512_rot(v13, 16);
            v8 = v8.wrapping_add(v13);
            v7 ^= v8;
            v7 = blake512_rot(v7, 11);
            v3 = v3.wrapping_add($m15 ^ $c15);
            v3 = v3.wrapping_add(v4);
            v14 ^= v3;
            v14 = blake512_rot(v14, 16);
            v9 = v9.wrapping_add(v14);
            v4 ^= v9;
            v4 = blake512_rot(v4, 11);
            v1 = v1.wrapping_add($m11 ^ $c11);
            v1 = v1.wrapping_add(v6);
            v12 ^= v1;
            v12 = blake512_rot(v12, 16);
            v11 = v11.wrapping_add(v12);
            v6 ^= v11;
            v6 = blake512_rot(v6, 11);
            v0 = v0.wrapping_add($m9 ^ $c9);
            v0 = v0.wrapping_add(v5);
            v15 ^= v0;
            v15 = blake512_rot(v15, 16);
            v10 = v10.wrapping_add(v15);
            v5 ^= v10;
            v5 = blake512_rot(v5, 11);
        };
    }

    round!(m0, CST[1], m1, CST[0], m2, CST[3], m3, CST[2], m4, CST[5], m5, CST[4], m6, CST[7], m7, CST[6], m8, CST[9], m9, CST[8], m10, CST[11], m11, CST[10], m12, CST[13], m13, CST[12], m14, CST[15], m15, CST[14]);
    round!(m14, CST[10], m10, CST[14], m4, CST[8], m8, CST[4], m9, CST[15], m15, CST[9], m13, CST[6], m6, CST[13], m1, CST[12], m12, CST[1], m0, CST[2], m2, CST[0], m11, CST[7], m7, CST[11], m5, CST[3], m3, CST[5]);
    round!(m11, CST[8], m8, CST[11], m12, CST[0], m0, CST[12], m5, CST[2], m2, CST[5], m15, CST[13], m13, CST[15], m10, CST[14], m14, CST[10], m3, CST[6], m6, CST[3], m7, CST[1], m1, CST[7], m9, CST[4], m4, CST[9]);
    round!(m7, CST[9], m9, CST[7], m3, CST[1], m1, CST[3], m13, CST[12], m12, CST[13], m11, CST[14], m14, CST[11], m2, CST[6], m6, CST[2], m5, CST[10], m10, CST[5], m4, CST[0], m0, CST[4], m15, CST[8], m8, CST[15]);
    round!(m9, CST[0], m0, CST[9], m5, CST[7], m7, CST[5], m2, CST[4], m4, CST[2], m10, CST[15], m15, CST[10], m14, CST[1], m1, CST[14], m11, CST[12], m12, CST[11], m6, CST[8], m8, CST[6], m3, CST[13], m13, CST[3]);
    round!(m2, CST[12], m12, CST[2], m6, CST[10], m10, CST[6], m0, CST[11], m11, CST[0], m8, CST[3], m3, CST[8], m4, CST[13], m13, CST[4], m7, CST[5], m5, CST[7], m15, CST[14], m14, CST[15], m1, CST[9], m9, CST[1]);
    round!(m12, CST[5], m5, CST[12], m1, CST[15], m15, CST[1], m14, CST[13], m13, CST[14], m4, CST[10], m10, CST[4], m0, CST[7], m7, CST[0], m6, CST[3], m3, CST[6], m9, CST[2], m2, CST[9], m8, CST[11], m11, CST[8]);
    round!(m13, CST[11], m11, CST[13], m7, CST[14], m14, CST[7], m12, CST[1], m1, CST[12], m3, CST[9], m9, CST[3], m5, CST[0], m0, CST[5], m15, CST[4], m4, CST[15], m8, CST[6], m6, CST[8], m2, CST[10], m10, CST[2]);
    round!(m6, CST[15], m15, CST[6], m14, CST[9], m9, CST[14], m11, CST[3], m3, CST[11], m0, CST[8], m8, CST[0], m12, CST[2], m2, CST[12], m13, CST[7], m7, CST[13], m1, CST[4], m4, CST[1], m10, CST[5], m5, CST[10]);
    round!(m10, CST[2], m2, CST[10], m8, CST[4], m4, CST[8], m7, CST[6], m6, CST[7], m1, CST[5], m5, CST[1], m15, CST[11], m11, CST[15], m9, CST[14], m14, CST[9], m3, CST[12], m12, CST[3], m13, CST[0], m0, CST[13]);
    round!(m0, CST[1], m1, CST[0], m2, CST[3], m3, CST[2], m4, CST[5], m5, CST[4], m6, CST[7], m7, CST[6], m8, CST[9], m9, CST[8], m10, CST[11], m11, CST[10], m12, CST[13], m13, CST[12], m14, CST[15], m15, CST[14]);
    round!(m14, CST[10], m10, CST[14], m4, CST[8], m8, CST[4], m9, CST[15], m15, CST[9], m13, CST[6], m6, CST[13], m1, CST[12], m12, CST[1], m0, CST[2], m2, CST[0], m11, CST[7], m7, CST[11], m5, CST[3], m3, CST[5]);
    round!(m11, CST[8], m8, CST[11], m12, CST[0], m0, CST[12], m5, CST[2], m2, CST[5], m15, CST[13], m13, CST[15], m10, CST[14], m14, CST[10], m3, CST[6], m6, CST[3], m7, CST[1], m1, CST[7], m9, CST[4], m4, CST[9]);
    round!(m7, CST[9], m9, CST[7], m3, CST[1], m1, CST[3], m13, CST[12], m12, CST[13], m11, CST[14], m14, CST[11], m2, CST[6], m6, CST[2], m5, CST[10], m10, CST[5], m4, CST[0], m0, CST[4], m15, CST[8], m8, CST[15]);
    round!(m9, CST[0], m0, CST[9], m5, CST[7], m7, CST[5], m2, CST[4], m4, CST[2], m10, CST[15], m15, CST[10], m14, CST[1], m1, CST[14], m11, CST[12], m12, CST[11], m6, CST[8], m8, CST[6], m3, CST[13], m13, CST[3]);
    round!(m2, CST[12], m12, CST[2], m6, CST[10], m10, CST[6], m0, CST[11], m11, CST[0], m8, CST[3], m3, CST[8], m4, CST[13], m13, CST[4], m7, CST[5], m5, CST[7], m15, CST[14], m14, CST[15], m1, CST[9], m9, CST[1]);

    v0 ^= v8;
    v1 ^= v9;
    v2 ^= v10;
    v3 ^= v11;
    v4 ^= v12;
    v5 ^= v13;
    v6 ^= v14;
    v7 ^= v15;

    v0 ^= (*S).s[0];
    v1 ^= (*S).s[1];
    v2 ^= (*S).s[2];
    v3 ^= (*S).s[3];
    v4 ^= (*S).s[0];
    v5 ^= (*S).s[1];
    v6 ^= (*S).s[2];
    v7 ^= (*S).s[3];

    (*S).h[0] ^= v0;
    (*S).h[1] ^= v1;
    (*S).h[2] ^= v2;
    (*S).h[3] ^= v3;
    (*S).h[4] ^= v4;
    (*S).h[5] ^= v5;
    (*S).h[6] ^= v6;
    (*S).h[7] ^= v7;
}

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
    (*S).t[0] = 0;
    (*S).t[1] = 0;
    (*S).buflen = 0;
    (*S).nullt = 0;
    (*S).s[0] = 0;
    (*S).s[1] = 0;
    (*S).s[2] = 0;
    (*S).s[3] = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_update(
    S: *mut blakestate512,
    in_: *const u8,
    inlen: c_ulonglong,
) {
    let mut data: *const u8 = in_;
    let mut datalen: u64 = inlen;

    let mut left: c_int = (*S).buflen >> 3;
    let fill: c_int = 128 - left;

    if left != 0 && (((datalen >> 3) & 0x7F) >= fill as u64) {
        core::ptr::copy_nonoverlapping(data, (*S).buf.as_mut_ptr().add(left as usize), fill as usize);
        (*S).t[0] = (*S).t[0].wrapping_add(1024);
        blake512_compress(S, (*S).buf.as_ptr());
        data = data.add(fill as usize);
        datalen = datalen.wrapping_sub((fill << 3) as u64);
        left = 0;
    }

    while datalen >= 1024 {
        (*S).t[0] = (*S).t[0].wrapping_add(1024);
        blake512_compress(S, data);
        data = data.add(128);
        datalen -= 1024;
    }

    if datalen > 0 {
        core::ptr::copy_nonoverlapping(
            data,
            (*S).buf.as_mut_ptr().add(left as usize),
            ((datalen >> 3) & 0x7F) as usize,
        );
        (*S).buflen = ((left << 3) as u64 + datalen) as c_int;
    } else {
        (*S).buflen = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_final(S: *mut blakestate512, out: *mut u8) {
    let mut msglen: [u8; 16] = [0; 16];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo: u64 = (*S).t[0].wrapping_add((*S).buflen as u64);
    let mut hi: u64 = (*S).t[1];
    if lo < (*S).buflen as u64 {
        hi = hi.wrapping_add(1);
    }
    u64to8(msglen.as_mut_ptr().add(0), hi);
    u64to8(msglen.as_mut_ptr().add(8), lo);

    if (*S).buflen == 888 {
        /* one padding byte */
        (*S).t[0] = (*S).t[0].wrapping_sub(8);
        blake512_update(S, &oo, 8);
    } else {
        if (*S).buflen < 888 {
            /* enough space to fill the block */
            if (*S).buflen == 0 {
                (*S).nullt = 1;
            }
            (*S).t[0] = (*S).t[0].wrapping_sub((888 - (*S).buflen) as u64);
            blake512_update(S, PADDING.as_ptr(), (888 - (*S).buflen) as c_ulonglong);
        } else {
            /* NOT enough space, need 2 compressions */
            (*S).t[0] = (*S).t[0].wrapping_sub((1024 - (*S).buflen) as u64);
            blake512_update(S, PADDING.as_ptr(), (1024 - (*S).buflen) as c_ulonglong);
            (*S).t[0] = (*S).t[0].wrapping_sub(888);
            blake512_update(S, PADDING.as_ptr().add(1), 888);
            (*S).nullt = 1;
        }
        blake512_update(S, &zo, 8);
        (*S).t[0] = (*S).t[0].wrapping_sub(8);
    }
    (*S).t[0] = (*S).t[0].wrapping_sub(128);
    blake512_update(S, msglen.as_ptr(), 128);

    u64to8(out.add(0), (*S).h[0]);
    u64to8(out.add(8), (*S).h[1]);
    u64to8(out.add(16), (*S).h[2]);
    u64to8(out.add(24), (*S).h[3]);
    u64to8(out.add(32), (*S).h[4]);
    u64to8(out.add(40), (*S).h[5]);
    u64to8(out.add(48), (*S).h[6]);
    u64to8(out.add(56), (*S).h[7]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512(
    out: *mut u8,
    in_: *const u8,
    inlen: c_ulonglong,
) -> c_int {
    let mut S = blakestate512::new();
    blake512_init(&mut S);
    blake512_update(&mut S, in_, inlen.wrapping_mul(8));
    blake512_final(&mut S, out);
    0
}

/* `SPX_BLAKE512_OUTPUT_BYTES` from lib/blake/include/blake.h. */
pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

/// `blake512_mgf1` from blake512.c -- namespaced by blake.h, so the linker
/// symbol is `SPX_blake512_mgf1`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_blake512_mgf1(
    mut out: *mut u8,
    outlen: core::ffi::c_ulong,
    in_: *const u8,
    inlen: core::ffi::c_ulong,
) {
    let inlen_us = inlen as usize;
    let mut inbuf = vec![0u8; inlen_us + 4];
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut i: core::ffi::c_ulong;

    core::ptr::copy_nonoverlapping(in_, inbuf.as_mut_ptr(), inlen_us);

    /* While we can fit in at least another full block of BLAKE512 output.. */
    i = 0;
    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES as core::ffi::c_ulong <= outlen {
        crate::utils::SPX_u32_to_bytes(inbuf.as_mut_ptr().add(inlen_us), i as u32);
        blake512(out, inbuf.as_ptr(), (inlen_us + 4) as c_ulonglong);
        out = out.add(SPX_BLAKE512_OUTPUT_BYTES);
        i += 1;
    }
    /* Until we cannot anymore, and we fill the remainder. */
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES as core::ffi::c_ulong {
        crate::utils::SPX_u32_to_bytes(inbuf.as_mut_ptr().add(inlen_us), i as u32);
        blake512(outbuf.as_mut_ptr(), inbuf.as_ptr(), (inlen_us + 4) as c_ulonglong);
        core::ptr::copy_nonoverlapping(
            outbuf.as_ptr(),
            out,
            (outlen - i * SPX_BLAKE512_OUTPUT_BYTES as core::ffi::c_ulong) as usize,
        );
    }
}
