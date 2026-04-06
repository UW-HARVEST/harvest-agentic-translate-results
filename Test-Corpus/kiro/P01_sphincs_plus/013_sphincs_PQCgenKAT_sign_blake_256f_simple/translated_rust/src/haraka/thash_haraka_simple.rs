use crate::context::SpxCtx;
use crate::params::*;
use super::haraka::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    in_: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    if inblocks == 1 {
        let mut buf_tmp = [0u8; 64];
        let mut outbuf = [0u8; 32];
        core::ptr::copy_nonoverlapping(addr as *const u8, buf_tmp.as_mut_ptr(), 32);
        core::ptr::copy_nonoverlapping(in_, buf_tmp.as_mut_ptr().add(SPX_ADDR_BYTES), SPX_N);
        SPX_haraka512(outbuf.as_mut_ptr(), buf_tmp.as_ptr(), ctx);
        core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
    } else {
        let buf_len = SPX_ADDR_BYTES + inblocks as usize * SPX_N;
        let mut buf = vec![0u8; buf_len];
        core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), 32);
        core::ptr::copy_nonoverlapping(in_, buf.as_mut_ptr().add(SPX_ADDR_BYTES), inblocks as usize * SPX_N);
        SPX_haraka_S(out, SPX_N as u64, buf.as_ptr(), buf_len as u64, ctx);
    }
}
