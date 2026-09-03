/* Based on the public domain implementation in
 * crypto_hash/sha512/ref/ from http://bench.cr.yp.to/supercop.html
 * by D. J. Bernstein */

#![allow(clippy::missing_safety_doc)]

use crate::params::*;

pub const SPX_SHA256_BLOCK_BYTES: usize = 64;
pub const SPX_SHA256_OUTPUT_BYTES: usize = 32;
pub const SPX_SHA512_BLOCK_BYTES: usize = 128;
pub const SPX_SHA512_OUTPUT_BYTES: usize = 64;
pub const SPX_SHA256_ADDR_BYTES: usize = 22;

#[inline]
unsafe fn load_bigendian_32(x: *const u8) -> u32 {
    (*x.add(3) as u32)
        | ((*x.add(2) as u32) << 8)
        | ((*x.add(1) as u32) << 16)
        | ((*x.add(0) as u32) << 24)
}

#[inline]
unsafe fn load_bigendian_64(x: *const u8) -> u64 {
    (*x.add(7) as u64)
        | ((*x.add(6) as u64) << 8)
        | ((*x.add(5) as u64) << 16)
        | ((*x.add(4) as u64) << 24)
        | ((*x.add(3) as u64) << 32)
        | ((*x.add(2) as u64) << 40)
        | ((*x.add(1) as u64) << 48)
        | ((*x.add(0) as u64) << 56)
}

#[inline]
unsafe fn store_bigendian_32(x: *mut u8, mut u: u64) {
    *x.add(3) = u as u8;
    u >>= 8;
    *x.add(2) = u as u8;
    u >>= 8;
    *x.add(1) = u as u8;
    u >>= 8;
    *x.add(0) = u as u8;
}

#[inline]
unsafe fn store_bigendian_64(x: *mut u8, mut u: u64) {
    *x.add(7) = u as u8;
    u >>= 8;
    *x.add(6) = u as u8;
    u >>= 8;
    *x.add(5) = u as u8;
    u >>= 8;
    *x.add(4) = u as u8;
    u >>= 8;
    *x.add(3) = u as u8;
    u >>= 8;
    *x.add(2) = u as u8;
    u >>= 8;
    *x.add(1) = u as u8;
    u >>= 8;
    *x.add(0) = u as u8;
}

// ---- SHA-256 round primitives ----
#[inline]
fn ch_32(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (!x & z)
}
#[inline]
fn maj_32(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (x & z) ^ (y & z)
}
#[inline]
fn big_sigma0_32(x: u32) -> u32 {
    x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
}
#[inline]
fn big_sigma1_32(x: u32) -> u32 {
    x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
}
#[inline]
fn small_sigma0_32(x: u32) -> u32 {
    x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
}
#[inline]
fn small_sigma1_32(x: u32) -> u32 {
    x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
}

// F_32(w, k): T1 = h + Sigma1(e) + Ch(e,f,g) + k + w; T2 = Sigma0(a) + Maj(a,b,c); ...
macro_rules! f_32 {
    ($a:ident,$b:ident,$c:ident,$d:ident,$e:ident,$f:ident,$g:ident,$h:ident,$t1:ident,$t2:ident,$w:expr,$k:expr) => {
        $t1 = $h
            .wrapping_add(big_sigma1_32($e))
            .wrapping_add(ch_32($e, $f, $g))
            .wrapping_add($k)
            .wrapping_add($w);
        $t2 = big_sigma0_32($a).wrapping_add(maj_32($a, $b, $c));
        $h = $g;
        $g = $f;
        $f = $e;
        $e = $d.wrapping_add($t1);
        $d = $c;
        $c = $b;
        $b = $a;
        $a = $t1.wrapping_add($t2);
    };
}

// M_32(w0, w14, w9, w1): w0 = sigma1(w14) + w9 + sigma0(w1) + w0
macro_rules! m_32 {
    ($w0:ident,$w14:ident,$w9:ident,$w1:ident) => {
        $w0 = small_sigma1_32($w14)
            .wrapping_add($w9)
            .wrapping_add(small_sigma0_32($w1))
            .wrapping_add($w0);
    };
}

