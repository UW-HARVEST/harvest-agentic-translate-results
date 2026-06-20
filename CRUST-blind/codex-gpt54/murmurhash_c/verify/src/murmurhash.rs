pub const MURMURHASH_VERSION: &str = "0.2.0";
pub const MURMURHASH_HAS_HTOLE32: usize = 1;
pub fn htole32(v: u32) -> u32 {
    u32::from_le(v)
}
pub fn murmurhash(key: &[u8], seed: u32) -> u32 {
    const C1: u32 = 0xcc9e2d51;
    const C2: u32 = 0x1b873593;
    const R1: u32 = 15;
    const R2: u32 = 13;
    const M: u32 = 5;
    const N: u32 = 0xe6546b64;

    let mut h = seed;

    let mut chunks = key.chunks_exact(4);
    for chunk in &mut chunks {
        let mut k = htole32(u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        k = k.wrapping_mul(C1);
        k = k.rotate_left(R1);
        k = k.wrapping_mul(C2);

        h ^= k;
        h = h.rotate_left(R2);
        h = h.wrapping_mul(M).wrapping_add(N);
    }

    let tail = chunks.remainder();
    let mut k = 0u32;

    match tail.len() {
        3 => {
            k ^= (tail[2] as u32) << 16;
            k ^= (tail[1] as u32) << 8;
            k ^= tail[0] as u32;
        }
        2 => {
            k ^= (tail[1] as u32) << 8;
            k ^= tail[0] as u32;
        }
        1 => {
            k ^= tail[0] as u32;
        }
        _ => {}
    }

    if !tail.is_empty() {
        k = k.wrapping_mul(C1);
        k = k.rotate_left(R1);
        k = k.wrapping_mul(C2);
        h ^= k;
    }

    h ^= key.len() as u32;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    h
}
