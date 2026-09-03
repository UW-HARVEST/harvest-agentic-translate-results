//! Translation of `c_src/lib/shake/src/thash_shake_robust.c` (THASH=robust,
//! the CMake default) and `c_src/lib/shake/src/thash_shake_simple.c`
//! (THASH=simple).
//!
//! The C code uses `SPX_VLA` (a C99 variable-length array) for the scratch
//! buffers.  `inblocks` is bounded by `max(SPX_WOTS_LEN, SPX_FORS_TREES)` at
//! every call site, so we use fixed-size stack arrays of that upper bound and
//! only ever touch the leading `inblocks * SPX_N` bytes.

use crate::context::SpxCtx;
use crate::params::{SPX_ADDR_BYTES, SPX_FORS_TREES, SPX_N, SPX_WOTS_LEN};

/// Upper bound on the `inblocks` argument over all call sites.
const MAX_INBLOCKS: usize = if SPX_WOTS_LEN > SPX_FORS_TREES {
    SPX_WOTS_LEN
} else {
    SPX_FORS_TREES
};

#[cfg(not(feature = "simple"))]
mod robust_impl {
    use super::{MAX_INBLOCKS, SPX_ADDR_BYTES, SPX_N};
    use crate::address::addr_bytes;
    use crate::backend::fips202;
    use crate::context::SpxCtx;

    const BUF_MAX: usize = SPX_N + SPX_ADDR_BYTES + MAX_INBLOCKS * SPX_N;
    const BITMASK_MAX: usize = MAX_INBLOCKS * SPX_N;

    /**
     * Takes an array of inblocks concatenated arrays of SPX_N bytes.
     */
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let inblocks = inblocks as usize;
        let buflen = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
        let masklen = inblocks * SPX_N;

        let mut buf = [0u8; BUF_MAX];
        let mut bitmask = [0u8; BITMASK_MAX];

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed[..SPX_N]);
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_ADDR_BYTES]);

        fips202::shake256(
            &mut bitmask[..masklen],
            &buf[..SPX_N + SPX_ADDR_BYTES],
        );

        for i in 0..masklen {
            buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }

        fips202::shake256(&mut out[..SPX_N], &buf[..buflen]);
    }
}

#[cfg(feature = "simple")]
mod simple_impl {
    use super::{MAX_INBLOCKS, SPX_ADDR_BYTES, SPX_N};
    use crate::address::addr_bytes;
    use crate::backend::fips202;
    use crate::context::SpxCtx;

    const BUF_MAX: usize = SPX_N + SPX_ADDR_BYTES + MAX_INBLOCKS * SPX_N;

    /**
     * Takes an array of inblocks concatenated arrays of SPX_N bytes.
     */
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let inblocks = inblocks as usize;
        let buflen = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;

        let mut buf = [0u8; BUF_MAX];

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed[..SPX_N]);
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_ADDR_BYTES]);
        buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N]
            .copy_from_slice(&input[..inblocks * SPX_N]);

        fips202::shake256(&mut out[..SPX_N], &buf[..buflen]);
    }
}

#[cfg(not(feature = "simple"))]
pub use robust_impl::thash;

#[cfg(feature = "simple")]
pub use simple_impl::thash;

// ---------------------------------------------------------------------------
// C ABI wrapper (`thash` is namespaced in `app/include/thash.h`)
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
        // `thash(out, out, ...)` is a legal (and used) C call pattern, so copy
        // the input out of the way before handing out a `&mut` to `out`.
        let mut in_copy = [0u8; MAX_INBLOCKS * SPX_N];
        if inlen != 0 {
            in_copy[..inlen].copy_from_slice(core::slice::from_raw_parts(input, inlen));
        }
        thash(
            core::slice::from_raw_parts_mut(out, SPX_N),
            &in_copy[..inlen],
            inblocks as u32,
            &*ctx,
            &mut *(addr as *mut [u32; 8]),
        )
    }
}
