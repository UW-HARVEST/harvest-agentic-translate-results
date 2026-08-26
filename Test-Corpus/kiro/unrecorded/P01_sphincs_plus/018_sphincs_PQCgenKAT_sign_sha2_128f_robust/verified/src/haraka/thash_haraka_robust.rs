use crate::context::SpxCtx;
use crate::params::*;
use crate::haraka::haraka::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    inp: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let inblocks = inblocks as usize;

    if inblocks == 1 {
        let mut outbuf = [0u8; 32];
        let mut buf_tmp = [0u8; 64];

        std::ptr::copy_nonoverlapping(addr as *const u8, buf_tmp.as_mut_ptr(), 32);

        // Generate bitmask via haraka256
        SPX_haraka256(outbuf.as_mut_ptr(), buf_tmp.as_ptr(), ctx);
        for i in 0..inblocks * SPX_N {
            buf_tmp[SPX_ADDR_BYTES + i] = *inp.add(i) ^ outbuf[i];
        }

        SPX_haraka512(outbuf.as_mut_ptr(), buf_tmp.as_ptr(), ctx);
        std::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
    } else {
        let buf_len = SPX_ADDR_BYTES + inblocks * SPX_N;
        let mut buf = vec![0u8; buf_len];
        let mut bitmask = vec![0u8; inblocks * SPX_N];

        std::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), 32);

        // Generate bitmask via haraka_S
        SPX_haraka_S(
            bitmask.as_mut_ptr(),
            (inblocks * SPX_N) as u64,
            buf.as_ptr(),
            SPX_ADDR_BYTES as u64,
            ctx,
        );

        for i in 0..inblocks * SPX_N {
            buf[SPX_ADDR_BYTES + i] = *inp.add(i) ^ bitmask[i];
        }

        SPX_haraka_S(out, SPX_N as u64, buf.as_ptr(), buf_len as u64, ctx);
    }
}
