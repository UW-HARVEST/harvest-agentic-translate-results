//! Translation of `c_src/lib/shake/src/fips202.c` and
//! `c_src/lib/shake/include/fips202.h`.
//!
//! Based on the public domain implementation in
//! crypto_hash/keccakc512/simple/ from http://bench.cr.yp.to/supercop.html
//! by Ronny Van Keer
//! and the public domain "TweetFips202" implementation
//! from https://twitter.com/tweetfips202
//! by Gilles Van Assche, Daniel J. Bernstein, and Peter Schwabe

const NROUNDS: usize = 24;

/// `ROL(a, offset) (((a) << (offset)) ^ ((a) >> (64 - (offset))))`
///
/// Every call site uses a rotation amount in `1..=62`, so this is exactly a
/// 64-bit rotate-left.
#[inline(always)]
fn rol(a: u64, offset: u32) -> u64 {
    (a << offset) ^ (a >> (64 - offset))
}

pub const SHAKE128_RATE: usize = 168;
pub const SHAKE256_RATE: usize = 136;
pub const SHA3_256_RATE: usize = 136;
pub const SHA3_512_RATE: usize = 72;

/*************************************************
 * Name:        load64
 *
 * Description: Load 8 bytes into uint64_t in little-endian order
 **************************************************/
#[inline]
fn load64(x: &[u8]) -> u64 {
    let mut r: u64 = 0;
    for i in 0..8 {
        r |= (x[i] as u64) << (8 * i);
    }
    r
}

/*************************************************
 * Name:        store64
 *
 * Description: Store a 64-bit integer to a byte array in little-endian order
 **************************************************/
#[inline]
fn store64(x: &mut [u8], u: u64) {
    for i in 0..8 {
        x[i] = (u >> (8 * i)) as u8;
    }
}

/* Keccak round constants */
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

/*************************************************
 * Name:        KeccakF1600_StatePermute
 *
 * Description: The Keccak F1600 Permutation
 *
 * Arguments:   - uint64_t *state: pointer to input/output Keccak state
 **************************************************/
