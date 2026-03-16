use crate::params::*;
use crate::context::SpxCtx;
use crate::blake256::*;
use crate::utils::bytes_to_ull;

// For SPX_N=16 < 24, we use blake256 as blakeX
pub fn initialize_hash_function(_ctx: &mut SpxCtx) {
    // no-op for blake
}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);

    blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
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
    // For SPX_N < 24, blakeX = blake256
    let mut s = Blake256State {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64],
    };
    blake256_init(&mut s);
    blake256_update(&mut s, &sk_prf[..SPX_N], (SPX_N as u64) * 8);
    blake256_update(&mut s, &optrand[..SPX_N], (SPX_N as u64) * 8);
    blake256_update(&mut s, &m[..mlen as usize], mlen * 8);
    blake256_final(&mut s, r);
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
    let mut buf = [0u8; SPX_DGST_BYTES];
    // For SPX_N < 24: blakeX = blake256, SPX_BLAKEX_OUTPUT_BYTES = 32
    let blakex_output = SPX_BLAKE256_OUTPUT_BYTES;
    let mut seed = vec![0u8; 2 * SPX_N + blakex_output];

    let mut s = Blake256State {
        h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64],
    };
    blake256_init(&mut s);
    blake256_update(&mut s, &r[..SPX_N], (SPX_N as u64) * 8);
    blake256_update(&mut s, &pk[..SPX_PK_BYTES], (SPX_PK_BYTES as u64) * 8);
    blake256_update(&mut s, &m[..mlen as usize], mlen * 8);
    blake256_final(&mut s, &mut seed[2 * SPX_N..]);

    seed[..SPX_N].copy_from_slice(&r[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    blake256_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + blakex_output);

    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut off = SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[off..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    off += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[off..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
