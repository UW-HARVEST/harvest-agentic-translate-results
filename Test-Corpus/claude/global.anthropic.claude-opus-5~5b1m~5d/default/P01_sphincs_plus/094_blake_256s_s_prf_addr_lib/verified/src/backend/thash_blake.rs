//! Translation of `lib/blake/src/thash_blake_robust.c` (THASH=robust, the CMake
//! default) and `lib/blake/src/thash_blake_simple.c` (THASH=simple).

use crate::context::SpxCtx;
use crate::params::{SPX_ADDR_BYTES, SPX_FORS_TREES, SPX_N, SPX_WOTS_LEN};

/// Upper bound on `inblocks` for the in-tree callers; used to size the stack
/// scratch buffers that stand in for the C `SPX_VLA(...)` variable length
/// arrays. Larger values transparently fall back to the heap.
const MAX_INBLOCKS: usize = if SPX_WOTS_LEN > SPX_FORS_TREES {
    SPX_WOTS_LEN
} else {
    SPX_FORS_TREES
};
const SCRATCH_MAX: usize = SPX_N + SPX_ADDR_BYTES + MAX_INBLOCKS * SPX_N;

/// Hands out a zeroed scratch buffer of `len` bytes, preferring the stack
/// array `arr` and falling back to `heap` for oversized requests.
#[inline]
fn scratch<'a>(arr: &'a mut [u8; SCRATCH_MAX], heap: &'a mut Vec<u8>, len: usize) -> &'a mut [u8] {
    if len <= SCRATCH_MAX {
        &mut arr[..len]
    } else {
        heap.resize(len, 0);
        &mut heap[..]
    }
}

// ---------------------------------------------------------------------------
// THASH = robust
// ---------------------------------------------------------------------------
#[cfg(not(feature = "simple"))]
mod robust_impl {
    use super::{scratch, SCRATCH_MAX};
    use crate::address::addr_bytes;
    use crate::backend::blake256::{blake256, blake256_mgf1, SPX_BLAKE256_OUTPUT_BYTES};
    use crate::backend::blake512::{blake512, blake512_mgf1, SPX_BLAKE512_OUTPUT_BYTES};
    use crate::context::SpxCtx;
    use crate::params::{SPX_ADDR_BYTES, SPX_BLAKE512, SPX_N};

    /// Takes an array of inblocks concatenated arrays of SPX_N bytes.
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        if SPX_BLAKE512 == 1 && inblocks > 1 {
            thash_512(out, input, inblocks, ctx, addr);
            return;
        }
        let n = inblocks as usize * SPX_N;
        let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

        let mut bitmask_arr = [0u8; SCRATCH_MAX];
        let mut bitmask_heap: Vec<u8> = Vec::new();
        let bitmask = scratch(&mut bitmask_arr, &mut bitmask_heap, n);

        let mut buf_arr = [0u8; SCRATCH_MAX];
        let mut buf_heap: Vec<u8> = Vec::new();
        let buf = scratch(&mut buf_arr, &mut buf_heap, SPX_N + SPX_ADDR_BYTES + n);

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_ADDR_BYTES]);

        blake256_mgf1(bitmask, &buf[..SPX_N + SPX_ADDR_BYTES]);

        for i in 0..n {
            buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }

        blake256(&mut outbuf, &buf[SPX_N..SPX_N + SPX_ADDR_BYTES + n]);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    fn thash_512(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let n = inblocks as usize * SPX_N;
        let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];

        let mut bitmask_arr = [0u8; SCRATCH_MAX];
        let mut bitmask_heap: Vec<u8> = Vec::new();
        let bitmask = scratch(&mut bitmask_arr, &mut bitmask_heap, n);

        let mut buf_arr = [0u8; SCRATCH_MAX];
        let mut buf_heap: Vec<u8> = Vec::new();
        let buf = scratch(&mut buf_arr, &mut buf_heap, SPX_N + SPX_ADDR_BYTES + n);

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_ADDR_BYTES]);

        blake512_mgf1(bitmask, &buf[..SPX_N + SPX_ADDR_BYTES]);

        for i in 0..n {
            buf[SPX_N + SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
        }

        blake512(&mut outbuf, &buf[SPX_N..SPX_N + SPX_ADDR_BYTES + n]);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }
}

// ---------------------------------------------------------------------------
// THASH = simple
// ---------------------------------------------------------------------------
#[cfg(feature = "simple")]
mod simple_impl {
    use super::{scratch, SCRATCH_MAX};
    use crate::address::addr_bytes;
    use crate::backend::blake256::{blake256, SPX_BLAKE256_OUTPUT_BYTES};
    use crate::backend::blake512::{blake512, SPX_BLAKE512_OUTPUT_BYTES};
    use crate::context::SpxCtx;
    use crate::params::{SPX_ADDR_BYTES, SPX_BLAKE512, SPX_N};

    /// Takes an array of inblocks concatenated arrays of SPX_N bytes.
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        if SPX_BLAKE512 == 1 && inblocks > 1 {
            thash_512(out, input, inblocks, ctx, addr);
            return;
        }
        let n = inblocks as usize * SPX_N;
        let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];

        let mut buf_arr = [0u8; SCRATCH_MAX];
        let mut buf_heap: Vec<u8> = Vec::new();
        let buf = scratch(&mut buf_arr, &mut buf_heap, SPX_N + SPX_ADDR_BYTES + n);

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_ADDR_BYTES]);
        buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + n].copy_from_slice(&input[..n]);

        blake256(&mut outbuf, &buf[..SPX_N + SPX_ADDR_BYTES + n]);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }

    fn thash_512(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let n = inblocks as usize * SPX_N;
        let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];

        let mut buf_arr = [0u8; SCRATCH_MAX];
        let mut buf_heap: Vec<u8> = Vec::new();
        let buf = scratch(&mut buf_arr, &mut buf_heap, SPX_N + SPX_ADDR_BYTES + n);

        buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
        buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&addr_bytes(addr)[..SPX_ADDR_BYTES]);
        buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + n].copy_from_slice(&input[..n]);

        blake512(&mut outbuf, &buf[..SPX_N + SPX_ADDR_BYTES + n]);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    }
}

#[cfg(not(feature = "simple"))]
pub use robust_impl::thash;
#[cfg(feature = "simple")]
pub use simple_impl::thash;

/// `void thash(unsigned char *out, const unsigned char *in,
///             unsigned int inblocks, const spx_ctx *ctx, uint32_t addr[8])`
#[unsafe(no_mangle)]
pub extern "C" fn SPX_thash(
    out: *mut u8,
    input: *const u8,
    inblocks: u32,
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
        let out = core::slice::from_raw_parts_mut(out, SPX_N);
        thash(out, in_copy, inblocks, &*ctx, &mut *(addr as *mut [u32; 8]))
    }
}
