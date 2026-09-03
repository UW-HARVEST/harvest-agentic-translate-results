//! Translation of `c_src/lib/shake/src/thash_shake_robust.c` (THASH=robust,
//! the CMake default) and `c_src/lib/shake/src/thash_shake_simple.c`
//! (THASH=simple).
//!
//! The C code uses `SPX_VLA` (a C99 variable-length array) for the scratch
//! buffers, i.e. it accepts *any* `inblocks`.  In-tree `inblocks` is bounded by
//! `max(SPX_WOTS_LEN, SPX_FORS_TREES)`, so the common case uses a fixed-size
//! stack array of that upper bound; but `thash` is a public entry point, so
//! anything larger transparently falls back to the heap instead of panicking
//! (which, with `panic = "abort"`, would kill the process where the C succeeds).

use crate::context::SpxCtx;
use crate::params::{SPX_ADDR_BYTES, SPX_FORS_TREES, SPX_N, SPX_WOTS_LEN};

/// Upper bound on the `inblocks` argument over all call sites.
const MAX_INBLOCKS: usize = if SPX_WOTS_LEN > SPX_FORS_TREES {
    SPX_WOTS_LEN
} else {
    SPX_FORS_TREES
};

/// Hands out a zeroed scratch buffer of `len` bytes, preferring the stack array
/// `arr` and falling back to `heap` for oversized requests.  Stands in for the
/// C `SPX_VLA(...)`, which has no upper bound.
#[inline]
fn scratch<'a, const CAP: usize>(
    arr: &'a mut [u8; CAP],
    heap: &'a mut Vec<u8>,
    len: usize,
) -> &'a mut [u8] {
    if len <= CAP {
        &mut arr[..len]
    } else {
        heap.clear();
        heap.resize(len, 0);
        &mut heap[..]
    }
}

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

        let mut buf_arr = [0u8; BUF_MAX];
        let mut buf_heap: Vec<u8> = Vec::new();
        let buf = super::scratch(&mut buf_arr, &mut buf_heap, buflen);
        let mut mask_arr = [0u8; BITMASK_MAX];
        let mut mask_heap: Vec<u8> = Vec::new();
        let bitmask = super::scratch(&mut mask_arr, &mut mask_heap, masklen);

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed[..SPX_N]);
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_ADDR_BYTES]);

        fips202::shake256(&mut bitmask[..masklen], &buf[..SPX_N + SPX_ADDR_BYTES]);

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

        let mut buf_arr = [0u8; BUF_MAX];
        let mut buf_heap: Vec<u8> = Vec::new();
        let buf = super::scratch(&mut buf_arr, &mut buf_heap, buflen);

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
        let mut in_arr = [0u8; MAX_INBLOCKS * SPX_N];
        let mut in_heap: Vec<u8> = Vec::new();
        let in_copy = scratch(&mut in_arr, &mut in_heap, inlen);
        if inlen != 0 {
            in_copy.copy_from_slice(core::slice::from_raw_parts(input, inlen));
        }
        thash(
            core::slice::from_raw_parts_mut(out, SPX_N),
            in_copy,
            inblocks as u32,
            &*ctx,
            &mut *(addr as *mut [u32; 8]),
        )
    }
}
