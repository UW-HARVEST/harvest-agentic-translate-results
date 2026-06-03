// Translation of c_src/lib/blake/src/hash_blake.c

use crate::blake::{
    blake256, blake256_final, blake256_init, blake256_update, BlakeState256,
    SPX_BLAKE256_OUTPUT_BYTES,
};
use crate::context::SpxCtx;
use crate::params::{
    SPX_ADDR_BYTES, SPX_D, SPX_FORS_MSG_BYTES, SPX_N, SPX_PK_BYTES, SPX_TREE_HEIGHT,
};
use crate::utils::bytes_to_ull;

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
use crate::blake::{
    blake512_final, blake512_init, blake512_mgf1, blake512_update, BlakeState512,
    SPX_BLAKE512_OUTPUT_BYTES,
};
#[cfg(any(feature = "128s", feature = "128f"))]
use crate::blake::blake256_mgf1;

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE512_OUTPUT_BYTES;
#[cfg(any(feature = "128s", feature = "128f"))]
const SPX_BLAKEX_OUTPUT_BYTES: usize = SPX_BLAKE256_OUTPUT_BYTES;

pub fn initialize_hash_function(_ctx: &mut SpxCtx) {}

pub fn prf_addr(out: &mut [u8], ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = vec![0u8; 2 * SPX_N + SPX_ADDR_BYTES];
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
    buf[SPX_N + SPX_ADDR_BYTES..2 * SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ctx.sk_seed);

    blake256(&mut outbuf, &buf, (SPX_N + SPX_ADDR_BYTES) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

// NOTE: The original C source for hash_blake.c calls blake{256,512}_update
// with byte-counts where the function expects bit-counts (i.e. it passes
// SPX_N rather than SPX_N * 8). We deliberately reproduce that behavior so
// our output matches the C reference byte-for-byte. We also size the output
// buffer to BLAKE_X_OUTPUT_BYTES (the size the function actually writes) and
// then copy the result back into the caller-supplied buffer of length SPX_N.

#[cfg(any(feature = "128s", feature = "128f"))]
pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    mlen: u64,
    _ctx: &SpxCtx,
) {
    let mut s = BlakeState256::zero();
    blake256_init(&mut s);
    blake256_update(&mut s, sk_prf, SPX_N as u64);
    blake256_update(&mut s, optrand, SPX_N as u64);
    blake256_update(&mut s, m, mlen);
    let mut tmp = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    blake256_final(&mut s, &mut tmp);
    r[..SPX_N].copy_from_slice(&tmp[..SPX_N]);
}

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
pub fn gen_message_random(
    r: &mut [u8],
    sk_prf: &[u8],
    optrand: &[u8],
    m: &[u8],
    mlen: u64,
    _ctx: &SpxCtx,
) {
    let mut s = BlakeState512::zero();
    blake512_init(&mut s);
    blake512_update(&mut s, sk_prf, SPX_N as u64);
    blake512_update(&mut s, optrand, SPX_N as u64);
    blake512_update(&mut s, m, mlen);
    let mut tmp = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    blake512_final(&mut s, &mut tmp);
    r[..SPX_N].copy_from_slice(&tmp[..SPX_N]);
}

#[cfg(any(feature = "128s", feature = "128f"))]
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
    const SPX_TREE_BITS_VAL: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS_VAL + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = vec![0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    let mut s = BlakeState256::zero();
    blake256_init(&mut s);
    // Match the C reference: it passes byte-counts where the function expects
    // bit-counts (a known quirk of the original blake hash_message in the
    // SPHINCS+ BLAKE backend).
    blake256_update(&mut s, r, SPX_N as u64);
    blake256_update(&mut s, pk, SPX_PK_BYTES as u64);
    blake256_update(&mut s, m, mlen);

    blake256_final(&mut s, &mut seed[2 * SPX_N..]);

    seed[..SPX_N].copy_from_slice(r);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    blake256_mgf1(
        &mut buf,
        SPX_DGST_BYTES,
        &seed,
        2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES,
    );

    let mut bufp = 0usize;
    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[bufp..bufp + SPX_FORS_MSG_BYTES]);
    bufp += SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= !0u64 >> (64 - SPX_TREE_BITS_VAL);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= !0u32 >> (32 - SPX_LEAF_BITS);
}

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
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
    const SPX_TREE_BITS_VAL: usize = SPX_TREE_HEIGHT * (SPX_D - 1);
    const SPX_TREE_BYTES: usize = (SPX_TREE_BITS_VAL + 7) / 8;
    const SPX_LEAF_BITS: usize = SPX_TREE_HEIGHT;
    const SPX_LEAF_BYTES: usize = (SPX_LEAF_BITS + 7) / 8;
    const SPX_DGST_BYTES: usize = SPX_FORS_MSG_BYTES + SPX_TREE_BYTES + SPX_LEAF_BYTES;

    let mut buf = [0u8; SPX_DGST_BYTES];
    let mut seed = vec![0u8; 2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES];

    let mut s = BlakeState512::zero();
    blake512_init(&mut s);
    // Match the C reference: byte-counts passed where bit-counts expected.
    blake512_update(&mut s, r, SPX_N as u64);
    blake512_update(&mut s, pk, SPX_PK_BYTES as u64);
    blake512_update(&mut s, m, mlen);
    blake512_final(&mut s, &mut seed[2 * SPX_N..]);

    seed[..SPX_N].copy_from_slice(r);
    seed[SPX_N..2 * SPX_N].copy_from_slice(&pk[..SPX_N]);

    blake512_mgf1(
        &mut buf,
        SPX_DGST_BYTES,
        &seed,
        2 * SPX_N + SPX_BLAKEX_OUTPUT_BYTES,
    );

    let mut bufp = 0usize;
    digest[..SPX_FORS_MSG_BYTES].copy_from_slice(&buf[bufp..bufp + SPX_FORS_MSG_BYTES]);
    bufp += SPX_FORS_MSG_BYTES;

    if SPX_D == 1 {
        *tree = 0;
    } else {
        *tree = bytes_to_ull(&buf[bufp..], SPX_TREE_BYTES);
        *tree &= !0u64 >> (64 - SPX_TREE_BITS_VAL);
    }
    bufp += SPX_TREE_BYTES;

    *leaf_idx = bytes_to_ull(&buf[bufp..], SPX_LEAF_BYTES) as u32;
    *leaf_idx &= !0u32 >> (32 - SPX_LEAF_BITS);
}
