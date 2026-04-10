// Based on the public domain implementation in
// crypto_hash/keccakc512/simple/ from http://bench.cr.yp.to/supercop.html
// by Ronny Van Keer
// and the public domain "TweetFips202" implementation
// from https://twitter.com/tweetfips202
// by Gilles Van Assche, Daniel J. Bernstein, and Peter Schwabe

pub const SHAKE128_RATE: usize = 168;
pub const SHAKE256_RATE: usize = 136;
pub const SHA3_256_RATE: usize = 136;
pub const SHA3_512_RATE: usize = 72;

const NROUNDS: usize = 24;

#[inline(always)]
fn rol(a: u64, offset: u32) -> u64 {
    a.rotate_left(offset)
}

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

static KECCAKF_ROUND_CONSTANTS: [u64; NROUNDS] = [
    0x0000000000000001, 0x0000000000008082,
    0x800000000000808a, 0x8000000080008000,
    0x000000000000808b, 0x0000000080000001,
    0x8000000080008081, 0x8000000000008009,
    0x000000000000008a, 0x0000000000000088,
    0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b,
    0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080,
    0x000000000000800a, 0x800000008000000a,
    0x8000000080008081, 0x8000000000008080,
    0x0000000080000001, 0x8000000080008008,
];

fn keccakf1600_state_permute(state: &mut [u64; 25]) {
    let (mut aba, mut abe, mut abi, mut abo, mut abu) = (state[0], state[1], state[2], state[3], state[4]);
    let (mut aga, mut age, mut agi, mut ago, mut agu) = (state[5], state[6], state[7], state[8], state[9]);
    let (mut aka, mut ake, mut aki, mut ako, mut aku) = (state[10], state[11], state[12], state[13], state[14]);
    let (mut ama, mut ame, mut ami, mut amo, mut amu) = (state[15], state[16], state[17], state[18], state[19]);
    let (mut asa, mut ase, mut asi, mut aso, mut asu) = (state[20], state[21], state[22], state[23], state[24]);

    let (mut eba, mut ebe, mut ebi, mut ebo, mut ebu);
    let (mut ega, mut ege, mut egi, mut ego, mut egu);
    let (mut eka, mut eke, mut eki, mut eko, mut eku);
    let (mut ema, mut eme, mut emi, mut emo, mut emu);
    let (mut esa, mut ese, mut esi, mut eso, mut esu);

    let mut round = 0;
    while round < NROUNDS {
        // prepareTheta
        let mut bca = aba ^ aga ^ aka ^ ama ^ asa;
        let mut bce = abe ^ age ^ ake ^ ame ^ ase;
        let mut bci = abi ^ agi ^ aki ^ ami ^ asi;
        let mut bco = abo ^ ago ^ ako ^ amo ^ aso;
        let mut bcu = abu ^ agu ^ aku ^ amu ^ asu;

        let da = bcu ^ rol(bce, 1);
        let de = bca ^ rol(bci, 1);
        let di = bce ^ rol(bco, 1);
        let d_o = bci ^ rol(bcu, 1);
        let du = bco ^ rol(bca, 1);

        aba ^= da; bca = aba;
        age ^= de; bce = rol(age, 44);
        aki ^= di; bci = rol(aki, 43);
        amo ^= d_o; bco = rol(amo, 21);
        asu ^= du; bcu = rol(asu, 14);
        eba = bca ^ ((!bce) & bci); eba ^= KECCAKF_ROUND_CONSTANTS[round];
        ebe = bce ^ ((!bci) & bco);
        ebi = bci ^ ((!bco) & bcu);
        ebo = bco ^ ((!bcu) & bca);
        ebu = bcu ^ ((!bca) & bce);

        abo ^= d_o; bca = rol(abo, 28);
        agu ^= du; bce = rol(agu, 20);
        aka ^= da; bci = rol(aka, 3);
        ame ^= de; bco = rol(ame, 45);
        asi ^= di; bcu = rol(asi, 61);
        ega = bca ^ ((!bce) & bci);
        ege = bce ^ ((!bci) & bco);
        egi = bci ^ ((!bco) & bcu);
        ego = bco ^ ((!bcu) & bca);
        egu = bcu ^ ((!bca) & bce);

        abe ^= de; bca = rol(abe, 1);
        agi ^= di; bce = rol(agi, 6);
        ako ^= d_o; bci = rol(ako, 25);
        amu ^= du; bco = rol(amu, 8);
        asa ^= da; bcu = rol(asa, 18);
        eka = bca ^ ((!bce) & bci);
        eke = bce ^ ((!bci) & bco);
        eki = bci ^ ((!bco) & bcu);
        eko = bco ^ ((!bcu) & bca);
        eku = bcu ^ ((!bca) & bce);

        abu ^= du; bca = rol(abu, 27);
        aga ^= da; bce = rol(aga, 36);
        ake ^= de; bci = rol(ake, 10);
        ami ^= di; bco = rol(ami, 15);
        aso ^= d_o; bcu = rol(aso, 56);
        ema = bca ^ ((!bce) & bci);
        eme = bce ^ ((!bci) & bco);
        emi = bci ^ ((!bco) & bcu);
        emo = bco ^ ((!bcu) & bca);
        emu = bcu ^ ((!bca) & bce);

        abi ^= di; bca = rol(abi, 62);
        ago ^= d_o; bce = rol(ago, 55);
        aku ^= du; bci = rol(aku, 39);
        ama ^= da; bco = rol(ama, 41);
        ase ^= de; bcu = rol(ase, 2);
        esa = bca ^ ((!bce) & bci);
        ese = bce ^ ((!bci) & bco);
        esi = bci ^ ((!bco) & bcu);
        eso = bco ^ ((!bcu) & bca);
        esu = bcu ^ ((!bca) & bce);

        // prepareTheta
        bca = eba ^ ega ^ eka ^ ema ^ esa;
        bce = ebe ^ ege ^ eke ^ eme ^ ese;
        bci = ebi ^ egi ^ eki ^ emi ^ esi;
        bco = ebo ^ ego ^ eko ^ emo ^ eso;
        bcu = ebu ^ egu ^ eku ^ emu ^ esu;

        let da = bcu ^ rol(bce, 1);
        let de = bca ^ rol(bci, 1);
        let di = bce ^ rol(bco, 1);
        let d_o = bci ^ rol(bcu, 1);
        let du = bco ^ rol(bca, 1);

        eba ^= da; bca = eba;
        ege ^= de; bce = rol(ege, 44);
        eki ^= di; bci = rol(eki, 43);
        emo ^= d_o; bco = rol(emo, 21);
        esu ^= du; bcu = rol(esu, 14);
        aba = bca ^ ((!bce) & bci); aba ^= KECCAKF_ROUND_CONSTANTS[round + 1];
        abe = bce ^ ((!bci) & bco);
        abi = bci ^ ((!bco) & bcu);
        abo = bco ^ ((!bcu) & bca);
        abu = bcu ^ ((!bca) & bce);

        ebo ^= d_o; bca = rol(ebo, 28);
        egu ^= du; bce = rol(egu, 20);
        eka ^= da; bci = rol(eka, 3);
        eme ^= de; bco = rol(eme, 45);
        esi ^= di; bcu = rol(esi, 61);
        aga = bca ^ ((!bce) & bci);
        age = bce ^ ((!bci) & bco);
        agi = bci ^ ((!bco) & bcu);
        ago = bco ^ ((!bcu) & bca);
        agu = bcu ^ ((!bca) & bce);

        ebe ^= de; bca = rol(ebe, 1);
        egi ^= di; bce = rol(egi, 6);
        eko ^= d_o; bci = rol(eko, 25);
        emu ^= du; bco = rol(emu, 8);
        esa ^= da; bcu = rol(esa, 18);
        aka = bca ^ ((!bce) & bci);
        ake = bce ^ ((!bci) & bco);
        aki = bci ^ ((!bco) & bcu);
        ako = bco ^ ((!bcu) & bca);
        aku = bcu ^ ((!bca) & bce);

        ebu ^= du; bca = rol(ebu, 27);
        ega ^= da; bce = rol(ega, 36);
        eke ^= de; bci = rol(eke, 10);
        emi ^= di; bco = rol(emi, 15);
        eso ^= d_o; bcu = rol(eso, 56);
        ama = bca ^ ((!bce) & bci);
        ame = bce ^ ((!bci) & bco);
        ami = bci ^ ((!bco) & bcu);
        amo = bco ^ ((!bcu) & bca);
        amu = bcu ^ ((!bca) & bce);

        ebi ^= di; bca = rol(ebi, 62);
        ego ^= d_o; bce = rol(ego, 55);
        eku ^= du; bci = rol(eku, 39);
        ema ^= da; bco = rol(ema, 41);
        ese ^= de; bcu = rol(ese, 2);
        asa = bca ^ ((!bce) & bci);
        ase = bce ^ ((!bci) & bco);
        asi = bci ^ ((!bco) & bcu);
        aso = bco ^ ((!bcu) & bca);
        asu = bcu ^ ((!bca) & bce);

        round += 2;
    }

    state[0] = aba; state[1] = abe; state[2] = abi; state[3] = abo; state[4] = abu;
    state[5] = aga; state[6] = age; state[7] = agi; state[8] = ago; state[9] = agu;
    state[10] = aka; state[11] = ake; state[12] = aki; state[13] = ako; state[14] = aku;
    state[15] = ama; state[16] = ame; state[17] = ami; state[18] = amo; state[19] = amu;
    state[20] = asa; state[21] = ase; state[22] = asi; state[23] = aso; state[24] = asu;
}

