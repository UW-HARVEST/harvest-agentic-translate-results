//! Translation of `lib/sha2/src/sha2.c`.
//!
//! Bit-exact port of the public-domain SHA-2 implementation used by
//! SPHINCS+. The structure of the original C is preserved: byte load/store
//! helpers, the two `crypto_hashblocks_*` compression functions, the IVs, the
//! incremental init/blocks/finalize functions, the one-shot `sha256`/`sha512`
//! functions, `mgf1_256`/`mgf1_512`, and `seed_state`.
//!
//! Based on the public domain implementation in
//! `crypto_hash/sha512/ref/` from <http://bench.cr.yp.to/supercop.html>
//! by D. J. Bernstein.

use crate::params::{SPX_N, SPX_WIDE};
use crate::utils::u32_to_bytes;

pub const SPX_SHA256_BLOCK_BYTES: usize = 64;
pub const SPX_SHA256_OUTPUT_BYTES: usize = 32;
pub const SPX_SHA512_BLOCK_BYTES: usize = 128;
pub const SPX_SHA512_OUTPUT_BYTES: usize = 64;
pub const SPX_SHA256_ADDR_BYTES: usize = 22;

// ---------------------------------------------------------------------------
// load/store helpers
// ---------------------------------------------------------------------------

#[inline]
fn load_bigendian_32(x: &[u8]) -> u32 {
    (x[3] as u32)
        | ((x[2] as u32) << 8)
        | ((x[1] as u32) << 16)
        | ((x[0] as u32) << 24)
}

#[inline]
fn load_bigendian_64(x: &[u8]) -> u64 {
    (x[7] as u64)
        | ((x[6] as u64) << 8)
        | ((x[5] as u64) << 16)
        | ((x[4] as u64) << 24)
        | ((x[3] as u64) << 32)
        | ((x[2] as u64) << 40)
        | ((x[1] as u64) << 48)
        | ((x[0] as u64) << 56)
}

/// Stores the low 4 bytes of `u` in big-endian order. Mirrors the C signature
/// which takes a `uint64_t` but only writes 4 bytes (truncation preserved).
#[inline]
fn store_bigendian_32(x: &mut [u8], mut u: u64) {
    x[3] = u as u8;
    u >>= 8;
    x[2] = u as u8;
    u >>= 8;
    x[1] = u as u8;
    u >>= 8;
    x[0] = u as u8;
}

#[inline]
fn store_bigendian_64(x: &mut [u8], mut u: u64) {
    x[7] = u as u8;
    u >>= 8;
    x[6] = u as u8;
    u >>= 8;
    x[5] = u as u8;
    u >>= 8;
    x[4] = u as u8;
    u >>= 8;
    x[3] = u as u8;
    u >>= 8;
    x[2] = u as u8;
    u >>= 8;
    x[1] = u as u8;
    u >>= 8;
    x[0] = u as u8;
}

// ---------------------------------------------------------------------------
// bit-op macros (mirroring the C preprocessor macros)
// ---------------------------------------------------------------------------

macro_rules! SHR {
    ($x:expr, $c:expr) => {
        ($x >> $c)
    };
}
macro_rules! ROTR_32 {
    ($x:expr, $c:expr) => {
        (($x >> $c) | ($x << (32 - $c)))
    };
}
macro_rules! ROTR_64 {
    ($x:expr, $c:expr) => {
        (($x >> $c) | ($x << (64 - $c)))
    };
}

macro_rules! Ch {
    ($x:expr, $y:expr, $z:expr) => {
        (($x & $y) ^ (!$x & $z))
    };
}
macro_rules! Maj {
    ($x:expr, $y:expr, $z:expr) => {
        (($x & $y) ^ ($x & $z) ^ ($y & $z))
    };
}

macro_rules! Sigma0_32 {
    ($x:expr) => {
        (ROTR_32!($x, 2) ^ ROTR_32!($x, 13) ^ ROTR_32!($x, 22))
    };
}
macro_rules! Sigma1_32 {
    ($x:expr) => {
        (ROTR_32!($x, 6) ^ ROTR_32!($x, 11) ^ ROTR_32!($x, 25))
    };
}
macro_rules! sigma0_32 {
    ($x:expr) => {
        (ROTR_32!($x, 7) ^ ROTR_32!($x, 18) ^ SHR!($x, 3))
    };
}
macro_rules! sigma1_32 {
    ($x:expr) => {
        (ROTR_32!($x, 17) ^ ROTR_32!($x, 19) ^ SHR!($x, 10))
    };
}