#[allow(clippy::needless_range_loop)]
#[allow(unused_assignments)]
fn keccak_f1600_state_permute(state: &mut [u64]) {
    let (mut bca, mut bce, mut bci, mut bco, mut bcu): (u64, u64, u64, u64, u64) = (0, 0, 0, 0, 0);
    let (mut eba, mut ebe, mut ebi, mut ebo, mut ebu): (u64, u64, u64, u64, u64) = (0, 0, 0, 0, 0);
    let (mut ega, mut ege, mut egi, mut ego, mut egu): (u64, u64, u64, u64, u64) = (0, 0, 0, 0, 0);
    let (mut eka, mut eke, mut eki, mut eko, mut eku): (u64, u64, u64, u64, u64) = (0, 0, 0, 0, 0);
    let (mut ema, mut eme, mut emi, mut emo, mut emu): (u64, u64, u64, u64, u64) = (0, 0, 0, 0, 0);
    let (mut esa, mut ese, mut esi, mut eso, mut esu): (u64, u64, u64, u64, u64) = (0, 0, 0, 0, 0);

    // copyFromState(A, state)
    let mut aba = state[0];
    let mut abe = state[1];
    let mut abi = state[2];
    let mut abo = state[3];
    let mut abu = state[4];
    let mut aga = state[5];
    let mut age = state[6];
    let mut agi = state[7];
    let mut ago = state[8];
    let mut agu = state[9];
    let mut aka = state[10];
    let mut ake = state[11];
    let mut aki = state[12];
    let mut ako = state[13];
    let mut aku = state[14];
    let mut ama = state[15];
    let mut ame = state[16];
    let mut ami = state[17];
    let mut amo = state[18];
    let mut amu = state[19];
    let mut asa = state[20];
    let mut ase = state[21];
    let mut asi = state[22];
    let mut aso = state[23];
    let mut asu = state[24];

    let mut round = 0usize;
    while round < NROUNDS {
        //    prepareTheta
        bca = aba ^ aga ^ aka ^ ama ^ asa;
        bce = abe ^ age ^ ake ^ ame ^ ase;
        bci = abi ^ agi ^ aki ^ ami ^ asi;
        bco = abo ^ ago ^ ako ^ amo ^ aso;
        bcu = abu ^ agu ^ aku ^ amu ^ asu;

        // thetaRhoPiChiIotaPrepareTheta(round  , A, E)
        let da = bcu ^ rol(bce, 1);
        let de = bca ^ rol(bci, 1);
        let di = bce ^ rol(bco, 1);
        let dob = bci ^ rol(bcu, 1);
        let du = bco ^ rol(bca, 1);

        aba ^= da;
        bca = aba;
        age ^= de;
        bce = rol(age, 44);
        aki ^= di;
        bci = rol(aki, 43);
        amo ^= dob;
        bco = rol(amo, 21);
        asu ^= du;
        bcu = rol(asu, 14);
        eba = bca ^ ((!bce) & bci);
        eba ^= KECCAK_F_ROUND_CONSTANTS[round];
        ebe = bce ^ ((!bci) & bco);
        ebi = bci ^ ((!bco) & bcu);
        ebo = bco ^ ((!bcu) & bca);
        ebu = bcu ^ ((!bca) & bce);

        abo ^= dob;
        bca = rol(abo, 28);
        agu ^= du;
        bce = rol(agu, 20);
        aka ^= da;
        bci = rol(aka, 3);
        ame ^= de;
        bco = rol(ame, 45);
        asi ^= di;
        bcu = rol(asi, 61);
        ega = bca ^ ((!bce) & bci);
        ege = bce ^ ((!bci) & bco);
        egi = bci ^ ((!bco) & bcu);
        ego = bco ^ ((!bcu) & bca);
        egu = bcu ^ ((!bca) & bce);

        abe ^= de;
        bca = rol(abe, 1);
        agi ^= di;
        bce = rol(agi, 6);
        ako ^= dob;
        bci = rol(ako, 25);
        amu ^= du;
        bco = rol(amu, 8);
        asa ^= da;
        bcu = rol(asa, 18);
        eka = bca ^ ((!bce) & bci);
        eke = bce ^ ((!bci) & bco);
        eki = bci ^ ((!bco) & bcu);
        eko = bco ^ ((!bcu) & bca);
        eku = bcu ^ ((!bca) & bce);

        abu ^= du;
        bca = rol(abu, 27);
        aga ^= da;
        bce = rol(aga, 36);
        ake ^= de;
        bci = rol(ake, 10);
        ami ^= di;
        bco = rol(ami, 15);
        aso ^= dob;
        bcu = rol(aso, 56);
        ema = bca ^ ((!bce) & bci);
        eme = bce ^ ((!bci) & bco);
        emi = bci ^ ((!bco) & bcu);
        emo = bco ^ ((!bcu) & bca);
        emu = bcu ^ ((!bca) & bce);

        abi ^= di;
        bca = rol(abi, 62);
        ago ^= dob;
        bce = rol(ago, 55);
        aku ^= du;
        bci = rol(aku, 39);
        ama ^= da;
        bco = rol(ama, 41);
        ase ^= de;
        bcu = rol(ase, 2);
        esa = bca ^ ((!bce) & bci);
        ese = bce ^ ((!bci) & bco);
        esi = bci ^ ((!bco) & bcu);
        eso = bco ^ ((!bcu) & bca);
        esu = bcu ^ ((!bca) & bce);

        //    prepareTheta
        bca = eba ^ ega ^ eka ^ ema ^ esa;
        bce = ebe ^ ege ^ eke ^ eme ^ ese;
        bci = ebi ^ egi ^ eki ^ emi ^ esi;
        bco = ebo ^ ego ^ eko ^ emo ^ eso;
        bcu = ebu ^ egu ^ eku ^ emu ^ esu;

        // thetaRhoPiChiIotaPrepareTheta(round+1, E, A)
        let da2 = bcu ^ rol(bce, 1);
        let de2 = bca ^ rol(bci, 1);
        let di2 = bce ^ rol(bco, 1);
        let do2 = bci ^ rol(bcu, 1);
        let du2 = bco ^ rol(bca, 1);

        eba ^= da2;
        bca = eba;
        ege ^= de2;
        bce = rol(ege, 44);
        eki ^= di2;
        bci = rol(eki, 43);
        emo ^= do2;
        bco = rol(emo, 21);
        esu ^= du2;
        bcu = rol(esu, 14);
        aba = bca ^ ((!bce) & bci);
        aba ^= KECCAK_F_ROUND_CONSTANTS[round + 1];
        abe = bce ^ ((!bci) & bco);
        abi = bci ^ ((!bco) & bcu);
        abo = bco ^ ((!bcu) & bca);
        abu = bcu ^ ((!bca) & bce);

        ebo ^= do2;
        bca = rol(ebo, 28);
        egu ^= du2;
        bce = rol(egu, 20);
        eka ^= da2;
        bci = rol(eka, 3);
        eme ^= de2;
        bco = rol(eme, 45);
        esi ^= di2;
        bcu = rol(esi, 61);
        aga = bca ^ ((!bce) & bci);
        age = bce ^ ((!bci) & bco);
        agi = bci ^ ((!bco) & bcu);
        ago = bco ^ ((!bcu) & bca);
        agu = bcu ^ ((!bca) & bce);

        ebe ^= de2;
        bca = rol(ebe, 1);
        egi ^= di2;
        bce = rol(egi, 6);
        eko ^= do2;
        bci = rol(eko, 25);
        emu ^= du2;
        bco = rol(emu, 8);
        esa ^= da2;
        bcu = rol(esa, 18);
        aka = bca ^ ((!bce) & bci);
        ake = bce ^ ((!bci) & bco);
        aki = bci ^ ((!bco) & bcu);
        ako = bco ^ ((!bcu) & bca);
        aku = bcu ^ ((!bca) & bce);

        ebu ^= du2;
        bca = rol(ebu, 27);
        ega ^= da2;
        bce = rol(ega, 36);
        eke ^= de2;
        bci = rol(eke, 10);
        emi ^= di2;
        bco = rol(emi, 15);
        eso ^= do2;
        bcu = rol(eso, 56);
        ama = bca ^ ((!bce) & bci);
        ame = bce ^ ((!bci) & bco);
        ami = bci ^ ((!bco) & bcu);
        amo = bco ^ ((!bcu) & bca);
        amu = bcu ^ ((!bca) & bce);

        ebi ^= di2;
        bca = rol(ebi, 62);
        ego ^= do2;
        bce = rol(ego, 55);
        eku ^= du2;
        bci = rol(eku, 39);
        ema ^= da2;
        bco = rol(ema, 41);
        ese ^= de2;
        bcu = rol(ese, 2);
        asa = bca ^ ((!bce) & bci);
        ase = bce ^ ((!bci) & bco);
        asi = bci ^ ((!bco) & bcu);
        aso = bco ^ ((!bcu) & bca);
        asu = bcu ^ ((!bca) & bce);

        round += 2;
    }

    // copyToState(state, A)
    state[0] = aba;
    state[1] = abe;
    state[2] = abi;
    state[3] = abo;
    state[4] = abu;
    state[5] = aga;
    state[6] = age;
    state[7] = agi;
    state[8] = ago;
    state[9] = agu;
    state[10] = aka;
    state[11] = ake;
    state[12] = aki;
    state[13] = ako;
    state[14] = aku;
    state[15] = ama;
    state[16] = ame;
    state[17] = ami;
    state[18] = amo;
    state[19] = amu;
    state[20] = asa;
    state[21] = ase;
    state[22] = asi;
    state[23] = aso;
    state[24] = asu;
}

