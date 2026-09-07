/* Based on the public domain implementation in
 * crypto_hash/keccakc512/simple/ from http://bench.cr.yp.to/supercop.html
 * by Ronny Van Keer
 * and the public domain "TweetFips202" implementation
 * from https://twitter.com/tweetfips202
 * by Gilles Van Assche, Daniel J. Bernstein, and Peter Schwabe */

// Translation of c_src/lib/shake/src/fips202.c
//
// NOTE: fips202.h declares shake128*, sha3_256*, sha3_512* and one-shot
// shake128/shake256, but this particular fips202.c only *defines* the SHAKE256
// family plus the shared Keccak core. Only the functions actually implemented
// in the C source are translated here; fabricating the others would violate the
// byte-identical requirement of the translation contract.

#![allow(non_upper_case_globals)]

const NROUNDS: usize = 24;

// #define SHAKE128_RATE 168
pub const SHAKE128_RATE: usize = 168;
// #define SHAKE256_RATE 136
pub const SHAKE256_RATE: usize = 136;
// #define SHA3_256_RATE 136
pub const SHA3_256_RATE: usize = 136;
// #define SHA3_512_RATE 72
pub const SHA3_512_RATE: usize = 72;

/// #define ROL(a, offset) (((a) << (offset)) ^ ((a) >> (64 - (offset))))
#[inline(always)]
fn rol(a: u64, offset: u32) -> u64 {
    a.rotate_left(offset)
}

/*************************************************
 * Name:        load64
 *
 * Description: Load 8 bytes into uint64_t in little-endian order
 **************************************************/
#[inline]
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
 **************************************************/
#[inline]
unsafe fn store64(x: *mut u8, u: u64) {
    let mut i: usize = 0;
    while i < 8 {
        *x.add(i) = (u >> (8 * i)) as u8;
        i += 1;
    }
}

