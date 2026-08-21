//! Translation of `lib/haraka/src/thash_haraka_robust.c` and
//! `lib/haraka/src/thash_haraka_simple.c`. Exactly one variant is compiled,
//! selected by the `simple` feature (otherwise `robust`, the default).

#[cfg(not(feature = "simple"))]
use super::haraka::SPX_haraka256;
use super::haraka::{SPX_haraka512, SPX_haraka_S};
use crate::context::SpxCtx;
use crate::params::*;

/// Robust `thash`. Takes an array of inblocks concatenated arrays of SPX_N
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
    let inblocks = inblocks as usize;
    let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks * SPX_N];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];

    if inblocks == 1 {
        // F function
        // Since SPX_N may be smaller than 32, we need a temporary buffer.
        // `memset(buf_tmp, 0, 64)`: already zeroed at declaration.
        core::ptr::copy_nonoverlapping(addr as *const u8, buf_tmp.as_mut_ptr(), 32);

        SPX_haraka256(outbuf.as_mut_ptr(), buf_tmp.as_ptr(), ctx);
        for i in 0..inblocks * SPX_N {
            buf_tmp[SPX_ADDR_BYTES + i] = *in_.add(i) ^ outbuf[i];
        }
        SPX_haraka512(outbuf.as_mut_ptr(), buf_tmp.as_ptr(), ctx);
        core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
    } else {
        // All other tweakable hashes
        core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), 32);
        SPX_haraka_S(
            bitmask.as_mut_ptr(),
            (inblocks * SPX_N) as u64,
            buf.as_ptr(),
            SPX_ADDR_BYTES as u64,
            ctx,
        );

        for i in 0..inblocks * SPX_N {
            buf[SPX_ADDR_BYTES + i] = *in_.add(i) ^ bitmask[i];
        }

        SPX_haraka_S(
            out,
            SPX_N as u64,
            buf.as_ptr(),
            (SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
            ctx,
        );
    }
}

/// Simple `thash`. Takes an array of inblocks concatenated arrays of SPX_N
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
    let inblocks = inblocks as usize;
    let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks * SPX_N];
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];

    if inblocks == 1 {
        // F function
        // Since SPX_N may be smaller than 32, we need a temporary buffer.
        // `memset(buf_tmp, 0, 64)`: already zeroed at declaration.
        core::ptr::copy_nonoverlapping(addr as *const u8, buf_tmp.as_mut_ptr(), 32);
        core::ptr::copy_nonoverlapping(in_, buf_tmp.as_mut_ptr().add(SPX_ADDR_BYTES), SPX_N);

        SPX_haraka512(outbuf.as_mut_ptr(), buf_tmp.as_ptr(), ctx);
        core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
    } else {
        // All other tweakable hashes
        core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), 32);
        core::ptr::copy_nonoverlapping(
            in_,
            buf.as_mut_ptr().add(SPX_ADDR_BYTES),
            inblocks * SPX_N,
        );

        SPX_haraka_S(
            out,
            SPX_N as u64,
            buf.as_ptr(),
            (SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
            ctx,
        );
    }
}
