// Re-implement confirm_result; pure standalone module.

const SIG_SIZE_BYTES: usize = 16;

const SECRET_KEY: [u8; 16] = [
    86, 93, 1, 209, 112, 176, 13, 40, 168, 223, 25, 22, 134, 58, 21, 211,
];

#[inline]
fn rotl(x: u64, b: u32) -> u64 {
    (x << b) | (x >> (64 - b))
}

fn sipround(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotl(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotl(*v0, 32);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotl(*v3, 16);
    *v3 ^= *v2;
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotl(*v3, 21);
    *v3 ^= *v0;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotl(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotl(*v2, 32);
}

fn u8to64_le(p: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    let n = p.len().min(8);
    buf[..n].copy_from_slice(&p[..n]);
    u64::from_le_bytes(buf)
}

fn siphash_compute(data: &[u8]) -> [u8; 16] {
    let k0 = u8to64_le(&SECRET_KEY[0..8]);
    let k1 = u8to64_le(&SECRET_KEY[8..16]);
    let mut v0 = 0x736f6d6570736575u64 ^ k0;
    let mut v1 = 0x646f72616e646f6du64 ^ k1;
    let mut v2 = 0x6c7967656e657261u64 ^ k0;
    let mut v3 = 0x7465646279746573u64 ^ k1;
    v1 ^= 0xee;

    let inlen = data.len() as u64;
    let mut idx = 0usize;
    while idx + 8 <= data.len() {
        let m = u8to64_le(&data[idx..idx + 8]);
        v3 ^= m;
        for _ in 0..2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= m;
        idx += 8;
    }

    let left = data.len() - idx;
    let mut b: u64 = inlen << 56;
    for i in 0..left {
        b |= (data[idx + i] as u64) << (8 * i);
    }

    v3 ^= b;
    for _ in 0..2 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= b;
    v2 ^= 0xee;
    for _ in 0..4 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    let bb = v0 ^ v1 ^ v2 ^ v3;

    let mut out = [0u8; 16];
    out[0..8].copy_from_slice(&bb.to_le_bytes());

    v1 ^= 0xdd;
    for _ in 0..4 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    let bb = v0 ^ v1 ^ v2 ^ v3;
    out[8..16].copy_from_slice(&bb.to_le_bytes());
    out
}

pub fn confirm_result(f_sig: &[u8], constant: u8, out: &mut [u8]) {
    let mut data = Vec::with_capacity(SIG_SIZE_BYTES + 1);
    data.extend_from_slice(&f_sig[..SIG_SIZE_BYTES]);
    data.push(constant);
    let sig = siphash_compute(&data);
    let n = SIG_SIZE_BYTES.min(out.len());
    out[..n].copy_from_slice(&sig[..n]);
}
