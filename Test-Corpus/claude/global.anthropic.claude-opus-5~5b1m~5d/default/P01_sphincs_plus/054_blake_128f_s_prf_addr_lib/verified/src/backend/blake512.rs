//! Translation of `lib/blake/src/blake512.c`.
//!
//! BLAKE reference C implementation
//! Copyright (c) 2012 Jean-Philippe Aumasson <jeanphilippe.aumasson@gmail.com>
//! (CC0 / public domain)

use crate::utils::u32_to_bytes;

/// `SPX_BLAKE512_OUTPUT_BYTES` from `lib/blake/include/blake.h`.
pub const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

/// C `blakestate512`:
/// ```c
/// typedef struct {
///   unsigned long long h[8], s[4], t[2];
///   int buflen, nullt;
///   unsigned char buf[128];
/// } blakestate512;
/// ```
#[repr(C)]
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
    #[inline]
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

/// `const u64 cst[16]` -- NOT `static` in `lib/blake/src/blake512.c`, so the C
/// `libblake.so` exports it as a read-only data symbol named `cst`.  We match
/// that exactly (`#[no_mangle]` + the bare `cst` name) so an external consumer
/// sees the same 128-byte table at the same symbol.
#[unsafe(no_mangle)]
pub static cst: [u64; 16] = CST;

static CST: [u64; 16] = [
    0x243F6A8885A308D3,
    0x13198A2E03707344,
    0xA4093822299F31D0,
    0x082EFA98EC4E6C89,
    0x452821E638D01377,
    0xBE5466CF34E90C6C,
    0xC0AC29B7C97C50DD,
    0x3F84D5B5B5470917,
    0x9216D5D98979FB1B,
    0xD1310BA698DFB5AC,
    0x2FFD72DBD01ADFB7,
    0xB8E1AFED6A267E96,
    0xBA7C9045F12C7F99,
    0x24A19947B3916CF7,
    0x0801F2E2858EFC16,
    0x636920D871574E69,
];

static PADDING: [u8; 129] = {
    let mut p = [0u8; 129];
    p[0] = 0x80;
    p
};

/// Message-word / constant permutation, see `blake256.rs`.
static SIGMA: [[usize; 16]; 10] = [
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

/// `#define U8TO64(p)` — big endian load.
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

/// `#define U64TO8(p, v)` — big endian store.
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

/// `#define BLAKE512_ROT(x,n) (((x)<<(64-n))|((x)>>(n)))`
#[inline(always)]
fn rot(x: u64, n: u32) -> u64 {
    x.rotate_right(n)
}

#[inline(always)]
fn g(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, mc: u64, r1: u32, r2: u32) {
    v[a] = v[a].wrapping_add(mc);
    v[a] = v[a].wrapping_add(v[b]);
    v[d] ^= v[a];
    v[d] = rot(v[d], r1);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] ^= v[c];
    v[b] = rot(v[b], r2);
}

/// The `ROUND(...)` macro, in the exact statement order of the C source.
#[inline(always)]
fn round(v: &mut [u64; 16], m: &[u64; 16], s: &[usize; 16]) {
    macro_rules! mc {
        ($k:expr) => {
            m[s[$k]] ^ CST[s[$k ^ 1]]
        };
    }

    g(v, 0, 4, 8, 12, mc!(0), 32, 25);
    g(v, 1, 5, 9, 13, mc!(2), 32, 25);
    g(v, 2, 6, 10, 14, mc!(4), 32, 25);
    g(v, 3, 7, 11, 15, mc!(6), 32, 25);
    g(v, 2, 6, 10, 14, mc!(5), 16, 11);
    g(v, 3, 7, 11, 15, mc!(7), 16, 11);
    g(v, 1, 5, 9, 13, mc!(3), 16, 11);
    g(v, 0, 4, 8, 12, mc!(1), 16, 11);
    g(v, 0, 5, 10, 15, mc!(8), 32, 25);
    g(v, 1, 6, 11, 12, mc!(10), 32, 25);
    g(v, 2, 7, 8, 13, mc!(12), 32, 25);
    g(v, 3, 4, 9, 14, mc!(14), 32, 25);
    g(v, 2, 7, 8, 13, mc!(13), 16, 11);
    g(v, 3, 4, 9, 14, mc!(15), 16, 11);
    g(v, 1, 6, 11, 12, mc!(11), 16, 11);
    g(v, 0, 5, 10, 15, mc!(9), 16, 11);
}

