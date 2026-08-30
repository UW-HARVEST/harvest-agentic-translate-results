//! Translation of `lib/blake/src/blake512.c`.
//!
//! BLAKE reference C implementation, Copyright (c) 2012 Jean-Philippe Aumasson
//! <jeanphilippe.aumasson@gmail.com>, dedicated to the public domain (CC0).
//! Taken from `supercop-20140525/crypto_hash/blake512/sandy`.

use core::ffi::c_ulong;

use crate::blake::blake256::{QUARTETS, SIGMA};
use crate::utils::u32_to_bytes;

pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

/// `blakestate512`
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

#[inline(always)]
fn u8to64(p: &[u8]) -> u64 {
    ((p[0] as u64) << 56)
        | ((p[1] as u64) << 48)
        | ((p[2] as u64) << 40)
        | ((p[3] as u64) << 32)
        | ((p[4] as u64) << 24)
        | ((p[5] as u64) << 16)
        | ((p[6] as u64) << 8)
        | (p[7] as u64)
}

#[inline(always)]
fn u64to8(p: &mut [u8], v: u64) {
    p[0] = (v >> 56) as u8;
    p[1] = (v >> 48) as u8;
    p[2] = (v >> 40) as u8;
    p[3] = (v >> 32) as u8;
    p[4] = (v >> 24) as u8;
    p[5] = (v >> 16) as u8;
    p[6] = (v >> 8) as u8;
    p[7] = v as u8;
}

/// `const u64 cst[16]` in the C file (a non-`static` global there).
#[rustfmt::skip]
static CST: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
];

/// `blake512.c` declares its `cst` table without `static`, so it ends up in the
/// shared library's symbol table; mirror that here.
#[allow(non_upper_case_globals)]
#[unsafe(no_mangle)]
pub static cst: [u64; 16] = CST;

static PADDING: [u8; 129] = {
    let mut p = [0u8; 129];
    p[0] = 0x80;
    p
};

#[inline(always)]
fn g(v: &mut [u64; 16], quartet: usize, m: &[u64; 16], i: usize, j: usize) {
    let [a, b, c, d] = QUARTETS[quartet];

    v[a] = v[a].wrapping_add(m[i] ^ CST[j]).wrapping_add(v[b]);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(25);

    v[a] = v[a].wrapping_add(m[j] ^ CST[i]).wrapping_add(v[b]);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(11);
}

