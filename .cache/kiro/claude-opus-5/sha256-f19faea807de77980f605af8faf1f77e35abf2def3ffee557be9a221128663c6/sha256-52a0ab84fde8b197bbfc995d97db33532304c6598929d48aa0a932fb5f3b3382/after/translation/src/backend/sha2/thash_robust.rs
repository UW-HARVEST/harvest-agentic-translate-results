//! Translation of `lib/sha2/src/thash_sha2_robust.c`.

use super::sha2::*;
use crate::address::{addr_ref, Addr};
use crate::context::SpxCtx;
use crate::params::*;
use crate::vla::Vla;
use core::ffi::c_uint;

const MASK_MAX: usize = SPX_THASH_MAX_INBLOCKS * SPX_N;
const BUF256_MAX: usize = SPX_N + SPX_SHA256_OUTPUT_BYTES + SPX_THASH_MAX_INBLOCKS * SPX_N;
#[cfg(spx_n_ge_24)]
const BUF512_MAX: usize = SPX_N + SPX_SHA256_ADDR_BYTES + SPX_THASH_MAX_INBLOCKS * SPX_N;

/// Takes an array of `inblocks` concatenated arrays of `SPX_N` bytes.
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &Addr) {
    #[cfg(spx_n_ge_24)]
    {
        if inblocks > 1 {
            thash_512(out, inp, inblocks, ctx, addr);
            return;
        }
    }

    let ib = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask_v = Vla::<MASK_MAX>::new(ib * SPX_N);
    let bitmask = bitmask_v.as_mut();
    let mut buf_v = Vla::<BUF256_MAX>::new(SPX_N + SPX_SHA256_OUTPUT_BYTES + ib * SPX_N);
    let buf = buf_v.as_mut();
    let mut sha2_state = [0u8; 40];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr[..SPX_SHA256_ADDR_BYTES]);
    mgf1_256_rs(
        &mut bitmask[..ib * SPX_N],
        ib * SPX_N,
        &buf[..SPX_N + SPX_SHA256_ADDR_BYTES],
        SPX_N + SPX_SHA256_ADDR_BYTES,
    );

    /* Retrieve precomputed state containing pub_seed */
    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..ib * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    {
        let end = SPX_N + SPX_SHA256_ADDR_BYTES + ib * SPX_N;
        sha256_inc_finalize_rs(&mut outbuf, &mut sha2_state, &buf[SPX_N..end]);
    }
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(spx_n_ge_24)]
fn thash_512(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &Addr) {
    let ib = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut bitmask_v = Vla::<MASK_MAX>::new(ib * SPX_N);
    let bitmask = bitmask_v.as_mut();
    let mut buf_v = Vla::<BUF512_MAX>::new(SPX_N + SPX_SHA256_ADDR_BYTES + ib * SPX_N);
    let buf = buf_v.as_mut();
    let mut sha2_state = [0u8; 72];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr[..SPX_SHA256_ADDR_BYTES]);
    mgf1_512_rs(
        &mut bitmask[..ib * SPX_N],
        ib * SPX_N,
        &buf[..SPX_N + SPX_SHA256_ADDR_BYTES],
        SPX_N + SPX_SHA256_ADDR_BYTES,
    );

    /* Retrieve precomputed state containing pub_seed */
    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    for i in 0..ib * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    {
        let end = SPX_N + SPX_SHA256_ADDR_BYTES + ib * SPX_N;
        sha512_inc_finalize_rs(&mut outbuf, &mut sha2_state, &buf[SPX_N..end]);
    }
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    inp: *const u8,
    inblocks: c_uint,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let o = core::slice::from_raw_parts_mut(out, SPX_N);
    let i = core::slice::from_raw_parts(inp, inblocks as usize * SPX_N);
    thash(o, i, inblocks as u32, &*ctx, addr_ref(addr as *const u32));
}