pub fn blake512_compress(state: &mut BlakeState512, block: &[u8]) {
    let mut m = [0u64; 16];
    for (i, w) in m.iter_mut().enumerate() {
        *w = u8to64(&block[8 * i..]);
    }

    let mut v = [0u64; 16];
    v[0] = state.h[0];
    v[1] = state.h[1];
    v[2] = state.h[2];
    v[3] = state.h[3];
    v[4] = state.h[4];
    v[5] = state.h[5];
    v[6] = state.h[6];
    v[7] = state.h[7];
    v[8] = state.s[0] ^ 0x243F6A8885A308D3;
    v[9] = state.s[1] ^ 0x13198A2E03707344;
    v[10] = state.s[2] ^ 0xA4093822299F31D0;
    v[11] = state.s[3] ^ 0x082EFA98EC4E6C89;
    v[12] = 0x452821E638D01377;
    v[13] = 0xBE5466CF34E90C6C;
    v[14] = 0xC0AC29B7C97C50DD;
    v[15] = 0x3F84D5B5B5470917;

    if state.nullt == 0 {
        v[12] ^= state.t[0];
        v[13] ^= state.t[0];
        v[14] ^= state.t[1];
        v[15] ^= state.t[1];
    }

    /* 16 rounds: sigma rows 0..9 followed by rows 0..5 again. */
    for r in 0..10 {
        round(&mut v, &m, &SIGMA[r]);
    }
    for r in 0..6 {
        round(&mut v, &m, &SIGMA[r]);
    }

    v[0] ^= v[8];
    v[1] ^= v[9];
    v[2] ^= v[10];
    v[3] ^= v[11];
    v[4] ^= v[12];
    v[5] ^= v[13];
    v[6] ^= v[14];
    v[7] ^= v[15];

    v[0] ^= state.s[0];
    v[1] ^= state.s[1];
    v[2] ^= state.s[2];
    v[3] ^= state.s[3];
    v[4] ^= state.s[0];
    v[5] ^= state.s[1];
    v[6] ^= state.s[2];
    v[7] ^= state.s[3];

    state.h[0] ^= v[0];
    state.h[1] ^= v[1];
    state.h[2] ^= v[2];
    state.h[3] ^= v[3];
    state.h[4] ^= v[4];
    state.h[5] ^= v[5];
    state.h[6] ^= v[6];
    state.h[7] ^= v[7];
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
    state.s[0] = 0;
    state.s[1] = 0;
    state.s[2] = 0;
    state.s[3] = 0;
}

/// `blake512_update` — the C API counts `datalen` in **bits**.
pub fn blake512_update_bits(state: &mut BlakeState512, data: &[u8], datalen: u64) {
    let mut off: usize = 0;
    let mut datalen = datalen;

    let mut left: usize = (state.buflen >> 3) as usize;
    let fill: usize = 128 - left;

    if left != 0 && (((datalen >> 3) & 0x7F) >= fill as u64) {
        state.buf[left..left + fill].copy_from_slice(&data[off..off + fill]);
        state.t[0] = state.t[0].wrapping_add(1024);
        let block = state.buf;
        blake512_compress(state, &block);
        off += fill;
        datalen = datalen.wrapping_sub((fill as i32 as u64) << 3);
        left = 0;
    }

    while datalen >= 1024 {
        state.t[0] = state.t[0].wrapping_add(1024);
        blake512_compress(state, &data[off..off + 128]);
        off += 128;
        datalen -= 1024;
    }

    if datalen > 0 {
        let n = ((datalen >> 3) & 0x7F) as usize;
        state.buf[left..left + n].copy_from_slice(&data[off..off + n]);
        state.buflen = (((left as u64) << 3).wrapping_add(datalen)) as i32;
    } else {
        state.buflen = 0;
    }
}

