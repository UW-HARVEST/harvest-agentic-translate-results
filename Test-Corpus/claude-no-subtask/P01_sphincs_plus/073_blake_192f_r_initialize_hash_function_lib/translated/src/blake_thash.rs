// BLAKE thash implementation
#![cfg(feature = "blake")]
#![allow(dead_code)]

use crate::blake::*;
use crate::context::SpxCtx;
use crate::params::*;

#[cfg(feature = "robust")]
pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
    {
        if inblocks > 1 {
            thash_512(out, input, inblocks, ctx, addr);
            return;
        }
    }
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);

    blake256_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }

    let total = SPX_ADDR_BYTES + inblocks * SPX_N;
    blake256(
        &mut outbuf,
        &buf[SPX_N..SPX_N + total],
        total as u64,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(feature = "robust", any(feature = "192f", feature = "192s", feature = "256f", feature = "256s")))]
fn thash_512(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);

    blake512_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }

    let total = SPX_ADDR_BYTES + inblocks * SPX_N;
    blake512(
        &mut outbuf,
        &buf[SPX_N..SPX_N + total],
        total as u64,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(feature = "simple")]
pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
    {
        if inblocks > 1 {
            thash_512(out, input, inblocks, ctx, addr);
            return;
        }
    }
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&input[..inblocks * SPX_N]);

    let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    blake256(&mut outbuf, &buf, total as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(feature = "simple", any(feature = "192f", feature = "192s", feature = "256f", feature = "256s")))]
fn thash_512(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..].copy_from_slice(&input[..inblocks * SPX_N]);

    let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    blake512(&mut outbuf, &buf, total as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
