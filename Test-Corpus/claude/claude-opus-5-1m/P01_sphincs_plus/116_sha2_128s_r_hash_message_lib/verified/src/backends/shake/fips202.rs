//! Translation of `lib/shake/src/fips202.c`.
//!
//! The Keccak-f[1600] permutation is a standard, fixed function; this uses the
//! well-known compact formulation, which produces output identical to the
//! reference `KeccakF1600_StatePermute`. All the SHAKE/SHA3 wrappers are ported
//! directly from the C so their padding/rate handling is byte-for-byte the same.

pub const SHAKE128_RATE: usize = 168;
pub const SHAKE256_RATE: usize = 136;
pub const SHA3_256_RATE: usize = 136;
pub const SHA3_512_RATE: usize = 72;

const NROUNDS: usize = 24;

#[inline(always)]
fn rol(a: u64, offset: u32) -> u64 {
    (a << offset) ^ (a >> (64 - offset))
}

#[inline(always)]
unsafe fn load64(x: *const u8) -> u64 {
    let mut r: u64 = 0;
    for i in 0..8 {
        r |= (*x.add(i) as u64) << (8 * i);
    }
    r
}

#[inline(always)]
unsafe fn store64(x: *mut u8, u: u64) {
    for i in 0..8 {
        *x.add(i) = (u >> (8 * i)) as u8;
    }
}

static KECCAK_F_ROUND_CONSTANTS: [u64; NROUNDS] = [
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808a,
    0x8000000080008000,
    0x000000000000808b,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008a,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000a,
    0x000000008000808b,
    0x800000000000008b,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800a,
    0x800000008000000a,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
];

const ROTC: [u32; 24] = [
    1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44,
];
const PILN: [usize; 24] = [
    10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1,
];

/// The Keccak-f[1600] permutation, operating on the 25 lanes at `state`.
unsafe fn keccak_f1600_state_permute(state: *mut u64) {
    let st = core::slice::from_raw_parts_mut(state, 25);
    let mut bc = [0u64; 5];

    for r in 0..NROUNDS {
        // Theta
        for i in 0..5 {
            bc[i] = st[i] ^ st[i + 5] ^ st[i + 10] ^ st[i + 15] ^ st[i + 20];
        }
        for i in 0..5 {
            let t = bc[(i + 4) % 5] ^ rol(bc[(i + 1) % 5], 1);
            let mut j = 0;
            while j < 25 {
                st[j + i] ^= t;
                j += 5;
            }
        }

        // Rho Pi
        let mut t = st[1];
        for i in 0..24 {
            let j = PILN[i];
            let tmp = st[j];
            st[j] = rol(t, ROTC[i]);
            t = tmp;
        }

        // Chi
        let mut j = 0;
        while j < 25 {
            for i in 0..5 {
                bc[i] = st[j + i];
            }
            for i in 0..5 {
                st[j + i] ^= (!bc[(i + 1) % 5]) & bc[(i + 2) % 5];
            }
            j += 5;
        }

        // Iota
        st[0] ^= KECCAK_F_ROUND_CONSTANTS[r];
    }
}

/// Absorb step of Keccak; non-incremental, starts by zeroeing the state.
unsafe fn keccak_absorb(s: *mut u64, r: usize, mut m: *const u8, mut mlen: usize, p: u8) {
    let mut t = [0u8; 200];

    for i in 0..25 {
        *s.add(i) = 0;
    }

    while mlen >= r {
        for i in 0..(r / 8) {
            *s.add(i) ^= load64(m.add(8 * i));
        }
        keccak_f1600_state_permute(s);
        mlen -= r;
        m = m.add(r);
    }

    for i in 0..r {
        t[i] = 0;
    }
    for i in 0..mlen {
        t[i] = *m.add(i);
    }
    t[mlen] = p;
    t[r - 1] |= 128;
    for i in 0..(r / 8) {
        *s.add(i) ^= load64(t.as_ptr().add(8 * i));
    }
}

