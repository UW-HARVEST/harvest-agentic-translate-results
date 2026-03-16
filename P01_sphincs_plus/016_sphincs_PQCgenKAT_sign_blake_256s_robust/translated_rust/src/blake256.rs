use crate::utils::u32_to_bytes;

const CST256: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

const PADDING256: [u8; 64] = {
    let mut p = [0u8; 64];
    p[0] = 0x80;
    p
};

fn u8to32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}

fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

#[derive(Clone)]
pub struct Blake256State {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

fn blake256_rot(x: u32, n: u32) -> u32 {
    (x << (32 - n)) | (x >> n)
}

pub fn blake256_compress(state: &mut Blake256State, block: &[u8]) {
    let m: [u32; 16] = [
        u8to32(&block[0..]), u8to32(&block[4..]), u8to32(&block[8..]), u8to32(&block[12..]),
        u8to32(&block[16..]), u8to32(&block[20..]), u8to32(&block[24..]), u8to32(&block[28..]),
        u8to32(&block[32..]), u8to32(&block[36..]), u8to32(&block[40..]), u8to32(&block[44..]),
        u8to32(&block[48..]), u8to32(&block[52..]), u8to32(&block[56..]), u8to32(&block[60..]),
    ];

    let mut v = [0u32; 16];
    v[0] = state.h[0]; v[1] = state.h[1]; v[2] = state.h[2]; v[3] = state.h[3];
    v[4] = state.h[4]; v[5] = state.h[5]; v[6] = state.h[6]; v[7] = state.h[7];
    v[8] = state.s[0] ^ 0x243F6A88;
    v[9] = state.s[1] ^ 0x85A308D3;
    v[10] = state.s[2] ^ 0x13198A2E;
    v[11] = state.s[3] ^ 0x03707344;
    v[12] = 0xA4093822; v[13] = 0x299F31D0; v[14] = 0x082EFA98; v[15] = 0xEC4E6C89;

    if state.nullt == 0 {
        v[12] ^= state.t[0];
        v[13] ^= state.t[0];
        v[14] ^= state.t[1];
        v[15] ^= state.t[1];
    }

    // The BLAKE-256 sigma permutations
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
        // Repeat first 4
        [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15],
        [14,10,4,8,9,15,13,6,1,12,0,2,11,7,5,3],
        [11,8,12,0,5,2,15,13,10,14,3,6,7,1,9,4],
        [7,9,3,1,13,12,11,14,2,6,5,10,4,0,15,8],
    ];

