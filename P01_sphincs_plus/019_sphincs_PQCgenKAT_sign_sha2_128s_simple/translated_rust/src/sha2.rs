// SHA-256 implementation matching the C reference exactly

fn load_bigendian_32(x: &[u8]) -> u32 {
    (x[3] as u32) | ((x[2] as u32) << 8) | ((x[1] as u32) << 16) | ((x[0] as u32) << 24)
}

fn load_bigendian_64(x: &[u8]) -> u64 {
    (x[7] as u64) | ((x[6] as u64) << 8) | ((x[5] as u64) << 16) | ((x[4] as u64) << 24)
        | ((x[3] as u64) << 32) | ((x[2] as u64) << 40) | ((x[1] as u64) << 48) | ((x[0] as u64) << 56)
}

fn store_bigendian_32(x: &mut [u8], mut u: u64) {
    x[3] = u as u8; u >>= 8;
    x[2] = u as u8; u >>= 8;
    x[1] = u as u8; u >>= 8;
    x[0] = u as u8;
}

fn store_bigendian_64(x: &mut [u8], mut u: u64) {
    x[7] = u as u8; u >>= 8;
    x[6] = u as u8; u >>= 8;
    x[5] = u as u8; u >>= 8;
    x[4] = u as u8; u >>= 8;
    x[3] = u as u8; u >>= 8;
    x[2] = u as u8; u >>= 8;
    x[1] = u as u8; u >>= 8;
    x[0] = u as u8;
}

