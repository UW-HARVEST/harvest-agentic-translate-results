#![allow(non_snake_case)]

use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::sha2::*;

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
unsafe fn thash_512(out: *mut u8, in_: *const u8, inblocks: u32, ctx: *const SpxCtx, addr: *mut u32) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 72];
    let buflen = SPX_SHA256_ADDR_BYTES + inblocks as usize * SPX_N;
    let mut buf = vec![0u8; buflen];

    core::ptr::copy_nonoverlapping((*ctx).state_seeded_512.as_ptr(), sha2_state.as_mut_ptr(), 72);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), SPX_SHA256_ADDR_BYTES);
    core::ptr::copy_nonoverlapping(in_, buf.as_mut_ptr().add(SPX_SHA256_ADDR_BYTES), inblocks as usize * SPX_N);

    sha512_inc_finalize(outbuf.as_mut_ptr(), sha2_state.as_mut_ptr(), buf.as_ptr(), buflen);
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    in_: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        if inblocks > 1 {
            thash_512(out, in_, inblocks, ctx, addr);
            return;
        }
    }

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    let buflen = SPX_SHA256_ADDR_BYTES + inblocks as usize * SPX_N;
    let mut buf = vec![0u8; buflen];

    core::ptr::copy_nonoverlapping((*ctx).state_seeded.as_ptr(), sha2_state.as_mut_ptr(), 40);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), SPX_SHA256_ADDR_BYTES);
    core::ptr::copy_nonoverlapping(in_, buf.as_mut_ptr().add(SPX_SHA256_ADDR_BYTES), inblocks as usize * SPX_N);

    sha256_inc_finalize(outbuf.as_mut_ptr(), sha2_state.as_mut_ptr(), buf.as_ptr(), buflen);
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}
