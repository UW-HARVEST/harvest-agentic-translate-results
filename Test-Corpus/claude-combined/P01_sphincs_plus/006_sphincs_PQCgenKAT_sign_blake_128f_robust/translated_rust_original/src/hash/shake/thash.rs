// Translation of c_src/lib/shake/src/thash_shake_{robust,simple}.c

use core::slice;

use crate::context::SpxCtx;
use crate::params::{SPX_ADDR_BYTES, SPX_N};

use super::fips202::shake256_inner;

#[cfg(feature = "robust")]
pub fn thash_robust(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32]) {
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];
    let mut bitmask = vec![0u8; inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);

    shake256_inner(&mut bitmask, &buf[..SPX_N + SPX_ADDR_BYTES]);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }
    shake256_inner(&mut out[..SPX_N], &buf);
}

#[cfg(feature = "simple")]
pub fn thash_simple(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32]) {
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N]
        .copy_from_slice(&input[..inblocks * SPX_N]);

    shake256_inner(&mut out[..SPX_N], &buf);
}
