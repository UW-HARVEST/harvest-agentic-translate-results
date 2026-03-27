// thash_shake_robust.rs - Robust SHAKE-256 thash for SPHINCS+
// Translated from c_src/lib/shake/src/thash_shake_robust.c

use core::ptr;
use crate::context::SpxCtx;
use crate::params::*;
use super::fips202::shake256;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8, in_: *const u8, inblocks: u32,
    ctx: *const SpxCtx, addr: *mut u32,
) {
    let in_len = inblocks as usize * SPX_N;
    let buf_len = SPX_N + SPX_ADDR_BYTES + in_len;
    let mut buf = vec![0u8; buf_len];
    let mut bitmask = vec![0u8; in_len];

    ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);

    shake256(&mut bitmask, in_len, &buf[..SPX_N + SPX_ADDR_BYTES]);

    let in_slice = core::slice::from_raw_parts(in_, in_len);
    for i in 0..in_len {
        buf[SPX_N + SPX_ADDR_BYTES + i] = in_slice[i] ^ bitmask[i];
    }

    let mut out_buf = [0u8; SPX_N];
    shake256(&mut out_buf, SPX_N, &buf);
    ptr::copy_nonoverlapping(out_buf.as_ptr(), out, SPX_N);
}
