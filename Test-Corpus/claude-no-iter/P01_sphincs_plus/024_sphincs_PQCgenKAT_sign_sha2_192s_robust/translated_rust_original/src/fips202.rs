// Translation of c_src/lib/shake/src/fips202.c

pub const SHAKE256_RATE: usize = 136;

const NROUNDS: usize = 24;

#[inline]
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

const KECCAK_F_ROUND_CONSTANTS: [u64; NROUNDS] = [
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

fn keccak_f1600_state_permute(state: &mut [u64]) {
    let (mut aba, mut abe, mut abi, mut abo, mut abu);
    let (mut aga, mut age, mut agi, mut ago, mut agu);
    let (mut aka, mut ake, mut aki, mut ako, mut aku);
    let (mut ama, mut ame, mut ami, mut amo, mut amu);
    let (mut asa, mut ase, mut asi, mut aso, mut asu);
    let (mut bca, mut bce, mut bci, mut bco, mut bcu);
    let (mut da, mut de_, mut di, mut do_, mut du);
    let (mut eba, mut ebe, mut ebi, mut ebo, mut ebu);
    let (mut ega, mut ege, mut egi, mut ego, mut egu);
    let (mut eka, mut eke, mut eki, mut eko, mut eku);
    let (mut ema, mut eme, mut emi, mut emo, mut emu);
    let (mut esa, mut ese, mut esi, mut eso, mut esu);

    aba = state[0]; abe = state[1]; abi = state[2]; abo = state[3]; abu = state[4];
    aga = state[5]; age = state[6]; agi = state[7]; ago = state[8]; agu = state[9];
    aka = state[10]; ake = state[11]; aki = state[12]; ako = state[13]; aku = state[14];
    ama = state[15]; ame = state[16]; ami = state[17]; amo = state[18]; amu = state[19];
    asa = state[20]; ase = state[21]; asi = state[22]; aso = state[23]; asu = state[24];

    let mut round = 0usize;
    while round < NROUNDS {
        bca = aba ^ aga ^ aka ^ ama ^ asa;
        bce = abe ^ age ^ ake ^ ame ^ ase;
        bci = abi ^ agi ^ aki ^ ami ^ asi;
        bco = abo ^ ago ^ ako ^ amo ^ aso;
        bcu = abu ^ agu ^ aku ^ amu ^ asu;

        da = bcu ^ rol(bce, 1);
        de_ = bca ^ rol(bci, 1);
        di = bce ^ rol(bco, 1);
        do_ = bci ^ rol(bcu, 1);
        du = bco ^ rol(bca, 1);

        aba ^= da; bca = aba;
        age ^= de_; bce = rol(age, 44);
        aki ^= di; bci = rol(aki, 43);
        amo ^= do_; bco = rol(amo, 21);
        asu ^= du; bcu = rol(asu, 14);
        eba = bca ^ ((!bce) & bci);
        eba ^= KECCAK_F_ROUND_CONSTANTS[round];
        ebe = bce ^ ((!bci) & bco);
        ebi = bci ^ ((!bco) & bcu);
        ebo = bco ^ ((!bcu) & bca);
        ebu = bcu ^ ((!bca) & bce);

        abo ^= do_; bca = rol(abo, 28);
        agu ^= du; bce = rol(agu, 20);
        aka ^= da; bci = rol(aka, 3);
        ame ^= de_; bco = rol(ame, 45);
        asi ^= di; bcu = rol(asi, 61);
        ega = bca ^ ((!bce) & bci);
        ege = bce ^ ((!bci) & bco);
        egi = bci ^ ((!bco) & bcu);
        ego = bco ^ ((!bcu) & bca);
        egu = bcu ^ ((!bca) & bce);

        abe ^= de_; bca = rol(abe, 1);
        agi ^= di; bce = rol(agi, 6);
        ako ^= do_; bci = rol(ako, 25);
        amu ^= du; bco = rol(amu, 8);
        asa ^= da; bcu = rol(asa, 18);
        eka = bca ^ ((!bce) & bci);
        eke = bce ^ ((!bci) & bco);
        eki = bci ^ ((!bco) & bcu);
        eko = bco ^ ((!bcu) & bca);
        eku = bcu ^ ((!bca) & bce);

        abu ^= du; bca = rol(abu, 27);
        aga ^= da; bce = rol(aga, 36);
        ake ^= de_; bci = rol(ake, 10);
        ami ^= di; bco = rol(ami, 15);
        aso ^= do_; bcu = rol(aso, 56);
        ema = bca ^ ((!bce) & bci);
        eme = bce ^ ((!bci) & bco);
        emi = bci ^ ((!bco) & bcu);
        emo = bco ^ ((!bcu) & bca);
        emu = bcu ^ ((!bca) & bce);

        abi ^= di; bca = rol(abi, 62);
        ago ^= do_; bce = rol(ago, 55);
        aku ^= du; bci = rol(aku, 39);
        ama ^= da; bco = rol(ama, 41);
        ase ^= de_; bcu = rol(ase, 2);
        esa = bca ^ ((!bce) & bci);
        ese = bce ^ ((!bci) & bco);
        esi = bci ^ ((!bco) & bcu);
        eso = bco ^ ((!bcu) & bca);
        esu = bcu ^ ((!bca) & bce);

        bca = eba ^ ega ^ eka ^ ema ^ esa;
        bce = ebe ^ ege ^ eke ^ eme ^ ese;
        bci = ebi ^ egi ^ eki ^ emi ^ esi;
        bco = ebo ^ ego ^ eko ^ emo ^ eso;
        bcu = ebu ^ egu ^ eku ^ emu ^ esu;

        da = bcu ^ rol(bce, 1);
        de_ = bca ^ rol(bci, 1);
        di = bce ^ rol(bco, 1);
        do_ = bci ^ rol(bcu, 1);
        du = bco ^ rol(bca, 1);

        eba ^= da; bca = eba;
        ege ^= de_; bce = rol(ege, 44);
        eki ^= di; bci = rol(eki, 43);
        emo ^= do_; bco = rol(emo, 21);
        esu ^= du; bcu = rol(esu, 14);
        aba = bca ^ ((!bce) & bci);
        aba ^= KECCAK_F_ROUND_CONSTANTS[round + 1];
        abe = bce ^ ((!bci) & bco);
        abi = bci ^ ((!bco) & bcu);
        abo = bco ^ ((!bcu) & bca);
        abu = bcu ^ ((!bca) & bce);

        ebo ^= do_; bca = rol(ebo, 28);
        egu ^= du; bce = rol(egu, 20);
        eka ^= da; bci = rol(eka, 3);
        eme ^= de_; bco = rol(eme, 45);
        esi ^= di; bcu = rol(esi, 61);
        aga = bca ^ ((!bce) & bci);
        age = bce ^ ((!bci) & bco);
        agi = bci ^ ((!bco) & bcu);
        ago = bco ^ ((!bcu) & bca);
        agu = bcu ^ ((!bca) & bce);

        ebe ^= de_; bca = rol(ebe, 1);
        egi ^= di; bce = rol(egi, 6);
        eko ^= do_; bci = rol(eko, 25);
        emu ^= du; bco = rol(emu, 8);
        esa ^= da; bcu = rol(esa, 18);
        aka = bca ^ ((!bce) & bci);
        ake = bce ^ ((!bci) & bco);
        aki = bci ^ ((!bco) & bcu);
        ako = bco ^ ((!bcu) & bca);
        aku = bcu ^ ((!bca) & bce);

        ebu ^= du; bca = rol(ebu, 27);
        ega ^= da; bce = rol(ega, 36);
        eke ^= de_; bci = rol(eke, 10);
        emi ^= di; bco = rol(emi, 15);
        eso ^= do_; bcu = rol(eso, 56);
        ama = bca ^ ((!bce) & bci);
        ame = bce ^ ((!bci) & bco);
        ami = bci ^ ((!bco) & bcu);
        amo = bco ^ ((!bcu) & bca);
        amu = bcu ^ ((!bca) & bce);

        ebi ^= di; bca = rol(ebi, 62);
        ego ^= do_; bce = rol(ego, 55);
        eku ^= du; bci = rol(eku, 39);
        ema ^= da; bco = rol(ema, 41);
        ese ^= de_; bcu = rol(ese, 2);
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

fn keccak_absorb(s: &mut [u64], r: usize, m: &[u8], mut mlen: usize, p: u8) {
    let mut t = [0u8; 200];
    for i in 0..25 { s[i] = 0; }
    let mut moff = 0usize;

    while mlen >= r {
        for i in 0..(r / 8) {
            s[i] ^= load64(&m[moff + 8 * i..]);
        }
        keccak_f1600_state_permute(s);
        mlen -= r;
        moff += r;
    }

    for i in 0..r {
        t[i] = 0;
    }
    for i in 0..mlen {
        t[i] = m[moff + i];
    }
    t[mlen] = p;
    t[r - 1] |= 128;
    for i in 0..(r / 8) {
        s[i] ^= load64(&t[8 * i..]);
    }
}

fn keccak_squeezeblocks(h: &mut [u8], mut nblocks: usize, s: &mut [u64], r: usize) {
    let mut hoff = 0usize;
    while nblocks > 0 {
        keccak_f1600_state_permute(s);
        for i in 0..(r >> 3) {
            store64(&mut h[hoff + 8 * i..], s[i]);
        }
        hoff += r;
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
    let mut moff = 0usize;
    while mlen + s_inc[25] as usize >= r {
        let take = r - s_inc[25] as usize;
        for i in 0..take {
            s_inc[(s_inc[25] as usize + i) >> 3] ^=
                (m[moff + i] as u64) << (8 * ((s_inc[25] as usize + i) & 0x07));
        }
        mlen -= take;
        moff += take;
        s_inc[25] = 0;

        keccak_f1600_state_permute(s_inc);
    }

    for i in 0..mlen {
        s_inc[(s_inc[25] as usize + i) >> 3] ^=
            (m[moff + i] as u64) << (8 * ((s_inc[25] as usize + i) & 0x07));
    }
    s_inc[25] += mlen as u64;
}

fn keccak_inc_finalize(s_inc: &mut [u64], r: usize, p: u8) {
    s_inc[s_inc[25] as usize >> 3] ^= (p as u64) << (8 * (s_inc[25] as usize & 0x07));
    s_inc[(r - 1) >> 3] ^= 128u64 << (8 * ((r - 1) & 0x07));
    s_inc[25] = 0;
}

fn keccak_inc_squeeze(h: &mut [u8], mut outlen: usize, s_inc: &mut [u64], r: usize) {
    let mut hoff = 0usize;
    let mut i = 0usize;
    while i < outlen && i < s_inc[25] as usize {
        h[hoff + i] = (s_inc[(r - s_inc[25] as usize + i) >> 3]
            >> (8 * ((r - s_inc[25] as usize + i) & 0x07))) as u8;
        i += 1;
    }
    hoff += i;
    outlen -= i;
    s_inc[25] -= i as u64;

    while outlen > 0 {
        keccak_f1600_state_permute(s_inc);
        let mut j = 0usize;
        while j < outlen && j < r {
            h[hoff + j] = (s_inc[j >> 3] >> (8 * (j & 0x07))) as u8;
            j += 1;
        }
        hoff += j;
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

    let written = nblocks * SHAKE256_RATE;
    let rem = outlen - written;
    if rem != 0 {
        shake256_squeezeblocks(&mut t, 1, &mut s);
        for i in 0..rem {
            output[written + i] = t[i];
        }
    }
}
