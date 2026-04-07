// SPHINCS+ SHAKE backend — translated from C (fips202.c, hash_shake.c, thash_shake_simple.c, thash_shake_robust.c)
use crate::context::SpxCtx;
use crate::params::*;

const SHAKE256_RATE: usize = 136;
const NROUNDS: usize = 24;

// Keccak round constants
static KECCAK_F_ROUND_CONSTANTS: [u64; NROUNDS] = [
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

#[inline]
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

fn keccak_f1600_state_permute(state: &mut [u64; 25]) {
    let (mut Aba, mut Abe, mut Abi, mut Abo, mut Abu) = (state[0], state[1], state[2], state[3], state[4]);
    let (mut Aga, mut Age, mut Agi, mut Ago, mut Agu) = (state[5], state[6], state[7], state[8], state[9]);
    let (mut Aka, mut Ake, mut Aki, mut Ako, mut Aku) = (state[10], state[11], state[12], state[13], state[14]);
    let (mut Ama, mut Ame, mut Ami, mut Amo, mut Amu) = (state[15], state[16], state[17], state[18], state[19]);
    let (mut Asa, mut Ase, mut Asi, mut Aso, mut Asu) = (state[20], state[21], state[22], state[23], state[24]);
    let (mut BCa, mut BCe, mut BCi, mut BCo, mut BCu);
    let (mut Da, mut De, mut Di, mut Do, mut Du);
    let (mut Eba, mut Ebe, mut Ebi, mut Ebo, mut Ebu);
    let (mut Ega, mut Ege, mut Egi, mut Ego, mut Egu);
    let (mut Eka, mut Eke, mut Eki, mut Eko, mut Eku);
    let (mut Ema, mut Eme, mut Emi, mut Emo, mut Emu);
    let (mut Esa, mut Ese, mut Esi, mut Eso, mut Esu);

    let mut round = 0;
    while round < NROUNDS {
        // prepareTheta
        BCa = Aba ^ Aga ^ Aka ^ Ama ^ Asa;
        BCe = Abe ^ Age ^ Ake ^ Ame ^ Ase;
        BCi = Abi ^ Agi ^ Aki ^ Ami ^ Asi;
        BCo = Abo ^ Ago ^ Ako ^ Amo ^ Aso;
        BCu = Abu ^ Agu ^ Aku ^ Amu ^ Asu;

        Da = BCu ^ rol(BCe, 1);
        De = BCa ^ rol(BCi, 1);
        Di = BCe ^ rol(BCo, 1);
        Do = BCi ^ rol(BCu, 1);
        Du = BCo ^ rol(BCa, 1);

        Aba ^= Da; BCa = Aba;
        Age ^= De; BCe = rol(Age, 44);
        Aki ^= Di; BCi = rol(Aki, 43);
        Amo ^= Do; BCo = rol(Amo, 21);
        Asu ^= Du; BCu = rol(Asu, 14);
        Eba = BCa ^ ((!BCe) & BCi); Eba ^= KECCAK_F_ROUND_CONSTANTS[round];
        Ebe = BCe ^ ((!BCi) & BCo);
        Ebi = BCi ^ ((!BCo) & BCu);
        Ebo = BCo ^ ((!BCu) & BCa);
        Ebu = BCu ^ ((!BCa) & BCe);

        Abo ^= Do; BCa = rol(Abo, 28);
        Agu ^= Du; BCe = rol(Agu, 20);
        Aka ^= Da; BCi = rol(Aka, 3);
        Ame ^= De; BCo = rol(Ame, 45);
        Asi ^= Di; BCu = rol(Asi, 61);
        Ega = BCa ^ ((!BCe) & BCi);
        Ege = BCe ^ ((!BCi) & BCo);
        Egi = BCi ^ ((!BCo) & BCu);
        Ego = BCo ^ ((!BCu) & BCa);
        Egu = BCu ^ ((!BCa) & BCe);

        Abe ^= De; BCa = rol(Abe, 1);
        Agi ^= Di; BCe = rol(Agi, 6);
        Ako ^= Do; BCi = rol(Ako, 25);
        Amu ^= Du; BCo = rol(Amu, 8);
        Asa ^= Da; BCu = rol(Asa, 18);
        Eka = BCa ^ ((!BCe) & BCi);
        Eke = BCe ^ ((!BCi) & BCo);
        Eki = BCi ^ ((!BCo) & BCu);
        Eko = BCo ^ ((!BCu) & BCa);
        Eku = BCu ^ ((!BCa) & BCe);

        Abu ^= Du; BCa = rol(Abu, 27);
        Aga ^= Da; BCe = rol(Aga, 36);
        Ake ^= De; BCi = rol(Ake, 10);
        Ami ^= Di; BCo = rol(Ami, 15);
        Aso ^= Do; BCu = rol(Aso, 56);
        Ema = BCa ^ ((!BCe) & BCi);
        Eme = BCe ^ ((!BCi) & BCo);
        Emi = BCi ^ ((!BCo) & BCu);
        Emo = BCo ^ ((!BCu) & BCa);
        Emu = BCu ^ ((!BCa) & BCe);

        Abi ^= Di; BCa = rol(Abi, 62);
        Ago ^= Do; BCe = rol(Ago, 55);
        Aku ^= Du; BCi = rol(Aku, 39);
        Ama ^= Da; BCo = rol(Ama, 41);
        Ase ^= De; BCu = rol(Ase, 2);
        Esa = BCa ^ ((!BCe) & BCi);
        Ese = BCe ^ ((!BCi) & BCo);
        Esi = BCi ^ ((!BCo) & BCu);
        Eso = BCo ^ ((!BCu) & BCa);
        Esu = BCu ^ ((!BCa) & BCe);

        // prepareTheta (round+1)
        BCa = Eba ^ Ega ^ Eka ^ Ema ^ Esa;
        BCe = Ebe ^ Ege ^ Eke ^ Eme ^ Ese;
        BCi = Ebi ^ Egi ^ Eki ^ Emi ^ Esi;
        BCo = Ebo ^ Ego ^ Eko ^ Emo ^ Eso;
        BCu = Ebu ^ Egu ^ Eku ^ Emu ^ Esu;

        Da = BCu ^ rol(BCe, 1);
        De = BCa ^ rol(BCi, 1);
        Di = BCe ^ rol(BCo, 1);
        Do = BCi ^ rol(BCu, 1);
        Du = BCo ^ rol(BCa, 1);

        Eba ^= Da; BCa = Eba;
        Ege ^= De; BCe = rol(Ege, 44);
        Eki ^= Di; BCi = rol(Eki, 43);
        Emo ^= Do; BCo = rol(Emo, 21);
        Esu ^= Du; BCu = rol(Esu, 14);
        Aba = BCa ^ ((!BCe) & BCi); Aba ^= KECCAK_F_ROUND_CONSTANTS[round + 1];
        Abe = BCe ^ ((!BCi) & BCo);
        Abi = BCi ^ ((!BCo) & BCu);
        Abo = BCo ^ ((!BCu) & BCa);
        Abu = BCu ^ ((!BCa) & BCe);

        Ebo ^= Do; BCa = rol(Ebo, 28);
        Egu ^= Du; BCe = rol(Egu, 20);
        Eka ^= Da; BCi = rol(Eka, 3);
        Eme ^= De; BCo = rol(Eme, 45);
        Esi ^= Di; BCu = rol(Esi, 61);
        Aga = BCa ^ ((!BCe) & BCi);
        Age = BCe ^ ((!BCi) & BCo);
        Agi = BCi ^ ((!BCo) & BCu);
        Ago = BCo ^ ((!BCu) & BCa);
        Agu = BCu ^ ((!BCa) & BCe);

        Ebe ^= De; BCa = rol(Ebe, 1);
        Egi ^= Di; BCe = rol(Egi, 6);
        Eko ^= Do; BCi = rol(Eko, 25);
        Emu ^= Du; BCo = rol(Emu, 8);
        Esa ^= Da; BCu = rol(Esa, 18);
        Aka = BCa ^ ((!BCe) & BCi);
        Ake = BCe ^ ((!BCi) & BCo);
        Aki = BCi ^ ((!BCo) & BCu);
        Ako = BCo ^ ((!BCu) & BCa);
        Aku = BCu ^ ((!BCa) & BCe);

        Ebu ^= Du; BCa = rol(Ebu, 27);
        Ega ^= Da; BCe = rol(Ega, 36);
        Eke ^= De; BCi = rol(Eke, 10);
        Emi ^= Di; BCo = rol(Emi, 15);
        Eso ^= Do; BCu = rol(Eso, 56);
        Ama = BCa ^ ((!BCe) & BCi);
        Ame = BCe ^ ((!BCi) & BCo);
        Ami = BCi ^ ((!BCo) & BCu);
        Amo = BCo ^ ((!BCu) & BCa);
        Amu = BCu ^ ((!BCa) & BCe);

        Ebi ^= Di; BCa = rol(Ebi, 62);
        Ego ^= Do; BCe = rol(Ego, 55);
        Eku ^= Du; BCi = rol(Eku, 39);
        Ema ^= Da; BCo = rol(Ema, 41);
        Ese ^= De; BCu = rol(Ese, 2);
        Asa = BCa ^ ((!BCe) & BCi);
        Ase = BCe ^ ((!BCi) & BCo);
        Asi = BCi ^ ((!BCo) & BCu);
        Aso = BCo ^ ((!BCu) & BCa);
        Asu = BCu ^ ((!BCa) & BCe);

        round += 2;
    }

    state[0] = Aba; state[1] = Abe; state[2] = Abi; state[3] = Abo; state[4] = Abu;
    state[5] = Aga; state[6] = Age; state[7] = Agi; state[8] = Ago; state[9] = Agu;
    state[10] = Aka; state[11] = Ake; state[12] = Aki; state[13] = Ako; state[14] = Aku;
    state[15] = Ama; state[16] = Ame; state[17] = Ami; state[18] = Amo; state[19] = Amu;
    state[20] = Asa; state[21] = Ase; state[22] = Asi; state[23] = Aso; state[24] = Asu;
}

fn keccak_absorb(s: &mut [u64; 25], r: usize, m: &[u8], mlen: usize, p: u8) {
    for i in 0..25 { s[i] = 0; }

    let mut off = 0usize;
    let mut remaining = mlen;

    while remaining >= r {
        for i in 0..(r / 8) {
            s[i] ^= load64(&m[off + 8 * i..]);
        }
        keccak_f1600_state_permute(s);
        remaining -= r;
        off += r;
    }

    let mut t = [0u8; 200];
    t[..remaining].copy_from_slice(&m[off..off + remaining]);
    t[remaining] = p;
    t[r - 1] |= 128;
    for i in 0..(r / 8) {
        s[i] ^= load64(&t[8 * i..]);
    }
}

fn keccak_squeezeblocks(h: &mut [u8], nblocks: usize, s: &mut [u64; 25], r: usize) {
    for blk in 0..nblocks {
        keccak_f1600_state_permute(s);
        for i in 0..(r >> 3) {
            store64(&mut h[blk * r + 8 * i..], s[i]);
        }
    }
}

fn keccak_inc_init(s_inc: &mut [u64; 26]) {
    for i in 0..26 { s_inc[i] = 0; }
}

fn keccak_inc_absorb(s_inc: &mut [u64; 26], r: usize, m: &[u8], mlen: usize) {
    let mut off = 0usize;
    let mut remaining = mlen;

    while remaining + (s_inc[25] as usize) >= r {
        let to_absorb = r - s_inc[25] as usize;
        for i in 0..to_absorb {
            let pos = s_inc[25] as usize + i;
            s_inc[pos >> 3] ^= (m[off + i] as u64) << (8 * (pos & 0x07));
        }
        remaining -= to_absorb;
        off += to_absorb;
        s_inc[25] = 0;

        let state: &mut [u64; 25] = (&mut s_inc[..25]).try_into().unwrap();
        keccak_f1600_state_permute(state);
    }

    for i in 0..remaining {
        let pos = s_inc[25] as usize + i;
        s_inc[pos >> 3] ^= (m[off + i] as u64) << (8 * (pos & 0x07));
    }
    s_inc[25] += remaining as u64;
}

fn keccak_inc_finalize(s_inc: &mut [u64; 26], r: usize, p: u8) {
    let pos = s_inc[25] as usize;
    s_inc[pos >> 3] ^= (p as u64) << (8 * (pos & 0x07));
    s_inc[(r - 1) >> 3] ^= 128u64 << (8 * ((r - 1) & 0x07));
    s_inc[25] = 0;
}

fn keccak_inc_squeeze(h: &mut [u8], mut outlen: usize, s_inc: &mut [u64; 26], r: usize) {
    let mut h_off = 0usize;

    // First consume leftover bytes
    let mut i = 0usize;
    while i < outlen && i < s_inc[25] as usize {
        let pos = r - s_inc[25] as usize + i;
        h[h_off + i] = (s_inc[pos >> 3] >> (8 * (pos & 0x07))) as u8;
        i += 1;
    }
    h_off += i;
    outlen -= i;
    s_inc[25] -= i as u64;

    // Squeeze remaining blocks
    while outlen > 0 {
        let state: &mut [u64; 25] = (&mut s_inc[..25]).try_into().unwrap();
        keccak_f1600_state_permute(state);

        let mut i = 0usize;
        while i < outlen && i < r {
            h[h_off + i] = (s_inc[i >> 3] >> (8 * (i & 0x07))) as u8;
            i += 1;
        }
        h_off += i;
        outlen -= i;
        s_inc[25] = (r - i) as u64;
    }
}

fn shake256_inc_init(s_inc: &mut [u64; 26]) {
    keccak_inc_init(s_inc);
}

fn shake256_inc_absorb(s_inc: &mut [u64; 26], input: &[u8], inlen: usize) {
    keccak_inc_absorb(s_inc, SHAKE256_RATE, input, inlen);
}

fn shake256_inc_finalize(s_inc: &mut [u64; 26]) {
    keccak_inc_finalize(s_inc, SHAKE256_RATE, 0x1F);
}

fn shake256_inc_squeeze(output: &mut [u8], outlen: usize, s_inc: &mut [u64; 26]) {
    keccak_inc_squeeze(output, outlen, s_inc, SHAKE256_RATE);
}

fn shake256_absorb(s: &mut [u64; 25], input: &[u8], inlen: usize) {
    keccak_absorb(s, SHAKE256_RATE, input, inlen, 0x1F);
}

fn shake256_squeezeblocks(output: &mut [u8], nblocks: usize, s: &mut [u64; 25]) {
    keccak_squeezeblocks(output, nblocks, s, SHAKE256_RATE);
}

fn shake256(output: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    let nblocks = outlen / SHAKE256_RATE;
    let mut s = [0u64; 25];

    shake256_absorb(&mut s, input, inlen);
    shake256_squeezeblocks(output, nblocks, &mut s);

    let done = nblocks * SHAKE256_RATE;
    let rem = outlen - done;

    if rem > 0 {
        let mut t = [0u8; SHAKE256_RATE];
        shake256_squeezeblocks(&mut t, 1, &mut s);
        output[done..done + rem].copy_from_slice(&t[..rem]);
    }
}

// ============ hash_shake.c ============

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {
    // No-op for SHAKE
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let addr_bytes: &[u8] = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);
    shake256(out, SPX_N, &buf, 2 * SPX_N + SPX_ADDR_BYTES);
}

pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mut s_inc = [0u64; 26];
    shake256_inc_init(&mut s_inc);
    shake256_inc_absorb(&mut s_inc, sk_prf, SPX_N);
    shake256_inc_absorb(&mut s_inc, optrand, SPX_N);
    shake256_inc_absorb(&mut s_inc, m, mlen as usize);
    shake256_inc_finalize(&mut s_inc);
    shake256_inc_squeeze(r, SPX_N, &mut s_inc);
}

pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                    r: &[u8], pk: &[u8], m: &[u8], mlen: u64, _ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut s_inc = [0u64; 26];

    shake256_inc_init(&mut s_inc);
    shake256_inc_absorb(&mut s_inc, r, SPX_N);
    shake256_inc_absorb(&mut s_inc, pk, SPX_PK_BYTES);
    shake256_inc_absorb(&mut s_inc, m, mlen as usize);
    shake256_inc_finalize(&mut s_inc);
    shake256_inc_squeeze(&mut buf, SPX_DGST_BYTES, &mut s_inc);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = crate::utils::bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = crate::utils::bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}

// ============ thash ============

#[cfg(all(feature = "simple", not(feature = "robust")))]
pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let addr_bytes: &[u8] = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..buf_len].copy_from_slice(&input[..inblocks * SPX_N]);
    shake256(out, SPX_N, &buf, buf_len);
}

