//! Translation of `lib/blake/src/blake256.c`.
//!
//! BLAKE reference C implementation
//! Copyright (c) 2012 Jean-Philippe Aumasson <jeanphilippe.aumasson@gmail.com>
//! (CC0 / public domain)
//!
//! All arithmetic is reproduced with wrapping semantics so that the observable
//! behaviour is bit-identical to the C original, including the (deliberately
//! preserved) quirks of the bit-length based `update`/`final` bookkeeping.

use crate::utils::u32_to_bytes;

/// `SPX_BLAKE256_OUTPUT_BYTES` from `lib/blake/include/blake.h`.
/// This does not necessarily equal `SPX_N`.
pub const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;

/// C `blakestate256`:
/// ```c
/// typedef struct {
///   unsigned int h[8], s[4], t[2];
///   int buflen, nullt;
///   unsigned char buf[64];
/// } blakestate256;
/// ```
#[repr(C)]
#[derive(Clone)]
pub struct BlakeState256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

impl BlakeState256 {
    #[inline]
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

static CST: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344, 0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C, 0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

static PADDING: [u8; 64] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0,
];

/// The message-word / constant permutation used by the `ROUND(...)` invocations.
/// Argument `k` of `ROUND` uses message word `SIGMA[r][k]` and constant
/// `cst[SIGMA[r][k ^ 1]]`, exactly as spelled out in the C source.
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

/// `#define U8TO32(p)` — big endian load.
#[inline(always)]
fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}

/// `#define U32TO8(p, v)` — big endian store.
#[inline(always)]
fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

/// `#define BLAKE256_ROT(x,n) (((x)<<(32-n))|((x)>>(n)))`
#[inline(always)]
fn rot(x: u32, n: u32) -> u32 {
    x.rotate_right(n)
}