macro_rules! expand_32 {
    ($w0:ident,$w1:ident,$w2:ident,$w3:ident,$w4:ident,$w5:ident,$w6:ident,$w7:ident,$w8:ident,$w9:ident,$w10:ident,$w11:ident,$w12:ident,$w13:ident,$w14:ident,$w15:ident) => {
        m_32!($w0, $w14, $w9, $w1);
        m_32!($w1, $w15, $w10, $w2);
        m_32!($w2, $w0, $w11, $w3);
        m_32!($w3, $w1, $w12, $w4);
        m_32!($w4, $w2, $w13, $w5);
        m_32!($w5, $w3, $w14, $w6);
        m_32!($w6, $w4, $w15, $w7);
        m_32!($w7, $w5, $w0, $w8);
        m_32!($w8, $w6, $w1, $w9);
        m_32!($w9, $w7, $w2, $w10);
        m_32!($w10, $w8, $w3, $w11);
        m_32!($w11, $w9, $w4, $w12);
        m_32!($w12, $w10, $w5, $w13);
        m_32!($w13, $w11, $w6, $w14);
        m_32!($w14, $w12, $w7, $w15);
        m_32!($w15, $w13, $w8, $w0);
    };
}

unsafe fn crypto_hashblocks_sha256(statebytes: *mut u8, mut in_: *const u8, mut inlen: usize) -> usize {
    let mut state = [0u32; 8];
    let mut a: u32;
    let mut b: u32;
    let mut c: u32;
    let mut d: u32;
    let mut e: u32;
    let mut f: u32;
    let mut g: u32;
    let mut h: u32;
    let mut t1: u32;
    let mut t2: u32;

    a = load_bigendian_32(statebytes.add(0));
    state[0] = a;
    b = load_bigendian_32(statebytes.add(4));
    state[1] = b;
    c = load_bigendian_32(statebytes.add(8));
    state[2] = c;
    d = load_bigendian_32(statebytes.add(12));
    state[3] = d;
    e = load_bigendian_32(statebytes.add(16));
    state[4] = e;
    f = load_bigendian_32(statebytes.add(20));
    state[5] = f;
    g = load_bigendian_32(statebytes.add(24));
    state[6] = g;
    h = load_bigendian_32(statebytes.add(28));
    state[7] = h;

    while inlen >= 64 {
        let mut w0 = load_bigendian_32(in_.add(0));
        let mut w1 = load_bigendian_32(in_.add(4));
        let mut w2 = load_bigendian_32(in_.add(8));
        let mut w3 = load_bigendian_32(in_.add(12));
        let mut w4 = load_bigendian_32(in_.add(16));
        let mut w5 = load_bigendian_32(in_.add(20));
        let mut w6 = load_bigendian_32(in_.add(24));
        let mut w7 = load_bigendian_32(in_.add(28));
        let mut w8 = load_bigendian_32(in_.add(32));
        let mut w9 = load_bigendian_32(in_.add(36));
        let mut w10 = load_bigendian_32(in_.add(40));
        let mut w11 = load_bigendian_32(in_.add(44));
        let mut w12 = load_bigendian_32(in_.add(48));
        let mut w13 = load_bigendian_32(in_.add(52));
        let mut w14 = load_bigendian_32(in_.add(56));
        let mut w15 = load_bigendian_32(in_.add(60));

        f_32!(a, b, c, d, e, f, g, h, t1, t2, w0, 0x428a2f98);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w1, 0x71374491);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w2, 0xb5c0fbcf);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w3, 0xe9b5dba5);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w4, 0x3956c25b);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w5, 0x59f111f1);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w6, 0x923f82a4);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w7, 0xab1c5ed5);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w8, 0xd807aa98);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w9, 0x12835b01);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w10, 0x243185be);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w11, 0x550c7dc3);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w12, 0x72be5d74);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w13, 0x80deb1fe);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w14, 0x9bdc06a7);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w15, 0xc19bf174);

        expand_32!(w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15);

        f_32!(a, b, c, d, e, f, g, h, t1, t2, w0, 0xe49b69c1);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w1, 0xefbe4786);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w2, 0x0fc19dc6);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w3, 0x240ca1cc);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w4, 0x2de92c6f);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w5, 0x4a7484aa);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w6, 0x5cb0a9dc);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w7, 0x76f988da);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w8, 0x983e5152);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w9, 0xa831c66d);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w10, 0xb00327c8);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w11, 0xbf597fc7);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w12, 0xc6e00bf3);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w13, 0xd5a79147);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w14, 0x06ca6351);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w15, 0x14292967);

        expand_32!(w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15);

        f_32!(a, b, c, d, e, f, g, h, t1, t2, w0, 0x27b70a85);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w1, 0x2e1b2138);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w2, 0x4d2c6dfc);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w3, 0x53380d13);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w4, 0x650a7354);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w5, 0x766a0abb);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w6, 0x81c2c92e);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w7, 0x92722c85);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w8, 0xa2bfe8a1);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w9, 0xa81a664b);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w10, 0xc24b8b70);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w11, 0xc76c51a3);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w12, 0xd192e819);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w13, 0xd6990624);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w14, 0xf40e3585);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w15, 0x106aa070);

        expand_32!(w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15);

        f_32!(a, b, c, d, e, f, g, h, t1, t2, w0, 0x19a4c116);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w1, 0x1e376c08);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w2, 0x2748774c);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w3, 0x34b0bcb5);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w4, 0x391c0cb3);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w5, 0x4ed8aa4a);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w6, 0x5b9cca4f);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w7, 0x682e6ff3);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w8, 0x748f82ee);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w9, 0x78a5636f);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w10, 0x84c87814);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w11, 0x8cc70208);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w12, 0x90befffa);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w13, 0xa4506ceb);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w14, 0xbef9a3f7);
        f_32!(a, b, c, d, e, f, g, h, t1, t2, w15, 0xc67178f2);

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

        in_ = in_.add(64);
        inlen -= 64;
    }

    store_bigendian_32(statebytes.add(0), state[0] as u64);
    store_bigendian_32(statebytes.add(4), state[1] as u64);
    store_bigendian_32(statebytes.add(8), state[2] as u64);
    store_bigendian_32(statebytes.add(12), state[3] as u64);
    store_bigendian_32(statebytes.add(16), state[4] as u64);
    store_bigendian_32(statebytes.add(20), state[5] as u64);
    store_bigendian_32(statebytes.add(24), state[6] as u64);
    store_bigendian_32(statebytes.add(28), state[7] as u64);

    inlen
}

