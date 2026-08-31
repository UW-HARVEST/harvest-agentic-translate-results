//! Translation of c_src/libsodium/crypto_hash/sha256/cp/hash_sha256_cp.c

use crate::common::{load32_be, store32_be, store64_be, rotr32};
use core::ffi::c_int;

// Public API struct, layout from include/sodium/crypto_hash_sha256.h.
// No #pragma pack in the header, so plain repr(C).
#[repr(C)]
pub struct crypto_hash_sha256_state {
    pub state: [u32; 8],
    pub count: u64,
    pub buf: [u8; 64],
}

extern "C" {
    fn sodium_memzero(pnt: *mut core::ffi::c_void, len: usize);
}

unsafe fn be32enc_vect(dst: *mut u8, src: *const u32, len: usize) {
    let mut i: usize = 0;
    while i < len / 4 {
        store32_be(dst.add(i * 4), *src.add(i));
        i += 1;
    }
}

// HAVE_SHA256_ARMCRYPTO undefined: portable variant.
unsafe fn be32dec_vect(dst: *mut u32, src: *const u8, len: usize) {
    let mut i: usize = 0;
    while i < len / 4 {
        *dst.add(i) = load32_be(src.add(i * 4));
        i += 1;
    }
}

static Krnd: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
    0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
    0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
    0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

// Macros expanded as inline helpers; all arithmetic wrapping.
#[inline(always)]
fn ch(x: u32, y: u32, z: u32) -> u32 {
    (x & (y ^ z)) ^ z
}
#[inline(always)]
fn maj(x: u32, y: u32, z: u32) -> u32 {
    (x & (y | z)) | (y & z)
}
#[inline(always)]
fn shr(x: u32, n: i32) -> u32 {
    x >> n
}
#[inline(always)]
fn rotr(x: u32, n: i32) -> u32 {
    rotr32(x, n)
}
#[inline(always)]
fn big_s0(x: u32) -> u32 {
    rotr(x, 2) ^ rotr(x, 13) ^ rotr(x, 22)
}
#[inline(always)]
fn big_s1(x: u32) -> u32 {
    rotr(x, 6) ^ rotr(x, 11) ^ rotr(x, 25)
}
#[inline(always)]
fn sml_s0(x: u32) -> u32 {
    rotr(x, 7) ^ rotr(x, 18) ^ shr(x, 3)
}
#[inline(always)]
fn sml_s1(x: u32) -> u32 {
    rotr(x, 17) ^ rotr(x, 19) ^ shr(x, 10)
}

// RND(a,b,c,d,e,f,g,h,k): mutates d and h in place; operates on S indexed
// by the RNDr macro. We expand RNDr inline in the transform below.
#[inline(always)]
unsafe fn rndr(s: *mut u32, w: *const u32, i: usize, ii: usize, imod: i32) {
    // Index expressions from the RNDr macro: S[(64 - i) % 8] etc.
    let ia = ((64 - i) % 8) as usize;
    let ib = ((65 - i) % 8) as usize;
    let ic = ((66 - i) % 8) as usize;
    let id = ((67 - i) % 8) as usize;
    let ie = ((68 - i) % 8) as usize;
    let if_ = ((69 - i) % 8) as usize;
    let ig = ((70 - i) % 8) as usize;
    let ih = ((71 - i) % 8) as usize;
    let _ = imod;

    let k = (*w.add(i + ii)).wrapping_add(Krnd[i + ii]);

    // RND expansion (a=ia, b=ib, c=ic, d=id, e=ie, f=if_, g=ig, h=ih)
    let a = *s.add(ia);
    let b = *s.add(ib);
    let c = *s.add(ic);
    let e = *s.add(ie);
    let f = *s.add(if_);
    let g = *s.add(ig);

    // h += S1(e) + Ch(e,f,g) + k;
    let mut h = *s.add(ih);
    h = h
        .wrapping_add(big_s1(e))
        .wrapping_add(ch(e, f, g))
        .wrapping_add(k);
    *s.add(ih) = h;
    // d += h;
    *s.add(id) = (*s.add(id)).wrapping_add(h);
    // h += S0(a) + Maj(a,b,c);
    h = h.wrapping_add(big_s0(a)).wrapping_add(maj(a, b, c));
    *s.add(ih) = h;
}

#[inline(always)]
unsafe fn msch(w: *mut u32, ii: usize, i: usize) {
    // W[i+ii+16] = s1(W[i+ii+14]) + W[i+ii+9] + s0(W[i+ii+1]) + W[i+ii]
    let v = sml_s1(*w.add(i + ii + 14))
        .wrapping_add(*w.add(i + ii + 9))
        .wrapping_add(sml_s0(*w.add(i + ii + 1)))
        .wrapping_add(*w.add(i + ii));
    *w.add(i + ii + 16) = v;
}

unsafe fn sha256_transform(state: *mut u32, block: *const u8, w: *mut u32, s: *mut u32) {
    be32dec_vect(w, block, 64);
    core::ptr::copy_nonoverlapping(state as *const u8, s as *mut u8, 32);
    let mut i: i32 = 0;
    while i < 64 {
        let iu = i as usize;
        rndr(s, w, 0, iu, i);
        rndr(s, w, 1, iu, i);
        rndr(s, w, 2, iu, i);
        rndr(s, w, 3, iu, i);
        rndr(s, w, 4, iu, i);
        rndr(s, w, 5, iu, i);
        rndr(s, w, 6, iu, i);
        rndr(s, w, 7, iu, i);
        rndr(s, w, 8, iu, i);
        rndr(s, w, 9, iu, i);
        rndr(s, w, 10, iu, i);
        rndr(s, w, 11, iu, i);
        rndr(s, w, 12, iu, i);
        rndr(s, w, 13, iu, i);
        rndr(s, w, 14, iu, i);
        rndr(s, w, 15, iu, i);
        if i == 48 {
            break;
        }
        msch(w, 0, iu);
        msch(w, 1, iu);
        msch(w, 2, iu);
        msch(w, 3, iu);
        msch(w, 4, iu);
        msch(w, 5, iu);
        msch(w, 6, iu);
        msch(w, 7, iu);
        msch(w, 8, iu);
        msch(w, 9, iu);
        msch(w, 10, iu);
        msch(w, 11, iu);
        msch(w, 12, iu);
        msch(w, 13, iu);
        msch(w, 14, iu);
        msch(w, 15, iu);
        i += 16;
    }
    let mut j: usize = 0;
    while j < 8 {
        *state.add(j) = (*state.add(j)).wrapping_add(*s.add(j));
        j += 1;
    }
}

