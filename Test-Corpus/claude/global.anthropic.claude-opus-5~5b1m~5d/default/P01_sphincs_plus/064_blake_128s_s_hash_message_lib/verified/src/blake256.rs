//! Translation of `lib/blake/src/blake256.c` (BLAKE-256).

use crate::params::SPX_N;
use crate::utils::u32_to_bytes;

pub const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;

#[derive(Clone)]
#[repr(C)]
pub struct BlakeState256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

impl BlakeState256 {
    pub fn new() -> Self {
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

const C256: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344, 0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C, 0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

const SIGMA: [[usize; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}

#[allow(clippy::too_many_arguments)]
fn g(
    v: &mut [u32; 16],
    m: &[u32; 16],
    sr: &[usize; 16],
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
) {
    v[a] = v[a]
        .wrapping_add(m[sr[e]] ^ C256[sr[e + 1]])
        .wrapping_add(v[b]);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(12);
    v[a] = v[a]
        .wrapping_add(m[sr[e + 1]] ^ C256[sr[e]])
        .wrapping_add(v[b]);
    v[d] = (v[d] ^ v[a]).rotate_right(8);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(7);
}

pub fn blake256_compress(state: &mut BlakeState256, block: &[u8]) {
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = u8to32(&block[4 * i..]);
    }
    let mut v = [0u32; 16];
    v[..8].copy_from_slice(&state.h);
    v[8] = state.s[0] ^ C256[0];
    v[9] = state.s[1] ^ C256[1];
    v[10] = state.s[2] ^ C256[2];
    v[11] = state.s[3] ^ C256[3];
    v[12] = C256[4];
    v[13] = C256[5];
    v[14] = C256[6];
    v[15] = C256[7];
    if state.nullt == 0 {
        v[12] ^= state.t[0];
        v[13] ^= state.t[0];
        v[14] ^= state.t[1];
        v[15] ^= state.t[1];
    }

    for r in 0..14 {
        let sr = &SIGMA[r % 10];
        g(&mut v, &m, sr, 0, 4, 8, 12, 0);
        g(&mut v, &m, sr, 1, 5, 9, 13, 2);
        g(&mut v, &m, sr, 2, 6, 10, 14, 4);
        g(&mut v, &m, sr, 3, 7, 11, 15, 6);
        g(&mut v, &m, sr, 0, 5, 10, 15, 8);
        g(&mut v, &m, sr, 1, 6, 11, 12, 10);
        g(&mut v, &m, sr, 2, 7, 8, 13, 12);
        g(&mut v, &m, sr, 3, 4, 9, 14, 14);
    }

    for i in 0..8 {
        state.h[i] ^= v[i] ^ v[i + 8] ^ state.s[i & 3];
    }
}

pub fn blake256_init(state: &mut BlakeState256) {
    state.h = [
        0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB,
        0x5BE0CD19,
    ];
    state.t = [0, 0];
    state.buflen = 0;
    state.nullt = 0;
    state.s = [0, 0, 0, 0];
}

pub fn blake256_update(state: &mut BlakeState256, mut data: &[u8], mut datalen: u64) {
    let mut left = (state.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && (((datalen >> 3) & 0x3F) as usize) >= fill {
        state.buf[left..left + fill].copy_from_slice(&data[..fill]);
        state.t[0] = state.t[0].wrapping_add(512);
        if state.t[0] == 0 {
            state.t[1] = state.t[1].wrapping_add(1);
        }
        let block = state.buf;
        blake256_compress(state, &block);
        data = &data[fill..];
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 512 {
        state.t[0] = state.t[0].wrapping_add(512);
        if state.t[0] == 0 {
            state.t[1] = state.t[1].wrapping_add(1);
        }
        blake256_compress(state, &data[..64]);
        data = &data[64..];
        datalen -= 512;
    }

    if datalen > 0 {
        let nbytes = (datalen >> 3) as usize;
        state.buf[left..left + nbytes].copy_from_slice(&data[..nbytes]);
        state.buflen = ((left as u64) << 3) as i32 + datalen as i32;
    } else {
        state.buflen = 0;
    }
}

pub fn blake256_final(state: &mut BlakeState256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = state.t[0].wrapping_add(state.buflen as u32);
    let mut hi = state.t[1];
    if lo < state.buflen as u32 {
        hi = hi.wrapping_add(1);
    }
    u32_to_bytes(&mut msglen[0..], hi);
    u32_to_bytes(&mut msglen[4..], lo);

    if state.buflen == 440 {
        state.t[0] = state.t[0].wrapping_sub(8);
        blake256_update(state, &[oo], 8);
    } else {
        if state.buflen < 440 {
            if state.buflen == 0 {
                state.nullt = 1;
            }
            state.t[0] = state.t[0].wrapping_sub((440 - state.buflen) as u32);
            let pad = &PADDING[..((440 - state.buflen) as usize) >> 3];
            blake256_update(state, pad, (440 - state.buflen) as u64);
        } else {
            state.t[0] = state.t[0].wrapping_sub((512 - state.buflen) as u32);
            let pad = &PADDING[..((512 - state.buflen) as usize) >> 3];
            blake256_update(state, pad, (512 - state.buflen) as u64);
            state.t[0] = state.t[0].wrapping_sub(440);
            blake256_update(state, &PADDING[1..1 + (440 >> 3)], 440);
            state.nullt = 1;
        }
        blake256_update(state, &[zo], 8);
        state.t[0] = state.t[0].wrapping_sub(8);
    }
    state.t[0] = state.t[0].wrapping_sub(64);
    blake256_update(state, &msglen, 64);

    u32_to_bytes(&mut digest[0..], state.h[0]);
    u32_to_bytes(&mut digest[4..], state.h[1]);
    u32_to_bytes(&mut digest[8..], state.h[2]);
    u32_to_bytes(&mut digest[12..], state.h[3]);
    u32_to_bytes(&mut digest[16..], state.h[4]);
    u32_to_bytes(&mut digest[20..], state.h[5]);
    u32_to_bytes(&mut digest[24..], state.h[6]);
    u32_to_bytes(&mut digest[28..], state.h[7]);
}

const PADDING: [u8; 64] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0,
];

pub fn blake256(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState256::new();
    blake256_init(&mut s);
    blake256_update(&mut s, inp, inlen * 8);
    blake256_final(&mut s, out);
    0
}

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i: usize = 0;
    let mut o = 0usize;
    while (i + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256(&mut out[o..o + SPX_BLAKE256_OUTPUT_BYTES], &inbuf, (inlen + 4) as u64);
        o += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        out[o..outlen].copy_from_slice(&outbuf[..outlen - i * SPX_BLAKE256_OUTPUT_BYTES]);
    }
}

const _: () = assert!(SPX_BLAKE256_OUTPUT_BYTES >= SPX_N);

// ------------------------------------------------------------------
// Exported C ABI wrappers.
// ------------------------------------------------------------------

#[export_name = "blake256"]
pub unsafe extern "C" fn c_blake256(out: *mut u8, inp: *const u8, inlen: core::ffi::c_ulonglong) -> core::ffi::c_int {
    let o = core::slice::from_raw_parts_mut(out, 32);
    let i = core::slice::from_raw_parts(inp, inlen as usize);
    blake256(o, i, inlen)
}
#[export_name = "blake256_init"]
pub unsafe extern "C" fn c_blake256_init(s: *mut BlakeState256) {
    blake256_init(&mut *s);
}
#[export_name = "blake256_compress"]
pub unsafe extern "C" fn c_blake256_compress(s: *mut BlakeState256, block: *const u8) {
    blake256_compress(&mut *s, core::slice::from_raw_parts(block, 64));
}
#[export_name = "blake256_update"]
pub unsafe extern "C" fn c_blake256_update(s: *mut BlakeState256, inp: *const u8, inlen: core::ffi::c_ulonglong) {
    // `inlen` is a bit length (matching the C callers); the routine reads
    // exactly `inlen / 8` bytes from `inp`.
    let nbytes = (inlen >> 3) as usize;
    let data = core::slice::from_raw_parts(inp, nbytes);
    blake256_update(&mut *s, data, inlen);
}
#[export_name = "blake256_final"]
pub unsafe extern "C" fn c_blake256_final(s: *mut BlakeState256, out: *mut u8) {
    blake256_final(&mut *s, core::slice::from_raw_parts_mut(out, 32));
}
#[no_mangle]
pub unsafe extern "C" fn SPX_blake256_mgf1(out: *mut u8, outlen: core::ffi::c_ulong, inp: *const u8, inlen: core::ffi::c_ulong) {
    let o = core::slice::from_raw_parts_mut(out, outlen as usize);
    let i = core::slice::from_raw_parts(inp, inlen as usize);
    blake256_mgf1(o, outlen as usize, i, inlen as usize);
}