// ---- SHA-512 round primitives ----
#[inline]
fn ch_64(x: u64, y: u64, z: u64) -> u64 {
    (x & y) ^ (!x & z)
}
#[inline]
fn maj_64(x: u64, y: u64, z: u64) -> u64 {
    (x & y) ^ (x & z) ^ (y & z)
}
#[inline]
fn big_sigma0_64(x: u64) -> u64 {
    x.rotate_right(28) ^ x.rotate_right(34) ^ x.rotate_right(39)
}
#[inline]
fn big_sigma1_64(x: u64) -> u64 {
    x.rotate_right(14) ^ x.rotate_right(18) ^ x.rotate_right(41)
}
#[inline]
fn small_sigma0_64(x: u64) -> u64 {
    x.rotate_right(1) ^ x.rotate_right(8) ^ (x >> 7)
}
#[inline]
fn small_sigma1_64(x: u64) -> u64 {
    x.rotate_right(19) ^ x.rotate_right(61) ^ (x >> 6)
}

macro_rules! f_64 {
    ($a:ident,$b:ident,$c:ident,$d:ident,$e:ident,$f:ident,$g:ident,$h:ident,$t1:ident,$t2:ident,$w:expr,$k:expr) => {
        $t1 = $h
            .wrapping_add(big_sigma1_64($e))
            .wrapping_add(ch_64($e, $f, $g))
            .wrapping_add($k)
            .wrapping_add($w);
        $t2 = big_sigma0_64($a).wrapping_add(maj_64($a, $b, $c));
        $h = $g;
        $g = $f;
        $f = $e;
        $e = $d.wrapping_add($t1);
        $d = $c;
        $c = $b;
        $b = $a;
        $a = $t1.wrapping_add($t2);
    };
}

