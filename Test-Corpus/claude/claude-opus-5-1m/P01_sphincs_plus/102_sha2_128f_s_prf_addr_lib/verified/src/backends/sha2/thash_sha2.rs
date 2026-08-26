//! Translation of `lib/sha2/src/thash_sha2_robust.c` and
//! `lib/sha2/src/thash_sha2_simple.c`. Exactly one variant is compiled,
//! selected by the `simple` feature (otherwise `robust`, the default).
//!
//! The `#if SPX_SHA512` guard around the `inblocks > 1` dispatch to the
//! SHA-512 variant is reproduced with a runtime `if crate::params::SPX_SHA512`.

use super::sha2::{
    sha256_inc_finalize, sha512_inc_finalize, SPX_SHA256_OUTPUT_BYTES, SPX_SHA512_OUTPUT_BYTES,
};
// The bit-mask generation is only used by the robust variant.
#[cfg(not(feature = "simple"))]
use super::sha2::{SPX_mgf1_256, SPX_mgf1_512};
use crate::context::SpxCtx;
use crate::params::*;
#[cfg(not(feature = "simple"))]
use core::ffi::c_ulong;

// ===========================================================================
// Robust
// ===========================================================================

/// Takes an array of `inblocks` concatenated arrays of `SPX_N` bytes.
#[cfg(not(feature = "simple"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    in_: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    if SPX_SHA512 && inblocks > 1 {
        thash_512(out, in_, inblocks, ctx, addr);
        return;
    }

    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_OUTPUT_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 40];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(
        addr as *const u8,
        buf.as_mut_ptr().add(SPX_N),
        SPX_SHA256_ADDR_BYTES,
    );
    SPX_mgf1_256(
        bitmask.as_mut_ptr(),
        (inblocks * SPX_N) as c_ulong,
        buf.as_ptr(),
        (SPX_N + SPX_SHA256_ADDR_BYTES) as c_ulong,
    );

    /* Retrieve precomputed state containing pub_seed */
    core::ptr::copy_nonoverlapping((*ctx).state_seeded.as_ptr(), sha2_state.as_mut_ptr(), 40);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = *in_.add(i) ^ bitmask[i];
    }

    sha256_inc_finalize(
        outbuf.as_mut_ptr(),
        sha2_state.as_mut_ptr(),
        buf.as_ptr().add(SPX_N),
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
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
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 72];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(
        addr as *const u8,
        buf.as_mut_ptr().add(SPX_N),
        SPX_SHA256_ADDR_BYTES,
    );
    SPX_mgf1_512(
        bitmask.as_mut_ptr(),
        (inblocks * SPX_N) as c_ulong,
        buf.as_ptr(),
        (SPX_N + SPX_SHA256_ADDR_BYTES) as c_ulong,
    );

    /* Retrieve precomputed state containing pub_seed */
    core::ptr::copy_nonoverlapping(
        (*ctx).state_seeded_512.as_ptr(),
        sha2_state.as_mut_ptr(),
        72,
    );

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = *in_.add(i) ^ bitmask[i];
    }

    sha512_inc_finalize(
        outbuf.as_mut_ptr(),
        sha2_state.as_mut_ptr(),
        buf.as_ptr().add(SPX_N),
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

// ===========================================================================
// Simple
// ===========================================================================

/// Takes an array of `inblocks` concatenated arrays of `SPX_N` bytes.
#[cfg(feature = "simple")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    in_: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    if SPX_SHA512 && inblocks > 1 {
        thash_512(out, in_, inblocks, ctx, addr);
        return;
    }

    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];

    /* Retrieve precomputed state containing pub_seed */
    core::ptr::copy_nonoverlapping((*ctx).state_seeded.as_ptr(), sha2_state.as_mut_ptr(), 40);

    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), SPX_SHA256_ADDR_BYTES);
    core::ptr::copy_nonoverlapping(
        in_,
        buf.as_mut_ptr().add(SPX_SHA256_ADDR_BYTES),
        inblocks * SPX_N,
    );

    sha256_inc_finalize(
        outbuf.as_mut_ptr(),
        sha2_state.as_mut_ptr(),
        buf.as_ptr(),
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
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
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 72];
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];

    /* Retrieve precomputed state containing pub_seed */
    core::ptr::copy_nonoverlapping(
        (*ctx).state_seeded_512.as_ptr(),
        sha2_state.as_mut_ptr(),
        72,
    );

    core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), SPX_SHA256_ADDR_BYTES);
    core::ptr::copy_nonoverlapping(
        in_,
        buf.as_mut_ptr().add(SPX_SHA256_ADDR_BYTES),
        inblocks * SPX_N,
    );

    sha512_inc_finalize(
        outbuf.as_mut_ptr(),
        sha2_state.as_mut_ptr(),
        buf.as_ptr(),
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}