macro_rules! Sigma0_64 {
    ($x:expr) => {
        (ROTR_64!($x, 28) ^ ROTR_64!($x, 34) ^ ROTR_64!($x, 39))
    };
}
macro_rules! Sigma1_64 {
    ($x:expr) => {
        (ROTR_64!($x, 14) ^ ROTR_64!($x, 18) ^ ROTR_64!($x, 41))
    };
}
macro_rules! sigma0_64 {
    ($x:expr) => {
        (ROTR_64!($x, 1) ^ ROTR_64!($x, 8) ^ SHR!($x, 7))
    };
}
macro_rules! sigma1_64 {
    ($x:expr) => {
        (ROTR_64!($x, 19) ^ ROTR_64!($x, 61) ^ SHR!($x, 6))
    };
}

// M_32(w0, w14, w9, w1): w0 = sigma1_32(w14) + w9 + sigma0_32(w1) + w0
macro_rules! M_32 {
    ($w0:ident, $w14:ident, $w9:ident, $w1:ident) => {
        $w0 = sigma1_32!($w14)
            .wrapping_add($w9)
            .wrapping_add(sigma0_32!($w1))
            .wrapping_add($w0);
    };
}
macro_rules! M_64 {
    ($w0:ident, $w14:ident, $w9:ident, $w1:ident) => {
        $w0 = sigma1_64!($w14)
            .wrapping_add($w9)
            .wrapping_add(sigma0_64!($w1))
            .wrapping_add($w0);
    };
}

macro_rules! EXPAND_32 {
    ($w0:ident, $w1:ident, $w2:ident, $w3:ident, $w4:ident, $w5:ident, $w6:ident, $w7:ident,
     $w8:ident, $w9:ident, $w10:ident, $w11:ident, $w12:ident, $w13:ident, $w14:ident, $w15:ident) => {
        M_32!($w0, $w14, $w9, $w1);
        M_32!($w1, $w15, $w10, $w2);
        M_32!($w2, $w0, $w11, $w3);
        M_32!($w3, $w1, $w12, $w4);
        M_32!($w4, $w2, $w13, $w5);
        M_32!($w5, $w3, $w14, $w6);
        M_32!($w6, $w4, $w15, $w7);
        M_32!($w7, $w5, $w0, $w8);
        M_32!($w8, $w6, $w1, $w9);
        M_32!($w9, $w7, $w2, $w10);
        M_32!($w10, $w8, $w3, $w11);
        M_32!($w11, $w9, $w4, $w12);
        M_32!($w12, $w10, $w5, $w13);
        M_32!($w13, $w11, $w6, $w14);
        M_32!($w14, $w12, $w7, $w15);
        M_32!($w15, $w13, $w8, $w0);
    };
}

macro_rules! EXPAND_64 {
    ($w0:ident, $w1:ident, $w2:ident, $w3:ident, $w4:ident, $w5:ident, $w6:ident, $w7:ident,
     $w8:ident, $w9:ident, $w10:ident, $w11:ident, $w12:ident, $w13:ident, $w14:ident, $w15:ident) => {
        M_64!($w0, $w14, $w9, $w1);
        M_64!($w1, $w15, $w10, $w2);
        M_64!($w2, $w0, $w11, $w3);
        M_64!($w3, $w1, $w12, $w4);
        M_64!($w4, $w2, $w13, $w5);
        M_64!($w5, $w3, $w14, $w6);
        M_64!($w6, $w4, $w15, $w7);
        M_64!($w7, $w5, $w0, $w8);
        M_64!($w8, $w6, $w1, $w9);
        M_64!($w9, $w7, $w2, $w10);
        M_64!($w10, $w8, $w3, $w11);
        M_64!($w11, $w9, $w4, $w12);
        M_64!($w12, $w10, $w5, $w13);
        M_64!($w13, $w11, $w6, $w14);
        M_64!($w14, $w12, $w7, $w15);
        M_64!($w15, $w13, $w8, $w0);
    };
}

// F_32(w, k):
//   T1 = h + Sigma1_32(e) + Ch(e,f,g) + k + w;
//   T2 = Sigma0_32(a) + Maj(a,b,c);
//   h=g; g=f; f=e; e=d+T1; d=c; c=b; b=a; a=T1+T2;
macro_rules! F_32 {
    ($a:ident, $b:ident, $c:ident, $d:ident, $e:ident, $f:ident, $g:ident, $h:ident,
     $T1:ident, $T2:ident, $w:expr, $k:expr) => {
        $T1 = $h
            .wrapping_add(Sigma1_32!($e))
            .wrapping_add(Ch!($e, $f, $g))
            .wrapping_add($k)
            .wrapping_add($w);
        $T2 = Sigma0_32!($a).wrapping_add(Maj!($a, $b, $c));
        $h = $g;
        $g = $f;
        $f = $e;
        $e = $d.wrapping_add($T1);
        $d = $c;
        $c = $b;
        $b = $a;
        $a = $T1.wrapping_add($T2);
    };
}

