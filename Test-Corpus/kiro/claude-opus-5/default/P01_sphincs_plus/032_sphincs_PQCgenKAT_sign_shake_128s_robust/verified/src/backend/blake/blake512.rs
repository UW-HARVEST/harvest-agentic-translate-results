// Translated from c_src/lib/blake/src/blake512.c
// BLAKE reference C implementation, Copyright (c) 2012 Jean-Philippe Aumasson.
// Bit-exact translation, preserving all original quirks and bugs.

use crate::utils::u32_to_bytes;

pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

#[allow(non_upper_case_globals, dead_code)]
const CST: [u64; 16] = [
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

/// `blake512.c` declares `const u64 cst[16]` **without** `static`, so the
/// reference `libblake.so` exports it as a read-only data symbol named `cst`
/// (unlike `blake256.c`'s `static const u32 cst[16]`, which stays internal).
/// Mirror that export so the dynamic symbol tables match.
#[unsafe(no_mangle)]
pub static cst: [u64; 16] = CST;

#[allow(non_upper_case_globals, dead_code)]
const PADDING: [u8; 129] = {
    let mut p = [0u8; 129];
    p[0] = 0x80;
    p
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BlakeState512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

impl BlakeState512 {
    pub const fn new() -> Self {
        BlakeState512 {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 128],
        }
    }
}

impl Default for BlakeState512 {
    fn default() -> Self {
        Self::new()
    }
}

// #define U8TO32(p) big-endian read of 4 bytes
#[inline(always)]
fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}

// #define U8TO64(p) (((uint64_t)U8TO32(p) << 32) | ((uint64_t)U8TO32(p + 4)))
#[inline(always)]
fn u8to64(p: &[u8]) -> u64 {
    ((u8to32(p) as u64) << 32) | (u8to32(&p[4..]) as u64)
}

// #define U32TO8(p, v) big-endian store of 4 bytes
#[inline(always)]
fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

// #define U64TO8(p, v)  -> two U32TO8 halves
#[inline(always)]
fn u64to8(p: &mut [u8], v: u64) {
    u32to8(p, (v >> 32) as u32);
    u32to8(&mut p[4..], v as u32);
}

// BLAKE512_ROT(x,n) == x.rotate_right(n) on u64
macro_rules! rot {
    ($x:expr, $n:expr) => {
        ($x).rotate_right($n)
    };
}