macro_rules! m_64 {
    ($w0:ident,$w14:ident,$w9:ident,$w1:ident) => {
        $w0 = small_sigma1_64($w14)
            .wrapping_add($w9)
            .wrapping_add(small_sigma0_64($w1))
            .wrapping_add($w0);
    };
}

macro_rules! expand_64 {
    ($w0:ident,$w1:ident,$w2:ident,$w3:ident,$w4:ident,$w5:ident,$w6:ident,$w7:ident,$w8:ident,$w9:ident,$w10:ident,$w11:ident,$w12:ident,$w13:ident,$w14:ident,$w15:ident) => {
        m_64!($w0, $w14, $w9, $w1);
        m_64!($w1, $w15, $w10, $w2);
        m_64!($w2, $w0, $w11, $w3);
        m_64!($w3, $w1, $w12, $w4);
        m_64!($w4, $w2, $w13, $w5);
        m_64!($w5, $w3, $w14, $w6);
        m_64!($w6, $w4, $w15, $w7);
        m_64!($w7, $w5, $w0, $w8);
        m_64!($w8, $w6, $w1, $w9);
        m_64!($w9, $w7, $w2, $w10);
        m_64!($w10, $w8, $w3, $w11);
        m_64!($w11, $w9, $w4, $w12);
        m_64!($w12, $w10, $w5, $w13);
        m_64!($w13, $w11, $w6, $w14);
        m_64!($w14, $w12, $w7, $w15);
        m_64!($w15, $w13, $w8, $w0);
    };
}