/// Byte-slice convenience wrapper matching `blake512_update(S, in, 8*inlen)`.
pub fn blake512_update(state: &mut BlakeState512, input: &[u8]) {
    blake512_update_bits(state, input, (input.len() as u64) * 8);
}

pub fn blake512_final(state: &mut BlakeState512, digest: &mut [u8]) {
    let mut msglen = [0u8; 16];
    let zo: [u8; 1] = [0x01];
    let oo: [u8; 1] = [0x81];

    let lo: u64 = state.t[0].wrapping_add(state.buflen as i64 as u64);
    let mut hi: u64 = state.t[1];
    if lo < state.buflen as i64 as u64 {
        hi = hi.wrapping_add(1);
    }
    u64to8(&mut msglen[0..], hi);
    u64to8(&mut msglen[8..], lo);

    if state.buflen == 888 {
        /* one padding byte */
        state.t[0] = state.t[0].wrapping_sub(8);
        blake512_update_bits(state, &oo, 8);
    } else {
        if state.buflen < 888 {
            /* enough space to fill the block */
            if state.buflen == 0 {
                state.nullt = 1;
            }
            state.t[0] = state.t[0].wrapping_sub((888 - state.buflen) as i64 as u64);
            blake512_update_bits(state, &PADDING, (888 - state.buflen) as i64 as u64);
        } else {
            /* NOT enough space, need 2 compressions */
            state.t[0] = state.t[0].wrapping_sub((1024 - state.buflen) as i64 as u64);
            blake512_update_bits(state, &PADDING, (1024 - state.buflen) as i64 as u64);
            state.t[0] = state.t[0].wrapping_sub(888);
            blake512_update_bits(state, &PADDING[1..], 888);
            state.nullt = 1;
        }
        blake512_update_bits(state, &zo, 8);
        state.t[0] = state.t[0].wrapping_sub(8);
    }
    state.t[0] = state.t[0].wrapping_sub(128);
    blake512_update_bits(state, &msglen, 128);

    u64to8(&mut digest[0..], state.h[0]);
    u64to8(&mut digest[8..], state.h[1]);
    u64to8(&mut digest[16..], state.h[2]);
    u64to8(&mut digest[24..], state.h[3]);
    u64to8(&mut digest[32..], state.h[4]);
    u64to8(&mut digest[40..], state.h[5]);
    u64to8(&mut digest[48..], state.h[6]);
    u64to8(&mut digest[56..], state.h[7]);
}

/// Stack scratch size for the `SPX_VLA(uint8_t, inbuf, inlen+4)` of `*_mgf1`.
const MGF1_INBUF_MAX: usize = 256;

pub fn blake512_mgf1(out: &mut [u8], input: &[u8]) {
    let inlen = input.len();
    let outlen = out.len();

    let mut inbuf_arr = [0u8; MGF1_INBUF_MAX];
    let mut inbuf_heap: Vec<u8> = Vec::new();
    let inbuf: &mut [u8] = if inlen + 4 <= MGF1_INBUF_MAX {
        &mut inbuf_arr[..inlen + 4]
    } else {
        inbuf_heap.resize(inlen + 4, 0);
        &mut inbuf_heap[..]
    };
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];

    inbuf[..inlen].copy_from_slice(input);

    /* While we can fit in at least another full block of BLAKE512 output.. */
    let mut i: usize = 0;
    while (i + 1) * SPX_BLAKE512_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512(&mut out[i * SPX_BLAKE512_OUTPUT_BYTES..], inbuf);
        i += 1;
    }
    /* Until we cannot anymore, and we fill the remainder. */
    if outlen > i * SPX_BLAKE512_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake512(&mut outbuf, inbuf);
        let n = outlen - i * SPX_BLAKE512_OUTPUT_BYTES;
        out[i * SPX_BLAKE512_OUTPUT_BYTES..outlen].copy_from_slice(&outbuf[..n]);
    }
}

pub fn blake512(out: &mut [u8], input: &[u8]) -> i32 {
    let mut s = BlakeState512::new();
    blake512_init(&mut s);
    blake512_update_bits(&mut s, input, (input.len() as u64) * 8);
    blake512_final(&mut s, out);
    0
}
