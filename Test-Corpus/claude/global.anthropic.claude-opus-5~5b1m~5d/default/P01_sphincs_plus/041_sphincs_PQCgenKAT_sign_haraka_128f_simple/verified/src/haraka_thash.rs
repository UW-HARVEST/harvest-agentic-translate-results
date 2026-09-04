//! Translation of `lib/haraka/src/thash_haraka_robust.c` and
//! `lib/haraka/src/thash_haraka_simple.c` (selected by `spx_thash`).

use crate::address::addr_bytes;
use crate::context::SpxCtx;
#[cfg(spx_thash = "robust")]
use crate::haraka::haraka256;
use crate::haraka::{haraka512, haraka_s};
use crate::params::*;

#[cfg(spx_thash = "robust")]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let ab = addr_bytes(addr);
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];

    if inblocks == 1 {
        for b in buf_tmp.iter_mut() {
            *b = 0;
        }
        buf_tmp[..32].copy_from_slice(&ab[..32]);
        haraka256(&mut outbuf, &buf_tmp, ctx);
        for i in 0..SPX_N {
            buf_tmp[SPX_ADDR_BYTES + i] = inp[i] ^ outbuf[i];
        }
        haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks * SPX_N];
        let mut bitmask = vec![0u8; inblocks * SPX_N];
        buf[..32].copy_from_slice(&ab[..32]);
        haraka_s(&mut bitmask, inblocks * SPX_N, &buf, SPX_ADDR_BYTES as u64, ctx);
        for i in 0..inblocks * SPX_N {
            buf[SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
        }
        haraka_s(
            &mut out[..SPX_N],
            SPX_N,
            &buf,
            (SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
            ctx,
        );
    }
}

#[cfg(spx_thash = "simple")]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let ab = addr_bytes(addr);
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];

    if inblocks == 1 {
        for b in buf_tmp.iter_mut() {
            *b = 0;
        }
        buf_tmp[..32].copy_from_slice(&ab[..32]);
        buf_tmp[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&inp[..SPX_N]);
        haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        let mut buf = vec![0u8; SPX_ADDR_BYTES + inblocks * SPX_N];
        buf[..32].copy_from_slice(&ab[..32]);
        buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + inblocks * SPX_N]
            .copy_from_slice(&inp[..inblocks * SPX_N]);
        haraka_s(
            &mut out[..SPX_N],
            SPX_N,
            &buf,
            (SPX_ADDR_BYTES + inblocks * SPX_N) as u64,
            ctx,
        );
    }
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
