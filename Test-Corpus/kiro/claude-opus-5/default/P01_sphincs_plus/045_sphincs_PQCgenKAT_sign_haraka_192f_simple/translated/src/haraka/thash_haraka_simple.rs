//! Translation of `lib/haraka/src/thash_haraka_simple.c`.

use core::ffi::c_uint;

use crate::context::SpxCtx;
use crate::haraka::haraka::{SPX_haraka512, SPX_haraka_S};
use crate::params::*;

/// Takes an array of inblocks concatenated arrays of SPX_N bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    in_: *const u8,
    inblocks: c_uint,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks as usize * SPX_N];
    let mut outbuf: [u8; 32] = [0u8; 32];
    let mut buf_tmp: [u8; 64] = [0u8; 64];

    if inblocks == 1 {
        /* F function */
        /* Since SPX_N may be smaller than 32, we need a temporary buffer. */
        core::ptr::write_bytes(buf_tmp.as_mut_ptr(), 0, 64);
        core::ptr::copy_nonoverlapping(addr as *const u8, buf_tmp.as_mut_ptr(), 32);
        core::ptr::copy_nonoverlapping(in_, buf_tmp.as_mut_ptr().add(SPX_ADDR_BYTES), SPX_N);

        SPX_haraka512(outbuf.as_mut_ptr(), buf_tmp.as_ptr(), ctx);
        core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
    } else {
        /* All other tweakable hashes*/
        core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), 32);
        core::ptr::copy_nonoverlapping(
            in_,
            buf.as_mut_ptr().add(SPX_ADDR_BYTES),
            inblocks as usize * SPX_N,
        );

        SPX_haraka_S(
            out,
            SPX_N as core::ffi::c_ulonglong,
            buf.as_ptr(),
            (SPX_ADDR_BYTES + inblocks as usize * SPX_N) as core::ffi::c_ulonglong,
            ctx,
        );
    }
}
