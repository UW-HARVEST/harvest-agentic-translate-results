//! Translation of `lib/sha2/src/thash_sha2_robust.c`.
//!
//! The C file wraps the SHA-512 variant (and the dispatch to it) in
//! `#if SPX_SHA512`; here both variants are always compiled and the dispatch
//! is done on the compile-time constant `crate::params::SPX_SHA512`, so the
//! generated code (and behaviour) matches the C code for every parameter set.

use core::ptr::{addr_of, copy_nonoverlapping};

use crate::context::SpxCtx;
use crate::params::{SPX_N, SPX_SHA512};
use crate::sha2::sha2::{
    sha256_inc_finalize, sha512_inc_finalize, SPX_mgf1_256, SPX_mgf1_512, SPX_SHA256_ADDR_BYTES,
    SPX_SHA256_OUTPUT_BYTES, SPX_SHA512_OUTPUT_BYTES,
};

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
    /* C: #if SPX_SHA512 ... #endif */
    if SPX_SHA512 && inblocks > 1 {
        thash_512(out, input, inblocks, ctx, addr);
        return;
    }

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks as usize * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_OUTPUT_BYTES + inblocks as usize * SPX_N];
    let mut sha2_state = [0u8; 40];
    let mut i: u32;

    let outbuf_ptr: *mut u8 = outbuf.as_mut_ptr();
    let bitmask_ptr: *mut u8 = bitmask.as_mut_ptr();
    let buf_ptr: *mut u8 = buf.as_mut_ptr();
    let sha2_state_ptr: *mut u8 = sha2_state.as_mut_ptr();

    copy_nonoverlapping(addr_of!((*ctx).pub_seed) as *const u8, buf_ptr, SPX_N);
    copy_nonoverlapping(addr as *const u8, buf_ptr.add(SPX_N), SPX_SHA256_ADDR_BYTES);
    SPX_mgf1_256(
        bitmask_ptr,
        inblocks as u64 * SPX_N as u64,
        buf_ptr as *const u8,
        (SPX_N + SPX_SHA256_ADDR_BYTES) as u64,
    );

    /* Retrieve precomputed state containing pub_seed */
    copy_nonoverlapping(
        addr_of!((*ctx).state_seeded) as *const u8,
        sha2_state_ptr,
        40,
    );

    i = 0;
    while (i as usize) < inblocks as usize * SPX_N {
        *buf_ptr.add(SPX_N + SPX_SHA256_ADDR_BYTES + i as usize) =
            *input.add(i as usize) ^ *bitmask_ptr.add(i as usize);
        i = i.wrapping_add(1);
    }

    sha256_inc_finalize(
        outbuf_ptr,
        sha2_state_ptr,
        buf_ptr.add(SPX_N) as *const u8,
        SPX_SHA256_ADDR_BYTES + inblocks as usize * SPX_N,
    );
    copy_nonoverlapping(outbuf_ptr as *const u8, out, SPX_N);
}

/// `static void thash_512(...)` (only reachable when `SPX_SHA512`).
unsafe fn thash_512(
    out: *mut u8,
    input: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks as usize * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks as usize * SPX_N];
    let mut sha2_state = [0u8; 72];
    let mut i: u32;

    let outbuf_ptr: *mut u8 = outbuf.as_mut_ptr();
    let bitmask_ptr: *mut u8 = bitmask.as_mut_ptr();
    let buf_ptr: *mut u8 = buf.as_mut_ptr();
    let sha2_state_ptr: *mut u8 = sha2_state.as_mut_ptr();

    copy_nonoverlapping(addr_of!((*ctx).pub_seed) as *const u8, buf_ptr, SPX_N);
    copy_nonoverlapping(addr as *const u8, buf_ptr.add(SPX_N), SPX_SHA256_ADDR_BYTES);
    SPX_mgf1_512(
        bitmask_ptr,
        inblocks as u64 * SPX_N as u64,
        buf_ptr as *const u8,
        (SPX_N + SPX_SHA256_ADDR_BYTES) as u64,
    );

    /* Retrieve precomputed state containing pub_seed */
    copy_nonoverlapping(
        addr_of!((*ctx).state_seeded_512) as *const u8,
        sha2_state_ptr,
        72,
    );

    i = 0;
    while (i as usize) < inblocks as usize * SPX_N {
        *buf_ptr.add(SPX_N + SPX_SHA256_ADDR_BYTES + i as usize) =
            *input.add(i as usize) ^ *bitmask_ptr.add(i as usize);
        i = i.wrapping_add(1);
    }

    sha512_inc_finalize(
        outbuf_ptr,
        sha2_state_ptr,
        buf_ptr.add(SPX_N) as *const u8,
        SPX_SHA256_ADDR_BYTES + inblocks as usize * SPX_N,
    );
    copy_nonoverlapping(outbuf_ptr as *const u8, out, SPX_N);
}
