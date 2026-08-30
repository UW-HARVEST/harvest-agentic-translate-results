//! Translation of `lib/shake/src/fips202.c` and `lib/shake/include/fips202.h`.
//!
//! Based on the public domain implementation in
//! `crypto_hash/keccakc512/simple/` from <http://bench.cr.yp.to/supercop.html>
//! by Ronny Van Keer and the public domain "TweetFips202" implementation by
//! Gilles Van Assche, Daniel J. Bernstein and Peter Schwabe.

pub const SHAKE128_RATE: usize = 168;
pub const SHAKE256_RATE: usize = 136;
pub const SHA3_256_RATE: usize = 136;
pub const SHA3_512_RATE: usize = 72;

const NROUNDS: usize = 24;

#[inline(always)]
fn rol(a: u64, offset: u32) -> u64 {
    (a << offset) ^ (a >> (64 - offset))
}

/// Load 8 bytes into `u64` in little-endian order.
#[inline(always)]
fn load64(x: &[u8]) -> u64 {
    let mut r: u64 = 0;
    for i in 0..8 {
        r |= (x[i] as u64) << (8 * i);
    }
    r
}

/// Store a 64-bit integer to a byte array in little-endian order.
#[inline(always)]
fn store64(x: &mut [u8], u: u64) {
    for i in 0..8 {
        x[i] = (u >> (8 * i)) as u8;
    }
}