#[cfg(all(feature = "robust", not(feature = "simple")))]
pub fn thash(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let addr_bytes: &[u8] = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut bitmask = vec![0u8; inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);

    shake256(&mut bitmask, inblocks * SPX_N, &buf[..SPX_N + SPX_ADDR_BYTES], SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }

    shake256(out, SPX_N, &buf, buf_len);
}

// FFI wrappers
pub fn shake256_ffi(output: &mut [u8], outlen: usize, input: &[u8], inlen: usize) {
    shake256(output, outlen, input, inlen);
}
pub fn shake256_absorb_ffi(s: &mut [u64; 25], input: &[u8], inlen: usize) {
    shake256_absorb(s, input, inlen);
}
pub fn shake256_squeezeblocks_ffi(output: &mut [u8], nblocks: usize, s: &mut [u64; 25]) {
    shake256_squeezeblocks(output, nblocks, s);
}
pub fn shake256_inc_init_ffi(s_inc: &mut [u64; 26]) {
    shake256_inc_init(s_inc);
}
pub fn shake256_inc_absorb_ffi(s_inc: &mut [u64; 26], input: &[u8], inlen: usize) {
    shake256_inc_absorb(s_inc, input, inlen);
}
pub fn shake256_inc_finalize_ffi(s_inc: &mut [u64; 26]) {
    shake256_inc_finalize(s_inc);
}
pub fn shake256_inc_squeeze_ffi(output: &mut [u8], outlen: usize, s_inc: &mut [u64; 26]) {
    shake256_inc_squeeze(output, outlen, s_inc);
}
