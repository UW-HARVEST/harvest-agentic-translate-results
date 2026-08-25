//! Translation of `lib/shake/src/fips202.c` (declarations in
//! `lib/shake/include/fips202.h`).
//!
//! > Based on the public domain implementation in
//! > `crypto_hash/keccakc512/simple/` from <http://bench.cr.yp.to/supercop.html>
//! > by Ronny Van Keer
//! > and the public domain "TweetFips202" implementation
//! > from <https://twitter.com/tweetfips202>
//! > by Gilles Van Assche, Daniel J. Bernstein, and Peter Schwabe
//!
//! The C file only *defines* the SHAKE256 family (the header also declares the
//! SHAKE128 / SHA3 entry points, but they have no definition here), plus the
//! `static` Keccak helpers.  Every expression, index, rotation offset and
//! round-constant index is transcribed verbatim so that the behaviour is
//! byte-identical.
//!
//! The incremental state is `[u64; 26]`: the first 25 words are the Keccak
//! state, `s_inc[25]` is the byte counter (bytes absorbed but not yet
//! permuted, resp. bytes squeezed but not yet consumed).

/// `#define NROUNDS 24`
const NROUNDS: usize = 24;

/// `#define SHAKE128_RATE 168`
pub const SHAKE128_RATE: u32 = 168;
/// `#define SHAKE256_RATE 136`
pub const SHAKE256_RATE: u32 = 136;
/// `#define SHA3_256_RATE 136`
pub const SHA3_256_RATE: u32 = 136;
/// `#define SHA3_512_RATE 72`
pub const SHA3_512_RATE: u32 = 72;

/// `#define ROL(a, offset) (((a) << (offset)) ^ ((a) >> (64 - (offset))))`
///
/// `offset` is always a non-zero compile-time constant in the code below, so
/// the `64 - offset` shift is always well defined.
#[inline(always)]
fn rol(a: u64, offset: u32) -> u64 {
    (a << offset) ^ (a >> (64 - offset))
}

/*************************************************
 * Name:        load64
 *
 * Description: Load 8 bytes into uint64_t in little-endian order
 *
 * Arguments:   - const uint8_t *x: pointer to input byte array
 *
 * Returns the loaded 64-bit unsigned integer
 **************************************************/
/// `static uint64_t load64(const uint8_t *x)`
unsafe fn load64(x: *const u8) -> u64 {
    let mut r: u64 = 0;
    let mut i: usize = 0;
    while i < 8 {
        r |= (*x.add(i) as u64) << (8 * i);
        i += 1;
    }

    r
}

/*************************************************
 * Name:        store64
 *
 * Description: Store a 64-bit integer to a byte array in little-endian order
 *
 * Arguments:   - uint8_t *x: pointer to the output byte array
 *              - uint64_t u: input 64-bit unsigned integer
 **************************************************/
/// `static void store64(uint8_t *x, uint64_t u)`
unsafe fn store64(x: *mut u8, u: u64) {
    let mut i: usize = 0;
    while i < 8 {
        *x.add(i) = (u >> (8 * i)) as u8;
        i += 1;
    }
}

