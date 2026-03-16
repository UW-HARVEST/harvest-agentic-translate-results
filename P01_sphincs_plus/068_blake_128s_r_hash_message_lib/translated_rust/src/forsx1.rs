use crate::address::*;
use crate::context::SpxCtx;
use crate::fors::ForsGenLeafInfo;
use crate::hash_blake::prf_addr;
use crate::params::*;
use crate::thash::thash;

pub fn fors_gen_leafx1(
    leaf: &mut [u8],
    ctx: &SpxCtx,
    addr_idx: u32,
    info: &mut ForsGenLeafInfo,
) {
    let fors_leaf_addr = &mut info.leaf_addrx;

    set_tree_index(fors_leaf_addr, addr_idx);
    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSPRF);
    prf_addr(leaf, ctx, fors_leaf_addr);

    set_type(fors_leaf_addr, SPX_ADDR_TYPE_FORSTREE);
    let tmp = leaf[..SPX_N].to_vec();
    thash(leaf, &tmp, 1, ctx, fors_leaf_addr);
}
