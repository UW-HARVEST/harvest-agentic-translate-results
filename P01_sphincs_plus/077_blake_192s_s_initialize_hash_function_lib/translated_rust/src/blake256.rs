pub const BLAKE256_CST: [u32; 16] = [
    0x243F6A88, 0x85A308D3, 0x13198A2E, 0x03707344,
    0xA4093822, 0x299F31D0, 0x082EFA98, 0xEC4E6C89,
    0x452821E6, 0x38D01377, 0xBE5466CF, 0x34E90C6C,
    0xC0AC29B7, 0xC97C50DD, 0x3F84D5B5, 0xB5470917,
];

pub const BLAKE256_PADDING: [u8; 64] = {
    let mut p = [0u8; 64];
    p[0] = 0x80;
    p
};

pub struct BlakeState256 {
    pub h: [u32; 8],
    pub s: [u32; 4],
    pub t: [u32; 2],
    pub buflen: i32,
    pub nullt: i32,
    pub buf: [u8; 64],
}

fn u8to32(p: &[u8]) -> u32 {
    (p[0] as u32) << 24 | (p[1] as u32) << 16 | (p[2] as u32) << 8 | p[3] as u32
}

fn u32to8(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

// The 14 round permutations for BLAKE-256
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

fn rot32(x: u32, n: u32) -> u32 {
    (x << (32 - n)) | (x >> n)
}

pub fn blake256_compress(s: &mut BlakeState256, block: &[u8]) {
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = u8to32(&block[i * 4..]);
    }

    let mut v = [0u32; 16];
    v[0] = s.h[0]; v[1] = s.h[1]; v[2] = s.h[2]; v[3] = s.h[3];
    v[4] = s.h[4]; v[5] = s.h[5]; v[6] = s.h[6]; v[7] = s.h[7];
    v[8] = s.s[0] ^ BLAKE256_CST[0];
    v[9] = s.s[1] ^ BLAKE256_CST[1];
    v[10] = s.s[2] ^ BLAKE256_CST[2];
    v[11] = s.s[3] ^ BLAKE256_CST[3];
    v[12] = BLAKE256_CST[4];
    v[13] = BLAKE256_CST[5];
    v[14] = BLAKE256_CST[6];
    v[15] = BLAKE256_CST[7];

    if s.nullt == 0 {
        v[12] ^= s.t[0];
        v[13] ^= s.t[0];
        v[14] ^= s.t[1];
        v[15] ^= s.t[1];
    }

    macro_rules! g256 {
        ($a:expr, $b:expr, $c:expr, $d:expr, $mi:expr, $ci:expr, $mj:expr, $cj:expr) => {
            v[$a] = v[$a].wrapping_add(m[$mi] ^ BLAKE256_CST[$ci]);
            v[$a] = v[$a].wrapping_add(v[$b]);
            v[$d] ^= v[$a];
            v[$d] = rot32(v[$d], 16);
            v[$c] = v[$c].wrapping_add(v[$d]);
            v[$b] ^= v[$c];
            v[$b] = rot32(v[$b], 12);
            v[$a] = v[$a].wrapping_add(m[$mj] ^ BLAKE256_CST[$cj]);
            v[$a] = v[$a].wrapping_add(v[$b]);
            v[$d] ^= v[$a];
            v[$d] = rot32(v[$d], 8);
            v[$c] = v[$c].wrapping_add(v[$d]);
            v[$b] ^= v[$c];
            v[$b] = rot32(v[$b], 7);
        };
    }

    for i in 0..14 {
        let s_ = &SIGMA[i];
        g256!(0, 4, 8, 12, s_[0], s_[1], s_[1], s_[0]);
        g256!(1, 5, 9, 13, s_[2], s_[3], s_[3], s_[2]);
        g256!(2, 6, 10, 14, s_[4], s_[5], s_[5], s_[4]);
        g256!(3, 7, 11, 15, s_[6], s_[7], s_[7], s_[6]);
        g256!(0, 5, 10, 15, s_[8], s_[9], s_[9], s_[8]);
        g256!(1, 6, 11, 12, s_[10], s_[11], s_[11], s_[10]);
        g256!(2, 7, 8, 13, s_[12], s_[13], s_[13], s_[12]);
        g256!(3, 4, 9, 14, s_[14], s_[15], s_[15], s_[14]);
    }

    for i in 0..8 {
        v[i] ^= v[i + 8];
    }
    for i in 0..4 {
        v[i] ^= s.s[i];
        v[i + 4] ^= s.s[i];
    }
    for i in 0..8 {
        s.h[i] ^= v[i];
    }
}