    for round in 0..14 {
        let s = &SIGMA[round];
        // Column step
        // G(v, 0, 4, 8, 12, m[s[0]], m[s[1]])
        v[0] = v[0].wrapping_add(m[s[0]] ^ CST256[s[1]]).wrapping_add(v[4]);
        v[12] ^= v[0]; v[12] = blake256_rot(v[12], 16);
        v[8] = v[8].wrapping_add(v[12]); v[4] ^= v[8]; v[4] = blake256_rot(v[4], 12);
        v[0] = v[0].wrapping_add(m[s[1]] ^ CST256[s[0]]).wrapping_add(v[4]);
        v[12] ^= v[0]; v[12] = blake256_rot(v[12], 8);
        v[8] = v[8].wrapping_add(v[12]); v[4] ^= v[8]; v[4] = blake256_rot(v[4], 7);

        v[1] = v[1].wrapping_add(m[s[2]] ^ CST256[s[3]]).wrapping_add(v[5]);
        v[13] ^= v[1]; v[13] = blake256_rot(v[13], 16);
        v[9] = v[9].wrapping_add(v[13]); v[5] ^= v[9]; v[5] = blake256_rot(v[5], 12);
        v[1] = v[1].wrapping_add(m[s[3]] ^ CST256[s[2]]).wrapping_add(v[5]);
        v[13] ^= v[1]; v[13] = blake256_rot(v[13], 8);
        v[9] = v[9].wrapping_add(v[13]); v[5] ^= v[9]; v[5] = blake256_rot(v[5], 7);

        v[2] = v[2].wrapping_add(m[s[4]] ^ CST256[s[5]]).wrapping_add(v[6]);
        v[14] ^= v[2]; v[14] = blake256_rot(v[14], 16);
        v[10] = v[10].wrapping_add(v[14]); v[6] ^= v[10]; v[6] = blake256_rot(v[6], 12);
        v[2] = v[2].wrapping_add(m[s[5]] ^ CST256[s[4]]).wrapping_add(v[6]);
        v[14] ^= v[2]; v[14] = blake256_rot(v[14], 8);
        v[10] = v[10].wrapping_add(v[14]); v[6] ^= v[10]; v[6] = blake256_rot(v[6], 7);

        v[3] = v[3].wrapping_add(m[s[6]] ^ CST256[s[7]]).wrapping_add(v[7]);
        v[15] ^= v[3]; v[15] = blake256_rot(v[15], 16);
        v[11] = v[11].wrapping_add(v[15]); v[7] ^= v[11]; v[7] = blake256_rot(v[7], 12);
        v[3] = v[3].wrapping_add(m[s[7]] ^ CST256[s[6]]).wrapping_add(v[7]);
        v[15] ^= v[3]; v[15] = blake256_rot(v[15], 8);
        v[11] = v[11].wrapping_add(v[15]); v[7] ^= v[11]; v[7] = blake256_rot(v[7], 7);

        // Diagonal step
        v[0] = v[0].wrapping_add(m[s[8]] ^ CST256[s[9]]).wrapping_add(v[5]);
        v[15] ^= v[0]; v[15] = blake256_rot(v[15], 16);
        v[10] = v[10].wrapping_add(v[15]); v[5] ^= v[10]; v[5] = blake256_rot(v[5], 12);
        v[0] = v[0].wrapping_add(m[s[9]] ^ CST256[s[8]]).wrapping_add(v[5]);
        v[15] ^= v[0]; v[15] = blake256_rot(v[15], 8);
        v[10] = v[10].wrapping_add(v[15]); v[5] ^= v[10]; v[5] = blake256_rot(v[5], 7);

        v[1] = v[1].wrapping_add(m[s[10]] ^ CST256[s[11]]).wrapping_add(v[6]);
        v[12] ^= v[1]; v[12] = blake256_rot(v[12], 16);
        v[11] = v[11].wrapping_add(v[12]); v[6] ^= v[11]; v[6] = blake256_rot(v[6], 12);
        v[1] = v[1].wrapping_add(m[s[11]] ^ CST256[s[10]]).wrapping_add(v[6]);
        v[12] ^= v[1]; v[12] = blake256_rot(v[12], 8);
        v[11] = v[11].wrapping_add(v[12]); v[6] ^= v[11]; v[6] = blake256_rot(v[6], 7);

        v[2] = v[2].wrapping_add(m[s[12]] ^ CST256[s[13]]).wrapping_add(v[7]);
        v[13] ^= v[2]; v[13] = blake256_rot(v[13], 16);
        v[8] = v[8].wrapping_add(v[13]); v[7] ^= v[8]; v[7] = blake256_rot(v[7], 12);
        v[2] = v[2].wrapping_add(m[s[13]] ^ CST256[s[12]]).wrapping_add(v[7]);
        v[13] ^= v[2]; v[13] = blake256_rot(v[13], 8);
        v[8] = v[8].wrapping_add(v[13]); v[7] ^= v[8]; v[7] = blake256_rot(v[7], 7);

        v[3] = v[3].wrapping_add(m[s[14]] ^ CST256[s[15]]).wrapping_add(v[4]);
        v[14] ^= v[3]; v[14] = blake256_rot(v[14], 16);
        v[9] = v[9].wrapping_add(v[14]); v[4] ^= v[9]; v[4] = blake256_rot(v[4], 12);
        v[3] = v[3].wrapping_add(m[s[15]] ^ CST256[s[14]]).wrapping_add(v[4]);
        v[14] ^= v[3]; v[14] = blake256_rot(v[14], 8);
        v[9] = v[9].wrapping_add(v[14]); v[4] ^= v[9]; v[4] = blake256_rot(v[4], 7);
    }

    for i in 0..8 { v[i] ^= v[i + 8]; }
    for i in 0..4 { v[i] ^= state.s[i]; v[i+4] ^= state.s[i]; }
    for i in 0..8 { state.h[i] ^= v[i]; }
}

