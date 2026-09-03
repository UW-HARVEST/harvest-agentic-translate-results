//! Translation of `c_src/lib/sha2/src/thash_sha2_robust.c` (THASH=robust, the
//! CMake default) and `c_src/lib/sha2/src/thash_sha2_simple.c` (THASH=simple).

use crate::context::SpxCtx;
use crate::params::SPX_N;

/// Upper bound on the `inblocks` argument used anywhere in SPHINCS+
/// (`1`, `2`, `SPX_WOTS_LEN` and `SPX_FORS_TREES`). Used to size the stack
/// scratch buffers that replace the C VLAs; larger values fall back to the
/// heap so that the safe Rust API stays total.
const MAX_INBLOCKS: usize = {
    let a = if crate::params::SPX_FORS_TREES > crate::params::SPX_WOTS_LEN {
        crate::params::SPX_FORS_TREES
    } else {
        crate::params::SPX_WOTS_LEN
    };
    if a > 2 {
        a
    } else {
        2
    }
};

// ===========================================================================
// THASH = robust
// ===========================================================================

#[cfg(not(feature = "simple"))]
mod robust_impl {
    use super::MAX_INBLOCKS;
    use crate::address::addr_bytes;
    use crate::backend::sha2::{
        mgf1_256, mgf1_512, sha256_inc_finalize, sha512_inc_finalize, SPX_SHA256_ADDR_BYTES,
        SPX_SHA256_OUTPUT_BYTES, SPX_SHA512_OUTPUT_BYTES,
    };
    use crate::context::SpxCtx;
    use crate::params::{SPX_N, SPX_SHA512};

    const BITMASK_CAP: usize = MAX_INBLOCKS * SPX_N;
    const BUF_CAP_256: usize = SPX_N + SPX_SHA256_OUTPUT_BYTES + MAX_INBLOCKS * SPX_N;
    const BUF_CAP_512: usize = SPX_N + SPX_SHA256_ADDR_BYTES + MAX_INBLOCKS * SPX_N;

    /// Takes an array of `inblocks` concatenated arrays of SPX_N bytes.
    pub fn thash(
        out: &mut [u8],
        input: &[u8],
        inblocks: u32,
        ctx: &SpxCtx,
        addr: &mut [u32; 8],
    ) {
        if SPX_SHA512 == 1 && inblocks > 1 {
            thash_512(out, input, inblocks, ctx, addr);
            return;
        }

        let n = inblocks as usize * SPX_N;

        let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];

        let mut stack_bitmask = [0u8; BITMASK_CAP];
        let mut heap_bitmask: Vec<u8>;
        let bitmask: &mut [u8] = if n <= BITMASK_CAP {
            &mut stack_bitmask[..n]
        } else {
            heap_bitmask = vec![0u8; n];
            &mut heap_bitmask[..]
        };

        let buflen = SPX_N + SPX_SHA256_OUTPUT_BYTES + n;
        let mut stack_buf = [0u8; BUF_CAP_256];
        let mut heap_buf: Vec<u8>;
        let buf: &mut [u8] = if buflen <= BUF_CAP_256 {
            &mut stack_buf[..buflen]
        } else {
            heap_buf = vec![0u8; buflen];
            &mut heap_buf[..]
        };

        let mut sha2_state = [0u8; 40];

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed[..SPX_N]);
        buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES]
            .copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
        mgf1_256(bitmask, &buf[..SPX_N + SPX_SHA256_ADDR_BYTES]);

        /* Retrieve precomputed state containing pub_seed */
        sha2_state.copy_from_slice(&ctx.state_seeded[..40]);

        for i in 0..n {
            buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }

        sha256_inc_finalize(
            &mut outbuf,
            &mut sha2_state,
            &buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES + n],
        );
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    // `ctx` is only read for `state_seeded_512`, which exists solely when
    // SPX_SHA512 == 1 (the whole function is `#if SPX_SHA512` in the C).
    #[allow(unused_variables)]
    fn thash_512(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let n = inblocks as usize * SPX_N;

        let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];

        let mut stack_bitmask = [0u8; BITMASK_CAP];
        let mut heap_bitmask: Vec<u8>;
        let bitmask: &mut [u8] = if n <= BITMASK_CAP {
            &mut stack_bitmask[..n]
        } else {
            heap_bitmask = vec![0u8; n];
            &mut heap_bitmask[..]
        };

        let buflen = SPX_N + SPX_SHA256_ADDR_BYTES + n;
        let mut stack_buf = [0u8; BUF_CAP_512];
        let mut heap_buf: Vec<u8>;
        let buf: &mut [u8] = if buflen <= BUF_CAP_512 {
            &mut stack_buf[..buflen]
        } else {
            heap_buf = vec![0u8; buflen];
            &mut heap_buf[..]
        };

        let mut sha2_state = [0u8; 72];

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed[..SPX_N]);
        buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES]
            .copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
        mgf1_512(bitmask, &buf[..SPX_N + SPX_SHA256_ADDR_BYTES]);

        /* Retrieve precomputed state containing pub_seed */
        // The whole enclosing `thash_512` is inside `#if SPX_SHA512` in the C,
        // so `state_seeded_512` is guaranteed to exist here.
        #[cfg(spx_big_hash)]
        sha2_state.copy_from_slice(&ctx.state_seeded_512[..72]);

        for i in 0..n {
            buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }

        sha512_inc_finalize(
            &mut outbuf,
            &mut sha2_state,
            &buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES + n],
        );
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }
}

