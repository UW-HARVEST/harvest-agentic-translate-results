//! `lib/sha2/src/thash_sha2_robust.c` -- robust SHA-2 tweakable hash.

use core::ffi::c_uint;
use core::ptr::copy_nonoverlapping;

use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::sha2::{
    sha256_inc_finalize, SPX_mgf1_256, SPX_SHA256_ADDR_BYTES,
    SPX_SHA256_OUTPUT_BYTES,
};

// `#if SPX_SHA512` -> feature gate equivalent to `SPX_N >= 24`.
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
use crate::sha2::sha2::{
    sha512_inc_finalize, SPX_mgf1_512, SPX_SHA512_OUTPUT_BYTES,
};

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
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        if inblocks > 1 {
            thash_512(out, in_, inblocks, ctx, addr);
            return;
        }
    }

    let mut outbuf: [u8; SPX_SHA256_OUTPUT_BYTES] = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks as usize * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_OUTPUT_BYTES + inblocks as usize * SPX_N];
    let mut sha2_state: [u8; 40] = [0u8; 40];
    let mut i: c_uint;

    copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    copy_nonoverlapping(
        addr as *const u8,
        buf.as_mut_ptr().add(SPX_N),
        SPX_SHA256_ADDR_BYTES,
    );
    SPX_mgf1_256(
        bitmask.as_mut_ptr(),
        (inblocks as usize * SPX_N) as core::ffi::c_ulong,
        buf.as_ptr(),
        (SPX_N + SPX_SHA256_ADDR_BYTES) as core::ffi::c_ulong,
    );

    /* Retrieve precomputed state containing pub_seed */
    copy_nonoverlapping((*ctx).state_seeded.as_ptr(), sha2_state.as_mut_ptr(), 40);

    i = 0;
    while i < inblocks * SPX_N as c_uint {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i as usize] =
            *in_.add(i as usize) ^ bitmask[i as usize];
        i += 1;
    }

    sha256_inc_finalize(
        outbuf.as_mut_ptr(),
        sha2_state.as_mut_ptr(),
        buf.as_ptr().add(SPX_N),
        SPX_SHA256_ADDR_BYTES + inblocks as usize * SPX_N,
    );
    copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
unsafe fn thash_512(
    out: *mut u8,
    in_: *const u8,
    inblocks: c_uint,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let mut outbuf: [u8; SPX_SHA512_OUTPUT_BYTES] = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks as usize * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks as usize * SPX_N];
    let mut sha2_state: [u8; 72] = [0u8; 72];
    let mut i: c_uint;

    copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    copy_nonoverlapping(
        addr as *const u8,
        buf.as_mut_ptr().add(SPX_N),
        SPX_SHA256_ADDR_BYTES,
    );
    SPX_mgf1_512(
        bitmask.as_mut_ptr(),
        (inblocks as usize * SPX_N) as core::ffi::c_ulong,
        buf.as_ptr(),
        (SPX_N + SPX_SHA256_ADDR_BYTES) as core::ffi::c_ulong,
    );

    /* Retrieve precomputed state containing pub_seed */
    copy_nonoverlapping(
        (*ctx).state_seeded_512.as_ptr(),
        sha2_state.as_mut_ptr(),
        72,
    );

    i = 0;
    while i < inblocks * SPX_N as c_uint {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i as usize] =
            *in_.add(i as usize) ^ bitmask[i as usize];
        i += 1;
    }

    sha512_inc_finalize(
        outbuf.as_mut_ptr(),
        sha2_state.as_mut_ptr(),
        buf.as_ptr().add(SPX_N),
        SPX_SHA256_ADDR_BYTES + inblocks as usize * SPX_N,
    );
    copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}