fn crypto_hashblocks_sha256(statebytes: &mut [u8], data: &[u8], mut inlen: usize) -> usize {
    let mut state = [0u32; 8];
    for i in 0..8 {
        state[i] = load_bigendian_32(&statebytes[4 * i..]);
    }
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) =
        (state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7]);

    let mut offset = 0usize;
    while inlen >= 64 {
        let inp = &data[offset..];
        let mut w = [0u32; 16];
        for i in 0..16 {
            w[i] = load_bigendian_32(&inp[4 * i..]);
        }

        macro_rules! ch { ($x:expr,$y:expr,$z:expr) => { ($x & $y) ^ (!$x & $z) } }
        macro_rules! maj { ($x:expr,$y:expr,$z:expr) => { ($x & $y) ^ ($x & $z) ^ ($y & $z) } }
        macro_rules! sigma0 { ($x:expr) => { $x.rotate_right(2) ^ $x.rotate_right(13) ^ $x.rotate_right(22) } }
        macro_rules! sigma1 { ($x:expr) => { $x.rotate_right(6) ^ $x.rotate_right(11) ^ $x.rotate_right(25) } }
        macro_rules! sig0 { ($x:expr) => { $x.rotate_right(7) ^ $x.rotate_right(18) ^ ($x >> 3) } }
        macro_rules! sig1 { ($x:expr) => { $x.rotate_right(17) ^ $x.rotate_right(19) ^ ($x >> 10) } }

        macro_rules! f32 {
            ($w:expr, $k:expr) => {
                let t1 = h.wrapping_add(sigma1!(e)).wrapping_add(ch!(e,f,g)).wrapping_add($k).wrapping_add($w);
                let t2 = sigma0!(a).wrapping_add(maj!(a,b,c));
                h = g; g = f; f = e; e = d.wrapping_add(t1); d = c; c = b; b = a; a = t1.wrapping_add(t2);
            }
        }

        macro_rules! expand {
            () => {
                w[0] = sig1!(w[14]).wrapping_add(w[9]).wrapping_add(sig0!(w[1])).wrapping_add(w[0]);
                w[1] = sig1!(w[15]).wrapping_add(w[10]).wrapping_add(sig0!(w[2])).wrapping_add(w[1]);
                w[2] = sig1!(w[0]).wrapping_add(w[11]).wrapping_add(sig0!(w[3])).wrapping_add(w[2]);
                w[3] = sig1!(w[1]).wrapping_add(w[12]).wrapping_add(sig0!(w[4])).wrapping_add(w[3]);
                w[4] = sig1!(w[2]).wrapping_add(w[13]).wrapping_add(sig0!(w[5])).wrapping_add(w[4]);
                w[5] = sig1!(w[3]).wrapping_add(w[14]).wrapping_add(sig0!(w[6])).wrapping_add(w[5]);
                w[6] = sig1!(w[4]).wrapping_add(w[15]).wrapping_add(sig0!(w[7])).wrapping_add(w[6]);
                w[7] = sig1!(w[5]).wrapping_add(w[0]).wrapping_add(sig0!(w[8])).wrapping_add(w[7]);
                w[8] = sig1!(w[6]).wrapping_add(w[1]).wrapping_add(sig0!(w[9])).wrapping_add(w[8]);
                w[9] = sig1!(w[7]).wrapping_add(w[2]).wrapping_add(sig0!(w[10])).wrapping_add(w[9]);
                w[10] = sig1!(w[8]).wrapping_add(w[3]).wrapping_add(sig0!(w[11])).wrapping_add(w[10]);
                w[11] = sig1!(w[9]).wrapping_add(w[4]).wrapping_add(sig0!(w[12])).wrapping_add(w[11]);
                w[12] = sig1!(w[10]).wrapping_add(w[5]).wrapping_add(sig0!(w[13])).wrapping_add(w[12]);
                w[13] = sig1!(w[11]).wrapping_add(w[6]).wrapping_add(sig0!(w[14])).wrapping_add(w[13]);
                w[14] = sig1!(w[12]).wrapping_add(w[7]).wrapping_add(sig0!(w[15])).wrapping_add(w[14]);
                w[15] = sig1!(w[13]).wrapping_add(w[8]).wrapping_add(sig0!(w[0])).wrapping_add(w[15]);
            }
        }

        f32!(w[0], 0x428a2f98); f32!(w[1], 0x71374491); f32!(w[2], 0xb5c0fbcf); f32!(w[3], 0xe9b5dba5);
        f32!(w[4], 0x3956c25b); f32!(w[5], 0x59f111f1); f32!(w[6], 0x923f82a4); f32!(w[7], 0xab1c5ed5);
        f32!(w[8], 0xd807aa98); f32!(w[9], 0x12835b01); f32!(w[10], 0x243185be); f32!(w[11], 0x550c7dc3);
        f32!(w[12], 0x72be5d74); f32!(w[13], 0x80deb1fe); f32!(w[14], 0x9bdc06a7); f32!(w[15], 0xc19bf174);
        expand!();
        f32!(w[0], 0xe49b69c1); f32!(w[1], 0xefbe4786); f32!(w[2], 0x0fc19dc6); f32!(w[3], 0x240ca1cc);
        f32!(w[4], 0x2de92c6f); f32!(w[5], 0x4a7484aa); f32!(w[6], 0x5cb0a9dc); f32!(w[7], 0x76f988da);
        f32!(w[8], 0x983e5152); f32!(w[9], 0xa831c66d); f32!(w[10], 0xb00327c8); f32!(w[11], 0xbf597fc7);
        f32!(w[12], 0xc6e00bf3); f32!(w[13], 0xd5a79147); f32!(w[14], 0x06ca6351); f32!(w[15], 0x14292967);
        expand!();
        f32!(w[0], 0x27b70a85); f32!(w[1], 0x2e1b2138); f32!(w[2], 0x4d2c6dfc); f32!(w[3], 0x53380d13);
        f32!(w[4], 0x650a7354); f32!(w[5], 0x766a0abb); f32!(w[6], 0x81c2c92e); f32!(w[7], 0x92722c85);
        f32!(w[8], 0xa2bfe8a1); f32!(w[9], 0xa81a664b); f32!(w[10], 0xc24b8b70); f32!(w[11], 0xc76c51a3);
        f32!(w[12], 0xd192e819); f32!(w[13], 0xd6990624); f32!(w[14], 0xf40e3585); f32!(w[15], 0x106aa070);
        expand!();
        f32!(w[0], 0x19a4c116); f32!(w[1], 0x1e376c08); f32!(w[2], 0x2748774c); f32!(w[3], 0x34b0bcb5);
        f32!(w[4], 0x391c0cb3); f32!(w[5], 0x4ed8aa4a); f32!(w[6], 0x5b9cca4f); f32!(w[7], 0x682e6ff3);
        f32!(w[8], 0x748f82ee); f32!(w[9], 0x78a5636f); f32!(w[10], 0x84c87814); f32!(w[11], 0x8cc70208);
        f32!(w[12], 0x90befffa); f32!(w[13], 0xa4506ceb); f32!(w[14], 0xbef9a3f7); f32!(w[15], 0xc67178f2);

        a = a.wrapping_add(state[0]); b = b.wrapping_add(state[1]);
        c = c.wrapping_add(state[2]); d = d.wrapping_add(state[3]);
        e = e.wrapping_add(state[4]); f = f.wrapping_add(state[5]);
        g = g.wrapping_add(state[6]); h = h.wrapping_add(state[7]);
        state[0] = a; state[1] = b; state[2] = c; state[3] = d;
        state[4] = e; state[5] = f; state[6] = g; state[7] = h;

        offset += 64;
        inlen -= 64;
    }

    for i in 0..8 {
        store_bigendian_32(&mut statebytes[4 * i..], state[i] as u64);
    }
    inlen
}