/// Keccak round constants
///
/// `static const uint64_t KeccakF_RoundConstants[NROUNDS]`
static KeccakF_RoundConstants: [u64; NROUNDS] = [
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
/// `static void KeccakF1600_StatePermute(uint64_t *state)`
unsafe fn KeccakF1600_StatePermute(state: *mut u64) {
    let mut round: usize;

    // copyFromState(A, state)
    let mut Aba = *state.add(0);
    let mut Abe = *state.add(1);
    let mut Abi = *state.add(2);
    let mut Abo = *state.add(3);
    let mut Abu = *state.add(4);
    let mut Aga = *state.add(5);
    let mut Age = *state.add(6);
    let mut Agi = *state.add(7);
    let mut Ago = *state.add(8);
    let mut Agu = *state.add(9);
    let mut Aka = *state.add(10);
    let mut Ake = *state.add(11);
    let mut Aki = *state.add(12);
    let mut Ako = *state.add(13);
    let mut Aku = *state.add(14);
    let mut Ama = *state.add(15);
    let mut Ame = *state.add(16);
    let mut Ami = *state.add(17);
    let mut Amo = *state.add(18);
    let mut Amu = *state.add(19);
    let mut Asa = *state.add(20);
    let mut Ase = *state.add(21);
    let mut Asi = *state.add(22);
    let mut Aso = *state.add(23);
    let mut Asu = *state.add(24);

    round = 0;
    while round < NROUNDS {
        //    prepareTheta
        let mut BCa = Aba ^ Aga ^ Aka ^ Ama ^ Asa;
        let mut BCe = Abe ^ Age ^ Ake ^ Ame ^ Ase;
        let mut BCi = Abi ^ Agi ^ Aki ^ Ami ^ Asi;
        let mut BCo = Abo ^ Ago ^ Ako ^ Amo ^ Aso;
        let mut BCu = Abu ^ Agu ^ Aku ^ Amu ^ Asu;

        // thetaRhoPiChiIotaPrepareTheta(round  , A, E)
        let mut Da = BCu ^ rol(BCe, 1);
        let mut De = BCa ^ rol(BCi, 1);
        let mut Di = BCe ^ rol(BCo, 1);
        let mut Do = BCi ^ rol(BCu, 1);
        let mut Du = BCo ^ rol(BCa, 1);

        Aba ^= Da;
        BCa = Aba;
        Age ^= De;
        BCe = rol(Age, 44);
        Aki ^= Di;
        BCi = rol(Aki, 43);
        Amo ^= Do;
        BCo = rol(Amo, 21);
        Asu ^= Du;
        BCu = rol(Asu, 14);
        let mut Eba = BCa ^ ((!BCe) & BCi);
        Eba ^= KeccakF_RoundConstants[round];
        let mut Ebe = BCe ^ ((!BCi) & BCo);
        let mut Ebi = BCi ^ ((!BCo) & BCu);
        let mut Ebo = BCo ^ ((!BCu) & BCa);
        let mut Ebu = BCu ^ ((!BCa) & BCe);

        Abo ^= Do;
        BCa = rol(Abo, 28);
        Agu ^= Du;
        BCe = rol(Agu, 20);
        Aka ^= Da;
        BCi = rol(Aka, 3);
        Ame ^= De;
        BCo = rol(Ame, 45);
        Asi ^= Di;
        BCu = rol(Asi, 61);
        let mut Ega = BCa ^ ((!BCe) & BCi);
        let mut Ege = BCe ^ ((!BCi) & BCo);
        let mut Egi = BCi ^ ((!BCo) & BCu);
        let mut Ego = BCo ^ ((!BCu) & BCa);
        let mut Egu = BCu ^ ((!BCa) & BCe);

        Abe ^= De;
        BCa = rol(Abe, 1);
        Agi ^= Di;
        BCe = rol(Agi, 6);
        Ako ^= Do;
        BCi = rol(Ako, 25);
        Amu ^= Du;
        BCo = rol(Amu, 8);
        Asa ^= Da;
        BCu = rol(Asa, 18);
        let mut Eka = BCa ^ ((!BCe) & BCi);
        let mut Eke = BCe ^ ((!BCi) & BCo);
        let mut Eki = BCi ^ ((!BCo) & BCu);
        let mut Eko = BCo ^ ((!BCu) & BCa);
        let mut Eku = BCu ^ ((!BCa) & BCe);

        Abu ^= Du;
        BCa = rol(Abu, 27);
        Aga ^= Da;
        BCe = rol(Aga, 36);
        Ake ^= De;
        BCi = rol(Ake, 10);
        Ami ^= Di;
        BCo = rol(Ami, 15);
        Aso ^= Do;
        BCu = rol(Aso, 56);
        let mut Ema = BCa ^ ((!BCe) & BCi);
        let mut Eme = BCe ^ ((!BCi) & BCo);
        let mut Emi = BCi ^ ((!BCo) & BCu);
        let mut Emo = BCo ^ ((!BCu) & BCa);
        let mut Emu = BCu ^ ((!BCa) & BCe);

        Abi ^= Di;
        BCa = rol(Abi, 62);
        Ago ^= Do;
        BCe = rol(Ago, 55);
        Aku ^= Du;
        BCi = rol(Aku, 39);
        Ama ^= Da;
        BCo = rol(Ama, 41);
        Ase ^= De;
        BCu = rol(Ase, 2);
        let mut Esa = BCa ^ ((!BCe) & BCi);
        let mut Ese = BCe ^ ((!BCi) & BCo);
        let mut Esi = BCi ^ ((!BCo) & BCu);
        let mut Eso = BCo ^ ((!BCu) & BCa);
        let mut Esu = BCu ^ ((!BCa) & BCe);

        //    prepareTheta
        BCa = Eba ^ Ega ^ Eka ^ Ema ^ Esa;
        BCe = Ebe ^ Ege ^ Eke ^ Eme ^ Ese;
        BCi = Ebi ^ Egi ^ Eki ^ Emi ^ Esi;
        BCo = Ebo ^ Ego ^ Eko ^ Emo ^ Eso;
        BCu = Ebu ^ Egu ^ Eku ^ Emu ^ Esu;

        // thetaRhoPiChiIotaPrepareTheta(round+1, E, A)
        Da = BCu ^ rol(BCe, 1);
        De = BCa ^ rol(BCi, 1);
        Di = BCe ^ rol(BCo, 1);
        Do = BCi ^ rol(BCu, 1);
        Du = BCo ^ rol(BCa, 1);

        Eba ^= Da;
        BCa = Eba;
        Ege ^= De;
        BCe = rol(Ege, 44);
        Eki ^= Di;
        BCi = rol(Eki, 43);
        Emo ^= Do;
        BCo = rol(Emo, 21);
        Esu ^= Du;
        BCu = rol(Esu, 14);
        Aba = BCa ^ ((!BCe) & BCi);
        Aba ^= KeccakF_RoundConstants[round + 1];
        Abe = BCe ^ ((!BCi) & BCo);
        Abi = BCi ^ ((!BCo) & BCu);
        Abo = BCo ^ ((!BCu) & BCa);
        Abu = BCu ^ ((!BCa) & BCe);

        Ebo ^= Do;
        BCa = rol(Ebo, 28);
        Egu ^= Du;
        BCe = rol(Egu, 20);
        Eka ^= Da;
        BCi = rol(Eka, 3);
        Eme ^= De;
        BCo = rol(Eme, 45);
        Esi ^= Di;
        BCu = rol(Esi, 61);
        Aga = BCa ^ ((!BCe) & BCi);
        Age = BCe ^ ((!BCi) & BCo);
        Agi = BCi ^ ((!BCo) & BCu);
        Ago = BCo ^ ((!BCu) & BCa);
        Agu = BCu ^ ((!BCa) & BCe);

        Ebe ^= De;
        BCa = rol(Ebe, 1);
        Egi ^= Di;
        BCe = rol(Egi, 6);
        Eko ^= Do;
        BCi = rol(Eko, 25);
        Emu ^= Du;
        BCo = rol(Emu, 8);
        Esa ^= Da;
        BCu = rol(Esa, 18);
        Aka = BCa ^ ((!BCe) & BCi);
        Ake = BCe ^ ((!BCi) & BCo);
        Aki = BCi ^ ((!BCo) & BCu);
        Ako = BCo ^ ((!BCu) & BCa);
        Aku = BCu ^ ((!BCa) & BCe);

        Ebu ^= Du;
        BCa = rol(Ebu, 27);
        Ega ^= Da;
        BCe = rol(Ega, 36);
        Eke ^= De;
        BCi = rol(Eke, 10);
        Emi ^= Di;
        BCo = rol(Emi, 15);
        Eso ^= Do;
        BCu = rol(Eso, 56);
        Ama = BCa ^ ((!BCe) & BCi);
        Ame = BCe ^ ((!BCi) & BCo);
        Ami = BCi ^ ((!BCo) & BCu);
        Amo = BCo ^ ((!BCu) & BCa);
        Amu = BCu ^ ((!BCa) & BCe);

        Ebi ^= Di;
        BCa = rol(Ebi, 62);
        Ego ^= Do;
        BCe = rol(Ego, 55);
        Eku ^= Du;
        BCi = rol(Eku, 39);
        Ema ^= Da;
        BCo = rol(Ema, 41);
        Ese ^= De;
        BCu = rol(Ese, 2);
        Asa = BCa ^ ((!BCe) & BCi);
        Ase = BCe ^ ((!BCi) & BCo);
        Asi = BCi ^ ((!BCo) & BCu);
        Aso = BCo ^ ((!BCu) & BCa);
        Asu = BCu ^ ((!BCa) & BCe);

        round += 2;
    }

    // copyToState(state, A)
    *state.add(0) = Aba;
    *state.add(1) = Abe;
    *state.add(2) = Abi;
    *state.add(3) = Abo;
    *state.add(4) = Abu;
    *state.add(5) = Aga;
    *state.add(6) = Age;
    *state.add(7) = Agi;
    *state.add(8) = Ago;
    *state.add(9) = Agu;
    *state.add(10) = Aka;
    *state.add(11) = Ake;
    *state.add(12) = Aki;
    *state.add(13) = Ako;
    *state.add(14) = Aku;
    *state.add(15) = Ama;
    *state.add(16) = Ame;
    *state.add(17) = Ami;
    *state.add(18) = Amo;
    *state.add(19) = Amu;
    *state.add(20) = Asa;
    *state.add(21) = Ase;
    *state.add(22) = Asi;
    *state.add(23) = Aso;
    *state.add(24) = Asu;
}

/*************************************************
 * Name:        keccak_absorb
 *
 * Description: Absorb step of Keccak;
 *              non-incremental, starts by zeroeing the state.
 *
 * Arguments:   - uint64_t *s: pointer to (uninitialized) output Keccak state
 *              - uint32_t r: rate in bytes (e.g., 168 for SHAKE128)
 *              - const uint8_t *m: pointer to input to be absorbed into s
 *              - size_t mlen: length of input in bytes
 *              - uint8_t p: domain-separation byte for different
 *                                 Keccak-derived functions
 **************************************************/
/// `static void keccak_absorb(uint64_t *s, uint32_t r, const uint8_t *m,
///                            size_t mlen, uint8_t p)`
unsafe fn keccak_absorb(s: *mut u64, r: u32, m: *const u8, mlen: usize, p: u8) {
    let mut m = m;
    let mut mlen = mlen;
    let r = r as usize;

    let mut i: usize;
    let mut t = [0u8; 200];

    /* Zero state */
    i = 0;
    while i < 25 {
        *s.add(i) = 0;
        i += 1;
    }

    while mlen >= r {
        i = 0;
        while i < r / 8 {
            *s.add(i) ^= load64(m.add(8 * i));
            i += 1;
        }

        KeccakF1600_StatePermute(s);
        mlen -= r;
        m = m.add(r);
    }

    i = 0;
    while i < r {
        t[i] = 0;
        i += 1;
    }
    i = 0;
    while i < mlen {
        t[i] = *m.add(i);
        i += 1;
    }
    t[i] = p;
    t[r - 1] |= 128;
    i = 0;
    while i < r / 8 {
        *s.add(i) ^= load64(t.as_ptr().add(8 * i));
        i += 1;
    }
}

/*************************************************
 * Name:        keccak_squeezeblocks
 *
 * Description: Squeeze step of Keccak. Squeezes full blocks of r bytes each.
 *              Modifies the state. Can be called multiple times to keep
 *              squeezing, i.e., is incremental.
 *
 * Arguments:   - uint8_t *h: pointer to output blocks
 *              - size_t nblocks: number of blocks to be
 *                                                squeezed (written to h)
 *              - uint64_t *s: pointer to input/output Keccak state
 *              - uint32_t r: rate in bytes (e.g., 168 for SHAKE128)
 **************************************************/
/// `static void keccak_squeezeblocks(uint8_t *h, size_t nblocks,
///                                   uint64_t *s, uint32_t r)`
unsafe fn keccak_squeezeblocks(h: *mut u8, nblocks: usize, s: *mut u64, r: u32) {
    let mut h = h;
    let mut nblocks = nblocks;

    while nblocks > 0 {
        KeccakF1600_StatePermute(s);
        let mut i: usize = 0;
        while i < (r >> 3) as usize {
            store64(h.add(8 * i), *s.add(i));
            i += 1;
        }
        h = h.add(r as usize);
        nblocks -= 1;
    }
}

/*************************************************
 * Name:        keccak_inc_init
 *
 * Description: Initializes the incremental Keccak state to zero.
 *
 * Arguments:   - uint64_t *s_inc: pointer to input/output incremental state
 *                First 25 values represent Keccak state.
 *                26th value represents either the number of absorbed bytes
 *                that have not been permuted, or not-yet-squeezed bytes.
 **************************************************/
/// `static void keccak_inc_init(uint64_t *s_inc)`
unsafe fn keccak_inc_init(s_inc: *mut u64) {
    let mut i: usize;

    i = 0;
    while i < 25 {
        *s_inc.add(i) = 0;
        i += 1;
    }
    *s_inc.add(25) = 0;
}

/*************************************************
 * Name:        keccak_inc_absorb
 *
 * Description: Incremental keccak absorb
 *              Preceded by keccak_inc_init, succeeded by keccak_inc_finalize
 *
 * Arguments:   - uint64_t *s_inc: pointer to input/output incremental state
 *                First 25 values represent Keccak state.
 *                26th value represents either the number of absorbed bytes
 *                that have not been permuted, or not-yet-squeezed bytes.
 *              - uint32_t r: rate in bytes (e.g., 168 for SHAKE128)
 *              - const uint8_t *m: pointer to input to be absorbed into s
 *              - size_t mlen: length of input in bytes
 **************************************************/
/// `static void keccak_inc_absorb(uint64_t *s_inc, uint32_t r, const uint8_t *m,
///                                size_t mlen)`
unsafe fn keccak_inc_absorb(s_inc: *mut u64, r: u32, m: *const u8, mlen: usize) {
    let mut m = m;
    let mut mlen = mlen;

    let mut i: u64;

    /* Recall that s_inc[25] is the non-absorbed bytes xored into the state */
    while (mlen as u64).wrapping_add(*s_inc.add(25)) >= r as u64 {
        i = 0;
        while i < (r as u64).wrapping_sub(*s_inc.add(25)) {
            /* Take the i'th byte from message
            xor with the s_inc[25] + i'th byte of the state; little-endian */
            let off = (*s_inc.add(25)).wrapping_add(i);
            *s_inc.add((off >> 3) as usize) ^= (*m.add(i as usize) as u64) << (8 * (off & 0x07));
            i += 1;
        }
        let step = (r as u64).wrapping_sub(*s_inc.add(25));
        mlen -= step as usize;
        m = m.add(step as usize);
        *s_inc.add(25) = 0;

        KeccakF1600_StatePermute(s_inc);
    }

    i = 0;
    while i < mlen as u64 {
        let off = (*s_inc.add(25)).wrapping_add(i);
        *s_inc.add((off >> 3) as usize) ^= (*m.add(i as usize) as u64) << (8 * (off & 0x07));
        i += 1;
    }
    *s_inc.add(25) = (*s_inc.add(25)).wrapping_add(mlen as u64);
}

/*************************************************
 * Name:        keccak_inc_finalize
 *
 * Description: Finalizes Keccak absorb phase, prepares for squeezing
 *
 * Arguments:   - uint64_t *s_inc: pointer to input/output incremental state
 *                First 25 values represent Keccak state.
 *                26th value represents either the number of absorbed bytes
 *                that have not been permuted, or not-yet-squeezed bytes.
 *              - uint32_t r: rate in bytes (e.g., 168 for SHAKE128)
 *              - uint8_t p: domain-separation byte for different
 *                                 Keccak-derived functions
 **************************************************/
/// `static void keccak_inc_finalize(uint64_t *s_inc, uint32_t r, uint8_t p)`
unsafe fn keccak_inc_finalize(s_inc: *mut u64, r: u32, p: u8) {
    /* After keccak_inc_absorb, we are guaranteed that s_inc[25] < r,
    so we can always use one more byte for p in the current state. */
    let n = *s_inc.add(25);
    *s_inc.add((n >> 3) as usize) ^= (p as u64) << (8 * (n & 0x07));
    *s_inc.add(((r - 1) >> 3) as usize) ^= 128u64 << (8 * ((r - 1) & 0x07));
    *s_inc.add(25) = 0;
}

/*************************************************
 * Name:        keccak_inc_squeeze
 *
 * Description: Incremental Keccak squeeze; can be called on byte-level
 *
 * Arguments:   - uint8_t *h: pointer to output bytes
 *              - size_t outlen: number of bytes to be squeezed
 *              - uint64_t *s_inc: pointer to input/output incremental state
 *                First 25 values represent Keccak state.
 *                26th value represents either the number of absorbed bytes
 *                that have not been permuted, or not-yet-squeezed bytes.
 *              - uint32_t r: rate in bytes (e.g., 168 for SHAKE128)
 **************************************************/
/// `static void keccak_inc_squeeze(uint8_t *h, size_t outlen,
///                                 uint64_t *s_inc, uint32_t r)`
unsafe fn keccak_inc_squeeze(h: *mut u8, outlen: usize, s_inc: *mut u64, r: u32) {
    let mut h = h;
    let mut outlen = outlen;

    let mut i: usize;

    /* First consume any bytes we still have sitting around */
    i = 0;
    while (i as u64) < outlen as u64 && (i as u64) < *s_inc.add(25) {
        /* There are s_inc[25] bytes left, so r - s_inc[25] is the first
        available byte. We consume from there, i.e., up to r. */
        let off = (r as u64)
            .wrapping_sub(*s_inc.add(25))
            .wrapping_add(i as u64);
        *h.add(i) = (*s_inc.add((off >> 3) as usize) >> (8 * (off & 0x07))) as u8;
        i += 1;
    }
    h = h.add(i);
    outlen -= i;
    *s_inc.add(25) = (*s_inc.add(25)).wrapping_sub(i as u64);

    /* Then squeeze the remaining necessary blocks */
    while outlen > 0 {
        KeccakF1600_StatePermute(s_inc);

        i = 0;
        while i < outlen && i < r as usize {
            *h.add(i) = (*s_inc.add(i >> 3) >> (8 * (i & 0x07))) as u8;
            i += 1;
        }
        h = h.add(i);
        outlen -= i;
        *s_inc.add(25) = (r as u64).wrapping_sub(i as u64);
    }
}

/// `void shake256_inc_init(uint64_t *s_inc)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_init(s_inc: *mut u64) {
    keccak_inc_init(s_inc);
}