fn keccak_absorb(s: &mut [u64; 25], r: usize, m: &[u8], p: u8) {
    for i in 0..25 {
        s[i] = 0;
    }

    let mut off = 0usize;
    let mut remaining = m.len();

    while remaining >= r {
        for i in 0..(r / 8) {
            s[i] ^= load64(&m[off + 8 * i..]);
        }
        keccakf1600_state_permute(s);
        remaining -= r;
        off += r;
    }

    let mut t = [0u8; 200];
    t[..remaining].copy_from_slice(&m[off..off + remaining]);
    t[remaining] = p;
    t[r - 1] |= 128;
    for i in 0..(r / 8) {
        s[i] ^= load64(&t[8 * i..]);
    }
}

fn keccak_squeezeblocks(h: &mut [u8], nblocks: usize, s: &mut [u64; 25], r: usize) {
    let mut off = 0;
    for _ in 0..nblocks {
        keccakf1600_state_permute(s);
        for i in 0..(r >> 3) {
            store64(&mut h[off + 8 * i..], s[i]);
        }
        off += r;
    }
}

fn keccak_inc_init(s_inc: &mut [u64; 26]) {
    for i in 0..26 {
        s_inc[i] = 0;
    }
}

fn keccak_inc_absorb(s_inc: &mut [u64; 26], r: usize, m: &[u8]) {
    let mut off = 0usize;
    let mut mlen = m.len();

    while mlen + (s_inc[25] as usize) >= r {
        let absorbed = s_inc[25] as usize;
        for i in 0..(r - absorbed) {
            let pos = absorbed + i;
            s_inc[pos >> 3] ^= (m[off + i] as u64) << (8 * (pos & 0x07));
        }
        mlen -= r - absorbed;
        off += r - absorbed;
        s_inc[25] = 0;

        let state = unsafe { &mut *(s_inc.as_mut_ptr() as *mut [u64; 25]) };
        keccakf1600_state_permute(state);
    }

    let absorbed = s_inc[25] as usize;
    for i in 0..mlen {
        let pos = absorbed + i;
        s_inc[pos >> 3] ^= (m[off + i] as u64) << (8 * (pos & 0x07));
    }
    s_inc[25] += mlen as u64;
}

