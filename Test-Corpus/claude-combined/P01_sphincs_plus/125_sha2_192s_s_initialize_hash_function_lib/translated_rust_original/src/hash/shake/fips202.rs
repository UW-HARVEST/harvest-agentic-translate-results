// Translation of c_src/lib/shake/src/fips202.c
// SHAKE-256 / Keccak-f1600

#![allow(clippy::needless_range_loop)]
#![allow(non_snake_case)]

use core::slice;

const NROUNDS: usize = 24;
pub const SHAKE256_RATE: usize = 136;

const KECCAKF_ROUND_CONSTANTS: [u64; NROUNDS] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a, 0x8000000080008000,
    0x000000000000808b, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008a, 0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b, 0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800a, 0x800000008000000a,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
];

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

fn keccak_f1600_state_permute(state: &mut [u64; 25]) {
    let mut Aba = state[0]; let mut Abe = state[1]; let mut Abi = state[2];
    let mut Abo = state[3]; let mut Abu = state[4];
    let mut Aga = state[5]; let mut Age = state[6]; let mut Agi = state[7];
    let mut Ago = state[8]; let mut Agu = state[9];
    let mut Aka = state[10]; let mut Ake = state[11]; let mut Aki = state[12];
    let mut Ako = state[13]; let mut Aku = state[14];
    let mut Ama = state[15]; let mut Ame = state[16]; let mut Ami = state[17];
    let mut Amo = state[18]; let mut Amu = state[19];
    let mut Asa = state[20]; let mut Ase = state[21]; let mut Asi = state[22];
    let mut Aso = state[23]; let mut Asu = state[24];

    let mut round = 0usize;
    while round < NROUNDS {
        let bca = Aba ^ Aga ^ Aka ^ Ama ^ Asa;
        let bce = Abe ^ Age ^ Ake ^ Ame ^ Ase;
        let bci = Abi ^ Agi ^ Aki ^ Ami ^ Asi;
        let bco = Abo ^ Ago ^ Ako ^ Amo ^ Aso;
        let bcu = Abu ^ Agu ^ Aku ^ Amu ^ Asu;

        let da = bcu ^ rol(bce, 1);
        let de = bca ^ rol(bci, 1);
        let di = bce ^ rol(bco, 1);
        let do_ = bci ^ rol(bcu, 1);
        let du = bco ^ rol(bca, 1);

        Aba ^= da; let mut bca = Aba;
        Age ^= de; let mut bce = rol(Age, 44);
        Aki ^= di; let mut bci = rol(Aki, 43);
        Amo ^= do_; let mut bco = rol(Amo, 21);
        Asu ^= du; let mut bcu = rol(Asu, 14);
        let mut Eba = bca ^ ((!bce) & bci); Eba ^= KECCAKF_ROUND_CONSTANTS[round];
        let Ebe = bce ^ ((!bci) & bco);
        let Ebi = bci ^ ((!bco) & bcu);
        let Ebo = bco ^ ((!bcu) & bca);
        let Ebu = bcu ^ ((!bca) & bce);

        Abo ^= do_; bca = rol(Abo, 28);
        Agu ^= du; bce = rol(Agu, 20);
        Aka ^= da; bci = rol(Aka, 3);
        Ame ^= de; bco = rol(Ame, 45);
        Asi ^= di; bcu = rol(Asi, 61);
        let Ega = bca ^ ((!bce) & bci);
        let Ege = bce ^ ((!bci) & bco);
        let Egi = bci ^ ((!bco) & bcu);
        let Ego = bco ^ ((!bcu) & bca);
        let Egu = bcu ^ ((!bca) & bce);

        Abe ^= de; bca = rol(Abe, 1);
        Agi ^= di; bce = rol(Agi, 6);
        Ako ^= do_; bci = rol(Ako, 25);
        Amu ^= du; bco = rol(Amu, 8);
        Asa ^= da; bcu = rol(Asa, 18);
        let Eka = bca ^ ((!bce) & bci);
        let Eke = bce ^ ((!bci) & bco);
        let Eki = bci ^ ((!bco) & bcu);
        let Eko = bco ^ ((!bcu) & bca);
        let Eku = bcu ^ ((!bca) & bce);

        Abu ^= du; bca = rol(Abu, 27);
        Aga ^= da; bce = rol(Aga, 36);
        Ake ^= de; bci = rol(Ake, 10);
        Ami ^= di; bco = rol(Ami, 15);
        Aso ^= do_; bcu = rol(Aso, 56);
        let Ema = bca ^ ((!bce) & bci);
        let Eme = bce ^ ((!bci) & bco);
        let Emi = bci ^ ((!bco) & bcu);
        let Emo = bco ^ ((!bcu) & bca);
        let Emu = bcu ^ ((!bca) & bce);

        Abi ^= di; bca = rol(Abi, 62);
        Ago ^= do_; bce = rol(Ago, 55);
        Aku ^= du; bci = rol(Aku, 39);
        Ama ^= da; bco = rol(Ama, 41);
        Ase ^= de; bcu = rol(Ase, 2);
        let Esa = bca ^ ((!bce) & bci);
        let Ese = bce ^ ((!bci) & bco);
        let Esi = bci ^ ((!bco) & bcu);
        let Eso = bco ^ ((!bcu) & bca);
        let Esu = bcu ^ ((!bca) & bce);

        let bca2 = Eba ^ Ega ^ Eka ^ Ema ^ Esa;
        let bce2 = Ebe ^ Ege ^ Eke ^ Eme ^ Ese;
        let bci2 = Ebi ^ Egi ^ Eki ^ Emi ^ Esi;
        let bco2 = Ebo ^ Ego ^ Eko ^ Emo ^ Eso;
        let bcu2 = Ebu ^ Egu ^ Eku ^ Emu ^ Esu;

        let da2 = bcu2 ^ rol(bce2, 1);
        let de2 = bca2 ^ rol(bci2, 1);
        let di2 = bce2 ^ rol(bco2, 1);
        let do_2 = bci2 ^ rol(bcu2, 1);
        let du2 = bco2 ^ rol(bca2, 1);

        let mut Eba_l = Eba ^ da2; let mut bca = Eba_l;
        let mut Ege_l = Ege ^ de2; let mut bce = rol(Ege_l, 44);
        let mut Eki_l = Eki ^ di2; let mut bci = rol(Eki_l, 43);
        let mut Emo_l = Emo ^ do_2; let mut bco = rol(Emo_l, 21);
        let mut Esu_l = Esu ^ du2; let mut bcu = rol(Esu_l, 14);
        let _ = (Eba_l, Ege_l, Eki_l, Emo_l, Esu_l);
        Aba = bca ^ ((!bce) & bci); Aba ^= KECCAKF_ROUND_CONSTANTS[round + 1];
        Abe = bce ^ ((!bci) & bco);
        Abi = bci ^ ((!bco) & bcu);
        Abo = bco ^ ((!bcu) & bca);
        Abu = bcu ^ ((!bca) & bce);

        let Ebo_l = Ebo ^ do_2; bca = rol(Ebo_l, 28);
        let Egu_l = Egu ^ du2; bce = rol(Egu_l, 20);
        let Eka_l = Eka ^ da2; bci = rol(Eka_l, 3);
        let Eme_l = Eme ^ de2; bco = rol(Eme_l, 45);
        let Esi_l = Esi ^ di2; bcu = rol(Esi_l, 61);
        Aga = bca ^ ((!bce) & bci);
        Age = bce ^ ((!bci) & bco);
        Agi = bci ^ ((!bco) & bcu);
        Ago = bco ^ ((!bcu) & bca);
        Agu = bcu ^ ((!bca) & bce);

        let Ebe_l = Ebe ^ de2; bca = rol(Ebe_l, 1);
        let Egi_l = Egi ^ di2; bce = rol(Egi_l, 6);
        let Eko_l = Eko ^ do_2; bci = rol(Eko_l, 25);
        let Emu_l = Emu ^ du2; bco = rol(Emu_l, 8);
        let Esa_l = Esa ^ da2; bcu = rol(Esa_l, 18);
        Aka = bca ^ ((!bce) & bci);
        Ake = bce ^ ((!bci) & bco);
        Aki = bci ^ ((!bco) & bcu);
        Ako = bco ^ ((!bcu) & bca);
        Aku = bcu ^ ((!bca) & bce);

        let Ebu_l = Ebu ^ du2; bca = rol(Ebu_l, 27);
        let Ega_l = Ega ^ da2; bce = rol(Ega_l, 36);
        let Eke_l = Eke ^ de2; bci = rol(Eke_l, 10);
        let Emi_l = Emi ^ di2; bco = rol(Emi_l, 15);
        let Eso_l = Eso ^ do_2; bcu = rol(Eso_l, 56);
        Ama = bca ^ ((!bce) & bci);
        Ame = bce ^ ((!bci) & bco);
        Ami = bci ^ ((!bco) & bcu);
        Amo = bco ^ ((!bcu) & bca);
        Amu = bcu ^ ((!bca) & bce);

        let Ebi_l = Ebi ^ di2; bca = rol(Ebi_l, 62);
        let Ego_l = Ego ^ do_2; bce = rol(Ego_l, 55);
        let Eku_l = Eku ^ du2; bci = rol(Eku_l, 39);
        let Ema_l = Ema ^ da2; bco = rol(Ema_l, 41);
        let Ese_l = Ese ^ de2; bcu = rol(Ese_l, 2);
        Asa = bca ^ ((!bce) & bci);
        Ase = bce ^ ((!bci) & bco);
        Asi = bci ^ ((!bco) & bcu);
        Aso = bco ^ ((!bcu) & bca);
        Asu = bcu ^ ((!bca) & bce);

        round += 2;
    }

    state[0] = Aba; state[1] = Abe; state[2] = Abi; state[3] = Abo; state[4] = Abu;
    state[5] = Aga; state[6] = Age; state[7] = Agi; state[8] = Ago; state[9] = Agu;
    state[10] = Aka; state[11] = Ake; state[12] = Aki; state[13] = Ako; state[14] = Aku;
    state[15] = Ama; state[16] = Ame; state[17] = Ami; state[18] = Amo; state[19] = Amu;
    state[20] = Asa; state[21] = Ase; state[22] = Asi; state[23] = Aso; state[24] = Asu;
}

