use crate::{blake_impl::*, context::SpxCtx, params::*, utils::*};

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &mut [u32]) {
    let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&address_to_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&ctx.sk_seed);
    let digest = blake256(&buf[..SPX_N + SPX_ADDR_BYTES]);
    out[..SPX_N].copy_from_slice(&digest[..SPX_N]);
}

pub fn gen_message_random(
    out: &mut [u8], sk_prf: &[u8], optrand: &[u8], msg: &[u8], mlen: usize, _ctx: &SpxCtx,
) {
    let parts = [
        (&sk_prf[..SPX_N], SPX_N as u64),
        (&optrand[..SPX_N], SPX_N as u64),
        (&msg[..mlen], mlen as u64),
    ];
    if SPX_N >= 24 {
        out[..SPX_N].copy_from_slice(&blake512_updates(&parts)[..SPX_N]);
    } else {
        out[..SPX_N].copy_from_slice(&blake256_updates(&parts)[..SPX_N]);
    }
}

pub fn hash_message(
    digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32, r: &[u8], pk: &[u8],
    msg: &[u8], mlen: usize, _ctx: &SpxCtx,
) {
    let parts = [
        (&r[..SPX_N], SPX_N as u64),
        (&pk[..SPX_PK_BYTES], SPX_PK_BYTES as u64),
        (&msg[..mlen], mlen as u64),
    ];
    let hash = if SPX_N >= 24 { blake512_updates(&parts).to_vec() } else { blake256_updates(&parts).to_vec() };
    let mut seed = Vec::with_capacity(2 * SPX_N + hash.len());
    seed.extend_from_slice(&r[..SPX_N]);
    seed.extend_from_slice(&pk[..SPX_N]);
    seed.extend_from_slice(&hash);
    let mut buf = vec![0u8; SPX_DGST_BYTES];
    mgf1(&mut buf, &seed, SPX_N >= 24);
    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut i = SPX_FORS_MSG_BYTES;
    *tree = bytes_to_ull(&buf[i..], SPX_TREE_BYTES) & (!0u64 >> (64 - SPX_TREE_BITS));
    i += SPX_TREE_BYTES;
    *leaf_idx = bytes_to_ull(&buf[i..], SPX_LEAF_BYTES) as u32
        & (!0u32 >> (32 - SPX_LEAF_BITS));
}
