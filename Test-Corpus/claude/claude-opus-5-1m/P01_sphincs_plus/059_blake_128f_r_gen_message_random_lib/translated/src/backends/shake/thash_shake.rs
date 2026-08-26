//! Translation of `lib/shake/src/thash_shake_robust.c` and
//! `lib/shake/src/thash_shake_simple.c`. Exactly one variant is compiled,
//! selected by the `simple` feature (otherwise `robust`, the default).

use super::fips202::shake256;
use crate::context::SpxCtx;
use crate::params::*;

/// Robust `thash`.
#[cfg(not(feature = "simple"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    in_: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let inblocks = inblocks as usize;
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];
    let mut bitmask = vec![0u8; inblocks * SPX_N];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);

    shake256(
        bitmask.as_mut_ptr(),
        inblocks * SPX_N,
        buf.as_ptr(),
        SPX_N + SPX_ADDR_BYTES,
    );

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = *in_.add(i) ^ bitmask[i];
    }

    shake256(out, SPX_N, buf.as_ptr(), SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N);
}

/// Simple `thash`.
#[cfg(feature = "simple")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    in_: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let inblocks = inblocks as usize;
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
    core::ptr::copy_nonoverlapping(in_, buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES), inblocks * SPX_N);

    shake256(out, SPX_N, buf.as_ptr(), SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N);
}