pub fn blake512_compress_rs(s: &mut BlakeState512, block: &[u8]) {
    let mut m: [u64; 16] = [0; 16];
    for i in 0..16 {
        m[i] = u8to64(&block[i * 8..]);
    }

    let mut v0: u64 = s.h[0];
    let mut v1: u64 = s.h[1];
    let mut v2: u64 = s.h[2];
    let mut v3: u64 = s.h[3];
    let mut v4: u64 = s.h[4];
    let mut v5: u64 = s.h[5];
    let mut v6: u64 = s.h[6];
    let mut v7: u64 = s.h[7];
    let mut v8: u64 = s.s[0] ^ 0x243F6A8885A308D3;
    let mut v9: u64 = s.s[1] ^ 0x13198A2E03707344;
    let mut v10: u64 = s.s[2] ^ 0xA4093822299F31D0;
    let mut v11: u64 = s.s[3] ^ 0x082EFA98EC4E6C89;
    let mut v12: u64 = 0x452821E638D01377;
    let mut v13: u64 = 0xBE5466CF34E90C6C;
    let mut v14: u64 = 0xC0AC29B7C97C50DD;
    let mut v15: u64 = 0x3F84D5B5B5470917;

    if s.nullt == 0 {
        v12 ^= s.t[0];
        v13 ^= s.t[0];
        v14 ^= s.t[1];
        v15 ^= s.t[1];
    }

    // Mirror of the C ROUND macro. Argument order matches the C macro exactly.
    macro_rules! round {
        ($m0:expr,$c0:expr,$m1:expr,$c1:expr,$m2:expr,$c2:expr,$m3:expr,$c3:expr,
         $m4:expr,$c4:expr,$m5:expr,$c5:expr,$m6:expr,$c6:expr,$m7:expr,$c7:expr,
         $m8:expr,$c8:expr,$m9:expr,$c9:expr,$m10:expr,$c10:expr,$m11:expr,$c11:expr,
         $m12:expr,$c12:expr,$m13:expr,$c13:expr,$m14:expr,$c14:expr,$m15:expr,$c15:expr) => {
            v0 = v0.wrapping_add($m0 ^ $c0);
            v0 = v0.wrapping_add(v4);
            v12 ^= v0;
            v12 = rot!(v12, 32);
            v8 = v8.wrapping_add(v12);
            v4 ^= v8;
            v4 = rot!(v4, 25);
            v1 = v1.wrapping_add($m2 ^ $c2);
            v1 = v1.wrapping_add(v5);
            v13 ^= v1;
            v13 = rot!(v13, 32);
            v9 = v9.wrapping_add(v13);
            v5 ^= v9;
            v5 = rot!(v5, 25);
            v2 = v2.wrapping_add($m4 ^ $c4);
            v2 = v2.wrapping_add(v6);
            v14 ^= v2;
            v14 = rot!(v14, 32);
            v10 = v10.wrapping_add(v14);
            v6 ^= v10;
            v6 = rot!(v6, 25);
            v3 = v3.wrapping_add($m6 ^ $c6);
            v3 = v3.wrapping_add(v7);
            v15 ^= v3;
            v15 = rot!(v15, 32);
            v11 = v11.wrapping_add(v15);
            v7 ^= v11;
            v7 = rot!(v7, 25);
            v2 = v2.wrapping_add($m5 ^ $c5);
            v2 = v2.wrapping_add(v6);
            v14 ^= v2;
            v14 = rot!(v14, 16);
            v10 = v10.wrapping_add(v14);
            v6 ^= v10;
            v6 = rot!(v6, 11);
            v3 = v3.wrapping_add($m7 ^ $c7);
            v3 = v3.wrapping_add(v7);
            v15 ^= v3;
            v15 = rot!(v15, 16);
            v11 = v11.wrapping_add(v15);
            v7 ^= v11;
            v7 = rot!(v7, 11);
            v1 = v1.wrapping_add($m3 ^ $c3);
            v1 = v1.wrapping_add(v5);
            v13 ^= v1;
            v13 = rot!(v13, 16);
            v9 = v9.wrapping_add(v13);
            v5 ^= v9;
            v5 = rot!(v5, 11);
            v0 = v0.wrapping_add($m1 ^ $c1);
            v0 = v0.wrapping_add(v4);
            v12 ^= v0;
            v12 = rot!(v12, 16);
            v8 = v8.wrapping_add(v12);
            v4 ^= v8;
            v4 = rot!(v4, 11);
            v0 = v0.wrapping_add($m8 ^ $c8);
            v0 = v0.wrapping_add(v5);
            v15 ^= v0;
            v15 = rot!(v15, 32);
            v10 = v10.wrapping_add(v15);
            v5 ^= v10;
            v5 = rot!(v5, 25);
            v1 = v1.wrapping_add($m10 ^ $c10);
            v1 = v1.wrapping_add(v6);
            v12 ^= v1;
            v12 = rot!(v12, 32);
            v11 = v11.wrapping_add(v12);
            v6 ^= v11;
            v6 = rot!(v6, 25);
            v2 = v2.wrapping_add($m12 ^ $c12);
            v2 = v2.wrapping_add(v7);
            v13 ^= v2;
            v13 = rot!(v13, 32);
            v8 = v8.wrapping_add(v13);
            v7 ^= v8;
            v7 = rot!(v7, 25);
            v3 = v3.wrapping_add($m14 ^ $c14);
            v3 = v3.wrapping_add(v4);
            v14 ^= v3;
            v14 = rot!(v14, 32);
            v9 = v9.wrapping_add(v14);
            v4 ^= v9;
            v4 = rot!(v4, 25);
            v2 = v2.wrapping_add($m13 ^ $c13);
            v2 = v2.wrapping_add(v7);
            v13 ^= v2;
            v13 = rot!(v13, 16);
            v8 = v8.wrapping_add(v13);
            v7 ^= v8;
            v7 = rot!(v7, 11);
            v3 = v3.wrapping_add($m15 ^ $c15);
            v3 = v3.wrapping_add(v4);
            v14 ^= v3;
            v14 = rot!(v14, 16);
            v9 = v9.wrapping_add(v14);
            v4 ^= v9;
            v4 = rot!(v4, 11);
            v1 = v1.wrapping_add($m11 ^ $c11);
            v1 = v1.wrapping_add(v6);
            v12 ^= v1;
            v12 = rot!(v12, 16);
            v11 = v11.wrapping_add(v12);
            v6 ^= v11;
            v6 = rot!(v6, 11);
            v0 = v0.wrapping_add($m9 ^ $c9);
            v0 = v0.wrapping_add(v5);
            v15 ^= v0;
            v15 = rot!(v15, 16);
            v10 = v10.wrapping_add(v15);
            v5 ^= v10;
            v5 = rot!(v5, 11);
        };
    }

    let m0 = m[0];
    let m1 = m[1];
    let m2 = m[2];
    let m3 = m[3];
    let m4 = m[4];
    let m5 = m[5];
    let m6 = m[6];
    let m7 = m[7];
    let m8 = m[8];
    let m9 = m[9];
    let m10 = m[10];
    let m11 = m[11];
    let m12 = m[12];
    let m13 = m[13];
    let m14 = m[14];
    let m15 = m[15];

    round!(m0,CST[1],m1,CST[0],m2,CST[3],m3,CST[2],m4,CST[5],m5,CST[4],m6,CST[7],m7,CST[6],m8,CST[9],m9,CST[8],m10,CST[11],m11,CST[10],m12,CST[13],m13,CST[12],m14,CST[15],m15,CST[14]);
    round!(m14,CST[10],m10,CST[14],m4,CST[8],m8,CST[4],m9,CST[15],m15,CST[9],m13,CST[6],m6,CST[13],m1,CST[12],m12,CST[1],m0,CST[2],m2,CST[0],m11,CST[7],m7,CST[11],m5,CST[3],m3,CST[5]);
    round!(m11,CST[8],m8,CST[11],m12,CST[0],m0,CST[12],m5,CST[2],m2,CST[5],m15,CST[13],m13,CST[15],m10,CST[14],m14,CST[10],m3,CST[6],m6,CST[3],m7,CST[1],m1,CST[7],m9,CST[4],m4,CST[9]);
    round!(m7,CST[9],m9,CST[7],m3,CST[1],m1,CST[3],m13,CST[12],m12,CST[13],m11,CST[14],m14,CST[11],m2,CST[6],m6,CST[2],m5,CST[10],m10,CST[5],m4,CST[0],m0,CST[4],m15,CST[8],m8,CST[15]);
    round!(m9,CST[0],m0,CST[9],m5,CST[7],m7,CST[5],m2,CST[4],m4,CST[2],m10,CST[15],m15,CST[10],m14,CST[1],m1,CST[14],m11,CST[12],m12,CST[11],m6,CST[8],m8,CST[6],m3,CST[13],m13,CST[3]);
    round!(m2,CST[12],m12,CST[2],m6,CST[10],m10,CST[6],m0,CST[11],m11,CST[0],m8,CST[3],m3,CST[8],m4,CST[13],m13,CST[4],m7,CST[5],m5,CST[7],m15,CST[14],m14,CST[15],m1,CST[9],m9,CST[1]);
    round!(m12,CST[5],m5,CST[12],m1,CST[15],m15,CST[1],m14,CST[13],m13,CST[14],m4,CST[10],m10,CST[4],m0,CST[7],m7,CST[0],m6,CST[3],m3,CST[6],m9,CST[2],m2,CST[9],m8,CST[11],m11,CST[8]);
    round!(m13,CST[11],m11,CST[13],m7,CST[14],m14,CST[7],m12,CST[1],m1,CST[12],m3,CST[9],m9,CST[3],m5,CST[0],m0,CST[5],m15,CST[4],m4,CST[15],m8,CST[6],m6,CST[8],m2,CST[10],m10,CST[2]);
    round!(m6,CST[15],m15,CST[6],m14,CST[9],m9,CST[14],m11,CST[3],m3,CST[11],m0,CST[8],m8,CST[0],m12,CST[2],m2,CST[12],m13,CST[7],m7,CST[13],m1,CST[4],m4,CST[1],m10,CST[5],m5,CST[10]);
    round!(m10,CST[2],m2,CST[10],m8,CST[4],m4,CST[8],m7,CST[6],m6,CST[7],m1,CST[5],m5,CST[1],m15,CST[11],m11,CST[15],m9,CST[14],m14,CST[9],m3,CST[12],m12,CST[3],m13,CST[0],m0,CST[13]);
    round!(m0,CST[1],m1,CST[0],m2,CST[3],m3,CST[2],m4,CST[5],m5,CST[4],m6,CST[7],m7,CST[6],m8,CST[9],m9,CST[8],m10,CST[11],m11,CST[10],m12,CST[13],m13,CST[12],m14,CST[15],m15,CST[14]);
    round!(m14,CST[10],m10,CST[14],m4,CST[8],m8,CST[4],m9,CST[15],m15,CST[9],m13,CST[6],m6,CST[13],m1,CST[12],m12,CST[1],m0,CST[2],m2,CST[0],m11,CST[7],m7,CST[11],m5,CST[3],m3,CST[5]);
    round!(m11,CST[8],m8,CST[11],m12,CST[0],m0,CST[12],m5,CST[2],m2,CST[5],m15,CST[13],m13,CST[15],m10,CST[14],m14,CST[10],m3,CST[6],m6,CST[3],m7,CST[1],m1,CST[7],m9,CST[4],m4,CST[9]);
    round!(m7,CST[9],m9,CST[7],m3,CST[1],m1,CST[3],m13,CST[12],m12,CST[13],m11,CST[14],m14,CST[11],m2,CST[6],m6,CST[2],m5,CST[10],m10,CST[5],m4,CST[0],m0,CST[4],m15,CST[8],m8,CST[15]);
    round!(m9,CST[0],m0,CST[9],m5,CST[7],m7,CST[5],m2,CST[4],m4,CST[2],m10,CST[15],m15,CST[10],m14,CST[1],m1,CST[14],m11,CST[12],m12,CST[11],m6,CST[8],m8,CST[6],m3,CST[13],m13,CST[3]);
    round!(m2,CST[12],m12,CST[2],m6,CST[10],m10,CST[6],m0,CST[11],m11,CST[0],m8,CST[3],m3,CST[8],m4,CST[13],m13,CST[4],m7,CST[5],m5,CST[7],m15,CST[14],m14,CST[15],m1,CST[9],m9,CST[1]);

    v0 ^= v8;
    v1 ^= v9;
    v2 ^= v10;
    v3 ^= v11;
    v4 ^= v12;
    v5 ^= v13;
    v6 ^= v14;
    v7 ^= v15;

    v0 ^= s.s[0];
    v1 ^= s.s[1];
    v2 ^= s.s[2];
    v3 ^= s.s[3];
    v4 ^= s.s[0];
    v5 ^= s.s[1];
    v6 ^= s.s[2];
    v7 ^= s.s[3];

    s.h[0] ^= v0;
    s.h[1] ^= v1;
    s.h[2] ^= v2;
    s.h[3] ^= v3;
    s.h[4] ^= v4;
    s.h[5] ^= v5;
    s.h[6] ^= v6;
    s.h[7] ^= v7;
}

