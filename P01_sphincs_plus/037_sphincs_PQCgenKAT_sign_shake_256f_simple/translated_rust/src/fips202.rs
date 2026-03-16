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
        let mut bca = aba ^ aga ^ aka ^ ama ^ asa;
        let mut bce = abe ^ age ^ ake ^ ame ^ ase;
        let mut bci = abi ^ agi ^ aki ^ ami ^ asi;
        let mut bco = abo ^ ago ^ ako ^ amo ^ aso;
        let mut bcu = abu ^ agu ^ aku ^ amu ^ asu;

        let da = bcu ^ bce.rotate_left(1);
        let de = bca ^ bci.rotate_left(1);
        let di = bce ^ bco.rotate_left(1);
        let d_o = bci ^ bcu.rotate_left(1);
        let du = bco ^ bca.rotate_left(1);

        aba ^= da; bca = aba;
        age ^= de; bce = age.rotate_left(44);
        aki ^= di; bci = aki.rotate_left(43);
        amo ^= d_o; bco = amo.rotate_left(21);
        asu ^= du; bcu = asu.rotate_left(14);
        let mut eba = bca ^ ((!bce) & bci); eba ^= KECCAK_RC[round];
        let ebe = bce ^ ((!bci) & bco);
        let ebi = bci ^ ((!bco) & bcu);
        let ebo = bco ^ ((!bcu) & bca);
        let ebu = bcu ^ ((!bca) & bce);

        abo ^= d_o; bca = abo.rotate_left(28);
        agu ^= du; bce = agu.rotate_left(20);
        aka ^= da; bci = aka.rotate_left(3);
        ame ^= de; bco = ame.rotate_left(45);
        asi ^= di; bcu = asi.rotate_left(61);
        let ega = bca ^ ((!bce) & bci);
        let ege = bce ^ ((!bci) & bco);
        let egi = bci ^ ((!bco) & bcu);
        let ego = bco ^ ((!bcu) & bca);
        let egu = bcu ^ ((!bca) & bce);

        abe ^= de; bca = abe.rotate_left(1);
        agi ^= di; bce = agi.rotate_left(6);
        ako ^= d_o; bci = ako.rotate_left(25);
        amu ^= du; bco = amu.rotate_left(8);
        asa ^= da; bcu = asa.rotate_left(18);
        let eka = bca ^ ((!bce) & bci);
        let eke = bce ^ ((!bci) & bco);
        let eki = bci ^ ((!bco) & bcu);
        let eko = bco ^ ((!bcu) & bca);
        let eku = bcu ^ ((!bca) & bce);

        abu ^= du; bca = abu.rotate_left(27);
        aga ^= da; bce = aga.rotate_left(36);
        ake ^= de; bci = ake.rotate_left(10);
        ami ^= di; bco = ami.rotate_left(15);
        aso ^= d_o; bcu = aso.rotate_left(56);
        let ema = bca ^ ((!bce) & bci);
        let eme = bce ^ ((!bci) & bco);
        let emi = bci ^ ((!bco) & bcu);
        let emo = bco ^ ((!bcu) & bca);
        let emu = bcu ^ ((!bca) & bce);

        abi ^= di; bca = abi.rotate_left(62);
        ago ^= d_o; bce = ago.rotate_left(55);
        aku ^= du; bci = aku.rotate_left(39);
        ama ^= da; bco = ama.rotate_left(41);
        ase ^= de; bcu = ase.rotate_left(2);
        let esa = bca ^ ((!bce) & bci);
        let ese = bce ^ ((!bci) & bco);
        let esi = bci ^ ((!bco) & bcu);
        let eso = bco ^ ((!bcu) & bca);
        let esu = bcu ^ ((!bca) & bce);

        bca = eba ^ ega ^ eka ^ ema ^ esa;
        bce = ebe ^ ege ^ eke ^ eme ^ ese;
        bci = ebi ^ egi ^ eki ^ emi ^ esi;
        bco = ebo ^ ego ^ eko ^ emo ^ eso;
        bcu = ebu ^ egu ^ eku ^ emu ^ esu;

        let da = bcu ^ bce.rotate_left(1);
        let de = bca ^ bci.rotate_left(1);
        let di = bce ^ bco.rotate_left(1);
        let d_o = bci ^ bcu.rotate_left(1);
        let du = bco ^ bca.rotate_left(1);

        { let t = eba ^ da; bca = t; }
        { let t = ege ^ de; bce = t.rotate_left(44); }
        { let t = eki ^ di; bci = t.rotate_left(43); }
        { let t = emo ^ d_o; bco = t.rotate_left(21); }
        { let t = esu ^ du; bcu = t.rotate_left(14); }
        aba = bca ^ ((!bce) & bci); aba ^= KECCAK_RC[round + 1];
        abe = bce ^ ((!bci) & bco);
        abi = bci ^ ((!bco) & bcu);
        abo = bco ^ ((!bcu) & bca);
        abu = bcu ^ ((!bca) & bce);

        { let t = ebo ^ d_o; bca = t.rotate_left(28); }
        { let t = egu ^ du; bce = t.rotate_left(20); }
        { let t = eka ^ da; bci = t.rotate_left(3); }
        { let t = eme ^ de; bco = t.rotate_left(45); }
        { let t = esi ^ di; bcu = t.rotate_left(61); }
        aga = bca ^ ((!bce) & bci);
        age = bce ^ ((!bci) & bco);
        agi = bci ^ ((!bco) & bcu);
        ago = bco ^ ((!bcu) & bca);
        agu = bcu ^ ((!bca) & bce);

        { let t = ebe ^ de; bca = t.rotate_left(1); }
        { let t = egi ^ di; bce = t.rotate_left(6); }
        { let t = eko ^ d_o; bci = t.rotate_left(25); }
        { let t = emu ^ du; bco = t.rotate_left(8); }
        { let t = esa ^ da; bcu = t.rotate_left(18); }
        aka = bca ^ ((!bce) & bci);
        ake = bce ^ ((!bci) & bco);
        aki = bci ^ ((!bco) & bcu);
        ako = bco ^ ((!bcu) & bca);
        aku = bcu ^ ((!bca) & bce);

        { let t = ebu ^ du; bca = t.rotate_left(27); }
        { let t = ega ^ da; bce = t.rotate_left(36); }
        { let t = eke ^ de; bci = t.rotate_left(10); }
        { let t = emi ^ di; bco = t.rotate_left(15); }
        { let t = eso ^ d_o; bcu = t.rotate_left(56); }
        ama = bca ^ ((!bce) & bci);
        ame = bce ^ ((!bci) & bco);
        ami = bci ^ ((!bco) & bcu);
        amo = bco ^ ((!bcu) & bca);
        amu = bcu ^ ((!bca) & bce);

        { let t = ebi ^ di; bca = t.rotate_left(62); }
        { let t = ego ^ d_o; bce = t.rotate_left(55); }
        { let t = eku ^ du; bci = t.rotate_left(39); }
        { let t = ema ^ da; bco = t.rotate_left(41); }
        { let t = ese ^ de; bcu = t.rotate_left(2); }
        asa = bca ^ ((!bce) & bci);
        ase = bce ^ ((!bci) & bco);
        asi = bci ^ ((!bco) & bcu);
        aso = bco ^ ((!bcu) & bca);
        asu = bcu ^ ((!bca) & bce);
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
    let mut off = 0;
    for _ in 0..nblocks {
        keccak_f1600(s);
        for i in 0..(r >> 3) {
            store64(&mut h[off + 8 * i..], s[i]);
        }
        off += r;
    }
}