pub fn blake256_init(state: &mut Blake256State) {
    state.h = [
        0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
        0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
    ];
    state.t = [0, 0];
    state.buflen = 0;
    state.nullt = 0;
    state.s = [0, 0, 0, 0];
    state.buf = [0; 64];
}

pub fn blake256_update(state: &mut Blake256State, data: &[u8], datalen: u64) {
    let mut datalen = datalen;
    let mut data_off = 0usize;
    let left = (state.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && ((datalen >> 3) & 0x3F) >= fill as u64 {
        state.buf[left..left + fill].copy_from_slice(&data[data_off..data_off + fill]);
        state.t[0] = state.t[0].wrapping_add(512);
        if state.t[0] == 0 { state.t[1] = state.t[1].wrapping_add(1); }
        let buf_copy = state.buf;
        blake256_compress(state, &buf_copy);
        data_off += fill;
        datalen -= (fill as u64) << 3;
        // left = 0 implicitly
    }

    while datalen >= 512 {
        state.t[0] = state.t[0].wrapping_add(512);
        if state.t[0] == 0 { state.t[1] = state.t[1].wrapping_add(1); }
        blake256_compress(state, &data[data_off..]);
        data_off += 64;
        datalen -= 512;
    }

    if datalen > 0 {
        let new_left = if left != 0 && ((data[..].len() - data_off) > 0) { 0 } else { left };
        let bytes = (datalen >> 3) as usize;
        state.buf[new_left..new_left + bytes].copy_from_slice(&data[data_off..data_off + bytes]);
        state.buflen = ((new_left as i32) << 3) + datalen as i32;
    } else {
        state.buflen = 0;
    }
}

pub fn blake256_final(state: &mut Blake256State, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let lo = state.t[0].wrapping_add(state.buflen as u32);
    let mut hi = state.t[1];
    if lo < state.buflen as u32 { hi = hi.wrapping_add(1); }
    u32to8(&mut msglen[0..4], hi);
    u32to8(&mut msglen[4..8], lo);

    if state.buflen == 440 {
        state.t[0] = state.t[0].wrapping_sub(8);
        blake256_update(state, &[0x81], 8);
    } else {
        if state.buflen < 440 {
            if state.buflen == 0 { state.nullt = 1; }
            state.t[0] = state.t[0].wrapping_sub((440 - state.buflen) as u32);
            blake256_update(state, &PADDING256[..(440 - state.buflen) as usize / 8], (440 - state.buflen) as u64);
        } else {
            state.t[0] = state.t[0].wrapping_sub((512 - state.buflen) as u32);
            blake256_update(state, &PADDING256[..(512 - state.buflen) as usize / 8], (512 - state.buflen) as u64);
            state.t[0] = state.t[0].wrapping_sub(440);
            blake256_update(state, &PADDING256[1..1 + 440 / 8], 440);
            state.nullt = 1;
        }
        blake256_update(state, &[0x01], 8);
        state.t[0] = state.t[0].wrapping_sub(8);
    }
    state.t[0] = state.t[0].wrapping_sub(64);
    blake256_update(state, &msglen, 64);

    for i in 0..8 {
        u32to8(&mut digest[4 * i..4 * i + 4], state.h[i]);
    }
}

pub fn blake256(out: &mut [u8], inp: &[u8], inlen: u64) {
    let mut s = Blake256State {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64],
    };
    blake256_init(&mut s);
    blake256_update(&mut s, inp, inlen.wrapping_mul(8));
    blake256_final(&mut s, out);
}

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; 32];
    let mut i: u32 = 0;
    let mut off = 0usize;

    while (i as usize + 1) * 32 <= outlen {
        u32_to_bytes(&mut inbuf[inlen..inlen + 4], i);
        blake256(&mut out[off..], &inbuf, (inlen + 4) as u64);
        off += 32;
        i += 1;
    }
    if outlen > i as usize * 32 {
        u32_to_bytes(&mut inbuf[inlen..inlen + 4], i);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i as usize * 32;
        out[off..off + rem].copy_from_slice(&outbuf[..rem]);
    }
}
