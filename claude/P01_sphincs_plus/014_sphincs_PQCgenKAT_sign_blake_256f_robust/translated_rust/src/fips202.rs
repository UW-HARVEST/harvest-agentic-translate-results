const NROUNDS: usize = 24;
pub const SHAKE256_RATE: usize = 136;

fn load64(x: &[u8]) -> u64 {
    u64::from_le_bytes([x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7]])
}

fn store64(x: &mut [u8], u: u64) {
    let bytes = u.to_le_bytes();
    x[0] = bytes[0];
    x[1] = bytes[1];
    x[2] = bytes[2];
    x[3] = bytes[3];
    x[4] = bytes[4];
    x[5] = bytes[5];
    x[6] = bytes[6];
    x[7] = bytes[7];
}

macro_rules! ROL {
    ($a:expr, $offset:expr) => {
        (($a) << ($offset)) ^ (($a) >> (64 - ($offset)))
    };
}

#[allow(non_snake_case)]
fn KeccakF1600_StatePermute(state: &mut [u64; 25]) {
    let rc: [u64; 24] = [
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

    // copyFromState
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

    let mut round = 0usize;
    while round < NROUNDS {
        // prepareTheta
        let mut BCa = Aba ^ Aga ^ Aka ^ Ama ^ Asa;
        let mut BCe = Abe ^ Age ^ Ake ^ Ame ^ Ase;
        let mut BCi = Abi ^ Agi ^ Aki ^ Ami ^ Asi;
        let mut BCo = Abo ^ Ago ^ Ako ^ Amo ^ Aso;
        let mut BCu = Abu ^ Agu ^ Aku ^ Amu ^ Asu;

        // thetaRhoPiChiIotaPrepareTheta(round, A, E)
        let mut Da = BCu ^ ROL!(BCe, 1);
        let mut De = BCa ^ ROL!(BCi, 1);
        let mut Di = BCe ^ ROL!(BCo, 1);
        let mut Do = BCi ^ ROL!(BCu, 1);
        let mut Du = BCo ^ ROL!(BCa, 1);

        Aba ^= Da;
        BCa = Aba;
        Age ^= De;
        BCe = ROL!(Age, 44);
        Aki ^= Di;
        BCi = ROL!(Aki, 43);
        Amo ^= Do;
        BCo = ROL!(Amo, 21);
        Asu ^= Du;
        BCu = ROL!(Asu, 14);
        let mut Eba = BCa ^ ((!BCe) & BCi);
        Eba ^= rc[round];
        let mut Ebe = BCe ^ ((!BCi) & BCo);
        let mut Ebi = BCi ^ ((!BCo) & BCu);
        let mut Ebo = BCo ^ ((!BCu) & BCa);
        let mut Ebu = BCu ^ ((!BCa) & BCe);

        Abo ^= Do;
        BCa = ROL!(Abo, 28);
        Agu ^= Du;
        BCe = ROL!(Agu, 20);
        Aka ^= Da;
        BCi = ROL!(Aka, 3);
        Ame ^= De;
        BCo = ROL!(Ame, 45);
        Asi ^= Di;
        BCu = ROL!(Asi, 61);
        let mut Ega = BCa ^ ((!BCe) & BCi);
        let mut Ege = BCe ^ ((!BCi) & BCo);
        let mut Egi = BCi ^ ((!BCo) & BCu);
        let mut Ego = BCo ^ ((!BCu) & BCa);
        let mut Egu = BCu ^ ((!BCa) & BCe);

        Abe ^= De;
        BCa = ROL!(Abe, 1);
        Agi ^= Di;
        BCe = ROL!(Agi, 6);
        Ako ^= Do;
        BCi = ROL!(Ako, 25);
        Amu ^= Du;
        BCo = ROL!(Amu, 8);
        Asa ^= Da;
        BCu = ROL!(Asa, 18);
        let mut Eka = BCa ^ ((!BCe) & BCi);
        let mut Eke = BCe ^ ((!BCi) & BCo);
        let mut Eki = BCi ^ ((!BCo) & BCu);
        let mut Eko = BCo ^ ((!BCu) & BCa);
        let mut Eku = BCu ^ ((!BCa) & BCe);

        Abu ^= Du;
        BCa = ROL!(Abu, 27);
        Aga ^= Da;
        BCe = ROL!(Aga, 36);
        Ake ^= De;
        BCi = ROL!(Ake, 10);
        Ami ^= Di;
        BCo = ROL!(Ami, 15);
        Aso ^= Do;
        BCu = ROL!(Aso, 56);
        let mut Ema = BCa ^ ((!BCe) & BCi);
        let mut Eme = BCe ^ ((!BCi) & BCo);
        let mut Emi = BCi ^ ((!BCo) & BCu);
        let mut Emo = BCo ^ ((!BCu) & BCa);
        let mut Emu = BCu ^ ((!BCa) & BCe);

        Abi ^= Di;
        BCa = ROL!(Abi, 62);
        Ago ^= Do;
        BCe = ROL!(Ago, 55);
        Aku ^= Du;
        BCi = ROL!(Aku, 39);
        Ama ^= Da;
        BCo = ROL!(Ama, 41);
        Ase ^= De;
        BCu = ROL!(Ase, 2);
        let mut Esa = BCa ^ ((!BCe) & BCi);
        let mut Ese = BCe ^ ((!BCi) & BCo);
        let mut Esi = BCi ^ ((!BCo) & BCu);
        let mut Eso = BCo ^ ((!BCu) & BCa);
        let mut Esu = BCu ^ ((!BCa) & BCe);

        // prepareTheta
        BCa = Eba ^ Ega ^ Eka ^ Ema ^ Esa;
        BCe = Ebe ^ Ege ^ Eke ^ Eme ^ Ese;
        BCi = Ebi ^ Egi ^ Eki ^ Emi ^ Esi;
        BCo = Ebo ^ Ego ^ Eko ^ Emo ^ Eso;
        BCu = Ebu ^ Egu ^ Eku ^ Emu ^ Esu;

        // thetaRhoPiChiIotaPrepareTheta(round+1, E, A)
        Da = BCu ^ ROL!(BCe, 1);
        De = BCa ^ ROL!(BCi, 1);
        Di = BCe ^ ROL!(BCo, 1);
        Do = BCi ^ ROL!(BCu, 1);
        Du = BCo ^ ROL!(BCa, 1);

        Eba ^= Da;
        BCa = Eba;
        Ege ^= De;
        BCe = ROL!(Ege, 44);
        Eki ^= Di;
        BCi = ROL!(Eki, 43);
        Emo ^= Do;
        BCo = ROL!(Emo, 21);
        Esu ^= Du;
        BCu = ROL!(Esu, 14);
        Aba = BCa ^ ((!BCe) & BCi);
        Aba ^= rc[round + 1];
        Abe = BCe ^ ((!BCi) & BCo);
        Abi = BCi ^ ((!BCo) & BCu);
        Abo = BCo ^ ((!BCu) & BCa);
        Abu = BCu ^ ((!BCa) & BCe);

        Ebo ^= Do;
        BCa = ROL!(Ebo, 28);
        Egu ^= Du;
        BCe = ROL!(Egu, 20);
        Eka ^= Da;
        BCi = ROL!(Eka, 3);
        Eme ^= De;
        BCo = ROL!(Eme, 45);
        Esi ^= Di;
        BCu = ROL!(Esi, 61);
        Aga = BCa ^ ((!BCe) & BCi);
        Age = BCe ^ ((!BCi) & BCo);
        Agi = BCi ^ ((!BCo) & BCu);
        Ago = BCo ^ ((!BCu) & BCa);
        Agu = BCu ^ ((!BCa) & BCe);

        Ebe ^= De;
        BCa = ROL!(Ebe, 1);
        Egi ^= Di;
        BCe = ROL!(Egi, 6);
        Eko ^= Do;
        BCi = ROL!(Eko, 25);
        Emu ^= Du;
        BCo = ROL!(Emu, 8);
        Esa ^= Da;
        BCu = ROL!(Esa, 18);
        Aka = BCa ^ ((!BCe) & BCi);
        Ake = BCe ^ ((!BCi) & BCo);
        Aki = BCi ^ ((!BCo) & BCu);
        Ako = BCo ^ ((!BCu) & BCa);
        Aku = BCu ^ ((!BCa) & BCe);

        Ebu ^= Du;
        BCa = ROL!(Ebu, 27);
        Ega ^= Da;
        BCe = ROL!(Ega, 36);
        Eke ^= De;
        BCi = ROL!(Eke, 10);
        Emi ^= Di;
        BCo = ROL!(Emi, 15);
        Eso ^= Do;
        BCu = ROL!(Eso, 56);
        Ama = BCa ^ ((!BCe) & BCi);
        Ame = BCe ^ ((!BCi) & BCo);
        Ami = BCi ^ ((!BCo) & BCu);
        Amo = BCo ^ ((!BCu) & BCa);
        Amu = BCu ^ ((!BCa) & BCe);

        Ebi ^= Di;
        BCa = ROL!(Ebi, 62);
        Ego ^= Do;
        BCe = ROL!(Ego, 55);
        Eku ^= Du;
        BCi = ROL!(Eku, 39);
        Ema ^= Da;
        BCo = ROL!(Ema, 41);
        Ese ^= De;
        BCu = ROL!(Ese, 2);
        Asa = BCa ^ ((!BCe) & BCi);
        Ase = BCe ^ ((!BCi) & BCo);
        Asi = BCi ^ ((!BCo) & BCu);
        Aso = BCo ^ ((!BCu) & BCa);
        Asu = BCu ^ ((!BCa) & BCe);

        round += 2;
    }

    // copyToState
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

fn keccak_absorb(s: &mut [u64; 25], r: u32, m: &[u8], mut mlen: usize, p: u8) {
    let mut idx = 0usize;

    // Zero state
    for i in 0..25 {
        s[i] = 0;
    }

    while mlen >= r as usize {
        for i in 0..(r as usize / 8) {
            s[i] ^= load64(&m[idx + 8 * i..]);
        }
        KeccakF1600_StatePermute(s);
        mlen -= r as usize;
        idx += r as usize;
    }

    // Last incomplete block
    let mut t = [0u8; 200];
    for i in 0..mlen {
        t[i] = m[idx + i];
    }
    t[mlen] = p;
    t[r as usize - 1] |= 128;
    for i in 0..(r as usize / 8) {
        s[i] ^= load64(&t[8 * i..]);
    }
}

fn keccak_squeezeblocks(h: &mut [u8], mut nblocks: usize, s: &mut [u64; 25], r: u32) {
    let mut idx = 0usize;
    while nblocks > 0 {
        KeccakF1600_StatePermute(s);
        for i in 0..(r as usize / 8) {
            store64(&mut h[idx + 8 * i..], s[i]);
        }
        idx += r as usize;
        nblocks -= 1;
    }
}

fn keccak_inc_init(s_inc: &mut [u64; 26]) {
    for i in 0..25 {
        s_inc[i] = 0;
    }
    s_inc[25] = 0;
}

fn keccak_inc_absorb(s_inc: &mut [u64; 26], r: u32, m: &[u8], mut mlen: usize) {
    let mut idx = 0usize;

    // Recall that s_inc[25] is the non-absorbed bytes count
    if s_inc[25] != 0 {
        // There are s_inc[25] bytes in the state that have not been absorbed yet
        let count = s_inc[25] as usize;
        if count + mlen >= r as usize {
            // We have enough to fill a block
            let remaining = r as usize - count;
            for i in 0..remaining {
                let pos = count + i;
                s_inc[pos >> 3] ^= (m[idx + i] as u64) << (8 * (pos & 0x07));
            }
            mlen -= remaining;
            idx += remaining;
            KeccakF1600_StatePermute(
                <&mut [u64; 25]>::try_from(&mut s_inc[0..25]).unwrap(),
            );
            s_inc[25] = 0;
        }
    }

    while mlen >= r as usize {
        for i in 0..(r as usize / 8) {
            s_inc[i] ^= load64(&m[idx + 8 * i..]);
        }
        KeccakF1600_StatePermute(
            <&mut [u64; 25]>::try_from(&mut s_inc[0..25]).unwrap(),
        );
        mlen -= r as usize;
        idx += r as usize;
    }

    // Store remaining bytes
    for i in 0..mlen {
        let pos = s_inc[25] as usize + i;
        s_inc[pos >> 3] ^= (m[idx + i] as u64) << (8 * (pos & 0x07));
    }
    s_inc[25] += mlen as u64;
}

fn keccak_inc_finalize(s_inc: &mut [u64; 26], r: u32, p: u8) {
    // After absorb, s_inc[25] has the number of non-absorbed bytes
    let pos = s_inc[25] as usize;

    // XOR padding
    s_inc[pos >> 3] ^= (p as u64) << (8 * (pos & 0x07));
    s_inc[(r as usize - 1) >> 3] ^= 128u64 << (8 * ((r as usize - 1) & 0x07));

    // Do NOT permute here - the squeeze function handles permutation
    s_inc[25] = 0;
}

fn keccak_inc_squeeze(h: &mut [u8], mut outlen: usize, s_inc: &mut [u64; 26], r: u32) {
    let mut idx = 0usize;

    // First, consume any bytes left from a previous squeeze
    let remaining = s_inc[25] as usize;
    if remaining > 0 {
        let offset = r as usize - remaining;
        let to_copy = if outlen < remaining { outlen } else { remaining };
        for i in 0..to_copy {
            let pos = offset + i;
            h[idx + i] = (s_inc[pos >> 3] >> (8 * (pos & 0x07))) as u8;
        }
        idx += to_copy;
        outlen -= to_copy;
        s_inc[25] -= to_copy as u64;
    }

    // Then squeeze full blocks
    while outlen >= r as usize {
        KeccakF1600_StatePermute(
            <&mut [u64; 25]>::try_from(&mut s_inc[0..25]).unwrap(),
        );
        for i in 0..(r as usize / 8) {
            store64(&mut h[idx + 8 * i..], s_inc[i]);
        }
        idx += r as usize;
        outlen -= r as usize;
        s_inc[25] = 0;
    }

    // Finally, squeeze a partial block if needed
    if outlen > 0 {
        KeccakF1600_StatePermute(
            <&mut [u64; 25]>::try_from(&mut s_inc[0..25]).unwrap(),
        );
        for i in 0..outlen {
            h[idx + i] = (s_inc[i >> 3] >> (8 * (i & 0x07))) as u8;
        }
        s_inc[25] = (r as usize - outlen) as u64;
    }
}

pub fn shake256_absorb(s: &mut [u64; 25], input: &[u8], inlen: usize) {
    keccak_absorb(s, SHAKE256_RATE as u32, input, inlen, 0x1F);
}

pub fn shake256_squeezeblocks(output: &mut [u8], nblocks: usize, s: &mut [u64; 25]) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE256_RATE as u32);
}