macro_rules! F_64 {
    ($a:ident, $b:ident, $c:ident, $d:ident, $e:ident, $f:ident, $g:ident, $h:ident,
     $T1:ident, $T2:ident, $w:expr, $k:expr) => {
        $T1 = $h
            .wrapping_add(Sigma1_64!($e))
            .wrapping_add(Ch!($e, $f, $g))
            .wrapping_add($k)
            .wrapping_add($w);
        $T2 = Sigma0_64!($a).wrapping_add(Maj!($a, $b, $c));
        $h = $g;
        $g = $f;
        $f = $e;
        $e = $d.wrapping_add($T1);
        $d = $c;
        $c = $b;
        $b = $a;
        $a = $T1.wrapping_add($T2);
    };
}

// ---------------------------------------------------------------------------
// crypto_hashblocks_sha256
// ---------------------------------------------------------------------------

#[allow(non_snake_case)]
fn crypto_hashblocks_sha256(statebytes: &mut [u8], inp: &[u8], mut inlen: usize) -> usize {
    let mut state = [0u32; 8];
    let mut a: u32;
    let mut b: u32;
    let mut c: u32;
    let mut d: u32;
    let mut e: u32;
    let mut f: u32;
    let mut g: u32;
    let mut h: u32;
    let mut T1: u32;
    let mut T2: u32;

    a = load_bigendian_32(&statebytes[0..]);
    state[0] = a;
    b = load_bigendian_32(&statebytes[4..]);
    state[1] = b;
    c = load_bigendian_32(&statebytes[8..]);
    state[2] = c;
    d = load_bigendian_32(&statebytes[12..]);
    state[3] = d;
    e = load_bigendian_32(&statebytes[16..]);
    state[4] = e;
    f = load_bigendian_32(&statebytes[20..]);
    state[5] = f;
    g = load_bigendian_32(&statebytes[24..]);
    state[6] = g;
    h = load_bigendian_32(&statebytes[28..]);
    state[7] = h;

    let mut off = 0usize;
    while inlen >= 64 {
        let mut w0 = load_bigendian_32(&inp[off + 0..]);
        let mut w1 = load_bigendian_32(&inp[off + 4..]);
        let mut w2 = load_bigendian_32(&inp[off + 8..]);
        let mut w3 = load_bigendian_32(&inp[off + 12..]);
        let mut w4 = load_bigendian_32(&inp[off + 16..]);
        let mut w5 = load_bigendian_32(&inp[off + 20..]);
        let mut w6 = load_bigendian_32(&inp[off + 24..]);
        let mut w7 = load_bigendian_32(&inp[off + 28..]);
        let mut w8 = load_bigendian_32(&inp[off + 32..]);
        let mut w9 = load_bigendian_32(&inp[off + 36..]);
        let mut w10 = load_bigendian_32(&inp[off + 40..]);
        let mut w11 = load_bigendian_32(&inp[off + 44..]);
        let mut w12 = load_bigendian_32(&inp[off + 48..]);
        let mut w13 = load_bigendian_32(&inp[off + 52..]);
        let mut w14 = load_bigendian_32(&inp[off + 56..]);
        let mut w15 = load_bigendian_32(&inp[off + 60..]);

        F_32!(a, b, c, d, e, f, g, h, T1, T2, w0, 0x428a2f98u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w1, 0x71374491u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w2, 0xb5c0fbcfu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w3, 0xe9b5dba5u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w4, 0x3956c25bu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w5, 0x59f111f1u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w6, 0x923f82a4u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w7, 0xab1c5ed5u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w8, 0xd807aa98u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w9, 0x12835b01u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w10, 0x243185beu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w11, 0x550c7dc3u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w12, 0x72be5d74u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w13, 0x80deb1feu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w14, 0x9bdc06a7u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w15, 0xc19bf174u32);

        EXPAND_32!(
            w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15
        );

        F_32!(a, b, c, d, e, f, g, h, T1, T2, w0, 0xe49b69c1u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w1, 0xefbe4786u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w2, 0x0fc19dc6u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w3, 0x240ca1ccu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w4, 0x2de92c6fu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w5, 0x4a7484aau32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w6, 0x5cb0a9dcu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w7, 0x76f988dau32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w8, 0x983e5152u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w9, 0xa831c66du32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w10, 0xb00327c8u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w11, 0xbf597fc7u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w12, 0xc6e00bf3u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w13, 0xd5a79147u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w14, 0x06ca6351u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w15, 0x14292967u32);

        EXPAND_32!(
            w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15
        );

        F_32!(a, b, c, d, e, f, g, h, T1, T2, w0, 0x27b70a85u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w1, 0x2e1b2138u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w2, 0x4d2c6dfcu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w3, 0x53380d13u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w4, 0x650a7354u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w5, 0x766a0abbu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w6, 0x81c2c92eu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w7, 0x92722c85u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w8, 0xa2bfe8a1u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w9, 0xa81a664bu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w10, 0xc24b8b70u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w11, 0xc76c51a3u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w12, 0xd192e819u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w13, 0xd6990624u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w14, 0xf40e3585u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w15, 0x106aa070u32);

        EXPAND_32!(
            w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15
        );

        F_32!(a, b, c, d, e, f, g, h, T1, T2, w0, 0x19a4c116u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w1, 0x1e376c08u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w2, 0x2748774cu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w3, 0x34b0bcb5u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w4, 0x391c0cb3u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w5, 0x4ed8aa4au32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w6, 0x5b9cca4fu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w7, 0x682e6ff3u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w8, 0x748f82eeu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w9, 0x78a5636fu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w10, 0x84c87814u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w11, 0x8cc70208u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w12, 0x90befffau32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w13, 0xa4506cebu32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w14, 0xbef9a3f7u32);
        F_32!(a, b, c, d, e, f, g, h, T1, T2, w15, 0xc67178f2u32);

        a = a.wrapping_add(state[0]);
        b = b.wrapping_add(state[1]);
        c = c.wrapping_add(state[2]);
        d = d.wrapping_add(state[3]);
        e = e.wrapping_add(state[4]);
        f = f.wrapping_add(state[5]);
        g = g.wrapping_add(state[6]);
        h = h.wrapping_add(state[7]);

        state[0] = a;
        state[1] = b;
        state[2] = c;
        state[3] = d;
        state[4] = e;
        state[5] = f;
        state[6] = g;
        state[7] = h;

        off += 64;
        inlen -= 64;
    }

    store_bigendian_32(&mut statebytes[0..], state[0] as u64);
    store_bigendian_32(&mut statebytes[4..], state[1] as u64);
    store_bigendian_32(&mut statebytes[8..], state[2] as u64);
    store_bigendian_32(&mut statebytes[12..], state[3] as u64);
    store_bigendian_32(&mut statebytes[16..], state[4] as u64);
    store_bigendian_32(&mut statebytes[20..], state[5] as u64);
    store_bigendian_32(&mut statebytes[24..], state[6] as u64);
    store_bigendian_32(&mut statebytes[28..], state[7] as u64);

    inlen
}

