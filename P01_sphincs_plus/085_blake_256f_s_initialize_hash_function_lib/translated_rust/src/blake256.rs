use crate::params::*;
use crate::context::u32_to_bytes;

pub struct Blakestate256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}

fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

const CST256: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

static PADDING256: [u8; 64] = [
    0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

#[inline(always)]
fn blake256_rot(x: u32, n: u32) -> u32 {
    (x << (32 - n)) | (x >> n)
}

// The ROUND macro from C, implemented as a function operating on the v array
// The sigma permutation is baked into the caller via the m/c index arrays
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn blake256_round(
    v: &mut [u32; 16],
    m: &[u32; 16],
    // indices into m and cst for each of the 16 message words
    mi: [usize; 16],
    ci: [usize; 16],
) {
    // Column step
    // G(0,4,8,12, mi[0],ci[0], mi[1],ci[1])
    v[0] = v[0].wrapping_add(m[mi[0]] ^ CST256[ci[0]]);
    v[0] = v[0].wrapping_add(v[4]);
    v[12] ^= v[0]; v[12] = blake256_rot(v[12], 16);
    v[8] = v[8].wrapping_add(v[12]);
    v[4] ^= v[8]; v[4] = blake256_rot(v[4], 12);

    v[1] = v[1].wrapping_add(m[mi[2]] ^ CST256[ci[2]]);
    v[1] = v[1].wrapping_add(v[5]);
    v[13] ^= v[1]; v[13] = blake256_rot(v[13], 16);
    v[9] = v[9].wrapping_add(v[13]);
    v[5] ^= v[9]; v[5] = blake256_rot(v[5], 12);

    v[2] = v[2].wrapping_add(m[mi[4]] ^ CST256[ci[4]]);
    v[2] = v[2].wrapping_add(v[6]);
    v[14] ^= v[2]; v[14] = blake256_rot(v[14], 16);
    v[10] = v[10].wrapping_add(v[14]);
    v[6] ^= v[10]; v[6] = blake256_rot(v[6], 12);

    v[3] = v[3].wrapping_add(m[mi[6]] ^ CST256[ci[6]]);
    v[3] = v[3].wrapping_add(v[7]);
    v[15] ^= v[3]; v[15] = blake256_rot(v[15], 16);
    v[11] = v[11].wrapping_add(v[15]);
    v[7] ^= v[11]; v[7] = blake256_rot(v[7], 12);

    v[2] = v[2].wrapping_add(m[mi[5]] ^ CST256[ci[5]]);
    v[2] = v[2].wrapping_add(v[6]);
    v[14] ^= v[2]; v[14] = blake256_rot(v[14], 8);
    v[10] = v[10].wrapping_add(v[14]);
    v[6] ^= v[10]; v[6] = blake256_rot(v[6], 7);

    v[3] = v[3].wrapping_add(m[mi[7]] ^ CST256[ci[7]]);
    v[3] = v[3].wrapping_add(v[7]);
    v[15] ^= v[3]; v[15] = blake256_rot(v[15], 8);
    v[11] = v[11].wrapping_add(v[15]);
    v[7] ^= v[11]; v[7] = blake256_rot(v[7], 7);

    v[1] = v[1].wrapping_add(m[mi[3]] ^ CST256[ci[3]]);
    v[1] = v[1].wrapping_add(v[5]);
    v[13] ^= v[1]; v[13] = blake256_rot(v[13], 8);
    v[9] = v[9].wrapping_add(v[13]);
    v[5] ^= v[9]; v[5] = blake256_rot(v[5], 7);

    v[0] = v[0].wrapping_add(m[mi[1]] ^ CST256[ci[1]]);
    v[0] = v[0].wrapping_add(v[4]);
    v[12] ^= v[0]; v[12] = blake256_rot(v[12], 8);
    v[8] = v[8].wrapping_add(v[12]);
    v[4] ^= v[8]; v[4] = blake256_rot(v[4], 7);

    // Diagonal step
    v[0] = v[0].wrapping_add(m[mi[8]] ^ CST256[ci[8]]);
    v[0] = v[0].wrapping_add(v[5]);
    v[15] ^= v[0]; v[15] = blake256_rot(v[15], 16);
    v[10] = v[10].wrapping_add(v[15]);
    v[5] ^= v[10]; v[5] = blake256_rot(v[5], 12);

    v[1] = v[1].wrapping_add(m[mi[10]] ^ CST256[ci[10]]);
    v[1] = v[1].wrapping_add(v[6]);
    v[12] ^= v[1]; v[12] = blake256_rot(v[12], 16);
    v[11] = v[11].wrapping_add(v[12]);
    v[6] ^= v[11]; v[6] = blake256_rot(v[6], 12);

    v[2] = v[2].wrapping_add(m[mi[12]] ^ CST256[ci[12]]);
    v[2] = v[2].wrapping_add(v[7]);
    v[13] ^= v[2]; v[13] = blake256_rot(v[13], 16);
    v[8] = v[8].wrapping_add(v[13]);
    v[7] ^= v[8]; v[7] = blake256_rot(v[7], 12);

    v[3] = v[3].wrapping_add(m[mi[14]] ^ CST256[ci[14]]);
    v[3] = v[3].wrapping_add(v[4]);
    v[14] ^= v[3]; v[14] = blake256_rot(v[14], 16);
    v[9] = v[9].wrapping_add(v[14]);
    v[4] ^= v[9]; v[4] = blake256_rot(v[4], 12);

    v[2] = v[2].wrapping_add(m[mi[13]] ^ CST256[ci[13]]);
    v[2] = v[2].wrapping_add(v[7]);
    v[13] ^= v[2]; v[13] = blake256_rot(v[13], 8);
    v[8] = v[8].wrapping_add(v[13]);
    v[7] ^= v[8]; v[7] = blake256_rot(v[7], 7);

    v[3] = v[3].wrapping_add(m[mi[15]] ^ CST256[ci[15]]);
    v[3] = v[3].wrapping_add(v[4]);
    v[14] ^= v[3]; v[14] = blake256_rot(v[14], 8);
    v[9] = v[9].wrapping_add(v[14]);
    v[4] ^= v[9]; v[4] = blake256_rot(v[4], 7);

    v[1] = v[1].wrapping_add(m[mi[11]] ^ CST256[ci[11]]);
    v[1] = v[1].wrapping_add(v[6]);
    v[12] ^= v[1]; v[12] = blake256_rot(v[12], 8);
    v[11] = v[11].wrapping_add(v[12]);
    v[6] ^= v[11]; v[6] = blake256_rot(v[6], 7);

    v[0] = v[0].wrapping_add(m[mi[9]] ^ CST256[ci[9]]);
    v[0] = v[0].wrapping_add(v[5]);
    v[15] ^= v[0]; v[15] = blake256_rot(v[15], 8);
    v[10] = v[10].wrapping_add(v[15]);
    v[5] ^= v[10]; v[5] = blake256_rot(v[5], 7);
}

// The 14 rounds use the BLAKE permutation sigma
// Each ROUND in the C code uses: ROUND(m_sigma[0], cst[sigma_c[0]], m_sigma[1], cst[sigma_c[1]], ...)
// The C code's ROUND macro takes pairs (m_val, c_val) for positions 0..15
// where the m_val is the actual message word and c_val is the actual constant
// We need to map this to indices.
// From the C source, the 14 round calls use these sigma permutations:
const SIGMA: [[usize; 16]; 14] = [
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
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
];

// The C ROUND macro's constant indices: for ROUND(m0,cst[c0],m1,cst[c1],...) 
// the c indices are the "other" sigma permutation element
// Looking at the C: ROUND(m0,cst[1],m1,cst[0],...) means mi=[0,1,...], ci=[1,0,...]
// The pattern is: ci[i] = sigma[the_other_element_at_position_i]
// Actually from the C code, each ROUND call passes pairs where:
//   position 0: (m_sigma[0], cst[sigma[1]])
//   position 1: (m_sigma[1], cst[sigma[0]])
//   position 2: (m_sigma[2], cst[sigma[3]])
//   position 3: (m_sigma[3], cst[sigma[2]])
//   etc - the constant index is the "partner" in each pair
const SIGMA_C: [[usize; 16]; 14] = [
    [1, 0, 3, 2, 5, 4, 7, 6, 9, 8, 11, 10, 13, 12, 15, 14],
    [10, 14, 8, 4, 15, 9, 6, 13, 12, 1, 2, 0, 7, 11, 3, 5],
    [8, 11, 0, 12, 2, 5, 13, 15, 14, 10, 6, 3, 1, 7, 4, 9],
    [9, 7, 1, 3, 12, 13, 14, 11, 6, 2, 10, 5, 0, 4, 8, 15],
    [0, 9, 7, 5, 4, 2, 15, 10, 1, 14, 12, 11, 8, 6, 13, 3],
    [12, 2, 10, 6, 11, 0, 3, 8, 13, 4, 5, 7, 14, 15, 9, 1],
    [5, 12, 15, 1, 13, 14, 10, 4, 7, 0, 3, 6, 2, 9, 11, 8],
    [11, 13, 14, 7, 1, 12, 9, 3, 0, 5, 4, 15, 6, 8, 10, 2],
    [15, 6, 9, 14, 3, 11, 8, 0, 2, 12, 7, 13, 4, 1, 5, 10],
    [2, 10, 4, 8, 6, 7, 5, 1, 11, 15, 14, 9, 12, 3, 0, 13],
    [1, 0, 3, 2, 5, 4, 7, 6, 9, 8, 11, 10, 13, 12, 15, 14],
    [10, 14, 8, 4, 15, 9, 6, 13, 12, 1, 2, 0, 7, 11, 3, 5],
    [8, 11, 0, 12, 2, 5, 13, 15, 14, 10, 6, 3, 1, 7, 4, 9],
    [9, 7, 1, 3, 12, 13, 14, 11, 6, 2, 10, 5, 0, 4, 8, 15],
];

pub fn blake256_compress(s: &mut Blakestate256, block: &[u8]) {
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = u8to32(&block[i * 4..]);
    }

    let mut v = [0u32; 16];
    v[0] = s.h[0]; v[1] = s.h[1]; v[2] = s.h[2]; v[3] = s.h[3];
    v[4] = s.h[4]; v[5] = s.h[5]; v[6] = s.h[6]; v[7] = s.h[7];
    v[8] = s.s[0] ^ 0x243F6A88;
    v[9] = s.s[1] ^ 0x85A308D3;
    v[10] = s.s[2] ^ 0x13198A2E;
    v[11] = s.s[3] ^ 0x03707344;
    v[12] = 0xA4093822;
    v[13] = 0x299F31D0;
    v[14] = 0x082EFA98;
    v[15] = 0xEC4E6C89;

    if s.nullt == 0 {
        v[12] ^= s.t[0];
        v[13] ^= s.t[0];
        v[14] ^= s.t[1];
        v[15] ^= s.t[1];
    }

    for r in 0..14 {
        blake256_round(&mut v, &m, SIGMA[r], SIGMA_C[r]);
    }

    for i in 0..8 { v[i] ^= v[i + 8]; }
    v[0] ^= s.s[0]; v[1] ^= s.s[1]; v[2] ^= s.s[2]; v[3] ^= s.s[3];
    v[4] ^= s.s[0]; v[5] ^= s.s[1]; v[6] ^= s.s[2]; v[7] ^= s.s[3];
    for i in 0..8 { s.h[i] ^= v[i]; }
}

