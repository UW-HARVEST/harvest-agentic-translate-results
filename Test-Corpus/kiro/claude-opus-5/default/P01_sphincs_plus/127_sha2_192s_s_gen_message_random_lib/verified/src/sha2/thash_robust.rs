//! Translation of `lib/sha2/src/thash_sha2_robust.c`.

use crate::address::addr_bytes;
use crate::context::SpxCtx;
use crate::params::*;
use crate::vla::Vla;
use crate::sha2::SPX_SHA512;
use crate::sha2::sha2::*;

/// Takes an array of `inblocks` concatenated arrays of `SPX_N` bytes.
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    if SPX_SHA512 && inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    let n = inblocks * SPX_N;
    let mut mask_vla = Vla::<{ SPX_MAX_INBLOCKS * SPX_N }>::new(n);
    let bitmask = mask_vla.as_mut_slice();
    let mut vla = Vla::<{ SPX_N + SPX_SHA256_OUTPUT_BYTES + SPX_MAX_INBLOCKS * SPX_N }>::new(
        SPX_N + SPX_SHA256_ADDR_BYTES + n,
    );
    let buf = vla.as_mut_slice();

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES]
        .copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    let seed_in: [u8; SPX_N + SPX_SHA256_ADDR_BYTES] =
        buf[..SPX_N + SPX_SHA256_ADDR_BYTES].try_into().unwrap();
    mgf1_256(&mut bitmask[..n], n, &seed_in, SPX_N + SPX_SHA256_ADDR_BYTES);

    /* Retrieve precomputed state containing pub_seed */
    sha2_state.copy_from_slice(&ctx.backend.state_seeded);

    for i in 0..n {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    let total = SPX_SHA256_ADDR_BYTES + n;
    let body: &[u8] = &buf[SPX_N..SPX_N + total];
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, body, total);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 72];
    let n = inblocks * SPX_N;
    let mut mask_vla = Vla::<{ SPX_MAX_INBLOCKS * SPX_N }>::new(n);
    let bitmask = mask_vla.as_mut_slice();
    let mut vla = Vla::<{ SPX_N + SPX_SHA256_ADDR_BYTES + SPX_MAX_INBLOCKS * SPX_N }>::new(
        SPX_N + SPX_SHA256_ADDR_BYTES + n,
    );
    let buf = vla.as_mut_slice();

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES]
        .copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    let seed_in: [u8; SPX_N + SPX_SHA256_ADDR_BYTES] =
        buf[..SPX_N + SPX_SHA256_ADDR_BYTES].try_into().unwrap();
    mgf1_512(&mut bitmask[..n], n, &seed_in, SPX_N + SPX_SHA256_ADDR_BYTES);

    /* Retrieve precomputed state containing pub_seed */
    let src = &ctx.backend.state_seeded_512;
    sha2_state[..src.len()].copy_from_slice(src);

    for i in 0..n {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    let total = SPX_SHA256_ADDR_BYTES + n;
    let body: &[u8] = &buf[SPX_N..SPX_N + total];
    sha512_inc_finalize(&mut outbuf, &mut sha2_state, body, total);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
