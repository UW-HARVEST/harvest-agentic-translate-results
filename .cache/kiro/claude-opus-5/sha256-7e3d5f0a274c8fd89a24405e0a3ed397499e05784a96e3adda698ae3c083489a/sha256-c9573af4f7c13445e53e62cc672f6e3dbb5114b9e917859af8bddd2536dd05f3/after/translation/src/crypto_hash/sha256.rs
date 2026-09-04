use core::ffi::c_int;

use crate::common::{load32_be, store32_be, store64_be};
use crate::sodium_utils::sodium_memzero;

pub const crypto_hash_sha256_BYTES: usize = 32;

#[repr(C)]
pub struct crypto_hash_sha256_state {
    pub state: [u32; 8],
    pub count: u64,
    pub buf: [u8; 64],
}

/* ---- from hash_sha256.c ---- */

#[unsafe(no_mangle)]
pub extern "C" fn crypto_hash_sha256_bytes() -> usize {
    crypto_hash_sha256_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_hash_sha256_statebytes() -> usize {
    core::mem::size_of::<crypto_hash_sha256_state>()
}

/* ---- from cp/hash_sha256_cp.c ---- */

unsafe fn be32enc_vect(dst: *mut u8, src: *const u32, len: usize) {
    let mut i: usize = 0;
    while i < len / 4 {
        store32_be(dst.add(i * 4), *src.add(i));
        i += 1;
    }
}

unsafe fn be32dec_vect(dst: *mut u32, src: *const u8, len: usize) {
    let mut i: usize = 0;
    while i < len / 4 {
        *dst.add(i) = load32_be(src.add(i * 4));
        i += 1;
    }
}

static Krnd: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

#[inline(always)]
fn Ch(x: u32, y: u32, z: u32) -> u32 {
    (x & (y ^ z)) ^ z
}
#[inline(always)]
fn Maj(x: u32, y: u32, z: u32) -> u32 {
    (x & (y | z)) | (y & z)
}
#[inline(always)]
fn SHR(x: u32, n: u32) -> u32 {
    x >> n
}
#[inline(always)]
fn ROTR(x: u32, n: i32) -> u32 {
    crate::common::rotr32(x, n)
}
#[inline(always)]
fn S0(x: u32) -> u32 {
    ROTR(x, 2) ^ ROTR(x, 13) ^ ROTR(x, 22)
}
#[inline(always)]
fn S1(x: u32) -> u32 {
    ROTR(x, 6) ^ ROTR(x, 11) ^ ROTR(x, 25)
}
#[inline(always)]
fn s0(x: u32) -> u32 {
    ROTR(x, 7) ^ ROTR(x, 18) ^ SHR(x, 3)
}
#[inline(always)]
fn s1(x: u32) -> u32 {
    ROTR(x, 17) ^ ROTR(x, 19) ^ SHR(x, 10)
}

unsafe fn SHA256_Transform(state: *mut u32, block: *const u8, W: *mut u32, S: *mut u32) {
    be32dec_vect(W, block, 64);
    core::ptr::copy_nonoverlapping(state as *const u8, S as *mut u8, 32);

    // RND macro expanded via helper closures using indices into S/W.
    // RNDr(S, W, i, ii): uses S[(64 - i)%8] as a, etc.
    macro_rules! rndr {
        ($i:expr, $ii:expr) => {{
            let a = (64 - $i) % 8;
            let b = (65 - $i) % 8;
            let c = (66 - $i) % 8;
            let d = (67 - $i) % 8;
            let e = (68 - $i) % 8;
            let f = (69 - $i) % 8;
            let g = (70 - $i) % 8;
            let h = (71 - $i) % 8;
            let k = (*W.add($i + $ii)).wrapping_add(Krnd[$i + $ii]);
            // h += S1(e) + Ch(e,f,g) + k;
            *S.add(h) = (*S.add(h))
                .wrapping_add(S1(*S.add(e)))
                .wrapping_add(Ch(*S.add(e), *S.add(f), *S.add(g)))
                .wrapping_add(k);
            // d += h;
            *S.add(d) = (*S.add(d)).wrapping_add(*S.add(h));
            // h += S0(a) + Maj(a,b,c);
            *S.add(h) = (*S.add(h)).wrapping_add(S0(*S.add(a))).wrapping_add(Maj(
                *S.add(a),
                *S.add(b),
                *S.add(c),
            ));
        }};
    }
    macro_rules! msch {
        ($ii:expr, $i:expr) => {{
            *W.add($i + $ii + 16) = s1(*W.add($i + $ii + 14))
                .wrapping_add(*W.add($i + $ii + 9))
                .wrapping_add(s0(*W.add($i + $ii + 1)))
                .wrapping_add(*W.add($i + $ii));
        }};
    }

    let mut i: usize = 0;
    while i < 64 {
        rndr!(0, i);
        rndr!(1, i);
        rndr!(2, i);
        rndr!(3, i);
        rndr!(4, i);
        rndr!(5, i);
        rndr!(6, i);
        rndr!(7, i);
        rndr!(8, i);
        rndr!(9, i);
        rndr!(10, i);
        rndr!(11, i);
        rndr!(12, i);
        rndr!(13, i);
        rndr!(14, i);
        rndr!(15, i);
        if i == 48 {
            break;
        }
        msch!(0, i);
        msch!(1, i);
        msch!(2, i);
        msch!(3, i);
        msch!(4, i);
        msch!(5, i);
        msch!(6, i);
        msch!(7, i);
        msch!(8, i);
        msch!(9, i);
        msch!(10, i);
        msch!(11, i);
        msch!(12, i);
        msch!(13, i);
        msch!(14, i);
        msch!(15, i);
        i += 16;
    }
    let mut i: usize = 0;
    while i < 8 {
        *state.add(i) = (*state.add(i)).wrapping_add(*S.add(i));
        i += 1;
    }
}

static PAD: [u8; 64] = {
    let mut p = [0u8; 64];
    p[0] = 0x80;
    p
};

unsafe fn SHA256_Pad(state: *mut crypto_hash_sha256_state, tmp32: *mut u32) {
    let r: u32 = (((*state).count >> 3) & 0x3f) as u32;
    if r < 56 {
        let mut i: u32 = 0;
        while i < 56 - r {
            (*state).buf[(r + i) as usize] = PAD[i as usize];
            i += 1;
        }
    } else {
        let mut i: u32 = 0;
        while i < 64 - r {
            (*state).buf[(r + i) as usize] = PAD[i as usize];
            i += 1;
        }
        SHA256_Transform(
            (*state).state.as_mut_ptr(),
            (*state).buf.as_ptr(),
            tmp32,
            tmp32.add(64),
        );
        core::ptr::write_bytes((*state).buf.as_mut_ptr(), 0, 56);
    }
    store64_be((*state).buf.as_mut_ptr().add(56), (*state).count);
    SHA256_Transform(
        (*state).state.as_mut_ptr(),
        (*state).buf.as_ptr(),
        tmp32,
        tmp32.add(64),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_init(state: *mut crypto_hash_sha256_state) -> c_int {
    static sha256_initial_state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    (*state).count = 0u64;
    core::ptr::copy_nonoverlapping(
        sha256_initial_state.as_ptr(),
        (*state).state.as_mut_ptr(),
        8,
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
    r = ((*state).count >> 3) & 0x3f;

    (*state).count = (*state).count.wrapping_add((inlen as u64) << 3);
    if inlen < 64 - r {
        i = 0;
        while i < inlen {
            (*state).buf[(r + i) as usize] = *in_.add(i as usize);
            i += 1;
        }
        return 0;
    }
    i = 0;
    while i < 64 - r {
        (*state).buf[(r + i) as usize] = *in_.add(i as usize);
        i += 1;
    }
    SHA256_Transform(
        (*state).state.as_mut_ptr(),
        (*state).buf.as_ptr(),
        tmp32.as_mut_ptr(),
        tmp32.as_mut_ptr().add(64),
    );
    in_ = in_.add((64 - r) as usize);
    inlen -= 64 - r;

    while inlen >= 64 {
        SHA256_Transform(
            (*state).state.as_mut_ptr(),
            in_,
            tmp32.as_mut_ptr(),
            tmp32.as_mut_ptr().add(64),
        );
        in_ = in_.add(64);
        inlen -= 64;
    }
    inlen &= 63;
    i = 0;
    while i < inlen {
        (*state).buf[i as usize] = *in_.add(i as usize);
        i += 1;
    }
    sodium_memzero(
        tmp32.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of_val(&tmp32),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_final(
    state: *mut crypto_hash_sha256_state,
    out: *mut u8,
) -> c_int {
    let mut tmp32: [u32; 64 + 8] = [0; 64 + 8];

    SHA256_Pad(state, tmp32.as_mut_ptr());
    be32enc_vect(out, (*state).state.as_ptr(), 32);
    sodium_memzero(
        tmp32.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of_val(&tmp32),
    );
    sodium_memzero(
        state as *mut core::ffi::c_void,
        core::mem::size_of::<crypto_hash_sha256_state>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256(out: *mut u8, in_: *const u8, inlen: u64) -> c_int {
    let mut state: crypto_hash_sha256_state = core::mem::zeroed();

    crypto_hash_sha256_init(&mut state);
    crypto_hash_sha256_update(&mut state, in_, inlen);
    crypto_hash_sha256_final(&mut state, out);

    0
}