// ---------------------------------------------------------------------------
// crypto_hashblocks_sha512
// ---------------------------------------------------------------------------

#[allow(non_snake_case)]
fn crypto_hashblocks_sha512(statebytes: &mut [u8], inp: &[u8], mut inlen: u64) -> u64 {
    let mut state = [0u64; 8];
    let mut a: u64;
    let mut b: u64;
    let mut c: u64;
    let mut d: u64;
    let mut e: u64;
    let mut f: u64;
    let mut g: u64;
    let mut h: u64;
    let mut T1: u64;
    let mut T2: u64;

    a = load_bigendian_64(&statebytes[0..]);
    state[0] = a;
    b = load_bigendian_64(&statebytes[8..]);
    state[1] = b;
    c = load_bigendian_64(&statebytes[16..]);
    state[2] = c;
    d = load_bigendian_64(&statebytes[24..]);
    state[3] = d;
    e = load_bigendian_64(&statebytes[32..]);
    state[4] = e;
    f = load_bigendian_64(&statebytes[40..]);
    state[5] = f;
    g = load_bigendian_64(&statebytes[48..]);
    state[6] = g;
    h = load_bigendian_64(&statebytes[56..]);
    state[7] = h;

    let mut off = 0usize;
    while inlen >= 128 {
        let mut w0 = load_bigendian_64(&inp[off + 0..]);
        let mut w1 = load_bigendian_64(&inp[off + 8..]);
        let mut w2 = load_bigendian_64(&inp[off + 16..]);
        let mut w3 = load_bigendian_64(&inp[off + 24..]);
        let mut w4 = load_bigendian_64(&inp[off + 32..]);
        let mut w5 = load_bigendian_64(&inp[off + 40..]);
        let mut w6 = load_bigendian_64(&inp[off + 48..]);
        let mut w7 = load_bigendian_64(&inp[off + 56..]);
        let mut w8 = load_bigendian_64(&inp[off + 64..]);
        let mut w9 = load_bigendian_64(&inp[off + 72..]);
        let mut w10 = load_bigendian_64(&inp[off + 80..]);
        let mut w11 = load_bigendian_64(&inp[off + 88..]);
        let mut w12 = load_bigendian_64(&inp[off + 96..]);
        let mut w13 = load_bigendian_64(&inp[off + 104..]);
        let mut w14 = load_bigendian_64(&inp[off + 112..]);
        let mut w15 = load_bigendian_64(&inp[off + 120..]);

        F_64!(a, b, c, d, e, f, g, h, T1, T2, w0, 0x428a2f98d728ae22u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w1, 0x7137449123ef65cdu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w2, 0xb5c0fbcfec4d3b2fu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w3, 0xe9b5dba58189dbbcu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w4, 0x3956c25bf348b538u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w5, 0x59f111f1b605d019u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w6, 0x923f82a4af194f9bu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w7, 0xab1c5ed5da6d8118u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w8, 0xd807aa98a3030242u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w9, 0x12835b0145706fbeu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w10, 0x243185be4ee4b28cu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w11, 0x550c7dc3d5ffb4e2u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w12, 0x72be5d74f27b896fu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w13, 0x80deb1fe3b1696b1u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w14, 0x9bdc06a725c71235u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w15, 0xc19bf174cf692694u64);

        EXPAND_64!(
            w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15
        );

        F_64!(a, b, c, d, e, f, g, h, T1, T2, w0, 0xe49b69c19ef14ad2u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w1, 0xefbe4786384f25e3u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w2, 0x0fc19dc68b8cd5b5u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w3, 0x240ca1cc77ac9c65u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w4, 0x2de92c6f592b0275u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w5, 0x4a7484aa6ea6e483u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w6, 0x5cb0a9dcbd41fbd4u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w7, 0x76f988da831153b5u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w8, 0x983e5152ee66dfabu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w9, 0xa831c66d2db43210u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w10, 0xb00327c898fb213fu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w11, 0xbf597fc7beef0ee4u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w12, 0xc6e00bf33da88fc2u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w13, 0xd5a79147930aa725u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w14, 0x06ca6351e003826fu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w15, 0x142929670a0e6e70u64);

        EXPAND_64!(
            w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15
        );

        F_64!(a, b, c, d, e, f, g, h, T1, T2, w0, 0x27b70a8546d22ffcu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w1, 0x2e1b21385c26c926u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w2, 0x4d2c6dfc5ac42aedu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w3, 0x53380d139d95b3dfu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w4, 0x650a73548baf63deu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w5, 0x766a0abb3c77b2a8u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w6, 0x81c2c92e47edaee6u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w7, 0x92722c851482353bu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w8, 0xa2bfe8a14cf10364u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w9, 0xa81a664bbc423001u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w10, 0xc24b8b70d0f89791u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w11, 0xc76c51a30654be30u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w12, 0xd192e819d6ef5218u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w13, 0xd69906245565a910u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w14, 0xf40e35855771202au64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w15, 0x106aa07032bbd1b8u64);

        EXPAND_64!(
            w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15
        );

        F_64!(a, b, c, d, e, f, g, h, T1, T2, w0, 0x19a4c116b8d2d0c8u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w1, 0x1e376c085141ab53u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w2, 0x2748774cdf8eeb99u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w3, 0x34b0bcb5e19b48a8u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w4, 0x391c0cb3c5c95a63u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w5, 0x4ed8aa4ae3418acbu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w6, 0x5b9cca4f7763e373u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w7, 0x682e6ff3d6b2b8a3u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w8, 0x748f82ee5defb2fcu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w9, 0x78a5636f43172f60u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w10, 0x84c87814a1f0ab72u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w11, 0x8cc702081a6439ecu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w12, 0x90befffa23631e28u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w13, 0xa4506cebde82bde9u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w14, 0xbef9a3f7b2c67915u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w15, 0xc67178f2e372532bu64);

        EXPAND_64!(
            w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15
        );

        F_64!(a, b, c, d, e, f, g, h, T1, T2, w0, 0xca273eceea26619cu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w1, 0xd186b8c721c0c207u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w2, 0xeada7dd6cde0eb1eu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w3, 0xf57d4f7fee6ed178u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w4, 0x06f067aa72176fbau64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w5, 0x0a637dc5a2c898a6u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w6, 0x113f9804bef90daeu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w7, 0x1b710b35131c471bu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w8, 0x28db77f523047d84u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w9, 0x32caab7b40c72493u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w10, 0x3c9ebe0a15c9bebcu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w11, 0x431d67c49c100d4cu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w12, 0x4cc5d4becb3e42b6u64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w13, 0x597f299cfc657e2au64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w14, 0x5fcb6fab3ad6faecu64);
        F_64!(a, b, c, d, e, f, g, h, T1, T2, w15, 0x6c44198c4a475817u64);

        a = a.wrapping_add(state[0]);
        b = b.wrapping_add(state[1]);
        c = c.wrapping_add(state[2]);
        d = d.wrapping_add(state[3]);
        e = e.wrapping_add(state[4]);
        f = f.wrapping_add(state[5]);
        g = g.wrapping_add(state[6]);
        h = h.wrapping_add(state[7]);

        state[0] = a;
        state[1] = b;
        state[2] = c;
        state[3] = d;
        state[4] = e;
        state[5] = f;
        state[6] = g;
        state[7] = h;

        off += 128;
        inlen -= 128;
    }

    store_bigendian_64(&mut statebytes[0..], state[0]);
    store_bigendian_64(&mut statebytes[8..], state[1]);
    store_bigendian_64(&mut statebytes[16..], state[2]);
    store_bigendian_64(&mut statebytes[24..], state[3]);
    store_bigendian_64(&mut statebytes[32..], state[4]);
    store_bigendian_64(&mut statebytes[40..], state[5]);
    store_bigendian_64(&mut statebytes[48..], state[6]);
    store_bigendian_64(&mut statebytes[56..], state[7]);

    inlen
}