/// `void shake256_inc_absorb(uint64_t *s_inc, const uint8_t *input, size_t inlen)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize) {
    keccak_inc_absorb(s_inc, SHAKE256_RATE, input, inlen);
}

/// `void shake256_inc_finalize(uint64_t *s_inc)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_finalize(s_inc: *mut u64) {
    keccak_inc_finalize(s_inc, SHAKE256_RATE, 0x1F);
}

/// `void shake256_inc_squeeze(uint8_t *output, size_t outlen, uint64_t *s_inc)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_squeeze(output: *mut u8, outlen: usize, s_inc: *mut u64) {
    keccak_inc_squeeze(output, outlen, s_inc, SHAKE256_RATE);
}

/*************************************************
 * Name:        shake256_absorb
 *
 * Description: Absorb step of the SHAKE256 XOF.
 *              non-incremental, starts by zeroeing the state.
 *
 * Arguments:   - uint64_t *s: pointer to (uninitialized) output Keccak state
 *              - const uint8_t *input: pointer to input to be absorbed
 *                                            into s
 *              - size_t inlen: length of input in bytes
 **************************************************/
/// `void shake256_absorb(uint64_t *s, const uint8_t *input, size_t inlen)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    keccak_absorb(s, SHAKE256_RATE, input, inlen, 0x1F);
}

