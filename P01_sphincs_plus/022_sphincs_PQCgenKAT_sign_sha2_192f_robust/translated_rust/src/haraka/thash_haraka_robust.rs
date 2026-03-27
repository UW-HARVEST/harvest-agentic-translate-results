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

        // Generate bitmask via haraka256
        SPX_haraka256(outbuf.as_mut_ptr(), buf_tmp.as_ptr(), ctx);
        for i in 0..SPX_N {
            buf_tmp[SPX_ADDR_BYTES + i] = *in_.add(i) ^ outbuf[i];
        }
        SPX_haraka512(outbuf.as_mut_ptr(), buf_tmp.as_ptr(), ctx);
        core::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
    } else {
        let n_bytes = inblocks as usize * SPX_N;
        let buf_len = SPX_ADDR_BYTES + n_bytes;
        let mut buf = vec![0u8; buf_len];
        let mut bitmask = vec![0u8; n_bytes];

        core::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), 32);
        SPX_haraka_S(bitmask.as_mut_ptr(), n_bytes as u64, buf.as_ptr(), SPX_ADDR_BYTES as u64, ctx);

        for i in 0..n_bytes {
            buf[SPX_ADDR_BYTES + i] = *in_.add(i) ^ bitmask[i];
        }
        SPX_haraka_S(out, SPX_N as u64, buf.as_ptr(), buf_len as u64, ctx);
    }
}
