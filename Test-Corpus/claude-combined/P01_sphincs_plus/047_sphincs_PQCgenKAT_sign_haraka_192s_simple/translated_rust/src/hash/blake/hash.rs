// Translation of c_src/lib/blake/src/hash_blake.c

use core::slice;

use crate::context::SpxCtx;
use crate::params::{
    SPX_ADDR_BYTES, SPX_BLAKE256_OUTPUT_BYTES, SPX_BLAKE512_OUTPUT_BYTES, SPX_D, SPX_FORS_MSG_BYTES,
    SPX_N, SPX_PK_BYTES, SPX_TREE_HEIGHT,
};
use crate::utils::bytes_to_ull;

use super::blake256::{
    blake256_final_inner, blake256_init_inner, blake256_oneshot, blake256_update_inner,
    blake256_mgf1_inner, BlakeState256,
};
use super::blake512::{
    blake512_final_inner, blake512_init_inner, blake512_update_inner,
    blake512_mgf1_inner, BlakeState512,
};

const fn blakex_output_bytes() -> usize {
    if SPX_N >= 24 { SPX_BLAKE512_OUTPUT_BYTES } else { SPX_BLAKE256_OUTPUT_BYTES }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_initialize_hash_function(_ctx: *mut SpxCtx) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_prf_addr(out: *mut u8, ctx: *const SpxCtx, addr: *const u32) {
    let out = unsafe { slice::from_raw_parts_mut(out, SPX_N) };
    let ctx = unsafe { &*ctx };
    let addr = unsafe { slice::from_raw_parts(addr, 8) };
    let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);
    // Note: C uses blake256(outbuf, buf, SPX_N + SPX_ADDR_BYTES) — only first SPX_N + SPX_ADDR_BYTES bytes
    blake256_oneshot(&mut outbuf, &buf[..SPX_N + SPX_ADDR_BYTES]);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_gen_message_random(
    r: *mut u8,
    sk_prf: *const u8,
    optrand: *const u8,
    m: *const u8,
    mlen: u64,
    _ctx: *const SpxCtx,
) {
    let r = unsafe { slice::from_raw_parts_mut(r, SPX_N) };
    let sk_prf = unsafe { slice::from_raw_parts(sk_prf, SPX_N) };
    let optrand = unsafe { slice::from_raw_parts(optrand, SPX_N) };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };

    // NOTE: matches C bug — C passes raw byte counts here instead of bits
    if SPX_N >= 24 {
        let mut s: BlakeState512 = unsafe { core::mem::zeroed() };
        blake512_init_inner(&mut s);
        blake512_update_inner(&mut s, sk_prf, SPX_N as u64);
        blake512_update_inner(&mut s, optrand, SPX_N as u64);
        blake512_update_inner(&mut s, m, mlen);
        let mut full = [0u8; 64];
        blake512_final_inner(&mut s, &mut full);
        r.copy_from_slice(&full[..SPX_N]);
    } else {
        let mut s: BlakeState256 = unsafe { core::mem::zeroed() };
        blake256_init_inner(&mut s);
        blake256_update_inner(&mut s, sk_prf, SPX_N as u64);
        blake256_update_inner(&mut s, optrand, SPX_N as u64);
        blake256_update_inner(&mut s, m, mlen);
        let mut full = [0u8; 32];
        blake256_final_inner(&mut s, &mut full);
        r.copy_from_slice(&full[..SPX_N]);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_hash_message(
    digest: *mut u8,
    tree: *mut u64,
    leaf_idx: *mut u32,
    r: *const u8,
    pk: *const u8,
    m: *const u8,
    mlen: u64,
    _ctx: *const SpxCtx,
) {
    const SPX_TREE_BITS: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let digest = unsafe { slice::from_raw_parts_mut(digest, SPX_FORS_MSG_BYTES) };
    let r_slice = unsafe { slice::from_raw_parts(r, SPX_N) };
    let pk_slice = unsafe { slice::from_raw_parts(pk, SPX_PK_BYTES) };
    let m = unsafe { slice::from_raw_parts(m, mlen as usize) };

    let ob = blakex_output_bytes();
    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = vec![0u8; 2 * SPX_N + ob];

    // NOTE: matches C bug — C passes raw byte counts here instead of bits
    if SPX_N >= 24 {
        let mut s: BlakeState512 = unsafe { core::mem::zeroed() };
        blake512_init_inner(&mut s);
        blake512_update_inner(&mut s, r_slice, SPX_N as u64);
        blake512_update_inner(&mut s, pk_slice, SPX_PK_BYTES as u64);
        blake512_update_inner(&mut s, m, mlen);
        let mut tmp = [0u8; 64];
        blake512_final_inner(&mut s, &mut tmp);
        seed[2 * SPX_N..2 * SPX_N + ob].copy_from_slice(&tmp[..ob]);
    } else {
        let mut s: BlakeState256 = unsafe { core::mem::zeroed() };
        blake256_init_inner(&mut s);
        blake256_update_inner(&mut s, r_slice, SPX_N as u64);
        blake256_update_inner(&mut s, pk_slice, SPX_PK_BYTES as u64);
        blake256_update_inner(&mut s, m, mlen);
        let mut tmp = [0u8; 32];
        blake256_final_inner(&mut s, &mut tmp);
        seed[2 * SPX_N..2 * SPX_N + ob].copy_from_slice(&tmp[..ob]);
    }
    seed[..SPX_N].copy_from_slice(r_slice);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk_slice[..SPX_N]);

    if SPX_N >= 24 {
        blake512_mgf1_inner(&mut buf, &seed);
    } else {
        blake256_mgf1_inner(&mut buf, &seed);
    }

    digest.copy_from_slice(&buf[..SPX_FORS_MSG_BYTES]);
    let mut bufp = SPX_FORS_MSG_BYTES;

    let tree_val: u64 = if SPX_D == 1 {
        0
    } else {
        let v = bytes_to_ull(&buf[bufp..bufp + SPX_TREE_BYTES]);
        v & ((!0u64) >> (64 - SPX_TREE_BITS))
    };
    unsafe { *tree = tree_val; }
    bufp += SPX_TREE_BYTES;

    let leaf_idx_val =
        bytes_to_ull(&buf[bufp..bufp + SPX_LEAF_BYTES]) as u32 & ((!0u32) >> (32 - SPX_LEAF_BITS));
    unsafe { *leaf_idx = leaf_idx_val; }
}
