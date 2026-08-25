//! Translation of `lib/haraka/src/thash_haraka_simple.c`.

use core::ptr::{copy_nonoverlapping, write_bytes};

use crate::context::SpxCtx;
use crate::haraka::haraka::{SPX_haraka512, SPX_haraka_S};
use crate::params::{SPX_ADDR_BYTES, SPX_N};

/**
 * Takes an array of inblocks concatenated arrays of SPX_N bytes.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    input: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks as usize * SPX_N];
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];

    let buf_ptr: *mut u8 = buf.as_mut_ptr();
    let buf_tmp_ptr: *mut u8 = buf_tmp.as_mut_ptr();
    let outbuf_ptr: *mut u8 = outbuf.as_mut_ptr();

    if inblocks == 1 {
        /* F function */
        /* Since SPX_N may be smaller than 32, we need a temporary buffer. */
        write_bytes(buf_tmp_ptr, 0, 64);
        copy_nonoverlapping(addr as *const u8, buf_tmp_ptr, 32);
        copy_nonoverlapping(input, buf_tmp_ptr.add(SPX_ADDR_BYTES), SPX_N);

        SPX_haraka512(outbuf_ptr, buf_tmp_ptr, ctx);
        copy_nonoverlapping(outbuf_ptr as *const u8, out, SPX_N);
    } else {
        /* All other tweakable hashes*/
        copy_nonoverlapping(addr as *const u8, buf_ptr, 32);
        copy_nonoverlapping(
            input,
            buf_ptr.add(SPX_ADDR_BYTES),
            inblocks as usize * SPX_N,
        );

        SPX_haraka_S(
            out,
            SPX_N as u64,
            buf_ptr as *const u8,
            (SPX_ADDR_BYTES + inblocks as usize * SPX_N) as u64,
            ctx,
        );
    }
}