fn keccak_absorb(s: &mut [u64; 25], r: usize, mut m: &[u8], mut mlen: usize, p: u8) {
    let mut t = [0u8; 200];
    for i in 0..25 { s[i] = 0; }
    while mlen >= r {
        for i in 0..(r / 8) {
            s[i] ^= load64(&m[8 * i..]);
        }
        keccak_f1600_state_permute(s);
        mlen -= r;
        m = &m[r..];
    }
    for i in 0..r { t[i] = 0; }
    for i in 0..mlen { t[i] = m[i]; }
    t[mlen] = p;
    t[r - 1] |= 128;
    for i in 0..(r / 8) {
        s[i] ^= load64(&t[8 * i..]);
    }
}

fn keccak_squeezeblocks(mut h: &mut [u8], mut nblocks: usize, s: &mut [u64; 25], r: usize) {
    while nblocks > 0 {
        keccak_f1600_state_permute(s);
        for i in 0..(r >> 3) {
            store64(&mut h[8 * i..], s[i]);
        }
        h = &mut h[r..];
        nblocks -= 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256(out: *mut u8, outlen: usize, input: *const u8, inlen: usize) {
    let out = unsafe { slice::from_raw_parts_mut(out, outlen) };
    let input = unsafe { slice::from_raw_parts(input, inlen) };
    shake256_inner(out, input);
}

pub fn shake256_inner(out: &mut [u8], input: &[u8]) {
    let mut s = [0u64; 25];
    let mut t = [0u8; SHAKE256_RATE];
    keccak_absorb(&mut s, SHAKE256_RATE, input, input.len(), 0x1F);
    let nblocks = out.len() / SHAKE256_RATE;
    let mut out_off = 0usize;
    {
        let mut full = nblocks;
        let mut h = &mut out[..];
        while full > 0 {
            keccak_f1600_state_permute(&mut s);
            for i in 0..(SHAKE256_RATE >> 3) {
                store64(&mut h[8 * i..], s[i]);
            }
            h = &mut h[SHAKE256_RATE..];
            full -= 1;
        }
    }
    out_off += nblocks * SHAKE256_RATE;
    let outlen = out.len() - out_off;
    if outlen > 0 {
        keccak_squeezeblocks(&mut t, 1, &mut s, SHAKE256_RATE);
        for i in 0..outlen {
            out[out_off + i] = t[i];
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_init(s_inc: *mut u64) {
    let s_inc = unsafe { slice::from_raw_parts_mut(s_inc, 26) };
    for i in 0..26 { s_inc[i] = 0; }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_absorb(s_inc: *mut u64, input: *const u8, inlen: usize) {
    let s_inc = unsafe { slice::from_raw_parts_mut(s_inc, 26) };
    let input = unsafe { slice::from_raw_parts(input, inlen) };
    keccak_inc_absorb(s_inc, SHAKE256_RATE, input);
}

fn keccak_inc_absorb(s_inc: &mut [u64], r: usize, mut m: &[u8]) {
    let mut mlen = m.len();
    while mlen + (s_inc[25] as usize) >= r {
        let cap = r - s_inc[25] as usize;
        for i in 0..cap {
            let pos = (s_inc[25] as usize) + i;
            s_inc[pos >> 3] ^= (m[i] as u64) << (8 * (pos & 0x07));
        }
        mlen -= cap;
        m = &m[cap..];
        s_inc[25] = 0;
        let s_arr_ptr = s_inc.as_mut_ptr() as *mut [u64; 25];
        unsafe { keccak_f1600_state_permute(&mut *s_arr_ptr); }
    }
    for i in 0..mlen {
        let pos = (s_inc[25] as usize) + i;
        s_inc[pos >> 3] ^= (m[i] as u64) << (8 * (pos & 0x07));
    }
    s_inc[25] += mlen as u64;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_finalize(s_inc: *mut u64) {
    let s_inc = unsafe { slice::from_raw_parts_mut(s_inc, 26) };
    keccak_inc_finalize(s_inc, SHAKE256_RATE, 0x1F);
}

fn keccak_inc_finalize(s_inc: &mut [u64], r: usize, p: u8) {
    let pos = s_inc[25] as usize;
    s_inc[pos >> 3] ^= (p as u64) << (8 * (pos & 0x07));
    s_inc[(r - 1) >> 3] ^= 128u64 << (8 * ((r - 1) & 0x07));
    s_inc[25] = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_inc_squeeze(out: *mut u8, outlen: usize, s_inc: *mut u64) {
    let out = unsafe { slice::from_raw_parts_mut(out, outlen) };
    let s_inc = unsafe { slice::from_raw_parts_mut(s_inc, 26) };
    keccak_inc_squeeze(out, s_inc, SHAKE256_RATE);
}

fn keccak_inc_squeeze(mut out: &mut [u8], s_inc: &mut [u64], r: usize) {
    let mut outlen = out.len();
    let mut i: usize = 0;
    while i < outlen && i < s_inc[25] as usize {
        let pos = r - s_inc[25] as usize + i;
        out[i] = (s_inc[pos >> 3] >> (8 * (pos & 0x07))) as u8;
        i += 1;
    }
    out = &mut out[i..];
    outlen -= i;
    s_inc[25] -= i as u64;

    while outlen > 0 {
        let s_arr_ptr = s_inc.as_mut_ptr() as *mut [u64; 25];
        unsafe { keccak_f1600_state_permute(&mut *s_arr_ptr); }

        let mut j: usize = 0;
        while j < outlen && j < r {
            out[j] = (s_inc[j >> 3] >> (8 * (j & 0x07))) as u8;
            j += 1;
        }
        out = &mut out[j..];
        outlen -= j;
        s_inc[25] = (r - j) as u64;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_absorb(s: *mut u64, input: *const u8, inlen: usize) {
    let s = unsafe { slice::from_raw_parts_mut(s, 25) };
    let input = unsafe { slice::from_raw_parts(input, inlen) };
    let s_arr_ptr = s.as_mut_ptr() as *mut [u64; 25];
    keccak_absorb(unsafe { &mut *s_arr_ptr }, SHAKE256_RATE, input, input.len(), 0x1F);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shake256_squeezeblocks(out: *mut u8, nblocks: usize, s: *mut u64) {
    let out = unsafe { slice::from_raw_parts_mut(out, nblocks * SHAKE256_RATE) };
    let s = unsafe { slice::from_raw_parts_mut(s, 25) };
    let s_arr_ptr = s.as_mut_ptr() as *mut [u64; 25];
    keccak_squeezeblocks(out, nblocks, unsafe { &mut *s_arr_ptr }, SHAKE256_RATE);
}

// Pure Rust API for internal use
pub fn shake256_inc_init_inner(s_inc: &mut [u64]) {
    for i in 0..26 { s_inc[i] = 0; }
}
pub fn shake256_inc_absorb_inner(s_inc: &mut [u64], m: &[u8]) {
    keccak_inc_absorb(s_inc, SHAKE256_RATE, m);
}
pub fn shake256_inc_finalize_inner(s_inc: &mut [u64]) {
    keccak_inc_finalize(s_inc, SHAKE256_RATE, 0x1F);
}
pub fn shake256_inc_squeeze_inner(out: &mut [u8], s_inc: &mut [u64]) {
    keccak_inc_squeeze(out, s_inc, SHAKE256_RATE);
}