unsafe fn crypto_hashblocks_sha512(statebytes: *mut u8, mut in_: *const u8, mut inlen: u64) -> u64 {
    let mut state = [0u64; 8];
    let mut a: u64;
    let mut b: u64;
    let mut c: u64;
    let mut d: u64;
    let mut e: u64;
    let mut f: u64;
    let mut g: u64;
    let mut h: u64;
    let mut t1: u64;
    let mut t2: u64;

    a = load_bigendian_64(statebytes.add(0));
    state[0] = a;
    b = load_bigendian_64(statebytes.add(8));
    state[1] = b;
    c = load_bigendian_64(statebytes.add(16));
    state[2] = c;
    d = load_bigendian_64(statebytes.add(24));
    state[3] = d;
    e = load_bigendian_64(statebytes.add(32));
    state[4] = e;
    f = load_bigendian_64(statebytes.add(40));
    state[5] = f;
    g = load_bigendian_64(statebytes.add(48));
    state[6] = g;
    h = load_bigendian_64(statebytes.add(56));
    state[7] = h;

    while inlen >= 128 {
        let mut w0 = load_bigendian_64(in_.add(0));
        let mut w1 = load_bigendian_64(in_.add(8));
        let mut w2 = load_bigendian_64(in_.add(16));
        let mut w3 = load_bigendian_64(in_.add(24));
        let mut w4 = load_bigendian_64(in_.add(32));
        let mut w5 = load_bigendian_64(in_.add(40));
        let mut w6 = load_bigendian_64(in_.add(48));
        let mut w7 = load_bigendian_64(in_.add(56));
        let mut w8 = load_bigendian_64(in_.add(64));
        let mut w9 = load_bigendian_64(in_.add(72));
        let mut w10 = load_bigendian_64(in_.add(80));
        let mut w11 = load_bigendian_64(in_.add(88));
        let mut w12 = load_bigendian_64(in_.add(96));
        let mut w13 = load_bigendian_64(in_.add(104));
        let mut w14 = load_bigendian_64(in_.add(112));
        let mut w15 = load_bigendian_64(in_.add(120));

        f_64!(a, b, c, d, e, f, g, h, t1, t2, w0, 0x428a2f98d728ae22);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w1, 0x7137449123ef65cd);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w2, 0xb5c0fbcfec4d3b2f);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w3, 0xe9b5dba58189dbbc);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w4, 0x3956c25bf348b538);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w5, 0x59f111f1b605d019);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w6, 0x923f82a4af194f9b);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w7, 0xab1c5ed5da6d8118);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w8, 0xd807aa98a3030242);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w9, 0x12835b0145706fbe);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w10, 0x243185be4ee4b28c);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w11, 0x550c7dc3d5ffb4e2);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w12, 0x72be5d74f27b896f);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w13, 0x80deb1fe3b1696b1);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w14, 0x9bdc06a725c71235);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w15, 0xc19bf174cf692694);

        expand_64!(w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15);

        f_64!(a, b, c, d, e, f, g, h, t1, t2, w0, 0xe49b69c19ef14ad2);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w1, 0xefbe4786384f25e3);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w2, 0x0fc19dc68b8cd5b5);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w3, 0x240ca1cc77ac9c65);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w4, 0x2de92c6f592b0275);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w5, 0x4a7484aa6ea6e483);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w6, 0x5cb0a9dcbd41fbd4);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w7, 0x76f988da831153b5);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w8, 0x983e5152ee66dfab);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w9, 0xa831c66d2db43210);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w10, 0xb00327c898fb213f);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w11, 0xbf597fc7beef0ee4);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w12, 0xc6e00bf33da88fc2);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w13, 0xd5a79147930aa725);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w14, 0x06ca6351e003826f);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w15, 0x142929670a0e6e70);

        expand_64!(w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15);

        f_64!(a, b, c, d, e, f, g, h, t1, t2, w0, 0x27b70a8546d22ffc);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w1, 0x2e1b21385c26c926);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w2, 0x4d2c6dfc5ac42aed);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w3, 0x53380d139d95b3df);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w4, 0x650a73548baf63de);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w5, 0x766a0abb3c77b2a8);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w6, 0x81c2c92e47edaee6);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w7, 0x92722c851482353b);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w8, 0xa2bfe8a14cf10364);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w9, 0xa81a664bbc423001);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w10, 0xc24b8b70d0f89791);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w11, 0xc76c51a30654be30);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w12, 0xd192e819d6ef5218);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w13, 0xd69906245565a910);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w14, 0xf40e35855771202a);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w15, 0x106aa07032bbd1b8);

        expand_64!(w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15);

        f_64!(a, b, c, d, e, f, g, h, t1, t2, w0, 0x19a4c116b8d2d0c8);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w1, 0x1e376c085141ab53);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w2, 0x2748774cdf8eeb99);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w3, 0x34b0bcb5e19b48a8);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w4, 0x391c0cb3c5c95a63);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w5, 0x4ed8aa4ae3418acb);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w6, 0x5b9cca4f7763e373);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w7, 0x682e6ff3d6b2b8a3);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w8, 0x748f82ee5defb2fc);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w9, 0x78a5636f43172f60);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w10, 0x84c87814a1f0ab72);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w11, 0x8cc702081a6439ec);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w12, 0x90befffa23631e28);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w13, 0xa4506cebde82bde9);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w14, 0xbef9a3f7b2c67915);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w15, 0xc67178f2e372532b);

        expand_64!(w0, w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15);

        f_64!(a, b, c, d, e, f, g, h, t1, t2, w0, 0xca273eceea26619c);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w1, 0xd186b8c721c0c207);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w2, 0xeada7dd6cde0eb1e);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w3, 0xf57d4f7fee6ed178);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w4, 0x06f067aa72176fba);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w5, 0x0a637dc5a2c898a6);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w6, 0x113f9804bef90dae);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w7, 0x1b710b35131c471b);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w8, 0x28db77f523047d84);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w9, 0x32caab7b40c72493);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w10, 0x3c9ebe0a15c9bebc);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w11, 0x431d67c49c100d4c);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w12, 0x4cc5d4becb3e42b6);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w13, 0x597f299cfc657e2a);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w14, 0x5fcb6fab3ad6faec);
        f_64!(a, b, c, d, e, f, g, h, t1, t2, w15, 0x6c44198c4a475817);

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

        in_ = in_.add(128);
        inlen -= 128;
    }

    store_bigendian_64(statebytes.add(0), state[0]);
    store_bigendian_64(statebytes.add(8), state[1]);
    store_bigendian_64(statebytes.add(16), state[2]);
    store_bigendian_64(statebytes.add(24), state[3]);
    store_bigendian_64(statebytes.add(32), state[4]);
    store_bigendian_64(statebytes.add(40), state[5]);
    store_bigendian_64(statebytes.add(48), state[6]);
    store_bigendian_64(statebytes.add(56), state[7]);

    inlen
}