/// Squeeze step of Keccak. Squeezes full blocks of `r` bytes each.
unsafe fn keccak_squeezeblocks(mut h: *mut u8, mut nblocks: usize, s: *mut u64, r: usize) {
    while nblocks > 0 {
        keccak_f1600_state_permute(s);
        for i in 0..(r >> 3) {
            store64(h.add(8 * i), *s.add(i));
        }
        h = h.add(r);
        nblocks -= 1;
    }
}

/// Initializes the incremental Keccak state to zero. `s_inc` has 26 lanes.
unsafe fn keccak_inc_init(s_inc: *mut u64) {
    for i in 0..25 {
        *s_inc.add(i) = 0;
    }
    *s_inc.add(25) = 0;
}

unsafe fn keccak_inc_absorb(s_inc: *mut u64, r: usize, mut m: *const u8, mut mlen: usize) {
    // Recall that s_inc[25] is the non-absorbed bytes xored into the state.
    while mlen + (*s_inc.add(25)) as usize >= r {
        let pos = (*s_inc.add(25)) as usize;
        for i in 0..(r - pos) {
            let bitpos = pos + i;
            *s_inc.add(bitpos >> 3) ^= (*m.add(i) as u64) << (8 * (bitpos & 0x07));
        }
        mlen -= r - pos;
        m = m.add(r - pos);
        *s_inc.add(25) = 0;

        keccak_f1600_state_permute(s_inc);
    }

    let pos = (*s_inc.add(25)) as usize;
    for i in 0..mlen {
        let bitpos = pos + i;
        *s_inc.add(bitpos >> 3) ^= (*m.add(i) as u64) << (8 * (bitpos & 0x07));
    }
    *s_inc.add(25) += mlen as u64;
}

unsafe fn keccak_inc_finalize(s_inc: *mut u64, r: usize, p: u8) {
    let pos = (*s_inc.add(25)) as usize;
    *s_inc.add(pos >> 3) ^= (p as u64) << (8 * (pos & 0x07));
    *s_inc.add((r - 1) >> 3) ^= 128u64 << (8 * ((r - 1) & 0x07));
    *s_inc.add(25) = 0;
}

unsafe fn keccak_inc_squeeze(mut h: *mut u8, mut outlen: usize, s_inc: *mut u64, r: usize) {
    // First consume any bytes we still have sitting around.
    let mut i: usize = 0;
    while i < outlen && i < (*s_inc.add(25)) as usize {
        let pos = r - (*s_inc.add(25)) as usize + i;
        *h.add(i) = (*s_inc.add(pos >> 3) >> (8 * (pos & 0x07))) as u8;
        i += 1;
    }
    h = h.add(i);
    outlen -= i;
    *s_inc.add(25) -= i as u64;

    // Then squeeze the remaining necessary blocks.
    while outlen > 0 {
        keccak_f1600_state_permute(s_inc);

        let mut i2: usize = 0;
        while i2 < outlen && i2 < r {
            *h.add(i2) = (*s_inc.add(i2 >> 3) >> (8 * (i2 & 0x07))) as u8;
            i2 += 1;
        }
        h = h.add(i2);
        outlen -= i2;
        *s_inc.add(25) = (r - i2) as u64;
    }
}

