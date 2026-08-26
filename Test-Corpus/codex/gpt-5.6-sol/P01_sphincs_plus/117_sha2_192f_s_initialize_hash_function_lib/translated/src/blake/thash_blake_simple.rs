//! Translation of `lib/blake/src/thash_blake_simple.c`.

use core::ptr::{addr_of, copy_nonoverlapping};

use crate::blake::blake256::*;
use crate::blake::blake512::*;
use crate::context::SpxCtx;
use crate::params::{SPX_ADDR_BYTES, SPX_BLAKE512, SPX_N};

/* From `lib/blake/include/blake.h`. */
const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

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
    /* C:
     *   #if SPX_BLAKE512
     *       if (inblocks > 1) { thash_512(out, in, inblocks, ctx, addr); return; }
     *   #endif
     */
    if SPX_BLAKE512 {
        if inblocks > 1 {
            thash_512(out, input, inblocks, ctx, addr);
            return;
        }
    }

    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks as usize * SPX_N];
    let buf_ptr: *mut u8 = buf.as_mut_ptr();

    copy_nonoverlapping(addr_of!((*ctx).pub_seed) as *const u8, buf_ptr, SPX_N);
    copy_nonoverlapping(addr as *const u8, buf_ptr.add(SPX_N), SPX_ADDR_BYTES);
    copy_nonoverlapping(
        input,
        buf_ptr.add(SPX_N + SPX_ADDR_BYTES),
        inblocks as usize * SPX_N,
    );

    blake256(
        outbuf.as_mut_ptr(),
        buf_ptr as *const u8,
        (SPX_N + SPX_ADDR_BYTES + inblocks as usize * SPX_N) as u64,
    );
    copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

/* C: static void thash_512(...), compiled only `#if SPX_BLAKE512`; here it is
 * always present but only reachable when `SPX_BLAKE512` is true. */
unsafe fn thash_512(
    out: *mut u8,
    input: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks as usize * SPX_N];
    let buf_ptr: *mut u8 = buf.as_mut_ptr();

    copy_nonoverlapping(addr_of!((*ctx).pub_seed) as *const u8, buf_ptr, SPX_N);
    copy_nonoverlapping(addr as *const u8, buf_ptr.add(SPX_N), SPX_ADDR_BYTES);
    copy_nonoverlapping(
        input,
        buf_ptr.add(SPX_N + SPX_ADDR_BYTES),
        inblocks as usize * SPX_N,
    );

    blake512(
        outbuf.as_mut_ptr(),
        buf_ptr as *const u8,
        (SPX_N + SPX_ADDR_BYTES + inblocks as usize * SPX_N) as u64,
    );
    copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}
