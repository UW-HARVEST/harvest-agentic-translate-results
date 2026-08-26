//! Translation of `lib/blake/src/thash_blake_robust.c` and
//! `lib/blake/src/thash_blake_simple.c`. Exactly one variant is compiled,
//! selected by the `simple` feature (otherwise `robust`, the default).
//!
//! The `#if SPX_BLAKE512` dispatch to `thash_512` becomes a check against the
//! runtime-constant `crate::params::SPX_BLAKE512`.

use super::blake256::{blake256, SPX_BLAKE256_OUTPUT_BYTES};
use super::blake512::{blake512, SPX_BLAKE512_OUTPUT_BYTES};
use crate::context::SpxCtx;
use crate::params::*;

// The bitmask (mgf1) is only used by the robust variant.
#[cfg(not(feature = "simple"))]
use super::blake256::SPX_blake256_mgf1;
#[cfg(not(feature = "simple"))]
use super::blake512::SPX_blake512_mgf1;
#[cfg(not(feature = "simple"))]
use core::ffi::c_ulong;

/// Robust `thash`: takes an array of inblocks concatenated arrays of SPX_N
/// bytes.
#[cfg(not(feature = "simple"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    in_: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    if SPX_BLAKE512 && inblocks > 1 {
        thash_512(out, in_, inblocks, ctx, addr);
        return;
    }
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);

    SPX_blake256_mgf1(
        bitmask.as_mut_ptr(),
        (inblocks * SPX_N) as c_ulong,
        buf.as_ptr(),
        (SPX_N + SPX_ADDR_BYTES) as c_ulong,
    );

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = *in_.add(i) ^ bitmask[i];
    }

    blake256(
        outbuf.as_mut_ptr(),
        buf.as_ptr().add(SPX_N),
        (SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
    );
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

#[cfg(not(feature = "simple"))]
unsafe fn thash_512(
    out: *mut u8,
    in_: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);

    SPX_blake512_mgf1(
        bitmask.as_mut_ptr(),
        (inblocks * SPX_N) as c_ulong,
        buf.as_ptr(),
        (SPX_N + SPX_ADDR_BYTES) as c_ulong,
    );

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = *in_.add(i) ^ bitmask[i];
    }

    blake512(
        outbuf.as_mut_ptr(),
        buf.as_ptr().add(SPX_N),
        (SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
    );
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

/// Simple `thash`: takes an array of inblocks concatenated arrays of SPX_N
/// bytes.
#[cfg(feature = "simple")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    in_: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    if SPX_BLAKE512 && inblocks > 1 {
        thash_512(out, in_, inblocks, ctx, addr);
        return;
    }
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
    core::ptr::copy_nonoverlapping(
        in_,
        buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES),
        inblocks * SPX_N,
    );

    blake256(
        outbuf.as_mut_ptr(),
        buf.as_ptr(),
        (SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
    );
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

#[cfg(feature = "simple")]
unsafe fn thash_512(
    out: *mut u8,
    in_: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
    core::ptr::copy_nonoverlapping(
        in_,
        buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES),
        inblocks * SPX_N,
    );

    blake512(
        outbuf.as_mut_ptr(),
        buf.as_ptr(),
        (SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
    );
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}
