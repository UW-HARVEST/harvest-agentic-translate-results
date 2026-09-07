/* Based on the public domain implementation in
 * crypto_hash/keccakc512/simple/ from http://bench.cr.yp.to/supercop.html
 * by Ronny Van Keer
 * and the public domain "TweetFips202" implementation
 * from https://twitter.com/tweetfips202
 * by Gilles Van Assche, Daniel J. Bernstein, and Peter Schwabe */

// Bit-exact translation of fips202.c (SHAKE256 family) into Rust.

#[allow(dead_code)]
const NROUNDS: usize = 24;

#[allow(non_snake_case)]
#[inline(always)]
fn ROL(a: u64, offset: u32) -> u64 {
    a.rotate_left(offset)
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
    let mut i: usize = 0;
    while i < 8 {
        r |= (x[i] as u64) << (8 * i);
        i += 1;
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
    let mut i: usize = 0;
    while i < 8 {
        x[i] = (u >> (8 * i)) as u8;
        i += 1;
    }
}

/* Keccak round constants */
#[allow(non_upper_case_globals)]
static KeccakF_RoundConstants: [u64; NROUNDS] = [
    0x0000000000000001u64,
    0x0000000000008082u64,
    0x800000000000808au64,
    0x8000000080008000u64,
    0x000000000000808bu64,
    0x0000000080000001u64,
    0x8000000080008081u64,
    0x8000000000008009u64,
    0x000000000000008au64,
    0x0000000000000088u64,
    0x0000000080008009u64,
    0x000000008000000au64,
    0x000000008000808bu64,
    0x800000000000008bu64,
    0x8000000000008089u64,
    0x8000000000008003u64,
    0x8000000000008002u64,
    0x8000000000000080u64,
    0x000000000000800au64,
    0x800000008000000au64,
    0x8000000080008081u64,
    0x8000000000008080u64,
    0x0000000080000001u64,
    0x8000000080008008u64,
];

/*************************************************
 * Name:        KeccakF1600_StatePermute
 *
 * Description: The Keccak F1600 Permutation
 *
 * Arguments:   - state: input/output Keccak state (first 25 words used)
 **************************************************/
#[allow(non_snake_case)]
fn KeccakF1600_StatePermute(state: &mut [u64]) {
    let mut round: usize;

    let (mut Aba, mut Abe, mut Abi, mut Abo, mut Abu): (u64, u64, u64, u64, u64);
    let (mut Aga, mut Age, mut Agi, mut Ago, mut Agu): (u64, u64, u64, u64, u64);
    let (mut Aka, mut Ake, mut Aki, mut Ako, mut Aku): (u64, u64, u64, u64, u64);
    let (mut Ama, mut Ame, mut Ami, mut Amo, mut Amu): (u64, u64, u64, u64, u64);
    let (mut Asa, mut Ase, mut Asi, mut Aso, mut Asu): (u64, u64, u64, u64, u64);
    let (mut BCa, mut BCe, mut BCi, mut BCo, mut BCu): (u64, u64, u64, u64, u64);
    let (mut Da, mut De, mut Di, mut Do, mut Du): (u64, u64, u64, u64, u64);
    let (mut Eba, mut Ebe, mut Ebi, mut Ebo, mut Ebu): (u64, u64, u64, u64, u64);
    let (mut Ega, mut Ege, mut Egi, mut Ego, mut Egu): (u64, u64, u64, u64, u64);
    let (mut Eka, mut Eke, mut Eki, mut Eko, mut Eku): (u64, u64, u64, u64, u64);
    let (mut Ema, mut Eme, mut Emi, mut Emo, mut Emu): (u64, u64, u64, u64, u64);
    let (mut Esa, mut Ese, mut Esi, mut Eso, mut Esu): (u64, u64, u64, u64, u64);

    // copyFromState(A, state)
    Aba = state[0];
    Abe = state[1];
    Abi = state[2];
    Abo = state[3];
    Abu = state[4];
    Aga = state[5];
    Age = state[6];
    Agi = state[7];
    Ago = state[8];
    Agu = state[9];
    Aka = state[10];
    Ake = state[11];
    Aki = state[12];
    Ako = state[13];
    Aku = state[14];
    Ama = state[15];
    Ame = state[16];
    Ami = state[17];
    Amo = state[18];
    Amu = state[19];
    Asa = state[20];
    Ase = state[21];
    Asi = state[22];
    Aso = state[23];
    Asu = state[24];

    round = 0;
    while round < NROUNDS {
        //    prepareTheta
        BCa = Aba ^ Aga ^ Aka ^ Ama ^ Asa;
        BCe = Abe ^ Age ^ Ake ^ Ame ^ Ase;
        BCi = Abi ^ Agi ^ Aki ^ Ami ^ Asi;
        BCo = Abo ^ Ago ^ Ako ^ Amo ^ Aso;
        BCu = Abu ^ Agu ^ Aku ^ Amu ^ Asu;

        // thetaRhoPiChiIotaPrepareTheta(round  , A, E)
        Da = BCu ^ ROL(BCe, 1);
        De = BCa ^ ROL(BCi, 1);
        Di = BCe ^ ROL(BCo, 1);
        Do = BCi ^ ROL(BCu, 1);
        Du = BCo ^ ROL(BCa, 1);

        Aba ^= Da;
        BCa = Aba;
        Age ^= De;
        BCe = ROL(Age, 44);
        Aki ^= Di;
        BCi = ROL(Aki, 43);
        Amo ^= Do;
        BCo = ROL(Amo, 21);
        Asu ^= Du;
        BCu = ROL(Asu, 14);
        Eba = BCa ^ ((!BCe) & BCi);
        Eba ^= KeccakF_RoundConstants[round];
        Ebe = BCe ^ ((!BCi) & BCo);
        Ebi = BCi ^ ((!BCo) & BCu);
        Ebo = BCo ^ ((!BCu) & BCa);
        Ebu = BCu ^ ((!BCa) & BCe);

        Abo ^= Do;
        BCa = ROL(Abo, 28);
        Agu ^= Du;
        BCe = ROL(Agu, 20);
        Aka ^= Da;
        BCi = ROL(Aka, 3);
        Ame ^= De;
        BCo = ROL(Ame, 45);
        Asi ^= Di;
        BCu = ROL(Asi, 61);
        Ega = BCa ^ ((!BCe) & BCi);
        Ege = BCe ^ ((!BCi) & BCo);
        Egi = BCi ^ ((!BCo) & BCu);
        Ego = BCo ^ ((!BCu) & BCa);
        Egu = BCu ^ ((!BCa) & BCe);

        Abe ^= De;
        BCa = ROL(Abe, 1);
        Agi ^= Di;
        BCe = ROL(Agi, 6);
        Ako ^= Do;
        BCi = ROL(Ako, 25);
        Amu ^= Du;
        BCo = ROL(Amu, 8);
        Asa ^= Da;
        BCu = ROL(Asa, 18);
        Eka = BCa ^ ((!BCe) & BCi);
        Eke = BCe ^ ((!BCi) & BCo);
        Eki = BCi ^ ((!BCo) & BCu);
        Eko = BCo ^ ((!BCu) & BCa);
        Eku = BCu ^ ((!BCa) & BCe);

        Abu ^= Du;
        BCa = ROL(Abu, 27);
        Aga ^= Da;
        BCe = ROL(Aga, 36);
        Ake ^= De;
        BCi = ROL(Ake, 10);
        Ami ^= Di;
        BCo = ROL(Ami, 15);
        Aso ^= Do;
        BCu = ROL(Aso, 56);
        Ema = BCa ^ ((!BCe) & BCi);
        Eme = BCe ^ ((!BCi) & BCo);
        Emi = BCi ^ ((!BCo) & BCu);
        Emo = BCo ^ ((!BCu) & BCa);
        Emu = BCu ^ ((!BCa) & BCe);

        Abi ^= Di;
        BCa = ROL(Abi, 62);
        Ago ^= Do;
        BCe = ROL(Ago, 55);
        Aku ^= Du;
        BCi = ROL(Aku, 39);
        Ama ^= Da;
        BCo = ROL(Ama, 41);
        Ase ^= De;
        BCu = ROL(Ase, 2);
        Esa = BCa ^ ((!BCe) & BCi);
        Ese = BCe ^ ((!BCi) & BCo);
        Esi = BCi ^ ((!BCo) & BCu);
        Eso = BCo ^ ((!BCu) & BCa);
        Esu = BCu ^ ((!BCa) & BCe);

        //    prepareTheta
        BCa = Eba ^ Ega ^ Eka ^ Ema ^ Esa;
        BCe = Ebe ^ Ege ^ Eke ^ Eme ^ Ese;
        BCi = Ebi ^ Egi ^ Eki ^ Emi ^ Esi;
        BCo = Ebo ^ Ego ^ Eko ^ Emo ^ Eso;
        BCu = Ebu ^ Egu ^ Eku ^ Emu ^ Esu;

        // thetaRhoPiChiIotaPrepareTheta(round+1, E, A)
        Da = BCu ^ ROL(BCe, 1);
        De = BCa ^ ROL(BCi, 1);
        Di = BCe ^ ROL(BCo, 1);
        Do = BCi ^ ROL(BCu, 1);
        Du = BCo ^ ROL(BCa, 1);

        Eba ^= Da;
        BCa = Eba;
        Ege ^= De;
        BCe = ROL(Ege, 44);
        Eki ^= Di;
        BCi = ROL(Eki, 43);
        Emo ^= Do;
        BCo = ROL(Emo, 21);
        Esu ^= Du;
        BCu = ROL(Esu, 14);
        Aba = BCa ^ ((!BCe) & BCi);
        Aba ^= KeccakF_RoundConstants[round + 1];
        Abe = BCe ^ ((!BCi) & BCo);
        Abi = BCi ^ ((!BCo) & BCu);
        Abo = BCo ^ ((!BCu) & BCa);
        Abu = BCu ^ ((!BCa) & BCe);

        Ebo ^= Do;
        BCa = ROL(Ebo, 28);
        Egu ^= Du;
        BCe = ROL(Egu, 20);
        Eka ^= Da;
        BCi = ROL(Eka, 3);
        Eme ^= De;
        BCo = ROL(Eme, 45);
        Esi ^= Di;
        BCu = ROL(Esi, 61);
        Aga = BCa ^ ((!BCe) & BCi);
        Age = BCe ^ ((!BCi) & BCo);
        Agi = BCi ^ ((!BCo) & BCu);
        Ago = BCo ^ ((!BCu) & BCa);
        Agu = BCu ^ ((!BCa) & BCe);

        Ebe ^= De;
        BCa = ROL(Ebe, 1);
        Egi ^= Di;
        BCe = ROL(Egi, 6);
        Eko ^= Do;
        BCi = ROL(Eko, 25);
        Emu ^= Du;
        BCo = ROL(Emu, 8);
        Esa ^= Da;
        BCu = ROL(Esa, 18);
        Aka = BCa ^ ((!BCe) & BCi);
        Ake = BCe ^ ((!BCi) & BCo);
        Aki = BCi ^ ((!BCo) & BCu);
        Ako = BCo ^ ((!BCu) & BCa);
        Aku = BCu ^ ((!BCa) & BCe);

        Ebu ^= Du;
        BCa = ROL(Ebu, 27);
        Ega ^= Da;
        BCe = ROL(Ega, 36);
        Eke ^= De;
        BCi = ROL(Eke, 10);
        Emi ^= Di;
        BCo = ROL(Emi, 15);
        Eso ^= Do;
        BCu = ROL(Eso, 56);
        Ama = BCa ^ ((!BCe) & BCi);
        Ame = BCe ^ ((!BCi) & BCo);
        Ami = BCi ^ ((!BCo) & BCu);
        Amo = BCo ^ ((!BCu) & BCa);
        Amu = BCu ^ ((!BCa) & BCe);

        Ebi ^= Di;
        BCa = ROL(Ebi, 62);
        Ego ^= Do;
        BCe = ROL(Ego, 55);
        Eku ^= Du;
        BCi = ROL(Eku, 39);
        Ema ^= Da;
        BCo = ROL(Ema, 41);
        Ese ^= De;
        BCu = ROL(Ese, 2);
        Asa = BCa ^ ((!BCe) & BCi);
        Ase = BCe ^ ((!BCi) & BCo);
        Asi = BCi ^ ((!BCo) & BCu);
        Aso = BCo ^ ((!BCu) & BCa);
        Asu = BCu ^ ((!BCa) & BCe);

        round += 2;
    }

    // copyToState(state, A)
    state[0] = Aba;
    state[1] = Abe;
    state[2] = Abi;
    state[3] = Abo;
    state[4] = Abu;
    state[5] = Aga;
    state[6] = Age;
    state[7] = Agi;
    state[8] = Ago;
    state[9] = Agu;
    state[10] = Aka;
    state[11] = Ake;
    state[12] = Aki;
    state[13] = Ako;
    state[14] = Aku;
    state[15] = Ama;
    state[16] = Ame;
    state[17] = Ami;
    state[18] = Amo;
    state[19] = Amu;
    state[20] = Asa;
    state[21] = Ase;
    state[22] = Asi;
    state[23] = Aso;
    state[24] = Asu;
}

/*************************************************
 * Name:        keccak_absorb
 *
 * Description: Absorb step of Keccak;
 *              non-incremental, starts by zeroeing the state.
 **************************************************/
fn keccak_absorb(s: &mut [u64], r: u32, m: &[u8], mut mlen: usize, p: u8) {
    let mut i: usize;
    let mut t: [u8; 200] = [0u8; 200];
    let r_us: usize = r as usize;

    // running pointer into the message
    let mut m_off: usize = 0;

    /* Zero state */
    i = 0;
    while i < 25 {
        s[i] = 0;
        i += 1;
    }

    while mlen >= r_us {
        i = 0;
        while i < r_us / 8 {
            s[i] ^= load64(&m[m_off + 8 * i..]);
            i += 1;
        }

        KeccakF1600_StatePermute(s);
        mlen -= r_us;
        m_off += r_us;
    }

    i = 0;
    while i < r_us {
        t[i] = 0;
        i += 1;
    }
    i = 0;
    while i < mlen {
        t[i] = m[m_off + i];
        i += 1;
    }
    t[i] = p;
    t[r_us - 1] |= 128;
    i = 0;
    while i < r_us / 8 {
        s[i] ^= load64(&t[8 * i..]);
        i += 1;
    }
}

/*************************************************
 * Name:        keccak_squeezeblocks
 *
 * Description: Squeeze step of Keccak. Squeezes full blocks of r bytes each.
 **************************************************/
fn keccak_squeezeblocks(h: &mut [u8], mut nblocks: usize, s: &mut [u64], r: u32) {
    let r_us: usize = r as usize;
    let mut h_off: usize = 0;

    while nblocks > 0 {
        KeccakF1600_StatePermute(s);
        let mut i: usize = 0;
        while i < (r_us >> 3) {
            store64(&mut h[h_off + 8 * i..], s[i]);
            i += 1;
        }
        h_off += r_us;
        nblocks -= 1;
    }
}

/*************************************************
 * Name:        keccak_inc_init
 *
 * Description: Initializes the incremental Keccak state to zero.
 **************************************************/
fn keccak_inc_init(s_inc: &mut [u64]) {
    let mut i: usize = 0;
    while i < 25 {
        s_inc[i] = 0;
        i += 1;
    }
    s_inc[25] = 0;
}

/*************************************************
 * Name:        keccak_inc_absorb
 *
 * Description: Incremental keccak absorb
 **************************************************/
fn keccak_inc_absorb(s_inc: &mut [u64], r: u32, m: &[u8], mut mlen: usize) {
    let mut i: u64;
    let r64: u64 = r as u64;
    let mut m_off: usize = 0;

    /* Recall that s_inc[25] is the non-absorbed bytes xored into the state */
    while (mlen as u64).wrapping_add(s_inc[25]) >= r64 {
        i = 0;
        while i < r64.wrapping_sub(s_inc[25]) {
            /* Take the i'th byte from message
               xor with the s_inc[25] + i'th byte of the state; little-endian */
            let idx = (s_inc[25].wrapping_add(i)) >> 3;
            let sh = 8u64.wrapping_mul((s_inc[25].wrapping_add(i)) & 0x07);
            s_inc[idx as usize] ^= (m[m_off + i as usize] as u64) << sh;
            i += 1;
        }
        mlen -= (r64.wrapping_sub(s_inc[25])) as usize;
        m_off += (r64.wrapping_sub(s_inc[25])) as usize;
        s_inc[25] = 0;

        KeccakF1600_StatePermute(&mut s_inc[..25]);
    }

    i = 0;
    while i < mlen as u64 {
        let idx = (s_inc[25].wrapping_add(i)) >> 3;
        let sh = 8u64.wrapping_mul((s_inc[25].wrapping_add(i)) & 0x07);
        s_inc[idx as usize] ^= (m[m_off + i as usize] as u64) << sh;
        i += 1;
    }
    s_inc[25] = s_inc[25].wrapping_add(mlen as u64);
}

/*************************************************
 * Name:        keccak_inc_finalize
 *
 * Description: Finalizes Keccak absorb phase, prepares for squeezing
 **************************************************/
fn keccak_inc_finalize(s_inc: &mut [u64], r: u32, p: u8) {
    let r64: u64 = r as u64;
    /* After keccak_inc_absorb, we are guaranteed that s_inc[25] < r,
       so we can always use one more byte for p in the current state. */
    {
        let idx = (s_inc[25] >> 3) as usize;
        let sh = 8u64.wrapping_mul(s_inc[25] & 0x07);
        s_inc[idx] ^= (p as u64) << sh;
    }
    {
        let idx = ((r64.wrapping_sub(1)) >> 3) as usize;
        let sh = 8u64.wrapping_mul((r64.wrapping_sub(1)) & 0x07);
        s_inc[idx] ^= 128u64 << sh;
    }
    s_inc[25] = 0;
}

/*************************************************
 * Name:        keccak_inc_squeeze
 *
 * Description: Incremental Keccak squeeze; can be called on byte-level
 **************************************************/
fn keccak_inc_squeeze(h: &mut [u8], mut outlen: usize, s_inc: &mut [u64], r: u32) {
    let mut i: u64;
    let r64: u64 = r as u64;
    let mut h_off: usize = 0;

    /* First consume any bytes we still have sitting around */
    i = 0;
    while (i < outlen as u64) && (i < s_inc[25]) {
        /* There are s_inc[25] bytes left, so r - s_inc[25] is the first
           available byte. We consume from there, i.e., up to r. */
        let pos = r64.wrapping_sub(s_inc[25]).wrapping_add(i);
        let idx = (pos >> 3) as usize;
        let sh = 8u64.wrapping_mul(pos & 0x07);
        h[h_off + i as usize] = (s_inc[idx] >> sh) as u8;
        i += 1;
    }
    h_off += i as usize;
    outlen -= i as usize;
    s_inc[25] = s_inc[25].wrapping_sub(i);

    /* Then squeeze the remaining necessary blocks */
    while outlen > 0 {
        KeccakF1600_StatePermute(&mut s_inc[..25]);

        i = 0;
        while (i < outlen as u64) && (i < r64) {
            let idx = (i >> 3) as usize;
            let sh = 8u64.wrapping_mul(i & 0x07);
            h[h_off + i as usize] = (s_inc[idx] >> sh) as u8;
            i += 1;
        }
        h_off += i as usize;
        outlen -= i as usize;
        s_inc[25] = r64.wrapping_sub(i);
    }
}

// ---------------------------------------------------------------------------
// Public Rust API
// ---------------------------------------------------------------------------

pub fn shake256_inc_init_rs(s_inc: &mut [u64; 26]) {
    keccak_inc_init(s_inc);
}

pub fn shake256_inc_absorb_rs(s_inc: &mut [u64; 26], input: &[u8]) {
    keccak_inc_absorb(s_inc, SHAKE256_RATE as u32, input, input.len());
}

pub fn shake256_inc_finalize_rs(s_inc: &mut [u64; 26]) {
    keccak_inc_finalize(s_inc, SHAKE256_RATE as u32, 0x1F);
}

pub fn shake256_inc_squeeze_rs(output: &mut [u8], s_inc: &mut [u64; 26]) {
    let outlen = output.len();
    keccak_inc_squeeze(output, outlen, s_inc, SHAKE256_RATE as u32);
}

pub fn shake256_absorb_rs(s: &mut [u64; 25], input: &[u8]) {
    keccak_absorb(s, SHAKE256_RATE as u32, input, input.len(), 0x1F);
}

pub fn shake256_squeezeblocks_rs(output: &mut [u8], nblocks: usize, s: &mut [u64; 25]) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE256_RATE as u32);
}