pub fn blake512_init_rs(s: &mut BlakeState512) {
    s.h[0] = 0x6A09E667F3BCC908;
    s.h[1] = 0xBB67AE8584CAA73B;
    s.h[2] = 0x3C6EF372FE94F82B;
    s.h[3] = 0xA54FF53A5F1D36F1;
    s.h[4] = 0x510E527FADE682D1;
    s.h[5] = 0x9B05688C2B3E6C1F;
    s.h[6] = 0x1F83D9ABFB41BD6B;
    s.h[7] = 0x5BE0CD19137E2179;
    s.t[0] = 0;
    s.t[1] = 0;
    s.buflen = 0;
    s.nullt = 0;
    s.s[0] = 0;
    s.s[1] = 0;
    s.s[2] = 0;
    s.s[3] = 0;
}

/// datalen is in BITS
pub fn blake512_update_rs(s: &mut BlakeState512, data: &[u8], datalen: u64) {
    let mut datalen = datalen;
    let mut off: usize = 0; // offset into `data`, mirrors C pointer advancement

    let mut left: i32 = s.buflen >> 3;
    let fill: i32 = 128 - left;

    if left != 0 && (((datalen >> 3) & 0x7F) >= fill as u64) {
        // memcpy( S->buf + left, data, fill )
        s.buf[left as usize..(left as usize + fill as usize)]
            .copy_from_slice(&data[off..off + fill as usize]);
        s.t[0] = s.t[0].wrapping_add(1024);
        let buf = s.buf;
        blake512_compress_rs(s, &buf);
        off += fill as usize;
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 1024 {
        s.t[0] = s.t[0].wrapping_add(1024);
        blake512_compress_rs(s, &data[off..off + 128]);
        off += 128;
        datalen -= 1024;
    }

    if datalen > 0 {
        // memcpy( S->buf + left, data, (datalen>>3) & 0x7F )  -- WITH mask
        let n = ((datalen >> 3) & 0x7F) as usize;
        s.buf[left as usize..left as usize + n].copy_from_slice(&data[off..off + n]);
        s.buflen = (left << 3) + datalen as i32;
    } else {
        s.buflen = 0;
    }
}

pub fn blake512_final_rs(s: &mut BlakeState512, digest: &mut [u8]) {
    let mut msglen: [u8; 16] = [0; 16];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;

    let lo: u64 = s.t[0].wrapping_add(s.buflen as u64);
    let mut hi: u64 = s.t[1];
    if lo < (s.buflen as u64) {
        hi = hi.wrapping_add(1);
    }
    u64to8(&mut msglen[0..], hi);
    u64to8(&mut msglen[8..], lo);

    if s.buflen == 888 {
        // one padding byte
        s.t[0] = s.t[0].wrapping_sub(8);
        let b = [oo];
        blake512_update_rs(s, &b, 8);
    } else {
        if s.buflen < 888 {
            // enough space to fill the block
            if s.buflen == 0 {
                s.nullt = 1;
            }
            s.t[0] = s.t[0].wrapping_sub((888 - s.buflen) as u64);
            let plen = (888 - s.buflen) as u64;
            let pad = PADDING;
            blake512_update_rs(s, &pad, plen);
        } else {
            // NOT enough space, need 2 compressions
            s.t[0] = s.t[0].wrapping_sub((1024 - s.buflen) as u64);
            let plen = (1024 - s.buflen) as u64;
            let pad = PADDING;
            blake512_update_rs(s, &pad, plen);
            s.t[0] = s.t[0].wrapping_sub(888);
            let pad2 = PADDING;
            blake512_update_rs(s, &pad2[1..], 888);
            s.nullt = 1;
        }
        let b = [zo];
        blake512_update_rs(s, &b, 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(128);
    blake512_update_rs(s, &msglen, 128);

    u64to8(&mut digest[0..], s.h[0]);
    u64to8(&mut digest[8..], s.h[1]);
    u64to8(&mut digest[16..], s.h[2]);
    u64to8(&mut digest[24..], s.h[3]);
    u64to8(&mut digest[32..], s.h[4]);
    u64to8(&mut digest[40..], s.h[5]);
    u64to8(&mut digest[48..], s.h[6]);
    u64to8(&mut digest[56..], s.h[7]);
}

/// inlen is in BYTES (one-shot hash; internally calls update with inlen*8)
pub fn blake512_rs(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState512::new();
    blake512_init_rs(&mut s);
    blake512_update_rs(&mut s, inp, inlen.wrapping_mul(8));
    blake512_final_rs(&mut s, out);
    0
}

pub fn blake512_mgf1_rs(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    // VLA uint8_t inbuf[inlen+4]
    let mut inbuf: Vec<u8> = vec![0u8; inlen + 4];
    let mut outbuf: [u8; SPX_BLAKE512_OUTPUT_BYTES] = [0; SPX_BLAKE512_OUTPUT_BYTES];

    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut out_off: usize = 0;
    let mut i: usize = 0;
    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512_rs(&mut out[out_off..], &inbuf, (inlen + 4) as u64);
        out_off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512_rs(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i * SPX_BLAKE512_OUTPUT_BYTES;
        out[out_off..out_off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

// ---------------------------------------------------------------------------
// C-ABI wrappers
// ---------------------------------------------------------------------------

#[allow(non_snake_case, dead_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512(out: *mut u8, inp: *const u8, inlen: u64) -> core::ffi::c_int {
    let in_slice = core::slice::from_raw_parts(inp, inlen as usize);
    let out_slice = core::slice::from_raw_parts_mut(out, SPX_BLAKE512_OUTPUT_BYTES);
    blake512_rs(out_slice, in_slice, inlen) as core::ffi::c_int
}

#[allow(non_snake_case, dead_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_init(s: *mut BlakeState512) {
    blake512_init_rs(&mut *s);
}

#[allow(non_snake_case, dead_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_compress(s: *mut BlakeState512, block: *const u8) {
    let block_slice = core::slice::from_raw_parts(block, 128);
    blake512_compress_rs(&mut *s, block_slice);
}

#[allow(non_snake_case, dead_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_update(s: *mut BlakeState512, inp: *const u8, inlen: u64) {
    // inlen is the raw BIT count
    let in_slice = core::slice::from_raw_parts(inp, ((inlen >> 3) as usize) + 1);
    blake512_update_rs(&mut *s, in_slice, inlen);
}

#[allow(non_snake_case, dead_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blake512_final(s: *mut BlakeState512, out: *mut u8) {
    let out_slice = core::slice::from_raw_parts_mut(out, SPX_BLAKE512_OUTPUT_BYTES);
    blake512_final_rs(&mut *s, out_slice);
}

#[allow(non_snake_case, dead_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_blake512_mgf1(
    out: *mut u8,
    outlen: core::ffi::c_ulong,
    inp: *const u8,
    inlen: core::ffi::c_ulong,
) {
    let out_slice = core::slice::from_raw_parts_mut(out, outlen as usize);
    let in_slice = core::slice::from_raw_parts(inp, inlen as usize);
    blake512_mgf1_rs(out_slice, outlen as usize, in_slice, inlen as usize);
}
