// thash_shake_simple.rs - Simple SHAKE-256 thash for SPHINCS+
// Translated from c_src/lib/shake/src/thash_shake_simple.c

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

    ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
    ptr::copy_nonoverlapping(in_, buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES), in_len);

    let mut out_buf = [0u8; SPX_N];
    shake256(&mut out_buf, SPX_N, &buf);
    ptr::copy_nonoverlapping(out_buf.as_ptr(), out, SPX_N);
}
