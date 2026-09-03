//! Translation of `c_src/lib/haraka/src/thash_haraka_robust.c` (default) and
//! `c_src/lib/haraka/src/thash_haraka_simple.c` (feature `simple`).

use crate::context::SpxCtx;
use crate::params::{SPX_ADDR_BYTES, SPX_FORS_TREES, SPX_N, SPX_WOTS_LEN};

/// Upper bound on `inblocks` over all *in-tree* call sites; the C VLAs
/// `SPX_VLA(uint8_t, buf, SPX_ADDR_BYTES + inblocks*SPX_N)` have no upper bound
/// at all, so anything larger falls back to the heap via [`scratch`] rather
/// than panicking (which, with `panic = "abort"`, would kill the process where
/// the C succeeds).
const MAX_INBLOCKS: usize = {
    let a = if SPX_WOTS_LEN > SPX_FORS_TREES {
        SPX_WOTS_LEN
    } else {
        SPX_FORS_TREES
    };
    if a > 2 {
        a
    } else {
        2
    }
};
const MAX_BUF: usize = SPX_ADDR_BYTES + MAX_INBLOCKS * SPX_N;

/// Hands out a zeroed scratch buffer of `len` bytes, preferring the stack array
/// `arr` and falling back to `heap` for oversized requests.
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
    use super::{scratch, MAX_BUF, MAX_INBLOCKS};
    use crate::address::addr_bytes;
    use crate::backend::haraka::{haraka256, haraka512, haraka_S};
    use crate::context::SpxCtx;
    use crate::params::{SPX_ADDR_BYTES, SPX_N};

    /// Takes an array of inblocks concatenated arrays of SPX_N bytes.
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let inblocks = inblocks as usize;
        let mut buf_arr = [0u8; MAX_BUF];
        let mut buf_heap: Vec<u8> = Vec::new();
        let buf = scratch(
            &mut buf_arr,
            &mut buf_heap,
            SPX_ADDR_BYTES + inblocks * SPX_N,
        );
        let mut mask_arr = [0u8; MAX_INBLOCKS * SPX_N];
        let mut mask_heap: Vec<u8> = Vec::new();
        let bitmask = scratch(&mut mask_arr, &mut mask_heap, inblocks * SPX_N);
        let mut outbuf = [0u8; 32];
        let mut buf_tmp = [0u8; 64];

        if inblocks == 1 {
            /* F function */
            /* Since SPX_N may be smaller than 32, we need a temporary buffer. */
            for i in 0..64 {
                buf_tmp[i] = 0;
            }
            buf_tmp[..32].copy_from_slice(&addr_bytes(addr)[..32]);

            haraka256(&mut outbuf, &buf_tmp[..32], ctx);
            for i in 0..(inblocks * SPX_N) {
                buf_tmp[SPX_ADDR_BYTES + i] = input[i] ^ outbuf[i];
            }
            haraka512(&mut outbuf, &buf_tmp, ctx);
            out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
        } else {
            /* All other tweakable hashes*/
            buf[..32].copy_from_slice(&addr_bytes(addr)[..32]);
            haraka_S(
                &mut bitmask[..inblocks * SPX_N],
                &buf[..SPX_ADDR_BYTES],
                ctx,
            );

            for i in 0..(inblocks * SPX_N) {
                buf[SPX_ADDR_BYTES + i] = input[i] ^ bitmask[i];
            }

            haraka_S(
                &mut out[..SPX_N],
                &buf[..SPX_ADDR_BYTES + inblocks * SPX_N],
                ctx,
            );
        }
    }
}

#[cfg(feature = "simple")]
mod simple_impl {
    use super::{scratch, MAX_BUF};
    use crate::address::addr_bytes;
    use crate::backend::haraka::{haraka512, haraka_S};
    use crate::context::SpxCtx;
    use crate::params::{SPX_ADDR_BYTES, SPX_N};

    /// Takes an array of inblocks concatenated arrays of SPX_N bytes.
    pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
        let inblocks = inblocks as usize;
        let mut buf_arr = [0u8; MAX_BUF];
        let mut buf_heap: Vec<u8> = Vec::new();
        let buf = scratch(
            &mut buf_arr,
            &mut buf_heap,
            SPX_ADDR_BYTES + inblocks * SPX_N,
        );
        let mut outbuf = [0u8; 32];
        let mut buf_tmp = [0u8; 64];

        if inblocks == 1 {
            /* F function */
            /* Since SPX_N may be smaller than 32, we need a temporary buffer. */
            for i in 0..64 {
                buf_tmp[i] = 0;
            }
            buf_tmp[..32].copy_from_slice(&addr_bytes(addr)[..32]);
            buf_tmp[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&input[..SPX_N]);

            haraka512(&mut outbuf, &buf_tmp, ctx);
            out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
        } else {
            /* All other tweakable hashes*/
            buf[..32].copy_from_slice(&addr_bytes(addr)[..32]);
            buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + inblocks * SPX_N]
                .copy_from_slice(&input[..inblocks * SPX_N]);

            haraka_S(
                &mut out[..SPX_N],
                &buf[..SPX_ADDR_BYTES + inblocks * SPX_N],
                ctx,
            );
        }
    }
}

#[cfg(not(feature = "simple"))]
pub use robust_impl::thash;
#[cfg(feature = "simple")]
pub use simple_impl::thash;

// ---------------------------------------------------------------------------
// C ABI wrapper (app/include/thash.h)
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
        )
    }
}
