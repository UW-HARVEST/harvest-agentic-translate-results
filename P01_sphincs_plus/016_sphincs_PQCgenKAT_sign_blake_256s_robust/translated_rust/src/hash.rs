use crate::address::addr_bytes;
use crate::blake256::{blake256, blake256_mgf1};
use crate::blake512::{blake512, blake512_mgf1};
use crate::params::*;

pub struct SpxCtx {
    pub pub_seed: [u8; SPX_N],
    pub sk_seed: [u8; SPX_N],
}

pub fn initialize_hash_function(_ctx: &SpxCtx) {}

/// thash for blake robust variant
/// For inblocks > 1, uses blake512; for inblocks == 1, uses blake256
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));

    blake256_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake256(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));

    blake512_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake512(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + inblocks * SPX_N) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// prf_addr: uses blake256 only, hashes pub_seed||addr (SPX_N + SPX_ADDR_BYTES bytes)
pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + SPX_N].copy_from_slice(&ctx.sk_seed);

    blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// gen_message_random: uses blake512 (since SPX_N >= 24)
pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], _ctx: &SpxCtx) {
    use crate::blake512::{blake512_init, blake512_update, blake512_final, Blake512State};
    let mut s = Blake512State {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128],
    };
    blake512_init(&mut s);
    blake512_update(&mut s, sk_prf, (SPX_N as u64) * 8);
    blake512_update(&mut s, optrand, (SPX_N as u64) * 8);
    blake512_update(&mut s, m, (m.len() as u64) * 8);
    let mut out = [0u8; 64];
    blake512_final(&mut s, &mut out);
    r[..SPX_N].copy_from_slice(&out[..SPX_N]);
}

/// hash_message: uses blake512 (since SPX_N >= 24)
pub fn hash_message(
    digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
    r: &[u8], pk: &[u8], m: &[u8], _ctx: &SpxCtx,
) {
    use crate::blake512::{blake512_init, blake512_update, blake512_final, Blake512State};
    use crate::utils::bytes_to_ull;

    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = [0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    let mut s = Blake512State {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128],
    };
    blake512_init(&mut s);
    blake512_update(&mut s, r, (SPX_N as u64) * 8);
    blake512_update(&mut s, pk, (SPX_PK_BYTES as u64) * 8);
    blake512_update(&mut s, m, (m.len() as u64) * 8);
    blake512_final(&mut s, &mut seed[2 * SPX_N..]);

    seed[..SPX_N].copy_from_slice(r);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    blake512_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut off = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[off..off + SPX_TREE_BYTES]);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    off += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[off..off + SPX_LEAF_BYTES]) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