/*************************************************
 * Name:        keccak_absorb
 *
 * Description: Absorb step of Keccak;
 *              non-incremental, starts by zeroeing the state.
 **************************************************/
fn keccak_absorb(s: &mut [u64], r: usize, m: &[u8], p: u8) {
    let mut t = [0u8; 200];

    /* Zero state */
    for i in 0..25 {
        s[i] = 0;
    }

    let mut off: usize = 0;
    let mut mlen: usize = m.len();

    while mlen >= r {
        for i in 0..(r / 8) {
            s[i] ^= load64(&m[off + 8 * i..]);
        }

        keccak_f1600_state_permute(s);
        mlen -= r;
        off += r;
    }

    for i in 0..r {
        t[i] = 0;
    }
    for i in 0..mlen {
        t[i] = m[off + i];
    }
    t[mlen] = p;
    t[r - 1] |= 128;
    for i in 0..(r / 8) {
        s[i] ^= load64(&t[8 * i..]);
    }
}

/*************************************************
 * Name:        keccak_squeezeblocks
 *
 * Description: Squeeze step of Keccak. Squeezes full blocks of r bytes each.
 *              Modifies the state. Can be called multiple times to keep
 *              squeezing, i.e., is incremental.
 **************************************************/
fn keccak_squeezeblocks(h: &mut [u8], mut nblocks: usize, s: &mut [u64], r: usize) {
    let mut off: usize = 0;
    while nblocks > 0 {
        keccak_f1600_state_permute(s);
        for i in 0..(r >> 3) {
            let v = s[i];
            store64(&mut h[off + 8 * i..], v);
        }
        off += r;
        nblocks -= 1;
    }
}

