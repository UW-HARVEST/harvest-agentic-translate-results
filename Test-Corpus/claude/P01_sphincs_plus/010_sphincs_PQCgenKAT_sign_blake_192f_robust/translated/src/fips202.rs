// Pure-Rust translation of fips202.c (Keccak / SHAKE256 incremental API).

pub const SHAKE256_RATE: usize = 136;

const NROUNDS: usize = 24;

#[inline(always)]
fn rol(a: u64, offset: u32) -> u64 {
    a.rotate_left(offset)
}

#[inline]
fn load64(x: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&x[..8]);
    u64::from_le_bytes(buf)
}

#[inline]
fn store64(x: &mut [u8], u: u64) {
    x[..8].copy_from_slice(&u.to_le_bytes());
}

const KECCAK_F_ROUND_CONSTANTS: [u64; NROUNDS] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
    0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
];

/// Standard 24-round Keccak-F[1600] permutation operating on a 25-lane state.
pub fn keccak_f1600_state_permute(state: &mut [u64]) {
    // Pi-rotation offsets (the rho values applied after each lane is moved).
    const ROT: [[u32; 5]; 5] = [
        [0,  1,  62, 28, 27],
        [36, 44, 6,  55, 20],
        [3,  10, 43, 25, 39],
        [41, 45, 15, 21, 8],
        [18, 2,  61, 56, 14],
    ];

    let mut a = [[0u64; 5]; 5];
    for x in 0..5 {
        for y in 0..5 {
            a[x][y] = state[x + 5 * y];
        }
    }

    for round in 0..NROUNDS {
        // Theta
        let mut c = [0u64; 5];
        for x in 0..5 {
            c[x] = a[x][0] ^ a[x][1] ^ a[x][2] ^ a[x][3] ^ a[x][4];
        }
        let mut d = [0u64; 5];
        for x in 0..5 {
            d[x] = c[(x + 4) % 5] ^ rol(c[(x + 1) % 5], 1);
        }
        for x in 0..5 {
            for y in 0..5 {
                a[x][y] ^= d[x];
            }
        }

        // Rho + Pi
        let mut b = [[0u64; 5]; 5];
        for x in 0..5 {
            for y in 0..5 {
                let nx = y;
                let ny = (2 * x + 3 * y) % 5;
                b[nx][ny] = rol(a[x][y], ROT[y][x]);
            }
        }

        // Chi
        for x in 0..5 {
            for y in 0..5 {
                a[x][y] = b[x][y] ^ ((!b[(x + 1) % 5][y]) & b[(x + 2) % 5][y]);
            }
        }

        // Iota
        a[0][0] ^= KECCAK_F_ROUND_CONSTANTS[round];
    }

    for x in 0..5 {
        for y in 0..5 {
            state[x + 5 * y] = a[x][y];
        }
    }
}

fn keccak_inc_init(s_inc: &mut [u64]) {
    for i in 0..26 {
        s_inc[i] = 0;
    }
}

fn keccak_inc_absorb(s_inc: &mut [u64], r: usize, m: &[u8], mut mlen: usize) {
    let mut m_pos = 0usize;
    while mlen + s_inc[25] as usize >= r {
        let take = r - s_inc[25] as usize;
        for i in 0..take {
            let pos = (s_inc[25] as usize + i) >> 3;
            let shift = 8 * ((s_inc[25] as usize + i) & 0x07);
            s_inc[pos] ^= (m[m_pos + i] as u64) << shift;
        }
        mlen -= take;
        m_pos += take;
        s_inc[25] = 0;
        keccak_f1600_state_permute(s_inc);
    }

    for i in 0..mlen {
        let pos = (s_inc[25] as usize + i) >> 3;
        let shift = 8 * ((s_inc[25] as usize + i) & 0x07);
        s_inc[pos] ^= (m[m_pos + i] as u64) << shift;
    }
    s_inc[25] += mlen as u64;
}