fn keccak_inc_finalize(s_inc: &mut [u64; 26], r: usize, p: u8) {
    let pos = s_inc[25] as usize;
    s_inc[pos >> 3] ^= (p as u64) << (8 * (pos & 0x07));
    s_inc[(r - 1) >> 3] ^= 128u64 << (8 * ((r - 1) & 0x07));
    s_inc[25] = 0;
}

fn keccak_inc_squeeze(h: &mut [u8], mut outlen: usize, s_inc: &mut [u64; 26], r: usize) {
    let mut off = 0usize;

    // First consume any bytes we still have sitting around
    let mut i = 0;
    let avail = s_inc[25] as usize;
    while i < outlen && i < avail {
        let pos = r - avail + i;
        h[off + i] = (s_inc[pos >> 3] >> (8 * (pos & 0x07))) as u8;
        i += 1;
    }
    off += i;
    outlen -= i;
    s_inc[25] -= i as u64;

    // Then squeeze the remaining necessary blocks
    while outlen > 0 {
        let state = unsafe { &mut *(s_inc.as_mut_ptr() as *mut [u64; 25]) };
        keccakf1600_state_permute(state);

        let mut i = 0;
        while i < outlen && i < r {
            h[off + i] = (s_inc[i >> 3] >> (8 * (i & 0x07))) as u8;
            i += 1;
        }
        off += i;
        outlen -= i;
        s_inc[25] = (r - i) as u64;
    }
}

// --- Public SHAKE256 API ---

pub fn shake256_inc_init(s_inc: &mut [u64; 26]) {
    keccak_inc_init(s_inc);
}

pub fn shake256_inc_absorb(s_inc: &mut [u64; 26], input: &[u8]) {
    keccak_inc_absorb(s_inc, SHAKE256_RATE, input);
}

pub fn shake256_inc_finalize(s_inc: &mut [u64; 26]) {
    keccak_inc_finalize(s_inc, SHAKE256_RATE, 0x1F);
}

pub fn shake256_inc_squeeze(output: &mut [u8], outlen: usize, s_inc: &mut [u64; 26]) {
    keccak_inc_squeeze(output, outlen, s_inc, SHAKE256_RATE);
}

pub fn shake256_absorb(s: &mut [u64; 25], input: &[u8]) {
    keccak_absorb(s, SHAKE256_RATE, input, 0x1F);
}

