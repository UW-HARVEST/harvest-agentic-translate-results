const NROUNDS: usize = 24;

const KECCAK_RC: [u64; 24] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
    0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
];

pub const SHAKE256_RATE: usize = 136;

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

fn keccak_f1600(state: &mut [u64; 25]) {
    let (mut aba, mut abe, mut abi, mut abo, mut abu) = (state[0], state[1], state[2], state[3], state[4]);
    let (mut aga, mut age, mut agi, mut ago, mut agu) = (state[5], state[6], state[7], state[8], state[9]);
    let (mut aka, mut ake, mut aki, mut ako, mut aku) = (state[10], state[11], state[12], state[13], state[14]);
    let (mut ama, mut ame, mut ami, mut amo, mut amu) = (state[15], state[16], state[17], state[18], state[19]);
    let (mut asa, mut ase, mut asi, mut aso, mut asu) = (state[20], state[21], state[22], state[23], state[24]);

    for round in (0..NROUNDS).step_by(2) {
        let bca = aba ^ aga ^ aka ^ ama ^ asa;
        let bce = abe ^ age ^ ake ^ ame ^ ase;
        let bci = abi ^ agi ^ aki ^ ami ^ asi;
        let bco = abo ^ ago ^ ako ^ amo ^ aso;
        let bcu = abu ^ agu ^ aku ^ amu ^ asu;

        let da = bcu ^ bce.rotate_left(1);
        let de = bca ^ bci.rotate_left(1);
        let di = bce ^ bco.rotate_left(1);
        let d_o = bci ^ bcu.rotate_left(1);
        let du = bco ^ bca.rotate_left(1);

        aba ^= da; let mut t_bca = aba;
        age ^= de; let mut t_bce = age.rotate_left(44);
        aki ^= di; let mut t_bci = aki.rotate_left(43);
        amo ^= d_o; let mut t_bco = amo.rotate_left(21);
        asu ^= du; let mut t_bcu = asu.rotate_left(14);
        let mut eba = t_bca ^ ((!t_bce) & t_bci); eba ^= KECCAK_RC[round];
        let ebe = t_bce ^ ((!t_bci) & t_bco);
        let ebi = t_bci ^ ((!t_bco) & t_bcu);
        let ebo = t_bco ^ ((!t_bcu) & t_bca);
        let ebu = t_bcu ^ ((!t_bca) & t_bce);

        abo ^= d_o; t_bca = abo.rotate_left(28);
        agu ^= du; t_bce = agu.rotate_left(20);
        aka ^= da; t_bci = aka.rotate_left(3);
        ame ^= de; t_bco = ame.rotate_left(45);
        asi ^= di; t_bcu = asi.rotate_left(61);
        let ega = t_bca ^ ((!t_bce) & t_bci);
        let ege = t_bce ^ ((!t_bci) & t_bco);
        let egi = t_bci ^ ((!t_bco) & t_bcu);
        let ego = t_bco ^ ((!t_bcu) & t_bca);
        let egu = t_bcu ^ ((!t_bca) & t_bce);

        abe ^= de; t_bca = abe.rotate_left(1);
        agi ^= di; t_bce = agi.rotate_left(6);
        ako ^= d_o; t_bci = ako.rotate_left(25);
        amu ^= du; t_bco = amu.rotate_left(8);
        asa ^= da; t_bcu = asa.rotate_left(18);
        let eka = t_bca ^ ((!t_bce) & t_bci);
        let eke = t_bce ^ ((!t_bci) & t_bco);
        let eki = t_bci ^ ((!t_bco) & t_bcu);
        let eko = t_bco ^ ((!t_bcu) & t_bca);
        let eku = t_bcu ^ ((!t_bca) & t_bce);

        abu ^= du; t_bca = abu.rotate_left(27);
        aga ^= da; t_bce = aga.rotate_left(36);
        ake ^= de; t_bci = ake.rotate_left(10);
        ami ^= di; t_bco = ami.rotate_left(15);
        aso ^= d_o; t_bcu = aso.rotate_left(56);
        let ema = t_bca ^ ((!t_bce) & t_bci);
        let eme = t_bce ^ ((!t_bci) & t_bco);
        let emi = t_bci ^ ((!t_bco) & t_bcu);
        let emo = t_bco ^ ((!t_bcu) & t_bca);
        let emu = t_bcu ^ ((!t_bca) & t_bce);

        abi ^= di; t_bca = abi.rotate_left(62);
        ago ^= d_o; t_bce = ago.rotate_left(55);
        aku ^= du; t_bci = aku.rotate_left(39);
        ama ^= da; t_bco = ama.rotate_left(41);
        ase ^= de; t_bcu = ase.rotate_left(2);
        let esa = t_bca ^ ((!t_bce) & t_bci);
        let ese = t_bce ^ ((!t_bci) & t_bco);
        let esi = t_bci ^ ((!t_bco) & t_bcu);
        let eso = t_bco ^ ((!t_bcu) & t_bca);
        let esu = t_bcu ^ ((!t_bca) & t_bce);

        // Round 2
        let bca = eba ^ ega ^ eka ^ ema ^ esa;
        let bce = ebe ^ ege ^ eke ^ eme ^ ese;
        let bci = ebi ^ egi ^ eki ^ emi ^ esi;
        let bco = ebo ^ ego ^ eko ^ emo ^ eso;
        let bcu = ebu ^ egu ^ eku ^ emu ^ esu;

        let da = bcu ^ bce.rotate_left(1);
        let de = bca ^ bci.rotate_left(1);
        let di = bce ^ bco.rotate_left(1);
        let d_o = bci ^ bcu.rotate_left(1);
        let du = bco ^ bca.rotate_left(1);

        let mut t_eba = eba ^ da; t_bca = t_eba;
        let mut t_ege = ege ^ de; t_bce = t_ege.rotate_left(44);
        let mut t_eki = eki ^ di; t_bci = t_eki.rotate_left(43);
        let mut t_emo = emo ^ d_o; t_bco = t_emo.rotate_left(21);
        let mut t_esu = esu ^ du; t_bcu = t_esu.rotate_left(14);
        aba = t_bca ^ ((!t_bce) & t_bci); aba ^= KECCAK_RC[round + 1];
        abe = t_bce ^ ((!t_bci) & t_bco);
        abi = t_bci ^ ((!t_bco) & t_bcu);
        abo = t_bco ^ ((!t_bcu) & t_bca);
        abu = t_bcu ^ ((!t_bca) & t_bce);

        t_eba = ebo ^ d_o; t_bca = t_eba.rotate_left(28);
        t_ege = egu ^ du; t_bce = t_ege.rotate_left(20);
        t_eki = eka ^ da; t_bci = t_eki.rotate_left(3);
        t_emo = eme ^ de; t_bco = t_emo.rotate_left(45);
        t_esu = esi ^ di; t_bcu = t_esu.rotate_left(61);
        aga = t_bca ^ ((!t_bce) & t_bci);
        age = t_bce ^ ((!t_bci) & t_bco);
        agi = t_bci ^ ((!t_bco) & t_bcu);
        ago = t_bco ^ ((!t_bcu) & t_bca);
        agu = t_bcu ^ ((!t_bca) & t_bce);

        t_eba = ebe ^ de; t_bca = t_eba.rotate_left(1);
        t_ege = egi ^ di; t_bce = t_ege.rotate_left(6);
        t_eki = eko ^ d_o; t_bci = t_eki.rotate_left(25);
        t_emo = emu ^ du; t_bco = t_emo.rotate_left(8);
        t_esu = esa ^ da; t_bcu = t_esu.rotate_left(18);
        aka = t_bca ^ ((!t_bce) & t_bci);
        ake = t_bce ^ ((!t_bci) & t_bco);
        aki = t_bci ^ ((!t_bco) & t_bcu);
        ako = t_bco ^ ((!t_bcu) & t_bca);
        aku = t_bcu ^ ((!t_bca) & t_bce);

        t_eba = ebu ^ du; t_bca = t_eba.rotate_left(27);
        t_ege = ega ^ da; t_bce = t_ege.rotate_left(36);
        t_eki = eke ^ de; t_bci = t_eki.rotate_left(10);
        t_emo = emi ^ di; t_bco = t_emo.rotate_left(15);
        t_esu = eso ^ d_o; t_bcu = t_esu.rotate_left(56);
        ama = t_bca ^ ((!t_bce) & t_bci);
        ame = t_bce ^ ((!t_bci) & t_bco);
        ami = t_bci ^ ((!t_bco) & t_bcu);
        amo = t_bco ^ ((!t_bcu) & t_bca);
        amu = t_bcu ^ ((!t_bca) & t_bce);

        t_eba = ebi ^ di; t_bca = t_eba.rotate_left(62);
        t_ege = ego ^ d_o; t_bce = t_ege.rotate_left(55);
        t_eki = eku ^ du; t_bci = t_eki.rotate_left(39);
        t_emo = ema ^ da; t_bco = t_emo.rotate_left(41);
        t_esu = ese ^ de; t_bcu = t_esu.rotate_left(2);
        asa = t_bca ^ ((!t_bce) & t_bci);
        ase = t_bce ^ ((!t_bci) & t_bco);
        asi = t_bci ^ ((!t_bco) & t_bcu);
        aso = t_bco ^ ((!t_bcu) & t_bca);
        asu = t_bcu ^ ((!t_bca) & t_bce);
    }

    state[0] = aba; state[1] = abe; state[2] = abi; state[3] = abo; state[4] = abu;
    state[5] = aga; state[6] = age; state[7] = agi; state[8] = ago; state[9] = agu;
    state[10] = aka; state[11] = ake; state[12] = aki; state[13] = ako; state[14] = aku;
    state[15] = ama; state[16] = ame; state[17] = ami; state[18] = amo; state[19] = amu;
    state[20] = asa; state[21] = ase; state[22] = asi; state[23] = aso; state[24] = asu;
}

