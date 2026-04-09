use crate::params::*;
use crate::context::SpxCtx;
use crate::utils::bytes_to_ull;
use crate::blake::blake256::{BlakeState256, blake256_init, blake256_update, blake256_final, blake256_fn, blake256_mgf1};
use crate::blake::blake512::{BlakeState512, blake512_init, blake512_update, blake512_final, blake512_fn, blake512_mgf1};

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { &*(addr as *const [u32; 8] as *const [u8; SPX_ADDR_BYTES]) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);

    blake256_fn(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn gen_message_random(r: &mut [u8], sk_prf: &[u8], optrand: &[u8], m: &[u8], _mlen: u64, _ctx: &SpxCtx) {
    if SPX_BLAKE512 {
        let mut s = BlakeState512 { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128] };
        blake512_init(&mut s);
        blake512_update(&mut s, &sk_prf[..SPX_N], SPX_N as u64);
        blake512_update(&mut s, &optrand[..SPX_N], SPX_N as u64);
        blake512_update(&mut s, m, m.len() as u64);
        blake512_final(&mut s, r);
    } else {
        let mut s = BlakeState256 { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64] };
        blake256_init(&mut s);
        blake256_update(&mut s, &sk_prf[..SPX_N], SPX_N as u64);
        blake256_update(&mut s, &optrand[..SPX_N], SPX_N as u64);
        blake256_update(&mut s, m, m.len() as u64);
        blake256_final(&mut s, r);
    }
}

pub fn hash_message(digest: &mut [u8], tree: &mut u64, leaf_idx: &mut u32,
                    r_val: &[u8], pk: &[u8], m: &[u8], _mlen: u64, _ctx: &SpxCtx) {
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = vec![0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    if SPX_BLAKE512 {
        let mut s = BlakeState512 { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 128] };
        blake512_init(&mut s);
        blake512_update(&mut s, &r_val[..SPX_N], SPX_N as u64);
        blake512_update(&mut s, &pk[..SPX_PK_BYTES], SPX_PK_BYTES as u64);
        blake512_update(&mut s, m, m.len() as u64);
        blake512_final(&mut s, &mut seed[2 * SPX_N..]);
    } else {
        let mut s = BlakeState256 { h: [0; 8], s: [0; 4], t: [0; 2], buflen: 0, nullt: 0, buf: [0; 64] };
        blake256_init(&mut s);
        blake256_update(&mut s, &r_val[..SPX_N], SPX_N as u64);
        blake256_update(&mut s, &pk[..SPX_PK_BYTES], SPX_PK_BYTES as u64);
        blake256_update(&mut s, m, m.len() as u64);
        blake256_final(&mut s, &mut seed[2 * SPX_N..]);
    }

    seed[..SPX_N].copy_from_slice(&r_val[..SPX_N]);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    if SPX_BLAKE512 {
        blake512_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES);
    } else {
        blake256_mgf1(&mut buf, SPX_DGST_BYTES, &seed, 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES);
    }

    let mut bufp = 0;
    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[bufp..bufp + SPX_FORS_MSG_BYTES]);
    bufp += SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= (!0u64) >> (64 - SPX_TREE_BITS);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= (!0u32) >> (32 - SPX_LEAF_BITS);
}