pub fn blake256_init(s: &mut Blakestate256) {
    s.h[0] = 0x6A09E667; s.h[1] = 0xBB67AE85;
    s.h[2] = 0x3C6EF372; s.h[3] = 0xA54FF53A;
    s.h[4] = 0x510E527F; s.h[5] = 0x9B05688C;
    s.h[6] = 0x1F83D9AB; s.h[7] = 0x5BE0CD19;
    s.t[0] = 0; s.t[1] = 0; s.buflen = 0; s.nullt = 0;
    s.s[0] = 0; s.s[1] = 0; s.s[2] = 0; s.s[3] = 0;
}

pub fn blake256_update(s: &mut Blakestate256, data: &[u8], mut datalen: u64) {
    let mut offset = 0usize;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && (((datalen >> 3) & 0x3F) as usize) >= fill {
        s.buf[left..left + fill].copy_from_slice(&data[offset..offset + fill]);
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 { s.t[1] = s.t[1].wrapping_add(1); }
        let buf_copy = s.buf;
        blake256_compress(s, &buf_copy);
        offset += fill;
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 512 {
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 { s.t[1] = s.t[1].wrapping_add(1); }
        blake256_compress(s, &data[offset..]);
        offset += 64;
        datalen -= 512;
    }

    if datalen > 0 {
        let bytes = (datalen >> 3) as usize;
        s.buf[left..left + bytes].copy_from_slice(&data[offset..offset + bytes]);
        s.buflen = ((left << 3) as u64 + datalen) as i32;
    } else {
        s.buflen = 0;
    }
}

pub fn blake256_final(s: &mut Blakestate256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = s.t[0].wrapping_add(s.buflen as u32);
    let mut hi = s.t[1];
    if lo < s.buflen as u32 { hi = hi.wrapping_add(1); }
    u32to8(&mut msglen[0..4], hi);
    u32to8(&mut msglen[4..8], lo);

    if s.buflen == 440 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake256_update(s, &[oo], 8);
    } else {
        if s.buflen < 440 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((440 - s.buflen) as u32);
            blake256_update(s, &PADDING256[..], (440 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((512 - s.buflen) as u32);
            blake256_update(s, &PADDING256[..], (512 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &PADDING256[1..], 440);
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

pub fn blake256(out: &mut [u8], inp: &[u8], inlen: u64) -> i32 {
    let mut s = Blakestate256 {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64],
    };
    blake256_init(&mut s);
    blake256_update(&mut s, inp, inlen.wrapping_mul(8));
    blake256_final(&mut s, out);
    0
}

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i: u64 = 0;
    let mut off = 0usize;
    while (i + 1) * (SPX_BLAKE256_OUTPUT_BYTES as u64) <= outlen as u64 {
        u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        blake256(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > (i as usize) * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - (i as usize) * SPX_BLAKE256_OUTPUT_BYTES;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}