pub fn blake256_init(s: &mut BlakeState256) {
    s.h[0] = 0x6A09E667; s.h[1] = 0xBB67AE85;
    s.h[2] = 0x3C6EF372; s.h[3] = 0xA54FF53A;
    s.h[4] = 0x510E527F; s.h[5] = 0x9B05688C;
    s.h[6] = 0x1F83D9AB; s.h[7] = 0x5BE0CD19;
    s.t = [0; 2]; s.buflen = 0; s.nullt = 0;
    s.s = [0; 4];
}

pub fn blake256_update(s: &mut BlakeState256, data: &[u8], datalen: u64) {
    let mut data = data;
    let mut datalen = datalen;
    let mut left = (s.buflen >> 3) as usize;
    let fill = 64 - left;

    if left != 0 && ((datalen >> 3) & 0x3F) as usize >= fill {
        s.buf[left..left + fill].copy_from_slice(&data[..fill]);
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 { s.t[1] = s.t[1].wrapping_add(1); }
        let buf_copy = s.buf;
        blake256_compress(s, &buf_copy);
        data = &data[fill..];
        datalen -= (fill as u64) << 3;
        left = 0;
    }

    while datalen >= 512 {
        s.t[0] = s.t[0].wrapping_add(512);
        if s.t[0] == 0 { s.t[1] = s.t[1].wrapping_add(1); }
        blake256_compress(s, &data[..64]);
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

pub fn blake256_final(s: &mut BlakeState256, digest: &mut [u8]) {
    let mut msglen = [0u8; 8];
    let lo = s.t[0].wrapping_add(s.buflen as u32);
    let mut hi = s.t[1];
    if lo < s.buflen as u32 { hi = hi.wrapping_add(1); }
    u32to8(&mut msglen[0..4], hi);
    u32to8(&mut msglen[4..8], lo);

    if s.buflen == 440 {
        s.t[0] = s.t[0].wrapping_sub(8);
        let oo: u8 = 0x81;
        blake256_update(s, &[oo], 8);
    } else {
        if s.buflen < 440 {
            if s.buflen == 0 { s.nullt = 1; }
            s.t[0] = s.t[0].wrapping_sub((440 - s.buflen) as u32);
            blake256_update(s, &BLAKE256_PADDING[..(440 - s.buflen) as usize / 8], (440 - s.buflen) as u64);
        } else {
            s.t[0] = s.t[0].wrapping_sub((512 - s.buflen) as u32);
            blake256_update(s, &BLAKE256_PADDING[..(512 - s.buflen) as usize / 8], (512 - s.buflen) as u64);
            s.t[0] = s.t[0].wrapping_sub(440);
            blake256_update(s, &BLAKE256_PADDING[1..1 + 440 / 8], 440);
            s.nullt = 1;
        }
        let zo: u8 = 0x01;
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
    let mut s = BlakeState256 {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64],
    };
    blake256_init(&mut s);
    blake256_update(&mut s, inp, inlen.wrapping_mul(8));
    blake256_final(&mut s, out);
    0
}

pub fn blake256_mgf1(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);
    let mut outbuf = [0u8; crate::params::SPX_BLAKE256_OUTPUT_BYTES];
    let mut i: usize = 0;

    while (i + 1) * crate::params::SPX_BLAKE256_OUTPUT_BYTES <= outlen {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        blake256(&mut out[i * crate::params::SPX_BLAKE256_OUTPUT_BYTES..], &inbuf, (inlen + 4) as u64);
        i += 1;
    }
    if outlen > i * crate::params::SPX_BLAKE256_OUTPUT_BYTES {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..inlen + 4], i as u32);
        blake256(&mut outbuf, &inbuf, (inlen + 4) as u64);
        let rem = outlen - i * crate::params::SPX_BLAKE256_OUTPUT_BYTES;
        out[i * crate::params::SPX_BLAKE256_OUTPUT_BYTES..i * crate::params::SPX_BLAKE256_OUTPUT_BYTES + rem]
            .copy_from_slice(&outbuf[..rem]);
    }
}
