//! Translation of `lib/blake/src/thash_blake_robust.c`.

use crate::address::addr_bytes;
use crate::blake::SPX_BLAKE512;
use crate::blake::blake256::{SPX_BLAKE256_OUTPUT_BYTES, blake256, blake256_mgf1};
use crate::blake::blake512::{SPX_BLAKE512_OUTPUT_BYTES, blake512, blake512_mgf1};
use crate::context::SpxCtx;
use crate::params::*;
use crate::vla::Vla;

/// Takes an array of `inblocks` concatenated arrays of `SPX_N` bytes.
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    if SPX_BLAKE512 && inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }

    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let n = inblocks * SPX_N;
    let mut mask_vla = Vla::<{ SPX_MAX_INBLOCKS * SPX_N }>::new(n);
    let bitmask = mask_vla.as_mut_slice();
    let mut vla =
        Vla::<{ SPX_N + SPX_ADDR_BYTES + SPX_MAX_INBLOCKS * SPX_N }>::new(SPX_N + SPX_ADDR_BYTES + n);
    let buf = vla.as_mut_slice();

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));

    let seed_in: [u8; SPX_N + SPX_ADDR_BYTES] = buf[..SPX_N + SPX_ADDR_BYTES].try_into().unwrap();
    blake256_mgf1(
        &mut bitmask[..n],
        n as core::ffi::c_ulong,
        &seed_in,
        (SPX_N + SPX_ADDR_BYTES) as core::ffi::c_ulong,
    );

    for i in 0..n {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    let total = SPX_ADDR_BYTES + n;
    blake256(&mut outbuf, &buf[SPX_N..SPX_N + total], total as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let n = inblocks * SPX_N;
    let mut mask_vla = Vla::<{ SPX_MAX_INBLOCKS * SPX_N }>::new(n);
    let bitmask = mask_vla.as_mut_slice();
    let mut vla =
        Vla::<{ SPX_N + SPX_ADDR_BYTES + SPX_MAX_INBLOCKS * SPX_N }>::new(SPX_N + SPX_ADDR_BYTES + n);
    let buf = vla.as_mut_slice();

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));

    let seed_in: [u8; SPX_N + SPX_ADDR_BYTES] = buf[..SPX_N + SPX_ADDR_BYTES].try_into().unwrap();
    blake512_mgf1(
        &mut bitmask[..n],
        n as core::ffi::c_ulong,
        &seed_in,
        (SPX_N + SPX_ADDR_BYTES) as core::ffi::c_ulong,
    );

    for i in 0..n {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    let total = SPX_ADDR_BYTES + n;
    blake512(&mut outbuf, &buf[SPX_N..SPX_N + total], total as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
