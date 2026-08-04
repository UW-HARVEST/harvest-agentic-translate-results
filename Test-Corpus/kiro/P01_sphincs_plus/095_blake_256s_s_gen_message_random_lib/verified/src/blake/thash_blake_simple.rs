use crate::context::SpxCtx;
use crate::params::*;
use crate::blake::blake256::blake256;
use crate::blake::blake512::blake512;

#[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
unsafe fn thash_512(
    out: *mut u8, inp: *const u8, inblocks: u32,
    ctx: *const SpxCtx, addr: *mut u32,
) {
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];

    std::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    std::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
    std::ptr::copy_nonoverlapping(inp, buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES), inblocks * SPX_N);

    blake512(outbuf.as_mut_ptr(), buf.as_ptr(), buf_len as u64);
    std::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8, inp: *const u8, inblocks: u32,
    ctx: *const SpxCtx, addr: *mut u32,
) {
    #[cfg(any(feature = "192s", feature = "192f", feature = "256s", feature = "256f"))]
    {
        if inblocks > 1 {
            thash_512(out, inp, inblocks, ctx, addr);
            return;
        }
    }

    let inblocks_usize = inblocks as usize;
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks_usize * SPX_N;
    let mut buf = vec![0u8; buf_len];

    std::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    std::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
    std::ptr::copy_nonoverlapping(inp, buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES), inblocks_usize * SPX_N);

    blake256(outbuf.as_mut_ptr(), buf.as_ptr(), buf_len as u64);
    std::ptr::copy_nonoverlapping(outbuf.as_ptr(), out, SPX_N);
}