pub fn shake256_inc_init(s_inc: &mut [u64; 26]) {
    for i in 0..26 { s_inc[i] = 0; }
}

pub fn shake256_inc_absorb(s_inc: &mut [u64; 26], input: &[u8]) {
    let r = SHAKE256_RATE;
    let mut off = 0usize;
    let mut mlen = input.len();
    while mlen + (s_inc[25] as usize) >= r {
        let to_absorb = r - s_inc[25] as usize;
        for i in 0..to_absorb {
            let pos = s_inc[25] as usize + i;
            s_inc[pos >> 3] ^= (input[off + i] as u64) << (8 * (pos & 0x07));
        }
        mlen -= to_absorb;
        off += to_absorb;
        s_inc[25] = 0;
        let state: &mut [u64; 25] = (&mut s_inc[..25]).try_into().unwrap();
        keccak_f1600(state);
    }
    for i in 0..mlen {
        let pos = s_inc[25] as usize + i;
        s_inc[pos >> 3] ^= (input[off + i] as u64) << (8 * (pos & 0x07));
    }
    s_inc[25] += mlen as u64;
}

pub fn shake256_inc_finalize(s_inc: &mut [u64; 26]) {
    let r = SHAKE256_RATE;
    let pos = s_inc[25] as usize;
    s_inc[pos >> 3] ^= 0x1Fu64 << (8 * (pos & 0x07));
    s_inc[(r - 1) >> 3] ^= 128u64 << (8 * ((r - 1) & 0x07));
    s_inc[25] = 0;
}

pub fn shake256_inc_squeeze(output: &mut [u8], outlen: usize, s_inc: &mut [u64; 26]) {
    let r = SHAKE256_RATE;
    let mut off = 0usize;
    let mut remaining = outlen;

    // consume leftover
    let avail = s_inc[25] as usize;
    let mut i = 0;
    while i < remaining && i < avail {
        let pos = r - avail + i;
        output[off] = (s_inc[pos >> 3] >> (8 * (pos & 0x07))) as u8;
        off += 1;
        i += 1;
    }
    remaining -= i;
    s_inc[25] -= i as u64;

    while remaining > 0 {
        let state: &mut [u64; 25] = (&mut s_inc[..25]).try_into().unwrap();
        keccak_f1600(state);
        let mut i = 0;
        while i < remaining && i < r {
            output[off] = (s_inc[i >> 3] >> (8 * (i & 0x07))) as u8;
            off += 1;
            i += 1;
        }
        remaining -= i;
        s_inc[25] = (r - i) as u64;
    }
}

pub fn shake256(output: &mut [u8], outlen: usize, input: &[u8]) {
    let nblocks = outlen / SHAKE256_RATE;
    let mut s = [0u64; 25];
    keccak_absorb(&mut s, SHAKE256_RATE, input, 0x1F);
    keccak_squeezeblocks(output, nblocks, &mut s, SHAKE256_RATE);
    let done = nblocks * SHAKE256_RATE;
    let left = outlen - done;
    if left > 0 {
        let mut t = [0u8; SHAKE256_RATE];
        keccak_squeezeblocks(&mut t, 1, &mut s, SHAKE256_RATE);
        output[done..done + left].copy_from_slice(&t[..left]);
    }
}
