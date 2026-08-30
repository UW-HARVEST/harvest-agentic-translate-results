//! Translation of `lib/sha2/src/thash_sha2_simple.c`.

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
    let mut vla = Vla::<{ SPX_SHA256_ADDR_BYTES + SPX_MAX_INBLOCKS * SPX_N }>::new(
        SPX_SHA256_ADDR_BYTES + n,
    );
    let buf = vla.as_mut_slice();

    /* Retrieve precomputed state containing pub_seed */
    sha2_state.copy_from_slice(&ctx.backend.state_seeded);

    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + n].copy_from_slice(&inp[..n]);

    let total = SPX_SHA256_ADDR_BYTES + n;
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf[..total], total);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 72];
    let n = inblocks * SPX_N;
    let mut vla = Vla::<{ SPX_SHA256_ADDR_BYTES + SPX_MAX_INBLOCKS * SPX_N }>::new(
        SPX_SHA256_ADDR_BYTES + n,
    );
    let buf = vla.as_mut_slice();

    /* Retrieve precomputed state containing pub_seed */
    let src = &ctx.backend.state_seeded_512;
    sha2_state[..src.len()].copy_from_slice(src);

    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + n].copy_from_slice(&inp[..n]);

    let total = SPX_SHA256_ADDR_BYTES + n;
    sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf[..total], total);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
