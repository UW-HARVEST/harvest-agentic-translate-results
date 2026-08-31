//! Translation of c_src/libsodium/crypto_hash/sha512/cp/hash_sha512_cp.c

use crate::common::{load64_be, store64_be, rotr64};
use core::ffi::c_int;

// Public API struct, layout from include/sodium/crypto_hash_sha512.h.
// No #pragma pack in the header, so plain repr(C).
#[repr(C)]
pub struct crypto_hash_sha512_state {
    pub state: [u64; 8],
    pub count: [u64; 2],
    pub buf: [u8; 128],
}

extern "C" {
    fn sodium_memzero(pnt: *mut core::ffi::c_void, len: usize);
}

unsafe fn be64enc_vect(dst: *mut u8, src: *const u64, len: usize) {
    let mut i: usize = 0;
    while i < len / 8 {
        store64_be(dst.add(i * 8), *src.add(i));
        i += 1;
    }
}

unsafe fn be64dec_vect(dst: *mut u64, src: *const u8, len: usize) {
    let mut i: usize = 0;
    while i < len / 8 {
        *dst.add(i) = load64_be(src.add(i * 8));
        i += 1;
    }
}

static Krnd: [u64; 80] = [
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
    0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
    0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
    0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
    0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
    0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
    0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
    0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
    0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
    0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
];

#[inline(always)]
fn ch(x: u64, y: u64, z: u64) -> u64 {
    (x & (y ^ z)) ^ z
}
#[inline(always)]
fn maj(x: u64, y: u64, z: u64) -> u64 {
    (x & (y | z)) | (y & z)
}
#[inline(always)]
fn shr(x: u64, n: i32) -> u64 {
    x >> n
}
#[inline(always)]
fn rotr(x: u64, n: i32) -> u64 {
    rotr64(x, n)
}
#[inline(always)]
fn big_s0(x: u64) -> u64 {
    rotr(x, 28) ^ rotr(x, 34) ^ rotr(x, 39)
}
#[inline(always)]
fn big_s1(x: u64) -> u64 {
    rotr(x, 14) ^ rotr(x, 18) ^ rotr(x, 41)
}
#[inline(always)]
fn sml_s0(x: u64) -> u64 {
    rotr(x, 1) ^ rotr(x, 8) ^ shr(x, 7)
}
#[inline(always)]
fn sml_s1(x: u64) -> u64 {
    rotr(x, 19) ^ rotr(x, 61) ^ shr(x, 6)
}

#[inline(always)]
unsafe fn rndr(s: *mut u64, w: *const u64, i: usize, ii: usize) {
    // Index expressions from RNDr: S[(80 - i) % 8] etc.
    let ia = ((80 - i) % 8) as usize;
    let ib = ((81 - i) % 8) as usize;
    let ic = ((82 - i) % 8) as usize;
    let id = ((83 - i) % 8) as usize;
    let ie = ((84 - i) % 8) as usize;
    let if_ = ((85 - i) % 8) as usize;
    let ig = ((86 - i) % 8) as usize;
    let ih = ((87 - i) % 8) as usize;

    let k = (*w.add(i + ii)).wrapping_add(Krnd[i + ii]);

    let a = *s.add(ia);
    let b = *s.add(ib);
    let c = *s.add(ic);
    let e = *s.add(ie);
    let f = *s.add(if_);
    let g = *s.add(ig);

    let mut h = *s.add(ih);
    h = h
        .wrapping_add(big_s1(e))
        .wrapping_add(ch(e, f, g))
        .wrapping_add(k);
    *s.add(ih) = h;
    *s.add(id) = (*s.add(id)).wrapping_add(h);
    h = h.wrapping_add(big_s0(a)).wrapping_add(maj(a, b, c));
    *s.add(ih) = h;
}

#[inline(always)]
unsafe fn msch(w: *mut u64, ii: usize, i: usize) {
    let v = sml_s1(*w.add(i + ii + 14))
        .wrapping_add(*w.add(i + ii + 9))
        .wrapping_add(sml_s0(*w.add(i + ii + 1)))
        .wrapping_add(*w.add(i + ii));
    *w.add(i + ii + 16) = v;
}