// ---------------------------------------------------------------------------
// initial values
// ---------------------------------------------------------------------------

#[allow(non_upper_case_globals)]
static iv_256: [u8; 32] = [
    0x6a, 0x09, 0xe6, 0x67, 0xbb, 0x67, 0xae, 0x85, 0x3c, 0x6e, 0xf3, 0x72, 0xa5, 0x4f, 0xf5, 0x3a,
    0x51, 0x0e, 0x52, 0x7f, 0x9b, 0x05, 0x68, 0x8c, 0x1f, 0x83, 0xd9, 0xab, 0x5b, 0xe0, 0xcd, 0x19,
];

#[allow(non_upper_case_globals)]
static iv_512: [u8; 64] = [
    0x6a, 0x09, 0xe6, 0x67, 0xf3, 0xbc, 0xc9, 0x08, 0xbb, 0x67, 0xae, 0x85, 0x84, 0xca, 0xa7, 0x3b,
    0x3c, 0x6e, 0xf3, 0x72, 0xfe, 0x94, 0xf8, 0x2b, 0xa5, 0x4f, 0xf5, 0x3a, 0x5f, 0x1d, 0x36, 0xf1,
    0x51, 0x0e, 0x52, 0x7f, 0xad, 0xe6, 0x82, 0xd1, 0x9b, 0x05, 0x68, 0x8c, 0x2b, 0x3e, 0x6c, 0x1f,
    0x1f, 0x83, 0xd9, 0xab, 0xfb, 0x41, 0xbd, 0x6b, 0x5b, 0xe0, 0xcd, 0x19, 0x13, 0x7e, 0x21, 0x79,
];

