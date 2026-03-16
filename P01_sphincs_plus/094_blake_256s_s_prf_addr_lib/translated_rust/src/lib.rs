#![allow(non_snake_case, non_camel_case_types, clippy::identity_op)]

// SPHINCS+ blake-256s parameters
const SPX_N: usize = 32;
const SPX_ADDR_BYTES: usize = 32;
const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;

#[repr(C)]
pub struct spx_ctx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
}

// BLAKE-256 constants
const CST: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

const PADDING: [u8; 64] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

struct Blakestate256 {
    h: [u32; 8],
    s: [u32; 4],
    t: [u32; 2],
    buflen: i32,
    nullt: i32,
    buf: [u8; 64],
}

fn u8to32(p: &[u8]) -> u32 {
    (u32::from(p[0]) << 24)
        | (u32::from(p[1]) << 16)
        | (u32::from(p[2]) << 8)
        | u32::from(p[3])
}

fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

fn blake256_rot(x: u32, n: u32) -> u32 {
    (x << (32 - n)) | (x >> n)
}

/// The G function used in each quarter-round of BLAKE-256.
/// Operates on the 4x4 working vector v, using message words and constants.
#[inline(always)]
fn g256(v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mx: u32, cx: u32, my: u32, cy: u32) {
    v[a] = v[a].wrapping_add(mx ^ cx).wrapping_add(v[b]);
    v[d] ^= v[a];
    v[d] = blake256_rot(v[d], 16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] ^= v[c];
    v[b] = blake256_rot(v[b], 12);
    v[a] = v[a].wrapping_add(my ^ cy).wrapping_add(v[b]);
    v[d] ^= v[a];
    v[d] = blake256_rot(v[d], 8);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] ^= v[c];
    v[b] = blake256_rot(v[b], 7);
}

// BLAKE-256 sigma permutations
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

fn blake256_compress(state: &mut Blakestate256, block: &[u8]) {
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = u8to32(&block[i * 4..]);
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

    // 14 rounds (sigma repeats after 10)
    for round in 0..14 {
        let s = &SIGMA[round % 10];
        // Column step
        g256(&mut v, 0, 4, 8, 12, m[s[0]], CST[s[1]], m[s[1]], CST[s[0]]);
        g256(&mut v, 1, 5, 9, 13, m[s[2]], CST[s[3]], m[s[3]], CST[s[2]]);
        g256(&mut v, 2, 6, 10, 14, m[s[4]], CST[s[5]], m[s[5]], CST[s[4]]);
        g256(&mut v, 3, 7, 11, 15, m[s[6]], CST[s[7]], m[s[7]], CST[s[6]]);
        // Diagonal step
        g256(&mut v, 0, 5, 10, 15, m[s[8]], CST[s[9]], m[s[9]], CST[s[8]]);
        g256(&mut v, 1, 6, 11, 12, m[s[10]], CST[s[11]], m[s[11]], CST[s[10]]);
        g256(&mut v, 2, 7, 8, 13, m[s[12]], CST[s[13]], m[s[13]], CST[s[12]]);
        g256(&mut v, 3, 4, 9, 14, m[s[14]], CST[s[15]], m[s[15]], CST[s[14]]);
    }

    for i in 0..8 {
        v[i] ^= v[i + 8];
    }
    for i in 0..4 {
        v[i] ^= state.s[i];
        v[i + 4] ^= state.s[i];
    }
    for i in 0..8 {
        state.h[i] ^= v[i];
    }
}

fn blake256_init(s: &mut Blakestate256) {
    s.h[0] = 0x6A09E667;
    s.h[1] = 0xBB67AE85;
    s.h[2] = 0x3C6EF372;
    s.h[3] = 0xA54FF53A;
    s.h[4] = 0x510E527F;
    s.h[5] = 0x9B05688C;
    s.h[6] = 0x1F83D9AB;
    s.h[7] = 0x5BE0CD19;
    s.t = [0; 2];
    s.buflen = 0;
    s.nullt = 0;
    s.s = [0; 4];
}

fn blake256_update(s: &mut Blakestate256, data: &[u8], datalen: u64) {
    let mut datalen = datalen;
    let mut data = data;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && ((datalen >> 3) & 0x3F) >= fill as u64 {
        s.buf[left..left + fill].copy_from_slice(&data[..fill]);
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 {
            s.t[1] = s.t[1].wrapping_add(1);
        }
        let buf_copy = s.buf;
        blake256_compress(s, &buf_copy);
        data = &data[fill..];
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 512 {
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 {
            s.t[1] = s.t[1].wrapping_add(1);
        }
        blake256_compress(s, data);
        data = &data[64..];
        datalen -= 512;
    }

    if datalen > 0 {
        let bytes = (datalen >> 3) as usize;
        s.buf[left..left + bytes].copy_from_slice(&data[..bytes]);
        s.buflen = ((left << 3) as u64 + datalen) as i32;
    } else {
        s.buflen = 0;
    }
}

fn blake256_final(s: &mut Blakestate256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = s.t[0].wrapping_add(s.buflen as u32);
    let mut hi = s.t[1];
    if lo < s.buflen as u32 {
        hi = hi.wrapping_add(1);
    }
    u32to8(&mut msglen[0..4], hi);
    u32to8(&mut msglen[4..8], lo);

    if s.buflen == 440 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake256_update(s, &[oo], 8);
    } else {
        if s.buflen < 440 {
            if s.buflen == 0 {
                s.nullt = 1;
            }
            s.t[0] = s.t[0].wrapping_sub((440 - s.buflen) as u32);
            blake256_update(s, &PADDING[..(((440 - s.buflen) >> 3) as usize)], (440 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((512 - s.buflen) as u32);
            blake256_update(s, &PADDING[..(((512 - s.buflen) >> 3) as usize)], (512 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &PADDING[1..(1 + 55)], 440);
            s.nullt = 1;
        }
        blake256_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(64);
    blake256_update(s, &msglen, 64);

    for i in 0..8 {
        u32to8(&mut digest[i * 4..], s.h[i]);
    }
}

fn blake256(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = Blakestate256 {
        h: [0; 8],
        s: [0; 4],
        t: [0; 2],
        buflen: 0,
        nullt: 0,
        buf: [0; 64],
    };
    blake256_init(&mut s);
    blake256_update(&mut s, inp, inlen.wrapping_mul(8));
    blake256_final(&mut s, out);
    0
}

/// # Safety
/// All pointers must be valid. `out` must point to at least SPX_N bytes.
/// `ctx` must point to a valid spx_ctx. `addr` must point to 8 u32s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(
    out: *mut u8,
    ctx: *const spx_ctx,
    addr: *const u32,
) {
    let ctx = unsafe { &*ctx };
    let addr_bytes: &[u8] =
        unsafe { core::slice::from_raw_parts(addr as *const u8, SPX_ADDR_BYTES) };
    let out = unsafe { core::slice::from_raw_parts_mut(out, SPX_N) };

    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);

    blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);

    out.copy_from_slice(&outbuf[..SPX_N]);
}
