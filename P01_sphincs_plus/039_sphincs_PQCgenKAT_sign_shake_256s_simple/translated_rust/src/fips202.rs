const NROUNDS: usize = 24;
const SHAKE256_RATE: usize = 136;

const KECCAK_RC: [u64; 24] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
    0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
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

        aba ^= da; let mut c0 = aba;
        age ^= de; let mut c1 = age.rotate_left(44);
        aki ^= di; let mut c2 = aki.rotate_left(43);
        amo ^= d_o; let mut c3 = amo.rotate_left(21);
        asu ^= du; let mut c4 = asu.rotate_left(14);
        let mut eba = c0 ^ ((!c1) & c2); eba ^= KECCAK_RC[round];
        let ebe = c1 ^ ((!c2) & c3);
        let ebi = c2 ^ ((!c3) & c4);
        let ebo = c3 ^ ((!c4) & c0);
        let ebu = c4 ^ ((!c0) & c1);

        abo ^= d_o; c0 = abo.rotate_left(28);
        agu ^= du; c1 = agu.rotate_left(20);
        aka ^= da; c2 = aka.rotate_left(3);
        ame ^= de; c3 = ame.rotate_left(45);
        asi ^= di; c4 = asi.rotate_left(61);
        let ega = c0 ^ ((!c1) & c2);
        let ege = c1 ^ ((!c2) & c3);
        let egi = c2 ^ ((!c3) & c4);
        let ego = c3 ^ ((!c4) & c0);
        let egu = c4 ^ ((!c0) & c1);

        abe ^= de; c0 = abe.rotate_left(1);
        agi ^= di; c1 = agi.rotate_left(6);
        ako ^= d_o; c2 = ako.rotate_left(25);
        amu ^= du; c3 = amu.rotate_left(8);
        asa ^= da; c4 = asa.rotate_left(18);
        let eka = c0 ^ ((!c1) & c2);
        let eke = c1 ^ ((!c2) & c3);
        let eki = c2 ^ ((!c3) & c4);
        let eko = c3 ^ ((!c4) & c0);
        let eku = c4 ^ ((!c0) & c1);

        abu ^= du; c0 = abu.rotate_left(27);
        aga ^= da; c1 = aga.rotate_left(36);
        ake ^= de; c2 = ake.rotate_left(10);
        ami ^= di; c3 = ami.rotate_left(15);
        aso ^= d_o; c4 = aso.rotate_left(56);
        let ema = c0 ^ ((!c1) & c2);
        let eme = c1 ^ ((!c2) & c3);
        let emi = c2 ^ ((!c3) & c4);
        let emo = c3 ^ ((!c4) & c0);
        let emu = c4 ^ ((!c0) & c1);

        abi ^= di; c0 = abi.rotate_left(62);
        ago ^= d_o; c1 = ago.rotate_left(55);
        aku ^= du; c2 = aku.rotate_left(39);
        ama ^= da; c3 = ama.rotate_left(41);
        ase ^= de; c4 = ase.rotate_left(2);
        let esa = c0 ^ ((!c1) & c2);
        let ese = c1 ^ ((!c2) & c3);
        let esi = c2 ^ ((!c3) & c4);
        let eso = c3 ^ ((!c4) & c0);
        let esu = c4 ^ ((!c0) & c1);

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

        let mut t0 = eba ^ da; c0 = t0;
        let mut t1 = ege ^ de; c1 = t1.rotate_left(44);
        let mut t2 = eki ^ di; c2 = t2.rotate_left(43);
        let mut t3 = emo ^ d_o; c3 = t3.rotate_left(21);
        let mut t4 = esu ^ du; c4 = t4.rotate_left(14);
        aba = c0 ^ ((!c1) & c2); aba ^= KECCAK_RC[round + 1];
        abe = c1 ^ ((!c2) & c3);
        abi = c2 ^ ((!c3) & c4);
        abo = c3 ^ ((!c4) & c0);
        abu = c4 ^ ((!c0) & c1);

        t0 = ebo ^ d_o; c0 = t0.rotate_left(28);
        t1 = egu ^ du; c1 = t1.rotate_left(20);
        t2 = eka ^ da; c2 = t2.rotate_left(3);
        t3 = eme ^ de; c3 = t3.rotate_left(45);
        t4 = esi ^ di; c4 = t4.rotate_left(61);
        aga = c0 ^ ((!c1) & c2);
        age = c1 ^ ((!c2) & c3);
        agi = c2 ^ ((!c3) & c4);
        ago = c3 ^ ((!c4) & c0);
        agu = c4 ^ ((!c0) & c1);

        t0 = ebe ^ de; c0 = t0.rotate_left(1);
        t1 = egi ^ di; c1 = t1.rotate_left(6);
        t2 = eko ^ d_o; c2 = t2.rotate_left(25);
        t3 = emu ^ du; c3 = t3.rotate_left(8);
        t4 = esa ^ da; c4 = t4.rotate_left(18);
        aka = c0 ^ ((!c1) & c2);
        ake = c1 ^ ((!c2) & c3);
        aki = c2 ^ ((!c3) & c4);
        ako = c3 ^ ((!c4) & c0);
        aku = c4 ^ ((!c0) & c1);

        t0 = ebu ^ du; c0 = t0.rotate_left(27);
        t1 = ega ^ da; c1 = t1.rotate_left(36);
        t2 = eke ^ de; c2 = t2.rotate_left(10);
        t3 = emi ^ di; c3 = t3.rotate_left(15);
        t4 = eso ^ d_o; c4 = t4.rotate_left(56);
        ama = c0 ^ ((!c1) & c2);
        ame = c1 ^ ((!c2) & c3);
        ami = c2 ^ ((!c3) & c4);
        amo = c3 ^ ((!c4) & c0);
        amu = c4 ^ ((!c0) & c1);

        t0 = ebi ^ di; c0 = t0.rotate_left(62);
        t1 = ego ^ d_o; c1 = t1.rotate_left(55);
        t2 = eku ^ du; c2 = t2.rotate_left(39);
        t3 = ema ^ da; c3 = t3.rotate_left(41);
        t4 = ese ^ de; c4 = t4.rotate_left(2);
        asa = c0 ^ ((!c1) & c2);
        ase = c1 ^ ((!c2) & c3);
        asi = c2 ^ ((!c3) & c4);
        aso = c3 ^ ((!c4) & c0);
        asu = c4 ^ ((!c0) & c1);
    }

    state[0] = aba; state[1] = abe; state[2] = abi; state[3] = abo; state[4] = abu;
    state[5] = aga; state[6] = age; state[7] = agi; state[8] = ago; state[9] = agu;
    state[10] = aka; state[11] = ake; state[12] = aki; state[13] = ako; state[14] = aku;
    state[15] = ama; state[16] = ame; state[17] = ami; state[18] = amo; state[19] = amu;
    state[20] = asa; state[21] = ase; state[22] = asi; state[23] = aso; state[24] = asu;
}