static PAD: [u8; 64] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0,
];

unsafe fn sha256_pad(state: *mut crypto_hash_sha256_state, tmp32: *mut u32) {
    let r: u32;
    // ACQUIRE_FENCE -> nothing
    let count = core::ptr::addr_of!((*state).count).read_unaligned();
    r = ((count >> 3) & 0x3f) as u32;
    if r < 56 {
        let mut i: u32 = 0;
        while i < 56 - r {
            let buf = core::ptr::addr_of_mut!((*state).buf) as *mut u8;
            *buf.add((r + i) as usize) = PAD[i as usize];
            i += 1;
        }
    } else {
        let mut i: u32 = 0;
        while i < 64 - r {
            let buf = core::ptr::addr_of_mut!((*state).buf) as *mut u8;
            *buf.add((r + i) as usize) = PAD[i as usize];
            i += 1;
        }
        let statep = core::ptr::addr_of_mut!((*state).state) as *mut u32;
        let bufp = core::ptr::addr_of_mut!((*state).buf) as *const u8;
        sha256_transform(statep, bufp, tmp32, tmp32.add(64));
        let bufm = core::ptr::addr_of_mut!((*state).buf) as *mut u8;
        core::ptr::write_bytes(bufm, 0, 56);
    }
    let bufm = core::ptr::addr_of_mut!((*state).buf) as *mut u8;
    store64_be(bufm.add(56), count);
    let statep = core::ptr::addr_of_mut!((*state).state) as *mut u32;
    let bufp = core::ptr::addr_of_mut!((*state).buf) as *const u8;
    sha256_transform(statep, bufp, tmp32, tmp32.add(64));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_init(
    state: *mut crypto_hash_sha256_state,
) -> c_int {
    static sha256_initial_state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    core::ptr::addr_of_mut!((*state).count).write_unaligned(0u64);
    let statep = core::ptr::addr_of_mut!((*state).state) as *mut u8;
    core::ptr::copy_nonoverlapping(
        sha256_initial_state.as_ptr() as *const u8,
        statep,
        core::mem::size_of::<[u32; 8]>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_update(
    state: *mut crypto_hash_sha256_state,
    mut in_: *const u8,
    mut inlen: u64,
) -> c_int {
    let mut tmp32: [u32; 64 + 8] = [0; 64 + 8];
    let mut i: u64;
    let r: u64;

    if inlen == 0 {
        return 0;
    }
    // ACQUIRE_FENCE -> nothing
    let count = core::ptr::addr_of!((*state).count).read_unaligned();
    r = (count >> 3) & 0x3f;

    let new_count = count.wrapping_add((inlen as u64) << 3);
    core::ptr::addr_of_mut!((*state).count).write_unaligned(new_count);

    let buf = core::ptr::addr_of_mut!((*state).buf) as *mut u8;
    if inlen < 64 - r {
        i = 0;
        while i < inlen {
            *buf.add((r + i) as usize) = *in_.add(i as usize);
            i += 1;
        }
        return 0;
    }
    i = 0;
    while i < 64 - r {
        *buf.add((r + i) as usize) = *in_.add(i as usize);
        i += 1;
    }
    let statep = core::ptr::addr_of_mut!((*state).state) as *mut u32;
    let bufp = core::ptr::addr_of_mut!((*state).buf) as *const u8;
    sha256_transform(statep, bufp, tmp32.as_mut_ptr(), tmp32.as_mut_ptr().add(64));
    in_ = in_.add((64 - r) as usize);
    inlen -= 64 - r;

    while inlen >= 64 {
        sha256_transform(statep, in_, tmp32.as_mut_ptr(), tmp32.as_mut_ptr().add(64));
        in_ = in_.add(64);
        inlen -= 64;
    }
    inlen &= 63;
    i = 0;
    while i < inlen {
        *buf.add(i as usize) = *in_.add(i as usize);
        i += 1;
    }
    sodium_memzero(
        tmp32.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<[u32; 64 + 8]>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_final(
    state: *mut crypto_hash_sha256_state,
    out: *mut u8,
) -> c_int {
    let mut tmp32: [u32; 64 + 8] = [0; 64 + 8];

    sha256_pad(state, tmp32.as_mut_ptr());
    let statep = core::ptr::addr_of!((*state).state) as *const u32;
    be32enc_vect(out, statep, 32);
    sodium_memzero(
        tmp32.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<[u32; 64 + 8]>(),
    );
    sodium_memzero(
        state as *mut core::ffi::c_void,
        core::mem::size_of::<crypto_hash_sha256_state>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    let mut state = core::mem::MaybeUninit::<crypto_hash_sha256_state>::uninit();
    let state = state.as_mut_ptr();

    crypto_hash_sha256_init(state);
    crypto_hash_sha256_update(state, in_, inlen);
    crypto_hash_sha256_final(state, out);

    0
}
