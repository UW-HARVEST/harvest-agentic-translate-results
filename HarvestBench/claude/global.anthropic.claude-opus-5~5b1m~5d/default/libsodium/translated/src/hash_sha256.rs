//! Translation of:
//! * `crypto_hash/sha256/hash_sha256.c`
//! * `crypto_hash/sha256/cp/hash_sha256_cp.c`
#![allow(dead_code)]

use core::ffi::c_int;
use core::ffi::c_void;

use crate::common::{load32_be, rotr32, store32_be, store64_be};
use crate::types::crypto_hash_sha256_state;

extern "C" {
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

// ---- crypto_hash/sha256/hash_sha256.c ----

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha256_bytes() -> usize {
    32
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha256_statebytes() -> usize {
    core::mem::size_of::<crypto_hash_sha256_state>()
}

// ---- crypto_hash/sha256/cp/hash_sha256_cp.c ----

#[inline(always)]
unsafe fn be32enc_vect(dst: *mut u8, src: *const u32, len: usize) {
    for i in 0..(len / 4) {
        store32_be(dst.add(i * 4), *src.add(i));
    }
}

#[inline(always)]
unsafe fn be32dec_vect(dst: *mut u32, src: *const u8, len: usize) {
    for i in 0..(len / 4) {
        *dst.add(i) = load32_be(src.add(i * 4));
    }
}

static KRND: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// One `RND` macro application, operating on the rotating-index `S[8]` state
/// array, mirroring the literal unrolled expansion in the reference C.
#[inline(always)]
unsafe fn rnd256(s: *mut u32, w: *const u32, idx: usize, j: usize) {
    let se = *s.add((68 - j) % 8);
    let sf = *s.add((69 - j) % 8);
    let sg = *s.add((70 - j) % 8);

    let t0 = (rotr32(se, 6) ^ rotr32(se, 11) ^ rotr32(se, 25))
        .wrapping_add((se & (sf ^ sg)) ^ sg)
        .wrapping_add(*w.add(idx))
        .wrapping_add(KRND[idx]);

    let p71 = s.add((71 - j) % 8);
    *p71 = (*p71).wrapping_add(t0);

    let p67 = s.add((67 - j) % 8);
    *p67 = (*p67).wrapping_add(*p71);

    let sa = *s.add((64 - j) % 8);
    let sb = *s.add((65 - j) % 8);
    let sc = *s.add((66 - j) % 8);
    let t1 = (rotr32(sa, 2) ^ rotr32(sa, 13) ^ rotr32(sa, 22))
        .wrapping_add((sa & (sb | sc)) | (sb & sc));

    let p71 = s.add((71 - j) % 8);
    *p71 = (*p71).wrapping_add(t1);
}

/// One `MSCH` macro application (message schedule extension).
#[inline(always)]
unsafe fn msch256(w: *mut u32, idx: usize) {
    let w14 = *w.add(idx + 14);
    let w9 = *w.add(idx + 9);
    let w1 = *w.add(idx + 1);
    let w0 = *w.add(idx);

    let val = (rotr32(w14, 17) ^ rotr32(w14, 19) ^ (w14 >> 10))
        .wrapping_add(w9)
        .wrapping_add(rotr32(w1, 7) ^ rotr32(w1, 18) ^ (w1 >> 3))
        .wrapping_add(w0);

    *w.add(idx + 16) = val;
}

unsafe fn sha256_transform(state: *mut u32, block: *const u8, w: *mut u32, s: *mut u32) {
    be32dec_vect(w, block, 64);
    core::ptr::copy_nonoverlapping(state, s, 8);

    let mut i = 0usize;
    loop {
        for j in 0..16usize {
            rnd256(s, w, i + j, j);
        }
        if i == 48 {
            break;
        }
        for j in 0..16usize {
            msch256(w, i + j);
        }
        i += 16;
    }

    for k in 0..8usize {
        let p = state.add(k);
        *p = (*p).wrapping_add(*s.add(k));
    }
}

static PAD: [u8; 64] = {
    let mut pad = [0u8; 64];
    pad[0] = 0x80;
    pad
};

unsafe fn sha256_pad(state: *mut crypto_hash_sha256_state, tmp32: *mut u32) {
    let r = (((*state).count >> 3) & 0x3f) as usize;

    if r < 56 {
        for i in 0..(56 - r) {
            (*state).buf[r + i] = PAD[i];
        }
    } else {
        for i in 0..(64 - r) {
            (*state).buf[r + i] = PAD[i];
        }
        sha256_transform(
            (*state).state.as_mut_ptr(),
            (*state).buf.as_ptr(),
            tmp32,
            tmp32.add(64),
        );
        for i in 0..56 {
            (*state).buf[i] = 0;
        }
    }
    store64_be((*state).buf.as_mut_ptr().add(56), (*state).count);
    sha256_transform(
        (*state).state.as_mut_ptr(),
        (*state).buf.as_ptr(),
        tmp32,
        tmp32.add(64),
    );
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha256_init(state: *mut crypto_hash_sha256_state) -> c_int {
    static SHA256_INITIAL_STATE: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    (*state).count = 0u64;
    (*state).state = SHA256_INITIAL_STATE;

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha256_update(
    state: *mut crypto_hash_sha256_state,
    inp: *const u8,
    inlen: u64,
) -> c_int {
    let mut tmp32: [u32; 64 + 8] = [0; 64 + 8];
    let mut inp = inp;
    let mut inlen = inlen;

    if inlen == 0 {
        return 0;
    }

    let r = ((*state).count >> 3) & 0x3f;

    (*state).count = (*state).count.wrapping_add(inlen << 3);

    if inlen < 64 - r {
        for i in 0..inlen as usize {
            (*state).buf[r as usize + i] = *inp.add(i);
        }
        return 0;
    }

    for i in 0..(64 - r) as usize {
        (*state).buf[r as usize + i] = *inp.add(i);
    }
    sha256_transform(
        (*state).state.as_mut_ptr(),
        (*state).buf.as_ptr(),
        tmp32.as_mut_ptr(),
        tmp32.as_mut_ptr().add(64),
    );
    inp = inp.add((64 - r) as usize);
    inlen -= 64 - r;

    while inlen >= 64 {
        sha256_transform(
            (*state).state.as_mut_ptr(),
            inp,
            tmp32.as_mut_ptr(),
            tmp32.as_mut_ptr().add(64),
        );
        inp = inp.add(64);
        inlen -= 64;
    }

    inlen &= 63;
    for i in 0..inlen as usize {
        (*state).buf[i] = *inp.add(i);
    }
    sodium_memzero(
        tmp32.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&tmp32),
    );

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha256_final(
    state: *mut crypto_hash_sha256_state,
    out: *mut u8,
) -> c_int {
    let mut tmp32: [u32; 64 + 8] = [0; 64 + 8];

    sha256_pad(state, tmp32.as_mut_ptr());
    be32enc_vect(out, (*state).state.as_ptr(), 32);
    sodium_memzero(
        tmp32.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&tmp32),
    );
    sodium_memzero(
        state as *mut c_void,
        core::mem::size_of::<crypto_hash_sha256_state>(),
    );

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha256(out: *mut u8, inp: *const u8, inlen: u64) -> c_int {
    let mut state: crypto_hash_sha256_state = crypto_hash_sha256_state {
        state: [0; 8],
        count: 0,
        buf: [0; 64],
    };

    crypto_hash_sha256_init(&mut state);
    crypto_hash_sha256_update(&mut state, inp, inlen);
    crypto_hash_sha256_final(&mut state, out);

    0
}
