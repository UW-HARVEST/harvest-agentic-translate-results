//! Translation of `lib/shake/src/fips202.c`.
//!
//! The Keccak-f[1600] permutation is implemented in the standard round-based
//! form; it is bit-for-bit identical to the unrolled C version. The
//! incremental sponge bookkeeping mirrors the C code exactly.

pub const SHAKE128_RATE: usize = 168;
pub const SHAKE256_RATE: usize = 136;
pub const SHA3_256_RATE: usize = 136;
pub const SHA3_512_RATE: usize = 72;

fn load64(x: &[u8]) -> u64 {
    let mut r = 0u64;
    for i in 0..8 {
        r |= (x[i] as u64) << (8 * i);
    }
    r
}

fn store64(x: &mut [u8], u: u64) {
    for i in 0..8 {
        x[i] = (u >> (8 * i)) as u8;
    }
}

const KECCAK_RC: [u64; 24] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
    0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
];

const KECCAK_ROTC: [u32; 24] = [
    1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44,
];

const KECCAK_PILN: [usize; 24] = [
    10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1,
];

fn keccak_f1600_state_permute(s: &mut [u64]) {
    for round in 0..24 {
        // Theta
        let mut bc = [0u64; 5];
        for i in 0..5 {
            bc[i] = s[i] ^ s[i + 5] ^ s[i + 10] ^ s[i + 15] ^ s[i + 20];
        }
        for i in 0..5 {
            let t = bc[(i + 4) % 5] ^ bc[(i + 1) % 5].rotate_left(1);
            let mut j = 0;
            while j < 25 {
                s[j + i] ^= t;
                j += 5;
            }
        }
        // Rho + Pi
        let mut t = s[1];
        for i in 0..24 {
            let j = KECCAK_PILN[i];
            let tmp = s[j];
            s[j] = t.rotate_left(KECCAK_ROTC[i]);
            t = tmp;
        }
        // Chi
        let mut j = 0;
        while j < 25 {
            let mut tmp = [0u64; 5];
            for i in 0..5 {
                tmp[i] = s[j + i];
            }
            for i in 0..5 {
                s[j + i] ^= (!tmp[(i + 1) % 5]) & tmp[(i + 2) % 5];
            }
            j += 5;
        }
        // Iota
        s[0] ^= KECCAK_RC[round];
    }
}

fn keccak_absorb(s: &mut [u64], r: usize, mut m: &[u8], mut mlen: usize, p: u8) {
    let mut t = [0u8; 200];
    for i in 0..25 {
        s[i] = 0;
    }
    while mlen >= r {
        for i in 0..r / 8 {
            s[i] ^= load64(&m[8 * i..]);
        }
        keccak_f1600_state_permute(s);
        mlen -= r;
        m = &m[r..];
    }
    for i in 0..r {
        t[i] = 0;
    }
    for i in 0..mlen {
        t[i] = m[i];
    }
    t[mlen] = p;
    t[r - 1] |= 128;
    for i in 0..r / 8 {
        s[i] ^= load64(&t[8 * i..]);
    }
}

fn keccak_squeezeblocks(h: &mut [u8], mut nblocks: usize, s: &mut [u64], r: usize) {
    let mut ho = 0usize;
    while nblocks > 0 {
        keccak_f1600_state_permute(s);
        for i in 0..(r >> 3) {
            store64(&mut h[ho + 8 * i..], s[i]);
        }
        ho += r;
        nblocks -= 1;
    }
}

fn keccak_inc_init(s_inc: &mut [u64]) {
    for i in 0..25 {
        s_inc[i] = 0;
    }
    s_inc[25] = 0;
}

fn keccak_inc_absorb(s_inc: &mut [u64], r: usize, mut m: &[u8], mut mlen: usize) {
    let r = r as u64;
    while mlen as u64 + s_inc[25] >= r {
        let cnt = (r - s_inc[25]) as usize;
        for i in 0..cnt {
            let pos = (s_inc[25] + i as u64) as usize;
            s_inc[pos >> 3] ^= (m[i] as u64) << (8 * (pos & 0x07));
        }
        mlen -= cnt;
        m = &m[cnt..];
        s_inc[25] = 0;
        keccak_f1600_state_permute(s_inc);
    }
    for i in 0..mlen {
        let pos = (s_inc[25] + i as u64) as usize;
        s_inc[pos >> 3] ^= (m[i] as u64) << (8 * (pos & 0x07));
    }
    s_inc[25] += mlen as u64;
}