// ---- SHAKE128 --------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake128_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    keccak_absorb(s, SHAKE128_RATE, input, inlen, 0x1F);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake128_squeezeblocks(output: *mut u8, nblocks: usize, s: *mut u64) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE128_RATE);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake128_inc_init(s_inc: *mut u64) {
    keccak_inc_init(s_inc);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake128_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize) {
    keccak_inc_absorb(s_inc, SHAKE128_RATE, input, inlen);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake128_inc_finalize(s_inc: *mut u64) {
    keccak_inc_finalize(s_inc, SHAKE128_RATE, 0x1F);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake128_inc_squeeze(output: *mut u8, outlen: usize, s_inc: *mut u64) {
    keccak_inc_squeeze(output, outlen, s_inc, SHAKE128_RATE);
}

// ---- SHAKE256 --------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_init(s_inc: *mut u64) {
    keccak_inc_init(s_inc);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize) {
    keccak_inc_absorb(s_inc, SHAKE256_RATE, input, inlen);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_finalize(s_inc: *mut u64) {
    keccak_inc_finalize(s_inc, SHAKE256_RATE, 0x1F);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_squeeze(output: *mut u8, outlen: usize, s_inc: *mut u64) {
    keccak_inc_squeeze(output, outlen, s_inc, SHAKE256_RATE);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    keccak_absorb(s, SHAKE256_RATE, input, inlen, 0x1F);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_squeezeblocks(output: *mut u8, nblocks: usize, s: *mut u64) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE256_RATE);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake128(output: *mut u8, outlen: usize, input: *const u8, inlen: usize) {
    let nblocks = outlen / SHAKE128_RATE;
    let mut t = [0u8; SHAKE128_RATE];
    let mut s = [0u64; 25];

    shake128_absorb(s.as_mut_ptr(), input, inlen);
    shake128_squeezeblocks(output, nblocks, s.as_mut_ptr());

    let output2 = output.add(nblocks * SHAKE128_RATE);
    let rem = outlen - nblocks * SHAKE128_RATE;

    if rem != 0 {
        shake128_squeezeblocks(t.as_mut_ptr(), 1, s.as_mut_ptr());
        for i in 0..rem {
            *output2.add(i) = t[i];
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256(output: *mut u8, outlen: usize, input: *const u8, inlen: usize) {
    let nblocks = outlen / SHAKE256_RATE;
    let mut t = [0u8; SHAKE256_RATE];
    let mut s = [0u64; 25];

    shake256_absorb(s.as_mut_ptr(), input, inlen);
    shake256_squeezeblocks(output, nblocks, s.as_mut_ptr());

    let output2 = output.add(nblocks * SHAKE256_RATE);
    let rem = outlen - nblocks * SHAKE256_RATE;

    if rem != 0 {
        shake256_squeezeblocks(t.as_mut_ptr(), 1, s.as_mut_ptr());
        for i in 0..rem {
            *output2.add(i) = t[i];
        }
    }
}

// ---- SHA3-256 --------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha3_256_inc_init(s_inc: *mut u64) {
    keccak_inc_init(s_inc);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha3_256_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize) {
    keccak_inc_absorb(s_inc, SHA3_256_RATE, input, inlen);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha3_256_inc_finalize(output: *mut u8, s_inc: *mut u64) {
    let mut t = [0u8; SHA3_256_RATE];
    keccak_inc_finalize(s_inc, SHA3_256_RATE, 0x06);
    keccak_squeezeblocks(t.as_mut_ptr(), 1, s_inc, SHA3_256_RATE);
    for i in 0..32 {
        *output.add(i) = t[i];
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha3_256(output: *mut u8, input: *const u8, inlen: usize) {
    let mut s = [0u64; 25];
    let mut t = [0u8; SHA3_256_RATE];

    keccak_absorb(s.as_mut_ptr(), SHA3_256_RATE, input, inlen, 0x06);
    keccak_squeezeblocks(t.as_mut_ptr(), 1, s.as_mut_ptr(), SHA3_256_RATE);
    for i in 0..32 {
        *output.add(i) = t[i];
    }
}

// ---- SHA3-512 --------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha3_512_inc_init(s_inc: *mut u64) {
    keccak_inc_init(s_inc);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha3_512_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize) {
    keccak_inc_absorb(s_inc, SHA3_512_RATE, input, inlen);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha3_512_inc_finalize(output: *mut u8, s_inc: *mut u64) {
    let mut t = [0u8; SHA3_512_RATE];
    keccak_inc_finalize(s_inc, SHA3_512_RATE, 0x06);
    keccak_squeezeblocks(t.as_mut_ptr(), 1, s_inc, SHA3_512_RATE);
    for i in 0..64 {
        *output.add(i) = t[i];
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sha3_512(output: *mut u8, input: *const u8, inlen: usize) {
    let mut s = [0u64; 25];
    let mut t = [0u8; SHA3_512_RATE];

    keccak_absorb(s.as_mut_ptr(), SHA3_512_RATE, input, inlen, 0x06);
    keccak_squeezeblocks(t.as_mut_ptr(), 1, s.as_mut_ptr(), SHA3_512_RATE);
    for i in 0..64 {
        *output.add(i) = t[i];
    }
}
