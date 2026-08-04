pub const MURMURHASH_VERSION: &str = "0.2.0";
pub const MURMURHASH_HAS_HTOLE32: usize = 1;

pub fn htole32(v: u32) -> u32 {
    // On little-endian (and conditional compile in C), this is a no-op.
    // On big-endian, swap bytes. Match the C semantics that swaps bytes
    // when the host is big-endian. Use Rust's to_le() which is correct
    // on both: it converts a value from native to little-endian.
    v.to_le()
}

pub fn murmurhash(key: &[u8], seed: u32) -> u32 {
    let c1: u32 = 0xcc9e2d51;
    let c2: u32 = 0x1b873593;
    let r1: u32 = 15;
    let r2: u32 = 13;
    let m: u32 = 5;
    let n: u32 = 0xe6546b64;

    let len: u32 = key.len() as u32;
    let mut h: u32 = seed;
    let l: usize = (len / 4) as usize; // number of 4-byte chunks

    // Process each 4-byte chunk
    for i in 0..l {
        let base = i * 4;
        let mut k = (key[base] as u32)
            | ((key[base + 1] as u32) << 8)
            | ((key[base + 2] as u32) << 16)
            | ((key[base + 3] as u32) << 24);

        k = k.wrapping_mul(c1);
        k = (k << r1) | (k >> (32 - r1));
        k = k.wrapping_mul(c2);

        h ^= k;
        h = (h << r2) | (h >> (32 - r2));
        h = h.wrapping_mul(m).wrapping_add(n);
    }

    // Tail
    let tail_start = l * 4;
    let mut k: u32 = 0;
    let rem = (len & 3) as usize;

    if rem == 3 {
        k ^= (key[tail_start + 2] as u32) << 16;
        k ^= (key[tail_start + 1] as u32) << 8;
        k ^= key[tail_start] as u32;
        k = k.wrapping_mul(c1);
        k = (k << r1) | (k >> (32 - r1));
        k = k.wrapping_mul(c2);
        h ^= k;
    } else if rem == 2 {
        k ^= (key[tail_start + 1] as u32) << 8;
        k ^= key[tail_start] as u32;
        k = k.wrapping_mul(c1);
        k = (k << r1) | (k >> (32 - r1));
        k = k.wrapping_mul(c2);
        h ^= k;
    } else if rem == 1 {
        k ^= key[tail_start] as u32;
        k = k.wrapping_mul(c1);
        k = (k << r1) | (k >> (32 - r1));
        k = k.wrapping_mul(c2);
        h ^= k;
    }

    h ^= len;

    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;

    h
}
