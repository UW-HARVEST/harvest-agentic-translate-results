//! Translation of `lib/blake/src/blake256.c`.
//!
//! BLAKE reference C implementation, Copyright (c) 2012 Jean-Philippe Aumasson
//! <jeanphilippe.aumasson@gmail.com>, dedicated to the public domain (CC0).
//! Taken from `supercop-20140525/crypto_hash/blake256/sandy`.

use core::ffi::c_ulong;

use crate::params::SPX_N;
use crate::utils::u32_to_bytes;

/// This does not necessarily equal `SPX_N`.
pub const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;

const _: () = assert!(
    SPX_BLAKE256_OUTPUT_BYTES >= SPX_N,
    "Linking against BLAKE-256 with N larger than 32 bytes is not supported"
);

/// `blakestate256`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BlakeState256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

impl BlakeState256 {
    pub const fn new() -> Self {
        BlakeState256 {
            h: [0; 8],
            s: [0; 4],
            t: [0; 2],
            buflen: 0,
            nullt: 0,
            buf: [0; 64],
        }
    }
}

impl Default for BlakeState256 {
    fn default() -> Self {
        Self::new()
    }
}

#[inline(always)]
fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}

#[inline(always)]
fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

#[rustfmt::skip]
static CST: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

static PADDING: [u8; 64] = {
    let mut p = [0u8; 64];
    p[0] = 0x80;
    p
};

/// The message word permutation implied by the `ROUND(...)` argument lists in
/// the C source.
#[rustfmt::skip]
pub(crate) static SIGMA: [[usize; 16]; 10] = [
    [ 0,  1,  2,  3,  4,  5,  6,  7,  8,  9, 10, 11, 12, 13, 14, 15],
    [14, 10,  4,  8,  9, 15, 13,  6,  1, 12,  0,  2, 11,  7,  5,  3],
    [11,  8, 12,  0,  5,  2, 15, 13, 10, 14,  3,  6,  7,  1,  9,  4],
    [ 7,  9,  3,  1, 13, 12, 11, 14,  2,  6,  5, 10,  4,  0, 15,  8],
    [ 9,  0,  5,  7,  2,  4, 10, 15, 14,  1, 11, 12,  6,  8,  3, 13],
    [ 2, 12,  6, 10,  0, 11,  8,  3,  4, 13,  7,  5, 15, 14,  1,  9],
    [12,  5,  1, 15, 14, 13,  4, 10,  0,  7,  6,  3,  9,  2,  8, 11],
    [13, 11,  7, 14, 12,  1,  3,  9,  5,  0, 15,  4,  8,  6,  2, 10],
    [ 6, 15, 14,  9, 11,  3,  0,  8, 12,  2, 13,  7,  1,  4, 10,  5],
    [10,  2,  8,  4,  7,  6,  1,  5, 15, 11,  9, 14,  3, 12, 13,  0],
];

/// The four `(a, b, c, d)` column/diagonal quartets of a BLAKE round, in the
/// order the C `ROUND` macro pairs them with `SIGMA`.
#[rustfmt::skip]
pub(crate) static QUARTETS: [[usize; 4]; 8] = [
    [0, 4,  8, 12],
    [1, 5,  9, 13],
    [2, 6, 10, 14],
    [3, 7, 11, 15],
    [0, 5, 10, 15],
    [1, 6, 11, 12],
    [2, 7,  8, 13],
    [3, 4,  9, 14],
];

#[inline(always)]
fn g(v: &mut [u32; 16], quartet: usize, m: &[u32; 16], i: usize, j: usize) {
    let [a, b, c, d] = QUARTETS[quartet];

    v[a] = v[a].wrapping_add(m[i] ^ CST[j]).wrapping_add(v[b]);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(12);

    v[a] = v[a].wrapping_add(m[j] ^ CST[i]).wrapping_add(v[b]);
    v[d] = (v[d] ^ v[a]).rotate_right(8);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(7);
}

