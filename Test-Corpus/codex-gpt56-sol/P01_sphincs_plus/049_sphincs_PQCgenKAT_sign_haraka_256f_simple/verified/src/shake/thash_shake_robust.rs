//! Translation of `lib/shake/src/thash_shake_robust.c`.

use core::ptr::{addr_of, copy_nonoverlapping};

use crate::context::SpxCtx;
use crate::params::{SPX_ADDR_BYTES, SPX_N};
use crate::shake::fips202::shake256;

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
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks as usize * SPX_N];
    let mut bitmask = vec![0u8; inblocks as usize * SPX_N];
    let mut i: u32;

    let buf_ptr: *mut u8 = buf.as_mut_ptr();
    let bitmask_ptr: *mut u8 = bitmask.as_mut_ptr();

    copy_nonoverlapping(addr_of!((*ctx).pub_seed) as *const u8, buf_ptr, SPX_N);
    copy_nonoverlapping(addr as *const u8, buf_ptr.add(SPX_N), SPX_ADDR_BYTES);

    shake256(
        bitmask_ptr,
        inblocks as usize * SPX_N,
        buf_ptr as *const u8,
        SPX_N + SPX_ADDR_BYTES,
    );

    i = 0;
    while (i as usize) < inblocks as usize * SPX_N {
        *buf_ptr.add(SPX_N + SPX_ADDR_BYTES + i as usize) =
            *input.add(i as usize) ^ *bitmask_ptr.add(i as usize);
        i = i.wrapping_add(1);
    }

    shake256(
        out,
        SPX_N,
        buf_ptr as *const u8,
        SPX_N + SPX_ADDR_BYTES + inblocks as usize * SPX_N,
    );
}
