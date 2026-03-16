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
    let mut aba = state[0]; let mut abe = state[1]; let mut abi = state[2]; let mut abo = state[3]; let mut abu = state[4];
    let mut aga = state[5]; let mut age = state[6]; let mut agi = state[7]; let mut ago = state[8]; let mut agu = state[9];
    let mut aka = state[10]; let mut ake = state[11]; let mut aki = state[12]; let mut ako = state[13]; let mut aku = state[14];
    let mut ama = state[15]; let mut ame = state[16]; let mut ami = state[17]; let mut amo = state[18]; let mut amu = state[19];
    let mut asa = state[20]; let mut ase = state[21]; let mut asi = state[22]; let mut aso = state[23]; let mut asu = state[24];

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

        let mut t = eba ^ da; aba = t ^ ((!((ege ^ de).rotate_left(44))) & (eki ^ di).rotate_left(43));
        // Rewrite second round fully:
        let mut v_eba = eba ^ da;
        let mut v_ege = (ege ^ de).rotate_left(44);
        let mut v_eki = (eki ^ di).rotate_left(43);
        let mut v_emo = (emo ^ d_o).rotate_left(21);
        let mut v_esu = (esu ^ du).rotate_left(14);
        aba = v_eba ^ ((!v_ege) & v_eki); aba ^= KECCAK_RC[round + 1];
        abe = v_ege ^ ((!v_eki) & v_emo);
        abi = v_eki ^ ((!v_emo) & v_esu);
        abo = v_emo ^ ((!v_esu) & v_eba);
        abu = v_esu ^ ((!v_eba) & v_ege);

        v_eba = (ebo ^ d_o).rotate_left(28);
        v_ege = (egu ^ du).rotate_left(20);
        v_eki = (eka ^ da).rotate_left(3);
        v_emo = (eme ^ de).rotate_left(45);
        v_esu = (esi ^ di).rotate_left(61);
        aga = v_eba ^ ((!v_ege) & v_eki);
        age = v_ege ^ ((!v_eki) & v_emo);
        agi = v_eki ^ ((!v_emo) & v_esu);
        ago = v_emo ^ ((!v_esu) & v_eba);
        agu = v_esu ^ ((!v_eba) & v_ege);

        v_eba = (ebe ^ de).rotate_left(1);
        v_ege = (egi ^ di).rotate_left(6);
        v_eki = (eko ^ d_o).rotate_left(25);
        v_emo = (emu ^ du).rotate_left(8);
        v_esu = (esa ^ da).rotate_left(18);
        aka = v_eba ^ ((!v_ege) & v_eki);
        ake = v_ege ^ ((!v_eki) & v_emo);
        aki = v_eki ^ ((!v_emo) & v_esu);
        ako = v_emo ^ ((!v_esu) & v_eba);
        aku = v_esu ^ ((!v_eba) & v_ege);

        v_eba = (ebu ^ du).rotate_left(27);
        v_ege = (ega ^ da).rotate_left(36);
        v_eki = (eke ^ de).rotate_left(10);
        v_emo = (emi ^ di).rotate_left(15);
        v_esu = (eso ^ d_o).rotate_left(56);
        ama = v_eba ^ ((!v_ege) & v_eki);
        ame = v_ege ^ ((!v_eki) & v_emo);
        ami = v_eki ^ ((!v_emo) & v_esu);
        amo = v_emo ^ ((!v_esu) & v_eba);
        amu = v_esu ^ ((!v_eba) & v_ege);

        v_eba = (ebi ^ di).rotate_left(62);
        v_ege = (ego ^ d_o).rotate_left(55);
        v_eki = (eku ^ du).rotate_left(39);
        v_emo = (ema ^ da).rotate_left(41);
        v_esu = (ese ^ de).rotate_left(2);
        asa = v_eba ^ ((!v_ege) & v_eki);
        ase = v_ege ^ ((!v_eki) & v_emo);
        asi = v_eki ^ ((!v_emo) & v_esu);
        aso = v_emo ^ ((!v_esu) & v_eba);
        asu = v_esu ^ ((!v_eba) & v_ege);
    }

    state[0] = aba; state[1] = abe; state[2] = abi; state[3] = abo; state[4] = abu;
    state[5] = aga; state[6] = age; state[7] = agi; state[8] = ago; state[9] = agu;
    state[10] = aka; state[11] = ake; state[12] = aki; state[13] = ako; state[14] = aku;
    state[15] = ama; state[16] = ame; state[17] = ami; state[18] = amo; state[19] = amu;
    state[20] = asa; state[21] = ase; state[22] = asi; state[23] = aso; state[24] = asu;
}

fn keccak_absorb(s: &mut [u64; 25], r: usize, m: &[u8], p: u8) {
    for i in 0..25 { s[i] = 0; }
    let mut off = 0;
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
    let mut off = 0;
    for _ in 0..nblocks {
        keccak_f1600(s);
        for i in 0..(r >> 3) {
            store64(&mut h[off + 8 * i..], s[i]);
        }
        off += r;
    }
}

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

    pub fn squeeze(&mut self, output: &mut [u8], outlen: usize) {
        keccak_inc_squeeze(output, outlen, &mut self.s, SHAKE256_RATE);
    }

    pub fn state_mut(&mut self) -> &mut [u64; 26] {
        &mut self.s
    }

    // Raw methods operating on [u64; 26] directly, for KAT transcript
    pub fn init_raw(s: &mut [u64; 26]) {
        for i in 0..26 { s[i] = 0; }
    }

    pub fn absorb_raw(s: &mut [u64; 26], input: &[u8]) {
        keccak_inc_absorb(s, SHAKE256_RATE, input);
    }

    pub fn finalize_raw(s: &mut [u64; 26]) {
        keccak_inc_finalize(s, SHAKE256_RATE, 0x1F);
    }

    pub fn squeeze_raw(output: &mut [u8], outlen: usize, s: &mut [u64; 26]) {
        keccak_inc_squeeze(output, outlen, s, SHAKE256_RATE);
    }
}

fn keccak_inc_absorb(s_inc: &mut [u64; 26], r: usize, m: &[u8]) {
    let mut off = 0;
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

fn keccak_inc_squeeze(h: &mut [u8], mut outlen: usize, s_inc: &mut [u64; 26], r: usize) {
    let mut h_off = 0;
    // First consume leftover bytes
    let mut i = 0;
    while i < outlen && i < s_inc[25] as usize {
        let pos = r - s_inc[25] as usize + i;
        h[h_off + i] = (s_inc[pos >> 3] >> (8 * (pos & 0x07))) as u8;
        i += 1;
    }
    h_off += i;
    outlen -= i;
    s_inc[25] -= i as u64;

    while outlen > 0 {
        let state: &mut [u64; 25] = (&mut s_inc[..25]).try_into().unwrap();
        keccak_f1600(state);
        i = 0;
        while i < outlen && i < r {
            h[h_off + i] = (s_inc[i >> 3] >> (8 * (i & 0x07))) as u8;
            i += 1;
        }
        h_off += i;
        outlen -= i;
        s_inc[25] = (r - i) as u64;
    }
}