pub fn shake256_squeezeblocks(output: &mut [u8], nblocks: usize, s: &mut [u64; 25]) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE256_RATE);
}

pub fn shake256(output: &mut [u8], outlen: usize, input: &[u8]) {
    let nblocks = outlen / SHAKE256_RATE;
    let mut s = [0u64; 25];

    shake256_absorb(&mut s, input);
    shake256_squeezeblocks(output, nblocks, &mut s);

    let done = nblocks * SHAKE256_RATE;
    let remaining = outlen - done;

    if remaining > 0 {
        let mut t = [0u8; SHAKE256_RATE];
        shake256_squeezeblocks(&mut t, 1, &mut s);
        output[done..done + remaining].copy_from_slice(&t[..remaining]);
    }
}

// --- Public SHAKE128 API ---

pub fn shake128_inc_init(s_inc: &mut [u64; 26]) {
    keccak_inc_init(s_inc);
}

pub fn shake128_inc_absorb(s_inc: &mut [u64; 26], input: &[u8]) {
    keccak_inc_absorb(s_inc, SHAKE128_RATE, input);
}

pub fn shake128_inc_finalize(s_inc: &mut [u64; 26]) {
    keccak_inc_finalize(s_inc, SHAKE128_RATE, 0x1F);
}

pub fn shake128_inc_squeeze(output: &mut [u8], outlen: usize, s_inc: &mut [u64; 26]) {
    keccak_inc_squeeze(output, outlen, s_inc, SHAKE128_RATE);
}

pub fn shake128_absorb(s: &mut [u64; 25], input: &[u8]) {
    keccak_absorb(s, SHAKE128_RATE, input, 0x1F);
}

pub fn shake128_squeezeblocks(output: &mut [u8], nblocks: usize, s: &mut [u64; 25]) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE128_RATE);
}

pub fn shake128(output: &mut [u8], outlen: usize, input: &[u8]) {
    let nblocks = outlen / SHAKE128_RATE;
    let mut s = [0u64; 25];

    shake128_absorb(&mut s, input);
    shake128_squeezeblocks(output, nblocks, &mut s);

    let done = nblocks * SHAKE128_RATE;
    let remaining = outlen - done;

    if remaining > 0 {
        let mut t = [0u8; SHAKE128_RATE];
        shake128_squeezeblocks(&mut t, 1, &mut s);
        output[done..done + remaining].copy_from_slice(&t[..remaining]);
    }
}

// --- Public SHA3-256 API ---

pub fn sha3_256_inc_init(s_inc: &mut [u64; 26]) {
    keccak_inc_init(s_inc);
}

pub fn sha3_256_inc_absorb(s_inc: &mut [u64; 26], input: &[u8]) {
    keccak_inc_absorb(s_inc, SHA3_256_RATE, input);
}

pub fn sha3_256_inc_finalize(output: &mut [u8], s_inc: &mut [u64; 26]) {
    keccak_inc_finalize(s_inc, SHA3_256_RATE, 0x06);
    let mut t = [0u8; SHA3_256_RATE];
    keccak_squeezeblocks(
        &mut t,
        1,
        unsafe { &mut *(s_inc.as_mut_ptr() as *mut [u64; 25]) },
        SHA3_256_RATE,
    );
    output[..32].copy_from_slice(&t[..32]);
}

pub fn sha3_256(output: &mut [u8], input: &[u8]) {
    let mut s = [0u64; 26];
    sha3_256_inc_init(&mut s);
    sha3_256_inc_absorb(&mut s, input);
    sha3_256_inc_finalize(output, &mut s);
}

// --- Public SHA3-512 API ---

pub fn sha3_512_inc_init(s_inc: &mut [u64; 26]) {
    keccak_inc_init(s_inc);
}

pub fn sha3_512_inc_absorb(s_inc: &mut [u64; 26], input: &[u8]) {
    keccak_inc_absorb(s_inc, SHA3_512_RATE, input);
}

pub fn sha3_512_inc_finalize(output: &mut [u8], s_inc: &mut [u64; 26]) {
    keccak_inc_finalize(s_inc, SHA3_512_RATE, 0x06);
    let mut t = [0u8; SHA3_512_RATE];
    keccak_squeezeblocks(
        &mut t,
        1,
        unsafe { &mut *(s_inc.as_mut_ptr() as *mut [u64; 25]) },
        SHA3_512_RATE,
    );
    output[..64].copy_from_slice(&t[..64]);
}

pub fn sha3_512(output: &mut [u8], input: &[u8]) {
    let mut s = [0u64; 26];
    sha3_512_inc_init(&mut s);
    sha3_512_inc_absorb(&mut s, input);
    sha3_512_inc_finalize(output, &mut s);
}
