// fips202.rs - Keccak / SHAKE / SHA-3 implementation
// Translated from c_src/lib/shake/src/fips202.c

pub(crate) const SHAKE256_RATE: usize = 136;

const NROUNDS: usize = 24;

fn rol(a: u64, offset: u32) -> u64 {
    (a << offset) ^ (a >> (64 - offset))
}

fn load64(x: &[u8]) -> u64 {
    let mut r: u64 = 0;
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

fn keccak_f1600_state_permute(state: &mut [u64; 25]) {
    let (mut Aba, mut Abe, mut Abi, mut Abo, mut Abu) = (state[0], state[1], state[2], state[3], state[4]);
    let (mut Aga, mut Age, mut Agi, mut Ago, mut Agu) = (state[5], state[6], state[7], state[8], state[9]);
    let (mut Aka, mut Ake, mut Aki, mut Ako, mut Aku) = (state[10], state[11], state[12], state[13], state[14]);
    let (mut Ama, mut Ame, mut Ami, mut Amo, mut Amu) = (state[15], state[16], state[17], state[18], state[19]);
    let (mut Asa, mut Ase, mut Asi, mut Aso, mut Asu) = (state[20], state[21], state[22], state[23], state[24]);

    for round in (0..NROUNDS).step_by(2) {
        // prepareTheta
        let BCa = Aba ^ Aga ^ Aka ^ Ama ^ Asa;
        let BCe = Abe ^ Age ^ Ake ^ Ame ^ Ase;
        let BCi = Abi ^ Agi ^ Aki ^ Ami ^ Asi;
        let BCo = Abo ^ Ago ^ Ako ^ Amo ^ Aso;
        let BCu = Abu ^ Agu ^ Aku ^ Amu ^ Asu;

        let Da = BCu ^ rol(BCe, 1);
        let De = BCa ^ rol(BCi, 1);
        let Di = BCe ^ rol(BCo, 1);
        let Do = BCi ^ rol(BCu, 1);
        let Du = BCo ^ rol(BCa, 1);

        Aba ^= Da; let BCa2 = Aba;
        Age ^= De; let BCe2 = rol(Age, 44);
        Aki ^= Di; let BCi2 = rol(Aki, 43);
        Amo ^= Do; let BCo2 = rol(Amo, 21);
        Asu ^= Du; let BCu2 = rol(Asu, 14);
        let mut Eba = BCa2 ^ ((!BCe2) & BCi2); Eba ^= KECCAKF_ROUND_CONSTANTS[round];
        let Ebe = BCe2 ^ ((!BCi2) & BCo2);
        let Ebi = BCi2 ^ ((!BCo2) & BCu2);
        let Ebo = BCo2 ^ ((!BCu2) & BCa2);
        let Ebu = BCu2 ^ ((!BCa2) & BCe2);

        Abo ^= Do; let BCa2 = rol(Abo, 28);
        Agu ^= Du; let BCe2 = rol(Agu, 20);
        Aka ^= Da; let BCi2 = rol(Aka, 3);
        Ame ^= De; let BCo2 = rol(Ame, 45);
        Asi ^= Di; let BCu2 = rol(Asi, 61);
        let Ega = BCa2 ^ ((!BCe2) & BCi2);
        let Ege = BCe2 ^ ((!BCi2) & BCo2);
        let Egi = BCi2 ^ ((!BCo2) & BCu2);
        let Ego = BCo2 ^ ((!BCu2) & BCa2);
        let Egu = BCu2 ^ ((!BCa2) & BCe2);

        Abe ^= De; let BCa2 = rol(Abe, 1);
        Agi ^= Di; let BCe2 = rol(Agi, 6);
        Ako ^= Do; let BCi2 = rol(Ako, 25);
        Amu ^= Du; let BCo2 = rol(Amu, 8);
        Asa ^= Da; let BCu2 = rol(Asa, 18);
        let Eka = BCa2 ^ ((!BCe2) & BCi2);
        let Eke = BCe2 ^ ((!BCi2) & BCo2);
        let Eki = BCi2 ^ ((!BCo2) & BCu2);
        let Eko = BCo2 ^ ((!BCu2) & BCa2);
        let Eku = BCu2 ^ ((!BCa2) & BCe2);

        Abu ^= Du; let BCa2 = rol(Abu, 27);
        Aga ^= Da; let BCe2 = rol(Aga, 36);
        Ake ^= De; let BCi2 = rol(Ake, 10);
        Ami ^= Di; let BCo2 = rol(Ami, 15);
        Aso ^= Do; let BCu2 = rol(Aso, 56);
        let Ema = BCa2 ^ ((!BCe2) & BCi2);
        let Eme = BCe2 ^ ((!BCi2) & BCo2);
        let Emi = BCi2 ^ ((!BCo2) & BCu2);
        let Emo = BCo2 ^ ((!BCu2) & BCa2);
        let Emu = BCu2 ^ ((!BCa2) & BCe2);

        Abi ^= Di; let BCa2 = rol(Abi, 62);
        Ago ^= Do; let BCe2 = rol(Ago, 55);
        Aku ^= Du; let BCi2 = rol(Aku, 39);
        Ama ^= Da; let BCo2 = rol(Ama, 41);
        Ase ^= De; let BCu2 = rol(Ase, 2);
        let Esa = BCa2 ^ ((!BCe2) & BCi2);
        let Ese = BCe2 ^ ((!BCi2) & BCo2);
        let Esi = BCi2 ^ ((!BCo2) & BCu2);
        let Eso = BCo2 ^ ((!BCu2) & BCa2);
        let Esu = BCu2 ^ ((!BCa2) & BCe2);

        // Round+1: prepareTheta
        let BCa = Eba ^ Ega ^ Eka ^ Ema ^ Esa;
        let BCe = Ebe ^ Ege ^ Eke ^ Eme ^ Ese;
        let BCi = Ebi ^ Egi ^ Eki ^ Emi ^ Esi;
        let BCo = Ebo ^ Ego ^ Eko ^ Emo ^ Eso;
        let BCu = Ebu ^ Egu ^ Eku ^ Emu ^ Esu;

        let Da = BCu ^ rol(BCe, 1);
        let De = BCa ^ rol(BCi, 1);
        let Di = BCe ^ rol(BCo, 1);
        let Do = BCi ^ rol(BCu, 1);
        let Du = BCo ^ rol(BCa, 1);

        { let mut t = Eba; t ^= Da; Aba = t ^ ((!rol(Ege ^ De, 44)) & rol(Eki ^ Di, 43)); /* inline */ }
        // Expand second half properly:
        let t_eba = Eba ^ Da; let BCa2 = t_eba;
        let t_ege = Ege ^ De; let BCe2 = rol(t_ege, 44);
        let t_eki = Eki ^ Di; let BCi2 = rol(t_eki, 43);
        let t_emo = Emo ^ Do; let BCo2 = rol(t_emo, 21);
        let t_esu = Esu ^ Du; let BCu2 = rol(t_esu, 14);
        Aba = BCa2 ^ ((!BCe2) & BCi2); Aba ^= KECCAKF_ROUND_CONSTANTS[round + 1];
        Abe = BCe2 ^ ((!BCi2) & BCo2);
        Abi = BCi2 ^ ((!BCo2) & BCu2);
        Abo = BCo2 ^ ((!BCu2) & BCa2);
        Abu = BCu2 ^ ((!BCa2) & BCe2);

        let t = Ebo ^ Do; let BCa2 = rol(t, 28);
        let t = Egu ^ Du; let BCe2 = rol(t, 20);
        let t = Eka ^ Da; let BCi2 = rol(t, 3);
        let t = Eme ^ De; let BCo2 = rol(t, 45);
        let t = Esi ^ Di; let BCu2 = rol(t, 61);
        Aga = BCa2 ^ ((!BCe2) & BCi2);
        Age = BCe2 ^ ((!BCi2) & BCo2);
        Agi = BCi2 ^ ((!BCo2) & BCu2);
        Ago = BCo2 ^ ((!BCu2) & BCa2);
        Agu = BCu2 ^ ((!BCa2) & BCe2);

        let t = Ebe ^ De; let BCa2 = rol(t, 1);
        let t = Egi ^ Di; let BCe2 = rol(t, 6);
        let t = Eko ^ Do; let BCi2 = rol(t, 25);
        let t = Emu ^ Du; let BCo2 = rol(t, 8);
        let t = Esa ^ Da; let BCu2 = rol(t, 18);
        Aka = BCa2 ^ ((!BCe2) & BCi2);
        Ake = BCe2 ^ ((!BCi2) & BCo2);
        Aki = BCi2 ^ ((!BCo2) & BCu2);
        Ako = BCo2 ^ ((!BCu2) & BCa2);
        Aku = BCu2 ^ ((!BCa2) & BCe2);

        let t = Ebu ^ Du; let BCa2 = rol(t, 27);
        let t = Ega ^ Da; let BCe2 = rol(t, 36);
        let t = Eke ^ De; let BCi2 = rol(t, 10);
        let t = Emi ^ Di; let BCo2 = rol(t, 15);
        let t = Eso ^ Do; let BCu2 = rol(t, 56);
        Ama = BCa2 ^ ((!BCe2) & BCi2);
        Ame = BCe2 ^ ((!BCi2) & BCo2);
        Ami = BCi2 ^ ((!BCo2) & BCu2);
        Amo = BCo2 ^ ((!BCu2) & BCa2);
        Amu = BCu2 ^ ((!BCa2) & BCe2);

        let t = Ebi ^ Di; let BCa2 = rol(t, 62);
        let t = Ego ^ Do; let BCe2 = rol(t, 55);
        let t = Eku ^ Du; let BCi2 = rol(t, 39);
        let t = Ema ^ Da; let BCo2 = rol(t, 41);
        let t = Ese ^ De; let BCu2 = rol(t, 2);
        Asa = BCa2 ^ ((!BCe2) & BCi2);
        Ase = BCe2 ^ ((!BCi2) & BCo2);
        Asi = BCi2 ^ ((!BCo2) & BCu2);
        Aso = BCo2 ^ ((!BCu2) & BCa2);
        Asu = BCu2 ^ ((!BCa2) & BCe2);
    }

    *state = [
        Aba, Abe, Abi, Abo, Abu,
        Aga, Age, Agi, Ago, Agu,
        Aka, Ake, Aki, Ako, Aku,
        Ama, Ame, Ami, Amo, Amu,
        Asa, Ase, Asi, Aso, Asu,
    ];
}

fn keccak_absorb(s: &mut [u64; 25], r: usize, m: &[u8], p: u8) {
    for i in 0..25 { s[i] = 0; }
    let mut off = 0usize;
    let mut mlen = m.len();
    while mlen >= r {
        for i in 0..(r / 8) {
            s[i] ^= load64(&m[off + 8 * i..]);
        }
        keccak_f1600_state_permute(s);
        mlen -= r;
        off += r;
    }
    let mut t = [0u8; 200];
    t[..mlen].copy_from_slice(&m[off..off + mlen]);
    t[mlen] = p;
    t[r - 1] |= 128;
    for i in 0..(r / 8) {
        s[i] ^= load64(&t[8 * i..]);
    }
}

fn keccak_squeezeblocks(h: &mut [u8], nblocks: usize, s: &mut [u64; 25], r: usize) {
    let mut off = 0usize;
    for _ in 0..nblocks {
        keccak_f1600_state_permute(s);
        for i in 0..(r >> 3) {
            store64(&mut h[off + 8 * i..], s[i]);
        }
        off += r;
    }
}

// Incremental API uses s_inc: [u64; 26] where s_inc[25] is byte counter

fn keccak_inc_init(s_inc: &mut [u64; 26]) {
    for i in 0..26 { s_inc[i] = 0; }
}

fn keccak_inc_absorb(s_inc: &mut [u64; 26], r: usize, m: &[u8]) {
    let mut off = 0usize;
    let mut mlen = m.len();
    while mlen + (s_inc[25] as usize) >= r {
        let take = r - s_inc[25] as usize;
        for i in 0..take {
            let pos = s_inc[25] as usize + i;
            s_inc[pos >> 3] ^= (m[off + i] as u64) << (8 * (pos & 0x07));
        }
        mlen -= take;
        off += take;
        s_inc[25] = 0;
        let state: &mut [u64; 25] = (&mut s_inc[..25]).try_into().unwrap();
        keccak_f1600_state_permute(state);
    }
    for i in 0..mlen {
        let pos = s_inc[25] as usize + i;
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
    // consume leftover bytes
    let mut i = 0usize;
    while i < outlen && i < s_inc[25] as usize {
        let pos = r - s_inc[25] as usize + i;
        h[off] = (s_inc[pos >> 3] >> (8 * (pos & 0x07))) as u8;
        off += 1;
        i += 1;
    }
    outlen -= i;
    s_inc[25] -= i as u64;

    while outlen > 0 {
        let state: &mut [u64; 25] = (&mut s_inc[..25]).try_into().unwrap();
        keccak_f1600_state_permute(state);
        let take = if outlen < r { outlen } else { r };
        for i in 0..take {
            h[off] = (s_inc[i >> 3] >> (8 * (i & 0x07))) as u8;
            off += 1;
        }
        outlen -= take;
        s_inc[25] = (r - take) as u64;
    }
}

// Public incremental SHAKE-256 API

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

// Non-incremental SHAKE-256

pub fn shake256(output: &mut [u8], outlen: usize, input: &[u8]) {
    let nblocks = outlen / SHAKE256_RATE;
    let mut s = [0u64; 25];

    keccak_absorb(&mut s, SHAKE256_RATE, input, 0x1F);
    keccak_squeezeblocks(output, nblocks, &mut s, SHAKE256_RATE);

    let done = nblocks * SHAKE256_RATE;
    let remaining = outlen - done;
    if remaining > 0 {
        let mut t = [0u8; SHAKE256_RATE];
        keccak_squeezeblocks(&mut t, 1, &mut s, SHAKE256_RATE);
        output[done..done + remaining].copy_from_slice(&t[..remaining]);
    }
}