/// Keccak round constants.
#[rustfmt::skip]
static KECCAK_F_ROUND_CONSTANTS: [u64; NROUNDS] = [
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

/// The Keccak F1600 permutation.
fn keccak_f1600_state_permute(state: &mut [u64]) {
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

    let mut round = 0;
    while round < NROUNDS {
        // prepareTheta
        let mut bca = aba ^ aga ^ aka ^ ama ^ asa;
        let mut bce = abe ^ age ^ ake ^ ame ^ ase;
        let mut bci = abi ^ agi ^ aki ^ ami ^ asi;
        let mut bco = abo ^ ago ^ ako ^ amo ^ aso;
        let mut bcu = abu ^ agu ^ aku ^ amu ^ asu;

        // thetaRhoPiChiIotaPrepareTheta(round, A, E)
        let mut da = bcu ^ rol(bce, 1);
        let mut de = bca ^ rol(bci, 1);
        let mut di = bce ^ rol(bco, 1);
        let mut dobj = bci ^ rol(bcu, 1);
        let mut du = bco ^ rol(bca, 1);

        aba ^= da;
        bca = aba;
        age ^= de;
        bce = rol(age, 44);
        aki ^= di;
        bci = rol(aki, 43);
        amo ^= dobj;
        bco = rol(amo, 21);
        asu ^= du;
        bcu = rol(asu, 14);
        let mut eba = bca ^ ((!bce) & bci);
        eba ^= KECCAK_F_ROUND_CONSTANTS[round];
        let ebe = bce ^ ((!bci) & bco);
        let ebi = bci ^ ((!bco) & bcu);
        let ebo = bco ^ ((!bcu) & bca);
        let ebu = bcu ^ ((!bca) & bce);

        abo ^= dobj;
        bca = rol(abo, 28);
        agu ^= du;
        bce = rol(agu, 20);
        aka ^= da;
        bci = rol(aka, 3);
        ame ^= de;
        bco = rol(ame, 45);
        asi ^= di;
        bcu = rol(asi, 61);
        let ega = bca ^ ((!bce) & bci);
        let ege = bce ^ ((!bci) & bco);
        let egi = bci ^ ((!bco) & bcu);
        let ego = bco ^ ((!bcu) & bca);
        let egu = bcu ^ ((!bca) & bce);

        abe ^= de;
        bca = rol(abe, 1);
        agi ^= di;
        bce = rol(agi, 6);
        ako ^= dobj;
        bci = rol(ako, 25);
        amu ^= du;
        bco = rol(amu, 8);
        asa ^= da;
        bcu = rol(asa, 18);
        let eka = bca ^ ((!bce) & bci);
        let eke = bce ^ ((!bci) & bco);
        let eki = bci ^ ((!bco) & bcu);
        let eko = bco ^ ((!bcu) & bca);
        let eku = bcu ^ ((!bca) & bce);

        abu ^= du;
        bca = rol(abu, 27);
        aga ^= da;
        bce = rol(aga, 36);
        ake ^= de;
        bci = rol(ake, 10);
        ami ^= di;
        bco = rol(ami, 15);
        aso ^= dobj;
        bcu = rol(aso, 56);
        let ema = bca ^ ((!bce) & bci);
        let eme = bce ^ ((!bci) & bco);
        let emi = bci ^ ((!bco) & bcu);
        let emo = bco ^ ((!bcu) & bca);
        let emu = bcu ^ ((!bca) & bce);

        abi ^= di;
        bca = rol(abi, 62);
        ago ^= dobj;
        bce = rol(ago, 55);
        aku ^= du;
        bci = rol(aku, 39);
        ama ^= da;
        bco = rol(ama, 41);
        ase ^= de;
        bcu = rol(ase, 2);
        let esa = bca ^ ((!bce) & bci);
        let ese = bce ^ ((!bci) & bco);
        let esi = bci ^ ((!bco) & bcu);
        let eso = bco ^ ((!bcu) & bca);
        let esu = bcu ^ ((!bca) & bce);

        // prepareTheta
        bca = eba ^ ega ^ eka ^ ema ^ esa;
        bce = ebe ^ ege ^ eke ^ eme ^ ese;
        bci = ebi ^ egi ^ eki ^ emi ^ esi;
        bco = ebo ^ ego ^ eko ^ emo ^ eso;
        bcu = ebu ^ egu ^ eku ^ emu ^ esu;

        // thetaRhoPiChiIotaPrepareTheta(round+1, E, A)
        da = bcu ^ rol(bce, 1);
        de = bca ^ rol(bci, 1);
        di = bce ^ rol(bco, 1);
        dobj = bci ^ rol(bcu, 1);
        du = bco ^ rol(bca, 1);

        let mut eba = eba;
        eba ^= da;
        bca = eba;
        let mut ege = ege;
        ege ^= de;
        bce = rol(ege, 44);
        let mut eki = eki;
        eki ^= di;
        bci = rol(eki, 43);
        let mut emo = emo;
        emo ^= dobj;
        bco = rol(emo, 21);
        let mut esu = esu;
        esu ^= du;
        bcu = rol(esu, 14);
        aba = bca ^ ((!bce) & bci);
        aba ^= KECCAK_F_ROUND_CONSTANTS[round + 1];
        abe = bce ^ ((!bci) & bco);
        abi = bci ^ ((!bco) & bcu);
        abo = bco ^ ((!bcu) & bca);
        abu = bcu ^ ((!bca) & bce);

        let mut ebo = ebo;
        ebo ^= dobj;
        bca = rol(ebo, 28);
        let mut egu = egu;
        egu ^= du;
        bce = rol(egu, 20);
        let mut eka = eka;
        eka ^= da;
        bci = rol(eka, 3);
        let mut eme = eme;
        eme ^= de;
        bco = rol(eme, 45);
        let mut esi = esi;
        esi ^= di;
        bcu = rol(esi, 61);
        aga = bca ^ ((!bce) & bci);
        age = bce ^ ((!bci) & bco);
        agi = bci ^ ((!bco) & bcu);
        ago = bco ^ ((!bcu) & bca);
        agu = bcu ^ ((!bca) & bce);

        let mut ebe = ebe;
        ebe ^= de;
        bca = rol(ebe, 1);
        let mut egi = egi;
        egi ^= di;
        bce = rol(egi, 6);
        let mut eko = eko;
        eko ^= dobj;
        bci = rol(eko, 25);
        let mut emu = emu;
        emu ^= du;
        bco = rol(emu, 8);
        let mut esa = esa;
        esa ^= da;
        bcu = rol(esa, 18);
        aka = bca ^ ((!bce) & bci);
        ake = bce ^ ((!bci) & bco);
        aki = bci ^ ((!bco) & bcu);
        ako = bco ^ ((!bcu) & bca);
        aku = bcu ^ ((!bca) & bce);

        let mut ebu = ebu;
        ebu ^= du;
        bca = rol(ebu, 27);
        let mut ega = ega;
        ega ^= da;
        bce = rol(ega, 36);
        let mut eke = eke;
        eke ^= de;
        bci = rol(eke, 10);
        let mut emi = emi;
        emi ^= di;
        bco = rol(emi, 15);
        let mut eso = eso;
        eso ^= dobj;
        bcu = rol(eso, 56);
        ama = bca ^ ((!bce) & bci);
        ame = bce ^ ((!bci) & bco);
        ami = bci ^ ((!bco) & bcu);
        amo = bco ^ ((!bcu) & bca);
        amu = bcu ^ ((!bca) & bce);

        let mut ebi = ebi;
        ebi ^= di;
        bca = rol(ebi, 62);
        let mut ego = ego;
        ego ^= dobj;
        bce = rol(ego, 55);
        let mut eku = eku;
        eku ^= du;
        bci = rol(eku, 39);
        let mut ema = ema;
        ema ^= da;
        bco = rol(ema, 41);
        let mut ese = ese;
        ese ^= de;
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

/// Absorb step of Keccak; non-incremental, starts by zeroing the state.
fn keccak_absorb(s: &mut [u64; 25], r: usize, m: &[u8], mlen: usize, p: u8) {
    let mut t = [0u8; 200];
    let mut mlen = mlen;
    let mut off = 0usize;

    /* Zero state */
    for i in 0..25 {
        s[i] = 0;
    }

    while mlen >= r {
        for i in 0..r / 8 {
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
    for i in 0..r / 8 {
        s[i] ^= load64(&t[8 * i..]);
    }
}

/// Squeeze step of Keccak; squeezes full blocks of `r` bytes each.
fn keccak_squeezeblocks(h: &mut [u8], nblocks: usize, s: &mut [u64; 25], r: usize) {
    let mut nblocks = nblocks;
    let mut off = 0usize;
    while nblocks > 0 {
        keccak_f1600_state_permute(s);
        for i in 0..(r >> 3) {
            store64(&mut h[off + 8 * i..], s[i]);
        }
        off += r;
        nblocks -= 1;
    }
}

/// Initializes the incremental Keccak state to zero.
fn keccak_inc_init(s_inc: &mut [u64; 26]) {
    for i in 0..25 {
        s_inc[i] = 0;
    }
    s_inc[25] = 0;
}

/// Incremental Keccak absorb.
fn keccak_inc_absorb(s_inc: &mut [u64; 26], r: usize, m: &[u8], mlen: usize) {
    let mut mlen = mlen;
    let mut off = 0usize;

    /* Recall that s_inc[25] is the non-absorbed bytes xored into the state */
    while mlen as u64 + s_inc[25] >= r as u64 {
        let taken = r - s_inc[25] as usize;
        for i in 0..taken {
            /* Take the i'th byte from message, xor with the s_inc[25] + i'th
               byte of the state; little-endian */
            let pos = s_inc[25] as usize + i;
            s_inc[pos >> 3] ^= (m[off + i] as u64) << (8 * (pos & 0x07));
        }
        mlen -= taken;
        off += taken;
        s_inc[25] = 0;

        let mut state: [u64; 25] = s_inc[..25].try_into().unwrap();
        keccak_f1600_state_permute(&mut state);
        s_inc[..25].copy_from_slice(&state);
    }

    for i in 0..mlen {
        let pos = s_inc[25] as usize + i;
        s_inc[pos >> 3] ^= (m[off + i] as u64) << (8 * (pos & 0x07));
    }
    s_inc[25] += mlen as u64;
}

/// Finalizes the Keccak absorb phase, prepares for squeezing.
fn keccak_inc_finalize(s_inc: &mut [u64; 26], r: usize, p: u8) {
    /* After keccak_inc_absorb, we are guaranteed that s_inc[25] < r, so we can
       always use one more byte for p in the current state. */
    let pos = s_inc[25] as usize;
    s_inc[pos >> 3] ^= (p as u64) << (8 * (pos & 0x07));
    s_inc[(r - 1) >> 3] ^= 128u64 << (8 * ((r - 1) & 0x07));
    s_inc[25] = 0;
}

/// Incremental Keccak squeeze; can be called on byte level.
fn keccak_inc_squeeze(h: &mut [u8], outlen: usize, s_inc: &mut [u64; 26], r: usize) {
    let mut outlen = outlen;
    let mut off = 0usize;

    /* First consume any bytes we still have sitting around */
    let mut i = 0usize;
    while i < outlen && (i as u64) < s_inc[25] {
        /* There are s_inc[25] bytes left, so r - s_inc[25] is the first
           available byte. We consume from there, i.e., up to r. */
        let pos = r - s_inc[25] as usize + i;
        h[off + i] = (s_inc[pos >> 3] >> (8 * (pos & 0x07))) as u8;
        i += 1;
    }
    off += i;
    outlen -= i;
    s_inc[25] -= i as u64;

    /* Then squeeze the remaining necessary blocks */
    while outlen > 0 {
        let mut state: [u64; 25] = s_inc[..25].try_into().unwrap();
        keccak_f1600_state_permute(&mut state);
        s_inc[..25].copy_from_slice(&state);

        let mut i = 0usize;
        while i < outlen && i < r {
            h[off + i] = (s_inc[i >> 3] >> (8 * (i & 0x07))) as u8;
            i += 1;
        }
        off += i;
        outlen -= i;
        s_inc[25] = (r - i) as u64;
    }
}

pub fn shake256_inc_init(s_inc: &mut [u64; 26]) {
    keccak_inc_init(s_inc);
}

pub fn shake256_inc_absorb(s_inc: &mut [u64; 26], input: &[u8], inlen: usize) {
    keccak_inc_absorb(s_inc, SHAKE256_RATE, input, inlen);
}

pub fn shake256_inc_finalize(s_inc: &mut [u64; 26]) {
    keccak_inc_finalize(s_inc, SHAKE256_RATE, 0x1F);
}

pub fn shake256_inc_squeeze(output: &mut [u8], outlen: usize, s_inc: &mut [u64; 26]) {
    keccak_inc_squeeze(output, outlen, s_inc, SHAKE256_RATE);
}

/// Absorb step of the SHAKE256 XOF; non-incremental, starts by zeroing the
/// state.
pub fn shake256_absorb(s: &mut [u64; 25], input: &[u8], inlen: usize) {
    keccak_absorb(s, SHAKE256_RATE, input, inlen, 0x1F);
}

/// Squeeze step of the SHAKE256 XOF.
pub fn shake256_squeezeblocks(output: &mut [u8], nblocks: usize, s: &mut [u64; 25]) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE256_RATE);
}

/// SHAKE256 XOF with non-incremental API.
pub fn shake256(output: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let nblocks = outlen / SHAKE256_RATE;
    let mut t = [0u8; SHAKE256_RATE];
    let mut s = [0u64; 25];
    let mut outlen = outlen;

    shake256_absorb(&mut s, input, inlen);
    shake256_squeezeblocks(output, nblocks, &mut s);

    let off = nblocks * SHAKE256_RATE;
    outlen -= nblocks * SHAKE256_RATE;

    if outlen != 0 {
        shake256_squeezeblocks(&mut t, 1, &mut s);
        output[off..off + outlen].copy_from_slice(&t[..outlen]);
    }
}

// ---------------------------------------------------------------------------
// C ABI.  `fips202.h` does not rename anything, so the exported wrappers keep
// the plain C names; they live in their own module so that they can share the
// names of the safe Rust functions above.
// ---------------------------------------------------------------------------

pub mod abi {
    use super::*;

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn shake256_inc_init(s_inc: *mut u64) {
        unsafe { super::shake256_inc_init(&mut *(s_inc as *mut [u64; 26])) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn shake256_inc_absorb(
        s_inc: *mut u64,
        input: *const u8,
        inlen: usize,
    ) {
        unsafe {
            super::shake256_inc_absorb(
                &mut *(s_inc as *mut [u64; 26]),
                core::slice::from_raw_parts(input, inlen),
                inlen,
            )
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn shake256_inc_finalize(s_inc: *mut u64) {
        unsafe { super::shake256_inc_finalize(&mut *(s_inc as *mut [u64; 26])) }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn shake256_inc_squeeze(
        output: *mut u8,
        outlen: usize,
        s_inc: *mut u64,
    ) {
        unsafe {
            super::shake256_inc_squeeze(
                core::slice::from_raw_parts_mut(output, outlen),
                outlen,
                &mut *(s_inc as *mut [u64; 26]),
            )
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn shake256_absorb(s: *mut u64, input: *const u8, inlen: usize) {
        unsafe {
            super::shake256_absorb(
                &mut *(s as *mut [u64; 25]),
                core::slice::from_raw_parts(input, inlen),
                inlen,
            )
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn shake256_squeezeblocks(
        output: *mut u8,
        nblocks: usize,
        s: *mut u64,
    ) {
        unsafe {
            super::shake256_squeezeblocks(
                core::slice::from_raw_parts_mut(output, nblocks * SHAKE256_RATE),
                nblocks,
                &mut *(s as *mut [u64; 25]),
            )
        }
    }

    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn shake256(
        output: *mut u8,
        outlen: usize,
        input: *const u8,
        inlen: usize,
    ) {
        unsafe {
            super::shake256(
                core::slice::from_raw_parts_mut(output, outlen),
                outlen,
                core::slice::from_raw_parts(input, inlen),
                inlen,
            )
        }
    }
}
