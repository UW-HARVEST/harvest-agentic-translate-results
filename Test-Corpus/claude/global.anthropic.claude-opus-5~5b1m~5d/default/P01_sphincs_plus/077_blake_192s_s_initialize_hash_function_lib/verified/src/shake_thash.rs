//! Translation of `lib/shake/src/thash_shake_robust.c` and
//! `lib/shake/src/thash_shake_simple.c` (selected by `spx_thash`).

use crate::address::addr_bytes;
use crate::context::SpxCtx;
use crate::fips202::shake256;
use crate::params::*;

#[cfg(spx_thash = "robust")]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];
    let mut bitmask = vec![0u8; inblocks * SPX_N];

    let ab = addr_bytes(addr);
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ab[..SPX_ADDR_BYTES]);

    shake256(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    shake256(&mut out[..SPX_N], SPX_N, &buf, SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N);
}

#[cfg(spx_thash = "simple")]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    let ab = addr_bytes(addr);
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ab[..SPX_ADDR_BYTES]);
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N]
        .copy_from_slice(&inp[..inblocks * SPX_N]);

    shake256(&mut out[..SPX_N], SPX_N, &buf, SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N);
}

#[no_mangle]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    inp: *const u8,
    inblocks: core::ffi::c_uint,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let o = core::slice::from_raw_parts_mut(out, SPX_N);
    let i = core::slice::from_raw_parts(inp, inblocks as usize * SPX_N);
    thash(o, i, inblocks, &*ctx, &*(addr as *const [u32; 8]));
}