// ---------------------------------------------------------------------------
// incremental API (Rust idiomatic)
// ---------------------------------------------------------------------------

pub fn sha256_inc_init_rs(state: &mut [u8; 40]) {
    for i in 0..32 {
        state[i] = iv_256[i];
    }
    for i in 32..40 {
        state[i] = 0;
    }
}

pub fn sha512_inc_init_rs(state: &mut [u8; 72]) {
    for i in 0..64 {
        state[i] = iv_512[i];
    }
    for i in 64..72 {
        state[i] = 0;
    }
}

pub fn sha256_inc_blocks_rs(state: &mut [u8; 40], inp: &[u8], inblocks: usize) {
    let mut bytes = load_bigendian_64(&state[32..]);

    crypto_hashblocks_sha256(&mut state[..], inp, 64 * inblocks);
    bytes += (64 * inblocks) as u64;

    store_bigendian_64(&mut state[32..], bytes);
}

pub fn sha512_inc_blocks_rs(state: &mut [u8; 72], inp: &[u8], inblocks: usize) {
    let mut bytes = load_bigendian_64(&state[64..]);

    crypto_hashblocks_sha512(&mut state[..], inp, (128 * inblocks) as u64);
    bytes += (128 * inblocks) as u64;

    store_bigendian_64(&mut state[64..], bytes);
}

