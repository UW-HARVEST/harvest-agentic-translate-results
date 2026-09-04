//! Translation of `lib/haraka/src/thash_haraka_simple.c`.

use super::haraka::*;
use crate::address::{addr_ref, Addr};
use crate::context::SpxCtx;
use crate::params::*;
use crate::vla::Vla;
use core::ffi::c_uint;

const BUF_MAX: usize = SPX_ADDR_BYTES + SPX_THASH_MAX_INBLOCKS * SPX_N;

/// Takes an array of `inblocks` concatenated arrays of `SPX_N` bytes.
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &Addr) {
    let ib = inblocks as usize;
    let mut buf_v = Vla::<BUF_MAX>::new(SPX_ADDR_BYTES + ib * SPX_N);
    let buf = buf_v.as_mut();
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];

    if inblocks == 1 {
        /* F function */
        /* Since SPX_N may be smaller than 32, we need a temporary buffer. */
        buf_tmp.fill(0);
        buf_tmp[..32].copy_from_slice(addr);
        buf_tmp[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&inp[..SPX_N]);

        haraka512_rs(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        /* All other tweakable hashes */
        buf[..32].copy_from_slice(addr);
        buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + ib * SPX_N].copy_from_slice(&inp[..ib * SPX_N]);

        haraka_s_rs(
            &mut out[..SPX_N],
            SPX_N,
            &buf[..SPX_ADDR_BYTES + ib * SPX_N],
            SPX_ADDR_BYTES + ib * SPX_N,
            ctx,
        );
    }
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