/// One quarter of the `ROUND` macro body:
/// `va += mc; va += vb; vd ^= va; vd = ROT(vd,r1); vc += vd; vb ^= vc; vb = ROT(vb,r2);`
#[inline(always)]
fn g(v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mc: u32, r1: u32, r2: u32) {
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
fn round(v: &mut [u32; 16], m: &[u32; 16], s: &[usize; 16]) {
    macro_rules! mc {
        ($k:expr) => {
            m[s[$k]] ^ CST[s[$k ^ 1]]
        };
    }

    g(v, 0, 4, 8, 12, mc!(0), 16, 12);
    g(v, 1, 5, 9, 13, mc!(2), 16, 12);
    g(v, 2, 6, 10, 14, mc!(4), 16, 12);
    g(v, 3, 7, 11, 15, mc!(6), 16, 12);
    g(v, 2, 6, 10, 14, mc!(5), 8, 7);
    g(v, 3, 7, 11, 15, mc!(7), 8, 7);
    g(v, 1, 5, 9, 13, mc!(3), 8, 7);
    g(v, 0, 4, 8, 12, mc!(1), 8, 7);
    g(v, 0, 5, 10, 15, mc!(8), 16, 12);
    g(v, 1, 6, 11, 12, mc!(10), 16, 12);
    g(v, 2, 7, 8, 13, mc!(12), 16, 12);
    g(v, 3, 4, 9, 14, mc!(14), 16, 12);
    g(v, 2, 7, 8, 13, mc!(13), 8, 7);
    g(v, 3, 4, 9, 14, mc!(15), 8, 7);
    g(v, 1, 6, 11, 12, mc!(11), 8, 7);
    g(v, 0, 5, 10, 15, mc!(9), 8, 7);
}

pub fn blake256_compress(state: &mut BlakeState256, block: &[u8]) {
    let mut m = [0u32; 16];
    for (i, w) in m.iter_mut().enumerate() {
        *w = u8to32(&block[4 * i..]);
    }

    let mut v = [0u32; 16];
    v[0] = state.h[0];
    v[1] = state.h[1];
    v[2] = state.h[2];
    v[3] = state.h[3];
    v[4] = state.h[4];
    v[5] = state.h[5];
    v[6] = state.h[6];
    v[7] = state.h[7];
    v[8] = state.s[0] ^ 0x243F6A88;
    v[9] = state.s[1] ^ 0x85A308D3;
    v[10] = state.s[2] ^ 0x13198A2E;
    v[11] = state.s[3] ^ 0x03707344;
    v[12] = 0xA4093822;
    v[13] = 0x299F31D0;
    v[14] = 0x082EFA98;
    v[15] = 0xEC4E6C89;
    if state.nullt == 0 {
        v[12] ^= state.t[0];
        v[13] ^= state.t[0];
        v[14] ^= state.t[1];
        v[15] ^= state.t[1];
    }

    /* 14 rounds: sigma rows 0..9 followed by rows 0..3 again. */
    for r in 0..10 {
        round(&mut v, &m, &SIGMA[r]);
    }
    for r in 0..4 {
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
    state.s[0] = 0;
    state.s[1] = 0;
    state.s[2] = 0;
    state.s[3] = 0;
}

/// `blake256_update` — note that the C API counts `datalen` in **bits**.
/// This is the faithful translation; `blake256_update` below is the
/// byte-slice convenience wrapper (`datalen = 8 * input.len()`).
pub fn blake256_update_bits(state: &mut BlakeState256, data: &[u8], datalen: u64) {
    let mut off: usize = 0; /* stands in for the `data` pointer increments */
    let mut datalen = datalen;

    let mut left: usize = (state.buflen >> 3) as usize;
    let fill: usize = 64 - left;

    if left != 0 && (((datalen >> 3) & 0x3F) >= fill as u64) {
        state.buf[left..left + fill].copy_from_slice(&data[off..off + fill]);
        state.t[0] = state.t[0].wrapping_add(512);
        if state.t[0] == 0 {
            state.t[1] = state.t[1].wrapping_add(1);
        }
        let block = state.buf;
        blake256_compress(state, &block);
        off += fill;
        datalen = datalen.wrapping_sub((fill as i32 as u64) << 3);
        left = 0;
    }

    while datalen >= 512 {
        state.t[0] = state.t[0].wrapping_add(512);
        if state.t[0] == 0 {
            state.t[1] = state.t[1].wrapping_add(1);
        }
        blake256_compress(state, &data[off..off + 64]);
        off += 64;
        datalen -= 512;
    }

    if datalen > 0 {
        let n = (datalen >> 3) as usize;
        state.buf[left..left + n].copy_from_slice(&data[off..off + n]);
        state.buflen = (((left as u64) << 3).wrapping_add(datalen)) as i32;
    } else {
        state.buflen = 0;
    }
}

/// Byte-slice convenience wrapper matching `blake256_update(S, in, 8*inlen)`.
pub fn blake256_update(state: &mut BlakeState256, input: &[u8]) {
    blake256_update_bits(state, input, (input.len() as u64) * 8);
}

pub fn blake256_final(state: &mut BlakeState256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let zo: [u8; 1] = [0x01];
    let oo: [u8; 1] = [0x81];

    let lo: u32 = state.t[0].wrapping_add(state.buflen as u32);
    let mut hi: u32 = state.t[1];
    if lo < state.buflen as u32 {
        hi = hi.wrapping_add(1);
    }
    u32to8(&mut msglen[0..], hi);
    u32to8(&mut msglen[4..], lo);

    if state.buflen == 440 {
        /* one padding byte */
        state.t[0] = state.t[0].wrapping_sub(8);
        blake256_update_bits(state, &oo, 8);
    } else {
        if state.buflen < 440 {
            /* enough space to fill the block */
            if state.buflen == 0 {
                state.nullt = 1;
            }
            state.t[0] = state.t[0].wrapping_sub((440 - state.buflen) as u32);
            blake256_update_bits(state, &PADDING, (440 - state.buflen) as u64);
        } else {
            /* need 2 compressions */
            state.t[0] = state.t[0].wrapping_sub((512 - state.buflen) as u32);
            blake256_update_bits(state, &PADDING, (512 - state.buflen) as u64);
            state.t[0] = state.t[0].wrapping_sub(440);
            blake256_update_bits(state, &PADDING[1..], 440);
            state.nullt = 1;
        }
        blake256_update_bits(state, &zo, 8);
        state.t[0] = state.t[0].wrapping_sub(8);
    }
    state.t[0] = state.t[0].wrapping_sub(64);
    blake256_update_bits(state, &msglen, 64);

    u32to8(&mut digest[0..], state.h[0]);
    u32to8(&mut digest[4..], state.h[1]);
    u32to8(&mut digest[8..], state.h[2]);
    u32to8(&mut digest[12..], state.h[3]);
    u32to8(&mut digest[16..], state.h[4]);
    u32to8(&mut digest[20..], state.h[5]);
    u32to8(&mut digest[24..], state.h[6]);
    u32to8(&mut digest[28..], state.h[7]);
}

pub fn blake256_mgf1(out: &mut [u8], input: &[u8]) {
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
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    inbuf[..inlen].copy_from_slice(input);

    /* While we can fit in at least another full block of BLAKE256 output.. */
    let mut i: usize = 0;
    while (i + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256(&mut out[i * SPX_BLAKE256_OUTPUT_BYTES..], inbuf);
        i += 1;
    }
    /* Until we cannot anymore, and we fill the remainder. */
    if outlen > i * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256(&mut outbuf, inbuf);
        let n = outlen - i * SPX_BLAKE256_OUTPUT_BYTES;
        out[i * SPX_BLAKE256_OUTPUT_BYTES..outlen].copy_from_slice(&outbuf[..n]);
    }
}

/// Stack scratch size for the `SPX_VLA(uint8_t, inbuf, inlen+4)` of `*_mgf1`.
/// Larger inputs fall back to the heap; the C version uses a VLA and therefore
/// has no such limit, but every in-tree caller stays far below this bound.
const MGF1_INBUF_MAX: usize = 256;

pub fn blake256(out: &mut [u8], input: &[u8]) -> i32 {
    let mut s = BlakeState256::new();
    blake256_init(&mut s);
    blake256_update_bits(&mut s, input, (input.len() as u64) * 8);
    blake256_final(&mut s, out);
    0
}
