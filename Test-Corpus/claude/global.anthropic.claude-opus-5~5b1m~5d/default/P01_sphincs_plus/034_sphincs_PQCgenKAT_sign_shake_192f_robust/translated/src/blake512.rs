//! Translation of `lib/blake/src/blake512.c` (BLAKE-512).

use crate::utils::u32_to_bytes;

pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

#[derive(Clone)]
pub struct BlakeState512 {
    pub h: [u64; 8],
    pub s: [u64; 4],
    pub t: [u64; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 128],
}

impl BlakeState512 {
    pub fn new() -> Self {
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

const C512: [u64; 16] = [
    0x243F6A8885A308D3, 0x13198A2E03707344, 0xA4093822299F31D0, 0x082EFA98EC4E6C89,
    0x452821E638D01377, 0xBE5466CF34E90C6C, 0xC0AC29B7C97C50DD, 0x3F84D5B5B5470917,
    0x9216D5D98979FB1B, 0xD1310BA698DFB5AC, 0x2FFD72DBD01ADFB7, 0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99, 0x24A19947B3916CF7, 0x0801F2E2858EFC16, 0x636920D871574E69,
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

fn u8to64(p: &[u8]) -> u64 {
    let hi = ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32);
    let lo = ((p[4] as u32) << 24) | ((p[5] as u32) << 16) | ((p[6] as u32) << 8) | (p[7] as u32);
    ((hi as u64) << 32) | (lo as u64)
}

fn u64to8(p: &mut [u8], v: u64) {
    u32_to_bytes(&mut p[0..], (v >> 32) as u32);
    u32_to_bytes(&mut p[4..], v as u32);
}

#[allow(clippy::too_many_arguments)]
fn g(
    v: &mut [u64; 16],
    m: &[u64; 16],
    sr: &[usize; 16],
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
) {
    v[a] = v[a]
        .wrapping_add(m[sr[e]] ^ C512[sr[e + 1]])
        .wrapping_add(v[b]);
    v[d] = (v[d] ^ v[a]).rotate_right(32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(25);
    v[a] = v[a]
        .wrapping_add(m[sr[e + 1]] ^ C512[sr[e]])
        .wrapping_add(v[b]);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(11);
}

pub fn blake512_compress(state: &mut BlakeState512, block: &[u8]) {
    let mut m = [0u64; 16];
    for i in 0..16 {
        m[i] = u8to64(&block[8 * i..]);
    }
    let mut v = [0u64; 16];
    v[..8].copy_from_slice(&state.h);
    v[8] = state.s[0] ^ C512[0];
    v[9] = state.s[1] ^ C512[1];
    v[10] = state.s[2] ^ C512[2];
    v[11] = state.s[3] ^ C512[3];
    v[12] = C512[4];
    v[13] = C512[5];
    v[14] = C512[6];
    v[15] = C512[7];
    if state.nullt == 0 {
        v[12] ^= state.t[0];
        v[13] ^= state.t[0];
        v[14] ^= state.t[1];
        v[15] ^= state.t[1];
    }

    for r in 0..16 {
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

pub fn blake512_init(state: &mut BlakeState512) {
    state.h = [
        0x6A09E667F3BCC908,
        0xBB67AE8584CAA73B,
        0x3C6EF372FE94F82B,
        0xA54FF53A5F1D36F1,
        0x510E527FADE682D1,
        0x9B05688C2B3E6C1F,
        0x1F83D9ABFB41BD6B,
        0x5BE0CD19137E2179,
    ];
    state.t = [0, 0];
    state.buflen = 0;
    state.nullt = 0;
    state.s = [0, 0, 0, 0];
}

pub fn blake512_update(state: &mut BlakeState512, mut data: &[u8], mut datalen: u64) {
    let mut left = (state.buflen >> 3) as usize;
    let fill = 128 - left;

    if left != 0 && (((datalen >> 3) & 0x7F) as usize) >= fill {
        state.buf[left..left + fill].copy_from_slice(&data[..fill]);
        state.t[0] = state.t[0].wrapping_add(1024);
        let block = state.buf;
        blake512_compress(state, &block);
        data = &data[fill..];
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 1024 {
        state.t[0] = state.t[0].wrapping_add(1024);
        blake512_compress(state, &data[..128]);
        data = &data[128..];
        datalen -= 1024;
    }

    if datalen > 0 {
        let nbytes = ((datalen >> 3) & 0x7F) as usize;
        state.buf[left..left + nbytes].copy_from_slice(&data[..nbytes]);
        state.buflen = (((left as u64) << 3) as i32) + datalen as i32;
    } else {
        state.buflen = 0;
    }
}

pub fn blake512_final(state: &mut BlakeState512, digest: &mut [u8]) {
    let mut msglen = [0u8; 16];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = state.t[0].wrapping_add(state.buflen as u64);
    let mut hi = state.t[1];
    if lo < state.buflen as u64 {
        hi = hi.wrapping_add(1);
    }
    u64to8(&mut msglen[0..], hi);
    u64to8(&mut msglen[8..], lo);

    if state.buflen == 888 {
        state.t[0] = state.t[0].wrapping_sub(8);
        blake512_update(state, &[oo], 8);
    } else {
        if state.buflen < 888 {
            if state.buflen == 0 {
                state.nullt = 1;
            }
            state.t[0] = state.t[0].wrapping_sub((888 - state.buflen) as u64);
            let pad = &PADDING[..((888 - state.buflen) as usize) >> 3];
            blake512_update(state, pad, (888 - state.buflen) as u64);
        } else {
            state.t[0] = state.t[0].wrapping_sub((1024 - state.buflen) as u64);
            let pad = &PADDING[..((1024 - state.buflen) as usize) >> 3];
            blake512_update(state, pad, (1024 - state.buflen) as u64);
            state.t[0] = state.t[0].wrapping_sub(888);
            blake512_update(state, &PADDING[1..1 + (888 >> 3)], 888);
            state.nullt = 1;
        }
        blake512_update(state, &[zo], 8);
        state.t[0] = state.t[0].wrapping_sub(8);
    }
    state.t[0] = state.t[0].wrapping_sub(128);
    blake512_update(state, &msglen, 128);

    u64to8(&mut digest[0..], state.h[0]);
    u64to8(&mut digest[8..], state.h[1]);
    u64to8(&mut digest[16..], state.h[2]);
    u64to8(&mut digest[24..], state.h[3]);
    u64to8(&mut digest[32..], state.h[4]);
    u64to8(&mut digest[40..], state.h[5]);
    u64to8(&mut digest[48..], state.h[6]);
    u64to8(&mut digest[56..], state.h[7]);
}

const fn make_padding() -> [u8; 129] {
    let mut a = [0u8; 129];
    a[0] = 0x80;
    a
}
const PADDING: [u8; 129] = make_padding();

pub fn blake512(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState512::new();
    blake512_init(&mut s);
    blake512_update(&mut s, inp, inlen * 8);
    blake512_final(&mut s, out);
    0
}

pub fn blake512_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i: usize = 0;
    let mut o = 0usize;
    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512(&mut out[o..o + SPX_BLAKE512_OUTPUT_BYTES], &inbuf, (inlen + 4) as u64);
        o += SPX_BLAKE512_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512(&mut outbuf, &inbuf, (inlen + 4) as u64);
        out[o..outlen].copy_from_slice(&outbuf[..outlen - i * SPX_BLAKE512_OUTPUT_BYTES]);
    }
}

// ------------------------------------------------------------------
// Exported C ABI wrappers.
// ------------------------------------------------------------------

#[export_name = "blake512"]
pub unsafe extern "C" fn c_blake512(out: *mut u8, inp: *const u8, inlen: core::ffi::c_ulonglong) -> core::ffi::c_int {
    let o = core::slice::from_raw_parts_mut(out, 64);
    let i = core::slice::from_raw_parts(inp, inlen as usize);
    blake512(o, i, inlen)
}
#[export_name = "blake512_init"]
pub unsafe extern "C" fn c_blake512_init(s: *mut BlakeState512) {
    blake512_init(&mut *s);
}
#[export_name = "blake512_compress"]
pub unsafe extern "C" fn c_blake512_compress(s: *mut BlakeState512, block: *const u8) {
    blake512_compress(&mut *s, core::slice::from_raw_parts(block, 128));
}
#[export_name = "blake512_update"]
pub unsafe extern "C" fn c_blake512_update(s: *mut BlakeState512, inp: *const u8, inlen: core::ffi::c_ulonglong) {
    let nbytes = (inlen >> 3) as usize;
    let data = core::slice::from_raw_parts(inp, nbytes);
    blake512_update(&mut *s, data, inlen);
}
#[export_name = "blake512_final"]
pub unsafe extern "C" fn c_blake512_final(s: *mut BlakeState512, out: *mut u8) {
    blake512_final(&mut *s, core::slice::from_raw_parts_mut(out, 64));
}
#[no_mangle]
pub unsafe extern "C" fn SPX_blake512_mgf1(out: *mut u8, outlen: core::ffi::c_ulong, inp: *const u8, inlen: core::ffi::c_ulong) {
    let o = core::slice::from_raw_parts_mut(out, outlen as usize);
    let i = core::slice::from_raw_parts(inp, inlen as usize);
    blake512_mgf1(o, outlen as usize, i, inlen as usize);
}


