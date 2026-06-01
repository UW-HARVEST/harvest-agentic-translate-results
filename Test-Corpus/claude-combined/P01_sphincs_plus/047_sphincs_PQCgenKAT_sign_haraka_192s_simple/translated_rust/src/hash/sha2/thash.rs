// Translation of c_src/lib/sha2/src/thash_sha2_{robust,simple}.c

use crate::context::SpxCtx;
use crate::params::{
    SPX_N, SPX_SHA256_ADDR_BYTES, SPX_SHA256_OUTPUT_BYTES, SPX_SHA512, SPX_SHA512_OUTPUT_BYTES,
};
use core::slice;

use super::sha2::{
    mgf1_256_inner, mgf1_512_inner, sha256_inc_finalize_inner, sha512_inc_finalize_inner,
};

#[cfg(feature = "robust")]
pub fn thash_robust(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32]) {
    if SPX_SHA512 && inblocks > 1 {
        thash_robust_512(out, input, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_OUTPUT_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 40];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    mgf1_256_inner(&mut bitmask, &buf[..SPX_N + SPX_SHA256_ADDR_BYTES]);

    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }
    sha256_inc_finalize_inner(
        &mut outbuf,
        &mut sha2_state,
        &buf[SPX_N..],
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(feature = "robust")]
fn thash_robust_512(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32]) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 72];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    mgf1_512_inner(&mut bitmask, &buf[..SPX_N + SPX_SHA256_ADDR_BYTES]);

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }
    sha512_inc_finalize_inner(
        &mut outbuf,
        &mut sha2_state,
        &buf[SPX_N..],
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(feature = "simple")]
pub fn thash_simple(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32]) {
    if SPX_SHA512 && inblocks > 1 {
        thash_simple_512(out, input, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];

    sha2_state.copy_from_slice(&ctx.state_seeded);
    let addr_bytes = unsafe { slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + inblocks * SPX_N]
        .copy_from_slice(&input[..inblocks * SPX_N]);

    sha256_inc_finalize_inner(
        &mut outbuf,
        &mut sha2_state,
        &buf,
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(feature = "simple")]
fn thash_simple_512(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32]) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 72];
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];

    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    sha2_state.copy_from_slice(&ctx.state_seeded_512);
    let addr_bytes = unsafe { slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + inblocks * SPX_N]
        .copy_from_slice(&input[..inblocks * SPX_N]);

    sha512_inc_finalize_inner(
        &mut outbuf,
        &mut sha2_state,
        &buf,
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
