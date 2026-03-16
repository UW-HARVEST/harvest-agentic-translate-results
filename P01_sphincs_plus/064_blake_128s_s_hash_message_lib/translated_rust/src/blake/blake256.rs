use crate::params::SPX_BLAKE256_OUTPUT_BYTES;
use crate::utils::u32_to_bytes;

fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}

fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

const CST: [u32; 16] = [
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

fn rot32(x: u32, n: u32) -> u32 {
    (x << (32 - n)) | (x >> n)
}

#[derive(Clone)]
pub struct BlakeState256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
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
    state.t = [0, 0];
    state.buflen = 0;
    state.nullt = 0;
    state.s = [0, 0, 0, 0];
}

pub fn blake256_compress(s: &mut BlakeState256, block: &[u8]) {
    let m: [u32; 16] = [
        u8to32(&block[0..]), u8to32(&block[4..]), u8to32(&block[8..]), u8to32(&block[12..]),
        u8to32(&block[16..]), u8to32(&block[20..]), u8to32(&block[24..]), u8to32(&block[28..]),
        u8to32(&block[32..]), u8to32(&block[36..]), u8to32(&block[40..]), u8to32(&block[44..]),
        u8to32(&block[48..]), u8to32(&block[52..]), u8to32(&block[56..]), u8to32(&block[60..]),
    ];

    let mut v: [u32; 16] = [
        s.h[0], s.h[1], s.h[2], s.h[3], s.h[4], s.h[5], s.h[6], s.h[7],
        s.s[0] ^ 0x243F6A88, s.s[1] ^ 0x85A308D3, s.s[2] ^ 0x13198A2E, s.s[3] ^ 0x03707344,
        0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    ];

    if s.nullt == 0 {
        v[12] ^= s.t[0];
        v[13] ^= s.t[0];
        v[14] ^= s.t[1];
        v[15] ^= s.t[1];
    }

    // The BLAKE-256 sigma permutations (14 rounds)
    const SIGMA: [[usize; 16]; 14] = [
        [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15],
        [14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3],
        [11,8,12,0,5,2,15,13,10,14,3,6,7,1,9,4],
        [7,9,3,1,13,12,11,14,2,6,5,10,4,0,15,8],
        [9,0,5,7,2,4,10,15,14,1,11,12,6,8,3,13],
        [2,12,6,10,0,11,8,3,4,13,7,5,15,14,1,9],
        [12,5,1,15,14,13,4,10,0,7,6,3,9,2,8,11],
        [13,11,7,14,12,1,3,9,5,0,15,4,8,6,2,10],
        [6,15,14,9,11,3,0,8,12,2,13,7,1,4,10,5],
        [10,2,8,4,7,6,1,5,15,11,9,14,3,12,13,0],
        [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15],
        [14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3],
        [11,8,12,0,5,2,15,13,10,14,3,6,7,1,9,4],
        [7,9,3,1,13,12,11,14,2,6,5,10,4,0,15,8],
    ];

    for round in 0..14 {
        let sg = &SIGMA[round];
        // Column step
        // G(0,4,8,12, sg[0],sg[1])
        v[0] = v[0].wrapping_add(m[sg[0]] ^ CST[sg[1]]).wrapping_add(v[4]);
        v[12] ^= v[0]; v[12] = rot32(v[12], 16);
        v[8] = v[8].wrapping_add(v[12]);
        v[4] ^= v[8]; v[4] = rot32(v[4], 12);
        v[0] = v[0].wrapping_add(m[sg[1]] ^ CST[sg[0]]).wrapping_add(v[4]);
        v[12] ^= v[0]; v[12] = rot32(v[12], 8);
        v[8] = v[8].wrapping_add(v[12]);
        v[4] ^= v[8]; v[4] = rot32(v[4], 7);

        // G(1,5,9,13, sg[2],sg[3])
        v[1] = v[1].wrapping_add(m[sg[2]] ^ CST[sg[3]]).wrapping_add(v[5]);
        v[13] ^= v[1]; v[13] = rot32(v[13], 16);
        v[9] = v[9].wrapping_add(v[13]);
        v[5] ^= v[9]; v[5] = rot32(v[5], 12);
        v[1] = v[1].wrapping_add(m[sg[3]] ^ CST[sg[2]]).wrapping_add(v[5]);
        v[13] ^= v[1]; v[13] = rot32(v[13], 8);
        v[9] = v[9].wrapping_add(v[13]);
        v[5] ^= v[9]; v[5] = rot32(v[5], 7);

        // G(2,6,10,14, sg[4],sg[5])
        v[2] = v[2].wrapping_add(m[sg[4]] ^ CST[sg[5]]).wrapping_add(v[6]);
        v[14] ^= v[2]; v[14] = rot32(v[14], 16);
        v[10] = v[10].wrapping_add(v[14]);
        v[6] ^= v[10]; v[6] = rot32(v[6], 12);
        v[2] = v[2].wrapping_add(m[sg[5]] ^ CST[sg[4]]).wrapping_add(v[6]);
        v[14] ^= v[2]; v[14] = rot32(v[14], 8);
        v[10] = v[10].wrapping_add(v[14]);
        v[6] ^= v[10]; v[6] = rot32(v[6], 7);

        // G(3,7,11,15, sg[6],sg[7])
        v[3] = v[3].wrapping_add(m[sg[6]] ^ CST[sg[7]]).wrapping_add(v[7]);
        v[15] ^= v[3]; v[15] = rot32(v[15], 16);
        v[11] = v[11].wrapping_add(v[15]);
        v[7] ^= v[11]; v[7] = rot32(v[7], 12);
        v[3] = v[3].wrapping_add(m[sg[7]] ^ CST[sg[6]]).wrapping_add(v[7]);
        v[15] ^= v[3]; v[15] = rot32(v[15], 8);
        v[11] = v[11].wrapping_add(v[15]);
        v[7] ^= v[11]; v[7] = rot32(v[7], 7);

        // Diagonal step
        // G(0,5,10,15, sg[8],sg[9])
        v[0] = v[0].wrapping_add(m[sg[8]] ^ CST[sg[9]]).wrapping_add(v[5]);
        v[15] ^= v[0]; v[15] = rot32(v[15], 16);
        v[10] = v[10].wrapping_add(v[15]);
        v[5] ^= v[10]; v[5] = rot32(v[5], 12);
        v[0] = v[0].wrapping_add(m[sg[9]] ^ CST[sg[8]]).wrapping_add(v[5]);
        v[15] ^= v[0]; v[15] = rot32(v[15], 8);
        v[10] = v[10].wrapping_add(v[15]);
        v[5] ^= v[10]; v[5] = rot32(v[5], 7);

        // G(1,6,11,12, sg[10],sg[11])
        v[1] = v[1].wrapping_add(m[sg[10]] ^ CST[sg[11]]).wrapping_add(v[6]);
        v[12] ^= v[1]; v[12] = rot32(v[12], 16);
        v[11] = v[11].wrapping_add(v[12]);
        v[6] ^= v[11]; v[6] = rot32(v[6], 12);
        v[1] = v[1].wrapping_add(m[sg[11]] ^ CST[sg[10]]).wrapping_add(v[6]);
        v[12] ^= v[1]; v[12] = rot32(v[12], 8);
        v[11] = v[11].wrapping_add(v[12]);
        v[6] ^= v[11]; v[6] = rot32(v[6], 7);

        // G(2,7,8,13, sg[12],sg[13])
        v[2] = v[2].wrapping_add(m[sg[12]] ^ CST[sg[13]]).wrapping_add(v[7]);
        v[13] ^= v[2]; v[13] = rot32(v[13], 16);
        v[8] = v[8].wrapping_add(v[13]);
        v[7] ^= v[8]; v[7] = rot32(v[7], 12);
        v[2] = v[2].wrapping_add(m[sg[13]] ^ CST[sg[12]]).wrapping_add(v[7]);
        v[13] ^= v[2]; v[13] = rot32(v[13], 8);
        v[8] = v[8].wrapping_add(v[13]);
        v[7] ^= v[8]; v[7] = rot32(v[7], 7);

        // G(3,4,9,14, sg[14],sg[15])
        v[3] = v[3].wrapping_add(m[sg[14]] ^ CST[sg[15]]).wrapping_add(v[4]);
        v[14] ^= v[3]; v[14] = rot32(v[14], 16);
        v[9] = v[9].wrapping_add(v[14]);
        v[4] ^= v[9]; v[4] = rot32(v[4], 12);
        v[3] = v[3].wrapping_add(m[sg[15]] ^ CST[sg[14]]).wrapping_add(v[4]);
        v[14] ^= v[3]; v[14] = rot32(v[14], 8);
        v[9] = v[9].wrapping_add(v[14]);
        v[4] ^= v[9]; v[4] = rot32(v[4], 7);
    }

    for i in 0..8 { v[i] ^= v[i + 8]; }
    for i in 0..4 { v[i] ^= s.s[i]; v[i+4] ^= s.s[i]; }
    for i in 0..8 { s.h[i] ^= v[i]; }
}

