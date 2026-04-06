use crate::context::SpxCtx;
use crate::fors::{ForsGenLeafInfo, SPX_fors_gen_leafx1};

pub unsafe fn fors_gen_leafx1(
    leaf: *mut u8,
    ctx: &SpxCtx,
    addr_idx: u32,
    info: *mut ForsGenLeafInfo,
) {
    SPX_fors_gen_leafx1(leaf, ctx, addr_idx, info);
}
