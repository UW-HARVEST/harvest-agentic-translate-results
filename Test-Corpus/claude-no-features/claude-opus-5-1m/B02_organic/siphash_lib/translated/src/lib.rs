use std::ffi::c_int;
use std::io::Write;

#[inline(always)]
fn sip_round(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = v1.rotate_left(13);
    *v1 ^= *v0;
    *v0 = v0.rotate_left(32);
    *v2 = v2.wrapping_add(*v3);
    *v3 = v3.rotate_left(16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = v1.rotate_left(17);
    *v1 ^= *v2;
    *v2 = v2.rotate_left(32);
    *v0 = v0.wrapping_add(*v3);
    *v3 = v3.rotate_left(21);
    *v3 ^= *v0;
}

fn stbds_siphash_bytes(p: &[u8], seed: u64) -> u64 {
    let len: u64 = p.len() as u64;

    // v0 = ((((size_t)0x736f6d65 << 16) << 16) + 0x70736575) ^ seed;
    let mut v0: u64 = ((0x736f6d65u64 << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: u64 = ((0x646f7261u64 << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: u64 = ((0x6c796765u64 << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: u64 = ((0x74656462u64 << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100u64 ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908u64 ^ !seed;
    v2 ^= 0x0706050403020100u64 ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908u64 ^ !seed;

    let mut i: usize = 0;
    let size_of_size_t: usize = 8;

    // Main loop: process full size_t chunks
    while i + size_of_size_t <= p.len() {
        let d = &p[i..];
        // C: data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        // In C, all of these are computed as `int` due to integer promotion.
        // If d[3] has the top bit set (d[3] >= 128), the result is a negative int,
        // which sign-extends when assigned to size_t. We replicate this exactly.
        let low: i32 = (d[0] as i32)
            | ((d[1] as i32) << 8)
            | ((d[2] as i32) << 16)
            | ((d[3] as i32) << 24);
        let high: i32 = (d[4] as i32)
            | ((d[5] as i32) << 8)
            | ((d[6] as i32) << 16)
            | ((d[7] as i32) << 24);
        // Sign-extend low (i32 -> u64 via signed cast)
        let mut data: u64 = low as i64 as u64;
        // (size_t)high << 16 << 16 - cast to size_t, then shift 32 left.
        // Sign extension happens but is shifted out completely (32-bit shift on 64-bit).
        let high_u: u64 = high as i64 as u64;
        data |= (high_u << 16) << 16;

        v3 ^= data;
        // 2 rounds
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i += size_of_size_t;
    }

    // data = len << ((sizeof(size_t) * 8) - 8); -> len << 56
    let mut data: u64 = len << 56;

    let remaining = p.len() - i;
    let d = &p[i..];

    // C switch with fall-through; cases 1, 2, 3 are positive ints (no sign-ext);
    // case 4 (d[3] << 24) can sign-extend.
    if remaining >= 7 {
        data |= ((d[6] as u64) << 24) << 24;
    }
    if remaining >= 6 {
        data |= ((d[5] as u64) << 20) << 20;
    }
    if remaining >= 5 {
        data |= ((d[4] as u64) << 16) << 16;
    }
    if remaining >= 4 {
        // C: data |= (d[3] << 24);
        // d[3] is unsigned char, promoted to int. (d[3] << 24) as int may have top bit set,
        // causing sign-extension when ORed into size_t.
        let v: i32 = (d[3] as i32) << 24;
        data |= v as i64 as u64;
    }
    if remaining >= 3 {
        // (d[2] << 16): max value 0xFF0000, positive int, no sign-extension
        data |= (d[2] as u64) << 16;
    }
    if remaining >= 2 {
        data |= (d[1] as u64) << 8;
    }
    if remaining >= 1 {
        data |= d[0] as u64;
    }
    // case 0: break (no-op)

    v3 ^= data;
    // 2 rounds
    for _ in 0..2 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    // 4 rounds
    for _ in 0..4 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

fn stbds_hash_bytes(p: &[u8], seed: u64) -> u64 {
    stbds_siphash_bytes(p, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn siphash(init: c_int) {
    let mut mem = [0u8; 64];
    let mut z: c_int = init;
    // for (i=0; i < 64; ++i,z++) mem[i] = z;
    for i in 0..64 {
        mem[i] = z as u8;
        z = z.wrapping_add(1);
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for i in 0..64usize {
        let hash = stbds_hash_bytes(&mem[..i], 0);
        let _ = out.write_all(b"  { ");
        for j in 0..8 {
            let byte: u8 = ((hash >> (j * 8)) & 255) as u8;
            // C printf("0x%02x, ", ...) - lowercase 2-digit hex with prefix and trailing ", "
            let _ = write!(out, "0x{:02x}, ", byte);
        }
        let _ = out.write_all(b" },\n");
    }
    let _ = out.flush();
}
