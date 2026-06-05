// FIPS-202 SHAKE/SHA3 implementation, ported from c_src/lib/shake/src/fips202.c
#![allow(dead_code)]

pub const SHAKE128_RATE: usize = 168;
pub const SHAKE256_RATE: usize = 136;
pub const SHA3_256_RATE: usize = 136;
pub const SHA3_512_RATE: usize = 72;

const NROUNDS: usize = 24;

#[inline]
fn rol(a: u64, offset: u32) -> u64 {
    a.rotate_left(offset)
}

#[inline]
fn load64(x: &[u8]) -> u64 {
    let mut r: u64 = 0;
    for i in 0..8 {
        r |= (x[i] as u64) << (8 * i);
    }
    r
}

#[inline]
fn store64(x: &mut [u8], u: u64) {
    for i in 0..8 {
        x[i] = (u >> (8 * i)) as u8;
    }
}

const KECCAK_F_ROUND_CONSTANTS: [u64; NROUNDS] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
    0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
];

#[allow(non_snake_case)]
fn keccak_f1600_state_permute(state: &mut [u64]) {
    let mut Aba = state[0];
    let mut Abe = state[1];
    let mut Abi = state[2];
    let mut Abo = state[3];
    let mut Abu = state[4];
    let mut Aga = state[5];
    let mut Age = state[6];
    let mut Agi = state[7];
    let mut Ago = state[8];
    let mut Agu = state[9];
    let mut Aka = state[10];
    let mut Ake = state[11];
    let mut Aki = state[12];
    let mut Ako = state[13];
    let mut Aku = state[14];
    let mut Ama = state[15];
    let mut Ame = state[16];
    let mut Ami = state[17];
    let mut Amo = state[18];
    let mut Amu = state[19];
    let mut Asa = state[20];
    let mut Ase = state[21];
    let mut Asi = state[22];
    let mut Aso = state[23];
    let mut Asu = state[24];

    let mut round = 0;
    while round < NROUNDS {
        let mut BCa = Aba ^ Aga ^ Aka ^ Ama ^ Asa;
        let mut BCe = Abe ^ Age ^ Ake ^ Ame ^ Ase;
        let mut BCi = Abi ^ Agi ^ Aki ^ Ami ^ Asi;
        let mut BCo = Abo ^ Ago ^ Ako ^ Amo ^ Aso;
        let mut BCu = Abu ^ Agu ^ Aku ^ Amu ^ Asu;

        let mut Da = BCu ^ rol(BCe, 1);
        let mut De = BCa ^ rol(BCi, 1);
        let mut Di = BCe ^ rol(BCo, 1);
        let mut Do_ = BCi ^ rol(BCu, 1);
        let mut Du = BCo ^ rol(BCa, 1);

        Aba ^= Da;
        BCa = Aba;
        Age ^= De;
        BCe = rol(Age, 44);
        Aki ^= Di;
        BCi = rol(Aki, 43);
        Amo ^= Do_;
        BCo = rol(Amo, 21);
        Asu ^= Du;
        BCu = rol(Asu, 14);
        let mut Eba = BCa ^ ((!BCe) & BCi);
        Eba ^= KECCAK_F_ROUND_CONSTANTS[round];
        let mut Ebe = BCe ^ ((!BCi) & BCo);
        let mut Ebi = BCi ^ ((!BCo) & BCu);
        let mut Ebo = BCo ^ ((!BCu) & BCa);
        let mut Ebu = BCu ^ ((!BCa) & BCe);

        Abo ^= Do_;
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
        Ako ^= Do_;
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
        Aso ^= Do_;
        BCu = rol(Aso, 56);
        let mut Ema = BCa ^ ((!BCe) & BCi);
        let mut Eme = BCe ^ ((!BCi) & BCo);
        let mut Emi = BCi ^ ((!BCo) & BCu);
        let mut Emo = BCo ^ ((!BCu) & BCa);
        let mut Emu = BCu ^ ((!BCa) & BCe);

        Abi ^= Di;
        BCa = rol(Abi, 62);
        Ago ^= Do_;
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

        // Round 2
        BCa = Eba ^ Ega ^ Eka ^ Ema ^ Esa;
        BCe = Ebe ^ Ege ^ Eke ^ Eme ^ Ese;
        BCi = Ebi ^ Egi ^ Eki ^ Emi ^ Esi;
        BCo = Ebo ^ Ego ^ Eko ^ Emo ^ Eso;
        BCu = Ebu ^ Egu ^ Eku ^ Emu ^ Esu;

        Da = BCu ^ rol(BCe, 1);
        De = BCa ^ rol(BCi, 1);
        Di = BCe ^ rol(BCo, 1);
        Do_ = BCi ^ rol(BCu, 1);
        Du = BCo ^ rol(BCa, 1);

        Eba ^= Da;
        BCa = Eba;
        Ege ^= De;
        BCe = rol(Ege, 44);
        Eki ^= Di;
        BCi = rol(Eki, 43);
        Emo ^= Do_;
        BCo = rol(Emo, 21);
        Esu ^= Du;
        BCu = rol(Esu, 14);
        Aba = BCa ^ ((!BCe) & BCi);
        Aba ^= KECCAK_F_ROUND_CONSTANTS[round + 1];
        Abe = BCe ^ ((!BCi) & BCo);
        Abi = BCi ^ ((!BCo) & BCu);
        Abo = BCo ^ ((!BCu) & BCa);
        Abu = BCu ^ ((!BCa) & BCe);

        Ebo ^= Do_;
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
        Eko ^= Do_;
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
        Eso ^= Do_;
        BCu = rol(Eso, 56);
        Ama = BCa ^ ((!BCe) & BCi);
        Ame = BCe ^ ((!BCi) & BCo);
        Ami = BCi ^ ((!BCo) & BCu);
        Amo = BCo ^ ((!BCu) & BCa);
        Amu = BCu ^ ((!BCa) & BCe);

        Ebi ^= Di;
        BCa = rol(Ebi, 62);
        Ego ^= Do_;
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

fn keccak_absorb(s: &mut [u64], r: usize, m: &[u8], mut mlen: usize, p: u8) {
    for i in 0..25 {
        s[i] = 0;
    }
    let mut off = 0;
    while mlen >= r {
        for i in 0..r / 8 {
            s[i] ^= load64(&m[off + 8 * i..]);
        }
        keccak_f1600_state_permute(s);
        mlen -= r;
        off += r;
    }
    let mut t = [0u8; 200];
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

fn keccak_squeezeblocks(h: &mut [u8], mut nblocks: usize, s: &mut [u64], r: usize) {
    let mut off = 0;
    while nblocks > 0 {
        keccak_f1600_state_permute(s);
        for i in 0..r >> 3 {
            store64(&mut h[off + 8 * i..], s[i]);
        }
        off += r;
        nblocks -= 1;
    }
}

fn keccak_inc_init(s_inc: &mut [u64]) {
    for i in 0..25 {
        s_inc[i] = 0;
    }
    s_inc[25] = 0;
}

fn keccak_inc_absorb(s_inc: &mut [u64], r: usize, m: &[u8], mut mlen: usize) {
    let mut off: usize = 0;
    while (mlen as u64) + s_inc[25] >= r as u64 {
        let take = r - s_inc[25] as usize;
        for i in 0..take {
            let pos = (s_inc[25] as usize + i) >> 3;
            let shift = 8 * ((s_inc[25] as usize + i) & 0x07);
            s_inc[pos] ^= (m[off + i] as u64) << shift;
        }
        mlen -= take;
        off += take;
        s_inc[25] = 0;
        keccak_f1600_state_permute(s_inc);
    }
    for i in 0..mlen {
        let pos = (s_inc[25] as usize + i) >> 3;
        let shift = 8 * ((s_inc[25] as usize + i) & 0x07);
        s_inc[pos] ^= (m[off + i] as u64) << shift;
    }
    s_inc[25] += mlen as u64;
}

fn keccak_inc_finalize(s_inc: &mut [u64], r: usize, p: u8) {
    s_inc[s_inc[25] as usize >> 3] ^= (p as u64) << (8 * (s_inc[25] as usize & 0x07));
    s_inc[(r - 1) >> 3] ^= 128u64 << (8 * ((r - 1) & 0x07));
    s_inc[25] = 0;
}

fn keccak_inc_squeeze(h: &mut [u8], mut outlen: usize, s_inc: &mut [u64], r: usize) {
    let mut off = 0;
    let mut i = 0;
    while i < outlen && i < s_inc[25] as usize {
        let pos = (r - s_inc[25] as usize + i) >> 3;
        let shift = 8 * ((r - s_inc[25] as usize + i) & 0x07);
        h[off + i] = (s_inc[pos] >> shift) as u8;
        i += 1;
    }
    off += i;
    outlen -= i;
    s_inc[25] -= i as u64;

    while outlen > 0 {
        keccak_f1600_state_permute(s_inc);
        let mut j = 0;
        while j < outlen && j < r {
            h[off + j] = (s_inc[j >> 3] >> (8 * (j & 0x07))) as u8;
            j += 1;
        }
        off += j;
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

pub fn shake256_inc_squeeze(output: &mut [u8], outlen: usize, s_inc: &mut [u64]) {
    keccak_inc_squeeze(output, outlen, s_inc, SHAKE256_RATE);
}

pub fn shake256_absorb(s: &mut [u64], input: &[u8], inlen: usize) {
    keccak_absorb(s, SHAKE256_RATE, input, inlen, 0x1F);
}

pub fn shake256_squeezeblocks(output: &mut [u8], nblocks: usize, s: &mut [u64]) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE256_RATE);
}

pub fn shake256(output: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let nblocks = outlen / SHAKE256_RATE;
    let mut t = [0u8; SHAKE256_RATE];
    let mut s = [0u64; 25];

    shake256_absorb(&mut s, input, inlen);
    shake256_squeezeblocks(output, nblocks, &mut s);

    let used = nblocks * SHAKE256_RATE;
    let rem = outlen - used;
    if rem > 0 {
        shake256_squeezeblocks(&mut t, 1, &mut s);
        for i in 0..rem {
            output[used + i] = t[i];
        }
    }
}

// C-ABI exports
#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_init_c(s_inc: *mut u64) {
    let s = unsafe { std::slice::from_raw_parts_mut(s_inc, 26) };
    shake256_inc_init(s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_absorb_c(s_inc: *mut u64, input: *const u8, inlen: usize) {
    let s = unsafe { std::slice::from_raw_parts_mut(s_inc, 26) };
    let i = unsafe { std::slice::from_raw_parts(input, inlen) };
    shake256_inc_absorb(s, i, inlen);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_finalize_c(s_inc: *mut u64) {
    let s = unsafe { std::slice::from_raw_parts_mut(s_inc, 26) };
    shake256_inc_finalize(s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_squeeze_c(output: *mut u8, outlen: usize, s_inc: *mut u64) {
    let o = unsafe { std::slice::from_raw_parts_mut(output, outlen) };
    let s = unsafe { std::slice::from_raw_parts_mut(s_inc, 26) };
    shake256_inc_squeeze(o, outlen, s);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_c(
    output: *mut u8,
    outlen: usize,
    input: *const u8,
    inlen: usize,
) {
    let o = unsafe { std::slice::from_raw_parts_mut(output, outlen) };
    let i = unsafe { std::slice::from_raw_parts(input, inlen) };
    shake256(o, outlen, i, inlen);
}