pub fn shake256_rs(output: &mut [u8], input: &[u8]) {
    let mut outlen: usize = output.len();
    let nblocks: usize = outlen / SHAKE256_RATE;
    let mut t: [u8; SHAKE256_RATE] = [0u8; SHAKE256_RATE];
    let mut s: [u64; 25] = [0u64; 25];

    shake256_absorb_rs(&mut s, input);
    shake256_squeezeblocks_rs(output, nblocks, &mut s);

    let mut output_off: usize = nblocks * SHAKE256_RATE;
    outlen -= nblocks * SHAKE256_RATE;

    if outlen != 0 {
        shake256_squeezeblocks_rs(&mut t, 1, &mut s);
        let mut i: usize = 0;
        while i < outlen {
            output[output_off + i] = t[i];
            i += 1;
        }
        let _ = &mut output_off;
    }
}

// ---------------------------------------------------------------------------
// C-ABI wrappers (linker names from fips202.h; no namespace macro applies).
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_init(s_inc: *mut u64) {
    let s = core::slice::from_raw_parts_mut(s_inc, 26);
    keccak_inc_init(s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize) {
    let s = core::slice::from_raw_parts_mut(s_inc, 26);
    let m = core::slice::from_raw_parts(input, inlen);
    keccak_inc_absorb(s, SHAKE256_RATE as u32, m, inlen);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_finalize(s_inc: *mut u64) {
    let s = core::slice::from_raw_parts_mut(s_inc, 26);
    keccak_inc_finalize(s, SHAKE256_RATE as u32, 0x1F);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_squeeze(output: *mut u8, outlen: usize, s_inc: *mut u64) {
    let s = core::slice::from_raw_parts_mut(s_inc, 26);
    let h = core::slice::from_raw_parts_mut(output, outlen);
    keccak_inc_squeeze(h, outlen, s, SHAKE256_RATE as u32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    let st = core::slice::from_raw_parts_mut(s, 25);
    let m = core::slice::from_raw_parts(input, inlen);
    keccak_absorb(st, SHAKE256_RATE as u32, m, inlen, 0x1F);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_squeezeblocks(output: *mut u8, nblocks: usize, s: *mut u64) {
    let st = core::slice::from_raw_parts_mut(s, 25);
    let h = core::slice::from_raw_parts_mut(output, nblocks * SHAKE256_RATE);
    keccak_squeezeblocks(h, nblocks, st, SHAKE256_RATE as u32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256(output: *mut u8, outlen: usize, input: *const u8, inlen: usize) {
    let mut outlen_m: usize = outlen;
    let nblocks: usize = outlen / SHAKE256_RATE;
    let mut t: [u8; SHAKE256_RATE] = [0u8; SHAKE256_RATE];
    let mut s: [u64; 25] = [0u64; 25];

    let m = core::slice::from_raw_parts(input, inlen);
    keccak_absorb(&mut s, SHAKE256_RATE as u32, m, inlen, 0x1F);

    {
        let h = core::slice::from_raw_parts_mut(output, nblocks * SHAKE256_RATE);
        keccak_squeezeblocks(h, nblocks, &mut s, SHAKE256_RATE as u32);
    }

    let output_off: usize = nblocks * SHAKE256_RATE;
    outlen_m -= nblocks * SHAKE256_RATE;

    if outlen_m != 0 {
        keccak_squeezeblocks(&mut t, 1, &mut s, SHAKE256_RATE as u32);
        let out_tail = core::slice::from_raw_parts_mut(output.add(output_off), outlen_m);
        let mut i: usize = 0;
        while i < outlen_m {
            out_tail[i] = t[i];
            i += 1;
        }
    }
}