unsafe fn sha512_transform(state: *mut u64, block: *const u8, w: *mut u64, s: *mut u64) {
    be64dec_vect(w, block, 128);
    core::ptr::copy_nonoverlapping(state as *const u8, s as *mut u8, 64);
    let mut i: i32 = 0;
    while i < 80 {
        let iu = i as usize;
        rndr(s, w, 0, iu);
        rndr(s, w, 1, iu);
        rndr(s, w, 2, iu);
        rndr(s, w, 3, iu);
        rndr(s, w, 4, iu);
        rndr(s, w, 5, iu);
        rndr(s, w, 6, iu);
        rndr(s, w, 7, iu);
        rndr(s, w, 8, iu);
        rndr(s, w, 9, iu);
        rndr(s, w, 10, iu);
        rndr(s, w, 11, iu);
        rndr(s, w, 12, iu);
        rndr(s, w, 13, iu);
        rndr(s, w, 14, iu);
        rndr(s, w, 15, iu);
        if i == 64 {
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

static PAD: [u8; 128] = {
    let mut p = [0u8; 128];
    p[0] = 0x80;
    p
};

unsafe fn sha512_pad(state: *mut crypto_hash_sha512_state, tmp64: *mut u64) {
    let r: u32;
    // ACQUIRE_FENCE -> nothing
    let count1 = core::ptr::addr_of!((*state).count[1]).read_unaligned();
    r = ((count1 >> 3) & 0x7f) as u32;
    if r < 112 {
        let mut i: u32 = 0;
        while i < 112 - r {
            let buf = core::ptr::addr_of_mut!((*state).buf) as *mut u8;
            *buf.add((r + i) as usize) = PAD[i as usize];
            i += 1;
        }
    } else {
        let mut i: u32 = 0;
        while i < 128 - r {
            let buf = core::ptr::addr_of_mut!((*state).buf) as *mut u8;
            *buf.add((r + i) as usize) = PAD[i as usize];
            i += 1;
        }
        let statep = core::ptr::addr_of_mut!((*state).state) as *mut u64;
        let bufp = core::ptr::addr_of_mut!((*state).buf) as *const u8;
        sha512_transform(statep, bufp, tmp64, tmp64.add(80));
        let bufm = core::ptr::addr_of_mut!((*state).buf) as *mut u8;
        core::ptr::write_bytes(bufm, 0, 112);
    }
    let bufm = core::ptr::addr_of_mut!((*state).buf) as *mut u8;
    let countp = core::ptr::addr_of!((*state).count) as *const u64;
    be64enc_vect(bufm.add(112), countp, 16);
    let statep = core::ptr::addr_of_mut!((*state).state) as *mut u64;
    let bufp = core::ptr::addr_of_mut!((*state).buf) as *const u8;
    sha512_transform(statep, bufp, tmp64, tmp64.add(80));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_init(
    state: *mut crypto_hash_sha512_state,
) -> c_int {
    static sha512_initial_state: [u64; 8] = [
        0x6a09e667f3bcc908, 0xbb67ae8584caa73b, 0x3c6ef372fe94f82b, 0xa54ff53a5f1d36f1,
        0x510e527fade682d1, 0x9b05688c2b3e6c1f, 0x1f83d9abfb41bd6b, 0x5be0cd19137e2179,
    ];

    core::ptr::addr_of_mut!((*state).count[0]).write_unaligned(0u64);
    core::ptr::addr_of_mut!((*state).count[1]).write_unaligned(0u64);
    let statep = core::ptr::addr_of_mut!((*state).state) as *mut u8;
    core::ptr::copy_nonoverlapping(
        sha512_initial_state.as_ptr() as *const u8,
        statep,
        core::mem::size_of::<[u64; 8]>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_update(
    state: *mut crypto_hash_sha512_state,
    mut in_: *const u8,
    mut inlen: u64,
) -> c_int {
    let mut tmp64: [u64; 80 + 8] = [0; 80 + 8];
    let mut bitlen: [u64; 2] = [0; 2];
    let mut i: u64;
    let r: u64;

    if inlen == 0 {
        return 0;
    }
    // ACQUIRE_FENCE -> nothing
    let count1 = core::ptr::addr_of!((*state).count[1]).read_unaligned();
    r = (count1 >> 3) & 0x7f;

    bitlen[1] = (inlen as u64) << 3;
    bitlen[0] = (inlen as u64) >> 61;
    // LCOV_EXCL_START
    let new_c1 = count1.wrapping_add(bitlen[1]);
    core::ptr::addr_of_mut!((*state).count[1]).write_unaligned(new_c1);
    if new_c1 < bitlen[1] {
        let c0 = core::ptr::addr_of!((*state).count[0]).read_unaligned();
        core::ptr::addr_of_mut!((*state).count[0]).write_unaligned(c0.wrapping_add(1));
    }
    // LCOV_EXCL_STOP
    let c0 = core::ptr::addr_of!((*state).count[0]).read_unaligned();
    core::ptr::addr_of_mut!((*state).count[0]).write_unaligned(c0.wrapping_add(bitlen[0]));

    let buf = core::ptr::addr_of_mut!((*state).buf) as *mut u8;
    if inlen < 128 - r {
        i = 0;
        while i < inlen {
            *buf.add((r + i) as usize) = *in_.add(i as usize);
            i += 1;
        }
        return 0;
    }
    i = 0;
    while i < 128 - r {
        *buf.add((r + i) as usize) = *in_.add(i as usize);
        i += 1;
    }
    let statep = core::ptr::addr_of_mut!((*state).state) as *mut u64;
    let bufp = core::ptr::addr_of_mut!((*state).buf) as *const u8;
    sha512_transform(statep, bufp, tmp64.as_mut_ptr(), tmp64.as_mut_ptr().add(80));
    in_ = in_.add((128 - r) as usize);
    inlen -= 128 - r;

    while inlen >= 128 {
        sha512_transform(statep, in_, tmp64.as_mut_ptr(), tmp64.as_mut_ptr().add(80));
        in_ = in_.add(128);
        inlen -= 128;
    }
    inlen &= 127;
    i = 0;
    while i < inlen {
        *buf.add(i as usize) = *in_.add(i as usize);
        i += 1;
    }
    sodium_memzero(
        tmp64.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<[u64; 80 + 8]>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512_final(
    state: *mut crypto_hash_sha512_state,
    out: *mut u8,
) -> c_int {
    let mut tmp64: [u64; 80 + 8] = [0; 80 + 8];

    sha512_pad(state, tmp64.as_mut_ptr());
    let statep = core::ptr::addr_of!((*state).state) as *const u64;
    be64enc_vect(out, statep, 64);
    sodium_memzero(
        tmp64.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<[u64; 80 + 8]>(),
    );
    sodium_memzero(
        state as *mut core::ffi::c_void,
        core::mem::size_of::<crypto_hash_sha512_state>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha512(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    let mut state = core::mem::MaybeUninit::<crypto_hash_sha512_state>::uninit();
    let state = state.as_mut_ptr();

    crypto_hash_sha512_init(state);
    crypto_hash_sha512_update(state, in_, inlen);
    crypto_hash_sha512_final(state, out);

    0
}