fn keccak_inc_finalize(s_inc: &mut [u64], r: usize, p: u8) {
    let r = r as u64;
    let pos = s_inc[25] as usize;
    s_inc[pos >> 3] ^= (p as u64) << (8 * (pos & 0x07));
    let last = (r - 1) as usize;
    s_inc[last >> 3] ^= 128u64 << (8 * (last & 0x07));
    s_inc[25] = 0;
}

fn keccak_inc_squeeze(h: &mut [u8], mut outlen: usize, s_inc: &mut [u64], r: usize) {
    let mut ho = 0usize;
    let mut i = 0usize;
    while i < outlen && (i as u64) < s_inc[25] {
        let pos = (r as u64 - s_inc[25] + i as u64) as usize;
        h[ho + i] = (s_inc[pos >> 3] >> (8 * (pos & 0x07))) as u8;
        i += 1;
    }
    ho += i;
    outlen -= i;
    s_inc[25] -= i as u64;

    while outlen > 0 {
        keccak_f1600_state_permute(s_inc);
        let mut k = 0usize;
        while k < outlen && k < r {
            h[ho + k] = (s_inc[k >> 3] >> (8 * (k & 0x07))) as u8;
            k += 1;
        }
        ho += k;
        outlen -= k;
        s_inc[25] = (r - k) as u64;
    }
}

// ---- SHAKE-128 ----
pub fn shake128_absorb(s: &mut [u64], input: &[u8], inlen: usize) {
    keccak_absorb(s, SHAKE128_RATE, input, inlen, 0x1F);
}
pub fn shake128_squeezeblocks(output: &mut [u8], nblocks: usize, s: &mut [u64]) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE128_RATE);
}
pub fn shake128_inc_init(s: &mut [u64]) {
    keccak_inc_init(s);
}
pub fn shake128_inc_absorb(s: &mut [u64], input: &[u8], inlen: usize) {
    keccak_inc_absorb(s, SHAKE128_RATE, input, inlen);
}
pub fn shake128_inc_finalize(s: &mut [u64]) {
    keccak_inc_finalize(s, SHAKE128_RATE, 0x1F);
}
pub fn shake128_inc_squeeze(output: &mut [u8], outlen: usize, s: &mut [u64]) {
    keccak_inc_squeeze(output, outlen, s, SHAKE128_RATE);
}

// ---- SHAKE-256 ----
pub fn shake256_absorb(s: &mut [u64], input: &[u8], inlen: usize) {
    keccak_absorb(s, SHAKE256_RATE, input, inlen, 0x1F);
}
pub fn shake256_squeezeblocks(output: &mut [u8], nblocks: usize, s: &mut [u64]) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE256_RATE);
}
pub fn shake256_inc_init(s: &mut [u64]) {
    keccak_inc_init(s);
}
pub fn shake256_inc_absorb(s: &mut [u64], input: &[u8], inlen: usize) {
    keccak_inc_absorb(s, SHAKE256_RATE, input, inlen);
}
pub fn shake256_inc_finalize(s: &mut [u64]) {
    keccak_inc_finalize(s, SHAKE256_RATE, 0x1F);
}
pub fn shake256_inc_squeeze(output: &mut [u8], outlen: usize, s: &mut [u64]) {
    keccak_inc_squeeze(output, outlen, s, SHAKE256_RATE);
}

pub fn shake128(output: &mut [u8], mut outlen: usize, input: &[u8], inlen: usize) {
    let nblocks = outlen / SHAKE128_RATE;
    let mut t = [0u8; SHAKE128_RATE];
    let mut s = [0u64; 25];
    shake128_absorb(&mut s, input, inlen);
    shake128_squeezeblocks(output, nblocks, &mut s);
    let mut o = nblocks * SHAKE128_RATE;
    outlen -= nblocks * SHAKE128_RATE;
    if outlen != 0 {
        shake128_squeezeblocks(&mut t, 1, &mut s);
        for i in 0..outlen {
            output[o + i] = t[i];
        }
        let _ = &mut o;
    }
}

