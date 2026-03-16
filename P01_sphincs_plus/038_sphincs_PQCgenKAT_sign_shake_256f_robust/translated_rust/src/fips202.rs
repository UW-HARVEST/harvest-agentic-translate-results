const NROUNDS: usize = 24;
const SHAKE256_RATE: usize = 136;

const KECCAK_RC: [u64; 24] = [
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

        aba ^= da; let bca2 = aba;
        age ^= de; let bce2 = age.rotate_left(44);
        aki ^= di; let bci2 = aki.rotate_left(43);
        amo ^= d_o; let bco2 = amo.rotate_left(21);
        asu ^= du; let bcu2 = asu.rotate_left(14);
        let mut eba = bca2 ^ ((!bce2) & bci2); eba ^= KECCAK_RC[round];
        let ebe = bce2 ^ ((!bci2) & bco2);
        let ebi = bci2 ^ ((!bco2) & bcu2);
        let ebo = bco2 ^ ((!bcu2) & bca2);
        let ebu = bcu2 ^ ((!bca2) & bce2);

        abo ^= d_o; let bca2 = abo.rotate_left(28);
        agu ^= du; let bce2 = agu.rotate_left(20);
        aka ^= da; let bci2 = aka.rotate_left(3);
        ame ^= de; let bco2 = ame.rotate_left(45);
        asi ^= di; let bcu2 = asi.rotate_left(61);
        let ega = bca2 ^ ((!bce2) & bci2);
        let ege = bce2 ^ ((!bci2) & bco2);
        let egi = bci2 ^ ((!bco2) & bcu2);
        let ego = bco2 ^ ((!bcu2) & bca2);
        let egu = bcu2 ^ ((!bca2) & bce2);

        abe ^= de; let bca2 = abe.rotate_left(1);
        agi ^= di; let bce2 = agi.rotate_left(6);
        ako ^= d_o; let bci2 = ako.rotate_left(25);
        amu ^= du; let bco2 = amu.rotate_left(8);
        asa ^= da; let bcu2 = asa.rotate_left(18);
        let eka = bca2 ^ ((!bce2) & bci2);
        let eke = bce2 ^ ((!bci2) & bco2);
        let eki = bci2 ^ ((!bco2) & bcu2);
        let eko = bco2 ^ ((!bcu2) & bca2);
        let eku = bcu2 ^ ((!bca2) & bce2);

        abu ^= du; let bca2 = abu.rotate_left(27);
        aga ^= da; let bce2 = aga.rotate_left(36);
        ake ^= de; let bci2 = ake.rotate_left(10);
        ami ^= di; let bco2 = ami.rotate_left(15);
        aso ^= d_o; let bcu2 = aso.rotate_left(56);
        let ema = bca2 ^ ((!bce2) & bci2);
        let eme = bce2 ^ ((!bci2) & bco2);
        let emi = bci2 ^ ((!bco2) & bcu2);
        let emo = bco2 ^ ((!bcu2) & bca2);
        let emu = bcu2 ^ ((!bca2) & bce2);

        abi ^= di; let bca2 = abi.rotate_left(62);
        ago ^= d_o; let bce2 = ago.rotate_left(55);
        aku ^= du; let bci2 = aku.rotate_left(39);
        ama ^= da; let bco2 = ama.rotate_left(41);
        ase ^= de; let bcu2 = ase.rotate_left(2);
        let esa = bca2 ^ ((!bce2) & bci2);
        let ese = bce2 ^ ((!bci2) & bco2);
        let esi = bci2 ^ ((!bco2) & bcu2);
        let eso = bco2 ^ ((!bcu2) & bca2);
        let esu = bcu2 ^ ((!bca2) & bce2);

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

        let t = eba ^ da; let bca2 = t;
        let t = ege ^ de; let bce2 = t.rotate_left(44);
        let t = eki ^ di; let bci2 = t.rotate_left(43);
        let t = emo ^ d_o; let bco2 = t.rotate_left(21);
        let t = esu ^ du; let bcu2 = t.rotate_left(14);
        aba = bca2 ^ ((!bce2) & bci2); aba ^= KECCAK_RC[round + 1];
        abe = bce2 ^ ((!bci2) & bco2);
        abi = bci2 ^ ((!bco2) & bcu2);
        abo = bco2 ^ ((!bcu2) & bca2);
        abu = bcu2 ^ ((!bca2) & bce2);

        let t = ebo ^ d_o; let bca2 = t.rotate_left(28);
        let t = egu ^ du; let bce2 = t.rotate_left(20);
        let t = eka ^ da; let bci2 = t.rotate_left(3);
        let t = eme ^ de; let bco2 = t.rotate_left(45);
        let t = esi ^ di; let bcu2 = t.rotate_left(61);
        aga = bca2 ^ ((!bce2) & bci2);
        age = bce2 ^ ((!bci2) & bco2);
        agi = bci2 ^ ((!bco2) & bcu2);
        ago = bco2 ^ ((!bcu2) & bca2);
        agu = bcu2 ^ ((!bca2) & bce2);

        let t = ebe ^ de; let bca2 = t.rotate_left(1);
        let t = egi ^ di; let bce2 = t.rotate_left(6);
        let t = eko ^ d_o; let bci2 = t.rotate_left(25);
        let t = emu ^ du; let bco2 = t.rotate_left(8);
        let t = esa ^ da; let bcu2 = t.rotate_left(18);
        aka = bca2 ^ ((!bce2) & bci2);
        ake = bce2 ^ ((!bci2) & bco2);
        aki = bci2 ^ ((!bco2) & bcu2);
        ako = bco2 ^ ((!bcu2) & bca2);
        aku = bcu2 ^ ((!bca2) & bce2);

        let t = ebu ^ du; let bca2 = t.rotate_left(27);
        let t = ega ^ da; let bce2 = t.rotate_left(36);
        let t = eke ^ de; let bci2 = t.rotate_left(10);
        let t = emi ^ di; let bco2 = t.rotate_left(15);
        let t = eso ^ d_o; let bcu2 = t.rotate_left(56);
        ama = bca2 ^ ((!bce2) & bci2);
        ame = bce2 ^ ((!bci2) & bco2);
        ami = bci2 ^ ((!bco2) & bcu2);
        amo = bco2 ^ ((!bcu2) & bca2);
        amu = bcu2 ^ ((!bca2) & bce2);

        let t = ebi ^ di; let bca2 = t.rotate_left(62);
        let t = ego ^ d_o; let bce2 = t.rotate_left(55);
        let t = eku ^ du; let bci2 = t.rotate_left(39);
        let t = ema ^ da; let bco2 = t.rotate_left(41);
        let t = ese ^ de; let bcu2 = t.rotate_left(2);
        asa = bca2 ^ ((!bce2) & bci2);
        ase = bce2 ^ ((!bci2) & bco2);
        asi = bci2 ^ ((!bco2) & bcu2);
        aso = bco2 ^ ((!bcu2) & bca2);
        asu = bcu2 ^ ((!bca2) & bce2);
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