// ===========================================================================
// THASH = simple
// ===========================================================================

#[cfg(feature = "simple")]
mod simple_impl {
    use super::MAX_INBLOCKS;
    use crate::address::addr_bytes;
    use crate::backend::sha2::{
        sha256_inc_finalize, sha512_inc_finalize, SPX_SHA256_ADDR_BYTES, SPX_SHA256_OUTPUT_BYTES,
        SPX_SHA512_OUTPUT_BYTES,
    };
    use crate::context::SpxCtx;
    use crate::params::{SPX_N, SPX_SHA512};

    const BUF_CAP: usize = SPX_SHA256_ADDR_BYTES + MAX_INBLOCKS * SPX_N;

    /// Takes an array of `inblocks` concatenated arrays of SPX_N bytes.
    pub fn thash(
        out: &mut [u8],
        input: &[u8],
        inblocks: u32,
        ctx: &SpxCtx,
        addr: &mut [u32; 8],
    ) {
        if SPX_SHA512 == 1 && inblocks > 1 {
            thash_512(out, input, inblocks, ctx, addr);
            return;
        }

        let n = inblocks as usize * SPX_N;

        let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
        let mut sha2_state = [0u8; 40];

        let buflen = SPX_SHA256_ADDR_BYTES + n;
        let mut stack_buf = [0u8; BUF_CAP];
        let mut heap_buf: Vec<u8>;
        let buf: &mut [u8] = if buflen <= BUF_CAP {
            &mut stack_buf[..buflen]
        } else {
            heap_buf = vec![0u8; buflen];
            &mut heap_buf[..]
        };

        /* Retrieve precomputed state containing pub_seed */
        sha2_state.copy_from_slice(&ctx.state_seeded[..40]);

        buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
        buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + n].copy_from_slice(&input[..n]);

        sha256_inc_finalize(&mut outbuf, &mut sha2_state, buf);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    // `ctx` is only read for `state_seeded_512`, which exists solely when
    // SPX_SHA512 == 1 (the whole function is `#if SPX_SHA512` in the C).
    #[allow(unused_variables)]
    fn thash_512(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let n = inblocks as usize * SPX_N;

        let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
        let mut sha2_state = [0u8; 72];

        let buflen = SPX_SHA256_ADDR_BYTES + n;
        let mut stack_buf = [0u8; BUF_CAP];
        let mut heap_buf: Vec<u8>;
        let buf: &mut [u8] = if buflen <= BUF_CAP {
            &mut stack_buf[..buflen]
        } else {
            heap_buf = vec![0u8; buflen];
            &mut heap_buf[..]
        };

        /* Retrieve precomputed state containing pub_seed */
        // The whole enclosing `thash_512` is inside `#if SPX_SHA512` in the C,
        // so `state_seeded_512` is guaranteed to exist here.
        #[cfg(spx_big_hash)]
        sha2_state.copy_from_slice(&ctx.state_seeded_512[..72]);

        buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
        buf[SPX_SHA256_ADDR_BYTES..SPX_SHA256_ADDR_BYTES + n].copy_from_slice(&input[..n]);

        sha512_inc_finalize(&mut outbuf, &mut sha2_state, buf);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }
}

#[cfg(not(feature = "simple"))]
pub use robust_impl::thash;
#[cfg(feature = "simple")]
pub use simple_impl::thash;

// ---------------------------------------------------------------------------
// C ABI wrapper -- app/include/thash.h
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn SPX_thash(
    out: *mut u8,
    input: *const u8,
    inblocks: core::ffi::c_uint,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    unsafe {
        let inlen = (inblocks as usize) * SPX_N;
        // `thash(out, out, ...)` is a legal (and used) C call pattern
        // (`wots.c:36` `gen_chain`), so stage the input out of the way before
        // handing out a `&mut` to `out`; otherwise we would form an aliasing
        // `&mut`/`&` pair.  Oversized `inblocks` falls back to the heap, since
        // the C `SPX_VLA` has no upper bound.
        let mut in_arr = [0u8; MAX_INBLOCKS * SPX_N];
        let mut in_heap: Vec<u8> = Vec::new();
        let in_copy: &mut [u8] = if inlen <= MAX_INBLOCKS * SPX_N {
            &mut in_arr[..inlen]
        } else {
            in_heap.resize(inlen, 0);
            &mut in_heap[..]
        };
        if inlen != 0 {
            in_copy.copy_from_slice(core::slice::from_raw_parts(input, inlen));
        }
        thash(
            core::slice::from_raw_parts_mut(out, SPX_N),
            in_copy,
            inblocks as u32,
            &*ctx,
            &mut *(addr as *mut [u32; 8]),
        );
    }
}
