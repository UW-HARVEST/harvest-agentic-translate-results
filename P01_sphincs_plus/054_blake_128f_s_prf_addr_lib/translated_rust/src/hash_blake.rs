use crate::blake256::{self, SPX_BLAKE256_OUTPUT_BYTES};
use crate::context::SpxCtx;
use crate::params::*;

// For blake-128f: SPX_N=16 < 24, so blakeX = blake256
// SPX_BLAKEX_OUTPUT_BYTES = SPX_BLAKE256_OUTPUT_BYTES

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe {
        core::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES)
    };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + SPX_N]
        .copy_from_slice(&ctx.sk_seed);

    blake256::blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    mlen: u64,
    _ctx: &SpxCtx,
) {
    let mut s = blake256::Blakestate256 {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64],
    };
    blake256::blake256_init(&mut s);
    blake256::blake256_update(&mut s, sk_prf, (SPX_N as u64) * 8);
    blake256::blake256_update(&mut s, optrand, (SPX_N as u64) * 8);
    blake256::blake256_update(&mut s, m, mlen * 8);
    blake256::blake256_final(&mut s, r);
}

pub fn hash_message(
    digest: &mut [u8],
    tree: &mut u64,
    leaf_idx: &mut u32,
    r: &[u8],
    pk: &[u8],
    m: &[u8],
    mlen: u64,
    _ctx: &SpxCtx,
) {
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = [0u8; 2 * SPX_N + SPX_BLAKE256_OUTPUT_BYTES];

    let mut s = blake256::Blakestate256 {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64],
    };
    blake256::blake256_init(&mut s);
    blake256::blake256_update(&mut s, r, (SPX_N as u64) * 8);
    blake256::blake256_update(&mut s, pk, (SPX_PK_BYTES as u64) * 8);
    blake256::blake256_update(&mut s, m, mlen * 8);
    blake256::blake256_final(&mut s, &mut seed[2 * SPX_N..]);

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    blake256::blake256_mgf1(
        &mut buf,
        SPX_DGST_BYTES,
        &seed,
        2 * SPX_N + SPX_BLAKE256_OUTPUT_BYTES,
    );

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);

    let bufp = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = crate::address::bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }

    let bufp2 = bufp + SPX_TREE_BYTES;
    *leaf_idx = crate::address::bytes_to_ull(&buf[bufp2..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
