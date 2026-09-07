use core::ffi::c_uint;

use crate::context::SpxCtx;
use crate::params::*;
use crate::shake::fips202::shake256;

/**
 * Takes an array of inblocks concatenated arrays of SPX_N bytes.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    in_: *const u8,
    inblocks: c_uint,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks as usize * SPX_N];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(
        addr as *const u8,
        buf.as_mut_ptr().add(SPX_N),
        SPX_ADDR_BYTES,
    );
    core::ptr::copy_nonoverlapping(
        in_,
        buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES),
        inblocks as usize * SPX_N,
    );

    shake256(
        out,
        SPX_N,
        buf.as_ptr(),
        SPX_N + SPX_ADDR_BYTES + inblocks as usize * SPX_N,
    );
}