pub fn blake512_compress(state: &mut BlakeState512, block: &[u8; 128]) {
    let mut m = [0u64; 16];
    for i in 0..16 {
        m[i] = u8to64(&block[8 * i..]);
    }

    let mut v = [0u64; 16];
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

    /* 16 rounds */
    for r in 0..16 {
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

pub fn blake512_init(state: &mut BlakeState512) {
    state.h[0] = 0x6A09E667F3BCC908;
    state.h[1] = 0xBB67AE8584CAA73B;
    state.h[2] = 0x3C6EF372FE94F82B;
    state.h[3] = 0xA54FF53A5F1D36F1;
    state.h[4] = 0x510E527FADE682D1;
    state.h[5] = 0x9B05688C2B3E6C1F;
    state.h[6] = 0x1F83D9ABFB41BD6B;
    state.h[7] = 0x5BE0CD19137E2179;
    state.t[0] = 0;
    state.t[1] = 0;
    state.buflen = 0;
    state.nullt = 0;
    state.s = [0; 4];
}

/// Note that `datalen` counts **bits**, as in the reference implementation.
pub fn blake512_update(state: &mut BlakeState512, data: &[u8], datalen: u64) {
    let mut datalen = datalen;
    let mut off = 0usize;
    let mut left = (state.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && (((datalen >> 3) & 0x7F) >= fill as u64) {
        state.buf[left..left + fill].copy_from_slice(&data[off..off + fill]);
        state.t[0] = state.t[0].wrapping_add(1024);
        let block = state.buf;
        blake512_compress(state, &block);
        off += fill;
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 1024 {
        state.t[0] = state.t[0].wrapping_add(1024);
        let block: [u8; 128] = data[off..off + 128].try_into().unwrap();
        blake512_compress(state, &block);
        off += 128;
        datalen -= 1024;
    }

    if datalen > 0 {
        let n = ((datalen >> 3) & 0x7F) as usize;
        state.buf[left..left + n].copy_from_slice(&data[off..off + n]);
        state.buflen = ((left as u64) << 3).wrapping_add(datalen) as i32;
    } else {
        state.buflen = 0;
    }
}

pub fn blake512_final(state: &mut BlakeState512, digest: &mut [u8]) {
    let mut msglen = [0u8; 16];
    let zo: [u8; 1] = [0x01];
    let oo: [u8; 1] = [0x81];

    let lo = state.t[0].wrapping_add(state.buflen as u64);
    let mut hi = state.t[1];
    if lo < state.buflen as u64 {
        hi = hi.wrapping_add(1);
    }
    u64to8(&mut msglen[0..], hi);
    u64to8(&mut msglen[8..], lo);

    if state.buflen == 888 {
        /* one padding byte */
        state.t[0] = state.t[0].wrapping_sub(8);
        blake512_update(state, &oo, 8);
    } else {
        if state.buflen < 888 {
            /* enough space to fill the block */
            if state.buflen == 0 {
                state.nullt = 1;
            }
            state.t[0] = state.t[0].wrapping_sub(888 - state.buflen as u64);
            blake512_update(state, &PADDING, (888 - state.buflen) as u64);
        } else {
            /* NOT enough space, need 2 compressions */
            state.t[0] = state.t[0].wrapping_sub(1024 - state.buflen as u64);
            blake512_update(state, &PADDING, (1024 - state.buflen) as u64);
            state.t[0] = state.t[0].wrapping_sub(888);
            blake512_update(state, &PADDING[1..], 888);
            state.nullt = 1;
        }
        blake512_update(state, &zo, 8);
        state.t[0] = state.t[0].wrapping_sub(8);
    }
    state.t[0] = state.t[0].wrapping_sub(128);
    blake512_update(state, &msglen, 128);

    for i in 0..8 {
        u64to8(&mut digest[8 * i..], state.h[i]);
    }
}

pub fn blake512_mgf1(out: &mut [u8], outlen: c_ulong, inp: &[u8], inlen: c_ulong) {
    let inlen = inlen as usize;
    let outlen = outlen as usize;
    let mut inbuf = [0u8; crate::blake::MGF1_INBUF_MAX];
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];

    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    /* While we can fit in at least another full block of BLAKE512 output.. */
    let mut i: usize = 0;
    let mut off: usize = 0;
    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(
            (&mut inbuf[inlen..inlen + 4]).try_into().unwrap(),
            i as u32,
        );
        let mut tmp = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
        blake512(&mut tmp, &inbuf[..inlen + 4], (inlen + 4) as u64);
        out[off..off + SPX_BLAKE512_OUTPUT_BYTES].copy_from_slice(&tmp);
        off += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    /* Until we cannot anymore, and we fill the remainder. */
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(
            (&mut inbuf[inlen..inlen + 4]).try_into().unwrap(),
            i as u32,
        );
        blake512(&mut outbuf, &inbuf[..inlen + 4], (inlen + 4) as u64);
        let rem = outlen - i * SPX_BLAKE512_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}

pub fn blake512(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState512::new();
    blake512_init(&mut s);
    blake512_update(&mut s, inp, inlen * 8);
    blake512_final(&mut s, out);
    0
}

// ---------------------------------------------------------------------------
// C ABI.  `blake.h` only renames `blake512_mgf1`; the rest keeps plain names.
// ---------------------------------------------------------------------------

pub mod abi {
    use super::*;

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn blake512_init(state: *mut BlakeState512) {
        unsafe { super::blake512_init(&mut *state) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn blake512_compress(state: *mut BlakeState512, block: *const u8) {
        unsafe { super::blake512_compress(&mut *state, &*(block as *const [u8; 128])) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn blake512_update(
        state: *mut BlakeState512,
        inp: *const u8,
        inlen: core::ffi::c_ulonglong,
    ) {
        unsafe {
            let bytes = ((inlen >> 3) as usize) + 1;
            super::blake512_update(
                &mut *state,
                core::slice::from_raw_parts(inp, bytes),
                inlen,
            )
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn blake512_final(state: *mut BlakeState512, out: *mut u8) {
        unsafe { super::blake512_final(&mut *state, core::slice::from_raw_parts_mut(out, 64)) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn blake512(
        out: *mut u8,
        inp: *const u8,
        inlen: core::ffi::c_ulonglong,
    ) -> core::ffi::c_int {
        unsafe {
            super::blake512(
                core::slice::from_raw_parts_mut(out, 64),
                core::slice::from_raw_parts(inp, inlen as usize),
                inlen,
            )
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_blake512_mgf1(
    out: *mut u8,
    outlen: c_ulong,
    inp: *const u8,
    inlen: c_ulong,
) {
    unsafe {
        blake512_mgf1(
            core::slice::from_raw_parts_mut(out, outlen as usize),
            outlen,
            core::slice::from_raw_parts(inp, inlen as usize),
            inlen,
        )
    }
}
