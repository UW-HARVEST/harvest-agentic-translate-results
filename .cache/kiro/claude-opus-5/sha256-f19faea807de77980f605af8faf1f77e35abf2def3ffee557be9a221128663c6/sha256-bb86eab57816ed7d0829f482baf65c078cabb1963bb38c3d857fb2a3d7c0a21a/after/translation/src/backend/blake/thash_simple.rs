//! Translation of `lib/blake/src/thash_blake_simple.c`.

use super::blake256::*;
#[cfg(spx_n_ge_24)]
use super::blake512::*;
use crate::address::{addr_ref, Addr};
use crate::context::SpxCtx;
use crate::params::*;
use core::ffi::c_uint;

const BUF_MAX: usize = SPX_N + SPX_ADDR_BYTES + SPX_THASH_MAX_INBLOCKS * SPX_N;

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
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut buf = [0u8; BUF_MAX];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr);
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + ib * SPX_N]
        .copy_from_slice(&inp[..ib * SPX_N]);

    let len = SPX_N + SPX_ADDR_BYTES + ib * SPX_N;
    blake256_rs(&mut outbuf, &buf[..len], len as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(spx_n_ge_24)]
fn thash_512(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &Addr) {
    let ib = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut buf = [0u8; BUF_MAX];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr);
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + ib * SPX_N]
        .copy_from_slice(&inp[..ib * SPX_N]);

    let len = SPX_N + SPX_ADDR_BYTES + ib * SPX_N;
    blake512_rs(&mut outbuf, &buf[..len], len as u64);
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