/*************************************************
 * Name:        shake256_squeezeblocks
 *
 * Description: Squeeze step of SHAKE256 XOF. Squeezes full blocks of
 *              SHAKE256_RATE bytes each. Modifies the state. Can be called
 *              multiple times to keep squeezing, i.e., is incremental.
 *
 * Arguments:   - uint8_t *output: pointer to output blocks
 *              - size_t nblocks: number of blocks to be squeezed
 *                                (written to output)
 *              - uint64_t *s: pointer to input/output Keccak state
 **************************************************/
/// `void shake256_squeezeblocks(uint8_t *output, size_t nblocks, uint64_t *s)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_squeezeblocks(output: *mut u8, nblocks: usize, s: *mut u64) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE256_RATE);
}

/*************************************************
 * Name:        shake256
 *
 * Description: SHAKE256 XOF with non-incremental API
 *
 * Arguments:   - uint8_t *output: pointer to output
 *              - size_t outlen: requested output length in bytes
 *              - const uint8_t *input: pointer to input
 *              - size_t inlen: length of input in bytes
 **************************************************/
/// `void shake256(uint8_t *output, size_t outlen,
///                const uint8_t *input, size_t inlen)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256(output: *mut u8, outlen: usize, input: *const u8, inlen: usize) {
    let mut output = output;
    let mut outlen = outlen;

    let nblocks: usize = outlen / SHAKE256_RATE as usize;
    let mut t = [0u8; SHAKE256_RATE as usize];
    let mut s = [0u64; 25];

    shake256_absorb(s.as_mut_ptr(), input, inlen);
    shake256_squeezeblocks(output, nblocks, s.as_mut_ptr());

    output = output.add(nblocks * SHAKE256_RATE as usize);
    outlen -= nblocks * SHAKE256_RATE as usize;

    if outlen != 0 {
        shake256_squeezeblocks(t.as_mut_ptr(), 1, s.as_mut_ptr());
        let mut i: usize = 0;
        while i < outlen {
            *output.add(i) = t[i];
            i += 1;
        }
    }
}