fn keccak_inc_finalize(s_inc: &mut [u64], r: usize, p: u8) {
    let pos = (s_inc[25] as usize) >> 3;
    let shift = 8 * ((s_inc[25] as usize) & 0x07);
    s_inc[pos] ^= (p as u64) << shift;
    let pos2 = (r - 1) >> 3;
    let shift2 = 8 * ((r - 1) & 0x07);
    s_inc[pos2] ^= 128u64 << shift2;
    s_inc[25] = 0;
}

fn keccak_inc_squeeze(h: &mut [u8], mut outlen: usize, s_inc: &mut [u64], r: usize) {
    let mut h_pos = 0usize;
    let mut i = 0usize;
    while i < outlen && i < s_inc[25] as usize {
        let pos = (r - s_inc[25] as usize + i) >> 3;
        let shift = 8 * ((r - s_inc[25] as usize + i) & 0x07);
        h[h_pos + i] = (s_inc[pos] >> shift) as u8;
        i += 1;
    }
    h_pos += i;
    outlen -= i;
    s_inc[25] -= i as u64;

    while outlen > 0 {
        keccak_f1600_state_permute(s_inc);
        let mut j = 0usize;
        while j < outlen && j < r {
            let pos = j >> 3;
            let shift = 8 * (j & 0x07);
            h[h_pos + j] = (s_inc[pos] >> shift) as u8;
            j += 1;
        }
        h_pos += j;
        outlen -= j;
        s_inc[25] = (r - j) as u64;
    }
}

pub fn shake256_inc_init(s_inc: &mut [u64]) {
    keccak_inc_init(s_inc);
}

pub fn shake256_inc_absorb(s_inc: &mut [u64], input: &[u8], inlen: usize) {
    keccak_inc_absorb(s_inc, SHAKE256_RATE, input, inlen);
}

pub fn shake256_inc_finalize(s_inc: &mut [u64]) {
    keccak_inc_finalize(s_inc, SHAKE256_RATE, 0x1F);
}

pub fn shake256_inc_squeeze(out: &mut [u8], outlen: usize, s_inc: &mut [u64]) {
    keccak_inc_squeeze(out, outlen, s_inc, SHAKE256_RATE);
}

fn keccak_absorb(s: &mut [u64], r: usize, m: &[u8], mut mlen: usize, p: u8) {
    let mut m_pos = 0usize;
    let mut t = [0u8; 200];
    for i in 0..25 {
        s[i] = 0;
    }
    while mlen >= r {
        for i in 0..(r / 8) {
            s[i] ^= load64(&m[m_pos + 8 * i..]);
        }
        keccak_f1600_state_permute(s);
        mlen -= r;
        m_pos += r;
    }
    for i in 0..r {
        t[i] = 0;
    }
    for i in 0..mlen {
        t[i] = m[m_pos + i];
    }
    t[mlen] = p;
    t[r - 1] |= 128;
    for i in 0..(r / 8) {
        s[i] ^= load64(&t[8 * i..]);
    }
}

fn keccak_squeezeblocks(h: &mut [u8], mut nblocks: usize, s: &mut [u64], r: usize) {
    let mut h_pos = 0usize;
    while nblocks > 0 {
        keccak_f1600_state_permute(s);
        for i in 0..(r >> 3) {
            store64(&mut h[h_pos + 8 * i..], s[i]);
        }
        h_pos += r;
        nblocks -= 1;
    }
}

pub fn shake256(output: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let nblocks = outlen / SHAKE256_RATE;
    let mut t = [0u8; SHAKE256_RATE];
    let mut s = [0u64; 25];
    keccak_absorb(&mut s, SHAKE256_RATE, input, inlen, 0x1F);
    keccak_squeezeblocks(&mut output[..nblocks * SHAKE256_RATE], nblocks, &mut s, SHAKE256_RATE);
    let off = nblocks * SHAKE256_RATE;
    let leftover = outlen - off;
    if leftover > 0 {
        keccak_squeezeblocks(&mut t, 1, &mut s, SHAKE256_RATE);
        for i in 0..leftover {
            output[off + i] = t[i];
        }
    }
}
