//! Translation of `lib/shake/src/thash_shake_robust.c`.

use super::fips202::shake256_rs;
use crate::address::{addr_ref, Addr};
use crate::context::SpxCtx;
use crate::params::*;
use core::ffi::c_uint;

const BUF_MAX: usize = SPX_N + SPX_ADDR_BYTES + SPX_THASH_MAX_INBLOCKS * SPX_N;
const MASK_MAX: usize = SPX_THASH_MAX_INBLOCKS * SPX_N;

/// Takes an array of `inblocks` concatenated arrays of `SPX_N` bytes.
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &Addr) {
    let ib = inblocks as usize;
    let mut buf = [0u8; BUF_MAX];
    let mut bitmask = [0u8; MASK_MAX];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr);

    shake256_rs(
        &mut bitmask[..ib * SPX_N],
        &buf[..SPX_N + SPX_ADDR_BYTES],
    );

    for i in 0..ib * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    shake256_rs(
        &mut out[..SPX_N],
        &buf[..SPX_N + SPX_ADDR_BYTES + ib * SPX_N],
    );
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
