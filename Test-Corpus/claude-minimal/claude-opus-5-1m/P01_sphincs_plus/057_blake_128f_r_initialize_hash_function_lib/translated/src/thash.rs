use crate::context::SpxCtx;
use crate::fips202::shake256;
use crate::params::{SPX_ADDR_BYTES, SPX_N};

/// Simple thash for SHAKE: shake256(pub_seed || addr || in).
pub fn thash(out: &mut [u8], in_buf: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; total];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { std::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..total].copy_from_slice(&in_buf[..inblocks * SPX_N]);

    shake256(out, SPX_N, &buf, total);
}