pub fn shake256_inc_init(s_inc: &mut [u64; 26]) {
    keccak_inc_init(s_inc);
}

pub fn shake256_inc_absorb(s_inc: &mut [u64; 26], input: &[u8], inlen: usize) {
    keccak_inc_absorb(s_inc, SHAKE256_RATE as u32, input, inlen);
}

pub fn shake256_inc_finalize(s_inc: &mut [u64; 26]) {
    keccak_inc_finalize(s_inc, SHAKE256_RATE as u32, 0x1F);
}

pub fn shake256_inc_squeeze(output: &mut [u8], outlen: usize, s_inc: &mut [u64; 26]) {
    keccak_inc_squeeze(output, outlen, s_inc, SHAKE256_RATE as u32);
}

pub fn shake256(output: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let mut s = [0u64; 25];
    let mut t = [0u8; SHAKE256_RATE];

    shake256_absorb(&mut s, input, inlen);

    let nblocks = outlen / SHAKE256_RATE;
    if nblocks > 0 {
        shake256_squeezeblocks(output, nblocks, &mut s);
    }

    let remaining = outlen - nblocks * SHAKE256_RATE;
    if remaining > 0 {
        shake256_squeezeblocks(&mut t, 1, &mut s);
        let offset = nblocks * SHAKE256_RATE;
        for i in 0..remaining {
            output[offset + i] = t[i];
        }
    }
}