/* Keccak round constants */
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
unsafe fn KeccakF1600_StatePermute(state: *mut u64) {
    let mut round: usize;

    let mut aba: u64;
    let mut abe: u64;
    let mut abi: u64;
    let mut abo: u64;
    let mut abu: u64;
    let mut aga: u64;
    let mut age: u64;
    let mut agi: u64;
    let mut ago: u64;
    let mut agu: u64;
    let mut aka: u64;
    let mut ake: u64;
    let mut aki: u64;
    let mut ako: u64;
    let mut aku: u64;
    let mut ama: u64;
    let mut ame: u64;
    let mut ami: u64;
    let mut amo: u64;
    let mut amu: u64;
    let mut asa: u64;
    let mut ase: u64;
    let mut asi: u64;
    let mut aso: u64;
    let mut asu: u64;
    let mut bca: u64;
    let mut bce: u64;
    let mut bci: u64;
    let mut bco: u64;
    let mut bcu: u64;
    let mut da: u64;
    let mut de: u64;
    let mut di: u64;
    let mut do_: u64;
    let mut du: u64;
    let mut eba: u64;
    let mut ebe: u64;
    let mut ebi: u64;
    let mut ebo: u64;
    let mut ebu: u64;
    let mut ega: u64;
    let mut ege: u64;
    let mut egi: u64;
    let mut ego: u64;
    let mut egu: u64;
    let mut eka: u64;
    let mut eke: u64;
    let mut eki: u64;
    let mut eko: u64;
    let mut eku: u64;
    let mut ema: u64;
    let mut eme: u64;
    let mut emi: u64;
    let mut emo: u64;
    let mut emu: u64;
    let mut esa: u64;
    let mut ese: u64;
    let mut esi: u64;
    let mut eso: u64;
    let mut esu: u64;

    // copyFromState(A, state)
    aba = *state.add(0);
    abe = *state.add(1);
    abi = *state.add(2);
    abo = *state.add(3);
    abu = *state.add(4);
    aga = *state.add(5);
    age = *state.add(6);
    agi = *state.add(7);
    ago = *state.add(8);
    agu = *state.add(9);
    aka = *state.add(10);
    ake = *state.add(11);
    aki = *state.add(12);
    ako = *state.add(13);
    aku = *state.add(14);
    ama = *state.add(15);
    ame = *state.add(16);
    ami = *state.add(17);
    amo = *state.add(18);
    amu = *state.add(19);
    asa = *state.add(20);
    ase = *state.add(21);
    asi = *state.add(22);
    aso = *state.add(23);
    asu = *state.add(24);

    round = 0;
    while round < NROUNDS {
        //    prepareTheta
        bca = aba ^ aga ^ aka ^ ama ^ asa;
        bce = abe ^ age ^ ake ^ ame ^ ase;
        bci = abi ^ agi ^ aki ^ ami ^ asi;
        bco = abo ^ ago ^ ako ^ amo ^ aso;
        bcu = abu ^ agu ^ aku ^ amu ^ asu;

        // thetaRhoPiChiIotaPrepareTheta(round  , A, E)
        da = bcu ^ rol(bce, 1);
        de = bca ^ rol(bci, 1);
        di = bce ^ rol(bco, 1);
        do_ = bci ^ rol(bcu, 1);
        du = bco ^ rol(bca, 1);

        aba ^= da;
        bca = aba;
        age ^= de;
        bce = rol(age, 44);
        aki ^= di;
        bci = rol(aki, 43);
        amo ^= do_;
        bco = rol(amo, 21);
        asu ^= du;
        bcu = rol(asu, 14);
        eba = bca ^ ((!bce) & bci);
        eba ^= KeccakF_RoundConstants[round];
        ebe = bce ^ ((!bci) & bco);
        ebi = bci ^ ((!bco) & bcu);
        ebo = bco ^ ((!bcu) & bca);
        ebu = bcu ^ ((!bca) & bce);

        abo ^= do_;
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
        ako ^= do_;
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
        aso ^= do_;
        bcu = rol(aso, 56);
        ema = bca ^ ((!bce) & bci);
        eme = bce ^ ((!bci) & bco);
        emi = bci ^ ((!bco) & bcu);
        emo = bco ^ ((!bcu) & bca);
        emu = bcu ^ ((!bca) & bce);

        abi ^= di;
        bca = rol(abi, 62);
        ago ^= do_;
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
        da = bcu ^ rol(bce, 1);
        de = bca ^ rol(bci, 1);
        di = bce ^ rol(bco, 1);
        do_ = bci ^ rol(bcu, 1);
        du = bco ^ rol(bca, 1);

        eba ^= da;
        bca = eba;
        ege ^= de;
        bce = rol(ege, 44);
        eki ^= di;
        bci = rol(eki, 43);
        emo ^= do_;
        bco = rol(emo, 21);
        esu ^= du;
        bcu = rol(esu, 14);
        aba = bca ^ ((!bce) & bci);
        aba ^= KeccakF_RoundConstants[round + 1];
        abe = bce ^ ((!bci) & bco);
        abi = bci ^ ((!bco) & bcu);
        abo = bco ^ ((!bcu) & bca);
        abu = bcu ^ ((!bca) & bce);

        ebo ^= do_;
        bca = rol(ebo, 28);
        egu ^= du;
        bce = rol(egu, 20);
        eka ^= da;
        bci = rol(eka, 3);
        eme ^= de;
        bco = rol(eme, 45);
        esi ^= di;
        bcu = rol(esi, 61);
        aga = bca ^ ((!bce) & bci);
        age = bce ^ ((!bci) & bco);
        agi = bci ^ ((!bco) & bcu);
        ago = bco ^ ((!bcu) & bca);
        agu = bcu ^ ((!bca) & bce);

        ebe ^= de;
        bca = rol(ebe, 1);
        egi ^= di;
        bce = rol(egi, 6);
        eko ^= do_;
        bci = rol(eko, 25);
        emu ^= du;
        bco = rol(emu, 8);
        esa ^= da;
        bcu = rol(esa, 18);
        aka = bca ^ ((!bce) & bci);
        ake = bce ^ ((!bci) & bco);
        aki = bci ^ ((!bco) & bcu);
        ako = bco ^ ((!bcu) & bca);
        aku = bcu ^ ((!bca) & bce);

        ebu ^= du;
        bca = rol(ebu, 27);
        ega ^= da;
        bce = rol(ega, 36);
        eke ^= de;
        bci = rol(eke, 10);
        emi ^= di;
        bco = rol(emi, 15);
        eso ^= do_;
        bcu = rol(eso, 56);
        ama = bca ^ ((!bce) & bci);
        ame = bce ^ ((!bci) & bco);
        ami = bci ^ ((!bco) & bcu);
        amo = bco ^ ((!bcu) & bca);
        amu = bcu ^ ((!bca) & bce);

        ebi ^= di;
        bca = rol(ebi, 62);
        ego ^= do_;
        bce = rol(ego, 55);
        eku ^= du;
        bci = rol(eku, 39);
        ema ^= da;
        bco = rol(ema, 41);
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
    *state.add(0) = aba;
    *state.add(1) = abe;
    *state.add(2) = abi;
    *state.add(3) = abo;
    *state.add(4) = abu;
    *state.add(5) = aga;
    *state.add(6) = age;
    *state.add(7) = agi;
    *state.add(8) = ago;
    *state.add(9) = agu;
    *state.add(10) = aka;
    *state.add(11) = ake;
    *state.add(12) = aki;
    *state.add(13) = ako;
    *state.add(14) = aku;
    *state.add(15) = ama;
    *state.add(16) = ame;
    *state.add(17) = ami;
    *state.add(18) = amo;
    *state.add(19) = amu;
    *state.add(20) = asa;
    *state.add(21) = ase;
    *state.add(22) = asi;
    *state.add(23) = aso;
    *state.add(24) = asu;
}

/*************************************************
 * Name:        keccak_absorb
 *
 * Description: Absorb step of Keccak;
 *              non-incremental, starts by zeroeing the state.
 **************************************************/
unsafe fn keccak_absorb(s: *mut u64, r: u32, mut m: *const u8, mut mlen: usize, p: u8) {
    let mut i: usize;
    let mut t: [u8; 200] = [0u8; 200];

    let r_us = r as usize;

    /* Zero state */
    i = 0;
    while i < 25 {
        *s.add(i) = 0;
        i += 1;
    }

    while mlen >= r_us {
        i = 0;
        while i < r_us / 8 {
            *s.add(i) ^= load64(m.add(8 * i));
            i += 1;
        }

        KeccakF1600_StatePermute(s);
        mlen -= r_us;
        m = m.add(r_us);
    }

    i = 0;
    while i < r_us {
        t[i] = 0;
        i += 1;
    }
    i = 0;
    while i < mlen {
        t[i] = *m.add(i);
        i += 1;
    }
    t[i] = p;
    t[r_us - 1] |= 128;
    i = 0;
    while i < r_us / 8 {
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
 **************************************************/
unsafe fn keccak_squeezeblocks(mut h: *mut u8, mut nblocks: usize, s: *mut u64, r: u32) {
    let r_us = r as usize;
    while nblocks > 0 {
        KeccakF1600_StatePermute(s);
        let mut i: usize = 0;
        while i < (r_us >> 3) {
            store64(h.add(8 * i), *s.add(i));
            i += 1;
        }
        h = h.add(r_us);
        nblocks -= 1;
    }
}

/*************************************************
 * Name:        keccak_inc_init
 *
 * Description: Initializes the incremental Keccak state to zero.
 **************************************************/
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
 **************************************************/
unsafe fn keccak_inc_absorb(s_inc: *mut u64, r: u32, mut m: *const u8, mut mlen: usize) {
    let mut i: usize;
    let r64: u64 = r as u64;

    /* Recall that s_inc[25] is the non-absorbed bytes xored into the state */
    while (mlen as u64) + *s_inc.add(25) >= r64 {
        i = 0;
        while (i as u64) < r64 - *s_inc.add(25) {
            /* Take the i'th byte from message
               xor with the s_inc[25] + i'th byte of the state; little-endian */
            let off = *s_inc.add(25) + i as u64;
            *s_inc.add((off >> 3) as usize) ^= (*m.add(i) as u64) << (8 * (off & 0x07));
            i += 1;
        }
        mlen -= (r64 - *s_inc.add(25)) as usize;
        m = m.add((r64 - *s_inc.add(25)) as usize);
        *s_inc.add(25) = 0;

        KeccakF1600_StatePermute(s_inc);
    }

    i = 0;
    while i < mlen {
        let off = *s_inc.add(25) + i as u64;
        *s_inc.add((off >> 3) as usize) ^= (*m.add(i) as u64) << (8 * (off & 0x07));
        i += 1;
    }
    *s_inc.add(25) += mlen as u64;
}

/*************************************************
 * Name:        keccak_inc_finalize
 *
 * Description: Finalizes Keccak absorb phase, prepares for squeezing
 **************************************************/
unsafe fn keccak_inc_finalize(s_inc: *mut u64, r: u32, p: u8) {
    /* After keccak_inc_absorb, we are guaranteed that s_inc[25] < r,
       so we can always use one more byte for p in the current state. */
    let r64: u64 = r as u64;
    let s25 = *s_inc.add(25);
    *s_inc.add((s25 >> 3) as usize) ^= (p as u64) << (8 * (s25 & 0x07));
    *s_inc.add(((r64 - 1) >> 3) as usize) ^= 128u64 << (8 * ((r64 - 1) & 0x07));
    *s_inc.add(25) = 0;
}

/*************************************************
 * Name:        keccak_inc_squeeze
 *
 * Description: Incremental Keccak squeeze; can be called on byte-level
 **************************************************/
unsafe fn keccak_inc_squeeze(mut h: *mut u8, mut outlen: usize, s_inc: *mut u64, r: u32) {
    let mut i: usize;
    let r64: u64 = r as u64;

    /* First consume any bytes we still have sitting around */
    i = 0;
    while i < outlen && (i as u64) < *s_inc.add(25) {
        /* There are s_inc[25] bytes left, so r - s_inc[25] is the first
           available byte. We consume from there, i.e., up to r. */
        let off = r64 - *s_inc.add(25) + i as u64;
        *h.add(i) = (*s_inc.add((off >> 3) as usize) >> (8 * (off & 0x07))) as u8;
        i += 1;
    }
    h = h.add(i);
    outlen -= i;
    *s_inc.add(25) -= i as u64;

    /* Then squeeze the remaining necessary blocks */
    while outlen > 0 {
        KeccakF1600_StatePermute(s_inc);

        i = 0;
        while i < outlen && (i as u64) < r64 {
            *h.add(i) = (*s_inc.add(i >> 3) >> (8 * (i & 0x07))) as u8;
            i += 1;
        }
        h = h.add(i);
        outlen -= i;
        *s_inc.add(25) = r64 - i as u64;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_init(s_inc: *mut u64) {
    keccak_inc_init(s_inc);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize) {
    keccak_inc_absorb(s_inc, SHAKE256_RATE as u32, input, inlen);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_finalize(s_inc: *mut u64) {
    keccak_inc_finalize(s_inc, SHAKE256_RATE as u32, 0x1F);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_squeeze(output: *mut u8, outlen: usize, s_inc: *mut u64) {
    keccak_inc_squeeze(output, outlen, s_inc, SHAKE256_RATE as u32);
}

/*************************************************
 * Name:        shake256_absorb
 *
 * Description: Absorb step of the SHAKE256 XOF.
 *              non-incremental, starts by zeroeing the state.
 **************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    keccak_absorb(s, SHAKE256_RATE as u32, input, inlen, 0x1F);
}

/*************************************************
 * Name:        shake256_squeezeblocks
 *
 * Description: Squeeze step of SHAKE256 XOF. Squeezes full blocks of
 *              SHAKE256_RATE bytes each. Modifies the state. Can be called
 *              multiple times to keep squeezing, i.e., is incremental.
 **************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_squeezeblocks(output: *mut u8, nblocks: usize, s: *mut u64) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE256_RATE as u32);
}

/*************************************************
 * Name:        shake256
 *
 * Description: SHAKE256 XOF with non-incremental API
 **************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256(
    mut output: *mut u8,
    mut outlen: usize,
    input: *const u8,
    inlen: usize,
) {
    let nblocks: usize = outlen / SHAKE256_RATE;
    let mut t: [u8; SHAKE256_RATE] = [0u8; SHAKE256_RATE];
    let mut s: [u64; 25] = [0u64; 25];

    shake256_absorb(s.as_mut_ptr(), input, inlen);
    shake256_squeezeblocks(output, nblocks, s.as_mut_ptr());

    output = output.add(nblocks * SHAKE256_RATE);
    outlen -= nblocks * SHAKE256_RATE;

    if outlen != 0 {
        shake256_squeezeblocks(t.as_mut_ptr(), 1, s.as_mut_ptr());
        let mut i: usize = 0;
        while i < outlen {
            *output.add(i) = t[i];
            i += 1;
        }
    }
}
