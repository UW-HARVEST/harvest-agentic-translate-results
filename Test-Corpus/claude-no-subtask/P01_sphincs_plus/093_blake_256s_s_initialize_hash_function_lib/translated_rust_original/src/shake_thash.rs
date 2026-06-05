// SHAKE thash implementation
#![cfg(feature = "shake")]
#![allow(dead_code)]

use crate::context::SpxCtx;
use crate::fips202::shake256;
use crate::params::*;

#[cfg(feature = "robust")]
pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let inblocks = inblocks as usize;
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];
    let mut bitmask = vec![0u8; inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);

    shake256(
        &mut bitmask,
        inblocks * SPX_N,
        &buf,
        SPX_N + SPX_ADDR_BYTES,
    );

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }

    let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    shake256(out, SPX_N, &buf, total);
}

#[cfg(feature = "simple")]
pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let inblocks = inblocks as usize;
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&input[..inblocks * SPX_N]);

    let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    shake256(out, SPX_N, &buf, total);
}
