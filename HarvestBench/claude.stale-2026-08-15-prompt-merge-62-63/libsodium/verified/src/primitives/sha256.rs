//! Translated from crypto_hash/sha256/cp/hash_sha256_cp.c and hash_sha256.c
use crate::primitives::cutil::*;
use core::ffi::c_void;

#[repr(C)]
pub struct crypto_hash_sha256_state {
    pub state: [u32; 8],
    pub count: u64,
    pub buf: [u8; 64],
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

#[inline(always)]
fn ch(x: u32, y: u32, z: u32) -> u32 {
    (x & (y ^ z)) ^ z
}
#[inline(always)]
fn maj(x: u32, y: u32, z: u32) -> u32 {
    (x & (y | z)) | (y & z)
}
#[inline(always)]
fn big_s0(x: u32) -> u32 {
    rotr32(x, 2) ^ rotr32(x, 13) ^ rotr32(x, 22)
}
#[inline(always)]
fn big_s1(x: u32) -> u32 {
    rotr32(x, 6) ^ rotr32(x, 11) ^ rotr32(x, 25)
}
#[inline(always)]
fn sm0(x: u32) -> u32 {
    rotr32(x, 7) ^ rotr32(x, 18) ^ (x >> 3)
}
#[inline(always)]
fn sm1(x: u32) -> u32 {
    rotr32(x, 17) ^ rotr32(x, 19) ^ (x >> 10)
}

unsafe fn be32dec_vect(dst: &mut [u32], src: *const u8, len: usize) {
    for i in 0..(len / 4) {
        dst[i] = load32_be(src.add(i * 4));
    }
}

unsafe fn be32enc_vect(dst: *mut u8, src: &[u32], len: usize) {
    for i in 0..(len / 4) {
        store32_be(dst.add(i * 4), src[i]);
    }
}

fn sha256_transform(state: &mut [u32; 8], block: &[u8], w: &mut [u32; 64], s: &mut [u32; 8]) {
    unsafe {
        be32dec_vect(w, block.as_ptr(), 64);
    }
    s.copy_from_slice(state);

    macro_rules! rnd {
        ($a:expr,$b:expr,$c:expr,$d:expr,$e:expr,$f:expr,$g:expr,$h:expr,$k:expr) => {{
            s[$h] = s[$h]
                .wrapping_add(big_s1(s[$e]))
                .wrapping_add(ch(s[$e], s[$f], s[$g]))
                .wrapping_add($k);
            s[$d] = s[$d].wrapping_add(s[$h]);
            s[$h] = s[$h]
                .wrapping_add(big_s0(s[$a]))
                .wrapping_add(maj(s[$a], s[$b], s[$c]));
        }};
    }
    macro_rules! rndr {
        ($i:expr,$ii:expr) => {
            rnd!(
                (64 - $i) % 8,
                (65 - $i) % 8,
                (66 - $i) % 8,
                (67 - $i) % 8,
                (68 - $i) % 8,
                (69 - $i) % 8,
                (70 - $i) % 8,
                (71 - $i) % 8,
                w[$i + $ii].wrapping_add(KRND[$i + $ii])
            )
        };
    }
    macro_rules! msch {
        ($ii:expr,$i:expr) => {
            w[$i + $ii + 16] = sm1(w[$i + $ii + 14])
                .wrapping_add(w[$i + $ii + 9])
                .wrapping_add(sm0(w[$i + $ii + 1]))
                .wrapping_add(w[$i + $ii])
        };
    }

    let mut i = 0usize;
    loop {
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
    for j in 0..8 {
        state[j] = state[j].wrapping_add(s[j]);
    }
}

static PAD: [u8; 64] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0,
];

unsafe fn sha256_pad(state: &mut crypto_hash_sha256_state, tmp32: &mut [u32; 72]) {
    let r = ((state.count >> 3) & 0x3f) as usize;
    let (mut w, mut s): ([u32; 64], [u32; 8]);
    if r < 56 {
        for i in 0..(56 - r) {
            state.buf[r + i] = PAD[i];
        }
    } else {
        for i in 0..(64 - r) {
            state.buf[r + i] = PAD[i];
        }
        w = [0u32; 64];
        s = [0u32; 8];
        let buf = state.buf;
        sha256_transform(&mut state.state, &buf, &mut w, &mut s);
        copy_wide(tmp32, &w, &s);
        for i in 0..56 {
            state.buf[i] = 0;
        }
    }
    store64_be(state.buf.as_mut_ptr().add(56), state.count);
    w = [0u32; 64];
    s = [0u32; 8];
    let buf = state.buf;
    sha256_transform(&mut state.state, &buf, &mut w, &mut s);
    copy_wide(tmp32, &w, &s);
}

#[inline(always)]
fn copy_wide(tmp32: &mut [u32; 72], w: &[u32; 64], s: &[u32; 8]) {
    tmp32[..64].copy_from_slice(w);
    tmp32[64..].copy_from_slice(s);
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_hash_sha256_bytes() -> usize {
    32
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_hash_sha256_statebytes() -> usize {
    core::mem::size_of::<crypto_hash_sha256_state>()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_init(state: *mut crypto_hash_sha256_state) -> i32 {
    static INIT: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    (*state).count = 0;
    (*state).state = INIT;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_update(
    state: *mut crypto_hash_sha256_state,
    mut input: *const u8,
    mut inlen: u64,
) -> i32 {
    let mut tmp32 = [0u32; 72];
    let st = &mut *state;
    if inlen == 0 {
        return 0;
    }
    let r = ((st.count >> 3) & 0x3f) as u64;
    st.count = st.count.wrapping_add((inlen as u64) << 3);
    if inlen < 64 - r {
        for i in 0..inlen {
            st.buf[(r + i) as usize] = *input.add(i as usize);
        }
        return 0;
    }
    for i in 0..(64 - r) {
        st.buf[(r + i) as usize] = *input.add(i as usize);
    }
    {
        let mut w = [0u32; 64];
        let mut s = [0u32; 8];
        let buf = st.buf;
        sha256_transform(&mut st.state, &buf, &mut w, &mut s);
        copy_wide(&mut tmp32, &w, &s);
    }
    input = input.add((64 - r) as usize);
    inlen -= 64 - r;

    while inlen >= 64 {
        let block = core::slice::from_raw_parts(input, 64);
        let mut w = [0u32; 64];
        let mut s = [0u32; 8];
        sha256_transform(&mut st.state, block, &mut w, &mut s);
        copy_wide(&mut tmp32, &w, &s);
        input = input.add(64);
        inlen -= 64;
    }
    inlen &= 63;
    for i in 0..inlen {
        st.buf[i as usize] = *input.add(i as usize);
    }
    sodium_memzero(tmp32.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u32; 72]>());
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256_final(
    state: *mut crypto_hash_sha256_state,
    out: *mut u8,
) -> i32 {
    let mut tmp32 = [0u32; 72];
    let st = &mut *state;
    sha256_pad(st, &mut tmp32);
    be32enc_vect(out, &st.state, 32);
    sodium_memzero(tmp32.as_mut_ptr() as *mut c_void, core::mem::size_of::<[u32; 72]>());
    sodium_memzero(
        state as *mut c_void,
        core::mem::size_of::<crypto_hash_sha256_state>(),
    );
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_hash_sha256(
    out: *mut u8,
    input: *const u8,
    inlen: u64,
) -> i32 {
    let mut state = crypto_hash_sha256_state {
        state: [0; 8],
        count: 0,
        buf: [0; 64],
    };
    crypto_hash_sha256_init(&mut state);
    crypto_hash_sha256_update(&mut state, input, inlen);
    crypto_hash_sha256_final(&mut state, out);
    0
}