pub fn blake256_compress(state: &mut BlakeState256, block: &[u8; 64]) {
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = u8to32(&block[4 * i..]);
    }

    let mut v = [0u32; 16];
    v[..8].copy_from_slice(&state.h);
    v[8] = state.s[0] ^ CST[0];
    v[9] = state.s[1] ^ CST[1];
    v[10] = state.s[2] ^ CST[2];
    v[11] = state.s[3] ^ CST[3];
    v[12] = CST[4];
    v[13] = CST[5];
    v[14] = CST[6];
    v[15] = CST[7];

    if state.nullt == 0 {
        v[12] ^= state.t[0];
        v[13] ^= state.t[0];
        v[14] ^= state.t[1];
        v[15] ^= state.t[1];
    }

    /* 14 rounds */
    for r in 0..14 {
        let sigma = &SIGMA[r % 10];
        for k in 0..8 {
            g(&mut v, k, &m, sigma[2 * k], sigma[2 * k + 1]);
        }
    }

    for i in 0..8 {
        v[i] ^= v[i + 8];
    }
    for i in 0..8 {
        v[i] ^= state.s[i % 4];
    }
    for i in 0..8 {
        state.h[i] ^= v[i];
    }
}

pub fn blake256_init(state: &mut BlakeState256) {
    state.h[0] = 0x6A09E667;
    state.h[1] = 0xBB67AE85;
    state.h[2] = 0x3C6EF372;
    state.h[3] = 0xA54FF53A;
    state.h[4] = 0x510E527F;
    state.h[5] = 0x9B05688C;
    state.h[6] = 0x1F83D9AB;
    state.h[7] = 0x5BE0CD19;
    state.t[0] = 0;
    state.t[1] = 0;
    state.buflen = 0;
    state.nullt = 0;
    state.s = [0; 4];
}