pub fn sha256_inc_finalize_rs(out: &mut [u8], state: &mut [u8; 40], inp: &[u8]) {
    let mut inlen = inp.len();
    let mut padded = [0u8; 128];
    let bytes = load_bigendian_64(&state[32..]) + inlen as u64;

    crypto_hashblocks_sha256(&mut state[..], inp, inlen);
    // in += inlen; inlen &= 63; in -= inlen;  -> offset into inp of the tail
    let mut in_off = inlen;
    inlen &= 63;
    in_off -= inlen;

    for i in 0..inlen {
        padded[i] = inp[in_off + i];
    }
    padded[inlen] = 0x80;

    if inlen < 56 {
        for i in (inlen + 1)..56 {
            padded[i] = 0;
        }
        padded[56] = (bytes >> 53) as u8;
        padded[57] = (bytes >> 45) as u8;
        padded[58] = (bytes >> 37) as u8;
        padded[59] = (bytes >> 29) as u8;
        padded[60] = (bytes >> 21) as u8;
        padded[61] = (bytes >> 13) as u8;
        padded[62] = (bytes >> 5) as u8;
        padded[63] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(&mut state[..], &padded, 64);
    } else {
        for i in (inlen + 1)..120 {
            padded[i] = 0;
        }
        padded[120] = (bytes >> 53) as u8;
        padded[121] = (bytes >> 45) as u8;
        padded[122] = (bytes >> 37) as u8;
        padded[123] = (bytes >> 29) as u8;
        padded[124] = (bytes >> 21) as u8;
        padded[125] = (bytes >> 13) as u8;
        padded[126] = (bytes >> 5) as u8;
        padded[127] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(&mut state[..], &padded, 128);
    }

    for i in 0..32 {
        out[i] = state[i];
    }
}

pub fn sha512_inc_finalize_rs(out: &mut [u8], state: &mut [u8; 72], inp: &[u8]) {
    let mut inlen = inp.len();
    let mut padded = [0u8; 256];
    let bytes = load_bigendian_64(&state[64..]) + inlen as u64;

    crypto_hashblocks_sha512(&mut state[..], inp, inlen as u64);
    let mut in_off = inlen;
    inlen &= 127;
    in_off -= inlen;

    for i in 0..inlen {
        padded[i] = inp[in_off + i];
    }
    padded[inlen] = 0x80;

    if inlen < 112 {
        for i in (inlen + 1)..119 {
            padded[i] = 0;
        }
        padded[119] = (bytes >> 61) as u8;
        padded[120] = (bytes >> 53) as u8;
        padded[121] = (bytes >> 45) as u8;
        padded[122] = (bytes >> 37) as u8;
        padded[123] = (bytes >> 29) as u8;
        padded[124] = (bytes >> 21) as u8;
        padded[125] = (bytes >> 13) as u8;
        padded[126] = (bytes >> 5) as u8;
        padded[127] = (bytes << 3) as u8;
        crypto_hashblocks_sha512(&mut state[..], &padded, 128);
    } else {
        for i in (inlen + 1)..247 {
            padded[i] = 0;
        }
        padded[247] = (bytes >> 61) as u8;
        padded[248] = (bytes >> 53) as u8;
        padded[249] = (bytes >> 45) as u8;
        padded[250] = (bytes >> 37) as u8;
        padded[251] = (bytes >> 29) as u8;
        padded[252] = (bytes >> 21) as u8;
        padded[253] = (bytes >> 13) as u8;
        padded[254] = (bytes >> 5) as u8;
        padded[255] = (bytes << 3) as u8;
        crypto_hashblocks_sha512(&mut state[..], &padded, 256);
    }

    for i in 0..64 {
        out[i] = state[i];
    }
}

pub fn sha256_rs(out: &mut [u8], inp: &[u8]) {
    let mut state = [0u8; 40];

    sha256_inc_init_rs(&mut state);
    sha256_inc_finalize_rs(out, &mut state, inp);
}

pub fn sha512_rs(out: &mut [u8], inp: &[u8]) {
    let mut state = [0u8; 72];

    sha512_inc_init_rs(&mut state);
    sha512_inc_finalize_rs(out, &mut state, inp);
}

// ---------------------------------------------------------------------------
// mgf1
// ---------------------------------------------------------------------------