pub fn blake256_update(s: &mut BlakeState256, data: &[u8], mut datalen: u64) {
    let mut offset = 0usize;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && ((datalen >> 3) & 0x3F) >= fill as u64 {
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

pub fn blake256_final(s: &mut BlakeState256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let zo: u8 = 0x01;
    let oo: u8 = 0x81;
    let lo = s.t[0].wrapping_add(s.buflen as u32);
    let mut hi = s.t[1];
    if lo < s.buflen as u32 { hi = hi.wrapping_add(1); }
    u32to8(&mut msglen[0..], hi);
    u32to8(&mut msglen[4..], lo);

    if s.buflen == 440 {
        s.t[0] = s.t[0].wrapping_sub(8);
        blake256_update(s, &[oo], 8);
    } else {
        if s.buflen < 440 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((440 - s.buflen) as u32);
            blake256_update(s, &PADDING[..(440 - s.buflen) as usize / 8], (440 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((512 - s.buflen) as u32);
            blake256_update(s, &PADDING[..(512 - s.buflen) as usize / 8], (512 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &PADDING[1..1 + 440 / 8], 440);
            s.nullt = 1;
        }
        blake256_update(s, &[zo], 8);
        s.t[0] = s.t[0].wrapping_sub(8);
    }
    s.t[0] = s.t[0].wrapping_sub(64);
    blake256_update(s, &msglen, 64);

    for i in 0..8 {
        u32to8(&mut digest[4 * i..], s.h[i]);
    }
}

pub fn blake256(out: &mut [u8], input: &[u8], inlen: u64) -> i32 {
    let mut s = BlakeState256 {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64],
    };
    blake256_init(&mut s);
    blake256_update(&mut s, input, inlen.wrapping_mul(8));
    blake256_final(&mut s, out);
    0
}

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&input[..inlen]);
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut i: usize = 0;
    let mut off = 0usize;

    while (i + 1) * SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += SPX_BLAKE256_OUTPUT_BYTES;
        i += 1;
    }
    if outlen > i * SPX_BLAKE256_OUTPUT_BYTES {
        u32_to_bytes(&mut inbuf[inlen..], i as u32);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        out[off..off + (outlen - i * SPX_BLAKE256_OUTPUT_BYTES)]
            .copy_from_slice(&outbuf[..outlen - i * SPX_BLAKE256_OUTPUT_BYTES]);
    }
}