/// Note that `datalen` counts **bits**, as in the reference implementation.
pub fn blake256_update(state: &mut BlakeState256, data: &[u8], datalen: u64) {
    let mut datalen = datalen;
    let mut off = 0usize;
    let mut left = (state.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && (((datalen >> 3) & 0x3F) >= fill as u64) {
        state.buf[left..left + fill].copy_from_slice(&data[off..off + fill]);
        state.t[0] = state.t[0].wrapping_add(512);
        if state.t[0] == 0 {
            state.t[1] = state.t[1].wrapping_add(1);
        }
        let block = state.buf;
        blake256_compress(state, &block);
        off += fill;
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 512 {
        state.t[0] = state.t[0].wrapping_add(512);
        if state.t[0] == 0 {
            state.t[1] = state.t[1].wrapping_add(1);
        }
        let block: [u8; 64] = data[off..off + 64].try_into().unwrap();
        blake256_compress(state, &block);
        off += 64;
        datalen -= 512;
    }

    if datalen > 0 {
        let n = (datalen >> 3) as usize;
        state.buf[left..left + n].copy_from_slice(&data[off..off + n]);
        state.buflen = ((left as u64) << 3).wrapping_add(datalen) as i32;
    } else {
        state.buflen = 0;
    }
}

pub fn blake256_final(state: &mut BlakeState256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let zo: [u8; 1] = [0x01];
    let oo: [u8; 1] = [0x81];

    let lo = state.t[0].wrapping_add(state.buflen as u32);
    let mut hi = state.t[1];
    if lo < state.buflen as u32 {
        hi = hi.wrapping_add(1);
    }
    u32to8(&mut msglen[0..], hi);
    u32to8(&mut msglen[4..], lo);

    if state.buflen == 440 {
        /* one padding byte */
        state.t[0] = state.t[0].wrapping_sub(8);
        blake256_update(state, &oo, 8);
    } else {
        if state.buflen < 440 {
            /* enough space to fill the block */
            if state.buflen == 0 {
                state.nullt = 1;
            }
            state.t[0] = state.t[0].wrapping_sub(440 - state.buflen as u32);
            blake256_update(state, &PADDING, (440 - state.buflen) as u64);
        } else {
            /* need 2 compressions */
            state.t[0] = state.t[0].wrapping_sub(512 - state.buflen as u32);
            blake256_update(state, &PADDING, (512 - state.buflen) as u64);
            state.t[0] = state.t[0].wrapping_sub(440);
            blake256_update(state, &PADDING[1..], 440);
            state.nullt = 1;
        }
        blake256_update(state, &zo, 8);
        state.t[0] = state.t[0].wrapping_sub(8);
    }
    state.t[0] = state.t[0].wrapping_sub(64);
    blake256_update(state, &msglen, 64);

    for i in 0..8 {
        u32to8(&mut digest[4 * i..], state.h[i]);
    }
}

pub fn blake256_mgf1(out: &mut [u8], outlen: c_ulong, inp: &[u8], inlen: c_ulong) {
    let inlen = inlen as usize;
    let outlen = outlen as usize;
    let mut inbuf = [0u8; crate::blake::MGF1_INBUF_MAX];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    /* While we can fit in at least another full block of BLAKE256 output.. */
    let mut i: usize = 0;
    let mut off: usize = 0;
    while (i + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(
            (&mut inbuf[inlen..inlen + 4]).try_into().unwrap(),
            i as u32,
        );
        let mut tmp = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
        blake256(&mut tmp, &inbuf[..inlen + 4], (inlen + 4) as u64);
        out[off..off + SPX_BLAKE256_OUTPUT_BYTES].copy_from_slice(&tmp);
        off += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    /* Until we cannot anymore, and we fill the remainder. */
    if outlen > i * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes(
            (&mut inbuf[inlen..inlen + 4]).try_into().unwrap(),
            i as u32,
        );
        blake256(&mut outbuf, &inbuf[..inlen + 4], (inlen + 4) as u64);
        let rem = outlen - i * SPX_BLAKE256_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

pub fn blake256(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState256::new();
    blake256_init(&mut s);
    blake256_update(&mut s, inp, inlen * 8);
    blake256_final(&mut s, out);
    0
}

// ---------------------------------------------------------------------------
// C ABI.  `blake.h` only renames `blake256_mgf1`; the rest keeps plain names,
// so those wrappers live in a submodule to avoid clashing with the safe
// functions above.
// ---------------------------------------------------------------------------

pub mod abi {
    use super::*;

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn blake256_init(state: *mut BlakeState256) {
        unsafe { super::blake256_init(&mut *state) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn blake256_compress(state: *mut BlakeState256, block: *const u8) {
        unsafe { super::blake256_compress(&mut *state, &*(block as *const [u8; 64])) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn blake256_update(
        state: *mut BlakeState256,
        inp: *const u8,
        inlen: core::ffi::c_ulonglong,
    ) {
        unsafe {
            // `inlen` counts bits; the byte length is `inlen / 8` rounded up in
            // the same way the reference implementation reads it.
            let bytes = ((inlen >> 3) as usize) + 1;
            super::blake256_update(
                &mut *state,
                core::slice::from_raw_parts(inp, bytes),
                inlen,
            )
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn blake256_final(state: *mut BlakeState256, out: *mut u8) {
        unsafe { super::blake256_final(&mut *state, core::slice::from_raw_parts_mut(out, 32)) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn blake256(
        out: *mut u8,
        inp: *const u8,
        inlen: core::ffi::c_ulonglong,
    ) -> core::ffi::c_int {
        unsafe {
            super::blake256(
                core::slice::from_raw_parts_mut(out, 32),
                core::slice::from_raw_parts(inp, inlen as usize),
                inlen,
            )
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_blake256_mgf1(
    out: *mut u8,
    outlen: c_ulong,
    inp: *const u8,
    inlen: c_ulong,
) {
    unsafe {
        blake256_mgf1(
            core::slice::from_raw_parts_mut(out, outlen as usize),
            outlen,
            core::slice::from_raw_parts(inp, inlen as usize),
            inlen,
        )
    }
}