/*************************************************
 * Name:        keccak_inc_init
 *
 * Description: Initializes the incremental Keccak state to zero.
 **************************************************/
fn keccak_inc_init(s_inc: &mut [u64]) {
    for i in 0..25 {
        s_inc[i] = 0;
    }
    s_inc[25] = 0;
}

/*************************************************
 * Name:        keccak_inc_absorb
 *
 * Description: Incremental keccak absorb
 *              Preceded by keccak_inc_init, succeeded by keccak_inc_finalize
 **************************************************/
fn keccak_inc_absorb(s_inc: &mut [u64], r: usize, m: &[u8]) {
    let mut off: usize = 0;
    let mut mlen: u64 = m.len() as u64;

    /* Recall that s_inc[25] is the non-absorbed bytes xored into the state */
    while mlen + s_inc[25] >= r as u64 {
        let n = r as u64 - s_inc[25];
        let mut i: u64 = 0;
        while i < n {
            /* Take the i'th byte from message
               xor with the s_inc[25] + i'th byte of the state; little-endian */
            let idx = s_inc[25] + i;
            s_inc[(idx >> 3) as usize] ^=
                (m[off + i as usize] as u64) << (8 * (idx & 0x07));
            i += 1;
        }
        mlen -= n;
        off += n as usize;
        s_inc[25] = 0;

        keccak_f1600_state_permute(s_inc);
    }

    let mut i: u64 = 0;
    while i < mlen {
        let idx = s_inc[25] + i;
        s_inc[(idx >> 3) as usize] ^= (m[off + i as usize] as u64) << (8 * (idx & 0x07));
        i += 1;
    }
    s_inc[25] += mlen;
}

/*************************************************
 * Name:        keccak_inc_finalize
 *
 * Description: Finalizes Keccak absorb phase, prepares for squeezing
 **************************************************/
fn keccak_inc_finalize(s_inc: &mut [u64], r: usize, p: u8) {
    /* After keccak_inc_absorb, we are guaranteed that s_inc[25] < r,
       so we can always use one more byte for p in the current state. */
    let c = s_inc[25];
    s_inc[(c >> 3) as usize] ^= (p as u64) << (8 * (c & 0x07));
    s_inc[(r - 1) >> 3] ^= 128u64 << (8 * ((r - 1) & 0x07));
    s_inc[25] = 0;
}

/*************************************************
 * Name:        keccak_inc_squeeze
 *
 * Description: Incremental Keccak squeeze; can be called on byte-level
 **************************************************/