fn keccak_absorb(s: &mut [u64; 25], r: usize, m: &[u8], p: u8) {
    for i in 0..25 { s[i] = 0; }
    let mut off = 0usize;
    let mut mlen = m.len();
    while mlen >= r {
        for i in 0..(r / 8) {
            s[i] ^= load64(&m[off + 8 * i..]);
        }
        keccak_f1600(s);
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
    for b in 0..nblocks {
        keccak_f1600(s);
        for i in 0..(r >> 3) {
            store64(&mut h[b * r + 8 * i..], s[i]);
        }
    }
}

pub fn shake256(output: &mut [u8], input: &[u8]) {
    let outlen = output.len();
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

pub struct Shake256Inc {
    s: [u64; 26],
}

impl Shake256Inc {
    pub fn new() -> Self {
        Shake256Inc { s: [0u64; 26] }
    }

    pub fn absorb(&mut self, input: &[u8]) {
        keccak_inc_absorb(&mut self.s, SHAKE256_RATE, input);
    }

    pub fn finalize(&mut self) {
        keccak_inc_finalize(&mut self.s, SHAKE256_RATE, 0x1F);
    }

    pub fn squeeze(&mut self, output: &mut [u8]) {
        keccak_inc_squeeze(output, &mut self.s, SHAKE256_RATE);
    }
}

fn keccak_inc_absorb(s_inc: &mut [u64; 26], r: usize, m: &[u8]) {
    let mut off = 0usize;
    let mut mlen = m.len();
    while mlen + (s_inc[25] as usize) >= r {
        let to_absorb = r - s_inc[25] as usize;
        for i in 0..to_absorb {
            let pos = s_inc[25] as usize + i;
            s_inc[pos >> 3] ^= (m[off + i] as u64) << (8 * (pos & 0x07));
        }
        mlen -= to_absorb;
        off += to_absorb;
        s_inc[25] = 0;
        let state: &mut [u64; 25] = (&mut s_inc[..25]).try_into().unwrap();
        keccak_f1600(state);
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

fn keccak_inc_squeeze(h: &mut [u8], s_inc: &mut [u64; 26], r: usize) {
    let mut outlen = h.len();
    let mut hoff = 0usize;

    // First consume leftover bytes
    let mut i = 0usize;
    while i < outlen && i < s_inc[25] as usize {
        let pos = r - s_inc[25] as usize + i;
        h[hoff + i] = (s_inc[pos >> 3] >> (8 * (pos & 0x07))) as u8;
        i += 1;
    }
    hoff += i;
    outlen -= i;
    s_inc[25] -= i as u64;

    while outlen > 0 {
        let state: &mut [u64; 25] = (&mut s_inc[..25]).try_into().unwrap();
        keccak_f1600(state);
        i = 0;
        while i < outlen && i < r {
            h[hoff + i] = (s_inc[i >> 3] >> (8 * (i & 0x07))) as u8;
            i += 1;
        }
        hoff += i;
        outlen -= i;
        s_inc[25] = (r - i) as u64;
    }
}
