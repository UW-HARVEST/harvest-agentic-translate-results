use crate::context::SpxCtx;
use crate::params::*;
use crate::shake::fips202::shake256;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    inp: *const u8,
    inblocks: u32,
    ctx: *const SpxCtx,
    addr: *mut u32,
) {
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks as usize * SPX_N;
    let mut buf = vec![0u8; buf_len];

    std::ptr::copy_nonoverlapping((*ctx).pub_seed.as_ptr(), buf.as_mut_ptr(), SPX_N);
    std::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr().add(SPX_N), SPX_ADDR_BYTES);
    std::ptr::copy_nonoverlapping(inp, buf.as_mut_ptr().add(SPX_N + SPX_ADDR_BYTES), inblocks as usize * SPX_N);

    shake256(out, SPX_N, buf.as_ptr(), buf_len);
}