fn keccak_inc_squeeze(h: &mut [u8], mut outlen: usize, s_inc: &mut [u64], r: usize) {
    let mut hoff: usize = 0;

    /* First consume any bytes we still have sitting around */
    let mut i: u64 = 0;
    while (i as usize) < outlen && i < s_inc[25] {
        /* There are s_inc[25] bytes left, so r - s_inc[25] is the first
           available byte. We consume from there, i.e., up to r. */
        let idx = r as u64 - s_inc[25] + i;
        h[hoff + i as usize] = (s_inc[(idx >> 3) as usize] >> (8 * (idx & 0x07))) as u8;
        i += 1;
    }
    hoff += i as usize;
    outlen -= i as usize;
    s_inc[25] -= i;

    /* Then squeeze the remaining necessary blocks */
    while outlen > 0 {
        keccak_f1600_state_permute(s_inc);

        let mut i: usize = 0;
        while i < outlen && i < r {
            h[hoff + i] = (s_inc[i >> 3] >> (8 * (i & 0x07))) as u8;
            i += 1;
        }
        hoff += i;
        outlen -= i;
        s_inc[25] = (r - i) as u64;
    }
}

// ---------------------------------------------------------------------------
// SHAKE128
// ---------------------------------------------------------------------------

pub fn shake128_absorb(s: &mut [u64], input: &[u8]) {
    keccak_absorb(s, SHAKE128_RATE, input, 0x1F);
}

pub fn shake128_squeezeblocks(output: &mut [u8], nblocks: usize, s: &mut [u64]) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE128_RATE);
}

pub fn shake128_inc_init(s_inc: &mut [u64]) {
    keccak_inc_init(s_inc);
}

pub fn shake128_inc_absorb(s_inc: &mut [u64], input: &[u8]) {
    keccak_inc_absorb(s_inc, SHAKE128_RATE, input);
}

pub fn shake128_inc_finalize(s_inc: &mut [u64]) {
    keccak_inc_finalize(s_inc, SHAKE128_RATE, 0x1F);
}

pub fn shake128_inc_squeeze(output: &mut [u8], s_inc: &mut [u64]) {
    let outlen = output.len();
    keccak_inc_squeeze(output, outlen, s_inc, SHAKE128_RATE);
}

/*************************************************
 * Name:        shake128
 *
 * Description: SHAKE128 XOF with non-incremental API
 **************************************************/
pub fn shake128(output: &mut [u8], input: &[u8]) {
    let mut outlen = output.len();
    let nblocks = outlen / SHAKE128_RATE;
    let mut t = [0u8; SHAKE128_RATE];
    let mut s = [0u64; 25];

    shake128_absorb(&mut s, input);
    shake128_squeezeblocks(&mut output[..nblocks * SHAKE128_RATE], nblocks, &mut s);

    let off = nblocks * SHAKE128_RATE;
    outlen -= nblocks * SHAKE128_RATE;

    if outlen != 0 {
        shake128_squeezeblocks(&mut t, 1, &mut s);
        for i in 0..outlen {
            output[off + i] = t[i];
        }
    }
}

// ---------------------------------------------------------------------------
// SHAKE256
// ---------------------------------------------------------------------------

pub fn shake256_inc_init(s_inc: &mut [u64]) {
    keccak_inc_init(s_inc);
}

pub fn shake256_inc_absorb(s_inc: &mut [u64], input: &[u8]) {
    keccak_inc_absorb(s_inc, SHAKE256_RATE, input);
}

pub fn shake256_inc_finalize(s_inc: &mut [u64]) {
    keccak_inc_finalize(s_inc, SHAKE256_RATE, 0x1F);
}

pub fn shake256_inc_squeeze(output: &mut [u8], s_inc: &mut [u64]) {
    let outlen = output.len();
    keccak_inc_squeeze(output, outlen, s_inc, SHAKE256_RATE);
}

/*************************************************
 * Name:        shake256_absorb
 *
 * Description: Absorb step of the SHAKE256 XOF.
 *              non-incremental, starts by zeroeing the state.
 **************************************************/
