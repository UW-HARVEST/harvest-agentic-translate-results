//! Translation of:
//! * `crypto_hash/sha512/hash_sha512.c`
//! * `crypto_hash/sha512/cp/hash_sha512_cp.c`
#![allow(dead_code)]

use core::ffi::c_int;
use core::ffi::c_void;

use crate::common::{load64_be, rotr64, store64_be};
use crate::types::crypto_hash_sha512_state;

extern "C" {
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

// ---- crypto_hash/sha512/hash_sha512.c ----

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha512_bytes() -> usize {
    64
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha512_statebytes() -> usize {
    core::mem::size_of::<crypto_hash_sha512_state>()
}

// ---- crypto_hash/sha512/cp/hash_sha512_cp.c ----

#[inline(always)]
unsafe fn be64enc_vect(dst: *mut u8, src: *const u64, len: usize) {
    for i in 0..(len / 8) {
        store64_be(dst.add(i * 8), *src.add(i));
    }
}

#[inline(always)]
unsafe fn be64dec_vect(dst: *mut u64, src: *const u8, len: usize) {
    for i in 0..(len / 8) {
        *dst.add(i) = load64_be(src.add(i * 8));
    }
}

static KRND: [u64; 80] = [
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

/// One `RND` macro application, operating on the rotating-index `S[8]` state
/// array, mirroring the literal unrolled expansion in the reference C.
#[inline(always)]
unsafe fn rnd512(s: *mut u64, w: *const u64, idx: usize, j: usize) {
    let se = *s.add((84 - j) % 8);
    let sf = *s.add((85 - j) % 8);
    let sg = *s.add((86 - j) % 8);

    let t0 = (rotr64(se, 14) ^ rotr64(se, 18) ^ rotr64(se, 41))
        .wrapping_add((se & (sf ^ sg)) ^ sg)
        .wrapping_add(*w.add(idx))
        .wrapping_add(KRND[idx]);

    let p87 = s.add((87 - j) % 8);
    *p87 = (*p87).wrapping_add(t0);

    let p83 = s.add((83 - j) % 8);
    *p83 = (*p83).wrapping_add(*p87);

    let sa = *s.add((80 - j) % 8);
    let sb = *s.add((81 - j) % 8);
    let sc = *s.add((82 - j) % 8);
    let t1 = (rotr64(sa, 28) ^ rotr64(sa, 34) ^ rotr64(sa, 39))
        .wrapping_add((sa & (sb | sc)) | (sb & sc));

    let p87 = s.add((87 - j) % 8);
    *p87 = (*p87).wrapping_add(t1);
}

/// One `MSCH` macro application (message schedule extension).
#[inline(always)]
unsafe fn msch512(w: *mut u64, idx: usize) {
    let w14 = *w.add(idx + 14);
    let w9 = *w.add(idx + 9);
    let w1 = *w.add(idx + 1);
    let w0 = *w.add(idx);

    let val = (rotr64(w14, 19) ^ rotr64(w14, 61) ^ (w14 >> 6))
        .wrapping_add(w9)
        .wrapping_add(rotr64(w1, 1) ^ rotr64(w1, 8) ^ (w1 >> 7))
        .wrapping_add(w0);

    *w.add(idx + 16) = val;
}

unsafe fn sha512_transform(state: *mut u64, block: *const u8, w: *mut u64, s: *mut u64) {
    be64dec_vect(w, block, 128);
    core::ptr::copy_nonoverlapping(state, s, 8);

    let mut i = 0usize;
    loop {
        for j in 0..16usize {
            rnd512(s, w, i + j, j);
        }
        if i == 64 {
            break;
        }
        for j in 0..16usize {
            msch512(w, i + j);
        }
        i += 16;
    }

    for k in 0..8usize {
        let p = state.add(k);
        *p = (*p).wrapping_add(*s.add(k));
    }
}

static PAD: [u8; 128] = {
    let mut pad = [0u8; 128];
    pad[0] = 0x80;
    pad
};

unsafe fn sha512_pad(state: *mut crypto_hash_sha512_state, tmp64: *mut u64) {
    let r = (((*state).count[1] >> 3) & 0x7f) as usize;

    if r < 112 {
        for i in 0..(112 - r) {
            (*state).buf[r + i] = PAD[i];
        }
    } else {
        for i in 0..(128 - r) {
            (*state).buf[r + i] = PAD[i];
        }
        sha512_transform(
            (*state).state.as_mut_ptr(),
            (*state).buf.as_ptr(),
            tmp64,
            tmp64.add(80),
        );
        for i in 0..112 {
            (*state).buf[i] = 0;
        }
    }
    be64enc_vect(
        (*state).buf.as_mut_ptr().add(112),
        (*state).count.as_ptr(),
        16,
    );
    sha512_transform(
        (*state).state.as_mut_ptr(),
        (*state).buf.as_ptr(),
        tmp64,
        tmp64.add(80),
    );
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha512_init(state: *mut crypto_hash_sha512_state) -> c_int {
    static SHA512_INITIAL_STATE: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];

    (*state).count[0] = 0u64;
    (*state).count[1] = 0u64;
    (*state).state = SHA512_INITIAL_STATE;

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha512_update(
    state: *mut crypto_hash_sha512_state,
    inp: *const u8,
    inlen: u64,
) -> c_int {
    let mut tmp64: [u64; 80 + 8] = [0; 80 + 8];
    let mut inp = inp;
    let mut inlen = inlen;

    if inlen == 0 {
        return 0;
    }

    let r = ((*state).count[1] >> 3) & 0x7f;

    let bitlen1 = inlen << 3;
    let bitlen0 = inlen >> 61;

    let (new_count1, carry) = (*state).count[1].overflowing_add(bitlen1);
    (*state).count[1] = new_count1;
    if carry {
        (*state).count[0] = (*state).count[0].wrapping_add(1);
    }
    (*state).count[0] = (*state).count[0].wrapping_add(bitlen0);

    if inlen < 128 - r {
        for i in 0..inlen as usize {
            (*state).buf[r as usize + i] = *inp.add(i);
        }
        return 0;
    }

    for i in 0..(128 - r) as usize {
        (*state).buf[r as usize + i] = *inp.add(i);
    }
    sha512_transform(
        (*state).state.as_mut_ptr(),
        (*state).buf.as_ptr(),
        tmp64.as_mut_ptr(),
        tmp64.as_mut_ptr().add(80),
    );
    inp = inp.add((128 - r) as usize);
    inlen -= 128 - r;

    while inlen >= 128 {
        sha512_transform(
            (*state).state.as_mut_ptr(),
            inp,
            tmp64.as_mut_ptr(),
            tmp64.as_mut_ptr().add(80),
        );
        inp = inp.add(128);
        inlen -= 128;
    }

    inlen &= 127;
    for i in 0..inlen as usize {
        (*state).buf[i] = *inp.add(i);
    }
    sodium_memzero(
        tmp64.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&tmp64),
    );

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha512_final(
    state: *mut crypto_hash_sha512_state,
    out: *mut u8,
) -> c_int {
    let mut tmp64: [u64; 80 + 8] = [0; 80 + 8];

    sha512_pad(state, tmp64.as_mut_ptr());
    be64enc_vect(out, (*state).state.as_ptr(), 64);
    sodium_memzero(
        tmp64.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&tmp64),
    );
    sodium_memzero(
        state as *mut c_void,
        core::mem::size_of::<crypto_hash_sha512_state>(),
    );

    0
}

#[no_mangle]
pub unsafe extern "C" fn crypto_hash_sha512(out: *mut u8, inp: *const u8, inlen: u64) -> c_int {
    let mut state: crypto_hash_sha512_state = crypto_hash_sha512_state {
        state: [0; 8],
        count: [0; 2],
        buf: [0; 128],
    };

    crypto_hash_sha512_init(&mut state);
    crypto_hash_sha512_update(&mut state, inp, inlen);
    crypto_hash_sha512_final(&mut state, out);

    0
}