pub fn shake256(output: &mut [u8], mut outlen: usize, input: &[u8], inlen: usize) {
    let nblocks = outlen / SHAKE256_RATE;
    let mut t = [0u8; SHAKE256_RATE];
    let mut s = [0u64; 25];
    shake256_absorb(&mut s, input, inlen);
    shake256_squeezeblocks(output, nblocks, &mut s);
    let o = nblocks * SHAKE256_RATE;
    outlen -= nblocks * SHAKE256_RATE;
    if outlen != 0 {
        shake256_squeezeblocks(&mut t, 1, &mut s);
        for i in 0..outlen {
            output[o + i] = t[i];
        }
    }
}

// ---- SHA3-256 / SHA3-512 ----
pub fn sha3_256_inc_init(s: &mut [u64]) {
    keccak_inc_init(s);
}
pub fn sha3_256_inc_absorb(s: &mut [u64], input: &[u8], inlen: usize) {
    keccak_inc_absorb(s, SHA3_256_RATE, input, inlen);
}
pub fn sha3_256_inc_finalize(output: &mut [u8], s: &mut [u64]) {
    let mut t = [0u8; SHA3_256_RATE];
    keccak_inc_finalize(s, SHA3_256_RATE, 0x06);
    keccak_squeezeblocks(&mut t, 1, s, SHA3_256_RATE);
    output[..32].copy_from_slice(&t[..32]);
}
pub fn sha3_256(output: &mut [u8], input: &[u8], inlen: usize) {
    let mut s = [0u64; 25];
    let mut t = [0u8; SHA3_256_RATE];
    keccak_absorb(&mut s, SHA3_256_RATE, input, inlen, 0x06);
    keccak_squeezeblocks(&mut t, 1, &mut s, SHA3_256_RATE);
    output[..32].copy_from_slice(&t[..32]);
}
pub fn sha3_512_inc_init(s: &mut [u64]) {
    keccak_inc_init(s);
}
pub fn sha3_512_inc_absorb(s: &mut [u64], input: &[u8], inlen: usize) {
    keccak_inc_absorb(s, SHA3_512_RATE, input, inlen);
}
pub fn sha3_512_inc_finalize(output: &mut [u8], s: &mut [u64]) {
    let mut t = [0u8; SHA3_512_RATE];
    keccak_inc_finalize(s, SHA3_512_RATE, 0x06);
    keccak_squeezeblocks(&mut t, 1, s, SHA3_512_RATE);
    output[..64].copy_from_slice(&t[..64]);
}
pub fn sha3_512(output: &mut [u8], input: &[u8], inlen: usize) {
    let mut s = [0u64; 25];
    let mut t = [0u8; SHA3_512_RATE];
    keccak_absorb(&mut s, SHA3_512_RATE, input, inlen, 0x06);
    keccak_squeezeblocks(&mut t, 1, &mut s, SHA3_512_RATE);
    output[..64].copy_from_slice(&t[..64]);
}

// ------------------------------------------------------------------
// Exported C ABI wrappers (plain names via export_name).
// ------------------------------------------------------------------

macro_rules! st {
    ($p:expr, $n:expr) => {
        core::slice::from_raw_parts_mut($p, $n)
    };
}