/// mgf1 function based on the SHA-256 hash function.
///
/// Note that `inlen` should be sufficiently small; the C uses a stack VLA of
/// length `inlen + 4`. Here a `Vec<u8>` of the same length is used. Outputs
/// `outlen` bytes.
pub fn mgf1_256_rs(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf: Vec<u8> = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut out_off = 0usize;
    let mut i: usize = 0;
    /* While we can fit in at least another full block of SHA256 output.. */
    while (i + 1) * SPX_SHA256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256_rs(&mut out[out_off..], &inbuf[..inlen + 4]);
        out_off += SPX_SHA256_OUTPUT_BYTES;
        i += 1;
    }
    /* Until we cannot anymore, and we fill the remainder. */
    if outlen > i * SPX_SHA256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256_rs(&mut outbuf, &inbuf[..inlen + 4]);
        let rem = outlen - i * SPX_SHA256_OUTPUT_BYTES;
        out[out_off..out_off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

/// mgf1 function based on the SHA-512 hash function.
pub fn mgf1_512_rs(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf: Vec<u8> = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];

    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut out_off = 0usize;
    let mut i: usize = 0;
    /* While we can fit in at least another full block of SHA512 output.. */
    while (i + 1) * SPX_SHA512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha512_rs(&mut out[out_off..], &inbuf[..inlen + 4]);
        out_off += SPX_SHA512_OUTPUT_BYTES;
        i += 1;
    }
    /* Until we cannot anymore, and we fill the remainder. */
    if outlen > i * SPX_SHA512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha512_rs(&mut outbuf, &inbuf[..inlen + 4]);
        let rem = outlen - i * SPX_SHA512_OUTPUT_BYTES;
        out[out_off..out_off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

// ---------------------------------------------------------------------------
// seed_state
// ---------------------------------------------------------------------------

/// Absorb the constant `pub_seed` using one round of the compression function.
/// This initializes `state_seeded` and `state_seeded_512`, which can then be
/// reused in `thash`.
pub fn seed_state_rs(ctx: &mut crate::context::SpxCtx) {
    let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];

    for i in 0..SPX_N {
        block[i] = ctx.pub_seed[i];
    }
    for i in SPX_N..SPX_SHA512_BLOCK_BYTES {
        block[i] = 0;
    }
    /* block has been properly initialized for both SHA-256 and SHA-512 */

    // Reference SPX_WIDE to keep parity with the C `#if SPX_SHA512` selector
    // even though gating is done via cfg below.
    let _ = SPX_WIDE;

    sha256_inc_init_rs(&mut ctx.state_seeded);
    sha256_inc_blocks_rs(&mut ctx.state_seeded, &block, 1);
    #[cfg(spx_n_ge_24)]
    {
        sha512_inc_init_rs(&mut ctx.state_seeded_512);
        sha512_inc_blocks_rs(&mut ctx.state_seeded_512, &block, 1);
    }
}

// ---------------------------------------------------------------------------
// C-ABI wrappers
// ---------------------------------------------------------------------------

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_init(state: *mut u8) {
    let state = unsafe { &mut *(state as *mut [u8; 40]) };
    sha256_inc_init_rs(state);
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_blocks(state: *mut u8, inp: *const u8, inblocks: usize) {
    let state = unsafe { &mut *(state as *mut [u8; 40]) };
    let inp = unsafe { core::slice::from_raw_parts(inp, 64 * inblocks) };
    sha256_inc_blocks_rs(state, inp, inblocks);
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_finalize(
    out: *mut u8,
    state: *mut u8,
    inp: *const u8,
    inlen: usize,
) {
    let state = unsafe { &mut *(state as *mut [u8; 40]) };
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen) };
    let out = unsafe { core::slice::from_raw_parts_mut(out, 32) };
    sha256_inc_finalize_rs(out, state, inp);
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256(out: *mut u8, inp: *const u8, inlen: usize) {
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen) };
    let out = unsafe { core::slice::from_raw_parts_mut(out, 32) };
    sha256_rs(out, inp);
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_init(state: *mut u8) {
    let state = unsafe { &mut *(state as *mut [u8; 72]) };
    sha512_inc_init_rs(state);
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_blocks(state: *mut u8, inp: *const u8, inblocks: usize) {
    let state = unsafe { &mut *(state as *mut [u8; 72]) };
    let inp = unsafe { core::slice::from_raw_parts(inp, 128 * inblocks) };
    sha512_inc_blocks_rs(state, inp, inblocks);
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_finalize(
    out: *mut u8,
    state: *mut u8,
    inp: *const u8,
    inlen: usize,
) {
    let state = unsafe { &mut *(state as *mut [u8; 72]) };
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen) };
    let out = unsafe { core::slice::from_raw_parts_mut(out, 64) };
    sha512_inc_finalize_rs(out, state, inp);
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512(out: *mut u8, inp: *const u8, inlen: usize) {
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen) };
    let out = unsafe { core::slice::from_raw_parts_mut(out, 64) };
    sha512_rs(out, inp);
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_mgf1_256(
    out: *mut u8,
    outlen: core::ffi::c_ulong,
    inp: *const u8,
    inlen: core::ffi::c_ulong,
) {
    let outlen = outlen as usize;
    let inlen = inlen as usize;
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen) };
    let out = unsafe { core::slice::from_raw_parts_mut(out, outlen) };
    mgf1_256_rs(out, outlen, inp, inlen);
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_mgf1_512(
    out: *mut u8,
    outlen: core::ffi::c_ulong,
    inp: *const u8,
    inlen: core::ffi::c_ulong,
) {
    let outlen = outlen as usize;
    let inlen = inlen as usize;
    let inp = unsafe { core::slice::from_raw_parts(inp, inlen) };
    let out = unsafe { core::slice::from_raw_parts_mut(out, outlen) };
    mgf1_512_rs(out, outlen, inp, inlen);
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_seed_state(ctx: *mut crate::context::SpxCtx) {
    let ctx = unsafe { &mut *ctx };
    seed_state_rs(ctx);
}
