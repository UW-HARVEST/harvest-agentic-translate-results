//! Translation of `lib/haraka/src/thash_haraka_robust.c`.

use core::ptr::{copy_nonoverlapping, write_bytes};

use crate::context::SpxCtx;
use crate::haraka::haraka::{SPX_haraka256, SPX_haraka512, SPX_haraka_S};
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
    let mut bitmask = vec![0u8; inblocks as usize * SPX_N];
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];
    let mut i: u32;

    let buf_ptr: *mut u8 = buf.as_mut_ptr();
    let bitmask_ptr: *mut u8 = bitmask.as_mut_ptr();
    let buf_tmp_ptr: *mut u8 = buf_tmp.as_mut_ptr();
    let outbuf_ptr: *mut u8 = outbuf.as_mut_ptr();

    if inblocks == 1 {
        /* F function */
        /* Since SPX_N may be smaller than 32, we need a temporary buffer. */
        write_bytes(buf_tmp_ptr, 0, 64);
        copy_nonoverlapping(addr as *const u8, buf_tmp_ptr, 32);

        SPX_haraka256(outbuf_ptr, buf_tmp_ptr, ctx);
        i = 0;
        while (i as usize) < inblocks as usize * SPX_N {
            *buf_tmp_ptr.add(SPX_ADDR_BYTES + i as usize) =
                *input.add(i as usize) ^ *outbuf_ptr.add(i as usize);
            i = i.wrapping_add(1);
        }
        SPX_haraka512(outbuf_ptr, buf_tmp_ptr, ctx);
        copy_nonoverlapping(outbuf_ptr as *const u8, out, SPX_N);
    } else {
        /* All other tweakable hashes*/
        copy_nonoverlapping(addr as *const u8, buf_ptr, 32);
        SPX_haraka_S(
            bitmask_ptr,
            (inblocks as usize * SPX_N) as u64,
            buf_ptr as *const u8,
            SPX_ADDR_BYTES as u64,
            ctx,
        );

        i = 0;
        while (i as usize) < inblocks as usize * SPX_N {
            *buf_ptr.add(SPX_ADDR_BYTES + i as usize) =
                *input.add(i as usize) ^ *bitmask_ptr.add(i as usize);
            i = i.wrapping_add(1);
        }

        SPX_haraka_S(
            out,
            SPX_N as u64,
            buf_ptr as *const u8,
            (SPX_ADDR_BYTES + inblocks as usize * SPX_N) as u64,
            ctx,
        );
    }
}