fn keccak_absorb(s: &mut [u64; 25], r: usize, m: &[u8], p: u8) {
    let mut pos = 0usize;
    let mlen = m.len();
    for i in 0..25 { s[i] = 0; }
    let mut remaining = mlen;
    while remaining >= r {
        for i in 0..(r / 8) {
            s[i] ^= load64(&m[pos + 8 * i..]);
        }
        keccak_f1600(s);
        remaining -= r;
        pos += r;
    }
    let mut t = [0u8; 200];
    for i in 0..remaining {
        t[i] = m[pos + i];
    }
    t[remaining] = p;
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
    keccak_inc_absorb(s_inc, SHAKE256_RATE, input);
}

pub fn shake256_inc_finalize(s_inc: &mut [u64; 26]) {
    keccak_inc_finalize(s_inc, SHAKE256_RATE, 0x1F);
}

pub fn shake256_inc_squeeze(output: &mut [u8], outlen: usize, s_inc: &mut [u64; 26]) {
    keccak_inc_squeeze(output, outlen, s_inc, SHAKE256_RATE);
}

fn keccak_inc_absorb(s_inc: &mut [u64; 26], r: usize, m: &[u8]) {
    let mut pos = 0usize;
    let mut mlen = m.len();
    while mlen + (s_inc[25] as usize) >= r {
        let avail = r - s_inc[25] as usize;
        for i in 0..avail {
            let idx = s_inc[25] as usize + i;
            s_inc[idx >> 3] ^= (m[pos + i] as u64) << (8 * (idx & 0x07));
        }
        mlen -= avail;
        pos += avail;
        s_inc[25] = 0;
        let state: &mut [u64; 25] = (&mut s_inc[..25]).try_into().unwrap();
        keccak_f1600(state);
    }
    for i in 0..mlen {
        let idx = s_inc[25] as usize + i;
        s_inc[idx >> 3] ^= (m[pos + i] as u64) << (8 * (idx & 0x07));
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
    let mut hoff = 0usize;
    let mut i = 0usize;
    while i < outlen && i < s_inc[25] as usize {
        let idx = r - s_inc[25] as usize + i;
        h[hoff] = (s_inc[idx >> 3] >> (8 * (idx & 0x07))) as u8;
        hoff += 1;
        i += 1;
    }
    outlen -= i;
    s_inc[25] -= i as u64;

    while outlen > 0 {
        let state: &mut [u64; 25] = (&mut s_inc[..25]).try_into().unwrap();
        keccak_f1600(state);
        i = 0;
        while i < outlen && i < r {
            h[hoff] = (s_inc[i >> 3] >> (8 * (i & 0x07))) as u8;
            hoff += 1;
            i += 1;
        }
        outlen -= i;
        s_inc[25] = (r - i) as u64;
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