static IV_256: [u8; 32] = [
    0x6a, 0x09, 0xe6, 0x67, 0xbb, 0x67, 0xae, 0x85, 0x3c, 0x6e, 0xf3, 0x72, 0xa5, 0x4f, 0xf5, 0x3a,
    0x51, 0x0e, 0x52, 0x7f, 0x9b, 0x05, 0x68, 0x8c, 0x1f, 0x83, 0xd9, 0xab, 0x5b, 0xe0, 0xcd, 0x19,
];

static IV_512: [u8; 64] = [
    0x6a, 0x09, 0xe6, 0x67, 0xf3, 0xbc, 0xc9, 0x08, 0xbb, 0x67, 0xae, 0x85, 0x84, 0xca, 0xa7, 0x3b,
    0x3c, 0x6e, 0xf3, 0x72, 0xfe, 0x94, 0xf8, 0x2b, 0xa5, 0x4f, 0xf5, 0x3a, 0x5f, 0x1d, 0x36, 0xf1,
    0x51, 0x0e, 0x52, 0x7f, 0xad, 0xe6, 0x82, 0xd1, 0x9b, 0x05, 0x68, 0x8c, 0x2b, 0x3e, 0x6c, 0x1f,
    0x1f, 0x83, 0xd9, 0xab, 0xfb, 0x41, 0xbd, 0x6b, 0x5b, 0xe0, 0xcd, 0x19, 0x13, 0x7e, 0x21, 0x79,
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_init(state: *mut u8) {
    let mut i: usize = 0;
    while i < 32 {
        *state.add(i) = IV_256[i];
        i += 1;
    }
    let mut i: usize = 32;
    while i < 40 {
        *state.add(i) = 0;
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_init(state: *mut u8) {
    let mut i: usize = 0;
    while i < 64 {
        *state.add(i) = IV_512[i];
        i += 1;
    }
    let mut i: usize = 64;
    while i < 72 {
        *state.add(i) = 0;
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_blocks(state: *mut u8, in_: *const u8, inblocks: usize) {
    let mut bytes: u64 = load_bigendian_64(state.add(32));

    crypto_hashblocks_sha256(state, in_, 64 * inblocks);
    bytes += (64 * inblocks) as u64;

    store_bigendian_64(state.add(32), bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_blocks(state: *mut u8, in_: *const u8, inblocks: usize) {
    let mut bytes: u64 = load_bigendian_64(state.add(64));

    crypto_hashblocks_sha512(state, in_, (128 * inblocks) as u64);
    bytes += (128 * inblocks) as u64;

    store_bigendian_64(state.add(64), bytes);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256_inc_finalize(out: *mut u8, state: *mut u8, mut in_: *const u8, mut inlen: usize) {
    let mut padded = [0u8; 128];
    let bytes: u64 = load_bigendian_64(state.add(32)).wrapping_add(inlen as u64);

    crypto_hashblocks_sha256(state, in_, inlen);
    in_ = in_.add(inlen);
    inlen &= 63;
    in_ = in_.sub(inlen);

    let mut i: usize = 0;
    while i < inlen {
        padded[i] = *in_.add(i);
        i += 1;
    }
    padded[inlen] = 0x80;

    if inlen < 56 {
        let mut i: usize = inlen + 1;
        while i < 56 {
            padded[i] = 0;
            i += 1;
        }
        padded[56] = (bytes >> 53) as u8;
        padded[57] = (bytes >> 45) as u8;
        padded[58] = (bytes >> 37) as u8;
        padded[59] = (bytes >> 29) as u8;
        padded[60] = (bytes >> 21) as u8;
        padded[61] = (bytes >> 13) as u8;
        padded[62] = (bytes >> 5) as u8;
        padded[63] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(state, padded.as_ptr(), 64);
    } else {
        let mut i: usize = inlen + 1;
        while i < 120 {
            padded[i] = 0;
            i += 1;
        }
        padded[120] = (bytes >> 53) as u8;
        padded[121] = (bytes >> 45) as u8;
        padded[122] = (bytes >> 37) as u8;
        padded[123] = (bytes >> 29) as u8;
        padded[124] = (bytes >> 21) as u8;
        padded[125] = (bytes >> 13) as u8;
        padded[126] = (bytes >> 5) as u8;
        padded[127] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(state, padded.as_ptr(), 128);
    }

    let mut i: usize = 0;
    while i < 32 {
        *out.add(i) = *state.add(i);
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512_inc_finalize(out: *mut u8, state: *mut u8, mut in_: *const u8, mut inlen: usize) {
    let mut padded = [0u8; 256];
    let bytes: u64 = load_bigendian_64(state.add(64)).wrapping_add(inlen as u64);

    crypto_hashblocks_sha512(state, in_, inlen as u64);
    in_ = in_.add(inlen);
    inlen &= 127;
    in_ = in_.sub(inlen);

    let mut i: usize = 0;
    while i < inlen {
        padded[i] = *in_.add(i);
        i += 1;
    }
    padded[inlen] = 0x80;

    if inlen < 112 {
        let mut i: usize = inlen + 1;
        while i < 119 {
            padded[i] = 0;
            i += 1;
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
        crypto_hashblocks_sha512(state, padded.as_ptr(), 128);
    } else {
        let mut i: usize = inlen + 1;
        while i < 247 {
            padded[i] = 0;
            i += 1;
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
        crypto_hashblocks_sha512(state, padded.as_ptr(), 256);
    }

    let mut i: usize = 0;
    while i < 64 {
        *out.add(i) = *state.add(i);
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha256(out: *mut u8, in_: *const u8, inlen: usize) {
    let mut state = [0u8; 40];
    sha256_inc_init(state.as_mut_ptr());
    sha256_inc_finalize(out, state.as_mut_ptr(), in_, inlen);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha512(out: *mut u8, in_: *const u8, inlen: usize) {
    let mut state = [0u8; 72];
    sha512_inc_init(state.as_mut_ptr());
    sha512_inc_finalize(out, state.as_mut_ptr(), in_, inlen);
}


use core::ffi::c_ulong;
use crate::context::SpxCtx;

/// mgf1 function based on the SHA-256 hash function
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_mgf1_256(
    mut out: *mut u8,
    outlen: c_ulong,
    in_: *const u8,
    inlen: c_ulong,
) {
    let mut inbuf = vec![0u8; (inlen as usize) + 4];
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut i: c_ulong;

    core::ptr::copy_nonoverlapping(in_, inbuf.as_mut_ptr(), inlen as usize);

    /* While we can fit in at least another full block of SHA256 output.. */
    i = 0;
    while (i + 1) * (SPX_SHA256_OUTPUT_BYTES as c_ulong) <= outlen {
        crate::utils::SPX_u32_to_bytes(inbuf.as_mut_ptr().add(inlen as usize), i as u32);
        sha256(out, inbuf.as_ptr(), (inlen as usize) + 4);
        out = out.add(SPX_SHA256_OUTPUT_BYTES);
        i += 1;
    }
    /* Until we cannot anymore, and we fill the remainder. */
    if outlen > i * (SPX_SHA256_OUTPUT_BYTES as c_ulong) {
        crate::utils::SPX_u32_to_bytes(inbuf.as_mut_ptr().add(inlen as usize), i as u32);
        sha256(outbuf.as_mut_ptr(), inbuf.as_ptr(), (inlen as usize) + 4);
        core::ptr::copy_nonoverlapping(
            outbuf.as_ptr(),
            out,
            (outlen - i * (SPX_SHA256_OUTPUT_BYTES as c_ulong)) as usize,
        );
    }
}

/// mgf1 function based on the SHA-512 hash function
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_mgf1_512(
    mut out: *mut u8,
    outlen: c_ulong,
    in_: *const u8,
    inlen: c_ulong,
) {
    let mut inbuf = vec![0u8; (inlen as usize) + 4];
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut i: c_ulong;

    core::ptr::copy_nonoverlapping(in_, inbuf.as_mut_ptr(), inlen as usize);

    /* While we can fit in at least another full block of SHA512 output.. */
    i = 0;
    while (i + 1) * (SPX_SHA512_OUTPUT_BYTES as c_ulong) <= outlen {
        crate::utils::SPX_u32_to_bytes(inbuf.as_mut_ptr().add(inlen as usize), i as u32);
        sha512(out, inbuf.as_ptr(), (inlen as usize) + 4);
        out = out.add(SPX_SHA512_OUTPUT_BYTES);
        i += 1;
    }
    /* Until we cannot anymore, and we fill the remainder. */
    if outlen > i * (SPX_SHA512_OUTPUT_BYTES as c_ulong) {
        crate::utils::SPX_u32_to_bytes(inbuf.as_mut_ptr().add(inlen as usize), i as u32);
        sha512(outbuf.as_mut_ptr(), inbuf.as_ptr(), (inlen as usize) + 4);
        core::ptr::copy_nonoverlapping(
            outbuf.as_ptr(),
            out,
            (outlen - i * (SPX_SHA512_OUTPUT_BYTES as c_ulong)) as usize,
        );
    }
}

/* seed the SHA-512 state; only present when SPX_SHA512 (SPX_N >= 24) */
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
#[inline]
unsafe fn seed_state_512(ctx: *mut SpxCtx, block: *const u8) {
    if SPX_SHA512 != 0 {
        sha512_inc_init((*ctx).state_seeded_512.as_mut_ptr());
        sha512_inc_blocks((*ctx).state_seeded_512.as_mut_ptr(), block, 1);
    }
}

#[cfg(not(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f")))]
#[inline]
unsafe fn seed_state_512(_ctx: *mut SpxCtx, _block: *const u8) {}

/**
 * Absorb the constant pub_seed using one round of the compression function
 * This initializes state_seeded and state_seeded_512, which can then be
 * reused in thash
 **/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_seed_state(ctx: *mut SpxCtx) {
    let mut block = [0u8; SPX_SHA512_BLOCK_BYTES];
    let mut i: usize;

    i = 0;
    while i < SPX_N {
        block[i] = (*ctx).pub_seed[i];
        i += 1;
    }
    i = SPX_N;
    while i < SPX_SHA512_BLOCK_BYTES {
        block[i] = 0;
        i += 1;
    }
    /* block has been properly initialized for both SHA-256 and SHA-512 */

    sha256_inc_init((*ctx).state_seeded.as_mut_ptr());
    sha256_inc_blocks((*ctx).state_seeded.as_mut_ptr(), block.as_ptr(), 1);
    seed_state_512(ctx, block.as_ptr());
}
