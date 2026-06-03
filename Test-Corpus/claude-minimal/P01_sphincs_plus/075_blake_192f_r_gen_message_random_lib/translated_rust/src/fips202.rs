// FIPS-202 (Keccak / SHAKE) implementation translated from fips202.c.

pub const SHAKE128_RATE: usize = 168;
pub const SHAKE256_RATE: usize = 136;
pub const SHA3_256_RATE: usize = 136;
pub const SHA3_512_RATE: usize = 72;

const NROUNDS: usize = 24;

#[inline(always)]
fn rol(a: u64, offset: u32) -> u64 {
    a.rotate_left(offset)
}

#[inline(always)]
fn load64(x: &[u8]) -> u64 {
    let mut r: u64 = 0;
    for i in 0..8 {
        r |= (x[i] as u64) << (8 * i);
    }
    r
}

#[inline(always)]
fn store64(x: &mut [u8], u: u64) {
    for i in 0..8 {
        x[i] = (u >> (8 * i)) as u8;
    }
}

const KECCAK_F_ROUND_CONSTANTS: [u64; NROUNDS] = [
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

fn keccak_f1600_state_permute(state: &mut [u64]) {
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
        // prepareTheta
        let mut bca = aba ^ aga ^ aka ^ ama ^ asa;
        let mut bce = abe ^ age ^ ake ^ ame ^ ase;
        let mut bci = abi ^ agi ^ aki ^ ami ^ asi;
        let mut bco = abo ^ ago ^ ako ^ amo ^ aso;
        let mut bcu = abu ^ agu ^ aku ^ amu ^ asu;

        let mut da = bcu ^ rol(bce, 1);
        let mut de = bca ^ rol(bci, 1);
        let mut di = bce ^ rol(bco, 1);
        let mut do_ = bci ^ rol(bcu, 1);
        let mut du = bco ^ rol(bca, 1);

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
        let mut eba = bca ^ ((!bce) & bci);
        eba ^= KECCAK_F_ROUND_CONSTANTS[round];
        let mut ebe = bce ^ ((!bci) & bco);
        let mut ebi = bci ^ ((!bco) & bcu);
        let mut ebo = bco ^ ((!bcu) & bca);
        let mut ebu = bcu ^ ((!bca) & bce);

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
        let mut ega = bca ^ ((!bce) & bci);
        let mut ege = bce ^ ((!bci) & bco);
        let mut egi = bci ^ ((!bco) & bcu);
        let mut ego = bco ^ ((!bcu) & bca);
        let mut egu = bcu ^ ((!bca) & bce);

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
        let mut eka = bca ^ ((!bce) & bci);
        let mut eke = bce ^ ((!bci) & bco);
        let mut eki = bci ^ ((!bco) & bcu);
        let mut eko = bco ^ ((!bcu) & bca);
        let mut eku = bcu ^ ((!bca) & bce);

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
        let mut ema = bca ^ ((!bce) & bci);
        let mut eme = bce ^ ((!bci) & bco);
        let mut emi = bci ^ ((!bco) & bcu);
        let mut emo = bco ^ ((!bcu) & bca);
        let mut emu = bcu ^ ((!bca) & bce);

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
        let mut esa = bca ^ ((!bce) & bci);
        let mut ese = bce ^ ((!bci) & bco);
        let mut esi = bci ^ ((!bco) & bcu);
        let mut eso = bco ^ ((!bcu) & bca);
        let mut esu = bcu ^ ((!bca) & bce);

        // round + 1
        bca = eba ^ ega ^ eka ^ ema ^ esa;
        bce = ebe ^ ege ^ eke ^ eme ^ ese;
        bci = ebi ^ egi ^ eki ^ emi ^ esi;
        bco = ebo ^ ego ^ eko ^ emo ^ eso;
        bcu = ebu ^ egu ^ eku ^ emu ^ esu;

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
        aba ^= KECCAK_F_ROUND_CONSTANTS[round + 1];
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

fn keccak_absorb(s: &mut [u64], r: usize, mut m: &[u8], mut mlen: usize, p: u8) {
    for i in 0..25 {
        s[i] = 0;
    }

    while mlen >= r {
        for i in 0..(r / 8) {
            s[i] ^= load64(&m[8 * i..]);
        }
        keccak_f1600_state_permute(s);
        mlen -= r;
        m = &m[r..];
    }

    let mut t = [0u8; 200];
    for i in 0..r {
        t[i] = 0;
    }
    for i in 0..mlen {
        t[i] = m[i];
    }
    t[mlen] = p;
    t[r - 1] |= 128;
    for i in 0..(r / 8) {
        s[i] ^= load64(&t[8 * i..]);
    }
}

fn keccak_squeezeblocks(mut h: &mut [u8], mut nblocks: usize, s: &mut [u64], r: usize) {
    while nblocks > 0 {
        keccak_f1600_state_permute(s);
        for i in 0..(r >> 3) {
            store64(&mut h[8 * i..], s[i]);
        }
        h = &mut h[r..];
        nblocks -= 1;
    }
}

fn keccak_inc_init(s_inc: &mut [u64]) {
    for i in 0..26 {
        s_inc[i] = 0;
    }
}

fn keccak_inc_absorb(s_inc: &mut [u64], r: usize, mut m: &[u8], mut mlen: usize) {
    while mlen + (s_inc[25] as usize) >= r {
        let chunk = r - s_inc[25] as usize;
        for i in 0..chunk {
            let pos = s_inc[25] as usize + i;
            s_inc[pos >> 3] ^= (m[i] as u64) << (8 * (pos & 0x07));
        }
        mlen -= chunk;
        m = &m[chunk..];
        s_inc[25] = 0;
        keccak_f1600_state_permute(s_inc);
    }

    for i in 0..mlen {
        let pos = s_inc[25] as usize + i;
        s_inc[pos >> 3] ^= (m[i] as u64) << (8 * (pos & 0x07));
    }
    s_inc[25] += mlen as u64;
}

fn keccak_inc_finalize(s_inc: &mut [u64], r: usize, p: u8) {
    let pos25 = s_inc[25] as usize;
    s_inc[pos25 >> 3] ^= (p as u64) << (8 * (pos25 & 0x07));
    s_inc[(r - 1) >> 3] ^= (128u64) << (8 * ((r - 1) & 0x07));
    s_inc[25] = 0;
}

fn keccak_inc_squeeze(mut h: &mut [u8], mut outlen: usize, s_inc: &mut [u64], r: usize) {
    let mut i = 0usize;
    while i < outlen && i < s_inc[25] as usize {
        let pos = r - s_inc[25] as usize + i;
        h[i] = (s_inc[pos >> 3] >> (8 * (pos & 0x07))) as u8;
        i += 1;
    }
    h = &mut h[i..];
    outlen -= i;
    s_inc[25] -= i as u64;

    while outlen > 0 {
        keccak_f1600_state_permute(s_inc);
        let mut j = 0usize;
        while j < outlen && j < r {
            h[j] = (s_inc[j >> 3] >> (8 * (j & 0x07))) as u8;
            j += 1;
        }
        h = &mut h[j..];
        outlen -= j;
        s_inc[25] = (r - j) as u64;
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

pub fn shake256_absorb(s: &mut [u64; 25], input: &[u8], inlen: usize) {
    keccak_absorb(s, SHAKE256_RATE, input, inlen, 0x1F);
}

pub fn shake256_squeezeblocks(output: &mut [u8], nblocks: usize, s: &mut [u64; 25]) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE256_RATE);
}

pub fn shake256(output: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let nblocks = outlen / SHAKE256_RATE;
    let mut t = [0u8; SHAKE256_RATE];
    let mut s = [0u64; 25];

    shake256_absorb(&mut s, input, inlen);
    shake256_squeezeblocks(output, nblocks, &mut s);

    let consumed = nblocks * SHAKE256_RATE;
    let remaining = outlen - consumed;

    if remaining > 0 {
        shake256_squeezeblocks(&mut t, 1, &mut s);
        for i in 0..remaining {
            output[consumed + i] = t[i];
        }
    }
}
