//! Translation of `lib/blake/src/thash_blake_robust.c` and
//! `lib/blake/src/thash_blake_simple.c` (selected by `spx_thash`).
//!
//! `SPX_BLAKE512` is 0 for the 128-bit level and 1 for 192/256-bit. When it is
//! 1 (`spx_blake512` cfg), multi-block thash uses BLAKE-512; the single-block
//! (F-function) path always uses BLAKE-256.

use crate::address::addr_bytes;
use crate::blake256::blake256;
use crate::context::SpxCtx;
use crate::params::*;

#[cfg(spx_thash = "robust")]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    use crate::blake256::blake256_mgf1;
    #[cfg(spx_blake512)]
    {
        if inblocks > 1 {
            thash_512(out, inp, inblocks, ctx, addr);
            return;
        }
    }

    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; 32];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    let ab = addr_bytes(addr);
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ab[..SPX_ADDR_BYTES]);

    blake256_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake256(
        &mut outbuf,
        &buf[SPX_N..],
        (SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(spx_thash = "robust", spx_blake512))]
fn thash_512(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    use crate::blake512::{blake512, blake512_mgf1};
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; 64];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    let ab = addr_bytes(addr);
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ab[..SPX_ADDR_BYTES]);

    blake512_mgf1(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake512(
        &mut outbuf,
        &buf[SPX_N..],
        (SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(spx_thash = "simple")]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    #[cfg(spx_blake512)]
    {
        if inblocks > 1 {
            thash_512(out, inp, inblocks, ctx, addr);
            return;
        }
    }

    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; 32];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    let ab = addr_bytes(addr);
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ab[..SPX_ADDR_BYTES]);
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N]
        .copy_from_slice(&inp[..inblocks * SPX_N]);

    blake256(
        &mut outbuf,
        &buf,
        (SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(spx_thash = "simple", spx_blake512))]
fn thash_512(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    use crate::blake512::blake512;
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; 64];
    let mut buf = vec![0u8; SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N];

    let ab = addr_bytes(addr);
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ab[..SPX_ADDR_BYTES]);
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N]
        .copy_from_slice(&inp[..inblocks * SPX_N]);

    blake512(
        &mut outbuf,
        &buf,
        (SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
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



