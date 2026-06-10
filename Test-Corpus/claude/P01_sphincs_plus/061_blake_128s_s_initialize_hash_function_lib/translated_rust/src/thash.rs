// Dispatch the thash function based on selected backend + variant.
use crate::context::SpxCtx;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn SPX_thash(
    out: *mut u8,
    inp: *const u8,
    inblocks: u32,
    ctx: *const crate::context::SpxCtx,
    addr: *mut [u32; 8],
) {
    let inblocks_us = inblocks as usize;
    let n_blocks = inblocks_us;
    let in_slice = unsafe { core::slice::from_raw_parts(inp, n_blocks * crate::params::SPX_N) };
    let out_slice = unsafe { core::slice::from_raw_parts_mut(out, crate::params::SPX_N) };
    let ctx_ref = unsafe { &*ctx };
    let addr_ref = unsafe { &mut *addr };
    thash(out_slice, in_slice, inblocks, ctx_ref, addr_ref);
}

#[inline]
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    crate::backend::thash_impl(out, inp, inblocks, ctx, addr);
}