pub fn shake256_absorb(s: &mut [u64], input: &[u8]) {
    keccak_absorb(s, SHAKE256_RATE, input, 0x1F);
}

/*************************************************
 * Name:        shake256_squeezeblocks
 *
 * Description: Squeeze step of SHAKE256 XOF. Squeezes full blocks of
 *              SHAKE256_RATE bytes each. Modifies the state. Can be called
 *              multiple times to keep squeezing, i.e., is incremental.
 **************************************************/
pub fn shake256_squeezeblocks(output: &mut [u8], nblocks: usize, s: &mut [u64]) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE256_RATE);
}

/*************************************************
 * Name:        shake256
 *
 * Description: SHAKE256 XOF with non-incremental API
 **************************************************/
pub fn shake256(output: &mut [u8], input: &[u8]) {
    let mut outlen = output.len();
    let nblocks = outlen / SHAKE256_RATE;
    let mut t = [0u8; SHAKE256_RATE];
    let mut s = [0u64; 25];

    shake256_absorb(&mut s, input);
    shake256_squeezeblocks(&mut output[..nblocks * SHAKE256_RATE], nblocks, &mut s);

    let off = nblocks * SHAKE256_RATE;
    outlen -= nblocks * SHAKE256_RATE;

    if outlen != 0 {
        shake256_squeezeblocks(&mut t, 1, &mut s);
        for i in 0..outlen {
            output[off + i] = t[i];
        }
    }
}

// ---------------------------------------------------------------------------
// SHA3-256
// ---------------------------------------------------------------------------

pub fn sha3_256_inc_init(s_inc: &mut [u64]) {
    keccak_inc_init(s_inc);
}

pub fn sha3_256_inc_absorb(s_inc: &mut [u64], input: &[u8]) {
    keccak_inc_absorb(s_inc, SHA3_256_RATE, input);
}

pub fn sha3_256_inc_finalize(output: &mut [u8], s_inc: &mut [u64]) {
    let mut t = [0u8; SHA3_256_RATE];
    keccak_inc_finalize(s_inc, SHA3_256_RATE, 0x06);

    keccak_squeezeblocks(&mut t, 1, s_inc, SHA3_256_RATE);

    for i in 0..32 {
        output[i] = t[i];
    }
}

pub fn sha3_256(output: &mut [u8], input: &[u8]) {
    let mut s = [0u64; 25];
    let mut t = [0u8; SHA3_256_RATE];

    /* Absorb input */
    keccak_absorb(&mut s, SHA3_256_RATE, input, 0x06);

    /* Squeeze output */
    keccak_squeezeblocks(&mut t, 1, &mut s, SHA3_256_RATE);

    for i in 0..32 {
        output[i] = t[i];
    }
}

// ---------------------------------------------------------------------------
// SHA3-512
// ---------------------------------------------------------------------------

pub fn sha3_512_inc_init(s_inc: &mut [u64]) {
    keccak_inc_init(s_inc);
}

pub fn sha3_512_inc_absorb(s_inc: &mut [u64], input: &[u8]) {
    keccak_inc_absorb(s_inc, SHA3_512_RATE, input);
}

pub fn sha3_512_inc_finalize(output: &mut [u8], s_inc: &mut [u64]) {
    let mut t = [0u8; SHA3_512_RATE];
    keccak_inc_finalize(s_inc, SHA3_512_RATE, 0x06);

    keccak_squeezeblocks(&mut t, 1, s_inc, SHA3_512_RATE);

    for i in 0..64 {
        output[i] = t[i];
    }
}

pub fn sha3_512(output: &mut [u8], input: &[u8]) {
    let mut s = [0u64; 25];
    let mut t = [0u8; SHA3_512_RATE];

    /* Absorb input */
    keccak_absorb(&mut s, SHA3_512_RATE, input, 0x06);

    /* Squeeze output */
    keccak_squeezeblocks(&mut t, 1, &mut s, SHA3_512_RATE);

    for i in 0..64 {
        output[i] = t[i];
    }
}