#[export_name = "shake256"]
pub unsafe extern "C" fn c_shake256(output: *mut u8, outlen: usize, input: *const u8, inlen: usize) {
    let o = core::slice::from_raw_parts_mut(output, outlen);
    let i = core::slice::from_raw_parts(input, inlen);
    shake256(o, outlen, i, inlen);
}
#[export_name = "shake128"]
pub unsafe extern "C" fn c_shake128(output: *mut u8, outlen: usize, input: *const u8, inlen: usize) {
    let o = core::slice::from_raw_parts_mut(output, outlen);
    let i = core::slice::from_raw_parts(input, inlen);
    shake128(o, outlen, i, inlen);
}
#[export_name = "shake256_inc_init"]
pub unsafe extern "C" fn c_shake256_inc_init(s: *mut u64) {
    shake256_inc_init(st!(s, 26));
}
#[export_name = "shake256_inc_absorb"]
pub unsafe extern "C" fn c_shake256_inc_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    shake256_inc_absorb(st!(s, 26), core::slice::from_raw_parts(input, inlen), inlen);
}
#[export_name = "shake256_inc_finalize"]
pub unsafe extern "C" fn c_shake256_inc_finalize(s: *mut u64) {
    shake256_inc_finalize(st!(s, 26));
}
#[export_name = "shake256_inc_squeeze"]
pub unsafe extern "C" fn c_shake256_inc_squeeze(output: *mut u8, outlen: usize, s: *mut u64) {
    shake256_inc_squeeze(core::slice::from_raw_parts_mut(output, outlen), outlen, st!(s, 26));
}
#[export_name = "shake128_inc_init"]
pub unsafe extern "C" fn c_shake128_inc_init(s: *mut u64) {
    shake128_inc_init(st!(s, 26));
}
#[export_name = "shake128_inc_absorb"]
pub unsafe extern "C" fn c_shake128_inc_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    shake128_inc_absorb(st!(s, 26), core::slice::from_raw_parts(input, inlen), inlen);
}
#[export_name = "shake128_inc_finalize"]
pub unsafe extern "C" fn c_shake128_inc_finalize(s: *mut u64) {
    shake128_inc_finalize(st!(s, 26));
}
#[export_name = "shake128_inc_squeeze"]
pub unsafe extern "C" fn c_shake128_inc_squeeze(output: *mut u8, outlen: usize, s: *mut u64) {
    shake128_inc_squeeze(core::slice::from_raw_parts_mut(output, outlen), outlen, st!(s, 26));
}
#[export_name = "shake128_absorb"]
pub unsafe extern "C" fn c_shake128_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    shake128_absorb(st!(s, 25), core::slice::from_raw_parts(input, inlen), inlen);
}
#[export_name = "shake128_squeezeblocks"]
pub unsafe extern "C" fn c_shake128_squeezeblocks(output: *mut u8, nblocks: usize, s: *mut u64) {
    shake128_squeezeblocks(core::slice::from_raw_parts_mut(output, nblocks * SHAKE128_RATE), nblocks, st!(s, 25));
}
#[export_name = "shake256_absorb"]
pub unsafe extern "C" fn c_shake256_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    shake256_absorb(st!(s, 25), core::slice::from_raw_parts(input, inlen), inlen);
}
#[export_name = "shake256_squeezeblocks"]
pub unsafe extern "C" fn c_shake256_squeezeblocks(output: *mut u8, nblocks: usize, s: *mut u64) {
    shake256_squeezeblocks(core::slice::from_raw_parts_mut(output, nblocks * SHAKE256_RATE), nblocks, st!(s, 25));
}
#[export_name = "sha3_256_inc_init"]
pub unsafe extern "C" fn c_sha3_256_inc_init(s: *mut u64) {
    sha3_256_inc_init(st!(s, 26));
}
#[export_name = "sha3_256_inc_absorb"]
pub unsafe extern "C" fn c_sha3_256_inc_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    sha3_256_inc_absorb(st!(s, 26), core::slice::from_raw_parts(input, inlen), inlen);
}
#[export_name = "sha3_256_inc_finalize"]
pub unsafe extern "C" fn c_sha3_256_inc_finalize(output: *mut u8, s: *mut u64) {
    sha3_256_inc_finalize(core::slice::from_raw_parts_mut(output, 32), st!(s, 26));
}
#[export_name = "sha3_256"]
pub unsafe extern "C" fn c_sha3_256(output: *mut u8, input: *const u8, inlen: usize) {
    sha3_256(core::slice::from_raw_parts_mut(output, 32), core::slice::from_raw_parts(input, inlen), inlen);
}
#[export_name = "sha3_512_inc_init"]
pub unsafe extern "C" fn c_sha3_512_inc_init(s: *mut u64) {
    sha3_512_inc_init(st!(s, 26));
}
#[export_name = "sha3_512_inc_absorb"]
pub unsafe extern "C" fn c_sha3_512_inc_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    sha3_512_inc_absorb(st!(s, 26), core::slice::from_raw_parts(input, inlen), inlen);
}
#[export_name = "sha3_512_inc_finalize"]
pub unsafe extern "C" fn c_sha3_512_inc_finalize(output: *mut u8, s: *mut u64) {
    sha3_512_inc_finalize(core::slice::from_raw_parts_mut(output, 64), st!(s, 26));
}
#[export_name = "sha3_512"]
pub unsafe extern "C" fn c_sha3_512(output: *mut u8, input: *const u8, inlen: usize) {
    sha3_512(core::slice::from_raw_parts_mut(output, 64), core::slice::from_raw_parts(input, inlen), inlen);
}
