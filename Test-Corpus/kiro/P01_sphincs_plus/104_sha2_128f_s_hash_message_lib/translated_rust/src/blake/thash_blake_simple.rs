use crate::context::SpxCtx;
use crate::params::*;

use super::blake256::{blake256, SPX_BLAKE256_OUTPUT_BYTES};
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
use super::blake512::{blake512, SPX_BLAKE512_OUTPUT_BYTES};

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
unsafe fn thash_512(
    out: *mut u8, in_: *const u8, inblocks: u32,
    ctx: *const SpxCtx, addr: *mut u32,
) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let buflen = SPX_N + SPX_ADDR_BYTES + (inblocks as usize) * SPX_N;
    let mut buf = vec![0u8; buflen];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
    core::ptr::copy_nonoverlapping(in_, buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES), (inblocks as usize) * SPX_N);

    blake512(outbuf.as_mut_ptr(), buf.as_ptr(), buflen as u64);
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8, in_: *const u8, inblocks: u32,
    ctx: *const SpxCtx, addr: *mut u32,
) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        if inblocks > 1 {
            thash_512(out, in_, inblocks, ctx, addr);
            return;
        }
    }

    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let buflen = SPX_N + SPX_ADDR_BYTES + (inblocks as usize) * SPX_N;
    let mut buf = vec![0u8; buflen];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
    core::ptr::copy_nonoverlapping(in_, buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES), (inblocks as usize) * SPX_N);

    blake256(outbuf.as_mut_ptr(), buf.as_ptr(), buflen as u64);
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}
