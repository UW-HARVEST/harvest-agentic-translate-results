// Translation of c_src/lib/shake/src/thash_shake_{robust,simple}.c

use crate::context::SpxCtx;
use crate::fips202::shake256;
use crate::params::{SPX_ADDR_BYTES, SPX_N};

#[cfg(feature = "robust")]
pub fn thash(out: &mut [u8], in_buf: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; total];
    let mut bitmask = vec![0u8; inblocks * SPX_N];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);

    shake256(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..(inblocks * SPX_N) {
        buf[SPX_N + SPX_ADDR_BYTES + i] = in_buf[i] ^ bitmask[i];
    }

    shake256(out, SPX_N, &buf, total);
}

#[cfg(feature = "simple")]
pub fn thash(out: &mut [u8], in_buf: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; total];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_ADDR_BYTES]);
    buf[SPX_N + SPX_ADDR_BYTES..total].copy_from_slice(&in_buf[..inblocks * SPX_N]);

    shake256(out, SPX_N, &buf, total);
}
