//! Translation of `lib/blake/src/thash_blake_robust.c`.
//!
//! The C `#if SPX_BLAKE512` block (present only when `SPX_N >= 24`) adds an
//! early dispatch to a wide-hash helper `thash_512` for `inblocks > 1`.  It is
//! reproduced with `#[cfg(any(feature="192s",feature="192f",feature="256s",
//! feature="256f"))]` (== `SPX_N >= 24`).  The fall-through body (used for
//! `inblocks == 1`, and always when `SPX_N < 24`) uses the BLAKE-256 variant
//! and is present unconditionally, exactly as in the C.

use core::ffi::c_uint;

use crate::context::SpxCtx;
use crate::params::*;

use crate::blake::blake256::blake256;

const SPX_BLAKE256_OUTPUT_BYTES: usize = 32;
#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
const SPX_BLAKE512_OUTPUT_BYTES: usize = 64;

/// Takes an array of inblocks concatenated arrays of SPX_N bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    in_: *const u8,
    inblocks: c_uint,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    /* #if SPX_BLAKE512 -> early dispatch to the 512-bit helper. */
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        if inblocks > 1 {
            thash_512(out, in_, inblocks, ctx, addr);
            return;
        }
    }

    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks as usize * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks as usize * SPX_N];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(
        addr as *const u8,
        buf.as_mut_ptr().add(SPX_N),
        SPX_ADDR_BYTES,
    );

    crate::blake::blake256::SPX_blake256_mgf1(
        bitmask.as_mut_ptr(),
        (inblocks as usize * SPX_N) as core::ffi::c_ulong,
        buf.as_ptr(),
        (SPX_N + SPX_ADDR_BYTES) as core::ffi::c_ulong,
    );

    let mut i: c_uint = 0;
    while (i as usize) < inblocks as usize * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i as usize] =
            *in_.add(i as usize) ^ bitmask[i as usize];
        i += 1;
    }

    blake256(
        outbuf.as_mut_ptr(),
        buf.as_ptr().add(SPX_N),
        (SPX_ADDR_BYTES + inblocks as usize * SPX_N) as core::ffi::c_ulonglong,
    );
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
unsafe fn thash_512(
    out: *mut u8,
    in_: *const u8,
    inblocks: c_uint,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    use crate::blake::blake512::blake512;

    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks as usize * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks as usize * SPX_N];

    core::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    core::ptr::copy_nonoverlapping(
        addr as *const u8,
        buf.as_mut_ptr().add(SPX_N),
        SPX_ADDR_BYTES,
    );

    crate::blake::blake512::SPX_blake512_mgf1(
        bitmask.as_mut_ptr(),
        (inblocks as usize * SPX_N) as core::ffi::c_ulong,
        buf.as_ptr(),
        (SPX_N + SPX_ADDR_BYTES) as core::ffi::c_ulong,
    );

    let mut i: c_uint = 0;
    while (i as usize) < inblocks as usize * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i as usize] =
            *in_.add(i as usize) ^ bitmask[i as usize];
        i += 1;
    }

    blake512(
        outbuf.as_mut_ptr(),
        buf.as_ptr().add(SPX_N),
        (SPX_ADDR_BYTES + inblocks as usize * SPX_N) as core::ffi::c_ulonglong,
    );
    core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}