static IV_256: [u8; 32] = [
    0x6a, 0x09, 0xe6, 0x67, 0xbb, 0x67, 0xae, 0x85,
    0x3c, 0x6e, 0xf3, 0x72, 0xa5, 0x4f, 0xf5, 0x3a,
    0x51, 0x0e, 0x52, 0x7f, 0x9b, 0x05, 0x68, 0x8c,
    0x1f, 0x83, 0xd9, 0xab, 0x5b, 0xe0, 0xcd, 0x19,
];

pub fn sha256_inc_init(state: &mut [u8]) {
    state[..32].copy_from_slice(&IV_256);
    for i in 32..40 { state[i] = 0; }
}

pub fn sha256_inc_blocks(state: &mut [u8], data: &[u8], inblocks: usize) {
    let bytes = load_bigendian_64(&state[32..]);
    crypto_hashblocks_sha256(state, data, 64 * inblocks);
    let bytes = bytes + (64 * inblocks) as u64;
    store_bigendian_64(&mut state[32..], bytes);
}

pub fn sha256_inc_finalize(out: &mut [u8], state: &mut [u8], data: &[u8], inlen: usize) {
    let mut padded = [0u8; 128];
    let bytes = load_bigendian_64(&state[32..]) + inlen as u64;

    crypto_hashblocks_sha256(state, data, inlen);
    let remaining = inlen & 63;
    let start = inlen - remaining;

    for i in 0..remaining {
        padded[i] = data[start + i];
    }
    padded[remaining] = 0x80;

    if remaining < 56 {
        for i in (remaining + 1)..56 { padded[i] = 0; }
        padded[56] = (bytes >> 53) as u8;
        padded[57] = (bytes >> 45) as u8;
        padded[58] = (bytes >> 37) as u8;
        padded[59] = (bytes >> 29) as u8;
        padded[60] = (bytes >> 21) as u8;
        padded[61] = (bytes >> 13) as u8;
        padded[62] = (bytes >> 5) as u8;
        padded[63] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(state, &padded, 64);
    } else {
        for i in (remaining + 1)..120 { padded[i] = 0; }
        padded[120] = (bytes >> 53) as u8;
        padded[121] = (bytes >> 45) as u8;
        padded[122] = (bytes >> 37) as u8;
        padded[123] = (bytes >> 29) as u8;
        padded[124] = (bytes >> 21) as u8;
        padded[125] = (bytes >> 13) as u8;
        padded[126] = (bytes >> 5) as u8;
        padded[127] = (bytes << 3) as u8;
        crypto_hashblocks_sha256(state, &padded, 128);
    }

    out[..32].copy_from_slice(&state[..32]);
}

pub fn sha256(out: &mut [u8], data: &[u8], inlen: usize) {
    let mut state = [0u8; 40];
    sha256_inc_init(&mut state);
    sha256_inc_finalize(out, &mut state, data, inlen);
}

pub fn mgf1_256(out: &mut [u8], outlen: usize, inp: &[u8], inlen: usize) {
    let mut inbuf = vec![0u8; inlen + 4];
    let mut outbuf = [0u8; 32];
    inbuf[..inlen].copy_from_slice(&inp[..inlen]);

    let mut i = 0usize;
    while (i + 1) * 32 <= outlen {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256(&mut out[i * 32..], &inbuf, inlen + 4);
        i += 1;
    }
    if outlen > i * 32 {
        crate::utils::u32_to_bytes(&mut inbuf[inlen..], i as u32);
        sha256(&mut outbuf, &inbuf, inlen + 4);
        out[i * 32..outlen].copy_from_slice(&outbuf[..outlen - i * 32]);
    }
}
